use std::path::PathBuf;

use jiff::Zoned;

use super::{Forecast, Weather, WeatherSource, cache};
use crate::{
    config::Coordinates,
    log::{Level, Log},
};

/// Keeps the current weather: from the cache at start, then from one fetch.
pub struct WeatherStation {
    coordinates: Option<Coordinates>,
    weather_source: Box<dyn WeatherSource>,
    cache_directory: PathBuf,
    weather: Option<Weather>,
    fetch_requested: bool,
}

impl WeatherStation {
    pub fn new(
        coordinates: Option<Coordinates>,
        weather_source: Box<dyn WeatherSource>,
        cache_directory: PathBuf,
    ) -> Self {
        let weather =
            coordinates.and_then(|coordinates| cache::read(&cache_directory, coordinates));
        Self {
            coordinates,
            weather_source,
            cache_directory,
            weather,
            fetch_requested: false,
        }
    }

    pub fn weather(&self) -> Option<&Weather> {
        self.weather.as_ref()
    }

    pub fn tick(&mut self, now: &Zoned, log: &mut Log) {
        let Some(coordinates) = self.coordinates else {
            return;
        };
        if let Some(result) = self.weather_source.finished() {
            match result.and_then(|body| Forecast::parse(&body)) {
                Ok(forecast) => self.receive(coordinates, forecast, now, log),
                Err(error) => log.write(Level::Warn, now, "fetch failed", &[("error", &error)]),
            }
        }
        if !self.fetch_requested {
            self.weather_source.request(coordinates);
            self.fetch_requested = true;
        }
    }

    fn receive(
        &mut self,
        coordinates: Coordinates,
        forecast: Forecast,
        now: &Zoned,
        log: &mut Log,
    ) {
        let weather = Weather {
            fetched_at: now.timestamp(),
            forecast,
        };
        if let Err(error) = cache::write(&self.cache_directory, coordinates, &weather) {
            log.write(Level::Warn, now, "cache write failed", &[("error", &error)]);
        }
        self.weather = Some(weather);
    }
}
