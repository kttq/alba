use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::{Config, ConfigError};

/// Written on first run, when no config file exists.
pub const TEMPLATE: &str = include_str!("template.toml");

/// `XDG_CONFIG_HOME` is ignored on purpose: the path is the same on macOS and Linux.
pub fn path(home: &Path) -> PathBuf {
    home.join(".config/alba/config.toml")
}

/// Reads the config at `path`. With no file there, writes the template and returns the defaults.
/// A failed template write is ignored, so alba still runs.
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
