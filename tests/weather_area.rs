mod common;

use alba::{
    config::Units,
    layout::SizeTier,
    strings::Strings,
    weather::{Forecast, Readings, Weather, WeatherArea, WeatherState},
};
use common::{default_instant, forecast_body};
use jiff::SignedDuration;
use ratatui::{
    style::{Color, Modifier},
    text::Line,
};
use serde_json::Value;

const CONFIG_FILE: &str = "~/.config/alba/config.toml";

fn weather() -> Weather {
    weather_from(&forecast_body())
}

fn weather_from(forecast_body: &str) -> Weather {
    Weather {
        fetched_at: default_instant().timestamp(),
        forecast: Forecast::parse(forecast_body).unwrap(),
    }
}

fn weather_area(weather_state: WeatherState, shows_fetch_indicator: bool) -> WeatherArea {
    WeatherArea::of(
        weather_state,
        shows_fetch_indicator,
        Units::Metric,
        default_instant().timestamp(),
        &Strings::EN,
        CONFIG_FILE,
    )
}

fn readings(weather_state: WeatherState, shows_fetch_indicator: bool) -> Readings {
    match weather_area(weather_state, shows_fetch_indicator) {
        WeatherArea::Readings(readings) => readings,
        WeatherArea::NoWeather { .. } => panic!("no readings for {weather_state:?}"),
    }
}

fn stale(weather: &Weather) -> WeatherState<'_> {
    WeatherState::Stale {
        weather,
        age: SignedDuration::from_hours(2),
    }
}

fn text(line: &Line) -> String {
    line.to_string()
}

/// Each way to show the size tier's temperature line, widest first.
fn temperature_lines(readings: &Readings, size_tier: SizeTier) -> Vec<String> {
    (readings.temperature_lines(size_tier).iter())
        .map(text)
        .collect()
}

#[test]
fn each_size_tier_shows_its_parts_of_the_readings() {
    let weather = weather();
    let readings = readings(WeatherState::Fresh(&weather), false);

    for size_tier in [
        SizeTier::Large,
        SizeTier::Wide,
        SizeTier::Tall,
        SizeTier::Medium,
        SizeTier::Compact,
    ] {
        assert_eq!(
            temperature_lines(&readings, size_tier),
            ["14°  ↑19° ↓13°"],
            "{size_tier:?}"
        );
    }
    assert_eq!(temperature_lines(&readings, SizeTier::Mini), ["14°"]);
    assert_eq!(temperature_lines(&readings, SizeTier::Strip), ["14° ‚‚"]);
}

#[test]
fn compact_puts_the_age_label_last_and_drops_it_first() {
    let weather = weather();
    let stale = readings(stale(&weather), false);

    assert_eq!(
        temperature_lines(&stale, SizeTier::Compact),
        ["14°  ↑19° ↓13° 2h ago", "14°  ↑19° ↓13°"]
    );
    assert_eq!(
        temperature_lines(&stale, SizeTier::Medium),
        ["14° 2h ago  ↑19° ↓13°"]
    );
}

#[test]
fn fetch_indicator_follows_the_age_label_when_shown_and_the_temperature_otherwise() {
    let weather = weather();
    let fresh = readings(WeatherState::Fresh(&weather), true);
    let stale = readings(stale(&weather), true);

    assert_eq!(
        temperature_lines(&fresh, SizeTier::Large),
        ["14°…  ↑19° ↓13°"]
    );
    assert_eq!(
        temperature_lines(&stale, SizeTier::Large),
        ["14° 2h ago…  ↑19° ↓13°"]
    );
    assert_eq!(
        temperature_lines(&stale, SizeTier::Compact),
        ["14°  ↑19° ↓13° 2h ago…", "14°…  ↑19° ↓13°"]
    );
}

#[test]
fn stale_weather_dims_the_temperatures_and_the_sparkline() {
    let weather = weather();
    let fresh = readings(WeatherState::Fresh(&weather), false);
    let stale = readings(stale(&weather), false);

    let line = stale.temperature_lines(SizeTier::Compact).swap_remove(1);
    for span in line.spans.iter().filter(|span| span.content.contains('°')) {
        assert_eq!(span.style.fg, Some(Color::DarkGray), "{span:?}");
    }
    let line = fresh.temperature_lines(SizeTier::Mini).swap_remove(0);
    assert_eq!(line.spans[0].style.fg, None);
    assert!(
        (stale.sparkline().spans.iter()).all(|bar| bar.style.fg == Some(Color::DarkGray)),
        "{:?}",
        stale.sparkline()
    );
}

#[test]
fn sparkline_bars_are_bright_blue_and_a_faint_low_bar_below_five_percent() {
    let next_hour = default_instant().timestamp().as_second() + 3600;
    let mut forecast: Value = serde_json::from_str(&forecast_body()).unwrap();
    let hourly = &mut forecast["hourly"];
    let times: Vec<i64> = serde_json::from_value(hourly["time"].clone()).unwrap();
    hourly["precipitation_probability"] = times
        .iter()
        .map(|&time| if time <= next_hour { 4 } else { 100 })
        .collect();
    let weather = weather_from(&forecast.to_string());

    let readings = readings(WeatherState::Fresh(&weather), false);

    let bars = &readings.sparkline().spans;
    assert_eq!(text(readings.sparkline()), "▁███████████");
    assert_eq!(bars[0].style.fg, Some(Color::DarkGray));
    assert!(bars[0].style.add_modifier.contains(Modifier::DIM));
    assert_eq!(bars[1].style.fg, Some(Color::LightBlue));
    assert!(!bars[1].style.add_modifier.contains(Modifier::DIM));
}

#[test]
fn without_a_location_the_weather_area_has_the_first_run_hint_and_a_short_line() {
    let WeatherArea::NoWeather {
        lines,
        first_run_hint,
    } = weather_area(WeatherState::NoLocation, false)
    else {
        panic!("readings without a location");
    };

    assert_eq!(
        lines.iter().map(text).collect::<Vec<_>>(),
        [Strings::EN.no_location]
    );
    let first_run_hint = first_run_hint.expect("a first-run hint");
    assert_eq!(text(&first_run_hint[1]), CONFIG_FILE);
}

#[test]
fn only_no_location_has_the_first_run_hint() {
    for weather_state in [
        WeatherState::Fetching,
        WeatherState::Offline,
        WeatherState::LocationNotFound("atlantis"),
    ] {
        let WeatherArea::NoWeather { first_run_hint, .. } = weather_area(weather_state, false)
        else {
            panic!("readings for {weather_state:?}");
        };
        assert!(first_run_hint.is_none(), "{weather_state:?}");
    }
}

#[test]
fn offline_lines_drop_the_text_after_the_separator_widest_first() {
    let WeatherArea::NoWeather { lines, .. } = weather_area(WeatherState::Offline, false) else {
        panic!("readings when offline");
    };

    let widths: Vec<_> = lines.iter().map(Line::width).collect();
    assert_eq!(widths.len(), 2);
    assert!(widths[0] > widths[1], "{widths:?}");
}

#[test]
fn condition_name_is_the_main_text_color_and_dims_when_stale() {
    let weather = weather();
    let fresh = readings(WeatherState::Fresh(&weather), false);
    let stale = readings(stale(&weather), false);

    let line = fresh.condition_line();
    assert_eq!(text(&line), "rain");
    assert_eq!(line.spans[0].style.fg, None);
    let line = stale.condition_line();
    assert_eq!(text(&line), "rain");
    assert_eq!(line.spans[0].style.fg, Some(Color::DarkGray));
}

#[test]
fn sparkline_label_is_dim() {
    let weather = weather();
    let readings = readings(WeatherState::Fresh(&weather), false);

    let label = readings.sparkline_label();
    assert_eq!(text(label), "precip · next 12h");
    assert_eq!(label.style.fg, Some(Color::DarkGray));
}
