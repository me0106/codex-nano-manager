use crate::error::AppError;
use serde::Deserialize;

const PROVIDERS_JSON: &str = include_str!("providers.json");

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ProviderTemplate {
    pub id: String,
    pub display_name: String,
    pub base_url: String,
    pub default_model: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateChoice {
    BuiltIn(ProviderTemplate),
    Custom,
}

pub fn load_builtin_templates() -> Result<Vec<ProviderTemplate>, AppError> {
    Ok(serde_json::from_str(PROVIDERS_JSON)?)
}

pub fn template_choices() -> Result<Vec<TemplateChoice>, AppError> {
    let mut choices = load_builtin_templates()?
        .into_iter()
        .map(TemplateChoice::BuiltIn)
        .collect::<Vec<_>>();
    choices.push(TemplateChoice::Custom);
    Ok(choices)
}

#[cfg(test)]
mod tests {
    use super::{TemplateChoice, load_builtin_templates, template_choices};

    #[test]
    fn parses_builtin_templates_and_preserves_ggboom_fields() {
        let templates = load_builtin_templates().unwrap();
        let ggboom = templates
            .iter()
            .find(|template| template.id == "ggboom")
            .unwrap();

        assert_eq!(ggboom.display_name, "GGBoom");
        assert_eq!(ggboom.base_url, "https://ai.qaq.al");
        assert_eq!(ggboom.default_model.as_deref(), None);
    }

    #[test]
    fn parses_builtin_templates_without_env_key() {
        let templates = load_builtin_templates().unwrap();
        let quickly = templates
            .iter()
            .find(|template| template.id == "quickly")
            .unwrap();

        assert_eq!(quickly.display_name, "Quickly");
        assert_eq!(quickly.base_url, "https://sub.jlypx.de");
        assert_eq!(quickly.default_model.as_deref(), None);
    }

    #[test]
    fn appends_custom_choice_last() {
        let choices = template_choices().unwrap();

        assert!(matches!(choices.last(), Some(TemplateChoice::Custom)));
    }
}
