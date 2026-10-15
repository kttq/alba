//! The bezel: a frame with rounded corners that hugs the face, with labels in its borders.

use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::{calendar::ShownCalendar, layout::SizeTier, strings::Strings};

/// The bezel's labels with the corners beside them. Border runs of `─` fill the rest.
#[derive(Debug)]
pub struct Bezel {
    /// The corner and the short date.
    pub top_left: Line<'static>,
    /// The ISO week and the corner.
    pub top_right: Line<'static>,
    /// None for configured coordinates.
    place_name: Option<String>,
    /// The corner and the greeting.
    pub bottom_left: Line<'static>,
    /// The Open-Meteo credit, when weather data is shown, and the corner.
    pub bottom_right: Line<'static>,
}

/// Under this many columns for the name itself, the place name drops.
const PLACE_NAME_MINIMUM_WIDTH: u16 = 6;
/// A `─` and a space on each side keep the place name apart from the labels.
const PLACE_NAME_MARGIN: u16 = 2;

impl Bezel {
    pub fn new(
        strings: &Strings,
        calendar: &ShownCalendar,
        now: &Zoned,
        place_name: Option<&str>,
        shows_credit: bool,
    ) -> Self {
        let mut top_left = Line::from(frame("╭─ "));
        top_left.extend(calendar.short_date().spans.iter().cloned());
        top_left.push_span(frame(" "));
        let mut top_right = Line::from(frame(" "));
        top_right.extend(calendar.iso_week().spans.iter().cloned());
        top_right.push_span(frame(" ─╮"));
        let bottom_left = Line::from(vec![
            frame("╰» "),
            Span::styled(greeting(strings, now.hour()), Style::new().green()),
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
            place_name: place_name.map(str::to_string),
            bottom_left,
            bottom_right,
        }
    }
}

impl Bezel {
    /// The place name for the top border, in the size tiers wide enough for it.
    pub fn place_name(&self, size_tier: SizeTier) -> Option<&str> {
        match size_tier {
            SizeTier::Large | SizeTier::Wide => self.place_name.as_deref(),
            _ => None,
        }
    }
}

/// The greeting for a local hour, 0 to 23. Before the first start hour, the last greeting still
/// holds from the evening.
fn greeting(strings: &Strings, hour: i8) -> &'static str {
    let (_, last) = strings.greetings[strings.greetings.len() - 1];
    (strings.greetings.iter())
        .take_while(|(start, _)| *start <= hour)
        .last()
        .map_or(last, |(_, greeting)| greeting)
}

fn frame(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::new().dark_gray())
}

/// A run of `─` across its area, with the place name centred in it when the name has room.
#[derive(Debug, Clone, Copy)]
pub struct BorderRun<'a> {
    pub place_name: Option<&'a str>,
}

impl Widget for BorderRun<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let style = Style::new().dark_gray();
        for x in area.left()..area.right() {
            buffer[(x, area.y)].set_symbol("─").set_style(style);
        }
        let Some(place_name) = self.place_name else {
            return;
        };
        let room = area.width.saturating_sub(2 * PLACE_NAME_MARGIN);
        if room < PLACE_NAME_MINIMUM_WIDTH {
            return;
        }
        let name = Span::styled(format!(" {} ", truncated(place_name, room)), style);
        let width = name.width() as u16;
        let x = area.x + (area.width - width) / 2;
        name.render(Rect::new(x, area.y, width, 1), buffer);
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

/// At most `width` columns, ending in `…` when shortened.
fn truncated(text: &str, width: u16) -> String {
    let width = usize::from(width);
    if Span::raw(text).width() <= width {
        return text.to_string();
    }
    let mut kept = String::new();
    for character in text.chars() {
        let mut next = kept.clone();
        next.push(character);
        if Span::raw(next.as_str()).width() + 1 > width {
            break;
        }
        kept = next;
    }
    kept + "…"
}
