//! alba's log, in logfmt.

use std::{
    fmt::{self, Display},
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

    pub fn write(
        &mut self,
        level: Level,
        now: &Zoned,
        message: &str,
        fields: &[(&str, &dyn Display)],
    ) {
        let Some(file) = &mut self.file else {
            return;
        };
        let line = logfmt_line(level, now, message, fields);
        let _ = file.write_all(line.as_bytes());
    }
}

fn logfmt_line(
    level: Level,
    now: &Zoned,
    message: &str,
    fields: &[(&str, &dyn Display)],
) -> String {
    let mut line = format!(
        "time={} level={level} msg={}",
        now.strftime("%Y-%m-%dT%H:%M:%S%:z"),
        quoted(message),
    );
    for (key, value) in fields {
        line.push_str(&format!(" {key}={}", quoted(&value.to_string())));
    }
    line.push('\n');
    line
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Warn,
}

impl fmt::Display for Level {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Self::Warn => "warn",
        };
        formatter.write_str(name)
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
