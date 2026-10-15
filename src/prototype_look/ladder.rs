//! PROTOTYPE (issue #55), throwaway. The v1 look with the week in place of the month and a free
//! row over the clock. A child of `layout`, so it builds from the same private blocks. C, the
//! week as a column, where there is room; U, the week under the readings, where C does not fit.
//! The week never sits beside the clock. The sparkline shows only with its label, in the large
//! rungs; without it, it reads as noise. Then Compact without the sparkline, Mini, and Strip.

use std::borrow::Cow;

use jiff::{
    ToSpan,
    civil::{Date, Weekday},
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
};

use super::{
    Arrangement, BREATHING_ROWS, Block, CALENDAR_GAP, Cell, LayoutReport, Piece, SizeTier,
    arrangements, centred, clock_and_icon, column, composed, fits, framed, icon_beside,
    icon_column, icon_slot, large_weather_area_arrangements, left, placed, row, text, timeline,
    widest,
};
use crate::{
    clock::DigitSize, config::WeekStart, face::Face, icon::IconSize, strings::Strings,
    weather::WeatherArea,
};

/// The week strip: the initials over the days. The week column: one day a row, today marked.
pub struct Week {
    pub strip: Vec<Line<'static>>,
    pub column: Vec<Line<'static>>,
}

impl Week {
    /// Today in the strong accent, bold and underlined; past days dim; the weekend's initials in
    /// the second accent.
    pub fn at(week_start: WeekStart, strings: &Strings, today: Date) -> Self {
        let first = match week_start {
            WeekStart::Monday => Weekday::Monday,
            WeekStart::Sunday => Weekday::Sunday,
        };
        let start = today
            .checked_sub(i64::from(today.weekday().since(first)).days())
            .unwrap_or(today);
        let days: Vec<Date> = (0..7)
            .filter_map(|offset| start.checked_add(offset.days()).ok())
            .collect();
        let initial = |day: Date| {
            let initial = strings.weekday_initials[day.weekday().to_sunday_zero_offset() as usize];
            let weekend = matches!(day.weekday(), Weekday::Saturday | Weekday::Sunday);
            Span::styled(
                initial,
                if weekend {
                    Style::new().red()
                } else {
                    Style::new().dark_gray()
                },
            )
        };
        let number = |day: Date| {
            let style = if day == today {
                Style::new()
                    .yellow()
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else if day < today {
                Style::new().dark_gray()
            } else {
                Style::new()
            };
            Span::styled(format!("{:>2}", day.day()), style)
        };
        let spaced = |spans: Vec<Span<'static>>| {
            let mut line = Vec::new();
            for (index, span) in spans.into_iter().enumerate() {
                if index > 0 {
                    line.push(Span::raw(" "));
                }
                line.push(span);
            }
            Line::from(line)
        };
        let column = (days.iter())
            .map(|&day| {
                let mark = if day == today {
                    " ◂".yellow()
                } else {
                    Span::raw("  ")
                };
                Line::from(vec![initial(day), Span::raw(" "), number(day), mark])
            })
            .collect();
        Self {
            strip: vec![
                spaced(days.iter().map(|&day| initial(day)).collect()),
                spaced(days.iter().map(|&day| number(day)).collect()),
            ],
            column,
        }
    }
}

/// The ladder's rungs, largest first; the first that fits wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    LargeColumn,
    LargeUnder,
    ShortUnder,
    MediumColumn,
    MediumUnder,
}

impl Rung {
    pub const ALL: [Self; 5] = [
        Self::LargeColumn,
        Self::LargeUnder,
        Self::ShortUnder,
        Self::MediumColumn,
        Self::MediumUnder,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::LargeColumn => "large C",
            Self::LargeUnder => "large U",
            Self::ShortUnder => "short U",
            Self::MediumColumn => "medium C",
            Self::MediumUnder => "medium U",
        }
    }

    fn arrangements<'a>(self, face: &'a Face, week: &'a Week) -> Vec<Arrangement<'a>> {
        match self {
            Self::LargeColumn | Self::LargeUnder => large(face, week, self == Self::LargeColumn),
            Self::ShortUnder => short_under(face, week),
            Self::MediumColumn => medium_column(face, week),
            Self::MediumUnder => medium_under(face, week),
        }
    }

    fn size_tier(self) -> SizeTier {
        match self {
            Self::LargeColumn | Self::LargeUnder => SizeTier::Large,
            Self::ShortUnder => SizeTier::Wide,
            Self::MediumColumn | Self::MediumUnder => SizeTier::Medium,
        }
    }
}

/// The rung that won, None below the ladder.
pub fn lay_out<'a>(pane: Rect, face: &'a Face, week: &'a Week) -> (LayoutReport<'a>, Option<Rung>) {
    let ours = Rung::ALL.map(|rung| (rung.size_tier(), Some(rung), rung.arrangements(face, week)));
    let compact = [(SizeTier::Compact, None, compact(face))];
    let theirs = [SizeTier::Mini, SizeTier::Strip]
        .map(|size_tier| (size_tier, None, arrangements(size_tier, face)));
    let fitting = (ours.into_iter().chain(compact).chain(theirs))
        .flat_map(|(size_tier, rung, arrangements)| {
            arrangements
                .into_iter()
                .map(move |arrangement| (size_tier, rung, arrangement))
        })
        .find(|(_, _, arrangement)| fits(arrangement.size, pane));
    let Some((size_tier, rung, arrangement)) = fitting else {
        return (
            LayoutReport {
                size_tier: None,
                items: Vec::new(),
            },
            None,
        );
    };
    (
        LayoutReport {
            size_tier: Some(size_tier),
            items: placed(arrangement, pane),
        },
        rung,
    )
}

fn lines<'a>(lines: &'a [Line<'static>]) -> Arrangement<'a> {
    column(
        lines
            .iter()
            .map(|line| left(Piece::Text(Cow::Borrowed(line))))
            .collect(),
    )
}

/// The temperature and the condition. Without weather, the first-run hint or the weather
/// state line.
fn short_readings(face: &Face) -> Vec<Arrangement<'_>> {
    match &face.weather_area {
        WeatherArea::NoWeather {
            first_run_hint: Some(first_run_hint),
            ..
        } => {
            vec![column(
                first_run_hint.iter().map(|line| left(text(line))).collect(),
            )]
        }
        WeatherArea::NoWeather { lines, .. } => lines
            .iter()
            .map(|line| Arrangement::of(text(line)))
            .collect(),
        WeatherArea::Readings(readings) => vec![column(vec![
            left(composed(widest(readings, SizeTier::Large))),
            left(composed(readings.condition_line())),
        ])],
    }
}

/// The readings centred over the week strip, so they share its width. `sparkline` adds the
/// sparkline under its label.
fn readings_over_week<'a>(face: &'a Face, week: &'a Week, sparkline: bool) -> Vec<Arrangement<'a>> {
    let week = || Block::Left(lines(&week.strip));
    match &face.weather_area {
        WeatherArea::NoWeather {
            first_run_hint: Some(first_run_hint),
            ..
        } => {
            let mut blocks: Vec<_> = first_run_hint
                .iter()
                .map(|line| centred(text(line)))
                .collect();
            blocks.extend([Block::Gap(1), week()]);
            vec![column(blocks)]
        }
        WeatherArea::NoWeather { lines, .. } => (lines.iter())
            .map(|line| column(vec![centred(text(line)), Block::Gap(1), week()]))
            .collect(),
        WeatherArea::Readings(readings) => {
            let mut blocks = vec![
                centred(composed(widest(readings, SizeTier::Large))),
                centred(composed(readings.condition_line())),
            ];
            if sparkline {
                blocks.extend([
                    Block::Gap(1),
                    centred(text(readings.sparkline_label())),
                    centred(text(readings.sparkline())),
                ]);
            }
            blocks.extend([Block::Gap(1), week()]);
            vec![column(blocks)]
        }
    }
}

fn large<'a>(face: &'a Face, week: &'a Week, week_column: bool) -> Vec<Arrangement<'a>> {
    let middles: Vec<Arrangement<'a>> = if week_column {
        (large_weather_area_arrangements(face).into_iter())
            .map(|weather_area| {
                let icon = Cell::Next(icon_slot(face.icon, IconSize::Large));
                row(
                    vec![
                        icon,
                        Cell::Next(weather_area),
                        Cell::Next(lines(&week.column)),
                    ],
                    CALENDAR_GAP,
                )
            })
            .collect()
    } else {
        (readings_over_week(face, week, true).into_iter())
            .map(|readings| {
                let icon = Cell::Next(icon_slot(face.icon, IconSize::Large));
                // Two columns, not three, so the face fits a 50-column square pane.
                row(vec![icon, Cell::Next(readings)], 2)
            })
            .collect()
    };
    middles
        .into_iter()
        .map(|middle| {
            let blocks = vec![
                centred(Piece::SegmentClock(&face.clock, DigitSize::Large)),
                Block::Gap(BREATHING_ROWS),
                Block::Centred(middle),
            ];
            framed(&face.bezel, SizeTier::Large, breathing(blocks, face))
        })
        .collect()
}

/// The large clock; the full icon beside the readings over the week.
fn short_under<'a>(face: &'a Face, week: &'a Week) -> Vec<Arrangement<'a>> {
    readings_over_week(face, week, false)
        .into_iter()
        .map(|right| {
            let middle = row(
                vec![
                    Cell::Next(icon_slot(face.icon, IconSize::Full)),
                    Cell::Next(right),
                ],
                CALENDAR_GAP,
            );
            let blocks = vec![
                centred(Piece::SegmentClock(&face.clock, DigitSize::Large)),
                Block::Gap(BREATHING_ROWS),
                Block::Centred(middle),
            ];
            framed(&face.bezel, SizeTier::Wide, breathing(blocks, face))
        })
        .collect()
}

/// The small clock over the readings, the full icon, and the week column.
fn medium_column<'a>(face: &'a Face, week: &'a Week) -> Vec<Arrangement<'a>> {
    short_readings(face)
        .into_iter()
        .map(|readings| {
            let clock = Arrangement::of(Piece::SegmentClock(&face.clock, DigitSize::Small));
            let left_side = column(vec![
                Block::Left(clock),
                Block::Gap(1),
                Block::Left(readings),
            ]);
            let cells = vec![
                Cell::Next(left_side),
                Cell::Next(icon_slot(face.icon, IconSize::Full)),
                Cell::Next(lines(&week.column)),
            ];
            framed(
                &face.bezel,
                SizeTier::Medium,
                breathing(vec![Block::Left(row(cells, 2))], face),
            )
        })
        .collect()
}

/// v1's Medium with the new week, and the condition where the sparkline was.
fn medium_under<'a>(face: &'a Face, week: &'a Week) -> Vec<Arrangement<'a>> {
    let icon_column = icon_column(face);
    let weather_rows: Vec<Arrangement<'a>> = match &face.weather_area {
        WeatherArea::NoWeather {
            first_run_hint: Some(first_run_hint),
            ..
        } => {
            vec![column(
                first_run_hint.iter().map(|line| left(text(line))).collect(),
            )]
        }
        WeatherArea::NoWeather { lines, .. } => lines
            .iter()
            .map(|line| Arrangement::of(text(line)))
            .collect(),
        WeatherArea::Readings(readings) => vec![row(
            vec![
                Cell::Next(Arrangement::of(composed(widest(
                    readings,
                    SizeTier::Medium,
                )))),
                Cell::AtColumn(
                    icon_column,
                    Arrangement::of(composed(readings.condition_line())),
                ),
            ],
            2,
        )],
    };
    weather_rows
        .into_iter()
        .map(|weather_row| {
            let blocks = vec![
                Block::Left(clock_and_icon(face, icon_column)),
                Block::Left(weather_row),
            ];
            let mut content = breathing(blocks, face);
            content = column(vec![
                Block::Left(content),
                Block::Gap(BREATHING_ROWS),
                Block::Centred(lines(&week.strip)),
            ]);
            framed(&face.bezel, SizeTier::Medium, content)
        })
        .collect()
}

/// v1's Compact with the condition in place of the sparkline.
fn compact(face: &Face) -> Vec<Arrangement<'_>> {
    let text_rows: Vec<Vec<Piece<'_>>> = match &face.weather_area {
        WeatherArea::NoWeather { lines, .. } => lines.iter().map(|line| vec![text(line)]).collect(),
        WeatherArea::Readings(readings) => {
            (readings.temperature_lines(SizeTier::Compact).into_iter())
                .map(|first_row| vec![composed(first_row), composed(readings.condition_line())])
                .collect()
        }
    };
    text_rows
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

/// A free row over the blocks, so the clock does not touch the top border, and the timeline
/// under them.
fn breathing<'a>(blocks: Vec<Block<'a>>, face: &'a Face) -> Arrangement<'a> {
    let mut all = vec![Block::Gap(BREATHING_ROWS)];
    all.extend(blocks);
    all.extend([Block::Gap(BREATHING_ROWS), timeline(face)]);
    column(all)
}
