use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
};

use crate::{
    config::Config,
    face::{Face, FaceSettings},
    layout,
    log::Log,
    weather::{WeatherSource, WeatherStation},
};

/// Time and time zone come only from `now`, never the system clock.
pub struct App {
    face_settings: FaceSettings,
    weather_station: WeatherStation,
    log: Log,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Control {
    Continue,
    Quit,
}

impl App {
    /// Reads the cache and empties the log.
    pub fn new(
        config: Config,
        weather_source: Box<dyn WeatherSource>,
        shown_config_file: String,
        cache_directory: PathBuf,
        state_directory: &Path,
    ) -> Self {
        Self {
            face_settings: FaceSettings::new(&config, shown_config_file),
            weather_station: WeatherStation::new(
                config.location,
                config.language,
                weather_source,
                cache_directory,
            ),
            log: Log::start(state_directory),
        }
    }

    pub fn tick(&mut self, now: &Zoned) {
        self.weather_station.tick(now, &mut self.log);
    }

    pub fn tick_interval(&self) -> Duration {
        self.face_settings.tick_interval()
    }

    /// Raw mode swallows the interrupt signal, so `ctrl-c` arrives here as a key.
    pub fn key(&mut self, event: KeyEvent) -> Control {
        match (event.code, event.modifiers) {
            (KeyCode::Char('q') | KeyCode::Esc, KeyModifiers::NONE) => Control::Quit,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Control::Quit,
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                self.weather_station.request_refresh();
                Control::Continue
            }
            _ => Control::Continue,
        }
    }

    pub fn render(&self, area: Rect, buffer: &mut Buffer, now: &Zoned) {
        let face = self.face(now);
        for item in layout::lay_out(area, &face).items {
            item.piece.render(item.area, buffer);
        }
    }

    pub fn face(&self, now: &Zoned) -> Face {
        Face::at(&self.face_settings, self.weather_station.shown(now), now)
    }
}
