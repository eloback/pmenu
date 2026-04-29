use super::{
    ActionOutcome, AppAction, AppError, AutofillBackend, ClipboardBackend, EntryContent,
    MenuBackend, PasswordStoreBackend,
};
use tracing::trace;

pub fn run_flow(
    menu: &dyn MenuBackend,
    store: &dyn PasswordStoreBackend,
    clipboard: Option<&dyn ClipboardBackend>,
    autofill: Option<&dyn AutofillBackend>,
    action: AppAction,
) -> Result<Option<ActionOutcome>, AppError> {
    trace!(?action, "starting password selection flow");
    let entries = store.list_entries()?;
    if entries.is_empty() {
        return Ok(None);
    }

    let Some(entry_name) = menu.select("󰌆 ", &entries)? else {
        return Ok(None);
    };

    let entry = store.show_entry(&entry_name)?;
    let field_names = field_names(&entry);

    let Some(field_name) = menu.select(" ", &field_names)? else {
        return Ok(None);
    };

    let value = selected_value(&entry, &field_name)?;
    match action {
        AppAction::Copy => {
            let clipboard = clipboard.ok_or_else(|| {
                AppError::Config("Clipboard backend is required for copy actions.".to_string())
            })?;
            trace!(field = field_name, value_len = value.len(), "copying selected value");
            clipboard.copy(value)?;
        }
        AppAction::Autofill => {
            let autofill = autofill.ok_or_else(|| {
                AppError::Config("Autofill backend is required for autofill actions.".to_string())
            })?;
            trace!(field = field_name, value_len = value.len(), "autofilling selected value");
            autofill.autofill(value)?;
        }
    }

    Ok(Some(ActionOutcome { field_name, action }))
}

fn field_names(entry: &EntryContent) -> Vec<String> {
    let mut fields = Vec::with_capacity(entry.fields.len() + 1);
    fields.push("password".to_string());
    fields.extend(entry.fields.iter().map(|(name, _)| name.clone()));
    fields
}

fn selected_value<'a>(entry: &'a EntryContent, field_name: &str) -> Result<&'a str, AppError> {
    if field_name == "password" {
        return Ok(entry.password.as_str());
    }

    entry.fields
        .iter()
        .find(|(name, _)| name == field_name)
        .map(|(_, value)| value.as_str())
        .ok_or_else(|| AppError::InvalidEntry(format!("Field not found: {field_name}")))
}

#[cfg(test)]
mod tests {
    use super::run_flow;
    use crate::core::{
        AppAction, AppError, AutofillBackend, ClipboardBackend, EntryContent, MenuBackend,
        PasswordStoreBackend,
    };
    use std::cell::RefCell;

    struct StubMenu {
        selections: RefCell<Vec<Option<String>>>,
    }

    impl StubMenu {
        fn new(selections: &[Option<&str>]) -> Self {
            Self {
                selections: RefCell::new(
                    selections
                        .iter()
                        .rev()
                        .map(|value| value.map(ToString::to_string))
                        .collect(),
                ),
            }
        }
    }

    impl MenuBackend for StubMenu {
        fn select(&self, _prompt: &str, _items: &[String]) -> Result<Option<String>, AppError> {
            Ok(self.selections.borrow_mut().pop().flatten())
        }
    }

    struct StubStore;

    impl PasswordStoreBackend for StubStore {
        fn list_entries(&self) -> Result<Vec<String>, AppError> {
            Ok(vec!["demo".to_string()])
        }

        fn show_entry(&self, _entry: &str) -> Result<EntryContent, AppError> {
            Ok(EntryContent {
                password: "secret".to_string(),
                fields: vec![("username".to_string(), "demo".to_string())],
            })
        }
    }

    struct StubClipboard(RefCell<Vec<String>>);

    impl ClipboardBackend for StubClipboard {
        fn copy(&self, value: &str) -> Result<(), AppError> {
            self.0.borrow_mut().push(value.to_string());
            Ok(())
        }
    }

    struct StubAutofill(RefCell<Vec<String>>);

    impl AutofillBackend for StubAutofill {
        fn autofill(&self, value: &str) -> Result<(), AppError> {
            self.0.borrow_mut().push(value.to_string());
            Ok(())
        }
    }

    #[test]
    fn copy_flow_uses_clipboard() {
        let menu = StubMenu::new(&[Some("demo"), Some("password")]);
        let store = StubStore;
        let clipboard = StubClipboard(RefCell::new(Vec::new()));

        let outcome = run_flow(
            &menu,
            &store,
            Some(&clipboard),
            None,
            AppAction::Copy,
        )
        .expect("flow should succeed")
        .expect("selection should complete");

        assert_eq!(outcome.field_name, "password");
        assert_eq!(clipboard.0.borrow().as_slice(), ["secret"]);
    }

    #[test]
    fn autofill_flow_uses_selected_field() {
        let menu = StubMenu::new(&[Some("demo"), Some("username")]);
        let store = StubStore;
        let autofill = StubAutofill(RefCell::new(Vec::new()));

        let outcome = run_flow(
            &menu,
            &store,
            None,
            Some(&autofill),
            AppAction::Autofill,
        )
        .expect("flow should succeed")
        .expect("selection should complete");

        assert_eq!(outcome.field_name, "username");
        assert_eq!(autofill.0.borrow().as_slice(), ["demo"]);
    }

    #[test]
    fn canceled_selection_returns_none() {
        let menu = StubMenu::new(&[None]);
        let store = StubStore;

        let outcome = run_flow(&menu, &store, None, None, AppAction::Copy)
            .expect("canceled flow should not error");
        assert!(outcome.is_none());
    }
}
