use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::{parse_entry_content, AppError, EntryContent, PasswordStoreBackend};
use tracing::{debug, trace};

pub fn build(
    name: &str,
    store_path: Option<PathBuf>,
    identities_file: Option<PathBuf>,
) -> Result<Box<dyn PasswordStoreBackend>, AppError> {
    match name {
        "pass" => Ok(Box::new(PassStore::new(store_path)?)),
        "passage" => Ok(Box::new(PassageStore::new(store_path, identities_file)?)),
        _ => Err(AppError::Config(format!("Unknown store backend: {name}"))),
    }
}

struct PassStore {
    store_dir: PathBuf,
}

impl PassStore {
    fn new(store_path: Option<PathBuf>) -> Result<Self, AppError> {
        let store_dir = store_path.unwrap_or(home_subdir(".password-store")?);
        Ok(Self {
            store_dir: require_existing_store_dir(store_dir)?,
        })
    }
}

impl PasswordStoreBackend for PassStore {
    fn list_entries(&self) -> Result<Vec<String>, AppError> {
        trace!(store_dir = %self.store_dir.display(), "listing pass entries");
        list_entries_with_extension(&self.store_dir, ".gpg")
    }

    fn show_entry(&self, entry: &str) -> Result<EntryContent, AppError> {
        let args = pass_show_args(entry);
        debug!(entry, store_dir = %self.store_dir.display(), "running pass show");

        let output = base_pass_command(&self.store_dir)
            .args(&args)
            .output()
            .map_err(|error| command_error("pass", error))?;
        debug!(
            entry,
            success = output.status.success(),
            code = output.status.code(),
            stdout_len = output.stdout.len(),
            stderr_len = output.stderr.len(),
            "pass show finished"
        );

        if !output.status.success() {
            return Err(AppError::CommandFailed {
                command: format!("pass {}", args.join(" ")),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        parse_entry_content(&String::from_utf8_lossy(&output.stdout))
    }
}

struct PassageStore {
    store_dir: PathBuf,
    identities_file: Option<PathBuf>,
}

impl PassageStore {
    fn new(
        store_path: Option<PathBuf>,
        identities_file: Option<PathBuf>,
    ) -> Result<Self, AppError> {
        let store_dir = store_path.unwrap_or(home_subdir(".passage/store")?);
        Ok(Self {
            store_dir: require_existing_store_dir(store_dir)?,
            identities_file,
        })
    }
}

impl PasswordStoreBackend for PassageStore {
    fn list_entries(&self) -> Result<Vec<String>, AppError> {
        trace!(store_dir = %self.store_dir.display(), "listing passage entries");
        list_entries_with_extension(&self.store_dir, ".age")
    }

    fn show_entry(&self, entry: &str) -> Result<EntryContent, AppError> {
        let mut command = base_passage_command(&self.store_dir);
        if let Some(identities_file) = &self.identities_file {
            command.env("PASSAGE_IDENTITIES_FILE", identities_file);
        }
        let args = passage_show_args(entry);
        debug!(
            entry,
            store_dir = %self.store_dir.display(),
            identities_file = self.identities_file.as_ref().map(|path| path.display().to_string()),
            "running passage show"
        );

        let output = command
            .args(&args)
            .output()
            .map_err(|error| command_error("passage", error))?;
        debug!(
            entry,
            success = output.status.success(),
            code = output.status.code(),
            stdout_len = output.stdout.len(),
            stderr_len = output.stderr.len(),
            "passage show finished"
        );

        if !output.status.success() {
            return Err(AppError::CommandFailed {
                command: format!("passage {}", args.join(" ")),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        parse_entry_content(&String::from_utf8_lossy(&output.stdout))
    }
}

fn list_entries_with_extension(store_dir: &Path, extension: &str) -> Result<Vec<String>, AppError> {
    let mut entries = Vec::new();
    collect_entries(store_dir, store_dir, extension, &mut entries)?;
    entries.sort();
    debug!(
        store_dir = %store_dir.display(),
        extension,
        entry_count = entries.len(),
        "listed password store entries"
    );
    Ok(entries)
}

fn collect_entries(
    root: &Path,
    current: &Path,
    extension: &str,
    entries: &mut Vec<String>,
) -> Result<(), AppError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if is_hidden_name(&entry.file_name()) {
            continue;
        }

        if path.is_dir() {
            collect_entries(root, &path, extension, entries)?;
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if let Some(_base_name) = name.strip_suffix(extension) {
            let relative = path.strip_prefix(root).map_err(|_| {
                AppError::InvalidEntry(format!(
                    "Failed to normalize password-store path: {}",
                    path.display()
                ))
            })?;
            let mut normalized = relative.to_string_lossy().replace('\\', "/");
            normalized.truncate(normalized.len() - extension.len());
            entries.push(normalized);
        }
    }

    Ok(())
}

fn is_hidden_name(name: &std::ffi::OsStr) -> bool {
    name.to_str().is_some_and(|value| value.starts_with('.'))
}

fn base_pass_command(store_dir: &Path) -> Command {
    let mut command = Command::new("pass");
    command.env("PASSWORD_STORE_DIR", store_dir);
    command
}

fn base_passage_command(store_dir: &Path) -> Command {
    let mut command = Command::new("passage");
    command.env("PASSAGE_DIR", store_dir);
    command
}

fn pass_show_args(entry: &str) -> Vec<String> {
    vec!["show".to_string(), entry.to_string()]
}

fn passage_show_args(entry: &str) -> Vec<String> {
    vec!["show".to_string(), entry.to_string()]
}

fn home_subdir(suffix: &str) -> Result<PathBuf, AppError> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| AppError::Config("`HOME` is not set.".to_string()))?;
    Ok(home.join(suffix))
}

fn require_existing_store_dir(path: PathBuf) -> Result<PathBuf, AppError> {
    if path.is_dir() {
        Ok(path)
    } else {
        Err(AppError::Config(format!(
            "Password store not found: {}",
            path.display()
        )))
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
    use super::{list_entries_with_extension, pass_show_args, passage_show_args};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn pass_show_args_are_stable() {
        assert_eq!(pass_show_args("mail/account"), vec!["show", "mail/account"]);
    }

    #[test]
    fn passage_show_args_are_stable() {
        assert_eq!(
            passage_show_args("mail/account"),
            vec!["show", "mail/account"]
        );
    }

    #[test]
    fn lists_normalized_entries() {
        let store_dir = unique_temp_dir("pmenu-store-test");
        fs::create_dir_all(store_dir.join("nested")).expect("store dir should be created");
        fs::write(store_dir.join("nested/login.age"), b"secret").expect("file should be written");
        fs::write(store_dir.join("top.age"), b"secret").expect("file should be written");

        let mut entries =
            list_entries_with_extension(&store_dir, ".age").expect("entries should list");
        entries.sort();
        assert_eq!(entries, vec!["nested/login".to_string(), "top".to_string()]);

        fs::remove_dir_all(store_dir).expect("temp dir should be removed");
    }

    #[test]
    fn skips_hidden_files_and_directories() {
        let store_dir = unique_temp_dir("pmenu-store-hidden-test");
        fs::create_dir_all(store_dir.join(".hidden")).expect("hidden dir should be created");
        fs::create_dir_all(store_dir.join("visible")).expect("visible dir should be created");
        fs::write(store_dir.join(".ignored.age"), b"secret").expect("file should be written");
        fs::write(store_dir.join(".hidden/login.age"), b"secret").expect("file should be written");
        fs::write(store_dir.join("visible/login.age"), b"secret").expect("file should be written");

        let entries = list_entries_with_extension(&store_dir, ".age").expect("entries should list");
        assert_eq!(entries, vec!["visible/login".to_string()]);

        fs::remove_dir_all(store_dir).expect("temp dir should be removed");
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }
}
