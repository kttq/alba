//! A config and a weather cache in a scratch `HOME`, written the way alba itself would.

use std::{ffi::OsString, fmt::Display, fs, io, path::Path};

use alba::{
    config::Coordinates,
    paths::Paths,
    weather::{Forecast, Weather, cache},
};
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

const DAY: i64 = 86_400;
/// Open-Meteo stamps `current` at the start of its 15-minute interval.
const CURRENT_INTERVAL: i64 = 900;

/// How old the cached weather is. Each age lands in its freshness band: stale after 60 minutes,
/// expired after 12 hours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CachedWeather {
    Fresh,
    Stale,
    Expired,
    NoCache,
}

impl CachedWeather {
    fn default_age(self) -> SignedDuration {
        match self {
            Self::Fresh | Self::NoCache => SignedDuration::from_mins(5),
            Self::Stale => SignedDuration::from_mins(90),
            Self::Expired => SignedDuration::from_hours(13),
        }
    }
}

/// The cache is `tests/fixtures/forecast.json` (Buenos Aires, rain, 14°, day), shifted by whole
/// days so its middle daily row is today at the forecast's UTC offset.
#[derive(Clone, Debug)]
pub struct FixtureHome {
    pub cached_weather: CachedWeather,
    /// Overrides the cached weather's default age.
    pub age: Option<SignedDuration>,
    pub weather_code: Option<u16>,
    pub night: bool,
    pub has_location: bool,
    pub config_lines: Vec<String>,
}

impl Default for FixtureHome {
    fn default() -> Self {
        Self {
            cached_weather: CachedWeather::Fresh,
            age: None,
            weather_code: None,
            night: false,
            has_location: true,
            config_lines: Vec::new(),
        }
    }
}

impl FixtureHome {
    pub fn write(&self, home: &Path, now: Timestamp) -> io::Result<()> {
        let paths = Paths::resolve(|name| (name == "HOME").then(|| OsString::from(home)))
            .map_err(failed)?;
        let mut forecast: Value = serde_json::from_str(&fixture_forecast()?)?;
        let coordinates = Coordinates {
            latitude: number(&forecast["latitude"])?,
            longitude: number(&forecast["longitude"])?,
        };
        let weather = match self.cached_weather {
            CachedWeather::NoCache => None,
            _ => Some(self.weather(&mut forecast, now)?),
        };
        write_file(&paths.config_file, &self.config(coordinates))?;
        match weather {
            None => Ok(()),
            Some(weather) => cache::write(&paths.cache_directory, coordinates, &weather),
        }
    }

    fn weather(&self, forecast: &mut Value, now: Timestamp) -> io::Result<Weather> {
        let fetched_at = now - self.age.unwrap_or(self.cached_weather.default_age());
        self.edit(forecast, now, fetched_at)?;
        Ok(Weather {
            fetched_at,
            forecast: Forecast::parse(&forecast.to_string()).map_err(failed)?,
        })
    }

    /// Writes the coordinates exactly as the cache stores them, so alba uses the cache.
    fn config(&self, coordinates: Coordinates) -> String {
        let location = self.has_location.then(|| {
            format!(
                "coordinates = [{}, {}]",
                coordinates.latitude, coordinates.longitude
            )
        });
        location
            .into_iter()
            .chain(self.config_lines.iter().cloned())
            .map(|line| line + "\n")
            .collect()
    }

    fn edit(&self, forecast: &mut Value, now: Timestamp, fetched_at: Timestamp) -> io::Result<()> {
        shift_to_today(forecast, now)?;
        let fetched_second = fetched_at.as_second();
        let current = &mut forecast["current"];
        current["time"] = (fetched_second - fetched_second.rem_euclid(CURRENT_INTERVAL)).into();
        if let Some(weather_code) = self.weather_code {
            current["weather_code"] = weather_code.into();
        }
        if self.night {
            current["is_day"] = 0.into();
        }
        Ok(())
    }
}

/// Moves every time by whole days, so sunrise, sunset, and midnights keep their clock times.
fn shift_to_today(forecast: &mut Value, now: Timestamp) -> io::Result<()> {
    let offset = integer(&forecast["utc_offset_seconds"])?;
    let fixture_today = (integer(&forecast["daily"]["time"][1])? + offset).div_euclid(DAY);
    let today = (now.as_second() + offset).div_euclid(DAY);
    let shift = (today - fixture_today) * DAY;
    for (section, column) in [
        ("daily", "time"),
        ("daily", "sunrise"),
        ("daily", "sunset"),
        ("hourly", "time"),
    ] {
        let Some(times) = forecast[section][column].as_array_mut() else {
            return Err(invalid(&format!("{section}.{column} is not a list")));
        };
        for time in times {
            *time = (integer(time)? + shift).into();
        }
    }
    Ok(())
}

fn fixture_forecast() -> io::Result<String> {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/forecast.json"))
}

fn write_file(path: &Path, text: &str) -> io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(path, text)
}

fn integer(value: &Value) -> io::Result<i64> {
    value
        .as_i64()
        .ok_or_else(|| invalid(&format!("{value} is not an integer")))
}

fn number(value: &Value) -> io::Result<f64> {
    value
        .as_f64()
        .ok_or_else(|| invalid(&format!("{value} is not a number")))
}

fn invalid(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

fn failed(error: impl Display) -> io::Error {
    io::Error::other(error.to_string())
}
