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
    bezel::Bezel,
    calendar::{self, Calendar},
    clock::{Clock, DigitSize},
    config::{Config, Units},
    icon::{Icon, IconPicture, IconSize},
    layout::{self, Face},
    log::Log,
    motion::{self, Motion},
    sky::ShownTimeline,
    strings::Strings,
    weather::{self, WeatherSource, WeatherStation},
};

/// Time and time zone come only from `now`, never the system clock.
pub struct App {
    units: Units,
    motion: bool,
    strings: &'static Strings,
    /// For the first-run hint, with `HOME` shown as `~`.
    shown_config_file: String,
    clock: Clock,
    calendar: Calendar,
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
            units: config.units,
            motion: config.motion,
            strings: Strings::of(config.language),
            shown_config_file,
            clock: Clock::new(&config),
            calendar: Calendar::new(&config),
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
        motion::tick_interval(self.motion)
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
        let weather_state = self.weather_station.weather_state(now);
        let shows_fetch_indicator = self.weather_station.shows_fetch_indicator(now);
        let motion = Motion::at(now.timestamp(), self.motion);
        let icon = Icon::shown(weather_state, now);
        Face {
            text_clock: self.clock.text_line(now, motion),
            segment_clock: self.clock.segment_clock(now, motion, DigitSize::Small),
            short_date: calendar::short_date(self.strings, now),
            weekday_and_day: calendar::weekday_and_day(self.strings, now),
            weather_line: weather::text_line(weather_state, shows_fetch_indicator, self.units),
            temperature: weather::temperature_line(
                weather_state,
                shows_fetch_indicator,
                self.units,
            ),
            temperature_high_and_low: weather::temperature_high_and_low_line(
                weather_state,
                shows_fetch_indicator,
                self.units,
                now.timestamp(),
            ),
            aged_temperature_high_and_low: weather::aged_temperature_high_and_low_line(
                weather_state,
                shows_fetch_indicator,
                self.units,
                now.timestamp(),
                self.strings,
            ),
            sparkline: weather::sparkline(weather_state, now.timestamp()),
            weather_state_lines: weather::weather_state_lines(weather_state, self.strings),
            mini_icon: icon.map(|icon| icon.canvas(IconSize::Mini, motion)),
            full_icon: icon.map(|icon| IconPicture {
                icon,
                size: IconSize::Full,
                motion,
            }),
            temperature_age_high_and_low: weather::temperature_age_high_and_low_line(
                weather_state,
                shows_fetch_indicator,
                self.units,
                now.timestamp(),
                self.strings,
            ),
            first_run_hint: weather::first_run_hint(
                weather_state,
                self.strings,
                &self.shown_config_file,
            ),
            timeline: weather_state.weather().and_then(|weather| {
                ShownTimeline::of(&weather.forecast, now, &self.clock, self.strings)
            }),
            week_strip: self.calendar.week_strip(now),
            bezel: Bezel::new(
                self.strings,
                now,
                self.weather_station.place_name(),
                weather_state.weather().is_some(),
            ),
        }
    }
}
