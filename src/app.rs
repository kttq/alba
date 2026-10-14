use std::{path::PathBuf, time::Duration};

use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Rect},
    text::{Line, Span},
    widgets::Widget,
};

use crate::{
    calendar,
    clock::Clock,
    config::{Config, Coordinates, Location, Units},
    log::Log,
    strings::Strings,
    weather::{self, WeatherFeed, WeatherSource},
};

/// Time and time zone come only from `now`, never the system clock.
pub struct App {
    units: Units,
    strings: &'static Strings,
    clock: Clock,
    weather_feed: WeatherFeed,
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
            weather_feed: WeatherFeed::new(coordinates(config), weather_source, cache_directory),
            log: Log::start(&state_directory),
        }
    }

    pub fn tick(&mut self, now: &Zoned) {
        self.weather_feed.tick(now, &mut self.log);
    }

    pub fn tick_interval(&self) -> Duration {
        self.clock.tick_interval()
    }

    /// Raw mode swallows the interrupt signal, so `ctrl-c` arrives here as a key.
    pub fn key(&mut self, event: KeyEvent) -> Control {
        match (event.code, event.modifiers) {
            (KeyCode::Char('q') | KeyCode::Esc, KeyModifiers::NONE) => Control::Quit,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Control::Quit,
            _ => Control::Continue,
        }
    }

    pub fn render(&self, area: Rect, buffer: &mut Buffer, now: &Zoned) {
        if area.height == 0 {
            return;
        }
        let Some(text_line) = self
            .text_lines_widest_first(now)
            .into_iter()
            .find(|line| line.width() <= area.width as usize)
        else {
            return;
        };
        let width = text_line.width() as u16;
        text_line.render(
            area.centered(Constraint::Length(width), Constraint::Length(1)),
            buffer,
        );
    }

    fn text_lines_widest_first(&self, now: &Zoned) -> Vec<Line<'static>> {
        let clock = self.clock.text_line(now);
        let date = calendar::short_date(self.strings, now);
        let weather = self
            .weather_feed
            .weather()
            .map(|weather| weather::text_line(weather, self.units));
        match weather {
            Some(weather) => vec![
                joined(vec![clock.clone(), date, weather.clone()]),
                joined(vec![clock.clone(), weather]),
                clock,
            ],
            None => vec![joined(vec![clock.clone(), date]), clock],
        }
    }
}

fn joined(parts: Vec<Line<'static>>) -> Line<'static> {
    let parts: Vec<_> = parts.into_iter().map(|part| part.spans).collect();
    Line::from(parts.join(&Span::raw(" · ")))
}

fn coordinates(config: &Config) -> Option<Coordinates> {
    match config.location {
        Some(Location::Coordinates(coordinates)) => Some(coordinates),
        _ => None,
    }
}
