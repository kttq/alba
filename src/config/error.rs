use std::{fmt, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// A syntax error, unknown key, or bad value.
    Invalid { line: usize, reason: String },
    /// The file exists but cannot be read as text.
    Unreadable { reason: String },
}

impl ConfigError {
    /// An `Invalid` error on the line where `span` starts. A span-less error points at line 1.
    pub(super) fn at(text: &str, span: Option<Range<usize>>, reason: &str) -> Self {
        let start = span.map_or(0, |span| span.start).min(text.len());
        let line = text[..start].matches('\n').count() + 1;
        Self::Invalid {
            line,
            reason: reason.to_string(),
        }
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
