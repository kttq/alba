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
    strings::Strings,
    weather::{self, Condition, FetchError, Forecast, Weather, WeatherSource, cache},
};

/// The app core. It never reads the system clock or time zone: time arrives only as `now`.
pub struct App {
    config: Config,
    strings: &'static Strings,
    weather_source: Box<dyn WeatherSource>,
    cache_directory: PathBuf,
    weather: Option<Weather>,
    fetch_requested: bool,
}

/// What the event loop does after a key.
#[derive(Debug, PartialEq, Eq)]
pub enum Control {
    Continue,
    Quit,
}

impl App {
    /// Shows the cached weather for the configured coordinates at once.
    pub fn new(
        config: &Config,
        weather_source: Box<dyn WeatherSource>,
        cache_directory: PathBuf,
    ) -> Self {
        let weather =
            coordinates(config).and_then(|coordinates| cache::read(&cache_directory, coordinates));
        Self {
            config: config.clone(),
            strings: Strings::of(config.language),
            weather_source,
            cache_directory,
            weather,
            fetch_requested: false,
        }
    }

    /// The event loop calls this on every wake, before it draws. Fetches once, on the first
    /// tick.
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

    /// Replaces the weather only with a valid forecast. A failed cache write keeps it in memory.
    fn receive(&mut self, result: Result<String, FetchError>, now: &Zoned) {
        let Some(coordinates) = coordinates(&self.config) else {
            return;
        };
        let Ok(forecast) = result.and_then(|body| Forecast::parse(&body)) else {
            return;
        };
        let weather = Weather {
            fetched_at: now.timestamp(),
            forecast,
        };
        let _ = cache::write(&self.cache_directory, coordinates, &weather);
        self.weather = Some(weather);
    }

    /// How often the event loop wakes. The blinking colon needs a quarter second.
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

    /// The Strip line, widest first: the full line, then without the date, then the clock alone.
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

    /// The temperature, then the condition mark when the code has a condition.
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

    /// The 12-hour clock reads midnight and noon as 12.
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

    /// Lit for the first half of each wall-clock second, faint in the second half.
    fn colon(&self, now: &Zoned) -> Span<'static> {
        let first_half = now.subsec_nanosecond() < 500_000_000;
        if first_half || !self.config.motion {
            Span::raw(":")
        } else {
            ":".dim()
        }
    }
}

/// Joins the parts of the text line with ` · `.
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

/// Two cells that stand for a condition on the text line.
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
