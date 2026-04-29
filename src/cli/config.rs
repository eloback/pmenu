use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::cli::args::CliArgs;
use crate::core::{AppAction, AppError};

const DEFAULT_CLIP_TIME_SECS: u64 = 45;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub store_backend: String,
    pub store_path: Option<PathBuf>,
    pub store_identities_file: Option<PathBuf>,
    pub store_key_file: Option<PathBuf>,
    pub menu_backend: String,
    pub clipboard_backend: String,
    pub autofill_backend: String,
    pub clip_time_secs: u64,
    pub action: AppAction,
    pub field: Option<String>,
    pub notify: bool,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            store_backend: default_store_backend().to_string(),
            store_path: None,
            store_identities_file: None,
            store_key_file: None,
            menu_backend: default_menu_backend().to_string(),
            clipboard_backend: default_clipboard_backend().to_string(),
            autofill_backend: default_autofill_backend().to_string(),
            clip_time_secs: DEFAULT_CLIP_TIME_SECS,
            action: AppAction::Copy,
            field: None,
            notify: true,
        }
    }
}

impl ResolvedConfig {
    pub fn load(args: CliArgs) -> Result<Self, AppError> {
        let config_path = args
            .config
            .as_deref()
            .map(expand_tilde)
            .transpose()?
            .unwrap_or(default_config_path()?);

        let file_config = load_file_config(&config_path, args.config.is_some())?;

        let mut resolved = Self::default();
        resolved.merge_file_config(file_config)?;
        resolved.merge_args(args)?;
        Ok(resolved)
    }

    fn merge_file_config(&mut self, file_config: FileConfig) -> Result<(), AppError> {
        if let Some(store) = file_config.store {
            if let Some(backend) = store.backend {
                self.store_backend = normalize_backend_name(&backend);
            }
            if let Some(path) = store.path {
                self.store_path = Some(expand_tilde(&path)?);
            }
            if let Some(identities_file) = store.identities_file {
                self.store_identities_file = Some(expand_tilde(&identities_file)?);
            }
            if let Some(key_file) = store.key_file {
                self.store_key_file = Some(expand_tilde(&key_file)?);
            }
            if let Some(database_path) = store.database_path {
                self.store_path = Some(expand_tilde(&database_path)?);
            }
        }

        if let Some(menu) = file_config.menu {
            if let Some(backend) = menu.backend {
                self.menu_backend = normalize_backend_name(&backend);
            }
        }

        if let Some(clipboard) = file_config.clipboard {
            if let Some(backend) = clipboard.backend {
                self.clipboard_backend = normalize_backend_name(&backend);
            }
            if let Some(clip_time_secs) = clipboard.clip_time_secs {
                self.clip_time_secs = clip_time_secs;
            }
        }

        if let Some(autofill) = file_config.autofill {
            if let Some(backend) = autofill.backend {
                self.autofill_backend = normalize_backend_name(&backend);
            }
        }

        if let Some(clip_time_secs) = file_config.clip_time_secs {
            self.clip_time_secs = clip_time_secs;
        }

        if let Some(notify) = file_config.notify {
            self.notify = notify;
        }

        Ok(())
    }

    fn merge_args(&mut self, args: CliArgs) -> Result<(), AppError> {
        if let Some(backend) = args.store_backend {
            self.store_backend = normalize_backend_name(&backend);
        }
        if let Some(path) = args.store_path {
            self.store_path = Some(expand_tilde(&path)?);
        }
        if let Some(identities_file) = args.store_identities_file {
            self.store_identities_file = Some(expand_tilde(&identities_file)?);
        }
        if let Some(key_file) = args.store_key_file {
            self.store_key_file = Some(expand_tilde(&key_file)?);
        }
        if let Some(backend) = args.menu_backend {
            self.menu_backend = normalize_backend_name(&backend);
        }
        if let Some(backend) = args.clipboard_backend {
            self.clipboard_backend = normalize_backend_name(&backend);
        }
        if let Some(backend) = args.autofill_backend {
            self.autofill_backend = normalize_backend_name(&backend);
        }
        if let Some(clip_time_secs) = args.clip_time {
            self.clip_time_secs = clip_time_secs;
        }
        if let Some(action) = args.action {
            self.action = action.into();
        }
        if let Some(field) = args.field {
            self.field = Some(field);
        }
        if args.no_notify {
            self.notify = false;
        }

        Ok(())
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    store: Option<StoreConfig>,
    menu: Option<MenuConfig>,
    clipboard: Option<ClipboardConfig>,
    autofill: Option<AutofillConfig>,
    clip_time_secs: Option<u64>,
    notify: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct StoreConfig {
    backend: Option<String>,
    path: Option<String>,
    identities_file: Option<String>,
    key_file: Option<String>,
    database_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct MenuConfig {
    backend: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ClipboardConfig {
    backend: Option<String>,
    clip_time_secs: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct AutofillConfig {
    backend: Option<String>,
}

fn load_file_config(path: &Path, required: bool) -> Result<FileConfig, AppError> {
    if !path.exists() {
        if required {
            return Err(AppError::Config(format!(
                "Config file not found: {}",
                path.display()
            )));
        }
        return Ok(FileConfig::default());
    }

    let raw = fs::read_to_string(path)?;
    toml::from_str(&raw).map_err(|error| {
        AppError::Config(format!(
            "Failed to parse config file {}: {error}",
            path.display()
        ))
    })
}

fn default_config_path() -> Result<PathBuf, AppError> {
    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = env::var_os("APPDATA") {
            return Ok(PathBuf::from(app_data).join("pmenu").join("config.toml"));
        }

        return Ok(home_dir()?
            .join("AppData")
            .join("Roaming")
            .join("pmenu")
            .join("config.toml"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(config_home).join("pmenu").join("config.toml"));
        }

        let home = home_dir()?;
        Ok(home.join(".config").join("pmenu").join("config.toml"))
    }
}

fn expand_tilde(raw: &str) -> Result<PathBuf, AppError> {
    if raw == "~" {
        return home_dir();
    }

    if let Some(stripped) = raw.strip_prefix("~/") {
        return Ok(home_dir()?.join(stripped));
    }

    Ok(PathBuf::from(raw))
}

fn home_dir() -> Result<PathBuf, AppError> {
    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(user_profile) = env::var_os("USERPROFILE") {
            return Ok(PathBuf::from(user_profile));
        }

        let home_drive = env::var_os("HOMEDRIVE");
        let home_path = env::var_os("HOMEPATH");
        if let (Some(home_drive), Some(home_path)) = (home_drive, home_path) {
            return Ok(PathBuf::from(home_drive).join(home_path));
        }

        Err(AppError::Config(
            "`HOME` and Windows home directory variables are not set.".to_string(),
        ))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(AppError::Config("`HOME` is not set.".to_string()))
    }
}

fn normalize_backend_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn default_store_backend() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "keepassxc"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "passage"
    }
}

fn default_menu_backend() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "out-gridview"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "wofi"
    }
}

fn default_clipboard_backend() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "powershell-clipboard"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "wl-clipboard"
    }
}

fn default_autofill_backend() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "powershell-paste"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "wtype"
    }
}

#[cfg(test)]
mod tests {
    use super::{expand_tilde, load_file_config, ResolvedConfig};
    use crate::cli::args::{CliAction, CliArgs};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cli_values_override_file_values() {
        let path = unique_temp_path("pmenu-config.toml");
        fs::write(
            &path,
            r#"
[store]
backend = "pass"
path = "~/.password-store"

[menu]
backend = "fuzzel"
"#,
        )
        .expect("config should be written");

        let config = ResolvedConfig::load(CliArgs {
            config: Some(path.to_string_lossy().into_owned()),
            store_backend: None,
            store_path: None,
            store_identities_file: None,
            store_key_file: None,
            menu_backend: Some("wofi".to_string()),
            clipboard_backend: None,
            autofill_backend: None,
            clip_time: Some(12),
            field: Some("fill".to_string()),
            action: Some(CliAction::Autofill),
            no_notify: true,
            trace: false,
        })
        .expect("config should load");

        assert_eq!(config.store_backend, "pass");
        assert_eq!(config.menu_backend, "wofi");
        assert_eq!(config.clip_time_secs, 12);
        assert_eq!(config.action, crate::core::AppAction::Autofill);
        assert_eq!(config.field.as_deref(), Some("fill"));
        assert!(!config.notify);

        fs::remove_file(path).expect("temp config should be removed");
    }

    #[test]
    fn expands_tilde_paths() {
        let expanded = expand_tilde("~/.config/pmenu/config.toml").expect("path should expand");
        assert!(expanded.is_absolute());
    }

    #[test]
    fn parses_toml_file() {
        let path = unique_temp_path("pmenu-config.toml");
        fs::write(
            &path,
            r#"
[store]
backend = "passage"
identities_file = "~/.passage/identities"

[clipboard]
backend = "wl-clipboard"
"#,
        )
        .expect("config should be written");

        let config = load_file_config(&path, true).expect("config should parse");
        assert!(config.store.is_some());
        assert!(config.clipboard.is_some());

        fs::remove_file(path).expect("temp config should be removed");
    }

    #[test]
    fn parses_keepassxc_store_fields() {
        let path = unique_temp_path("pmenu-keepassxc-config.toml");
        fs::write(
            &path,
            r#"
[store]
backend = "keepassxc"
database_path = "~/Passwords.kdbx"
key_file = "~/Passwords.keyx"
"#,
        )
        .expect("config should be written");

        let config = ResolvedConfig::load(CliArgs {
            config: Some(path.to_string_lossy().into_owned()),
            store_backend: None,
            store_path: None,
            store_identities_file: None,
            store_key_file: None,
            menu_backend: None,
            clipboard_backend: None,
            autofill_backend: None,
            clip_time: None,
            field: None,
            action: None,
            no_notify: false,
            trace: false,
        })
        .expect("config should load");

        assert_eq!(config.store_backend, "keepassxc");
        assert!(config.store_path.is_some());
        assert!(config.store_key_file.is_some());

        fs::remove_file(path).expect("temp config should be removed");
    }

    fn unique_temp_path(file_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("{nanos}-{file_name}"))
    }
}
