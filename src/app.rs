use std::{path::PathBuf, time::Duration};

use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
};

use crate::{
    calendar,
    clock::Clock,
    config::{Config, Coordinates, Location, Units},
    layout::{self, HomeContent},
    log::Log,
    strings::Strings,
    weather::{self, WeatherSource, WeatherStation},
};

/// Time and time zone come only from `now`, never the system clock.
pub struct App {
    units: Units,
    strings: &'static Strings,
    clock: Clock,
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
        config: &Config,
        weather_source: Box<dyn WeatherSource>,
        cache_directory: PathBuf,
        state_directory: PathBuf,
    ) -> Self {
        Self {
            units: config.units,
            strings: Strings::of(config.language),
            clock: Clock::new(config),
            weather_station: WeatherStation::new(
                coordinates(config),
                weather_source,
                cache_directory,
            ),
            log: Log::start(&state_directory),
        }
    }

    pub fn tick(&mut self, now: &Zoned) {
        self.weather_station.tick(now, &mut self.log);
    }

    pub fn tick_interval(&self) -> Duration {
        self.clock.tick_interval()
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
        let content = self.home_content(now);
        for item in layout::lay_out(area, &content).items {
            item.piece.render(item.area, buffer);
        }
    }

    pub fn home_content(&self, now: &Zoned) -> HomeContent {
        HomeContent {
            text_clock: self.clock.text_line(now),
            short_date: calendar::short_date(self.strings, now),
            weather_line: weather::text_line(
                self.weather_station.weather_state(now),
                self.weather_station.shows_fetch_indicator(now),
                self.units,
            ),
        }
    }
}

fn coordinates(config: &Config) -> Option<Coordinates> {
    match config.location {
        Some(Location::Coordinates(coordinates)) => Some(coordinates),
        _ => None,
    }
}
