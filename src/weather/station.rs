use std::path::PathBuf;

use jiff::{SignedDuration, Timestamp, Zoned};

use super::{
    FetchError, Forecast, Freshness, LastFetchResult, Request, ResolvedLocation, Weather,
    WeatherSource, cache, next_fetch,
};
use crate::{
    config::{Language, Location},
    geocode,
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
    /// Geocoding found no place by the configured name.
    LocationNotFound(&'a str),
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
    location: Option<Location>,
    /// Of the place name that geocoding returns.
    language: Language,
    weather_source: Box<dyn WeatherSource>,
    cache_directory: PathBuf,
    /// None while a configured name is not resolved.
    resolved_location: Option<ResolvedLocation>,
    weather: Option<Weather>,
    /// Holds until `r`, because an automatic retry would find no match again.
    location_not_found: bool,
    /// Failures in a row since the last success.
    failures: u32,
    next_fetch: Timestamp,
    running_fetch: Option<RunningFetch>,
    refresh_requested: bool,
}

/// A fetch for a name that is not resolved yet starts with a lookup.
struct RunningFetch {
    started_at: Timestamp,
    step: FetchStep,
}

#[derive(Clone, Copy)]
enum FetchStep {
    Lookup,
    Forecast,
}

impl WeatherStation {
    pub fn new(
        location: Option<Location>,
        language: Language,
        weather_source: Box<dyn WeatherSource>,
        cache_directory: PathBuf,
    ) -> Self {
        let (weather, cached_location) = location
            .as_ref()
            .and_then(|location| cache::read(&cache_directory, location))
            .unzip();
        let resolved_location = cached_location.or(match location {
            Some(Location::Coordinates(coordinates)) => Some(ResolvedLocation {
                coordinates,
                place_name: None,
            }),
            _ => None,
        });
        let next_fetch = match &weather {
            Some(weather) => next_fetch(LastFetchResult::Success, weather.fetched_at),
            None => Timestamp::MIN,
        };
        Self {
            location,
            language,
            weather_source,
            cache_directory,
            resolved_location,
            weather,
            location_not_found: false,
            failures: 0,
            next_fetch,
            running_fetch: None,
            refresh_requested: false,
        }
    }

    pub fn weather_state(&self, now: &Zoned) -> WeatherState<'_> {
        let Some(location) = &self.location else {
            return WeatherState::NoLocation;
        };
        if self.location_not_found
            && let Location::Name(name) = location
        {
            return WeatherState::LocationNotFound(name);
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

    /// Lowercased, with the country code. None for configured coordinates, and for a name not
    /// looked up yet.
    pub fn place_name(&self) -> Option<&str> {
        self.resolved_location.as_ref()?.place_name.as_deref()
    }

    pub fn shows_fetch_indicator(&self, now: &Zoned) -> bool {
        self.running_fetch.as_ref().is_some_and(|running_fetch| {
            now.timestamp().duration_since(running_fetch.started_at) >= FETCH_INDICATOR_DELAY
        })
    }

    /// The next tick starts a fetch, unless one is running.
    pub fn request_refresh(&mut self) {
        if self.running_fetch.is_none() {
            self.refresh_requested = true;
        }
    }

    pub fn tick(&mut self, now: &Zoned, log: &mut Log) {
        if self.location.is_none() {
            return;
        }
        if let Some(result) = self.weather_source.finished() {
            self.finish_step(result, now, log);
        }
        let due = self.refresh_requested
            || (!self.location_not_found && now.timestamp() >= self.next_fetch);
        if due && self.running_fetch.is_none() {
            self.refresh_requested = false;
            self.location_not_found = false;
            self.start_step(now.timestamp());
        }
    }

    fn start_step(&mut self, started_at: Timestamp) {
        let (request, step) = match (&self.resolved_location, &self.location) {
            (Some(resolved_location), _) => (
                Request::Forecast(resolved_location.coordinates),
                FetchStep::Forecast,
            ),
            (None, Some(Location::Name(name))) => (
                Request::Lookup {
                    name: name.clone(),
                    language: self.language,
                },
                FetchStep::Lookup,
            ),
            (None, _) => return,
        };
        self.weather_source.request(request);
        self.running_fetch = Some(RunningFetch { started_at, step });
    }

    fn finish_step(&mut self, result: Result<String, FetchError>, now: &Zoned, log: &mut Log) {
        let Some(running_fetch) = self.running_fetch.take() else {
            return;
        };
        match running_fetch.step {
            FetchStep::Lookup => self.finish_lookup(running_fetch.started_at, result, now, log),
            FetchStep::Forecast => self.finish_forecast(result, now, log),
        }
    }

    /// A found place goes straight on to its forecast, as part of the same fetch.
    fn finish_lookup(
        &mut self,
        started_at: Timestamp,
        result: Result<String, FetchError>,
        now: &Zoned,
        log: &mut Log,
    ) {
        match result.and_then(|body| geocode::parse(&body)) {
            Ok(Some(place)) => {
                self.resolved_location = Some(place.into());
                self.start_step(started_at);
            }
            Ok(None) => {
                log.write(Level::Warn, now, "location not found", &[]);
                self.location_not_found = true;
            }
            Err(error) => self.fail("lookup failed", error, now, log),
        }
    }

    fn finish_forecast(&mut self, result: Result<String, FetchError>, now: &Zoned, log: &mut Log) {
        match result.and_then(|body| Forecast::parse(&body)) {
            Ok(forecast) => {
                self.receive(forecast, now, log);
                self.failures = 0;
                self.next_fetch = next_fetch(LastFetchResult::Success, now.timestamp());
            }
            Err(error) => self.fail("fetch failed", error, now, log),
        }
    }

    fn fail(&mut self, message: &str, error: FetchError, now: &Zoned, log: &mut Log) {
        log.write(Level::Warn, now, message, &[("error", &error)]);
        self.failures += 1;
        let last_fetch_result = LastFetchResult::Failure {
            failures: self.failures,
            retry_after: error.retry_after(),
        };
        self.next_fetch = next_fetch(last_fetch_result, now.timestamp());
    }

    fn receive(&mut self, forecast: Forecast, now: &Zoned, log: &mut Log) {
        let weather = Weather {
            fetched_at: now.timestamp(),
            forecast,
        };
        if let (Some(location), Some(resolved_location)) = (&self.location, &self.resolved_location)
            && let Err(error) =
                cache::write(&self.cache_directory, location, resolved_location, &weather)
        {
            log.write(Level::Warn, now, "cache write failed", &[("error", &error)]);
        }
        self.weather = Some(weather);
    }
}
