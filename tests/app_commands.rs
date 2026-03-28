use codex_nano_manager::config::{AppConfig, ConfigStore, ProviderConfig};
use codex_nano_manager::error::AppError;
use codex_nano_manager::launcher::{CodexLauncher, LaunchRequest};
use codex_nano_manager::provider::{EditProviderInput, NewProviderInput};
use codex_nano_manager::selector::ProviderSelector;
use codex_nano_manager::ui::action::UiAction;
use codex_nano_manager::{App, ProviderPageDispatch};
use std::cell::RefCell;
use std::rc::Rc;
use tempfile::tempdir;

fn provider() -> ProviderConfig {
    ProviderConfig {
        name: "router-a".into(),
        base_url: "https://router.example.com/v1".into(),
        env_key: "ROUTER_A_API_KEY".into(),
        api_key: "secret".into(),
        model: Some("gpt-5.4".into()),
        last_used_at: None,
        notes: Some("internal relay".into()),
    }
}

struct FixedSelector {
    selected: String,
}

impl FixedSelector {
    fn new(selected: &str) -> Self {
        Self {
            selected: selected.to_string(),
        }
    }
}

impl ProviderSelector for FixedSelector {
    fn select(&self, _providers: &[ProviderConfig]) -> Result<String, AppError> {
        Ok(self.selected.clone())
    }
}

struct MemoryLauncher {
    exit_code: i32,
    calls: Rc<RefCell<Vec<LaunchRequest>>>,
}

impl MemoryLauncher {
    fn with_exit_code(exit_code: i32) -> Self {
        Self {
            exit_code,
            calls: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl CodexLauncher for MemoryLauncher {
    fn launch(&mut self, request: LaunchRequest) -> Result<i32, AppError> {
        self.calls.borrow_mut().push(request);
        Ok(self.exit_code)
    }
}

#[test]
fn execute_launches_selected_provider_with_passthrough_args() {
    let mut config = AppConfig::default();
    config.providers.insert("router-a".into(), provider());
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(
        dir.path()
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"),
    );
    let launcher = MemoryLauncher::with_exit_code(0);
    let calls = launcher.calls.clone();

    let mut app = App::new(
        store,
        config,
        FixedSelector::new("router-a"),
        launcher,
        || "2026-03-25T14:00:00Z".to_string(),
    );

    let code = app
        .execute(vec!["exec".into(), "--json".into(), "hello".into()])
        .unwrap();

    assert_eq!(code, 0);
    assert_eq!(
        app.config().providers["router-a"].last_used_at.as_deref(),
        Some("2026-03-25T14:00:00Z")
    );
    assert_eq!(calls.borrow()[0].env["ROUTER_A_API_KEY"], "secret");
    assert_eq!(
        calls.borrow()[0].args[calls.borrow()[0].args.len() - 3..],
        vec![
            "exec".to_string(),
            "--json".to_string(),
            "hello".to_string()
        ]
    );
}

#[test]
fn execute_propagates_non_zero_exit_code() {
    let mut config = AppConfig::default();
    config.providers.insert("router-a".into(), provider());
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(
        dir.path()
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"),
    );
    let launcher = MemoryLauncher::with_exit_code(23);
    let calls = launcher.calls.clone();

    let mut app = App::new(
        store,
        config,
        FixedSelector::new("router-a"),
        launcher,
        || "2026-03-25T14:00:00Z".to_string(),
    );

    let code = app
        .execute(vec!["exec".into(), "--json".into(), "hello".into()])
        .unwrap();

    assert_eq!(code, 23);
    assert_eq!(
        calls.borrow()[0].args[calls.borrow()[0].args.len() - 3..],
        vec![
            "exec".to_string(),
            "--json".to_string(),
            "hello".to_string()
        ]
    );
}

#[test]
fn ui_action_delete_selected_removes_provider() {
    let mut config = AppConfig::default();
    config.providers.insert("router-a".into(), provider());
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(
        dir.path()
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"),
    );

    let mut app = App::new(
        store,
        config,
        FixedSelector::new("router-a"),
        MemoryLauncher::with_exit_code(0),
        || "2026-03-25T14:00:00Z".to_string(),
    );

    let outcome = app
        .execute_ui_action(UiAction::DeleteSelected("router-a".into()), Vec::new())
        .unwrap();

    assert!(matches!(outcome, ProviderPageDispatch::Continue));
    assert!(!app.config().providers.contains_key("router-a"));
}

#[test]
fn ui_action_submit_add_inserts_provider_and_returns_to_loop() {
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(
        dir.path()
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"),
    );
    let mut app = App::new(
        store,
        AppConfig::default(),
        FixedSelector::new("openai"),
        MemoryLauncher::with_exit_code(0),
        || "2026-03-25T14:00:00Z".to_string(),
    );

    let outcome = app
        .execute_ui_action(
            UiAction::SubmitAdd(NewProviderInput {
                name: "openai".into(),
                base_url: "https://api.openai.com/v1".into(),
                api_key: "sk-test".into(),
                model: Some("gpt-5.4".into()),
                notes: Some("Official OpenAI endpoint".into()),
            }),
            Vec::new(),
        )
        .unwrap();

    assert!(matches!(outcome, ProviderPageDispatch::Continue));
    assert!(app.config().providers.contains_key("openai"));
}

#[test]
fn ui_action_submit_edit_updates_provider_and_returns_to_loop() {
    let mut config = AppConfig::default();
    config.providers.insert("router-a".into(), provider());
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(
        dir.path()
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"),
    );

    let mut app = App::new(
        store,
        config,
        FixedSelector::new("router-a"),
        MemoryLauncher::with_exit_code(0),
        || "2026-03-25T14:00:00Z".to_string(),
    );

    let outcome = app
        .execute_ui_action(
            UiAction::SubmitEdit {
                original_name: "router-a".into(),
                input: EditProviderInput {
                    name: None,
                    base_url: Some("https://router.example.com/v1".into()),
                    api_key: Some(String::new()),
                    model: Some("gpt-5.4".into()),
                    notes: Some("updated".into()),
                },
            },
            Vec::new(),
        )
        .unwrap();

    assert!(matches!(outcome, ProviderPageDispatch::Continue));
    assert_eq!(
        app.config().providers["router-a"].notes.as_deref(),
        Some("updated")
    );
}

#[test]
fn ui_action_submit_edit_can_rename_provider_and_returns_to_loop() {
    let mut config = AppConfig::default();
    config.providers.insert("router-a".into(), provider());
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(
        dir.path()
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"),
    );

    let mut app = App::new(
        store,
        config,
        FixedSelector::new("router-a"),
        MemoryLauncher::with_exit_code(0),
        || "2026-03-25T14:00:00Z".to_string(),
    );

    let outcome = app
        .execute_ui_action(
            UiAction::SubmitEdit {
                original_name: "router-a".into(),
                input: EditProviderInput {
                    name: Some("router-b".into()),
                    base_url: Some("https://router.example.com/v1".into()),
                    api_key: Some(String::new()),
                    model: Some("gpt-5.4".into()),
                    notes: Some("renamed".into()),
                },
            },
            Vec::new(),
        )
        .unwrap();

    assert!(matches!(outcome, ProviderPageDispatch::Continue));
    assert!(!app.config().providers.contains_key("router-a"));
    assert_eq!(
        app.config().providers["router-b"].notes.as_deref(),
        Some("renamed")
    );
}

#[test]
fn ui_action_run_selected_launches_provider() {
    let mut config = AppConfig::default();
    config.providers.insert("router-a".into(), provider());
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(
        dir.path()
            .join(".codex")
            .join("codex-nano-manager")
            .join("config.toml"),
    );
    let launcher = MemoryLauncher::with_exit_code(0);
    let calls = launcher.calls.clone();

    let mut app = App::new(
        store,
        config,
        FixedSelector::new("router-a"),
        launcher,
        || "2026-03-25T14:00:00Z".to_string(),
    );

    let outcome = app
        .execute_ui_action(UiAction::RunSelected("router-a".into()), Vec::new())
        .unwrap();

    assert!(matches!(outcome, ProviderPageDispatch::Exit(0)));
    assert_eq!(calls.borrow()[0].env["ROUTER_A_API_KEY"], "secret");
}
