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
pub use forecast::{DayOrNight, Forecast, HighAndLow, SunEvent};
pub use freshness::Freshness;
pub use schedule::{LastFetchResult, next_fetch};
pub use source::{BackgroundSource, FetchError, Request, WeatherSource};
pub use station::{WeatherState, WeatherStation};
pub use text::{
    aged_temperature_high_and_low_line, sparkline, temperature_high_and_low_line, temperature_line,
    text_line, weather_state_lines,
};

use jiff::Timestamp;

use crate::{
    config::{Coordinates, Units},
    geocode::Place,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    pub fetched_at: Timestamp,
    pub forecast: Forecast,
}

/// Where the forecast is for: the configured coordinates, or the place a name resolved to.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedLocation {
    pub coordinates: Coordinates,
    /// None for configured coordinates.
    pub place_name: Option<String>,
}

impl From<Place> for ResolvedLocation {
    fn from(place: Place) -> Self {
        Self {
            coordinates: place.coordinates,
            place_name: Some(place.name),
        }
    }
}

/// Open-Meteo always sends Celsius, so a units change keeps the cache valid.
pub fn whole_degrees(celsius: f64, units: Units) -> i64 {
    let degrees = match units {
        Units::Metric => celsius,
        Units::Imperial => celsius * 9.0 / 5.0 + 32.0,
    };
    degrees.round() as i64
}
