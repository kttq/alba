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
    let mut line = temperature_line(weather_state, shows_fetch_indicator, units)?;
    if let Some(condition) = weather_state.weather()?.forecast.condition() {
        line.push_span(Span::raw(format!(" {}", condition_mark(condition))));
    }
    Some(line)
}

/// The temperature, dimmed when stale, and the fetch indicator after it.
pub fn temperature_line(
    weather_state: WeatherState,
    shows_fetch_indicator: bool,
    units: Units,
) -> Option<Line<'static>> {
    let temperature_style = match weather_state {
        WeatherState::Stale { .. } => Style::new().dark_gray(),
        _ => Style::new(),
    };
    let degrees = whole_degrees(weather_state.weather()?.forecast.temperature(), units);
    let mut spans = vec![Span::styled(format!("{degrees}°"), temperature_style)];
    if shows_fetch_indicator {
        spans.push(Span::raw("…").dim());
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
