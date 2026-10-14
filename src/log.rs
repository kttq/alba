//! alba's log, in logfmt.

use std::{
    fmt::Display,
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

use jiff::Zoned;

const FILE_NAME: &str = "alba.log";

/// alba owns the terminal, so events go to a file instead of stderr.
pub struct Log {
    file: Option<File>,
}

impl Log {
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

/// The log of the running alba, or of the last run. Empty before alba's first run.
pub fn read(directory: &Path) -> io::Result<String> {
    match fs::read_to_string(directory.join(FILE_NAME)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        result => result,
    }
}

/// logfmt decoders accept JSON string escapes in quoted values.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("a string always serializes")
}
