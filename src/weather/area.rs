//! The weather area: the readings, or what the weather state says without weather data.

use jiff::{SignedDuration, Timestamp};
use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
};

use super::{Condition, Weather, WeatherState, whole_degrees};
use crate::{config::Units, strings::Strings};

/// One per eighth of 100%: `▁` up to 12.5%, `█` above 87.5%.
const SPARKLINE_BARS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
/// Below this percent, a bar is a faint `▁`.
const SPARKLINE_FLOOR: u8 = 5;

pub enum WeatherArea {
    Readings(Readings),
    NoWeather {
        /// Widest first.
        lines: Vec<Line<'static>>,
        /// Two dim lines that say where to set the location. Only without a location.
        first_run_hint: Option<[Line<'static>; 2]>,
    },
}

/// The weather's text parts, each styled for fresh or stale weather.
pub struct Readings {
    temperature: Span<'static>,
    /// From the forecast day's row; empty when the forecast has no row for it.
    high_and_low: Vec<Span<'static>>,
    /// Only for stale weather.
    age_label: Option<Span<'static>>,
    condition_mark: Option<Span<'static>>,
    /// None for a weather code with no condition.
    condition: Option<Span<'static>>,
    sparkline: Line<'static>,
    shows_fetch_indicator: bool,
}

/// A part of a readings line, in the order a size tier gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Temperature,
    /// Nothing unless the weather is stale.
    Age,
    HighAndLow,
    ConditionMark,
    /// The condition's name.
    Condition,
}

impl WeatherArea {
    pub fn of(
        weather_state: WeatherState,
        shows_fetch_indicator: bool,
        units: Units,
        now: Timestamp,
        strings: &Strings,
        shown_config_file: &str,
    ) -> Self {
        if let Some(weather) = weather_state.weather() {
            let age = match weather_state {
                WeatherState::Stale { age, .. } => Some(age),
                _ => None,
            };
            return Self::Readings(Readings::of(
                weather,
                age,
                shows_fetch_indicator,
                units,
                now,
                strings,
            ));
        }
        Self::NoWeather {
            lines: weather_state_lines(weather_state, strings),
            first_run_hint: (weather_state == WeatherState::NoLocation)
                .then(|| first_run_hint(strings, shown_config_file)),
        }
    }
}

impl Readings {
    fn of(
        weather: &Weather,
        age: Option<SignedDuration>,
        shows_fetch_indicator: bool,
        units: Units,
        now: Timestamp,
        strings: &Strings,
    ) -> Self {
        let text_style = match age {
            Some(_) => Style::new().dark_gray(),
            None => Style::new(),
        };
        let degrees =
            |celsius| Span::styled(format!("{}°", whole_degrees(celsius, units)), text_style);
        let arrow_style = Style::new().dark_gray();
        let high_and_low = (weather.forecast.high_and_low(now).into_iter())
            .flat_map(|high_and_low| {
                [
                    Span::styled("  ↑", arrow_style),
                    degrees(high_and_low.high),
                    Span::styled(" ↓", arrow_style),
                    degrees(high_and_low.low),
                ]
            })
            .collect();
        let condition_mark = (weather.forecast.condition())
            .map(|condition| Span::raw(format!(" {}", condition_mark(condition))));
        let condition = (weather.forecast.condition())
            .map(|condition| Span::styled(strings.condition(condition), text_style));
        Self {
            temperature: degrees(weather.forecast.temperature()),
            high_and_low,
            age_label: age.map(|age| age_label(age, strings)),
            condition_mark,
            condition,
            sparkline: sparkline(weather.forecast.precipitation_after(now), age.is_some()),
            shows_fetch_indicator,
        }
    }

    pub fn is_stale(&self) -> bool {
        self.age_label.is_some()
    }

    /// The parts in the given order. The fetch indicator follows the age label when the line
    /// shows it, and the temperature otherwise.
    pub fn line(&self, parts: &[Part]) -> Line<'static> {
        let fetch_indicator_after = match self.age_label {
            Some(_) if parts.contains(&Part::Age) => Part::Age,
            _ => Part::Temperature,
        };
        let mut spans = Vec::new();
        for &part in parts {
            match part {
                Part::Temperature => spans.push(self.temperature.clone()),
                Part::Age => spans.extend(self.age_label.clone()),
                Part::HighAndLow => spans.extend(self.high_and_low.iter().cloned()),
                Part::ConditionMark => spans.extend(self.condition_mark.clone()),
                Part::Condition => spans.extend(self.condition.clone()),
            }
            if part == fetch_indicator_after && self.shows_fetch_indicator {
                spans.push(Span::raw("…").dim());
            }
        }
        Line::from(spans)
    }

    /// The precipitation probability for each of the next 12 hours.
    pub fn sparkline(&self) -> &Line<'static> {
        &self.sparkline
    }
}

fn sparkline(percents: &[u8], is_stale: bool) -> Line<'static> {
    let bar_style = if is_stale {
        Style::new().dark_gray()
    } else {
        Style::new().light_blue()
    };
    let bars = percents.iter().map(|&percent| {
        if percent < SPARKLINE_FLOOR {
            return Span::styled("▁", Style::new().dark_gray().dim());
        }
        let eighths = (usize::from(percent) * 8).div_ceil(100).clamp(1, 8);
        Span::styled(SPARKLINE_BARS[eighths - 1], bar_style)
    });
    Line::from_iter(bars)
}

/// The age in whole hours.
fn age_label(age: SignedDuration, strings: &Strings) -> Span<'static> {
    let [before, after] = strings.age;
    Span::styled(
        format!(" {before}{}{after}", age.as_hours()),
        Style::new().dark_gray(),
    )
}

fn first_run_hint(strings: &Strings, shown_config_file: &str) -> [Line<'static>; 2] {
    let style = Style::new().dark_gray();
    [
        Line::styled(strings.first_run_hint, style),
        Line::styled(shown_config_file.to_string(), style),
    ]
}

/// Each way widest first.
fn weather_state_lines(weather_state: WeatherState, strings: &Strings) -> Vec<Line<'static>> {
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
