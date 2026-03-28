use codex_nano_manager::config::{AppConfig, ConfigStore, ProviderConfig};
use tempfile::tempdir;

#[test]
fn load_missing_file_returns_default_config() {
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(
        dir.path()
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"),
    );

    let config = store.load().unwrap();

    assert_eq!(config.version, 1);
    assert!(config.providers.is_empty());
}

#[test]
fn save_and_reload_round_trip_preserves_provider_fields() {
    let dir = tempdir().unwrap();
    let path = dir
        .path()
        .join(".codex")
        .join("codex-nano-manager")
        .join("config.toml");
    let store = ConfigStore::new(path.clone());

    let mut config = AppConfig::default();
    config.providers.insert(
        "router-a".into(),
        ProviderConfig {
            name: "router-a".into(),
            base_url: "https://router.example.com/v1".into(),
            env_key: "ROUTER_A_API_KEY".into(),
            api_key: "secret".into(),
            model: Some("gpt-5.4".into()),
            last_used_at: None,
            notes: Some("internal relay".into()),
        },
    );

    store.save(&config).unwrap();
    let reloaded = store.load().unwrap();

    assert_eq!(reloaded.providers["router-a"].env_key, "ROUTER_A_API_KEY");
    assert_eq!(
        reloaded.providers["router-a"].notes.as_deref(),
        Some("internal relay")
    );
    assert!(path.exists());
}
