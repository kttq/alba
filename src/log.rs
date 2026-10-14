//! alba's own log, in logfmt: one event per line, as `key=value` pairs.

use std::{
    fmt::Display,
    fs::{self, File},
    io::Write,
    path::Path,
};

use jiff::Zoned;

const FILE_NAME: &str = "alba.log";

/// alba owns the whole terminal while it runs, so events go to this file instead of stderr.
/// Logging is best effort: when the file can't be created or written, events are dropped.
pub struct Log {
    file: Option<File>,
}

impl Log {
    /// Creates `directory` when missing, and empties the log left by an earlier run.
    pub fn start(directory: &Path) -> Self {
        let file = fs::create_dir_all(directory)
            .and_then(|()| File::create(directory.join(FILE_NAME)))
            .ok();
        Self { file }
    }

    pub fn warn(&mut self, now: &Zoned, message: &str, error: &dyn Display) {
        let Some(file) = &mut self.file else {
            return;
        };
        let line = format!(
            "time={} level=warn msg={} error={}\n",
            now.strftime("%Y-%m-%dT%H:%M:%S%:z"),
            quoted(message),
            quoted(&error.to_string()),
        );
        let _ = file.write_all(line.as_bytes());
    }
}

/// logfmt decoders read a quoted value with JSON string escapes, such as `\u001b`.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("a string always serializes")
}
