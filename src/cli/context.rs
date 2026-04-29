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
    let _ = Command::new("qutebrowser")
        .arg(":yank")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let output = Command::new("wl-paste")
        .arg("--no-newline")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    host_query(&String::from_utf8_lossy(&output.stdout))
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
    use super::{focused_sway_class, host_query};
    #[cfg(target_os = "linux")]
    use serde_json::json;

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
}
