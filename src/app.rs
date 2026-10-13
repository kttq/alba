use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Rect},
    text::Span,
    widgets::Widget,
};

/// The app core. It never reads the system clock or time zone: time arrives only as `now`.
#[derive(Default)]
pub struct App;

/// What the event loop does after a key.
#[derive(Debug, PartialEq, Eq)]
pub enum Control {
    Continue,
    Quit,
}

impl App {
    pub fn new() -> Self {
        Self
    }

    /// The event loop calls this on every wake, before it draws.
    pub fn tick(&mut self, _now: &Zoned) {}

    /// Raw mode swallows the interrupt signal, so `ctrl-c` arrives here as a key.
    pub fn key(&mut self, event: KeyEvent) -> Control {
        match (event.code, event.modifiers) {
            (KeyCode::Char('q') | KeyCode::Esc, KeyModifiers::NONE) => Control::Quit,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Control::Quit,
            _ => Control::Continue,
        }
    }

    pub fn render(&self, area: Rect, buffer: &mut Buffer, now: &Zoned) {
        let text_clock = Span::raw(now.strftime("%H:%M").to_string());
        let width = text_clock.width() as u16;
        // A pane smaller than the clock renders nothing rather than clipping it.
        if area.width < width || area.height == 0 {
            return;
        }
        text_clock.render(
            area.centered(Constraint::Length(width), Constraint::Length(1)),
            buffer,
        );
    }
}
