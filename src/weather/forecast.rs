use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::{Condition, FetchError};

const PRECIPITATION_HOURS: usize = 12;
const DAY_SECONDS: i64 = 24 * 60 * 60;

/// Fields keep their Open-Meteo names. Times are Unix seconds; temperatures are Celsius.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Forecast {
    utc_offset_seconds: i32,
    current: Current,
    daily: Daily,
    hourly: Hourly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Current {
    time: i64,
    temperature_2m: f64,
    weather_code: u16,
    is_day: IsDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum IsDay {
    Night = 0,
    Day = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayOrNight {
    Day,
    Night,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunEvent {
    Sunrise(Timestamp),
    Sunset(Timestamp),
}

impl SunEvent {
    pub fn at(self) -> Timestamp {
        match self {
            Self::Sunrise(at) | Self::Sunset(at) => at,
        }
    }
}

/// Celsius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighAndLow {
    pub high: f64,
    pub low: f64,
}

/// Yesterday through tomorrow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Daily {
    time: Vec<i64>,
    sunrise: Vec<i64>,
    sunset: Vec<i64>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

/// Each value covers the hour before its time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Hourly {
    time: Vec<i64>,
    /// Percent.
    precipitation_probability: Vec<u8>,
}

/// Only for times that `validate` checked.
fn timestamp(second: i64) -> Timestamp {
    Timestamp::from_second(second).expect("validated time")
}

impl Forecast {
    pub fn parse(body: &str) -> Result<Self, FetchError> {
        let forecast: Self =
            serde_json::from_str(body).map_err(|error| FetchError::Invalid(error.to_string()))?;
        forecast.validate().map_err(FetchError::Invalid)?;
        Ok(forecast)
    }

    /// Celsius.
    pub fn temperature(&self) -> f64 {
        self.current.temperature_2m
    }

    /// Open-Meteo's own answer, for when no sunrise and sunset surround now.
    pub fn is_day(&self) -> DayOrNight {
        match self.current.is_day {
            IsDay::Day => DayOrNight::Day,
            IsDay::Night => DayOrNight::Night,
        }
    }

    /// Sorted, without polar sentinel times.
    pub fn sun_events(&self) -> Vec<SunEvent> {
        let daily = &self.daily;
        let mut events: Vec<SunEvent> = (0..daily.time.len())
            .filter(|&day| !self.is_polar(day))
            .flat_map(|day| {
                [
                    SunEvent::Sunrise(timestamp(daily.sunrise[day])),
                    SunEvent::Sunset(timestamp(daily.sunset[day])),
                ]
            })
            .collect();
        events.sort_by_key(|event| event.at());
        events
    }

    /// Open-Meteo marks a day without sunrise or sunset with its midnights: sunrise at the
    /// day's midnight, and sunset at the same or the next one. A looser rule would catch a real
    /// sunset after the next midnight, as in an Arctic summer.
    fn is_polar(&self, day: usize) -> bool {
        let midnight = self.daily.time[day];
        let sunset = self.daily.sunset[day];
        self.daily.sunrise[day] == midnight
            && (sunset == midnight || sunset == midnight + DAY_SECONDS)
    }

    pub fn condition(&self) -> Option<Condition> {
        Condition::from_wmo(self.current.weather_code)
    }

    /// From the forecast day's row.
    pub fn high_and_low(&self, now: Timestamp) -> Option<HighAndLow> {
        let now = now.as_second();
        let row = (self.daily.time.iter())
            .position(|&midnight| midnight <= now && now < midnight + DAY_SECONDS)?;
        Some(HighAndLow {
            high: self.daily.temperature_2m_max[row],
            low: self.daily.temperature_2m_min[row],
        })
    }

    /// Percents, for up to the next 12 hours.
    pub fn precipitation_after(&self, now: Timestamp) -> &[u8] {
        let now = now.as_second();
        let first = self.hourly.time.partition_point(|&time| time <= now);
        let probabilities = &self.hourly.precipitation_probability[first..];
        &probabilities[..probabilities.len().min(PRECIPITATION_HOURS)]
    }

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
        let times = [&self.daily.sunrise, &self.daily.sunset]
            .into_iter()
            .flatten();
        if let Some(time) = times
            .copied()
            .find(|&time| Timestamp::from_second(time).is_err())
        {
            return Err(format!("time {time} is out of range"));
        }
        Ok(())
    }
}
