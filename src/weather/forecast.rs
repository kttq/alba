use serde::{Deserialize, Serialize};

use super::FetchError;

/// The Open-Meteo response fields alba uses, under their Open-Meteo names. Times are Unix
/// seconds and temperatures are Celsius. A `null` anywhere fails to parse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Forecast {
    pub utc_offset_seconds: i32,
    pub current: Current,
    pub daily: Daily,
    pub hourly: Hourly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Current {
    pub time: i64,
    pub temperature_2m: f64,
    /// Kept raw, so a change to the condition table needs no cache migration.
    pub weather_code: u16,
    /// 1 by day, 0 by night.
    pub is_day: u8,
}

/// One entry per day, yesterday through tomorrow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Daily {
    pub time: Vec<i64>,
    pub sunrise: Vec<i64>,
    pub sunset: Vec<i64>,
    pub temperature_2m_max: Vec<f64>,
    pub temperature_2m_min: Vec<f64>,
}

/// One entry per hour. Each value covers the hour before its time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hourly {
    pub time: Vec<i64>,
    /// Percent.
    pub precipitation_probability: Vec<u8>,
}

impl Forecast {
    /// Rejects anything but a complete forecast: malformed JSON, an HTML or empty body, a
    /// `null`, or columns of different lengths.
    pub fn parse(body: &str) -> Result<Self, FetchError> {
        let forecast: Self =
            serde_json::from_str(body).map_err(|error| FetchError::Invalid(error.to_string()))?;
        forecast.validate().map_err(FetchError::Invalid)?;
        Ok(forecast)
    }

    /// Every column of a table has one value per row.
    pub fn validate(&self) -> Result<(), String> {
        let days = self.daily.time.len();
        let daily_columns = [
            self.daily.sunrise.len(),
            self.daily.sunset.len(),
            self.daily.temperature_2m_max.len(),
            self.daily.temperature_2m_min.len(),
        ];
        if daily_columns.iter().any(|&length| length != days) {
            return Err(format!("daily columns differ in length from {days} days"));
        }
        let hours = self.hourly.time.len();
        if self.hourly.precipitation_probability.len() != hours {
            return Err(format!(
                "hourly columns differ in length from {hours} hours"
            ));
        }
        Ok(())
    }
}
