//! Tests use a fake fetch instead, so they never touch the network.

use std::time::Duration;

use jiff::SignedDuration;
use ureq::{Agent, Body, http::Response};

use super::{FetchError, Request};
use crate::{config::Coordinates, geocode};

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
pub fn fetch(request: Request) -> Result<String, FetchError> {
    match request {
        Request::Forecast(coordinates) => forecast(coordinates),
        Request::Lookup { name, language } => geocode::look_up(&name, language),
    }
}

fn forecast(coordinates: Coordinates) -> Result<String, FetchError> {
    let latitude = coordinates.latitude.to_string();
    let longitude = coordinates.longitude.to_string();
    let mut query = vec![
        ("latitude", latitude.as_str()),
        ("longitude", longitude.as_str()),
    ];
    query.extend(FORECAST_QUERY);
    get(FORECAST_URL, &query)
}

/// Blocks until the response body arrives or the request fails. A body that is not valid
/// UTF-8 is invalid. `ureq` encodes the query values.
pub fn get(url: &str, query: &[(&str, &str)]) -> Result<String, FetchError> {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = agent
        .get(url)
        .query_pairs(query.iter().copied())
        .call()
        .map_err(FetchError::from)?;
    if !response.status().is_success() {
        return Err(status_error(&response));
    }
    let body = response
        .body_mut()
        .read_to_vec()
        .map_err(FetchError::from)?;
    String::from_utf8(body).map_err(|error| FetchError::Invalid(error.to_string()))
}

/// Reads `Retry-After` in seconds only; an HTTP date counts as no hint.
fn status_error(response: &Response<Body>) -> FetchError {
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u32>().ok())
        .map(|seconds| SignedDuration::from_secs(seconds.into()));
    FetchError::Status {
        status: response.status().as_u16(),
        retry_after,
    }
}

impl From<ureq::Error> for FetchError {
    fn from(error: ureq::Error) -> Self {
        Self::Network(error.to_string())
    }
}
