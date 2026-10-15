//! Picks the size tier that fits the pane, centres it, and places each piece of the face.

use crate::{
    bezel::{Bezel, BorderRun, BorderSide},
    clock::{DigitSize, ShownClock},
    face::Face,
    icon::{IconSize, ShownIcon},
    sky::ShownTimeline,
    weather::{Readings, WeatherArea},
};

use std::borrow::Cow;

/// PROTOTYPE (issue #55): the size ladder of the revamped look.
#[path = "prototype_look/ladder.rs"]
pub mod ladder;

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    text::{Line, Span},
    widgets::Widget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeTier {
    Large,
    Wide,
    Tall,
    Medium,
    Compact,
    Mini,
    Strip,
}

impl SizeTier {
    /// The first that fits wins.
    pub const IN_ORDER: [Self; 7] = [
        Self::Large,
        Self::Wide,
        Self::Tall,
        Self::Medium,
        Self::Compact,
        Self::Mini,
        Self::Strip,
    ];
}

#[derive(Debug)]
pub struct LayoutReport<'a> {
    /// None when the pane is smaller than the clock-only Strip.
    pub size_tier: Option<SizeTier>,
    pub items: Vec<PlacedItem<'a>>,
}

/// The area can be larger than the required size: a border run or the timeline stretches to
/// fill it.
#[derive(Debug, Clone)]
pub struct PlacedItem<'a> {
    pub piece: Piece<'a>,
    pub required_size: Size,
    pub area: Rect,
}

#[derive(Debug, Clone)]
pub enum Piece<'a> {
    Text(Cow<'a, Line<'static>>),
    Icon(ShownIcon, IconSize),
    SegmentClock(&'a ShownClock, DigitSize),
    Timeline(&'a ShownTimeline),
    BorderRun(BorderRun<'a>),
    BorderSide,
    Separator,
}

const SEPARATOR: &str = " · ";

fn text<'a>(line: &'a Line<'static>) -> Piece<'a> {
    Piece::Text(Cow::Borrowed(line))
}

/// A line built for one size tier, not held by the face.
fn composed(line: Line<'static>) -> Piece<'static> {
    Piece::Text(Cow::Owned(line))
}

impl Piece<'_> {
    pub fn required_size(&self) -> Size {
        match self {
            Self::Text(line) => Size::new(line.width() as u16, 1),
            Self::Icon(_, icon_size) => {
                let (width, height) = icon_size.cells();
                Size::new(width, height)
            }
            Self::SegmentClock(clock, digit_size) => clock.segment_size(*digit_size),
            Self::Timeline(timeline) => timeline.required_size(),
            Self::BorderRun(_) | Self::BorderSide => Size::new(1, 1),
            Self::Separator => Size::new(Span::raw(SEPARATOR).width() as u16, 1),
        }
    }

    pub fn render(self, area: Rect, buffer: &mut Buffer) {
        match self {
            Self::Text(line) => line.as_ref().render(area, buffer),
            Self::Icon(icon, icon_size) => icon.render(icon_size, area, buffer),
            Self::SegmentClock(clock, digit_size) => {
                (&clock.segment_clock(digit_size)).render(area, buffer);
            }
            Self::Timeline(timeline) => timeline.render(area, buffer),
            Self::BorderRun(border_run) => border_run.render(area, buffer),
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
            items: placed(arrangement, pane),
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
        SizeTier::Large => large_arrangements(face),
        SizeTier::Wide => wide_arrangements(face),
        SizeTier::Tall => tall_arrangements(face),
        SizeTier::Medium => medium_arrangements(face),
        SizeTier::Compact => compact_arrangements(face),
        SizeTier::Mini => mini_arrangement(face).into_iter().collect(),
        SizeTier::Strip => strip_arrangements(face),
    }
}

fn large_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    large_weather_area_arrangements(face)
        .into_iter()
        .map(|weather_area| {
            let middle = row(
                vec![
                    Cell::Next(icon_slot(face.icon, IconSize::Large)),
                    Cell::Next(weather_area),
                    Cell::Next(calendar_lines(face.calendar.month())),
                ],
                CALENDAR_GAP,
            );
            let content = column(vec![
                centred(Piece::SegmentClock(&face.clock, DigitSize::Large)),
                Block::Gap(BREATHING_ROWS),
                Block::Left(middle),
                Block::Gap(BREATHING_ROWS),
                timeline(face),
            ]);
            framed(&face.bezel, SizeTier::Large, content)
        })
        .collect()
}

/// Each way widest first: the readings over the labelled sparkline, the first-run hint, or the
/// weather state line.
fn large_weather_area_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    let readings = match &face.weather_area {
        WeatherArea::NoWeather {
            first_run_hint: Some(first_run_hint),
            ..
        } => {
            return vec![column(
                first_run_hint.iter().map(|line| left(text(line))).collect(),
            )];
        }
        WeatherArea::NoWeather { lines, .. } => {
            return (lines.iter())
                .map(|line| Arrangement::of(text(line)))
                .collect();
        }
        WeatherArea::Readings(readings) => readings,
    };
    vec![column(vec![
        left(composed(widest(readings, SizeTier::Large))),
        left(composed(readings.condition_line())),
        Block::Gap(1),
        left(text(readings.sparkline_label())),
        left(text(readings.sparkline())),
    ])]
}

fn wide_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    let icon_column = icon_column(face);
    weather_and_timeline(face, SizeTier::Wide, icon_column)
        .into_iter()
        .map(|rows| {
            let mut blocks = vec![Block::Left(clock_and_icon(face, icon_column))];
            blocks.extend(rows);
            let content = row(
                vec![
                    Cell::Next(column(blocks)),
                    Cell::Next(calendar_lines(face.calendar.month())),
                ],
                CALENDAR_GAP,
            );
            framed(&face.bezel, SizeTier::Wide, content)
        })
        .collect()
}

/// Between the timeline and the month calendar.
const CALENDAR_GAP: u16 = 3;

fn tall_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    let icon_column = icon_column(face);
    weather_and_timeline(face, SizeTier::Tall, icon_column)
        .into_iter()
        .map(|rows| {
            let mut blocks = vec![Block::Left(clock_and_icon(face, icon_column))];
            blocks.extend(rows);
            let month = calendar_lines(face.calendar.month());
            blocks.extend([Block::Gap(BREATHING_ROWS), Block::Centred(month)]);
            framed(&face.bezel, SizeTier::Tall, column(blocks))
        })
        .collect()
}

fn calendar_lines<'a>(lines: &'a [Line<'static>]) -> Arrangement<'a> {
    column(lines.iter().map(|line| centred(text(line))).collect())
}

/// The clock and the full icon, the weather row, the timeline, and the week strip, in the bezel.
fn medium_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    let icon_column = icon_column(face);
    weather_and_timeline(face, SizeTier::Medium, icon_column)
        .into_iter()
        .map(|rows| {
            let mut blocks = vec![
                Block::Gap(BREATHING_ROWS),
                Block::Left(clock_and_icon(face, icon_column)),
            ];
            blocks.extend(rows);
            blocks.extend([Block::Gap(BREATHING_ROWS), week_strip(face)]);
            framed(&face.bezel, SizeTier::Medium, column(blocks))
        })
        .collect()
}

/// The column the full icon starts at, which the sparkline shares.
fn icon_column(face: &Face) -> u16 {
    face.clock.segment_size(DigitSize::Small).width + CLOCK_GAP
}

/// Under the top border, above the timeline, and above the week strip, so the clock, the
/// timeline marker, and the week strip do not touch what is above them.
const BREATHING_ROWS: u16 = 1;

fn week_strip(face: &Face) -> Block<'_> {
    Block::Centred(calendar_lines(face.calendar.week_strip()))
}

fn clock_and_icon(face: &Face, icon_column: u16) -> Arrangement<'_> {
    row(
        vec![
            next(Piece::SegmentClock(&face.clock, DigitSize::Small)),
            Cell::AtColumn(icon_column, icon_slot(face.icon, IconSize::Full)),
        ],
        0,
    )
}

/// The icon's columns stay when there is no icon, so nothing moves when it returns.
fn icon_slot(icon: Option<ShownIcon>, icon_size: IconSize) -> Arrangement<'static> {
    match icon {
        Some(icon) => Arrangement::of(Piece::Icon(icon, icon_size)),
        None => {
            let (width, height) = icon_size.cells();
            Arrangement {
                size: Size::new(width, height),
                pieces: Vec::new(),
            }
        }
    }
}

/// Between the clock and the full icon.
const CLOCK_GAP: u16 = 1;

/// The rows under the clock and icon: the weather row, the condition row in Tall, and the
/// timeline, each way widest first. Without a location, the first-run hint takes the first two
/// rows.
fn weather_and_timeline(face: &Face, size_tier: SizeTier, icon_column: u16) -> Vec<Vec<Block<'_>>> {
    let has_condition_row = size_tier == SizeTier::Tall;
    let condition_rows = u16::from(has_condition_row);
    let readings = match &face.weather_area {
        WeatherArea::NoWeather {
            first_run_hint: Some(first_run_hint),
            ..
        } => {
            let hint = column(
                (first_run_hint.iter())
                    .map(|line| left(text(line)))
                    .collect(),
            );
            let rows = WEATHER_ROWS + condition_rows + BREATHING_ROWS + TIMELINE_ROWS;
            let hint_rows = first_run_hint.len() as u16;
            return vec![vec![Block::Left(hint), Block::Gap(rows - hint_rows)]];
        }
        WeatherArea::NoWeather { lines, .. } => {
            return (lines.iter())
                .map(|line| {
                    vec![
                        left(text(line)),
                        Block::Gap(condition_rows + BREATHING_ROWS),
                        timeline(face),
                    ]
                })
                .collect();
        }
        WeatherArea::Readings(readings) => readings,
    };
    let weather_row = row(
        vec![
            next(composed(widest(readings, size_tier))),
            Cell::AtColumn(icon_column, Arrangement::of(text(readings.sparkline()))),
        ],
        ICON_GAP,
    );
    let mut blocks = vec![Block::Left(weather_row)];
    if has_condition_row {
        blocks.push(left(composed(readings.condition_line())));
    }
    blocks.extend([Block::Gap(BREATHING_ROWS), timeline(face)]);
    vec![blocks]
}

/// The size tiers other than Compact have one way to show the temperature line.
fn widest(readings: &Readings, size_tier: SizeTier) -> Line<'static> {
    let mut lines = readings.temperature_lines(size_tier);
    lines.swap_remove(0)
}

/// Empty rows without a timeline, so the layout does not jump.
fn timeline(face: &Face) -> Block<'_> {
    match &face.timeline {
        Some(timeline) => Block::FullWidth(Piece::Timeline(timeline)),
        None => Block::Gap(TIMELINE_ROWS),
    }
}

const WEATHER_ROWS: u16 = 1;
const TIMELINE_ROWS: u16 = 3;

/// Inside the bezel, with a column of space on each side. The bezel hugs the content, and widens
/// only when its labels need more room.
fn framed<'a>(bezel: &'a Bezel, size_tier: SizeTier, content: Arrangement<'a>) -> Arrangement<'a> {
    let label_width = |left: &Line, right: &Line| (left.width() + right.width()) as u16 + 1;
    let width = (content.size.width + 4)
        .max(label_width(&bezel.top_left, &bezel.top_right))
        .max(label_width(&bezel.bottom_left, &bezel.bottom_right));
    let height = content.size.height + 2;
    let mut pieces = Vec::new();
    for (y, left, right, place_name) in [
        (
            0,
            &bezel.top_left,
            &bezel.top_right,
            bezel.place_name(size_tier),
        ),
        (height - 1, &bezel.bottom_left, &bezel.bottom_right, None),
    ] {
        let left_width = left.width() as u16;
        let right_width = right.width() as u16;
        pieces.push((text(left), Rect::new(0, y, left_width, 1)));
        let run_width = width - left_width - right_width;
        pieces.push((
            Piece::BorderRun(BorderRun { place_name }),
            Rect::new(left_width, y, run_width, 1),
        ));
        pieces.push((
            text(right),
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

/// Without weather data, the weather state line takes the weather's text row, and the weather
/// sits at the left edge whatever its width, so nothing moves when the weather returns.
fn compact_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    compact_text_rows(face)
        .into_iter()
        .map(|text_rows| {
            column(vec![
                centred(text(face.calendar.short_date())),
                centred(Piece::SegmentClock(&face.clock, DigitSize::Small)),
                Block::Gap(1),
                Block::Left(icon_beside(face.icon, text_rows)),
            ])
        })
        .collect()
}

/// Each way to fill the text rows beside the icon, widest first. The age label drops first.
fn compact_text_rows(face: &Face) -> Vec<Vec<Piece<'_>>> {
    let readings = match &face.weather_area {
        WeatherArea::NoWeather { lines, .. } => {
            return (lines.iter()).map(|line| vec![text(line)]).collect();
        }
        WeatherArea::Readings(readings) => readings,
    };
    (readings.temperature_lines(SizeTier::Compact).into_iter())
        .map(|first_row| vec![composed(first_row), text(readings.sparkline())])
        .collect()
}

fn icon_beside(icon: Option<ShownIcon>, text_rows: Vec<Piece<'_>>) -> Arrangement<'_> {
    let text_rows = column(text_rows.into_iter().map(left).collect());
    row(
        vec![
            Cell::Next(icon_slot(icon, IconSize::Mini)),
            Cell::Next(text_rows),
        ],
        ICON_GAP,
    )
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
    let WeatherArea::Readings(readings) = &face.weather_area else {
        return None;
    };
    let pieces = vec![
        text(face.clock.text_clock()),
        Piece::Icon(face.icon?, IconSize::Mini),
        composed(widest(readings, SizeTier::Mini)),
    ];
    Some(row(pieces.into_iter().map(next).collect(), 2))
}

fn strip_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    let clock = text(face.clock.text_clock());
    let date = text(face.calendar.weekday_and_day());
    let separator = Piece::Separator;
    let lines = match &face.weather_area {
        WeatherArea::Readings(readings) => {
            let weather = composed(widest(readings, SizeTier::Strip));
            vec![
                vec![
                    clock.clone(),
                    separator.clone(),
                    date,
                    separator.clone(),
                    weather.clone(),
                ],
                vec![clock.clone(), separator, weather],
                vec![clock],
            ]
        }
        WeatherArea::NoWeather { .. } => vec![vec![clock.clone(), separator, date], vec![clock]],
    };
    (lines.into_iter())
        .map(|pieces| row(pieces.into_iter().map(next).collect(), 0))
        .collect()
}

/// One part of a row.
enum Cell<'a> {
    /// After the cell before it and the gap.
    Next(Arrangement<'a>),
    /// At a column that a size tier shares between rows, or later when the cells before it
    /// reach past it.
    AtColumn(u16, Arrangement<'a>),
}

fn next(piece: Piece<'_>) -> Cell<'_> {
    Cell::Next(Arrangement::of(piece))
}

/// Side by side with `gap` columns between them, each centred vertically.
fn row(cells: Vec<Cell<'_>>, gap: u16) -> Arrangement<'_> {
    let cells: Vec<_> = (cells.into_iter())
        .map(|cell| match cell {
            Cell::Next(arrangement) => (0, arrangement),
            Cell::AtColumn(column, arrangement) => (column, arrangement),
        })
        .collect();
    let height = (cells.iter())
        .map(|(_, arrangement)| arrangement.size.height)
        .max()
        .unwrap_or(0);
    let mut end = None;
    let mut pieces = Vec::new();
    for (column, arrangement) in cells {
        let x = column.max(end.map_or(0, |end| end + gap));
        let y = (height - arrangement.size.height) / 2;
        end = Some(x + arrangement.size.width);
        pieces.extend(arrangement.moved(Position::new(x, y)));
    }
    Arrangement {
        size: Size::new(end.unwrap_or(0), height),
        pieces,
    }
}

fn fits(size: Size, pane: Rect) -> bool {
    size.width <= pane.width && size.height <= pane.height
}

/// The bounding box is centred as one block. An odd leftover cell goes before it, as
/// `Rect::centered` puts it; arithmetic, because the constraint solver is too slow for the size
/// sweep.
fn placed(arrangement: Arrangement<'_>, pane: Rect) -> Vec<PlacedItem<'_>> {
    let block = Position::new(
        pane.x + (pane.width - arrangement.size.width).div_ceil(2),
        pane.y + (pane.height - arrangement.size.height).div_ceil(2),
    );
    (arrangement.pieces.into_iter())
        .map(|(piece, area)| PlacedItem {
            required_size: piece.required_size(),
            piece,
            area: Rect::new(block.x + area.x, block.y + area.y, area.width, area.height),
        })
        .collect()
}
