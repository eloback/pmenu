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

fn command_error(program: &str, error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AppError::CommandMissing(program.to_string()),
        _ => AppError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use super::powershell_autofill_script;

    #[test]
    fn powershell_autofill_script_uses_sendkeys_and_restores_clipboard() {
        let script = powershell_autofill_script();
        assert!(script.contains("SendKeys"));
        assert!(script.contains("Get-Clipboard"));
        assert!(script.contains("Set-Clipboard"));
    }
}
