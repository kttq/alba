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
    weather::{FetchError, WeatherSource},
};
use jiff::Zoned;
use ratatui::{Terminal, backend::TestBackend};

pub const BUENOS_AIRES: &str = "coordinates = [-34.6037, -58.3816]";

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

/// Every Rust file under `src/`, as a path and its text.
pub fn source_files() -> Vec<(PathBuf, String)> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files: Vec<_> = rust_files(&source)
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .collect();
    assert!(
        !files.is_empty(),
        "no Rust files under {}",
        source.display()
    );
    files
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}
