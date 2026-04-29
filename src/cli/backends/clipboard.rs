use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::backends::powershell;
use crate::core::{AppError, ClipboardBackend};
use tracing::debug;

const CLIPBOARD_STARTUP_GRACE: Duration = Duration::from_millis(150);
const CLIPBOARD_STARTUP_POLL: Duration = Duration::from_millis(10);

pub fn build(name: &str, clip_time_secs: u64) -> Result<Box<dyn ClipboardBackend>, AppError> {
    match name {
        "wl-clipboard" => Ok(Box::new(WlClipboardBackend { clip_time_secs })),
        "xclip" => Ok(Box::new(XclipClipboardBackend { clip_time_secs })),
        "powershell-clipboard" => Ok(Box::new(PowershellClipboardBackend { clip_time_secs })),
        _ => Err(AppError::Config(format!(
            "Unknown clipboard backend: {name}"
        ))),
    }
}

struct WlClipboardBackend {
    clip_time_secs: u64,
}

impl ClipboardBackend for WlClipboardBackend {
    fn copy(&self, value: &str) -> Result<(), AppError> {
        run_copy_command("wl-copy", &[], value)?;
        spawn_clear_process(clear_wayland_command(self.clip_time_secs))
    }
}

struct XclipClipboardBackend {
    clip_time_secs: u64,
}

impl ClipboardBackend for XclipClipboardBackend {
    fn copy(&self, value: &str) -> Result<(), AppError> {
        run_copy_command("xclip", &["-selection", "clipboard"], value)?;
        spawn_clear_process(clear_xclip_command(self.clip_time_secs))
    }
}

struct PowershellClipboardBackend {
    clip_time_secs: u64,
}

impl ClipboardBackend for PowershellClipboardBackend {
    fn copy(&self, value: &str) -> Result<(), AppError> {
        run_powershell_copy_command(value)?;
        spawn_clear_process_command(powershell_clear_command(self.clip_time_secs))
    }
}

fn run_copy_command(program: &str, args: &[&str], value: &str) -> Result<(), AppError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| command_error(program, error))?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(value.as_bytes())?;
    }
    drop(child.stdin.take());

    if let Some(exit) = wait_for_startup_exit(&mut child)? {
        debug!(
            program,
            success = exit.status.success(),
            code = exit.status.code(),
            "clipboard command exited during startup grace period"
        );
        if exit.status.success() {
            return Ok(());
        }

        return Err(AppError::CommandFailed {
            command: format!("{program} {}", args.join(" ")).trim().to_string(),
            code: exit.status.code(),
            stderr: String::new(),
        });
    }

    debug!(
        program,
        grace_ms = CLIPBOARD_STARTUP_GRACE.as_millis(),
        "clipboard command still running after startup grace period; treating copy as active"
    );
    Ok(())
}

fn spawn_clear_process(command: Vec<String>) -> Result<(), AppError> {
    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(AppError::from)
}

fn spawn_clear_process_command(mut command: Command) -> Result<(), AppError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(AppError::from)
}

fn clear_wayland_command(clip_time_secs: u64) -> Vec<String> {
    vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("sleep {clip_time_secs}; wl-copy --clear"),
    ]
}

fn clear_xclip_command(clip_time_secs: u64) -> Vec<String> {
    vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("sleep {clip_time_secs}; printf '' | xclip -selection clipboard"),
    ]
}

fn run_powershell_copy_command(value: &str) -> Result<(), AppError> {
    let script = "$value = [Console]::In.ReadToEnd(); Set-Clipboard -Value $value";
    let mut child = powershell::command(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(powershell::command_error)?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(value.as_bytes())?;
    }
    drop(child.stdin.take());

    if let Some(exit) = wait_for_startup_exit(&mut child)? {
        debug!(
            program = "powershell",
            success = exit.status.success(),
            code = exit.status.code(),
            "clipboard command exited during startup grace period"
        );
        if exit.status.success() {
            return Ok(());
        }

        return Err(AppError::CommandFailed {
            command: "powershell Set-Clipboard".to_string(),
            code: exit.status.code(),
            stderr: String::new(),
        });
    }

    debug!(
        program = "powershell",
        grace_ms = CLIPBOARD_STARTUP_GRACE.as_millis(),
        "clipboard command still running after startup grace period; treating copy as active"
    );
    Ok(())
}

fn powershell_clear_command(clip_time_secs: u64) -> Command {
    powershell::command(&powershell_clear_script(clip_time_secs))
}

fn powershell_clear_script(clip_time_secs: u64) -> String {
    format!("Start-Sleep -Seconds {clip_time_secs}; Set-Clipboard -Value ''")
}

fn wait_for_startup_exit(child: &mut Child) -> Result<Option<StartupExit>, AppError> {
    let deadline = Instant::now() + CLIPBOARD_STARTUP_GRACE;

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(StartupExit { status }));
        }

        if Instant::now() >= deadline {
            return Ok(None);
        }

        thread::sleep(CLIPBOARD_STARTUP_POLL);
    }
}

struct StartupExit {
    status: ExitStatus,
}

fn command_error(program: &str, error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AppError::CommandMissing(program.to_string()),
        _ => AppError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{clear_wayland_command, clear_xclip_command, powershell_clear_script};

    #[test]
    fn clear_commands_include_timeout() {
        assert_eq!(
            clear_wayland_command(12),
            vec!["sh", "-c", "sleep 12; wl-copy --clear"]
        );
        assert_eq!(
            clear_xclip_command(12),
            vec![
                "sh",
                "-c",
                "sleep 12; printf '' | xclip -selection clipboard"
            ]
        );
        assert_eq!(
            powershell_clear_script(12),
            "Start-Sleep -Seconds 12; Set-Clipboard -Value ''"
        );
    }
}
