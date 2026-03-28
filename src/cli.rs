use crate::error::AppError;
use clap::{Command, CommandFactory, Parser};
use std::ffi::{OsStr, OsString};

const HELP_VERSION_EXTRA_ARGS_MESSAGE: &str =
    "manager help/version does not accept extra args; use '+' to forward them to codex";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliDispatch {
    ManagerEntry,
    ManagerHelp,
    ManagerVersion,
    Passthrough(Vec<String>),
}

#[derive(Debug, Parser)]
#[command(
    name = "codex-nano-manager",
    version,
    about = "Select a provider and launch codex with managed endpoint settings."
)]
struct ManagerCli;

pub fn manager_command() -> Command {
    ManagerCli::command()
        .after_help("Use '+' as the first argument to force passthrough to codex.")
}

pub fn classify_args<I, T>(args: I) -> Result<CliDispatch, AppError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut argv = args.into_iter().map(Into::into);
    let _program = argv.next();
    let tail: Vec<OsString> = argv.collect();

    let Some(first) = tail.first() else {
        return Ok(CliDispatch::ManagerEntry);
    };

    if first == OsStr::new("+") {
        return Ok(CliDispatch::Passthrough(os_strings_to_strings(
            tail.into_iter().skip(1),
        )?));
    }

    if is_help_token(first) {
        return classify_manager_command(CliDispatch::ManagerHelp, tail.len());
    }

    if is_version_token(first) {
        return classify_manager_command(CliDispatch::ManagerVersion, tail.len());
    }

    Ok(CliDispatch::Passthrough(os_strings_to_strings(tail.into_iter())?))
}

fn classify_manager_command(dispatch: CliDispatch, arg_count: usize) -> Result<CliDispatch, AppError> {
    if arg_count == 1 {
        Ok(dispatch)
    } else {
        Err(AppError::validation(HELP_VERSION_EXTRA_ARGS_MESSAGE.to_string()))
    }
}

fn is_help_token(arg: &OsStr) -> bool {
    matches!(arg.to_str(), Some("help" | "--help" | "-h"))
}

fn is_version_token(arg: &OsStr) -> bool {
    matches!(arg.to_str(), Some("version" | "--version" | "-V"))
}

fn os_strings_to_strings<I>(args: I) -> Result<Vec<String>, AppError>
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter().map(os_string_to_string).collect()
}

fn os_string_to_string(arg: OsString) -> Result<String, AppError> {
    arg.into_string()
        .map_err(|_| AppError::validation("arguments must be valid UTF-8".to_string()))
}

#[cfg(test)]
mod tests {
    use super::{CliDispatch, classify_args, manager_command};

    #[test]
    fn classify_args_without_extra_tokens_enters_manager_mode() {
        assert!(matches!(
            classify_args(["codex-nano-manager"]).unwrap(),
            CliDispatch::ManagerEntry
        ));
    }

    #[test]
    fn classify_args_recognizes_manager_help_and_version_tokens() {
        assert!(matches!(
            classify_args(["codex-nano-manager", "--help"]).unwrap(),
            CliDispatch::ManagerHelp
        ));
        assert!(matches!(
            classify_args(["codex-nano-manager", "-h"]).unwrap(),
            CliDispatch::ManagerHelp
        ));
        assert!(matches!(
            classify_args(["codex-nano-manager", "version"]).unwrap(),
            CliDispatch::ManagerVersion
        ));
        assert!(matches!(
            classify_args(["codex-nano-manager", "-V"]).unwrap(),
            CliDispatch::ManagerVersion
        ));
    }

    #[test]
    fn classify_args_preserves_default_passthrough_and_forced_passthrough() {
        assert_eq!(
            classify_args(["codex-nano-manager", "exec", "hello"]).unwrap(),
            CliDispatch::Passthrough(vec!["exec".to_string(), "hello".to_string()])
        );
        assert_eq!(
            classify_args(["codex-nano-manager", "+", "--version"]).unwrap(),
            CliDispatch::Passthrough(vec!["--version".to_string()])
        );
    }

    #[test]
    fn classify_args_rejects_extra_tokens_after_manager_help_or_version() {
        let help_err = classify_args(["codex-nano-manager", "--help", "exec"]).unwrap_err();
        let version_err =
            classify_args(["codex-nano-manager", "version", "--json"]).unwrap_err();

        assert_eq!(
            help_err.to_string(),
            "manager help/version does not accept extra args; use '+' to forward them to codex"
        );
        assert_eq!(
            version_err.to_string(),
            "manager help/version does not accept extra args; use '+' to forward them to codex"
        );
    }

    #[test]
    fn command_name_matches_codex_nano_manager() {
        let command = manager_command();

        assert_eq!(command.get_name(), "codex-nano-manager");
    }
}
