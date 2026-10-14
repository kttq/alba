use std::{fs, io, path::Path};

use super::{Config, ConfigError};

pub const TEMPLATE: &str = include_str!("template.toml");

/// A failed template write must not stop alba from starting.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    match fs::read_to_string(path) {
        Ok(text) => Config::parse(&text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let _ = write_template(path);
            Ok(Config::default())
        }
        Err(error) => Err(ConfigError::unreadable(path, &error)),
    }
}

/// Writes a temporary file, then links it into place, so an interrupted write never leaves a
/// partial config. The link fails if a file appeared since the read, so nothing is overwritten.
fn write_template(path: &Path) -> io::Result<()> {
    let directory = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(directory)?;
    let temporary = directory.join(format!(".config.toml.{}", std::process::id()));
    let written = fs::write(&temporary, TEMPLATE).and_then(|()| fs::hard_link(&temporary, path));
    let _ = fs::remove_file(&temporary);
    written
}
