use std::process::ExitCode;

fn main() -> ExitCode {
    match codex_nano_manager::run(std::env::args_os()) {
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}
