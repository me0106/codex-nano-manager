pub mod cli;
pub mod config;
pub mod error;
pub mod launcher;
pub mod provider;
pub mod provider_templates;
pub mod selector;
pub mod ui;

use cli::{CliDispatch, classify_args, manager_command};
use config::{AppConfig, ConfigStore, ProviderConfig};
use error::AppError;
use launcher::{CodexLauncher, ProcessLauncher, build_launch_request};
use provider::{apply_edit, insert_provider, remove_provider};
use selector::{ProviderSelector, RatatuiSelector};
use std::io::{IsTerminal, Write};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use ui::action::UiAction;
use ui::UiSession;
use ui::state::{UiMode, UiState};

pub enum ProviderPageDispatch {
    Continue,
    Exit(i32),
}

pub struct App<S, L> {
    store: ConfigStore,
    config: AppConfig,
    selector: S,
    launcher: L,
    now: fn() -> String,
}

impl<S, L> App<S, L>
where
    S: ProviderSelector,
    L: CodexLauncher,
{
    pub fn new(
        store: ConfigStore,
        config: AppConfig,
        selector: S,
        launcher: L,
        now: fn() -> String,
    ) -> Self {
        Self {
            store,
            config,
            selector,
            launcher,
            now,
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn execute(&mut self, passthrough_args: Vec<String>) -> Result<i32, AppError> {
        if std::io::stdout().is_terminal() {
            self.execute_ui_loop(passthrough_args)
        } else {
            self.launch_selected(passthrough_args)
        }
    }

    fn execute_launch_selected(
        &mut self,
        name: String,
        passthrough_args: Vec<String>,
    ) -> Result<i32, AppError> {
        let request = {
            let provider = self
                .config
                .providers
                .get(&name)
                .cloned()
                .ok_or_else(|| AppError::provider_not_found(&name))?;
            build_launch_request(&provider, passthrough_args)
        };

        let exit_code = self.launcher.launch(request)?;
        if let Some(provider) = self.config.providers.get_mut(&name) {
            provider.last_used_at = Some((self.now)());
        }
        self.store.save(&self.config)?;
        Ok(exit_code)
    }

    fn execute_ui_loop(&mut self, passthrough_args: Vec<String>) -> Result<i32, AppError> {
        let mut session = UiSession::new(UiState::new(UiMode::Run, self.providers_for_ui()))?;
        loop {
            let action = session.next_action()?;
            let clear_viewport_on_exit = matches!(action, UiAction::Quit);
            if matches!(action, UiAction::RunSelected(_) | UiAction::ExecSelected(_)) {
                session.prepare_for_launch()?;
            }
            match self.execute_ui_action(action, passthrough_args.clone())? {
                ProviderPageDispatch::Continue => {
                    session.replace_state(UiState::new(UiMode::Run, self.providers_for_ui()))?;
                }
                ProviderPageDispatch::Exit(code) => {
                    if clear_viewport_on_exit {
                        session.clear_viewport()?;
                    }
                    return Ok(code);
                }
            }
        }
    }

    fn providers_for_ui(&self) -> Vec<ProviderConfig> {
        self.config.providers.values().cloned().collect()
    }

    pub fn execute_ui_action(
        &mut self,
        action: UiAction,
        passthrough_args: Vec<String>,
    ) -> Result<ProviderPageDispatch, AppError> {
        match action {
            UiAction::Continue => Ok(ProviderPageDispatch::Continue),
            UiAction::Quit => Ok(ProviderPageDispatch::Exit(0)),
            UiAction::SubmitAdd(input) => {
                insert_provider(&mut self.config, input)?;
                self.store.save(&self.config)?;
                Ok(ProviderPageDispatch::Continue)
            }
            UiAction::SubmitEdit {
                original_name,
                input,
            } => {
                let existing = self
                    .config
                    .providers
                    .get(&original_name)
                    .cloned()
                    .ok_or_else(|| AppError::provider_not_found(&original_name))?;
                let updated = apply_edit(&existing, input)?;
                if updated.name != original_name
                    && self.config.providers.contains_key(&updated.name)
                {
                    return Err(AppError::validation(format!(
                        "provider '{}' already exists",
                        updated.name
                    )));
                }
                self.config.providers.remove(&original_name);
                self.config.providers.insert(updated.name.clone(), updated);
                self.store.save(&self.config)?;
                Ok(ProviderPageDispatch::Continue)
            }
            UiAction::DeleteSelected(name) => {
                remove_provider(&mut self.config, &name)?;
                self.store.save(&self.config)?;
                Ok(ProviderPageDispatch::Continue)
            }
            UiAction::RunSelected(name) | UiAction::ExecSelected(name) => self
                .execute_launch_selected(name, passthrough_args)
                .map(ProviderPageDispatch::Exit),
        }
    }

    fn launch_selected(
        &mut self,
        passthrough_args: Vec<String>,
    ) -> Result<i32, AppError> {
        let providers: Vec<ProviderConfig> = self.config.providers.values().cloned().collect();
        if providers.is_empty() {
            return Err(AppError::NoProvidersConfigured);
        }

        let selected = self.selector.select(&providers)?;
        let request = {
            let provider = self
                .config
                .providers
                .get(&selected)
                .cloned()
                .ok_or_else(|| AppError::provider_not_found(&selected))?;
            build_launch_request(&provider, passthrough_args)
        };

        let exit_code = self.launcher.launch(request)?;

        if let Some(provider) = self.config.providers.get_mut(&selected) {
            provider.last_used_at = Some((self.now)());
        }
        self.store.save(&self.config)?;

        Ok(exit_code)
    }
}

pub fn run<I, T>(args: I) -> Result<i32, AppError>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString>,
{
    match classify_args(args)? {
        CliDispatch::ManagerEntry => run_manager(Vec::new()),
        CliDispatch::ManagerHelp => {
            let mut command = manager_command();
            command.print_help()?;
            std::io::stdout().write_all(b"\n")?;
            Ok(0)
        }
        CliDispatch::ManagerVersion => {
            let version = manager_command().render_version().to_string();
            std::io::stdout().write_all(version.as_bytes())?;
            Ok(0)
        }
        CliDispatch::Passthrough(args) => run_manager(args),
    }
}

fn run_manager(passthrough_args: Vec<String>) -> Result<i32, AppError> {
    let store = ConfigStore::new(ConfigStore::default_path()?);
    let config = store.load()?;
    let mut app = App::new(
        store,
        config,
        RatatuiSelector,
        ProcessLauncher,
        || OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
    );

    app.execute(passthrough_args)
}
