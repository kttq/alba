mod error;
mod file;
mod options;

pub use error::ConfigError;
pub use file::{TEMPLATE, load};
pub use options::*;

use serde::Deserialize;

/// The settings from `config.toml`, parsed and validated.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub language: Language,
    pub units: Units,
    pub hour_format: HourFormat,
    pub week_start: WeekStart,
    pub location: Option<Location>,
    /// Unlit clock segments show faintly.
    pub ghost_segments: bool,
    /// The colon blinks, icons animate, and storms flash. `false` keeps everything still.
    pub motion: bool,
}

/// The defaults live once, on `ConfigFile`.
impl Default for Config {
    fn default() -> Self {
        Self::parse("").expect("the empty config is valid")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    Name(String),
    Coordinates(Coordinates),
}

/// Degrees, written `[latitude, longitude]` and range-checked on parse.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "Vec<f64>")]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

/// From a list, not a `[f64; 2]`: toml drops the extra numbers of a fixed-size array silently.
impl TryFrom<Vec<f64>> for Coordinates {
    type Error = String;

    /// NaN fails the range checks too.
    fn try_from(degrees: Vec<f64>) -> Result<Self, String> {
        let &[latitude, longitude] = degrees.as_slice() else {
            return Err(format!(
                "coordinates must be [latitude, longitude], found {} numbers",
                degrees.len()
            ));
        };
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(format!("latitude must be -90 to 90, found {latitude}"));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(format!("longitude must be -180 to 180, found {longitude}"));
        }
        Ok(Self {
            latitude,
            longitude,
        })
    }
}

/// The file's shape, before `location` and `coordinates` combine into one `Location`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    language: Language,
    #[serde(default)]
    units: Units,
    #[serde(default)]
    hour_format: HourFormat,
    #[serde(default)]
    week_start: WeekStart,
    location: Option<String>,
    coordinates: Option<Coordinates>,
    #[serde(default = "enabled")]
    ghost_segments: bool,
    #[serde(default = "enabled")]
    motion: bool,
}

fn enabled() -> bool {
    true
}

impl Config {
    /// Coordinates win over `location` when both are set.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let file: ConfigFile = toml::from_str(text).map_err(|error| {
            ConfigError::at(text, error.span().unwrap_or_default(), error.message())
        })?;
        Ok(Self {
            language: file.language,
            units: file.units,
            hour_format: file.hour_format,
            week_start: file.week_start,
            location: file
                .coordinates
                .map(Location::Coordinates)
                .or(file.location.map(Location::Name)),
            ghost_segments: file.ghost_segments,
            motion: file.motion,
        })
    }
}
