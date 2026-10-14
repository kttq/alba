//! Picks the size tier that fits the pane, centres it, and places each piece of the Home view.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Position, Rect, Size},
    text::{Line, Span},
    widgets::Widget,
};

/// The Home view at one instant, built once for every size tier.
pub struct HomeContent {
    pub text_clock: Line<'static>,
    pub short_date: Line<'static>,
    /// None when no weather data is shown.
    pub weather_line: Option<Line<'static>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeTier {
    Strip,
}

impl SizeTier {
    /// The first that fits wins.
    pub const IN_ORDER: [Self; 1] = [Self::Strip];
}

#[derive(Debug)]
pub struct LayoutReport<'a> {
    /// None when the pane is smaller than the clock-only Strip.
    pub size_tier: Option<SizeTier>,
    pub items: Vec<PlacedItem<'a>>,
}

#[derive(Debug, Clone, Copy)]
pub struct PlacedItem<'a> {
    pub piece: Piece<'a>,
    pub required_size: Size,
    pub area: Rect,
}

#[derive(Debug, Clone, Copy)]
pub enum Piece<'a> {
    Text(&'a Line<'static>),
    /// The ` · ` between the parts of the Strip line.
    Separator,
}

const SEPARATOR: &str = " · ";

impl Piece<'_> {
    pub fn required_size(self) -> Size {
        match self {
            Self::Text(line) => Size::new(line.width() as u16, 1),
            Self::Separator => Size::new(Span::raw(SEPARATOR).width() as u16, 1),
        }
    }

    pub fn render(self, area: Rect, buffer: &mut Buffer) {
        match self {
            Self::Text(line) => line.render(area, buffer),
            Self::Separator => Span::raw(SEPARATOR).render(area, buffer),
        }
    }
}

/// One way to show a size tier: its pieces at fixed offsets inside its bounding box.
struct Arrangement<'a> {
    size: Size,
    pieces: Vec<(Piece<'a>, Position)>,
}

pub fn lay_out<'a>(pane: Rect, content: &'a HomeContent) -> LayoutReport<'a> {
    let fitting = SizeTier::IN_ORDER
        .into_iter()
        .flat_map(|size_tier| {
            arrangements(size_tier, content)
                .into_iter()
                .map(move |arrangement| (size_tier, arrangement))
        })
        .find(|(_, arrangement)| fits(arrangement.size, pane));
    match fitting {
        Some((size_tier, arrangement)) => LayoutReport {
            size_tier: Some(size_tier),
            items: placed(&arrangement, pane),
        },
        None => LayoutReport {
            size_tier: None,
            items: Vec::new(),
        },
    }
}

/// The smallest pane the size tier fits, or None when it cannot show this content.
pub fn minimum_size(size_tier: SizeTier, content: &HomeContent) -> Option<Size> {
    arrangements(size_tier, content)
        .into_iter()
        .map(|arrangement| arrangement.size)
        .min_by_key(|size| (size.width, size.height))
}

/// Widest first.
fn arrangements(size_tier: SizeTier, content: &HomeContent) -> Vec<Arrangement<'_>> {
    match size_tier {
        SizeTier::Strip => strip_arrangements(content),
    }
}

/// The full line, then without the date, then the clock alone.
fn strip_arrangements(content: &HomeContent) -> Vec<Arrangement<'_>> {
    let clock = Piece::Text(&content.text_clock);
    let date = Piece::Text(&content.short_date);
    let separator = Piece::Separator;
    let lines = match &content.weather_line {
        Some(weather_line) => {
            let weather = Piece::Text(weather_line);
            vec![
                vec![clock, separator, date, separator, weather],
                vec![clock, separator, weather],
                vec![clock],
            ]
        }
        None => vec![vec![clock, separator, date], vec![clock]],
    };
    lines.into_iter().map(|pieces| row(pieces, 0)).collect()
}

/// Side by side with `gap` columns between them, each centred vertically.
fn row(pieces: Vec<Piece<'_>>, gap: u16) -> Arrangement<'_> {
    let height = pieces
        .iter()
        .map(|piece| piece.required_size().height)
        .max()
        .unwrap_or(0);
    let mut x = 0;
    let mut placed = Vec::new();
    for piece in pieces {
        let size = piece.required_size();
        placed.push((piece, Position::new(x, (height - size.height) / 2)));
        x += size.width + gap;
    }
    Arrangement {
        size: Size::new(x.saturating_sub(gap), height),
        pieces: placed,
    }
}

fn fits(size: Size, pane: Rect) -> bool {
    size.width <= pane.width && size.height <= pane.height
}

/// The bounding box is centred as one block.
fn placed<'a>(arrangement: &Arrangement<'a>, pane: Rect) -> Vec<PlacedItem<'a>> {
    let block = pane.centered(
        Constraint::Length(arrangement.size.width),
        Constraint::Length(arrangement.size.height),
    );
    arrangement
        .pieces
        .iter()
        .map(|&(piece, offset)| {
            let required_size = piece.required_size();
            PlacedItem {
                piece,
                required_size,
                area: Rect::new(
                    block.x + offset.x,
                    block.y + offset.y,
                    required_size.width,
                    required_size.height,
                ),
            }
        })
        .collect()
}
