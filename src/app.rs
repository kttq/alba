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
    config::{ColonStyle, Config, HourFormat},
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

    /// How often the event loop wakes: a quarter second while something moves, else a second.
    pub fn tick_interval(&self) -> Duration {
        if self.config.clock.colon == ColonStyle::Steady {
            Duration::from_secs(1)
        } else {
            Duration::from_millis(250)
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
        let minutes = format!("{:02}", now.minute());
        match self.config.hour_format {
            HourFormat::TwentyFour => Line::from(vec![
                Span::raw(format!("{:02}", now.hour())),
                self.colon(now),
                Span::raw(minutes),
            ]),
            HourFormat::Twelve => {
                let hour = match now.hour() % 12 {
                    0 => 12,
                    hour => hour,
                };
                let mark = if now.hour() < 12 {
                    self.strings.am
                } else {
                    self.strings.pm
                };
                Line::from(vec![
                    Span::raw(hour.to_string()),
                    self.colon(now),
                    Span::raw(minutes),
                    mark.dim(),
                ])
            }
        }
    }

    /// Lit for the first half of each wall-clock second; the `colon` style shows in the second.
    fn colon(&self, now: &Zoned) -> Span<'static> {
        let lit = now.subsec_nanosecond() < 500_000_000;
        match self.config.clock.colon {
            _ if lit => Span::raw(":"),
            ColonStyle::Steady => Span::raw(":"),
            ColonStyle::Ghost => ":".dim(),
            ColonStyle::Blank => Span::raw(" "),
        }
    }
}
