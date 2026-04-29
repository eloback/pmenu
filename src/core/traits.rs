use super::{AppError, EntryContent};

pub trait MenuBackend {
    fn select(&self, prompt: &str, items: &[String]) -> Result<Option<String>, AppError>;
}

pub trait PasswordStoreBackend {
    fn list_entries(&self) -> Result<Vec<String>, AppError>;
    fn show_entry(&self, entry: &str) -> Result<EntryContent, AppError>;
}

pub trait ClipboardBackend {
    fn copy(&self, value: &str) -> Result<(), AppError>;
}

pub trait AutofillBackend {
    fn autofill(&self, value: &str) -> Result<(), AppError>;
}
