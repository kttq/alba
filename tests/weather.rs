mod common;

use std::fs;

use alba::{app::Control, config::Coordinates, weather::FetchError};
use common::{
    BUENOS_AIRES, Harness, default_instant, forecast_body, make_read_only, temporary_directory,
};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    style::Stylize,
    text::{Line, Span},
};
use serde_json::{Value, json};

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

fn edited_forecast(edit: impl FnOnce(&mut Value)) -> String {
    let mut forecast: Value = serde_json::from_str(&forecast_body()).unwrap();
    edit(&mut forecast);
    forecast.to_string()
}

fn forecast_with_code(code: u16) -> String {
    edited_forecast(|forecast| forecast["current"]["weather_code"] = json!(code))
}

#[test]
fn failed_fetches_show_no_weather() {
    let failures = [
        (
            "null",
            Ok(edited_forecast(|forecast| {
                forecast["current"]["temperature_2m"] = Value::Null
            })),
        ),
        ("malformed", Ok(forecast_body()[..200].to_string())),
        (
            "html",
            Ok("<html><body>bad gateway</body></html>".to_string()),
        ),
        ("empty", Ok(String::new())),
        (
            "uneven columns",
            Ok(edited_forecast(|forecast| {
                forecast["daily"]["sunrise"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(1791800000))
            })),
        ),
        (
            "day flag out of range",
            Ok(edited_forecast(|forecast| {
                forecast["current"]["is_day"] = json!(7)
            })),
        ),
        (
            "http error",
            Err(FetchError::Status {
                status: 503,
                retry_after: None,
            }),
        ),
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

    next_start.fetch(
        &now,
        Err(FetchError::Status {
            status: 500,
            retry_after: None,
        }),
    );

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

fn hours_later(hours: i64) -> jiff::Zoned {
    default_instant()
        .checked_add(jiff::SignedDuration::from_hours(hours))
        .unwrap()
}

#[test]
fn stale_weather_dims_the_temperature_with_no_age_label() {
    let mut harness = Harness::new(BUENOS_AIRES);
    harness.fetch(&default_instant(), Ok(forecast_body()));

    harness
        .render_row(&hours_later(2), 23)
        .backend()
        .assert_buffer_lines([Line::from(vec![
            Span::raw("08:38 · "),
            "tue 13".light_blue(),
            Span::raw(" · "),
            "14°".dark_gray(),
            Span::raw(" ‚‚"),
        ])]);
}

#[test]
fn expired_weather_drops_the_weather_part() {
    let mut harness = Harness::new(BUENOS_AIRES);
    harness.fetch(&default_instant(), Ok(forecast_body()));

    assert_eq!(harness.text_line(&hours_later(13), 23), "19:38 · tue 13");
}

#[test]
fn r_fetches_the_weather_now() {
    let mut harness = Harness::new(BUENOS_AIRES);
    let now = default_instant();
    harness.fetch(&now, Ok(forecast_body()));

    let control = harness.app.key(KeyEvent::from(KeyCode::Char('r')));
    harness.app.tick(&now);

    assert_eq!(control, Control::Continue);
    assert_eq!(harness.weather_source.requests().len(), 2);
}

#[test]
fn running_fetch_adds_a_dim_ellipsis_after_300_ms() {
    let mut harness = Harness::new(BUENOS_AIRES);
    let now = default_instant();
    harness.fetch(&now, Ok(forecast_body()));
    harness.app.key(KeyEvent::from(KeyCode::Char('r')));
    harness.app.tick(&now);
    let later = |milliseconds| {
        now.checked_add(jiff::SignedDuration::from_millis(milliseconds))
            .unwrap()
    };

    assert_eq!(
        harness.text_line(&later(299), 24),
        "06:38 · tue 13 · 14° ‚‚"
    );
    harness
        .render_row(&later(300), 24)
        .backend()
        .assert_buffer_lines([Line::from(vec![
            Span::raw("06:38 · "),
            "tue 13".light_blue(),
            Span::raw(" · 14°"),
            "…".dim(),
            Span::raw(" ‚‚"),
        ])]);
}
