use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Config(String),
    CommandMissing(String),
    CommandFailed {
        command: String,
        code: Option<i32>,
        stderr: String,
    },
    InvalidEntry(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Config(message) => write!(f, "{message}"),
            Self::CommandMissing(command) => write!(f, "Missing required command: {command}"),
            Self::CommandFailed {
                command,
                code,
                stderr,
            } => {
                if stderr.trim().is_empty() {
                    write!(f, "Command `{command}` failed with exit code {:?}", code)
                } else {
                    write!(
                        f,
                        "Command `{command}` failed with exit code {:?}: {}",
                        code,
                        stderr.trim()
                    )
                }
            }
            Self::InvalidEntry(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
