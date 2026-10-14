use std::{fmt, io, ops::Range, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// A syntax error, unknown key, or bad value.
    Invalid { line: usize, reason: String },
    /// The file exists but cannot be read as text.
    Unreadable { reason: String },
}

impl ConfigError {
    pub(super) fn at(text: &str, span: Range<usize>, reason: &str) -> Self {
        let start = span.start.min(text.len());
        let line = text[..start].matches('\n').count() + 1;
        Self::Invalid {
            line,
            reason: reason.to_string(),
        }
    }

    /// `read_to_string` reports a file that isn't UTF-8 as `InvalidData`.
    pub(super) fn unreadable(path: &Path, error: &io::Error) -> Self {
        let reason = match error.kind() {
            io::ErrorKind::InvalidData => format!("{} is not valid UTF-8", path.display()),
            _ => format!("cannot read {}: {error}", path.display()),
        };
        Self::Unreadable { reason }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Invalid { line, reason } => {
                write!(formatter, "config error: line {line}: {reason}")
            }
            Self::Unreadable { reason } => write!(formatter, "config error: {reason}"),
        }
    }
}
