use crate::config::ProviderConfig;
use crate::error::AppError;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub trait CodexLauncher {
    fn launch(&mut self, request: LaunchRequest) -> Result<i32, AppError>;
}

pub struct ProcessLauncher;

impl CodexLauncher for ProcessLauncher {
    fn launch(&mut self, request: LaunchRequest) -> Result<i32, AppError> {
        let status = std::process::Command::new(&request.program)
            .args(&request.args)
            .envs(&request.env)
            .status()?;

        Ok(status.code().unwrap_or(1))
    }
}

pub fn build_launch_request(
    provider: &ProviderConfig,
    passthrough_args: Vec<String>,
) -> LaunchRequest {
    let mut args = Vec::new();

    args.push("-c".to_string());
    args.push(r#"model_provider="OpenAI""#.to_string());
    args.push("-c".to_string());
    args.push(format!(
        r#"model_providers.OpenAI.base_url="{}""#,
        provider.base_url
    ));
    args.push("-c".to_string());
    args.push(format!(
        r#"model_providers.OpenAI.env_key="{}""#,
        provider.env_key
    ));

    let user_specified_model = passthrough_args
        .iter()
        .any(|arg| arg == "-m" || arg == "--model" || arg.starts_with("--model="));

    if !user_specified_model {
        if let Some(model) = &provider.model {
            args.push("-m".to_string());
            args.push(model.clone());
        }
    }

    args.extend(passthrough_args);

    let mut env = BTreeMap::new();
    env.insert(provider.env_key.clone(), provider.api_key.clone());

    LaunchRequest {
        program: "codex".to_string(),
        args,
        env,
    }
}

#[cfg(test)]
mod tests {
    use super::build_launch_request;
    use crate::config::ProviderConfig;

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

    #[test]
    fn build_launch_request_includes_env_key_and_base_url() {
        let request = build_launch_request(&provider(), vec!["--full-auto".into()]);

        assert_eq!(request.program, "codex");
        assert_eq!(request.env["ROUTER_A_API_KEY"], "secret");
        assert!(request.args.contains(&"-c".to_string()));
        assert!(
            request
                .args
                .iter()
                .any(|arg| arg.contains("model_providers.OpenAI.base_url"))
        );
    }

    #[test]
    fn default_model_is_skipped_when_user_already_passed_one() {
        let request = build_launch_request(&provider(), vec!["-m".into(), "gpt-4.1".into()]);

        assert_eq!(
            request
                .args
                .iter()
                .filter(|arg| arg.as_str() == "-m")
                .count(),
            1
        );
    }

    #[test]
    fn build_launch_request_forwards_passthrough_args_unchanged() {
        let request = build_launch_request(
            &provider(),
            vec!["exec".into(), "--json".into(), "hello".into()],
        );

        assert_eq!(
            request.args[request.args.len() - 3..],
            vec![
                "exec".to_string(),
                "--json".to_string(),
                "hello".to_string()
            ]
        );
    }
}
