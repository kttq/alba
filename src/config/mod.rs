mod error;
mod file;
mod options;

pub use error::ConfigError;
pub use file::{TEMPLATE, load, path};
pub use options::*;

use serde::Deserialize;
use toml::Spanned;

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
    Coordinates { latitude: f64, longitude: f64 },
}

/// The file's shape. Coordinates keep their spans, so a location error can name its line.
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
    latitude: Option<Spanned<f64>>,
    longitude: Option<Spanned<f64>>,
    #[serde(default = "enabled")]
    ghost_segments: bool,
    #[serde(default = "enabled")]
    motion: bool,
}

fn enabled() -> bool {
    true
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let file: ConfigFile = toml::from_str(text).map_err(|error| {
            ConfigError::at(text, error.span().unwrap_or_default(), error.message())
        })?;
        let location = location(text, file.location, file.latitude, file.longitude)?;
        Ok(Self {
            language: file.language,
            units: file.units,
            hour_format: file.hour_format,
            week_start: file.week_start,
            location,
            ghost_segments: file.ghost_segments,
            motion: file.motion,
        })
    }
}

/// Coordinates win over the name. Only one of them set is an error.
fn location(
    text: &str,
    name: Option<String>,
    latitude: Option<Spanned<f64>>,
    longitude: Option<Spanned<f64>>,
) -> Result<Option<Location>, ConfigError> {
    let unpaired =
        |coordinate: Spanned<f64>, reason| ConfigError::at(text, coordinate.span(), reason);
    match (latitude, longitude) {
        (Some(latitude), Some(longitude)) => Ok(Some(Location::Coordinates {
            latitude: in_range(text, "latitude", latitude, 90.0)?,
            longitude: in_range(text, "longitude", longitude, 180.0)?,
        })),
        (Some(latitude), None) => Err(unpaired(latitude, "latitude is set without longitude")),
        (None, Some(longitude)) => Err(unpaired(longitude, "longitude is set without latitude")),
        (None, None) => Ok(name.map(Location::Name)),
    }
}

/// Checks that a coordinate in degrees lies within `-limit..=limit`. NaN fails too.
fn in_range(text: &str, name: &str, degrees: Spanned<f64>, limit: f64) -> Result<f64, ConfigError> {
    let span = degrees.span();
    let degrees = degrees.into_inner();
    if (-limit..=limit).contains(&degrees) {
        Ok(degrees)
    } else {
        let reason = format!("{name} must be -{limit} to {limit}, found {degrees}");
        Err(ConfigError::at(text, span, &reason))
    }
}
