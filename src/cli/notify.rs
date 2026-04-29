use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct Notifier {
    enabled: bool,
}

impl Notifier {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn notify(&self, verb: &str, field: &str) {
        if !self.enabled {
            return;
        }

        #[cfg(target_os = "windows")]
        {
            let _ = (verb, field);
            return;
        }

        #[cfg(not(target_os = "windows"))]
        let _ = Command::new("notify-send")
            .args(["Pmenu", &format!("󰌆 {verb} {field}!"), "-t", "2000"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
