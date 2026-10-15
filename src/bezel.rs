//! The bezel: a frame with rounded corners that hugs the face, with labels in its borders.

use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::{calendar, strings::Strings};

/// The bezel's labels with the corners beside them. Border runs of `─` fill the rest.
#[derive(Debug)]
pub struct Bezel {
    /// The corner and the short date.
    pub top_left: Line<'static>,
    /// The ISO week and the corner.
    pub top_right: Line<'static>,
    /// The corner and the greeting.
    pub bottom_left: Line<'static>,
    /// The Open-Meteo credit, when weather data is shown, and the corner.
    pub bottom_right: Line<'static>,
}

impl Bezel {
    pub fn new(strings: &Strings, now: &Zoned, shows_credit: bool) -> Self {
        let mut top_left = Line::from(frame("╭─ "));
        top_left.extend(calendar::short_date(strings, now).spans);
        top_left.push_span(frame(" "));
        let mut top_right = Line::from(frame(" "));
        top_right.extend(calendar::iso_week(strings, now).spans);
        top_right.push_span(frame(" ─╮"));
        let bottom_left = Line::from(vec![
            frame("╰» "),
            Span::styled(strings.greeting(now.hour()), Style::new().green()),
            frame(" "),
        ]);
        let bottom_right = if shows_credit {
            Line::from(vec![
                frame(" "),
                Span::styled("open-meteo", Style::new().dark_gray()),
                frame(" ─╯"),
            ])
        } else {
            Line::from(frame("─╯"))
        };
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }
}

fn frame(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::new().dark_gray())
}

/// A run of `─` across its area.
#[derive(Debug, Clone, Copy)]
pub struct BorderRun;

impl Widget for BorderRun {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        for x in area.left()..area.right() {
            buffer[(x, area.y)]
                .set_symbol("─")
                .set_style(Style::new().dark_gray());
        }
    }
}

/// A column of `│` down its area.
#[derive(Debug, Clone, Copy)]
pub struct BorderSide;

impl Widget for BorderSide {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        for y in area.top()..area.bottom() {
            buffer[(area.x, y)]
                .set_symbol("│")
                .set_style(Style::new().dark_gray());
        }
    }
}
