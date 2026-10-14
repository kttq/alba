mod common;

use std::fs;

use alba::{config::Coordinates, weather::FetchError};
use common::{
    BUENOS_AIRES, Harness, default_instant, forecast_body, make_read_only, temporary_directory,
};
use ratatui::{
    style::Stylize,
    text::{Line, Span},
};

#[test]
fn fetched_weather_shows_on_the_text_line() {
    let mut harness = Harness::new(BUENOS_AIRES);
    let now = default_instant();

    harness.fetch(&now, Ok(forecast_body()));

    harness
        .render_row(&now, 23)
        .backend()
        .assert_buffer_lines([Line::from(vec![
            Span::raw("06:38 · "),
            "tue 13".light_blue(),
            Span::raw(" · 14° ‚‚"),
        ])]);
}

#[test]
fn narrow_pane_drops_the_date_first() {
    let mut harness = Harness::new(BUENOS_AIRES);
    let now = default_instant();

    harness.fetch(&now, Ok(forecast_body()));

    assert_eq!(harness.text_line(&now, 22), "06:38 · 14° ‚‚");
    assert_eq!(harness.text_line(&now, 13), "06:38");
}

#[test]
fn imperial_units_show_whole_fahrenheit() {
    let mut harness = Harness::new(&format!("{BUENOS_AIRES}\nunits = \"imperial\""));
    let now = default_instant();

    harness.fetch(&now, Ok(forecast_body()));

    assert_eq!(harness.text_line(&now, 23), "06:38 · tue 13 · 58° ‚‚");
}

/// The fixture with its current weather code replaced.
fn forecast_with_code(code: u16) -> String {
    forecast_body().replace("\"weather_code\":61", &format!("\"weather_code\":{code}"))
}

#[test]
fn failed_fetches_show_no_weather() {
    let failures = [
        (
            "null",
            Ok(forecast_body().replace("\"temperature_2m\":14.4", "\"temperature_2m\":null")),
        ),
        ("malformed", Ok(forecast_body()[..200].to_string())),
        (
            "html",
            Ok("<html><body>bad gateway</body></html>".to_string()),
        ),
        ("empty", Ok(String::new())),
        (
            "uneven columns",
            Ok(forecast_body().replace("\"sunrise\":[", "\"sunrise\":[1791800000,")),
        ),
        (
            "day flag out of range",
            Ok(forecast_body().replace("\"is_day\":1", "\"is_day\":7")),
        ),
        ("http error", Err(FetchError::Status(503))),
        (
            "network error",
            Err(FetchError::Network("timed out".to_string())),
        ),
    ];
    let now = default_instant();

    for (name, failure) in failures {
        let mut harness = Harness::new(BUENOS_AIRES);
        harness.fetch(&now, failure);

        assert_eq!(harness.text_line(&now, 22), "06:38 · tue 13", "{name}");
    }
}

#[test]
fn unknown_weather_code_shows_the_temperature_alone() {
    let mut harness = Harness::new(BUENOS_AIRES);
    let now = default_instant();

    harness.fetch(&now, Ok(forecast_with_code(42)));

    assert_eq!(harness.text_line(&now, 23), "06:38 · tue 13 · 14°");
}

#[test]
fn each_condition_has_a_two_cell_mark() {
    let marks = [
        (0, "✦✦"),
        (2, "✦~"),
        (3, "~~"),
        (45, "≈≈"),
        (51, "′′"),
        (61, "‚‚"),
        (65, "┆┆"),
        (71, "**"),
        (95, "!!"),
    ];
    let now = default_instant();

    for (code, mark) in marks {
        let mut harness = Harness::new(BUENOS_AIRES);
        harness.fetch(&now, Ok(forecast_with_code(code)));

        assert_eq!(
            harness.text_line(&now, 23),
            format!("06:38 · tue 13 · 14° {mark}")
        );
    }
}

#[test]
fn configured_coordinates_are_fetched_once() {
    let mut harness = Harness::new(BUENOS_AIRES);
    let now = default_instant();

    harness.app.tick(&now);
    harness.app.tick(&now);

    assert_eq!(
        harness.weather_source.requests(),
        [Coordinates {
            latitude: -34.6037,
            longitude: -58.3816
        }]
    );
}

#[test]
fn no_coordinates_means_no_fetch() {
    for config_text in ["", "location = \"buenos aires\""] {
        let mut harness = Harness::new(config_text);

        harness.app.tick(&default_instant());

        assert_eq!(harness.weather_source.requests(), [], "{config_text}");
    }
}

#[test]
fn fetched_weather_shows_at_once_on_the_next_start() {
    let cache_directory = temporary_directory();
    let now = default_instant();
    Harness::with_cache(BUENOS_AIRES, cache_directory.clone()).fetch(&now, Ok(forecast_body()));

    let next_start = Harness::with_cache(BUENOS_AIRES, cache_directory.clone());

    assert_eq!(next_start.text_line(&now, 23), "06:38 · tue 13 · 14° ‚‚");
    let files: Vec<_> = fs::read_dir(&cache_directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(files, ["weather.json"]);
}

#[test]
fn failed_fetch_keeps_the_cached_weather() {
    let cache_directory = temporary_directory();
    let now = default_instant();
    Harness::with_cache(BUENOS_AIRES, cache_directory.clone()).fetch(&now, Ok(forecast_body()));
    let mut next_start = Harness::with_cache(BUENOS_AIRES, cache_directory);

    next_start.fetch(&now, Err(FetchError::Status(500)));

    assert_eq!(next_start.text_line(&now, 23), "06:38 · tue 13 · 14° ‚‚");
}

#[test]
fn cache_for_other_coordinates_counts_as_no_cache() {
    let cache_directory = temporary_directory();
    let now = default_instant();
    Harness::with_cache(BUENOS_AIRES, cache_directory.clone()).fetch(&now, Ok(forecast_body()));

    let elsewhere = Harness::with_cache("coordinates = [48.8566, 2.3522]", cache_directory);

    assert_eq!(elsewhere.text_line(&now, 22), "06:38 · tue 13");
}

/// Turns a good cache file's text into a broken one.
type Breakage = fn(String) -> String;

#[test]
fn unusable_cache_counts_as_no_cache() {
    let now = default_instant();
    let breakages: [(&str, Breakage); 3] = [
        ("other version", |text| {
            text.replace("\"version\":1", "\"version\":0")
        }),
        ("malformed", |text| text[..text.len() / 2].to_string()),
        ("not text", |_| "\u{0}\u{fffd}".to_string()),
    ];

    for (name, breakage) in breakages {
        let cache_directory = temporary_directory();
        Harness::with_cache(BUENOS_AIRES, cache_directory.clone()).fetch(&now, Ok(forecast_body()));
        let cache_file = cache_directory.join("weather.json");
        let text = fs::read_to_string(&cache_file).unwrap();
        fs::write(&cache_file, breakage(text)).unwrap();

        let next_start = Harness::with_cache(BUENOS_AIRES, cache_directory);

        assert_eq!(next_start.text_line(&now, 22), "06:38 · tue 13", "{name}");
    }
}

#[test]
fn failed_cache_write_keeps_the_weather_in_memory() {
    let cache_directory = temporary_directory();
    fs::create_dir_all(&cache_directory).unwrap();
    make_read_only(&cache_directory);
    let mut harness = Harness::with_cache(BUENOS_AIRES, cache_directory.clone());
    let now = default_instant();

    harness.fetch(&now, Ok(forecast_body()));

    assert_eq!(harness.text_line(&now, 23), "06:38 · tue 13 · 14° ‚‚");
    assert!(!cache_directory.join("weather.json").exists());
}
