use jiff::Zoned;
use ratatui::{style::Stylize, text::Line};

use crate::strings::Strings;

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
