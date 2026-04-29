#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};

#[cfg(target_os = "linux")]
use serde_json::Value;

pub fn initial_query() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        linux_initial_query()
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn linux_initial_query() -> Option<String> {
    let class = active_window_class()?;
    match class.to_ascii_lowercase().as_str() {
        "org.qutebrowser.qutebrowser" | "qutebrowser" => qutebrowser_query(),
        "discord" => Some("discord.com".to_string()),
        "steam" => Some("steampowered.com".to_string()),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn active_window_class() -> Option<String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok()?;
    let desktop = desktop.to_ascii_lowercase();

    if desktop.contains("hyprland") {
        let output = Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        let value: Value = serde_json::from_slice(&output.stdout).ok()?;
        return value
            .get("class")
            .and_then(Value::as_str)
            .map(ToString::to_string);
    }

    if desktop.contains("sway") {
        let output = Command::new("swaymsg")
            .args(["-t", "get_tree"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        let value: Value = serde_json::from_slice(&output.stdout).ok()?;
        return focused_sway_class(&value);
    }

    None
}

#[cfg(target_os = "linux")]
fn focused_sway_class(value: &Value) -> Option<String> {
    if value.get("focused").and_then(Value::as_bool) == Some(true) {
        if let Some(class) = value
            .get("window_properties")
            .and_then(|props| props.get("class"))
            .and_then(Value::as_str)
        {
            return Some(class.to_string());
        }

        if let Some(app_id) = value.get("app_id").and_then(Value::as_str) {
            return Some(app_id.to_string());
        }
    }

    for key in ["nodes", "floating_nodes"] {
        let Some(children) = value.get(key).and_then(Value::as_array) else {
            continue;
        };

        for child in children {
            if let Some(class) = focused_sway_class(child) {
                return Some(class);
            }
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn qutebrowser_query() -> Option<String> {
    qutebrowser_query_with(
        snapshot_wayland_clipboard,
        send_qutebrowser_yank,
        read_wayland_clipboard,
        restore_wayland_clipboard,
    )
}

#[cfg(target_os = "linux")]
fn qutebrowser_query_with<Snapshot, Yank, Read, Restore>(
    snapshot_clipboard: Snapshot,
    yank_url: Yank,
    read_clipboard: Read,
    restore_clipboard: Restore,
) -> Option<String>
where
    Snapshot: FnOnce() -> Option<ClipboardSnapshot>,
    Yank: FnOnce() -> bool,
    Read: FnOnce() -> Option<Vec<u8>>,
    Restore: FnOnce(&ClipboardSnapshot) -> bool,
{
    let snapshot = snapshot_clipboard()?;
    let raw_url = if yank_url() { read_clipboard() } else { None };

    if !restore_clipboard(&snapshot) {
        return None;
    }

    let raw_url = raw_url?;
    host_query(&String::from_utf8_lossy(&raw_url))
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq, Eq)]
enum ClipboardSnapshot {
    Empty,
    Text {
        mime_type: String,
        contents: Vec<u8>,
    },
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq, Eq)]
enum ClipboardSnapshotKind {
    Empty,
    Text(String),
}

#[cfg(target_os = "linux")]
fn snapshot_wayland_clipboard() -> Option<ClipboardSnapshot> {
    let output = Command::new("wl-paste")
        .arg("--list-types")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    match clipboard_snapshot_kind(&String::from_utf8_lossy(&output.stdout))? {
        ClipboardSnapshotKind::Empty => Some(ClipboardSnapshot::Empty),
        ClipboardSnapshotKind::Text(mime_type) => Some(ClipboardSnapshot::Text {
            contents: read_wayland_clipboard_with_type(&mime_type)?,
            mime_type,
        }),
    }
}

#[cfg(target_os = "linux")]
fn clipboard_snapshot_kind(raw_types: &str) -> Option<ClipboardSnapshotKind> {
    let mime_types: Vec<&str> = raw_types.lines().map(str::trim).filter(|line| !line.is_empty()).collect();

    if mime_types.is_empty() {
        return Some(ClipboardSnapshotKind::Empty);
    }

    for preferred in ["text/plain;charset=utf-8", "text/plain"] {
        if mime_types.contains(&preferred) {
            return Some(ClipboardSnapshotKind::Text(preferred.to_string()));
        }
    }

    mime_types
        .iter()
        .find(|mime_type| mime_type.starts_with("text/"))
        .map(|mime_type| ClipboardSnapshotKind::Text((*mime_type).to_string()))
}

#[cfg(target_os = "linux")]
fn send_qutebrowser_yank() -> bool {
    Command::new("qutebrowser")
        .arg(":yank")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "linux")]
fn read_wayland_clipboard() -> Option<Vec<u8>> {
    read_wayland_clipboard_with_type("text/plain;charset=utf-8")
        .or_else(|| read_wayland_clipboard_with_type("text/plain"))
        .or_else(|| read_wayland_clipboard_default())
}

#[cfg(target_os = "linux")]
fn read_wayland_clipboard_default() -> Option<Vec<u8>> {
    let output = Command::new("wl-paste")
        .arg("--no-newline")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    Some(output.stdout)
}

#[cfg(target_os = "linux")]
fn read_wayland_clipboard_with_type(mime_type: &str) -> Option<Vec<u8>> {
    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", mime_type])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    Some(output.stdout)
}

#[cfg(target_os = "linux")]
fn restore_wayland_clipboard(snapshot: &ClipboardSnapshot) -> bool {
    match snapshot {
        ClipboardSnapshot::Empty => Command::new("wl-copy")
            .arg("--clear")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success()),
        ClipboardSnapshot::Text {
            mime_type,
            contents,
        } => write_wayland_clipboard(mime_type, contents),
    }
}

#[cfg(target_os = "linux")]
fn write_wayland_clipboard(mime_type: &str, contents: &[u8]) -> bool {
    let mut child = match Command::new("wl-copy")
        .args(["--type", mime_type])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        if stdin.write_all(contents).is_err() {
            return false;
        }
    }
    drop(child.stdin.take());

    child.wait().is_ok_and(|status| status.success())
}

#[cfg(target_os = "linux")]
fn host_query(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let no_scheme = raw.split_once("://").map(|(_, value)| value).unwrap_or(raw);
    let host = no_scheme.split('/').next().unwrap_or(no_scheme);
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    let host = host.trim_start_matches("www.");

    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::{
        clipboard_snapshot_kind, focused_sway_class, host_query, qutebrowser_query_with,
        ClipboardSnapshot, ClipboardSnapshotKind,
    };
    #[cfg(target_os = "linux")]
    use serde_json::json;
    #[cfg(target_os = "linux")]
    use std::cell::Cell;

    #[cfg(target_os = "linux")]
    #[test]
    fn extracts_host_query_from_url() {
        assert_eq!(
            host_query("https://www.example.com/login?q=1"),
            Some("example.com".to_string())
        );
        assert_eq!(
            host_query("example.org/path"),
            Some("example.org".to_string())
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn finds_focused_sway_window_class() {
        let tree = json!({
            "focused": false,
            "nodes": [
                {
                    "focused": true,
                    "window_properties": {
                        "class": "qutebrowser"
                    }
                }
            ]
        });

        assert_eq!(focused_sway_class(&tree), Some("qutebrowser".to_string()));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detects_empty_wayland_clipboard_snapshot() {
        assert_eq!(
            clipboard_snapshot_kind(""),
            Some(ClipboardSnapshotKind::Empty)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detects_text_wayland_clipboard_snapshot() {
        assert_eq!(
            clipboard_snapshot_kind("image/png\ntext/plain;charset=utf-8\n"),
            Some(ClipboardSnapshotKind::Text(
                "text/plain;charset=utf-8".to_string()
            ))
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn prefers_plain_text_over_other_text_types() {
        assert_eq!(
            clipboard_snapshot_kind("text/html\ntext/plain\n"),
            Some(ClipboardSnapshotKind::Text("text/plain".to_string()))
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn qutebrowser_query_skips_probe_when_clipboard_snapshot_fails() {
        let yanked = Cell::new(false);
        let restored = Cell::new(false);

        let result = qutebrowser_query_with(
            || None,
            || {
                yanked.set(true);
                true
            },
            || Some(b"https://example.com".to_vec()),
            |_| {
                restored.set(true);
                true
            },
        );

        assert_eq!(result, None);
        assert!(!yanked.get());
        assert!(!restored.get());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn qutebrowser_query_restores_empty_clipboard_before_returning_url() {
        let restored = Cell::new(false);

        let result = qutebrowser_query_with(
            || Some(ClipboardSnapshot::Empty),
            || true,
            || Some(b"https://www.example.com/login".to_vec()),
            |_| {
                restored.set(true);
                true
            },
        );

        assert_eq!(result, Some("example.com".to_string()));
        assert!(restored.get());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn qutebrowser_query_restores_text_clipboard_before_returning_url() {
        let restored = Cell::new(false);

        let result = qutebrowser_query_with(
            || {
                Some(ClipboardSnapshot::Text {
                    mime_type: "text/plain;charset=utf-8".to_string(),
                    contents: b"keep me".to_vec(),
                })
            },
            || true,
            || Some(b"https://example.org/account".to_vec()),
            |_| {
                restored.set(true);
                true
            },
        );

        assert_eq!(result, Some("example.org".to_string()));
        assert!(restored.get());
    }
}
