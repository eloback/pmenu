use std::str::FromStr;

use super::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryContent {
    pub password: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Copy,
    Autofill,
}

impl AppAction {
    pub fn past_tense(self) -> &'static str {
        match self {
            Self::Copy => "Copied",
            Self::Autofill => "Autofilled",
        }
    }
}

impl FromStr for AppAction {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "copy" => Ok(Self::Copy),
            "autofill" => Ok(Self::Autofill),
            _ => Err(format!("Unknown action: `{value}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    pub field_name: String,
    pub action: AppAction,
}

pub fn parse_entry_content(raw: &str) -> Result<EntryContent, AppError> {
    let mut lines = raw.lines();
    let password = lines
        .next()
        .ok_or_else(|| AppError::InvalidEntry("Password entry is empty.".to_string()))?
        .to_string();

    let mut fields = Vec::new();
    for line in lines {
        if let Some((name, value)) = parse_field_line(line) {
            fields.push((name, value));
        }
    }

    Ok(EntryContent { password, fields })
}

fn parse_field_line(line: &str) -> Option<(String, String)> {
    let (name, value) = line.split_once(':')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), value.trim_start().to_string()))
}

#[cfg(test)]
mod tests {
    use super::{parse_entry_content, AppAction};
    use std::str::FromStr;

    #[test]
    fn parses_password_and_fields() {
        let content = parse_entry_content("secret\nusername: demo\nurl: https://example.com\n")
            .expect("entry should parse");

        assert_eq!(content.password, "secret");
        assert_eq!(
            content.fields,
            vec![
                ("username".to_string(), "demo".to_string()),
                ("url".to_string(), "https://example.com".to_string())
            ]
        );
    }

    #[test]
    fn parses_app_action() {
        assert_eq!(AppAction::from_str("copy").expect("copy should parse"), AppAction::Copy);
        assert_eq!(
            AppAction::from_str("autofill").expect("autofill should parse"),
            AppAction::Autofill
        );
    }
}
