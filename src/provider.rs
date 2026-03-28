use crate::config::{AppConfig, ProviderConfig};
use crate::error::AppError;
use crate::provider_templates::{ProviderTemplate, TemplateChoice};
use crate::selector::select_template_choice;
use dialoguer::{Confirm, Input, Password, theme::ColorfulTheme};

#[derive(Clone)]
pub struct NewProviderInput {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone)]
pub struct EditProviderInput {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub notes: Option<String>,
}

pub trait ProviderPrompter {
    fn prompt_new_provider(
        &mut self,
        existing: &AppConfig,
        templates: &[ProviderTemplate],
    ) -> Result<NewProviderInput, AppError>;
    fn prompt_edit_provider(
        &mut self,
        existing: &ProviderConfig,
    ) -> Result<EditProviderInput, AppError>;
    fn confirm_remove(&mut self, name: &str) -> Result<bool, AppError>;
}

pub struct DialoguerPrompter;

impl ProviderPrompter for DialoguerPrompter {
    fn prompt_new_provider(
        &mut self,
        existing: &AppConfig,
        templates: &[ProviderTemplate],
    ) -> Result<NewProviderInput, AppError> {
        let mut choices = templates
            .iter()
            .cloned()
            .map(TemplateChoice::BuiltIn)
            .collect::<Vec<_>>();
        choices.push(TemplateChoice::Custom);

        match select_template_choice(&choices)? {
            TemplateChoice::BuiltIn(template) => {
                let default_name = default_template_provider_name(&template);
                let name = if existing.providers.contains_key(&default_name) {
                    Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Local provider name")
                        .with_initial_text(default_name)
                        .interact_text()?
                } else {
                    default_name
                };
                let api_key = Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("API key")
                    .interact()?;
                Ok(new_provider_input_from_template(&template, name, api_key))
            }
            TemplateChoice::Custom => self.prompt_custom_provider(),
        }
    }

    fn prompt_edit_provider(
        &mut self,
        existing: &ProviderConfig,
    ) -> Result<EditProviderInput, AppError> {
        Ok(EditProviderInput {
            name: Some(
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Provider name")
                    .with_initial_text(existing.name.clone())
                    .interact_text()?,
            ),
            base_url: Some(
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Base URL")
                    .with_initial_text(existing.base_url.clone())
                    .interact_text()?,
            ),
            api_key: Some(
                Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("API key (leave blank to keep current)")
                    .allow_empty_password(true)
                    .interact()?,
            ),
            model: optional_input_with_initial(
                "Default model (optional)",
                existing.model.clone().unwrap_or_default(),
            )?,
            notes: optional_input_with_initial(
                "Notes (optional)",
                existing.notes.clone().unwrap_or_default(),
            )?,
        })
    }

    fn confirm_remove(&mut self, name: &str) -> Result<bool, AppError> {
        Ok(Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Remove provider '{name}'?"))
            .default(false)
            .interact()?)
    }
}

impl DialoguerPrompter {
    fn prompt_custom_provider(&mut self) -> Result<NewProviderInput, AppError> {
        Ok(NewProviderInput {
            name: Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Provider name")
                .interact_text()?,
            base_url: Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Base URL")
                .interact_text()?,
            api_key: Password::with_theme(&ColorfulTheme::default())
                .with_prompt("API key")
                .interact()?,
            model: optional_input("Default model (optional)")?,
            notes: optional_input("Notes (optional)")?,
        })
    }
}

pub fn default_template_provider_name(template: &ProviderTemplate) -> String {
    template.id.clone()
}

pub fn new_provider_input_from_template(
    template: &ProviderTemplate,
    name: String,
    api_key: String,
) -> NewProviderInput {
    NewProviderInput {
        name,
        base_url: template.base_url.clone(),
        api_key,
        model: template.default_model.clone(),
        notes: template.notes.clone(),
    }
}

fn generate_env_key(config: &AppConfig) -> String {
    loop {
        let candidate = format!("CXCM_KEY_{:08X}", fastrand::u32(..));
        if !config
            .providers
            .values()
            .any(|provider| provider.env_key == candidate)
        {
            return candidate;
        }
    }
}

pub fn insert_provider(config: &mut AppConfig, input: NewProviderInput) -> Result<(), AppError> {
    validate_required(&input.name, "name")?;
    validate_required(&input.base_url, "base_url")?;
    validate_required(&input.api_key, "api_key")?;
    validate_name_available(config, &input.name)?;
    let env_key = generate_env_key(config);
    validate_env_key(&env_key)?;

    config.providers.insert(
        input.name.clone(),
        ProviderConfig {
            name: input.name,
            base_url: input.base_url,
            env_key,
            api_key: input.api_key,
            model: input.model,
            last_used_at: None,
            notes: input.notes,
        },
    );

    Ok(())
}

pub fn apply_edit(
    existing: &ProviderConfig,
    input: EditProviderInput,
) -> Result<ProviderConfig, AppError> {
    let name = input.name.unwrap_or_else(|| existing.name.clone());
    validate_required(&name, "name")?;

    let base_url = input.base_url.unwrap_or_else(|| existing.base_url.clone());
    validate_required(&base_url, "base_url")?;

    let api_key = match input.api_key {
        Some(value) if value.trim().is_empty() => existing.api_key.clone(),
        Some(value) => value,
        None => existing.api_key.clone(),
    };

    Ok(ProviderConfig {
        name,
        base_url,
        env_key: existing.env_key.clone(),
        api_key,
        model: input.model.or_else(|| existing.model.clone()),
        last_used_at: existing.last_used_at.clone(),
        notes: input.notes.or_else(|| existing.notes.clone()),
    })
}

pub fn remove_provider(config: &mut AppConfig, name: &str) -> Result<(), AppError> {
    config
        .providers
        .remove(name)
        .map(|_| ())
        .ok_or_else(|| AppError::provider_not_found(name))
}

pub fn mask_api_key(value: &str) -> String {
    let suffix: String = value
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("********{suffix}")
}

pub fn render_provider_lines(config: &AppConfig) -> Vec<String> {
    config
        .providers
        .values()
        .map(|provider| {
            format!(
                "{} | {} | {} | {} | {} | {}",
                provider.name,
                provider.base_url,
                provider.env_key,
                provider.model.clone().unwrap_or_else(|| "-".to_string()),
                provider
                    .last_used_at
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                provider.notes.clone().unwrap_or_else(|| "-".to_string()),
            )
        })
        .collect()
}

fn validate_name_available(config: &AppConfig, name: &str) -> Result<(), AppError> {
    if config.providers.contains_key(name) {
        return Err(AppError::validation(format!(
            "provider '{name}' already exists"
        )));
    }
    Ok(())
}

fn validate_required(value: &str, field: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::validation(format!("{field} cannot be empty")));
    }
    Ok(())
}

fn validate_env_key(value: &str) -> Result<(), AppError> {
    let mut chars = value.chars();
    match chars.next() {
        Some(ch) if ch == '_' || ch.is_ascii_uppercase() => {}
        _ => {
            return Err(AppError::validation(format!(
                "env_key '{value}' is invalid"
            )));
        }
    }

    if chars.all(|ch| ch == '_' || ch.is_ascii_uppercase() || ch.is_ascii_digit()) {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "env_key '{value}' is invalid"
        )))
    }
}

fn optional_input(prompt: &str) -> Result<Option<String>, AppError> {
    optional_input_with_initial(prompt, String::new())
}

fn optional_input_with_initial(prompt: &str, initial: String) -> Result<Option<String>, AppError> {
    let value: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .with_initial_text(initial)
        .allow_empty(true)
        .interact_text()?;

    if value.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EditProviderInput, NewProviderInput, apply_edit, default_template_provider_name,
        insert_provider, mask_api_key, new_provider_input_from_template, remove_provider,
    };
    use crate::config::{AppConfig, ProviderConfig};
    use crate::provider_templates::ProviderTemplate;

    fn sample_provider() -> ProviderConfig {
        ProviderConfig {
            name: "router-a".into(),
            base_url: "https://router.example.com/v1".into(),
            env_key: "ROUTER_A_API_KEY".into(),
            api_key: "secret-1234".into(),
            model: Some("gpt-5.4".into()),
            last_used_at: None,
            notes: None,
        }
    }

    #[test]
    fn rejects_duplicate_provider_names() {
        let mut config = AppConfig::default();
        config
            .providers
            .insert("router-a".into(), sample_provider());

        let err = insert_provider(
            &mut config,
            NewProviderInput {
                name: "router-a".into(),
                base_url: "https://another.example.com/v1".into(),
                api_key: "secret".into(),
                model: None,
                notes: None,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn preserves_api_key_when_edit_secret_is_blank() {
        let existing = sample_provider();

        let updated = apply_edit(
            &existing,
            EditProviderInput {
                name: None,
                base_url: Some("https://updated.example.com/v1".into()),
                api_key: Some(String::new()),
                model: Some("gpt-5.4".into()),
                notes: Some("updated".into()),
            },
        )
        .unwrap();

        assert_eq!(updated.api_key, "secret-1234");
        assert_eq!(updated.env_key, "ROUTER_A_API_KEY");
    }

    #[test]
    fn generated_env_key_is_valid() {
        let mut config = AppConfig::default();

        insert_provider(
            &mut config,
            NewProviderInput {
                name: "router-a".into(),
                base_url: "https://router.example.com/v1".into(),
                api_key: "secret".into(),
                model: None,
                notes: None,
            },
        )
        .unwrap();

        assert!(
            config.providers["router-a"]
                .env_key
                .starts_with("CXCM_KEY_")
        );
    }

    #[test]
    fn masks_api_keys_in_list_output() {
        assert_eq!(mask_api_key("secret-1234"), "********1234");
    }

    #[test]
    fn remove_provider_fails_for_missing_name() {
        let mut config = AppConfig::default();
        let err = remove_provider(&mut config, "missing").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn insert_provider_generates_internal_env_key() {
        let mut config = AppConfig::default();

        insert_provider(
            &mut config,
            NewProviderInput {
                name: "openai".into(),
                base_url: "https://api.openai.com/v1".into(),
                api_key: "secret".into(),
                model: Some("gpt-5.4".into()),
                notes: None,
            },
        )
        .unwrap();

        let provider = &config.providers["openai"];
        assert!(provider.env_key.starts_with("CXCM_KEY_"));
        assert_eq!(provider.env_key.len(), 17);
    }

    #[test]
    fn edit_provider_preserves_existing_internal_env_key() {
        let existing = ProviderConfig {
            name: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            env_key: "CXCM_KEY_7F3A91C2".into(),
            api_key: "secret".into(),
            model: Some("gpt-5.4".into()),
            last_used_at: None,
            notes: None,
        };

        let updated = apply_edit(
            &existing,
            EditProviderInput {
                name: None,
                base_url: Some("https://api.openai.com/v1".into()),
                api_key: Some("updated".into()),
                model: Some("gpt-5.4".into()),
                notes: Some("updated".into()),
            },
        )
        .unwrap();

        assert_eq!(updated.env_key, "CXCM_KEY_7F3A91C2");
    }

    #[test]
    fn edit_provider_can_rename_name_when_supplied() {
        let existing = sample_provider();

        let updated = apply_edit(
            &existing,
            EditProviderInput {
                name: Some("router-b".into()),
                base_url: Some(existing.base_url.clone()),
                api_key: Some(String::new()),
                model: existing.model.clone(),
                notes: Some("renamed".into()),
            },
        )
        .unwrap();

        assert_eq!(updated.name, "router-b");
        assert_eq!(updated.env_key, "ROUTER_A_API_KEY");
    }

    #[test]
    fn builds_new_provider_input_from_template_defaults() {
        let template = ProviderTemplate {
            id: "ggboom".into(),
            display_name: "GGBoom".into(),
            base_url: "https://ai.qaq.al".into(),
            default_model: None,
            notes: Some("GGBoom OpenAI-compatible endpoint".into()),
        };

        let input =
            new_provider_input_from_template(&template, "ggboom".into(), "sk-test".into());

        assert_eq!(input.name, "ggboom");
        assert_eq!(input.base_url, "https://ai.qaq.al");
        assert_eq!(input.api_key, "sk-test");
    }

    #[test]
    fn template_default_name_uses_template_id() {
        let template = ProviderTemplate {
            id: "quickly".into(),
            display_name: "Quickly".into(),
            base_url: "https://sub.jlypx.de".into(),
            default_model: None,
            notes: None,
        };

        assert_eq!(default_template_provider_name(&template), "quickly");
    }
}
