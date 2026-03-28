use crate::config::ProviderConfig;
use crate::provider_templates::TemplateChoice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Manage,
    Run,
    Exec,
}

pub enum UiScreen {
    List(ListScreenState),
    AddTemplate(AddTemplateState),
    AddForm(AddFormState),
    EditForm(EditFormState),
    DeleteConfirm,
}

pub struct UiState {
    pub mode: UiMode,
    pub selected: usize,
    pub providers: Vec<ProviderConfig>,
    pub screen: UiScreen,
}

impl UiState {
    pub fn new(mode: UiMode, providers: Vec<ProviderConfig>) -> Self {
        Self {
            mode,
            selected: 0,
            providers,
            screen: UiScreen::List(ListScreenState::default()),
        }
    }
}

#[derive(Default)]
pub struct SearchState {
    pub query: String,
    pub active: bool,
}

#[derive(Default)]
pub struct ListScreenState {
    pub search: SearchState,
}

pub struct AddFormState {
    pub active_field: usize,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub notes: String,
    pub error: Option<String>,
}

pub struct EditFormState {
    pub original_name: String,
    pub active_field: usize,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub notes: String,
    pub error: Option<String>,
}

pub struct AddTemplateState {
    pub choices: Vec<TemplateChoice>,
    pub selected: usize,
    pub search: SearchState,
}

impl AddTemplateState {
    pub fn new(choices: Vec<TemplateChoice>) -> Self {
        Self {
            choices,
            selected: 0,
            search: SearchState::default(),
        }
    }
}

impl AddFormState {
    pub fn custom() -> Self {
        Self {
            active_field: 0,
            name: String::new(),
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            notes: String::new(),
            error: None,
        }
    }

    pub fn ready_to_submit(name: &str, base_url: &str, api_key: &str) -> Self {
        Self {
            active_field: 5,
            name: name.to_string(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model: String::new(),
            notes: String::new(),
            error: None,
        }
    }
}

impl EditFormState {
    pub fn from_provider(provider: &ProviderConfig) -> Self {
        Self {
            original_name: provider.name.clone(),
            active_field: 0,
            name: provider.name.clone(),
            base_url: provider.base_url.clone(),
            api_key: String::new(),
            model: provider.model.clone().unwrap_or_default(),
            notes: provider.notes.clone().unwrap_or_default(),
            error: None,
        }
    }

    pub fn ready_to_submit(provider: &ProviderConfig, name: &str) -> Self {
        Self {
            active_field: 5,
            name: name.to_string(),
            ..Self::from_provider(provider)
        }
    }
}

pub fn provider_matches_search(provider: &ProviderConfig, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }

    provider.name.to_ascii_lowercase().contains(&query)
        || provider.base_url.to_ascii_lowercase().contains(&query)
        || provider
            .notes
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(&query)
}

pub fn template_matches_search(choice: &TemplateChoice, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }

    match choice {
        TemplateChoice::BuiltIn(template) => {
            template.display_name.to_ascii_lowercase().contains(&query)
                || template.base_url.to_ascii_lowercase().contains(&query)
                || template
                    .notes
                    .as_deref()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .contains(&query)
        }
        TemplateChoice::Custom => {
            ["custom", "manual", "endpoint"]
                .into_iter()
                .any(|token| token.contains(&query) || query.contains(token))
        }
    }
}
