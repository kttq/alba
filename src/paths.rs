//! Where alba keeps its files, by the XDG Base Directory spec on macOS and Linux alike.

use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_file: PathBuf,
    /// The config file with `HOME` shown as `~`, for the first-run hint.
    pub shown_config_file: String,
    pub cache_directory: PathBuf,
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
        let config_file = base("XDG_CONFIG_HOME", ".config")?.join("alba/config.toml");
        Ok(Self {
            shown_config_file: shown_under_home(&config_file, absolute(&variable, "HOME")),
            config_file,
            cache_directory: base("XDG_CACHE_HOME", ".cache")?.join("alba"),
            state_directory: base("XDG_STATE_HOME", ".local/state")?.join("alba"),
        })
    }
}

/// A path under `HOME` starts with `~`; any other path shows whole.
fn shown_under_home(path: &Path, home: Option<PathBuf>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok().map(Path::to_path_buf)) {
        Some(relative) => format!("~/{}", relative.display()),
        None => path.display().to_string(),
    }
}

fn absolute(variable: &impl Fn(&str) -> Option<OsString>, name: &str) -> Option<PathBuf> {
    variable(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

/// An empty `HOME` counts as unset, so paths never resolve against the working directory.
fn base_directory(
    variable: &impl Fn(&str) -> Option<OsString>,
    name: &str,
    default_under_home: &str,
) -> Result<PathBuf, HomeNotSet> {
    match absolute(variable, name) {
        Some(directory) => Ok(directory),
        None => Ok(absolute(variable, "HOME")
            .ok_or(HomeNotSet)?
            .join(default_under_home)),
    }
}
