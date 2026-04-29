use std::process::Command;

use crate::core::AppError;

pub fn command(script: &str) -> Command {
    let mut command = Command::new("powershell");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        script,
    ]);
    command
}

pub fn command_error(error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AppError::CommandMissing("powershell".to_string()),
        _ => AppError::Io(error),
    }
}

pub fn escape_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}
