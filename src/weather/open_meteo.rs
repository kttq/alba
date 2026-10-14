//! The real forecast request. No test calls it: tests never touch the network.

use std::time::Duration;

use ureq::Agent;

use super::FetchError;
use crate::config::Coordinates;

const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";

/// Covers connecting, sending, and reading the whole body.
const TIMEOUT: Duration = Duration::from_secs(15);

/// Yesterday through tomorrow, because the night timeline and the 12-hour sparkline cross
/// midnight. Celsius always; alba converts locally.
const FORECAST_QUERY: [(&str, &str); 8] = [
    ("current", "temperature_2m,weather_code,is_day"),
    (
        "daily",
        "sunrise,sunset,temperature_2m_max,temperature_2m_min",
    ),
    ("hourly", "precipitation_probability"),
    ("timezone", "auto"),
    ("timeformat", "unixtime"),
    ("temperature_unit", "celsius"),
    ("past_days", "1"),
    ("forecast_days", "2"),
];

/// Blocks until the response body arrives or the request fails.
pub fn fetch(coordinates: Coordinates) -> Result<String, FetchError> {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .get(FORECAST_URL)
        .query("latitude", coordinates.latitude.to_string())
        .query("longitude", coordinates.longitude.to_string())
        .query_pairs(FORECAST_QUERY)
        .call()
        .map_err(FetchError::from)?;
    response
        .body_mut()
        .read_to_string()
        .map_err(FetchError::from)
}

impl From<ureq::Error> for FetchError {
    fn from(error: ureq::Error) -> Self {
        match error {
            ureq::Error::StatusCode(status) => Self::Status(status),
            other => Self::Network(other.to_string()),
        }
    }
}
