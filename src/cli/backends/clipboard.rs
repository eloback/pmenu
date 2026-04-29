use std::process::{Command, Stdio};

use crate::core::{AppError, ClipboardBackend};

pub fn build(name: &str, clip_time_secs: u64) -> Result<Box<dyn ClipboardBackend>, AppError> {
    match name {
        "wl-clipboard" => Ok(Box::new(WlClipboardBackend { clip_time_secs })),
        "xclip" => Ok(Box::new(XclipClipboardBackend { clip_time_secs })),
        _ => Err(AppError::Config(format!("Unknown clipboard backend: {name}"))),
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

fn run_copy_command(program: &str, args: &[&str], value: &str) -> Result<(), AppError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| command_error(program, error))?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(value.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(AppError::CommandFailed {
        command: format!("{program} {}", args.join(" ")).trim().to_string(),
        code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
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

fn command_error(program: &str, error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AppError::CommandMissing(program.to_string()),
        _ => AppError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{clear_wayland_command, clear_xclip_command};

    #[test]
    fn clear_commands_include_timeout() {
        assert_eq!(
            clear_wayland_command(12),
            vec!["sh", "-c", "sleep 12; wl-copy --clear"]
        );
        assert_eq!(
            clear_xclip_command(12),
            vec!["sh", "-c", "sleep 12; printf '' | xclip -selection clipboard"]
        );
    }
}
