mod error;
mod file;
mod options;

pub use error::ConfigError;
pub use file::{TEMPLATE, load, path};
pub use options::*;

use serde::Deserialize;
use toml::Spanned;

/// The settings from `config.toml`, parsed and validated.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub language: Language,
    pub units: Units,
    pub hour_format: HourFormat,
    pub week_start: WeekStart,
    pub location: Option<Location>,
    pub clock: ClockConfig,
    pub weather: WeatherConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    Name(String),
    Coordinates { latitude: f64, longitude: f64 },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ClockConfig {
    pub thickness: Thickness,
    pub digit_gap: DigitGap,
    pub colon_gap: ColonGap,
    pub colon: ColonStyle,
    pub colon_dots: ColonDots,
    pub ghost_segments: bool,
    pub tails: bool,
    pub hooked_seven: bool,
    pub am_pm: AmPm,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            thickness: Thickness::default(),
            digit_gap: DigitGap::default(),
            colon_gap: ColonGap::default(),
            colon: ColonStyle::default(),
            colon_dots: ColonDots::default(),
            ghost_segments: true,
            tails: true,
            hooked_seven: false,
            am_pm: AmPm::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WeatherConfig {
    pub animation_fps: AnimationFps,
    pub flash: Flash,
    pub flash_target: FlashTarget,
    pub moon_dark_side: MoonDarkSide,
    pub stars: Stars,
}

/// The file's shape. Coordinates keep their spans, so a location error can name its line.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ConfigFile {
    language: Language,
    units: Units,
    hour_format: HourFormat,
    week_start: WeekStart,
    location: Option<String>,
    latitude: Option<Spanned<f64>>,
    longitude: Option<Spanned<f64>>,
    clock: ClockConfig,
    weather: WeatherConfig,
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let file: ConfigFile = toml::from_str(text)
            .map_err(|error| ConfigError::at(text, error.span(), error.message()))?;
        let location = match (file.latitude, file.longitude) {
            (Some(latitude), Some(longitude)) => Some(Location::Coordinates {
                latitude: in_range(text, "latitude", latitude, 90.0)?,
                longitude: in_range(text, "longitude", longitude, 180.0)?,
            }),
            (Some(latitude), None) => {
                return Err(ConfigError::at(
                    text,
                    Some(latitude.span()),
                    "latitude is set without longitude",
                ));
            }
            (None, Some(longitude)) => {
                return Err(ConfigError::at(
                    text,
                    Some(longitude.span()),
                    "longitude is set without latitude",
                ));
            }
            (None, None) => file.location.map(Location::Name),
        };
        Ok(Self {
            language: file.language,
            units: file.units,
            hour_format: file.hour_format,
            week_start: file.week_start,
            location,
            clock: file.clock,
            weather: file.weather,
        })
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
        Err(ConfigError::at(text, Some(span), &reason))
    }
}
