use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
};

use super::{Condition, WeatherState, whole_degrees};
use crate::config::Units;

/// No usable weather means no weather part.
pub fn text_line(
    weather_state: WeatherState,
    shows_fetch_indicator: bool,
    units: Units,
) -> Option<Line<'static>> {
    let (weather, temperature_style) = match weather_state {
        WeatherState::Fresh(weather) => (weather, Style::new()),
        WeatherState::Stale { weather, .. } => (weather, Style::new().dark_gray()),
        _ => return None,
    };
    let degrees = whole_degrees(weather.forecast.temperature(), units);
    let mut spans = vec![Span::styled(format!("{degrees}°"), temperature_style)];
    if shows_fetch_indicator {
        spans.push(Span::raw("…").dim());
    }
    if let Some(condition) = weather.forecast.condition() {
        spans.push(Span::raw(format!(" {}", condition_mark(condition))));
    }
    Some(Line::from(spans))
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
