use std::process::Command;

use crate::core::{AppError, AutofillBackend};

pub fn build(name: &str) -> Result<Box<dyn AutofillBackend>, AppError> {
    match name {
        "wtype" => Ok(Box::new(WtypeAutofillBackend)),
        _ => Err(AppError::Config(format!(
            "Unknown autofill backend: {name}"
        ))),
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

    fn autofill_login(&self, username: &str, password: &str) -> Result<(), AppError> {
        for (program, args) in wtype_login_commands(username, password) {
            let command = redacted_wtype_login_command(program, &args);
            let output = Command::new(program)
                .args(&args)
                .output()
                .map_err(|error| command_error(program, error))?;

            if !output.status.success() {
                return Err(AppError::CommandFailed {
                    command,
                    code: output.status.code(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
        }

        Ok(())
    }
}

fn wtype_login_commands<'a>(
    username: &'a str,
    password: &'a str,
) -> [(&'static str, Vec<&'a str>); 3] {
    [
        ("wtype", vec![username]),
        ("wtype", vec!["-k", "tab"]),
        ("wtype", vec![password]),
    ]
}

fn redacted_wtype_login_command(program: &str, args: &[&str]) -> String {
    match args {
        ["-k", "tab"] => format!("{program} -k tab"),
        _ => format!("{program} <value>"),
    }
}

fn command_error(program: &str, error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AppError::CommandMissing(program.to_string()),
        _ => AppError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{redacted_wtype_login_command, wtype_login_commands};

    #[test]
    fn wtype_login_commands_type_username_tab_password() {
        let commands = wtype_login_commands("demo", "secret");
        assert_eq!(commands[0], ("wtype", vec!["demo"]));
        assert_eq!(commands[1], ("wtype", vec!["-k", "tab"]));
        assert_eq!(commands[2], ("wtype", vec!["secret"]));
    }

    #[test]
    fn redacts_wtype_login_command_values() {
        assert_eq!(
            redacted_wtype_login_command("wtype", &["demo"]),
            "wtype <value>"
        );
        assert_eq!(
            redacted_wtype_login_command("wtype", &["secret"]),
            "wtype <value>"
        );
        assert_eq!(
            redacted_wtype_login_command("wtype", &["-k", "tab"]),
            "wtype -k tab"
        );
    }
}
