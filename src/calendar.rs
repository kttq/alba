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

    /// The current week: the weekday header over the days, with a column of space on each side
    /// so today's pill keeps its padding at either end.
    pub fn week_strip(&self, now: &Zoned) -> [Line<'static>; 2] {
        let today = now.date();
        let days = self.week_of(today);
        let header = days.iter().map(|day| {
            let initials =
                self.strings.weekday_initials[day.weekday().to_sunday_zero_offset() as usize];
            let style = if is_weekend(day.weekday()) {
                Style::new().green()
            } else {
                Style::new().dark_gray()
            };
            Span::styled(initials, style)
        });
        let numbers = days
            .iter()
            .map(|&day| Span::styled(format!("{:>2}", day.day()), day_style(day, today)));
        let today_index = days.iter().position(|&day| day == today);
        [
            Line::from(spaced(header.collect(), &[])),
            Line::from(spaced(numbers.collect(), today_index.as_slice())),
        ]
    }

    /// The seven days of the week that holds `today`, from the configured first weekday.
    fn week_of(&self, today: Date) -> Vec<Date> {
        let first_weekday = match self.week_start {
            WeekStart::Monday => Weekday::Monday,
            WeekStart::Sunday => Weekday::Sunday,
        };
        let days_since_start = i64::from(today.weekday().since(first_weekday));
        (0..7)
            .filter_map(|offset| today.checked_add((offset - days_since_start).days()).ok())
            .collect()
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

/// One space before, between, and after the cells. The spaces on each side of a pill cell take
/// the pill's style.
fn spaced(cells: Vec<Span<'static>>, pills: &[usize]) -> Vec<Span<'static>> {
    let space = |touches_pill: bool| {
        if touches_pill {
            Span::styled(" ", today_style())
        } else {
            Span::raw(" ")
        }
    };
    let count = cells.len();
    let mut spans = Vec::with_capacity(2 * count + 1);
    for (index, cell) in cells.into_iter().enumerate() {
        let after_pill = index > 0 && pills.contains(&(index - 1));
        spans.push(space(after_pill || pills.contains(&index)));
        spans.push(cell);
    }
    spans.push(space(count > 0 && pills.contains(&(count - 1))));
    spans
}
