use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub version: u32,
    pub providers: BTreeMap<String, ProviderConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            providers: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub env_key: String,
    pub api_key: String,
    pub model: Option<String>,
    pub last_used_at: Option<String>,
    pub notes: Option<String>,
}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn default_path() -> Result<PathBuf, AppError> {
        let home = dirs::home_dir().ok_or(AppError::HomeDirectoryUnavailable)?;
        Ok(home
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"))
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<AppConfig, AppError> {
        if !self.path.exists() {
            return Ok(AppConfig::default());
        }

        let raw = fs::read_to_string(&self.path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), AppError> {
        let parent = self.path.parent().expect("config path must have a parent");
        fs::create_dir_all(parent)?;

        let raw = toml::to_string_pretty(config)?;
        let mut temp = NamedTempFile::new_in(parent)?;
        temp.write_all(raw.as_bytes())?;
        temp.persist(&self.path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigStore;

    #[test]
    fn default_path_points_to_codex_nano_manager_config() {
        let home = dirs::home_dir().unwrap();

        assert_eq!(
            ConfigStore::default_path().unwrap(),
            home.join(".codex")
                .join("codex-nano-manager")
                .join("config.toml"),
        );
    }
}
