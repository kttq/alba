mod common;

use alba::layout::{self, SizeTier};
use alba::weather::FetchError;
use common::{BUENOS_AIRES, Harness, NO_MATCH, default_instant, forecast_body};
use jiff::{ToSpan, Zoned};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Rect, Size},
    style::{Color, Modifier},
};
use serde_json::Value;

fn with_weather(config_text: &str) -> Harness {
    with_weather_at(config_text, &default_instant())
}

fn with_weather_at(config_text: &str, now: &Zoned) -> Harness {
    let mut harness = Harness::new(&format!("{BUENOS_AIRES}\n{config_text}"));
    harness.fetch(now, Ok(forecast_body()));
    harness
}

/// The default forecast, changed by `edit`.
fn with_forecast(config_text: &str, edit: impl FnOnce(&mut Value)) -> Harness {
    let mut forecast: Value = serde_json::from_str(&forecast_body()).unwrap();
    edit(&mut forecast);
    let mut harness = Harness::new(&format!("{BUENOS_AIRES}\n{config_text}"));
    harness.fetch(&default_instant(), Ok(forecast.to_string()));
    harness
}

/// The text of one row, trimmed.
fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect::<String>()
        .trim()
        .to_string()
}

fn size_tier(harness: &Harness, now: &Zoned, width: u16, height: u16) -> Option<SizeTier> {
    let face = harness.app.face(now);
    layout::lay_out(Rect::new(0, 0, width, height), &face).size_tier
}

fn minimum_size(harness: &Harness, now: &Zoned, size_tier: SizeTier) -> Option<Size> {
    let face = harness.app.face(now);
    layout::minimum_size(size_tier, &face)
}

fn rendered(harness: &Harness, now: &Zoned, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| harness.app.render(frame.area(), frame.buffer_mut(), now))
        .unwrap();
    terminal
}

#[test]
fn rain_by_day_at_each_tier_minimum_and_at_large_panes() {
    let harness = with_weather("");
    let panes = [
        ("strip_minimum", 5, 1),
        ("compact_minimum", 23, 8),
        ("mini_minimum", 18, 2),
        ("85x26", 85, 26),
        ("250x80", 250, 80),
    ];

    for (name, width, height) in panes {
        insta::assert_snapshot!(
            name,
            rendered(&harness, &default_instant(), width, height).backend()
        );
    }
}

#[test]
fn mini_fits_eighteen_by_two_with_the_default_config() {
    let harness = with_weather("");

    assert_eq!(
        minimum_size(&harness, &default_instant(), SizeTier::Mini),
        Some(Size::new(18, 2))
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 18, 2),
        Some(SizeTier::Mini)
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 17, 2),
        Some(SizeTier::Strip)
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 18, 1),
        Some(SizeTier::Strip)
    );
}

#[test]
fn twelve_hour_clock_widens_mini_by_two_columns() {
    let noon = default_instant().with().hour(12).build().unwrap();
    let harness = with_weather_at("hour_format = 12", &noon);

    assert_eq!(
        minimum_size(&harness, &noon, SizeTier::Mini),
        Some(Size::new(20, 2))
    );
}

#[test]
fn without_weather_mini_does_not_fit_and_strip_takes_a_pane_below_compact() {
    let harness = Harness::new(BUENOS_AIRES);

    assert_eq!(
        minimum_size(&harness, &default_instant(), SizeTier::Mini),
        None
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 22, 12),
        Some(SizeTier::Strip)
    );
}

#[test]
fn stale_weather_dims_the_mini_temperature() {
    let harness = with_weather("");
    let later = default_instant().checked_add(2.hours()).unwrap();

    let terminal = rendered(&harness, &later, 18, 2);

    let buffer = terminal.backend().buffer();
    let degrees = (0..18)
        .find(|&x| buffer[(x, 0)].symbol() == "°")
        .expect("a temperature");
    assert_eq!(buffer[(degrees, 0)].fg, Color::DarkGray);
    assert_eq!(buffer[(degrees - 1, 0)].fg, Color::DarkGray);
}

#[test]
fn compact_fits_twenty_three_by_eight_with_the_default_config() {
    let harness = with_weather("");

    assert_eq!(
        minimum_size(&harness, &default_instant(), SizeTier::Compact),
        Some(Size::new(23, 8))
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 23, 8),
        Some(SizeTier::Compact)
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 22, 8),
        Some(SizeTier::Mini)
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 23, 7),
        Some(SizeTier::Mini)
    );
}

#[test]
fn compact_sparkline_bars_are_bright_blue_and_a_faint_low_bar_below_five_percent() {
    let next_hour = default_instant().timestamp().as_second() + 3600;
    let harness = with_forecast("", |forecast| {
        let hourly = &mut forecast["hourly"];
        let times: Vec<i64> = serde_json::from_value(hourly["time"].clone()).unwrap();
        hourly["precipitation_probability"] = times
            .iter()
            .map(|&time| if time <= next_hour { 4 } else { 100 })
            .collect();
    });

    let terminal = rendered(&harness, &default_instant(), 23, 8);

    let buffer = terminal.backend().buffer();
    assert_eq!(row_text(buffer, 7), "▀▀▀▀▀▀  ▁███████████");
    let low_bar = &buffer[(8, 7)];
    assert_eq!(low_bar.fg, Color::DarkGray);
    assert!(low_bar.modifier.contains(Modifier::DIM));
    let high_bar = &buffer[(9, 7)];
    assert_eq!(high_bar.fg, Color::LightBlue);
    assert!(!high_bar.modifier.contains(Modifier::DIM));
}

#[test]
fn twelve_hour_clock_widens_compact_by_three_columns() {
    let noon = default_instant().with().hour(12).build().unwrap();
    let harness = with_weather_at("hour_format = 12", &noon);

    assert_eq!(
        minimum_size(&harness, &noon, SizeTier::Compact),
        Some(Size::new(26, 8))
    );
}

#[test]
fn offline_compact_drops_retrying_beside_the_ghost_icon_when_it_does_not_fit() {
    let mut harness = Harness::new(BUENOS_AIRES);
    harness.fetch(
        &default_instant(),
        Err(FetchError::Network("timed out".to_string())),
    );

    let narrow = rendered(&harness, &default_instant(), 23, 8);
    let wide = rendered(&harness, &default_instant(), 26, 8);

    assert_eq!(
        size_tier(&harness, &default_instant(), 23, 8),
        Some(SizeTier::Compact)
    );
    assert_eq!(row_text(narrow.backend().buffer(), 6), "▄▄██▄  offline");
    assert_eq!(row_text(narrow.backend().buffer(), 7), "▀████▀");
    assert_eq!(
        row_text(wide.backend().buffer(), 6),
        "▄▄██▄  offline · retrying"
    );
}

#[test]
fn compact_without_weather_shows_the_status_beside_the_ghost_icon() {
    let no_match = |config_text: &str| {
        let mut harness = Harness::new(config_text);
        harness.fetch(&default_instant(), Ok(NO_MATCH.to_string()));
        harness
    };
    let fetching = || {
        let mut harness = Harness::new(BUENOS_AIRES);
        harness.app.tick(&default_instant());
        harness
    };
    let cases = [
        (Harness::new(""), 23, "no location set"),
        (Harness::new("language = \"es\""), 23, "sin ubicación"),
        (fetching(), 23, "fetching…"),
        (
            no_match("location = \"atlantis\""),
            36,
            "location not found: atlantis",
        ),
        (
            no_match("location = \"atlantis\""),
            34,
            "location not found: atlan…",
        ),
        (
            no_match("location = \"atlantis\""),
            29,
            "location not found: …",
        ),
    ];

    for (harness, width, status) in cases {
        let terminal = rendered(&harness, &default_instant(), width, 8);

        assert_eq!(
            row_text(terminal.backend().buffer(), 6),
            format!("▄▄██▄  {status}"),
            "{status}"
        );
    }
}

#[test]
fn stale_compact_dims_the_text_and_sparkline_and_shows_the_age_only_when_it_fits() {
    let later = default_instant().checked_add(2.hours()).unwrap();
    let english = with_weather("");
    let spanish = with_weather("language = \"es\"");

    let narrow = rendered(&english, &later, 23, 8);
    let wide = rendered(&english, &later, 29, 8);
    let spanish_wide = rendered(&spanish, &later, 30, 8);

    let buffer = narrow.backend().buffer();
    assert_eq!(row_text(buffer, 6), "▄██▄   14°  ↑19° ↓13°");
    for x in 8..20 {
        assert_eq!(buffer[(x, 6)].fg, Color::DarkGray, "column {x}");
    }
    for x in 8..20 {
        assert_eq!(buffer[(x, 7)].fg, Color::DarkGray, "bar {x}");
    }
    assert_eq!(buffer[(2, 6)].fg, Color::Reset, "the icon keeps its colors");
    let buffer = wide.backend().buffer();
    assert_eq!(row_text(buffer, 6), "▄██▄   14°  ↑19° ↓13° 2h ago");
    assert_eq!(buffer[(23, 6)].fg, Color::DarkGray);
    assert_eq!(
        row_text(spanish_wide.backend().buffer(), 6),
        "▄██▄   14°  ↑19° ↓13° hace 2h"
    );
}

#[test]
fn stale_compact_shows_the_fetch_indicator_after_the_age_label_or_the_temperature() {
    let later = default_instant().checked_add(2.hours()).unwrap();
    let mut harness = with_weather("");
    harness.app.key(KeyEvent::from(KeyCode::Char('r')));
    harness.app.tick(&later);
    let shown_at = later.checked_add(1.second()).unwrap();

    let narrow = rendered(&harness, &shown_at, 23, 8);
    let wide = rendered(&harness, &shown_at, 30, 8);

    assert_eq!(
        row_text(narrow.backend().buffer(), 6),
        "▄██▄   14°…  ↑19° ↓13°"
    );
    assert_eq!(
        row_text(wide.backend().buffer(), 6),
        "▄██▄   14°  ↑19° ↓13° 2h ago…"
    );
}
