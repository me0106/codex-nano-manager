#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("could not resolve the home directory")]
    HomeDirectoryUnavailable,
    #[error("no providers configured; run `codex-nano-manager add` first")]
    NoProvidersConfigured,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl AppError {
    pub fn validation(message: String) -> Self {
        Self::Message(message)
    }

    pub fn provider_not_found(name: &str) -> Self {
        Self::Message(format!("provider '{name}' not found"))
    }
}

impl From<toml::de::Error> for AppError {
    fn from(value: toml::de::Error) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(value: toml::ser::Error) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<tempfile::PersistError> for AppError {
    fn from(value: tempfile::PersistError) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<dialoguer::Error> for AppError {
    fn from(value: dialoguer::Error) -> Self {
        match value {
            dialoguer::Error::IO(err) => Self::Io(err),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Message(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn no_providers_message_uses_codex_nano_manager_command() {
        assert_eq!(
            AppError::NoProvidersConfigured.to_string(),
            "no providers configured; run `codex-nano-manager add` first"
        );
    }
}
