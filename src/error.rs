use std::{error::Error, fmt, path::PathBuf};

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub location: Option<(PathBuf, usize)>,
    pub message: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            location: None,
            message: message.into(),
        }
    }

    pub fn at(location: (PathBuf, usize), message: impl Into<String>) -> Self {
        Self {
            location: Some(location),
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((path, line)) = &self.location {
            write!(f, "{}:{}: {}", path.display(), line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl Error for ParseError {}
