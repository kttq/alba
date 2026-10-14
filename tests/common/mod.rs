#![allow(dead_code)]

use std::{
    cell::RefCell,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use alba::{
    app::App,
    config::{Config, Coordinates},
    log::Log,
    weather::{FetchError, Forecast, WeatherSource, WeatherStation},
};
use jiff::{Timestamp, Zoned};
use ratatui::{Terminal, backend::TestBackend};
use serde_json::{Value, json};

pub const BUENOS_AIRES: &str = "coordinates = [-34.6037, -58.3816]";

pub const BUENOS_AIRES_COORDINATES: Coordinates = Coordinates {
    latitude: -34.6037,
    longitude: -58.3816,
};

pub fn default_instant() -> Zoned {
    "2026-10-13T06:38[America/Argentina/Buenos_Aires]"
        .parse()
        .unwrap()
}

/// An Open-Meteo response with rain at 14.4 °C.
pub fn forecast_body() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/forecast.json"))
        .unwrap()
}

pub fn fixture_forecast(name: &str) -> Forecast {
    edited_fixture_forecast(name, |_| {})
}

pub fn edited_fixture_forecast(name: &str, edit: impl FnOnce(&mut Value)) -> Forecast {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let mut forecast: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    edit(&mut forecast);
    Forecast::parse(&forecast.to_string()).unwrap()
}

/// 2026-10-13 00:00 UTC.
pub const MIDNIGHT: i64 = 1_791_849_600;
pub const HOUR: i64 = 3600;
pub const DAY: i64 = 24 * HOUR;

pub fn at(second: i64) -> Timestamp {
    Timestamp::from_second(second).unwrap()
}

/// Days 12 to 14 October at UTC, each with sunrise 06:00 and sunset 18:00, highs 21, 22, 23,
/// and lows 11, 12, 13.
pub fn three_day_forecast() -> Forecast {
    small_forecast(|forecast| {
        let midnights = [MIDNIGHT - DAY, MIDNIGHT, MIDNIGHT + DAY];
        forecast["daily"] = json!({
            "time": midnights,
            "sunrise": midnights.map(|midnight| midnight + 6 * HOUR),
            "sunset": midnights.map(|midnight| midnight + 18 * HOUR),
            "temperature_2m_max": [21.0, 22.0, 23.0],
            "temperature_2m_min": [11.0, 12.0, 13.0],
        });
    })
}

/// A one-day forecast at UTC offset 0, changed by `edit`: sunrise 06:00, sunset 18:00, no hours.
pub fn small_forecast(edit: impl FnOnce(&mut Value)) -> Forecast {
    let mut forecast = json!({
        "utc_offset_seconds": 0,
        "current": { "time": MIDNIGHT, "temperature_2m": 10.0, "weather_code": 0, "is_day": 1 },
        "daily": {
            "time": [MIDNIGHT],
            "sunrise": [MIDNIGHT + 6 * 3600],
            "sunset": [MIDNIGHT + 18 * 3600],
            "temperature_2m_max": [15.0],
            "temperature_2m_min": [5.0],
        },
        "hourly": { "time": [], "precipitation_probability": [] },
    });
    edit(&mut forecast);
    Forecast::parse(&forecast.to_string()).unwrap()
}

#[derive(Clone, Default)]
pub struct FakeWeatherSource(Rc<RefCell<Fetches>>);

#[derive(Default)]
struct Fetches {
    requests: Vec<Coordinates>,
    finished: Option<Result<String, FetchError>>,
}

impl FakeWeatherSource {
    pub fn requests(&self) -> Vec<Coordinates> {
        self.0.borrow().requests.clone()
    }

    pub fn finish(&self, result: Result<String, FetchError>) {
        self.0.borrow_mut().finished = Some(result);
    }
}

impl WeatherSource for FakeWeatherSource {
    fn request(&mut self, coordinates: Coordinates) {
        self.0.borrow_mut().requests.push(coordinates);
    }

    fn finished(&mut self) -> Option<Result<String, FetchError>> {
        self.0.borrow_mut().finished.take()
    }
}

pub struct Harness {
    pub app: App,
    pub weather_source: FakeWeatherSource,
    pub cache_directory: PathBuf,
    pub state_directory: PathBuf,
}

impl Harness {
    pub fn new(config_text: &str) -> Self {
        Self::with_cache(config_text, temporary_directory())
    }

    pub fn with_cache(config_text: &str, cache_directory: PathBuf) -> Self {
        Self::with_directories(config_text, cache_directory, temporary_directory())
    }

    pub fn with_directories(
        config_text: &str,
        cache_directory: PathBuf,
        state_directory: PathBuf,
    ) -> Self {
        let weather_source = FakeWeatherSource::default();
        let app = App::new(
            &Config::parse(config_text).unwrap(),
            Box::new(weather_source.clone()),
            cache_directory.clone(),
            state_directory.clone(),
        );
        Self {
            app,
            weather_source,
            cache_directory,
            state_directory,
        }
    }

    pub fn log(&self) -> String {
        fs::read_to_string(self.state_directory.join("alba.log")).unwrap()
    }

    pub fn fetch(&mut self, now: &Zoned, result: Result<String, FetchError>) {
        self.app.tick(now);
        self.weather_source.finish(result);
        self.app.tick(now);
    }

    pub fn render_row(&self, now: &Zoned, width: u16) -> Terminal<TestBackend> {
        render_row(&self.app, now, width)
    }

    pub fn text_line(&self, now: &Zoned, width: u16) -> String {
        let terminal = self.render_row(now, width);
        let line: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        line.trim().to_string()
    }
}

/// A weather station for Buenos Aires, with its own log.
pub struct WeatherStationHarness {
    pub weather_station: WeatherStation,
    pub weather_source: FakeWeatherSource,
    pub log: Log,
}

impl WeatherStationHarness {
    pub fn with_cache(cache_directory: PathBuf) -> Self {
        let weather_source = FakeWeatherSource::default();
        let weather_station = WeatherStation::new(
            Some(BUENOS_AIRES_COORDINATES),
            Box::new(weather_source.clone()),
            cache_directory,
        );
        Self {
            weather_station,
            weather_source,
            log: Log::start(&temporary_directory()),
        }
    }

    pub fn tick(&mut self, now: &Zoned) {
        self.weather_station.tick(now, &mut self.log);
    }

    /// Finishes the running fetch, then ticks so the station takes the result.
    pub fn finish(&mut self, now: &Zoned, result: Result<String, FetchError>) {
        self.weather_source.finish(result);
        self.tick(now);
    }

    pub fn fetches(&self) -> usize {
        self.weather_source.requests().len()
    }
}

pub fn render_row(app: &App, now: &Zoned, width: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
    terminal
        .draw(|frame| app.render(frame.area(), frame.buffer_mut(), now))
        .unwrap();
    terminal
}

/// Returns a path only; the directory does not exist yet.
pub fn temporary_directory() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let number = NEXT.fetch_add(1, Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("alba-cache-{}-{number}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    directory
}

pub fn make_read_only(directory: &Path) {
    fs::set_permissions(directory, fs::Permissions::from_mode(0o555)).unwrap();
}
