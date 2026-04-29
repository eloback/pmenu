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
    entry_query: Option<&str>,
    requested_field: Option<&str>,
) -> Result<Option<ActionOutcome>, AppError> {
    trace!(?action, "starting password selection flow");
    let entries = store.list_entries()?;
    if entries.is_empty() {
        return Ok(None);
    }

    let Some(entry_name) = menu.select("󰌆 ", &entries, entry_query)? else {
        return Ok(None);
    };

    let entry = store.show_entry(&entry_name)?;
    let options = menu_options(&entry_name, &entry, autofill.is_some());

    let selected_option = if let Some(requested_field) = requested_field {
        find_option(&options, requested_field)?
    } else {
        let labels = option_labels(&options);
        let Some(field_name) = menu.select(" ", &labels, None)? else {
            return Ok(None);
        };
        find_option(&options, &field_name)?
    };

    match &selected_option.kind {
        MenuOptionKind::Fill => {
            let autofill = autofill.ok_or_else(|| {
                AppError::Config("Autofill backend is required for fill actions.".to_string())
            })?;
            let username = resolve_username(&entry_name, &entry)
                .ok_or_else(|| AppError::InvalidEntry(format!("No username for {entry_name}")))?;
            let password = password_value(&entry_name, &entry)?;
            trace!(
                field = selected_option.label,
                username_len = username.len(),
                password_len = password.len(),
                "autofilling username and password"
            );
            autofill.autofill_login(&username, password)?;
            return Ok(Some(ActionOutcome {
                field_name: "credentials".to_string(),
                action: AppAction::Autofill,
            }));
        }
        _ => {
            let value = selected_value(&entry_name, &entry, &selected_option.kind)?;
            match action {
                AppAction::Copy => {
                    let clipboard = clipboard.ok_or_else(|| {
                        AppError::Config(
                            "Clipboard backend is required for copy actions.".to_string(),
                        )
                    })?;
                    trace!(
                        field = selected_option.label,
                        value_len = value.len(),
                        "copying selected value"
                    );
                    clipboard.copy(&value)?;
                }
                AppAction::Autofill => {
                    let autofill = autofill.ok_or_else(|| {
                        AppError::Config(
                            "Autofill backend is required for autofill actions.".to_string(),
                        )
                    })?;
                    trace!(
                        field = selected_option.label,
                        value_len = value.len(),
                        "autofilling selected value"
                    );
                    autofill.autofill(&value)?;
                }
            }
        }
    }

    Ok(Some(ActionOutcome {
        field_name: selected_option.label,
        action,
    }))
}

fn option_labels(options: &[MenuOption]) -> Vec<String> {
    options.iter().map(|option| option.label.clone()).collect()
}

fn find_option(options: &[MenuOption], requested_field: &str) -> Result<MenuOption, AppError> {
    options
        .iter()
        .find(|option| option.label.eq_ignore_ascii_case(requested_field))
        .cloned()
        .ok_or_else(|| AppError::InvalidEntry(format!("Field not found: {requested_field}")))
}

fn menu_options(entry_name: &str, entry: &EntryContent, can_fill: bool) -> Vec<MenuOption> {
    let mut options = vec![MenuOption::new("password", MenuOptionKind::Password)];

    if resolve_username(entry_name, entry).is_some() {
        options.push(MenuOption::new("username", MenuOptionKind::Username));
    }
    if resolve_url(entry_name, entry).is_some() {
        options.push(MenuOption::new("url", MenuOptionKind::Url));
    }
    if can_fill && resolve_username(entry_name, entry).is_some() {
        options.push(MenuOption::new("fill", MenuOptionKind::Fill));
    }

    options.extend(
        entry
            .fields
            .iter()
            .enumerate()
            .filter_map(|(index, (name, _))| {
                if is_builtin_field(name) {
                    None
                } else {
                    Some(MenuOption::new(name, MenuOptionKind::StoredField(index)))
                }
            }),
    );

    options
}

fn selected_value(
    entry_name: &str,
    entry: &EntryContent,
    kind: &MenuOptionKind,
) -> Result<String, AppError> {
    match kind {
        MenuOptionKind::Password => password_value(entry_name, entry).map(ToString::to_string),
        MenuOptionKind::Username => resolve_username(entry_name, entry)
            .ok_or_else(|| AppError::InvalidEntry(format!("No username for {entry_name}"))),
        MenuOptionKind::Url => resolve_url(entry_name, entry)
            .ok_or_else(|| AppError::InvalidEntry(format!("No url for {entry_name}"))),
        MenuOptionKind::StoredField(index) => entry
            .fields
            .get(*index)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| AppError::InvalidEntry("Field index out of bounds.".to_string())),
        MenuOptionKind::Fill => Err(AppError::InvalidEntry(
            "Fill is not a copyable field.".to_string(),
        )),
    }
}

fn password_value<'a>(entry_name: &str, entry: &'a EntryContent) -> Result<&'a str, AppError> {
    if entry.password.is_empty() {
        Err(AppError::InvalidEntry(format!(
            "No password for {entry_name}"
        )))
    } else {
        Ok(entry.password.as_str())
    }
}

fn resolve_username(entry_name: &str, entry: &EntryContent) -> Option<String> {
    resolve_field(entry, "username")
        .map(ToString::to_string)
        .or_else(|| fallback_username(entry_name))
}

fn resolve_url(entry_name: &str, entry: &EntryContent) -> Option<String> {
    resolve_field(entry, "url")
        .map(ToString::to_string)
        .or_else(|| fallback_url(entry_name))
}

fn resolve_field<'a>(entry: &'a EntryContent, field_name: &str) -> Option<&'a str> {
    entry.fields.iter().find_map(|(name, value)| {
        if name.eq_ignore_ascii_case(field_name) && !value.is_empty() {
            Some(value.as_str())
        } else {
            None
        }
    })
}

fn fallback_username(entry_name: &str) -> Option<String> {
    let segments = path_segments(entry_name);
    match segments.as_slice() {
        [] => None,
        [_single] => Some(segments[0].to_string()),
        _ => Some(segments[1].to_string()),
    }
}

fn fallback_url(entry_name: &str) -> Option<String> {
    path_segments(entry_name)
        .first()
        .map(|segment| (*segment).to_string())
}

fn path_segments(entry_name: &str) -> Vec<&str> {
    entry_name
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn is_builtin_field(field_name: &str) -> bool {
    matches!(
        field_name.to_ascii_lowercase().as_str(),
        "password" | "username" | "url" | "fill"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MenuOption {
    label: String,
    kind: MenuOptionKind,
}

impl MenuOption {
    fn new(label: &str, kind: MenuOptionKind) -> Self {
        Self {
            label: label.to_string(),
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MenuOptionKind {
    Password,
    Username,
    Url,
    StoredField(usize),
    Fill,
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
        fn select(
            &self,
            _prompt: &str,
            _items: &[String],
            _initial_query: Option<&str>,
        ) -> Result<Option<String>, AppError> {
            Ok(self.selections.borrow_mut().pop().flatten())
        }
    }

    struct StubStore;

    impl PasswordStoreBackend for StubStore {
        fn list_entries(&self) -> Result<Vec<String>, AppError> {
            Ok(vec!["mail/demo".to_string()])
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

        fn autofill_login(&self, username: &str, password: &str) -> Result<(), AppError> {
            self.0.borrow_mut().push(format!("{username}\t{password}"));
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
            None,
            None,
        )
        .expect("flow should succeed")
        .expect("selection should complete");

        assert_eq!(outcome.field_name, "password");
        assert_eq!(clipboard.0.borrow().as_slice(), ["secret"]);
    }

    #[test]
    fn autofill_flow_uses_selected_field() {
        let menu = StubMenu::new(&[Some("mail/demo"), Some("username")]);
        let store = StubStore;
        let autofill = StubAutofill(RefCell::new(Vec::new()));

        let outcome = run_flow(
            &menu,
            &store,
            None,
            Some(&autofill),
            AppAction::Autofill,
            None,
            None,
        )
        .expect("flow should succeed")
        .expect("selection should complete");

        assert_eq!(outcome.field_name, "username");
        assert_eq!(autofill.0.borrow().as_slice(), ["demo"]);
    }

    #[test]
    fn copy_flow_uses_path_derived_url() {
        let menu = StubMenu::new(&[Some("mail/demo"), Some("url")]);
        let store = StubStore;
        let clipboard = StubClipboard(RefCell::new(Vec::new()));

        let outcome = run_flow(
            &menu,
            &store,
            Some(&clipboard),
            None,
            AppAction::Copy,
            Some("mail"),
            None,
        )
        .expect("flow should succeed")
        .expect("selection should complete");

        assert_eq!(outcome.field_name, "url");
        assert_eq!(clipboard.0.borrow().as_slice(), ["mail"]);
    }

    #[test]
    fn copy_flow_supports_fill_option() {
        let menu = StubMenu::new(&[Some("mail/demo"), Some("fill")]);
        let store = StubStore;
        let autofill = StubAutofill(RefCell::new(Vec::new()));

        let outcome = run_flow(
            &menu,
            &store,
            None,
            Some(&autofill),
            AppAction::Copy,
            None,
            None,
        )
        .expect("flow should succeed")
        .expect("selection should complete");

        assert_eq!(outcome.action, AppAction::Autofill);
        assert_eq!(outcome.field_name, "credentials");
        assert_eq!(autofill.0.borrow().as_slice(), ["demo\tsecret"]);
    }

    #[test]
    fn requested_field_skips_second_prompt() {
        let menu = StubMenu::new(&[Some("mail/demo")]);
        let store = StubStore;
        let clipboard = StubClipboard(RefCell::new(Vec::new()));

        let outcome = run_flow(
            &menu,
            &store,
            Some(&clipboard),
            None,
            AppAction::Copy,
            None,
            Some("username"),
        )
        .expect("flow should succeed")
        .expect("selection should complete");

        assert_eq!(outcome.field_name, "username");
        assert_eq!(clipboard.0.borrow().as_slice(), ["demo"]);
    }

    #[test]
    fn canceled_selection_returns_none() {
        let menu = StubMenu::new(&[None]);
        let store = StubStore;

        let outcome = run_flow(&menu, &store, None, None, AppAction::Copy, None, None)
            .expect("canceled flow should not error");
        assert!(outcome.is_none());
    }
}
