use jiff::Zoned;
use ratatui::{style::Stylize, text::Line};

use crate::strings::Strings;

/// The short weekday and the day of the month, as in `tue 13`.
pub fn short_date(strings: &Strings, now: &Zoned) -> Line<'static> {
    let weekday = strings.short_weekdays[now.weekday().to_sunday_zero_offset() as usize];
    Line::from(format!("{weekday} {}", now.day()).light_blue())
}
