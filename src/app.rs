use std::{path::PathBuf, time::Duration};

use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::Widget,
};

use crate::{
    config::{Config, Coordinates, HourFormat, Location},
    log::Log,
    strings::Strings,
    weather::{self, Condition, FetchError, Forecast, Weather, WeatherSource, cache},
};

/// Time and time zone come only from `now`, never the system clock.
pub struct App {
    config: Config,
    strings: &'static Strings,
    weather_source: Box<dyn WeatherSource>,
    cache_directory: PathBuf,
    log: Log,
    weather: Option<Weather>,
    fetch_requested: bool,
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
        let weather =
            coordinates(config).and_then(|coordinates| cache::read(&cache_directory, coordinates));
        Self {
            config: config.clone(),
            strings: Strings::of(config.language),
            weather_source,
            cache_directory,
            log: Log::start(&state_directory),
            weather,
            fetch_requested: false,
        }
    }

    pub fn tick(&mut self, now: &Zoned) {
        if let Some(result) = self.weather_source.finished() {
            self.receive(result, now);
        }
        if let Some(coordinates) = coordinates(&self.config)
            && !self.fetch_requested
        {
            self.weather_source.request(coordinates);
            self.fetch_requested = true;
        }
    }

    fn receive(&mut self, result: Result<String, FetchError>, now: &Zoned) {
        let Some(coordinates) = coordinates(&self.config) else {
            return;
        };
        let forecast = match result.and_then(|body| Forecast::parse(&body)) {
            Ok(forecast) => forecast,
            Err(error) => return self.log.warn(now, "fetch failed", &error),
        };
        let weather = Weather {
            fetched_at: now.timestamp(),
            forecast,
        };
        if let Err(error) = cache::write(&self.cache_directory, coordinates, &weather) {
            self.log.warn(now, "cache write failed", &error);
        }
        self.weather = Some(weather);
    }

    /// The blinking colon needs a quarter-second tick.
    pub fn tick_interval(&self) -> Duration {
        if self.config.motion {
            Duration::from_millis(250)
        } else {
            Duration::from_secs(1)
        }
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
            .text_lines(now)
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

    fn text_lines(&self, now: &Zoned) -> Vec<Line<'static>> {
        let clock = self.text_clock(now);
        let date = Line::from(self.short_date(now).light_blue());
        match self.text_weather() {
            Some(weather) => vec![
                joined(vec![clock.clone(), date, weather.clone()]),
                joined(vec![clock.clone(), weather]),
                clock,
            ],
            None => vec![joined(vec![clock.clone(), date]), clock],
        }
    }

    fn text_weather(&self) -> Option<Line<'static>> {
        let current = &self.weather.as_ref()?.forecast.current;
        let degrees = weather::whole_degrees(current.temperature_2m, self.config.units);
        let text = match Condition::from_wmo(current.weather_code) {
            Some(condition) => format!("{degrees}° {}", condition_mark(condition)),
            None => format!("{degrees}°"),
        };
        Some(Line::raw(text))
    }

    fn text_clock(&self, now: &Zoned) -> Line<'static> {
        let mut spans = vec![
            Span::raw(self.hour(now)),
            self.colon(now),
            Span::raw(format!("{:02}", now.minute())),
        ];
        if self.config.hour_format == HourFormat::Twelve {
            spans.push(self.am_pm(now).dim());
        }
        Line::from(spans)
    }

    fn short_date(&self, now: &Zoned) -> String {
        let weekday = self.strings.short_weekdays[now.weekday().to_sunday_zero_offset() as usize];
        format!("{weekday} {}", now.day())
    }

    fn hour(&self, now: &Zoned) -> String {
        match (self.config.hour_format, now.hour() % 12) {
            (HourFormat::TwentyFour, _) => format!("{:02}", now.hour()),
            (HourFormat::Twelve, 0) => "12".to_string(),
            (HourFormat::Twelve, hour) => hour.to_string(),
        }
    }

    fn am_pm(&self, now: &Zoned) -> &'static str {
        if now.hour() < 12 {
            self.strings.am
        } else {
            self.strings.pm
        }
    }

    fn colon(&self, now: &Zoned) -> Span<'static> {
        let first_half = now.subsec_nanosecond() < 500_000_000;
        if first_half || !self.config.motion {
            Span::raw(":")
        } else {
            ":".dim()
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

/// Each mark must take two terminal cells.
fn condition_mark(condition: Condition) -> &'static str {
    match condition {
        Condition::Clear => "✦✦",
        Condition::PartlyCloudy => "✦~",
        Condition::Cloudy => "~~",
        Condition::Fog => "≈≈",
        Condition::Drizzle => "′′",
        Condition::Rain => "‚‚",
        Condition::HeavyRain => "┆┆",
        Condition::Snow => "**",
        Condition::Thunderstorm => "!!",
    }
}
