use ratatui::text::Line;

use super::{Condition, Weather, whole_degrees};
use crate::config::Units;

pub fn text_line(weather: &Weather, units: Units) -> Line<'static> {
    let current = &weather.forecast.current;
    let degrees = whole_degrees(current.temperature_2m, units);
    let text = match Condition::from_wmo(current.weather_code) {
        Some(condition) => format!("{degrees}° {}", condition_mark(condition)),
        None => format!("{degrees}°"),
    };
    Line::raw(text)
}

/// Each mark must take two terminal cells.
fn condition_mark(condition: Condition) -> &'static str {
    match condition {
        Condition::Clear => "✦✦",
        Condition::PartlyCloudy => "✦~",
        Condition::Cloudy => "~~",
        Condition::Fog => "≈≈",
        Condition::Drizzle => "′′",
        Condition::Rain => "‚‚",
        Condition::HeavyRain => "┆┆",
        Condition::Snow => "**",
        Condition::Thunderstorm => "!!",
    }
}
