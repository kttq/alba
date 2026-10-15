use jiff::{
    ToSpan, Zoned,
    civil::{Date, Weekday},
};
use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
};

use crate::{
    config::{Config, WeekStart},
    strings::Strings,
};

pub struct Calendar {
    week_start: WeekStart,
    strings: &'static Strings,
}

/// The calendar at one instant.
#[derive(Debug, Clone)]
pub struct ShownCalendar {
    week_strip: [Line<'static>; 2],
    short_date: Line<'static>,
    weekday_and_day: Line<'static>,
}

/// For Strip, which has no room for the month.
pub fn weekday_and_day(strings: &Strings, now: &Zoned) -> Line<'static> {
    Line::from(format!("{} {}", short_weekday(strings, now), now.day()).light_blue())
}

pub fn short_date(strings: &Strings, now: &Zoned) -> Line<'static> {
    let month = strings.short_months[now.month() as usize - 1];
    Line::from(format!("{} {} {month}", short_weekday(strings, now), now.day()).light_blue())
}

fn short_weekday(strings: &Strings, now: &Zoned) -> &'static str {
    strings.short_weekdays[now.weekday().to_sunday_zero_offset() as usize]
}

/// The ISO 8601 week number, dim.
pub fn iso_week(strings: &Strings, now: &Zoned) -> Line<'static> {
    let week = now.date().iso_week_date().week();
    Line::from(format!("{} {week}", strings.iso_week).dark_gray())
}

impl Calendar {
    pub fn new(config: &Config) -> Self {
        Self {
            week_start: config.week_start,
            strings: Strings::of(config.language),
        }
    }

    pub fn at(&self, now: &Zoned) -> ShownCalendar {
        ShownCalendar {
            week_strip: self.week_strip(now),
            short_date: short_date(self.strings, now),
            weekday_and_day: weekday_and_day(self.strings, now),
        }
    }

    /// The current week: the weekday header over the days.
    pub fn week_strip(&self, now: &Zoned) -> [Line<'static>; 2] {
        let today = now.date();
        let weeks = self.weeks(today, 1);
        [self.weekday_header(), week_row(&weeks[0], today)]
    }

    fn weekday_header(&self) -> Line<'static> {
        let initials = (0..7).map(|offset| {
            let weekday = self.first_weekday().wrapping_add(offset);
            let initial = self.strings.weekday_initials[weekday.to_sunday_zero_offset() as usize];
            let style = if is_weekend(weekday) {
                Style::new().green()
            } else {
                Style::new().dark_gray()
            };
            Span::styled(initial, style)
        });
        Line::from(spaced(initials.collect()))
    }

    /// `count` weeks from the configured first weekday, the first one holding `day`. A week at
    /// the edge of the supported dates is short.
    fn weeks(&self, day: Date, count: i64) -> Vec<Vec<Date>> {
        let days_since_start = i64::from(day.weekday().since(self.first_weekday()));
        (0..count)
            .map(|week| {
                (0..7)
                    .filter_map(|offset| {
                        day.checked_add((7 * week + offset - days_since_start).days())
                            .ok()
                    })
                    .collect()
            })
            .collect()
    }

    fn first_weekday(&self) -> Weekday {
        match self.week_start {
            WeekStart::Monday => Weekday::Monday,
            WeekStart::Sunday => Weekday::Sunday,
        }
    }
}

fn week_row(week: &[Date], today: Date) -> Line<'static> {
    let days =
        (week.iter()).map(|&day| Span::styled(format!("{:>2}", day.day()), day_style(day, today)));
    Line::from(spaced(days.collect()))
}

impl ShownCalendar {
    /// The weekday header over the current week.
    pub fn week_strip(&self) -> &[Line<'static>; 2] {
        &self.week_strip
    }

    pub fn short_date(&self) -> &Line<'static> {
        &self.short_date
    }

    pub fn weekday_and_day(&self) -> &Line<'static> {
        &self.weekday_and_day
    }
}

/// Today's pill, then past days dimmed, then the weekend tint.
fn day_style(day: Date, today: Date) -> Style {
    if day == today {
        today_style()
    } else if day < today {
        Style::new().dark_gray()
    } else if is_weekend(day.weekday()) {
        Style::new().green()
    } else {
        Style::new()
    }
}

/// Reverse video of the accent, so it follows the theme.
fn today_style() -> Style {
    Style::new().yellow().reversed()
}

/// Weekend tint follows the day of the week, not the column.
fn is_weekend(weekday: Weekday) -> bool {
    matches!(weekday, Weekday::Saturday | Weekday::Sunday)
}

/// One space between the cells, so today's pill keeps a space from its neighbours.
fn spaced(cells: Vec<Span<'static>>) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(2 * cells.len());
    for (index, cell) in cells.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(cell);
    }
    spans
}
