//! Weather from Open-Meteo: the request and response, conditions, units, and the disk cache.

pub mod cache;
mod condition;
mod forecast;
pub mod open_meteo;
mod source;

pub use condition::Condition;
pub use forecast::Forecast;
pub use source::{BackgroundSource, FetchError, WeatherSource};

use jiff::Timestamp;

use crate::config::Units;

/// A validated forecast and when it arrived.
#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    pub fetched_at: Timestamp,
    pub forecast: Forecast,
}

/// Open-Meteo always sends Celsius, so a units change keeps the cache valid.
pub fn whole_degrees(celsius: f64, units: Units) -> i64 {
    let degrees = match units {
        Units::Metric => celsius,
        Units::Imperial => celsius * 9.0 / 5.0 + 32.0,
    };
    degrees.round() as i64
}
