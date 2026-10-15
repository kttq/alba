//! Picks the size tier that fits the pane, centres it, and places each piece of the face.

use crate::{
    bezel::{Bezel, BorderRun, BorderSide},
    calendar::ShownCalendar,
    clock::{DigitSize, ShownClock},
    icon::{IconSize, ShownIcon},
    sky::ShownTimeline,
    weather::{Part, WeatherArea},
};

use std::borrow::Cow;

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    text::{Line, Span},
    widgets::Widget,
};

/// Everything alba shows at one instant. Each size tier asks a part for the size it places.
pub struct Face {
    pub clock: ShownClock,
    pub calendar: ShownCalendar,
    pub weather_area: WeatherArea,
    /// The ghost icon without weather data; None for a weather code with no condition.
    pub icon: Option<ShownIcon>,
    /// None without weather data, and when no sunrise and sunset surround now.
    pub timeline: Option<ShownTimeline>,
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
    BorderRun,
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
            Self::BorderRun | Self::BorderSide => Size::new(1, 1),
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
        (face.calendar.week_strip().iter())
            .map(|line| centred(text(line)))
            .collect(),
    ))
}

/// The icon's columns stay when there is no icon, so nothing moves when it returns.
fn clock_and_icon(face: &Face) -> Arrangement<'_> {
    let icon_x = face.clock.segment_size(DigitSize::Small).width + CLOCK_GAP;
    let mut pieces = vec![(
        Piece::SegmentClock(&face.clock, DigitSize::Small),
        Position::ORIGIN,
    )];
    pieces.extend(
        (face.icon.map(|icon| Piece::Icon(icon, IconSize::Full)))
            .map(|icon| (icon, Position::new(icon_x, 0))),
    );
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
            return vec![[
                Block::Left(hint),
                Block::Gap(WEATHER_ROWS + TIMELINE_ROWS - 2),
            ]];
        }
        WeatherArea::NoWeather { lines, .. } => {
            return (lines.iter())
                .map(|line| [left(text(line)), timeline()])
                .collect();
        }
        WeatherArea::Readings(readings) => readings,
    };
    let temperature = readings.line(&[Part::Temperature, Part::Age, Part::HighAndLow]);
    let icon_x = face.clock.segment_size(DigitSize::Small).width + CLOCK_GAP;
    let sparkline_x = icon_x.max(temperature.width() as u16 + ICON_GAP);
    let pieces = vec![
        (composed(temperature), Position::ORIGIN),
        (text(readings.sparkline()), Position::new(sparkline_x, 0)),
    ];
    vec![[Block::Left(at_offsets(pieces)), timeline()]]
}

const WEATHER_ROWS: u16 = 1;
const TIMELINE_ROWS: u16 = 3;

/// Each piece in an area of its required size, at its offset.
fn at_offsets(pieces: Vec<(Piece<'_>, Position)>) -> Arrangement<'_> {
    let pieces: Vec<_> = (pieces.into_iter())
        .map(|(piece, offset)| {
            let area = Rect::from((offset, piece.required_size()));
            (piece, area)
        })
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
        pieces.push((text(left), Rect::new(0, y, left_width, 1)));
        let run_width = width - left_width - right_width;
        pieces.push((Piece::BorderRun, Rect::new(left_width, y, run_width, 1)));
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

/// Without weather data, the weather state line takes the weather's text row, and the weather sits at the left
/// edge whatever its width, so nothing moves when the weather returns.
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
    let with_age = [Part::Temperature, Part::HighAndLow, Part::Age];
    let without_age = [Part::Temperature, Part::HighAndLow];
    let first_rows = if readings.is_stale() {
        vec![readings.line(&with_age), readings.line(&without_age)]
    } else {
        vec![readings.line(&without_age)]
    };
    (first_rows.into_iter())
        .map(|first_row| vec![composed(first_row), text(readings.sparkline())])
        .collect()
}

/// The text keeps its place when there is no icon, so it never jumps.
fn icon_beside(icon: Option<ShownIcon>, text_rows: Vec<Piece<'_>>) -> Arrangement<'_> {
    let (icon_width, icon_height) = IconSize::Mini.cells();
    let text_x = icon_width + ICON_GAP;
    let mut pieces: Vec<_> = icon
        .map(|icon| (Piece::Icon(icon, IconSize::Mini), Position::ORIGIN))
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
    let WeatherArea::Readings(readings) = &face.weather_area else {
        return None;
    };
    let pieces = vec![
        text(face.clock.text_line()),
        Piece::Icon(face.icon?, IconSize::Mini),
        composed(readings.line(&[Part::Temperature])),
    ];
    Some(row(pieces, 2))
}

fn strip_arrangements(face: &Face) -> Vec<Arrangement<'_>> {
    let clock = text(face.clock.text_line());
    let date = text(face.calendar.weekday_and_day());
    let separator = Piece::Separator;
    let lines = match &face.weather_area {
        WeatherArea::Readings(readings) => {
            let weather = composed(readings.line(&[Part::Temperature, Part::ConditionMark]));
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
