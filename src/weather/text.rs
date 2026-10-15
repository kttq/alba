use jiff::Timestamp;
use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
};

use super::{Condition, WeatherState, whole_degrees};
use crate::{config::Units, strings::Strings};

/// One per eighth of 100%: `▁` up to 12.5%, `█` above 87.5%.
const SPARKLINE_BARS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
/// Below this percent, a bar is a faint `▁`.
const SPARKLINE_FLOOR: u8 = 5;

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
    let temperature_style = text_style(weather_state);
    let degrees = whole_degrees(weather_state.weather()?.forecast.temperature(), units);
    let mut spans = vec![Span::styled(format!("{degrees}°"), temperature_style)];
    if shows_fetch_indicator {
        spans.push(Span::raw("…").dim());
    }
    Some(Line::from(spans))
}

/// High and low from the forecast day's row, after the temperature.
pub fn temperature_high_and_low_line(
    weather_state: WeatherState,
    shows_fetch_indicator: bool,
    units: Units,
    now: Timestamp,
) -> Option<Line<'static>> {
    let mut line = temperature_line(weather_state, shows_fetch_indicator, units)?;
    let Some(high_and_low) = weather_state.weather()?.forecast.high_and_low(now) else {
        return Some(line);
    };
    let style = text_style(weather_state);
    let arrow_style = Style::new().dark_gray();
    line.extend([
        Span::styled("  ↑", arrow_style),
        Span::styled(
            format!("{}°", whole_degrees(high_and_low.high, units)),
            style,
        ),
        Span::styled(" ↓", arrow_style),
        Span::styled(
            format!("{}°", whole_degrees(high_and_low.low, units)),
            style,
        ),
    ]);
    Some(line)
}

/// The precipitation probability for each of the next 12 hours.
pub fn sparkline(weather_state: WeatherState, now: Timestamp) -> Option<Line<'static>> {
    let percents = weather_state.weather()?.forecast.precipitation_after(now);
    let bar_style = match weather_state {
        WeatherState::Stale { .. } => Style::new().dark_gray(),
        _ => Style::new().light_blue(),
    };
    let bars = percents.iter().map(|&percent| {
        if percent < SPARKLINE_FLOOR {
            return Span::styled("▁", Style::new().dark_gray().dim());
        }
        let eighths = (usize::from(percent) * 8).div_ceil(100).clamp(1, 8);
        Span::styled(SPARKLINE_BARS[eighths - 1], bar_style)
    });
    Some(Line::from_iter(bars))
}

/// The age label follows the high and low, and the fetch indicator moves after the label.
/// None unless stale.
pub fn aged_temperature_high_and_low_line(
    weather_state: WeatherState,
    shows_fetch_indicator: bool,
    units: Units,
    now: Timestamp,
    strings: &Strings,
) -> Option<Line<'static>> {
    let WeatherState::Stale { age, .. } = weather_state else {
        return None;
    };
    let mut line = temperature_high_and_low_line(weather_state, false, units, now)?;
    let [before, after] = strings.age;
    line.push_span(Span::styled(
        format!(" {before}{}{after}", age.as_hours()),
        Style::new().dark_gray(),
    ));
    if shows_fetch_indicator {
        line.push_span(Span::raw("…").dim());
    }
    Some(line)
}

/// What the weather area says without weather, each way widest first; empty with weather.
pub fn status_lines(weather_state: WeatherState, strings: &Strings) -> Vec<Line<'static>> {
    let texts = match weather_state {
        WeatherState::NoLocation => vec![strings.no_location.to_string()],
        WeatherState::Fetching => vec![strings.fetching.to_string()],
        WeatherState::Offline => dropping_after_separator(strings.offline),
        WeatherState::LocationNotFound(name) => location_not_found(strings, name),
        WeatherState::Fresh(_) | WeatherState::Stale { .. } => Vec::new(),
    };
    texts
        .into_iter()
        .map(|text| Line::styled(text, Style::new().dark_gray()))
        .collect()
}

/// The text after `·` drops when the whole line does not fit.
fn dropping_after_separator(text: &str) -> Vec<String> {
    match text.split_once(" · ") {
        Some((before, _)) => vec![text.to_string(), before.to_string()],
        None => vec![text.to_string()],
    }
}

/// The name shortens with `…` one character at a time, down to `…` alone.
fn location_not_found(strings: &Strings, name: &str) -> Vec<String> {
    let name: Vec<char> = name
        .chars()
        .filter(|character| !character.is_control())
        .collect();
    let prefix = strings.location_not_found;
    let mut texts = vec![format!("{prefix}: {}", String::from_iter(&name))];
    texts.extend(
        (1..name.len())
            .rev()
            .map(|kept| format!("{prefix}: {}…", String::from_iter(&name[..kept]))),
    );
    texts.push(format!("{prefix}: …"));
    texts
}

fn text_style(weather_state: WeatherState) -> Style {
    match weather_state {
        WeatherState::Stale { .. } => Style::new().dark_gray(),
        _ => Style::new(),
    }
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
