use std::convert::Infallible;

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::{Condition, FetchError};

const PRECIPITATION_HOURS: usize = 12;
const ONE_DAY: SignedDuration = SignedDuration::from_hours(24);

/// Fields keep their Open-Meteo names; temperatures are Celsius.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ForecastColumns", into = "ForecastColumns")]
pub struct Forecast {
    utc_offset_seconds: i32,
    current: Current,
    daily: Daily<Timestamp>,
    hourly: Hourly<Timestamp>,
}

/// The forecast as Open-Meteo sends it and the cache stores it, with times in Unix seconds.
#[derive(Serialize, Deserialize)]
struct ForecastColumns {
    utc_offset_seconds: i32,
    current: Current,
    daily: Daily<i64>,
    hourly: Hourly<i64>,
}

impl TryFrom<ForecastColumns> for Forecast {
    type Error = String;

    fn try_from(columns: ForecastColumns) -> Result<Self, String> {
        let forecast = Self {
            utc_offset_seconds: columns.utc_offset_seconds,
            current: columns.current,
            daily: columns.daily.map_times(timestamp)?,
            hourly: columns.hourly.map_times(timestamp)?,
        };
        forecast.validate()?;
        Ok(forecast)
    }
}

impl From<Forecast> for ForecastColumns {
    fn from(forecast: Forecast) -> Self {
        let second = |time: Timestamp| Ok::<_, Infallible>(time.as_second());
        let (Ok(daily), Ok(hourly)) = (
            forecast.daily.map_times(second),
            forecast.hourly.map_times(second),
        );
        Self {
            utc_offset_seconds: forecast.utc_offset_seconds,
            current: forecast.current,
            daily,
            hourly,
        }
    }
}

fn timestamp(second: i64) -> Result<Timestamp, String> {
    Timestamp::from_second(second).map_err(|_| format!("time {second} is out of range"))
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
    /// A day with no sunrise or sunset, at its midnight.
    Polar(Timestamp),
}

impl SunEvent {
    pub fn at(self) -> Timestamp {
        match self {
            Self::Sunrise(at) | Self::Sunset(at) | Self::Polar(at) => at,
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
struct Daily<Time> {
    time: Vec<Time>,
    sunrise: Vec<Time>,
    sunset: Vec<Time>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

impl<Time> Daily<Time> {
    fn map_times<Mapped, Error>(
        self,
        mut map: impl FnMut(Time) -> Result<Mapped, Error>,
    ) -> Result<Daily<Mapped>, Error> {
        let mut map_column =
            |column: Vec<Time>| column.into_iter().map(&mut map).collect::<Result<_, _>>();
        Ok(Daily {
            time: map_column(self.time)?,
            sunrise: map_column(self.sunrise)?,
            sunset: map_column(self.sunset)?,
            temperature_2m_max: self.temperature_2m_max,
            temperature_2m_min: self.temperature_2m_min,
        })
    }
}

/// Each value covers the hour before its time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Hourly<Time> {
    time: Vec<Time>,
    /// Percent.
    precipitation_probability: Vec<u8>,
}

impl<Time> Hourly<Time> {
    fn map_times<Mapped, Error>(
        self,
        map: impl FnMut(Time) -> Result<Mapped, Error>,
    ) -> Result<Hourly<Mapped>, Error> {
        Ok(Hourly {
            time: self.time.into_iter().map(map).collect::<Result<_, _>>()?,
            precipitation_probability: self.precipitation_probability,
        })
    }
}

impl Forecast {
    pub fn parse(body: &str) -> Result<Self, FetchError> {
        serde_json::from_str(body).map_err(|error| FetchError::Invalid(error.to_string()))
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

    /// Sorted. A polar sentinel day gives one `Polar` event instead of its times.
    pub fn sun_events(&self) -> Vec<SunEvent> {
        let mut events: Vec<SunEvent> = (0..self.daily.time.len())
            .flat_map(|day| self.sun_events_of(day))
            .collect();
        events.sort_by_key(|event| event.at());
        events
    }

    fn sun_events_of(&self, day: usize) -> Vec<SunEvent> {
        let daily = &self.daily;
        if self.is_polar(day) {
            return vec![SunEvent::Polar(daily.time[day])];
        }
        vec![
            SunEvent::Sunrise(daily.sunrise[day]),
            SunEvent::Sunset(daily.sunset[day]),
        ]
    }

    /// Open-Meteo marks a day without sunrise or sunset with its midnights: sunrise at the
    /// day's midnight, and sunset at the same or the next one. A real sunset can fall after the
    /// next midnight, so a day of 24 hours or more is not enough.
    fn is_polar(&self, day: usize) -> bool {
        let midnight = self.daily.time[day];
        let sunset = self.daily.sunset[day];
        self.daily.sunrise[day] == midnight && (sunset == midnight || sunset == midnight + ONE_DAY)
    }

    pub fn condition(&self) -> Option<Condition> {
        Condition::from_wmo(self.current.weather_code)
    }

    /// From the forecast day's row.
    pub fn high_and_low(&self, now: Timestamp) -> Option<HighAndLow> {
        let row = (self.daily.time.iter())
            .position(|&midnight| midnight <= now && now < midnight + ONE_DAY)?;
        Some(HighAndLow {
            high: self.daily.temperature_2m_max[row],
            low: self.daily.temperature_2m_min[row],
        })
    }

    /// Percents, for up to the next 12 hours.
    pub fn precipitation_after(&self, now: Timestamp) -> &[u8] {
        let first = self.hourly.time.partition_point(|&time| time <= now);
        let probabilities = &self.hourly.precipitation_probability[first..];
        &probabilities[..probabilities.len().min(PRECIPITATION_HOURS)]
    }

    fn validate(&self) -> Result<(), String> {
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
        if !self.daily.time.is_sorted() || !self.hourly.time.is_sorted() {
            return Err("daily or hourly times are out of order".to_string());
        }
        // Sorted, so the last midnight is the latest; the day math adds a day to it.
        if let Some(midnight) = self.daily.time.last()
            && midnight.checked_add(ONE_DAY).is_err()
        {
            return Err(format!("day of {midnight} ends out of range"));
        }
        Ok(())
    }
}
