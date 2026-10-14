use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::{Condition, Weather, WeatherState, whole_degrees};
use crate::config::Units;

/// No usable weather means no weather part.
pub fn text_line(weather_state: WeatherState, units: Units) -> Option<Line<'static>> {
    match weather_state {
        WeatherState::Fresh(weather) => Some(weather_line(weather, units, Style::new())),
        WeatherState::Stale { weather, .. } => {
            Some(weather_line(weather, units, Style::new().dark_gray()))
        }
        _ => None,
    }
}

fn weather_line(weather: &Weather, units: Units, temperature_style: Style) -> Line<'static> {
    let current = &weather.forecast.current;
    let degrees = whole_degrees(current.temperature_2m, units);
    let mut spans = vec![Span::styled(format!("{degrees}°"), temperature_style)];
    if let Some(condition) = Condition::from_wmo(current.weather_code) {
        spans.push(Span::raw(format!(" {}", condition_mark(condition))));
    }
    Line::from(spans)
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
