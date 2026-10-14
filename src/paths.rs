//! Where alba keeps its files, by the XDG Base Directory spec on macOS and Linux alike.

use std::{ffi::OsString, fmt, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_file: PathBuf,
    pub cache_directory: PathBuf,
    /// Holds the log.
    pub state_directory: PathBuf,
}

/// A path fell back to its default under `HOME`, and `HOME` is unset or empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeNotSet;

impl fmt::Display for HomeNotSet {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "HOME is not set")
    }
}

impl Paths {
    pub fn resolve(variable: impl Fn(&str) -> Option<OsString>) -> Result<Self, HomeNotSet> {
        let base = |name, default| base_directory(&variable, name, default);
        Ok(Self {
            config_file: base("XDG_CONFIG_HOME", ".config")?.join("alba/config.toml"),
            cache_directory: base("XDG_CACHE_HOME", ".cache")?.join("alba"),
            state_directory: base("XDG_STATE_HOME", ".local/state")?.join("alba"),
        })
    }
}

/// The XDG spec says to ignore a relative path, and treats an empty variable as unset. An
/// empty `HOME` counts as unset too, so paths never resolve against the working directory.
fn base_directory(
    variable: &impl Fn(&str) -> Option<OsString>,
    name: &str,
    default_under_home: &str,
) -> Result<PathBuf, HomeNotSet> {
    let absolute = |name| {
        variable(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    match absolute(name) {
        Some(directory) => Ok(directory),
        None => Ok(absolute("HOME").ok_or(HomeNotSet)?.join(default_under_home)),
    }
}
