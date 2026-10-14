pub mod cache;
mod condition;
mod forecast;
mod freshness;
pub mod open_meteo;
mod schedule;
mod source;
mod station;
mod text;

pub use condition::Condition;
pub use forecast::{Forecast, IsDay};
pub use freshness::Freshness;
pub use schedule::{LastResult, next_fetch};
pub use source::{BackgroundSource, FetchError, WeatherSource};
pub use station::{WeatherState, WeatherStation};
pub use text::text_line;

use jiff::Timestamp;

use crate::config::Units;

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
