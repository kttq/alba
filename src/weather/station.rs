use std::path::PathBuf;

use jiff::{SignedDuration, Timestamp, Zoned};

use super::{
    FetchError, Forecast, Freshness, LastFetchResult, Weather, WeatherSource, cache, next_fetch,
};
use crate::{
    config::Coordinates,
    log::{Level, Log},
};

/// A quick fetch finishes before the indicator shows, so it never flickers.
const FETCH_INDICATOR_DELAY: SignedDuration = SignedDuration::from_millis(300);

/// What the weather station has to show now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeatherState<'a> {
    NoLocation,
    /// No usable weather, and no fetch has failed yet.
    Fetching,
    /// No usable weather, and the last fetch failed.
    Offline,
    Fresh(&'a Weather),
    Stale {
        weather: &'a Weather,
        age: SignedDuration,
    },
}

impl<'a> WeatherState<'a> {
    /// The weather to show, fresh or stale.
    pub fn weather(self) -> Option<&'a Weather> {
        match self {
            Self::Fresh(weather) | Self::Stale { weather, .. } => Some(weather),
            _ => None,
        }
    }
}

/// Keeps the current weather: from the cache at start, then from fetches on the fetch
/// schedule or on request.
pub struct WeatherStation {
    coordinates: Option<Coordinates>,
    weather_source: Box<dyn WeatherSource>,
    cache_directory: PathBuf,
    weather: Option<Weather>,
    /// Failures in a row since the last success.
    failures: u32,
    next_fetch: Timestamp,
    fetch_started_at: Option<Timestamp>,
    refresh_requested: bool,
}

impl WeatherStation {
    pub fn new(
        coordinates: Option<Coordinates>,
        weather_source: Box<dyn WeatherSource>,
        cache_directory: PathBuf,
    ) -> Self {
        let weather =
            coordinates.and_then(|coordinates| cache::read(&cache_directory, coordinates));
        let next_fetch = match &weather {
            Some(weather) => next_fetch(LastFetchResult::Success, weather.fetched_at),
            None => Timestamp::MIN,
        };
        Self {
            coordinates,
            weather_source,
            cache_directory,
            weather,
            failures: 0,
            next_fetch,
            fetch_started_at: None,
            refresh_requested: false,
        }
    }

    pub fn weather_state(&self, now: &Zoned) -> WeatherState<'_> {
        if self.coordinates.is_none() {
            return WeatherState::NoLocation;
        }
        let usable = self
            .weather
            .as_ref()
            .map(|weather| (weather, weather.freshness(now.timestamp())));
        match usable {
            Some((weather, Freshness::Fresh)) => WeatherState::Fresh(weather),
            Some((weather, Freshness::Stale(age))) => WeatherState::Stale { weather, age },
            _ if self.failures > 0 => WeatherState::Offline,
            _ => WeatherState::Fetching,
        }
    }

    pub fn shows_fetch_indicator(&self, now: &Zoned) -> bool {
        self.fetch_started_at.is_some_and(|started_at| {
            now.timestamp().duration_since(started_at) >= FETCH_INDICATOR_DELAY
        })
    }

    /// The next tick starts a fetch, unless one is running.
    pub fn request_refresh(&mut self) {
        if self.fetch_started_at.is_none() {
            self.refresh_requested = true;
        }
    }

    pub fn tick(&mut self, now: &Zoned, log: &mut Log) {
        let Some(coordinates) = self.coordinates else {
            return;
        };
        if let Some(result) = self.weather_source.finished() {
            self.fetch_started_at = None;
            self.finish_fetch(coordinates, result, now, log);
        }
        let due = self.refresh_requested || now.timestamp() >= self.next_fetch;
        if due && self.fetch_started_at.is_none() {
            self.weather_source.request(coordinates);
            self.fetch_started_at = Some(now.timestamp());
            self.refresh_requested = false;
        }
    }

    fn finish_fetch(
        &mut self,
        coordinates: Coordinates,
        result: Result<String, FetchError>,
        now: &Zoned,
        log: &mut Log,
    ) {
        let last_fetch_result = match result.and_then(|body| Forecast::parse(&body)) {
            Ok(forecast) => {
                self.receive(coordinates, forecast, now, log);
                self.failures = 0;
                LastFetchResult::Success
            }
            Err(error) => {
                log.write(Level::Warn, now, "fetch failed", &[("error", &error)]);
                self.failures += 1;
                LastFetchResult::Failure {
                    failures: self.failures,
                    retry_after: error.retry_after(),
                }
            }
        };
        self.next_fetch = next_fetch(last_fetch_result, now.timestamp());
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
