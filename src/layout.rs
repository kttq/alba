//! Picks the size tier that fits the pane, centres it, and places each piece of the face.

use crate::{clock::SegmentClock, icon::IconSize, pixels::Canvas};

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    text::{Line, Span},
    widgets::Widget,
};

/// Everything alba shows at one instant, built once for every size tier.
pub struct Face {
    pub text_clock: Line<'static>,
    pub segment_clock: SegmentClock,
    pub short_date: Line<'static>,
    pub weekday_and_day: Line<'static>,
    /// The weather pieces are None when no weather data is shown.
    pub weather_line: Option<Line<'static>>,
    pub temperature: Option<Line<'static>>,
    pub temperature_high_and_low: Option<Line<'static>>,
    /// Only for stale weather.
    pub aged_temperature_high_and_low: Option<Line<'static>>,
    pub sparkline: Option<Line<'static>>,
    /// What the weather area says without weather data, each way widest first.
    pub weather_status: Vec<Line<'static>>,
    /// The ghost icon without weather data; None for a weather code with no condition.
    pub mini_icon: Option<Canvas>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeTier {
    Compact,
    Mini,
    Strip,
}

impl SizeTier {
    /// The first that fits wins.
    pub const IN_ORDER: [Self; 3] = [Self::Compact, Self::Mini, Self::Strip];
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
    Picture(&'a Canvas),
    SegmentClock(&'a SegmentClock),
    Separator,
}

const SEPARATOR: &str = " · ";

impl Piece<'_> {
    pub fn required_size(self) -> Size {
        match self {
            Self::Text(line) => Size::new(line.width() as u16, 1),
            Self::Picture(canvas) => Size::new(canvas.width(), canvas.height().div_ceil(2)),
            Self::SegmentClock(segment_clock) => segment_clock.size(),
            Self::Separator => Size::new(Span::raw(SEPARATOR).width() as u16, 1),
        }
    }

    pub fn render(self, area: Rect, buffer: &mut Buffer) {
        match self {
            Self::Text(line) => line.render(area, buffer),
            Self::Picture(canvas) => canvas.render(area, buffer),
            Self::SegmentClock(segment_clock) => segment_clock.render(area, buffer),
            Self::Separator => Span::raw(SEPARATOR).render(area, buffer),
        }
    }
}

/// One way to show a size tier: its pieces at fixed offsets inside its bounding box.
struct Arrangement<'a> {
    size: Size,
    pieces: Vec<(Piece<'a>, Position)>,
}

pub fn lay_out<'a>(pane: Rect, face: &'a Face) -> LayoutReport<'a> {
    let fitting = SizeTier::IN_ORDER
        .into_iter()
        .flat_map(|size_tier| {
            arrangements(size_tier, face)
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

/// None when the size tier cannot show this face.
pub fn minimum_size(size_tier: SizeTier, face: &Face) -> Option<Size> {
    arrangements(size_tier, face)
        .into_iter()
        .map(|arrangement| arrangement.size)
        .min_by_key(|size| (size.width, size.height))
}

/// Widest first.
fn arrangements(size_tier: SizeTier, face: &Face) -> Vec<Arrangement<'_>> {
    match size_tier {
        SizeTier::Compact => compact_arrangements(face),
        SizeTier::Mini => mini_arrangement(face).into_iter().collect(),
        SizeTier::Strip => strip_arrangements(face),
    }
}

/// Without weather data, the status takes the weather's text row, so nothing moves when the
/// weather returns.
fn compact_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    compact_text_rows(face)
        .into_iter()
        .map(|text_rows| {
            column(vec![
                centred(Piece::Text(&face.short_date)),
                centred(Piece::SegmentClock(&face.segment_clock)),
                Block::Gap(1),
                Block::Centred(icon_beside(face.mini_icon.as_ref(), text_rows)),
            ])
        })
        .collect()
}

/// Each way to fill the text rows beside the icon, widest first.
fn compact_text_rows(face: &Face) -> Vec<Vec<Piece<'_>>> {
    let Some(temperature_high_and_low) = &face.temperature_high_and_low else {
        return (face.weather_status.iter())
            .map(|status| vec![Piece::Text(status)])
            .collect();
    };
    let sparkline = face.sparkline.as_ref().map(Piece::Text);
    (face.aged_temperature_high_and_low.iter())
        .chain([temperature_high_and_low])
        .map(|first_row| {
            std::iter::once(Piece::Text(first_row))
                .chain(sparkline)
                .collect()
        })
        .collect()
}

/// The text keeps its place when there is no icon, so it never jumps.
fn icon_beside<'a>(icon: Option<&'a Canvas>, text_rows: Vec<Piece<'a>>) -> Arrangement<'a> {
    let (icon_width, icon_height) = IconSize::Mini.cells();
    let text_x = icon_width + ICON_GAP;
    let mut pieces: Vec<_> = icon
        .map(|canvas| (Piece::Picture(canvas), Position::ORIGIN))
        .into_iter()
        .collect();
    let mut width = icon_width;
    let text_height = text_rows.len() as u16;
    for (y, piece) in text_rows.into_iter().enumerate() {
        width = width.max(text_x + piece.required_size().width);
        pieces.push((piece, Position::new(text_x, y as u16)));
    }
    Arrangement {
        size: Size::new(width, icon_height.max(text_height)),
        pieces,
    }
}

const ICON_GAP: u16 = 2;

/// One row of a column.
enum Block<'a> {
    Centred(Arrangement<'a>),
    /// Empty rows.
    Gap(u16),
}

fn centred(piece: Piece<'_>) -> Block<'_> {
    Block::Centred(Arrangement {
        size: piece.required_size(),
        pieces: vec![(piece, Position::ORIGIN)],
    })
}

/// Stacked top to bottom, each block centred in the widest; an odd leftover column goes after it.
fn column(blocks: Vec<Block<'_>>) -> Arrangement<'_> {
    let width = blocks
        .iter()
        .map(|block| match block {
            Block::Centred(arrangement) => arrangement.size.width,
            Block::Gap(_) => 0,
        })
        .max()
        .unwrap_or(0);
    let mut y = 0;
    let mut pieces = Vec::new();
    for block in blocks {
        match block {
            Block::Centred(arrangement) => {
                let x = (width - arrangement.size.width) / 2;
                pieces.extend(
                    arrangement
                        .pieces
                        .into_iter()
                        .map(|(piece, offset)| (piece, Position::new(x + offset.x, y + offset.y))),
                );
                y += arrangement.size.height;
            }
            Block::Gap(rows) => y += rows,
        }
    }
    Arrangement {
        size: Size::new(width, y),
        pieces,
    }
}

/// Without weather, Mini would show less than Strip in one more row.
fn mini_arrangement(face: &Face) -> Option<Arrangement<'_>> {
    let pieces = vec![
        Piece::Text(&face.text_clock),
        Piece::Picture(face.mini_icon.as_ref()?),
        Piece::Text(face.temperature.as_ref()?),
    ];
    Some(row(pieces, 2))
}

fn strip_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    let clock = Piece::Text(&face.text_clock);
    let date = Piece::Text(&face.weekday_and_day);
    let separator = Piece::Separator;
    let lines = match &face.weather_line {
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

/// The bounding box is centred as one block. An odd leftover cell goes before it, as
/// `Rect::centered` puts it; arithmetic, because the constraint solver is too slow for the size
/// sweep.
fn placed<'a>(arrangement: &Arrangement<'a>, pane: Rect) -> Vec<PlacedItem<'a>> {
    let block = Position::new(
        pane.x + (pane.width - arrangement.size.width).div_ceil(2),
        pane.y + (pane.height - arrangement.size.height).div_ceil(2),
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
