//! Picks the size tier that fits the pane, centres it, and places each piece of the face.

use crate::{
    bezel::{Bezel, BorderRun, BorderSide},
    clock::SegmentClock,
    icon::{IconPicture, IconSize},
    pixels::Canvas,
    sky::ShownTimeline,
};

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
    pub weather_state_lines: Vec<Line<'static>>,
    /// The ghost icon without weather data; None for a weather code with no condition.
    pub mini_icon: Option<Canvas>,
    /// As the mini icon, with its stars.
    pub full_icon: Option<IconPicture>,
    /// For the bezel tiers, where the age label follows the temperature. Only for stale weather.
    pub temperature_age_high_and_low: Option<Line<'static>>,
    /// Only without a location.
    pub first_run_hint: Option<[Line<'static>; 2]>,
    /// None without weather data, and when no sunrise and sunset surround now.
    pub timeline: Option<ShownTimeline>,
    /// The weekday header over the current week.
    pub week_strip: [Line<'static>; 2],
    pub bezel: Bezel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeTier {
    Medium,
    Compact,
    Mini,
    Strip,
}

impl SizeTier {
    /// The first that fits wins.
    pub const IN_ORDER: [Self; 4] = [Self::Medium, Self::Compact, Self::Mini, Self::Strip];
}

#[derive(Debug)]
pub struct LayoutReport<'a> {
    /// None when the pane is smaller than the clock-only Strip.
    pub size_tier: Option<SizeTier>,
    pub items: Vec<PlacedItem<'a>>,
}

/// The area can be larger than the required size: a border run or the timeline stretches to
/// fill it.
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
    Icon(IconPicture),
    SegmentClock(&'a SegmentClock),
    Timeline(&'a ShownTimeline),
    BorderRun,
    BorderSide,
    Separator,
}

const SEPARATOR: &str = " · ";

impl Piece<'_> {
    pub fn required_size(self) -> Size {
        match self {
            Self::Text(line) => Size::new(line.width() as u16, 1),
            Self::Picture(canvas) => Size::new(canvas.width(), canvas.height().div_ceil(2)),
            Self::Icon(icon) => {
                let (width, height) = icon.size.cells();
                Size::new(width, height)
            }
            Self::SegmentClock(segment_clock) => segment_clock.size(),
            Self::Timeline(timeline) => timeline.required_size(),
            Self::BorderRun | Self::BorderSide => Size::new(1, 1),
            Self::Separator => Size::new(Span::raw(SEPARATOR).width() as u16, 1),
        }
    }

    pub fn render(self, area: Rect, buffer: &mut Buffer) {
        match self {
            Self::Text(line) => line.render(area, buffer),
            Self::Picture(canvas) => canvas.render(area, buffer),
            Self::Icon(icon) => icon.render(area, buffer),
            Self::SegmentClock(segment_clock) => segment_clock.render(area, buffer),
            Self::Timeline(timeline) => timeline.render(area, buffer),
            Self::BorderRun => BorderRun.render(area, buffer),
            Self::BorderSide => BorderSide.render(area, buffer),
            Self::Separator => Span::raw(SEPARATOR).render(area, buffer),
        }
    }
}

/// One way to show a size tier: its pieces in fixed areas inside its bounding box.
struct Arrangement<'a> {
    size: Size,
    pieces: Vec<(Piece<'a>, Rect)>,
}

impl<'a> Arrangement<'a> {
    /// One piece in an area of its required size.
    fn of(piece: Piece<'a>) -> Self {
        let size = piece.required_size();
        Self {
            size,
            pieces: vec![(piece, Rect::from((Position::ORIGIN, size)))],
        }
    }

    fn moved(self, by: Position) -> impl Iterator<Item = (Piece<'a>, Rect)> {
        (self.pieces.into_iter()).map(move |(piece, area)| {
            let position = Position::new(area.x + by.x, area.y + by.y);
            (piece, Rect::from((position, area.as_size())))
        })
    }
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
        SizeTier::Medium => medium_arrangements(face),
        SizeTier::Compact => compact_arrangements(face),
        SizeTier::Mini => mini_arrangement(face).into_iter().collect(),
        SizeTier::Strip => strip_arrangements(face),
    }
}

/// The clock and the full icon, the weather row, the timeline, and the week strip, in the bezel.
/// Without a location, the first-run hint takes the weather row and the timeline's first row.
fn medium_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    medium_weather_rows(face)
        .into_iter()
        .map(|[weather_row, timeline_rows]| {
            framed(
                &face.bezel,
                column(vec![
                    Block::Gap(BREATHING_ROWS),
                    Block::Left(clock_and_icon(face)),
                    weather_row,
                    Block::Gap(BREATHING_ROWS),
                    timeline_rows,
                    Block::Gap(BREATHING_ROWS),
                    week_strip(face),
                ]),
            )
        })
        .collect()
}

/// Under the top border, above the timeline, and above the week strip, so the clock, the
/// timeline marker, and the week strip do not touch what is above them.
const BREATHING_ROWS: u16 = 1;

fn week_strip(face: &Face) -> Block<'_> {
    Block::Centred(column(
        (face.week_strip.iter())
            .map(|line| centred(Piece::Text(line)))
            .collect(),
    ))
}

/// The icon's columns stay when there is no icon, so nothing moves when it returns.
fn clock_and_icon(face: &Face) -> Arrangement<'_> {
    let icon_x = face.segment_clock.size().width + CLOCK_GAP;
    let mut pieces = vec![(Piece::SegmentClock(&face.segment_clock), Position::ORIGIN)];
    pieces.extend((face.full_icon.map(Piece::Icon)).map(|icon| (icon, Position::new(icon_x, 0))));
    let arrangement = at_offsets(pieces);
    let (icon_width, _) = IconSize::Full.cells();
    Arrangement {
        size: Size::new(icon_x + icon_width, arrangement.size.height),
        ..arrangement
    }
}

/// Between the clock and the full icon.
const CLOCK_GAP: u16 = 1;

/// The weather row and the timeline rows under the clock and icon, each way widest first.
fn medium_weather_rows(face: &Face) -> Vec<[Block<'_>; 2]> {
    let timeline = || match &face.timeline {
        Some(timeline) => Block::FullWidth(Piece::Timeline(timeline)),
        None => Block::Gap(TIMELINE_ROWS),
    };
    if let Some(first_run_hint) = &face.first_run_hint {
        let hint = column(
            first_run_hint
                .iter()
                .map(|line| left(Piece::Text(line)))
                .collect(),
        );
        return vec![[
            Block::Left(hint),
            Block::Gap(WEATHER_ROWS + TIMELINE_ROWS - 2),
        ]];
    }
    let temperature = face
        .temperature_age_high_and_low
        .as_ref()
        .or(face.temperature_high_and_low.as_ref());
    let Some(temperature) = temperature else {
        return (face.weather_state_lines.iter())
            .map(|line| [left(Piece::Text(line)), timeline()])
            .collect();
    };
    let icon_x = face.segment_clock.size().width + CLOCK_GAP;
    let mut pieces = vec![(Piece::Text(temperature), Position::ORIGIN)];
    if let Some(sparkline) = &face.sparkline {
        let x = icon_x.max(temperature.width() as u16 + ICON_GAP);
        pieces.push((Piece::Text(sparkline), Position::new(x, 0)));
    }
    vec![[Block::Left(at_offsets(pieces)), timeline()]]
}

const WEATHER_ROWS: u16 = 1;
const TIMELINE_ROWS: u16 = 3;

/// Each piece in an area of its required size, at its offset.
fn at_offsets(pieces: Vec<(Piece<'_>, Position)>) -> Arrangement<'_> {
    let pieces: Vec<_> = (pieces.into_iter())
        .map(|(piece, offset)| (piece, Rect::from((offset, piece.required_size()))))
        .collect();
    let size = pieces.iter().fold(Size::ZERO, |size, (_, area)| {
        Size::new(size.width.max(area.right()), size.height.max(area.bottom()))
    });
    Arrangement { size, pieces }
}

/// Inside the bezel, with a column of space on each side. The bezel hugs the content, and widens
/// only when its labels need more room.
fn framed<'a>(bezel: &'a Bezel, content: Arrangement<'a>) -> Arrangement<'a> {
    let label_width = |left: &Line, right: &Line| (left.width() + right.width()) as u16 + 1;
    let width = (content.size.width + 4)
        .max(label_width(&bezel.top_left, &bezel.top_right))
        .max(label_width(&bezel.bottom_left, &bezel.bottom_right));
    let height = content.size.height + 2;
    let mut pieces = Vec::new();
    for (y, left, right) in [
        (0, &bezel.top_left, &bezel.top_right),
        (height - 1, &bezel.bottom_left, &bezel.bottom_right),
    ] {
        let left_width = left.width() as u16;
        let right_width = right.width() as u16;
        pieces.push((Piece::Text(left), Rect::new(0, y, left_width, 1)));
        let run_width = width - left_width - right_width;
        pieces.push((Piece::BorderRun, Rect::new(left_width, y, run_width, 1)));
        pieces.push((
            Piece::Text(right),
            Rect::new(width - right_width, y, right_width, 1),
        ));
    }
    for x in [0, width - 1] {
        pieces.push((Piece::BorderSide, Rect::new(x, 1, 1, height - 2)));
    }
    let content_x = (width - content.size.width) / 2;
    pieces.extend(content.moved(Position::new(content_x, 1)));
    Arrangement {
        size: Size::new(width, height),
        pieces,
    }
}

/// Without weather data, the weather state line takes the weather's text row, and the weather sits at the left
/// edge whatever its width, so nothing moves when the weather returns.
fn compact_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    compact_text_rows(face)
        .into_iter()
        .map(|text_rows| {
            column(vec![
                centred(Piece::Text(&face.short_date)),
                centred(Piece::SegmentClock(&face.segment_clock)),
                Block::Gap(1),
                Block::Left(icon_beside(face.mini_icon.as_ref(), text_rows)),
            ])
        })
        .collect()
}

/// Each way to fill the text rows beside the icon, widest first.
fn compact_text_rows(face: &Face) -> Vec<Vec<Piece<'_>>> {
    let Some(temperature_high_and_low) = &face.temperature_high_and_low else {
        return (face.weather_state_lines.iter())
            .map(|line| vec![Piece::Text(line)])
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
    for (y, piece) in text_rows.into_iter().enumerate() {
        pieces.push((piece, Position::new(text_x, y as u16)));
    }
    let arrangement = at_offsets(pieces);
    Arrangement {
        size: Size::new(
            arrangement.size.width.max(icon_width),
            arrangement.size.height.max(icon_height),
        ),
        ..arrangement
    }
}

const ICON_GAP: u16 = 2;

/// One row of a column.
enum Block<'a> {
    Centred(Arrangement<'a>),
    Left(Arrangement<'a>),
    /// Stretched across the column.
    FullWidth(Piece<'a>),
    /// Empty rows.
    Gap(u16),
}

fn centred(piece: Piece<'_>) -> Block<'_> {
    Block::Centred(Arrangement::of(piece))
}

fn left(piece: Piece<'_>) -> Block<'_> {
    Block::Left(Arrangement::of(piece))
}

/// Stacked top to bottom in the width of the widest block. A centred block puts an odd leftover
/// column after it.
fn column(blocks: Vec<Block<'_>>) -> Arrangement<'_> {
    let width = blocks
        .iter()
        .map(|block| match block {
            Block::Centred(arrangement) | Block::Left(arrangement) => arrangement.size.width,
            Block::FullWidth(piece) => piece.required_size().width,
            Block::Gap(_) => 0,
        })
        .max()
        .unwrap_or(0);
    let mut y = 0;
    let mut pieces = Vec::new();
    for block in blocks {
        let (arrangement, x) = match block {
            Block::Centred(arrangement) => {
                let x = (width - arrangement.size.width) / 2;
                (arrangement, x)
            }
            Block::Left(arrangement) => (arrangement, 0),
            Block::FullWidth(piece) => {
                let height = piece.required_size().height;
                pieces.push((piece, Rect::new(0, y, width, height)));
                y += height;
                continue;
            }
            Block::Gap(rows) => {
                y += rows;
                continue;
            }
        };
        let height = arrangement.size.height;
        pieces.extend(arrangement.moved(Position::new(x, y)));
        y += height;
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
        let position = Position::new(x, (height - size.height) / 2);
        placed.push((piece, Rect::from((position, size))));
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
        .map(|&(piece, area)| PlacedItem {
            piece,
            required_size: piece.required_size(),
            area: Rect::new(block.x + area.x, block.y + area.y, area.width, area.height),
        })
        .collect()
}
