use std::process::Command;

use crate::core::{AppError, AutofillBackend};

pub fn build(name: &str) -> Result<Box<dyn AutofillBackend>, AppError> {
    match name {
        "wtype" => Ok(Box::new(WtypeAutofillBackend)),
        _ => Err(AppError::Config(format!("Unknown autofill backend: {name}"))),
    }
}

struct WtypeAutofillBackend;

impl AutofillBackend for WtypeAutofillBackend {
    fn autofill(&self, value: &str) -> Result<(), AppError> {
        let output = Command::new("wtype")
            .arg(value)
            .output()
            .map_err(|error| command_error("wtype", error))?;

        if output.status.success() {
            return Ok(());
        }

        Err(AppError::CommandFailed {
            command: "wtype <value>".to_string(),
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

fn command_error(program: &str, error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AppError::CommandMissing(program.to_string()),
        _ => AppError::Io(error),
    }
}
