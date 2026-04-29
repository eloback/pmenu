use std::process::Command;

use crate::cli::backends::powershell;
use crate::core::{AppError, AutofillBackend};

pub fn build(name: &str) -> Result<Box<dyn AutofillBackend>, AppError> {
    match name {
        "wtype" => Ok(Box::new(WtypeAutofillBackend)),
        "powershell-paste" => Ok(Box::new(PowershellPasteAutofillBackend)),
        _ => Err(AppError::Config(format!(
            "Unknown autofill backend: {name}"
        ))),
    }
}

struct WtypeAutofillBackend;
struct PowershellPasteAutofillBackend;

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

impl AutofillBackend for PowershellPasteAutofillBackend {
    fn autofill(&self, value: &str) -> Result<(), AppError> {
        let script = powershell_autofill_script();
        let mut child = powershell::command(script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(powershell::command_error)?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(value.as_bytes())?;
        }
        drop(child.stdin.take());

        let output = child.wait_with_output()?;
        if output.status.success() {
            return Ok(());
        }

        Err(AppError::CommandFailed {
            command: "powershell SendKeys".to_string(),
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn autofill_login(&self, username: &str, password: &str) -> Result<(), AppError> {
        let script = powershell_autofill_login_script();
        let mut child = powershell::command(script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(powershell::command_error)?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(username.as_bytes())?;
            stdin.write_all(b"\n")?;
            stdin.write_all(password.as_bytes())?;
        }
        drop(child.stdin.take());

        let output = child.wait_with_output()?;
        if output.status.success() {
            return Ok(());
        }

        Err(AppError::CommandFailed {
            command: "powershell SendKeys login".to_string(),
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

fn powershell_autofill_script() -> &'static str {
    "$ErrorActionPreference = 'Stop'; \
     Add-Type -AssemblyName System.Windows.Forms; \
     $value = [Console]::In.ReadToEnd(); \
     $previous = $null; \
     $hasPrevious = $false; \
     try { $previous = Get-Clipboard -Raw; $hasPrevious = $true } catch { } \
     try { \
         Set-Clipboard -Value $value; \
         Start-Sleep -Milliseconds 120; \
         [System.Windows.Forms.SendKeys]::SendWait('^v'); \
         Start-Sleep -Milliseconds 120; \
     } finally { \
         if ($hasPrevious) { \
             Set-Clipboard -Value $previous; \
         } else { \
             Set-Clipboard -Value ''; \
         } \
     }"
}

fn powershell_autofill_login_script() -> &'static str {
    "$ErrorActionPreference = 'Stop'; \
     Add-Type -AssemblyName System.Windows.Forms; \
     $input = [Console]::In.ReadToEnd() -split \"`r?`n\", 2; \
     if ($input.Length -lt 2) { throw 'missing credentials' } \
     $username = $input[0]; \
     $password = $input[1]; \
     $previous = $null; \
     $hasPrevious = $false; \
     try { $previous = Get-Clipboard -Raw; $hasPrevious = $true } catch { } \
     try { \
         Set-Clipboard -Value $username; \
         Start-Sleep -Milliseconds 120; \
         [System.Windows.Forms.SendKeys]::SendWait('^v'); \
         Start-Sleep -Milliseconds 120; \
         [System.Windows.Forms.SendKeys]::SendWait('{TAB}'); \
         Start-Sleep -Milliseconds 120; \
         Set-Clipboard -Value $password; \
         Start-Sleep -Milliseconds 120; \
         [System.Windows.Forms.SendKeys]::SendWait('^v'); \
         Start-Sleep -Milliseconds 120; \
     } finally { \
         if ($hasPrevious) { \
             Set-Clipboard -Value $previous; \
         } else { \
             Set-Clipboard -Value ''; \
         } \
     }"
}

fn wtype_login_commands<'a>(username: &'a str, password: &'a str) -> [(&'static str, Vec<&'a str>); 3] {
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
    use super::{
        powershell_autofill_login_script, powershell_autofill_script,
        redacted_wtype_login_command, wtype_login_commands,
    };

    #[test]
    fn powershell_autofill_script_uses_sendkeys_and_restores_clipboard() {
        let script = powershell_autofill_script();
        assert!(script.contains("SendKeys"));
        assert!(script.contains("Get-Clipboard"));
        assert!(script.contains("Set-Clipboard"));
    }

    #[test]
    fn powershell_autofill_login_script_tabs_between_values() {
        let script = powershell_autofill_login_script();
        assert!(script.contains("{TAB}"));
        assert!(script.contains("$username"));
        assert!(script.contains("$password"));
    }

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
