use std::time::Duration;

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
    config::{Config, HourFormat},
    strings::Strings,
};

/// The app core. It never reads the system clock or time zone: time arrives only as `now`.
pub struct App {
    config: Config,
    strings: &'static Strings,
}

/// What the event loop does after a key.
#[derive(Debug, PartialEq, Eq)]
pub enum Control {
    Continue,
    Quit,
}

impl App {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            strings: Strings::of(config.language),
        }
    }

    /// The event loop calls this on every wake, before it draws.
    pub fn tick(&mut self, _now: &Zoned) {}

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
        let text_clock = self.text_clock(now);
        let width = text_clock.width() as u16;
        // `centered` shrinks the area to fit the pane, which would clip the clock.
        if area.width < width || area.height == 0 {
            return;
        }
        text_clock.render(
            area.centered(Constraint::Length(width), Constraint::Length(1)),
            buffer,
        );
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

    /// `06` on the 24-hour clock; `6` on the 12-hour clock, where midnight and noon are `12`.
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
