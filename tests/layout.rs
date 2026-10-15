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

fn with_weather(config_text: &str) -> Harness {
    with_weather_at(config_text, &default_instant())
}

fn with_weather_at(config_text: &str, now: &Zoned) -> Harness {
    let mut harness = Harness::new(&format!("{BUENOS_AIRES}\n{config_text}"));
    harness.fetch(now, Ok(forecast_body()));
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
        ("medium_minimum", 40, 15),
        ("wide_minimum", 63, 11),
        ("tall_minimum", 40, 21),
        ("large_minimum", 71, 23),
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
fn compact_without_weather_shows_the_weather_state_beside_the_ghost_icon() {
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
fn stale_compact_shows_the_age_only_when_it_fits_and_keeps_the_icon_colors() {
    let later = default_instant().checked_add(2.hours()).unwrap();
    let english = with_weather("");
    let spanish = with_weather("language = \"es\"");

    let narrow = rendered(&english, &later, 23, 8);
    let wide = rendered(&english, &later, 29, 8);
    let spanish_wide = rendered(&spanish, &later, 30, 8);

    let buffer = narrow.backend().buffer();
    assert_eq!(row_text(buffer, 6), "▄██▄   14°  ↑19° ↓13°");
    assert_eq!(buffer[(2, 6)].fg, Color::Reset, "the icon keeps its colors");
    assert_eq!(
        row_text(wide.backend().buffer(), 6),
        "▄██▄   14°  ↑19° ↓13° 2h ago"
    );
    assert_eq!(
        row_text(spanish_wide.backend().buffer(), 6),
        "▄██▄   14°  ↑19° ↓13° hace 2h"
    );
}

#[test]
fn compact_icon_keeps_its_column_when_the_weather_returns() {
    let mut harness = Harness::new(BUENOS_AIRES);
    harness.fetch(
        &default_instant(),
        Err(FetchError::Network("timed out".to_string())),
    );
    let offline = rendered(&harness, &default_instant(), 23, 8);
    let retry = default_instant().checked_add(1.minute()).unwrap();
    harness.fetch(&retry, Ok(forecast_body()));
    let fresh = rendered(&harness, &retry, 23, 8);

    assert_eq!(size_tier(&harness, &retry, 23, 8), Some(SizeTier::Compact));

    assert_eq!(offline.backend().buffer()[(0, 7)].symbol(), "▀");
    assert_eq!(fresh.backend().buffer()[(0, 7)].symbol(), "▀");
}

#[test]
fn medium_fits_forty_by_fifteen_with_the_default_config() {
    let harness = with_weather("");

    assert_eq!(
        minimum_size(&harness, &default_instant(), SizeTier::Medium),
        Some(Size::new(40, 15))
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 40, 15),
        Some(SizeTier::Medium)
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 39, 15),
        Some(SizeTier::Compact)
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 40, 14),
        Some(SizeTier::Compact)
    );
}

#[test]
fn twelve_hour_clock_widens_medium_by_three_columns() {
    let noon = default_instant().with().hour(12).build().unwrap();
    let harness = with_weather_at("hour_format = 12", &noon);

    assert_eq!(
        minimum_size(&harness, &noon, SizeTier::Medium),
        Some(Size::new(43, 15))
    );
}

#[test]
fn wide_fits_sixty_three_by_eleven_with_the_default_config() {
    let harness = with_weather("");
    let now = default_instant();

    assert_eq!(
        minimum_size(&harness, &now, SizeTier::Wide),
        Some(Size::new(63, 11))
    );
    assert_eq!(size_tier(&harness, &now, 63, 11), Some(SizeTier::Wide));
    assert_eq!(size_tier(&harness, &now, 62, 11), Some(SizeTier::Compact));
    assert_eq!(size_tier(&harness, &now, 63, 10), Some(SizeTier::Compact));
}

#[test]
fn tall_fits_forty_by_twenty_one_with_the_default_config() {
    let harness = with_weather("");
    let now = default_instant();

    assert_eq!(
        minimum_size(&harness, &now, SizeTier::Tall),
        Some(Size::new(40, 21))
    );
    assert_eq!(size_tier(&harness, &now, 40, 21), Some(SizeTier::Tall));
    assert_eq!(size_tier(&harness, &now, 40, 20), Some(SizeTier::Medium));
    assert_eq!(size_tier(&harness, &now, 39, 21), Some(SizeTier::Compact));
}

#[test]
fn wide_comes_before_tall_when_both_fit() {
    let harness = with_weather("");
    let now = default_instant();

    assert_eq!(size_tier(&harness, &now, 63, 21), Some(SizeTier::Wide));
    assert_eq!(size_tier(&harness, &now, 62, 21), Some(SizeTier::Tall));
}

#[test]
fn twelve_hour_clock_widens_wide_and_tall_by_three_columns() {
    let noon = default_instant().with().hour(12).build().unwrap();
    let harness = with_weather_at("hour_format = 12", &noon);

    assert_eq!(
        minimum_size(&harness, &noon, SizeTier::Wide),
        Some(Size::new(66, 11))
    );
    assert_eq!(
        minimum_size(&harness, &noon, SizeTier::Tall),
        Some(Size::new(43, 21))
    );
}

#[test]
fn wide_top_border_centres_the_place_name() {
    let harness = named_with_weather("", &default_instant());

    let terminal = rendered(&harness, &default_instant(), 63, 11);

    assert_eq!(
        row_text(terminal.backend().buffer(), 0),
        "╭─ tue 13 oct ─────────── buenos aires, ar ─────────── wk 42 ─╮"
    );
}

#[test]
fn wide_top_border_truncates_a_long_place_name() {
    let mut harness = Harness::new("location = \"llanfair\"");
    harness.fetch(&default_instant(), Ok(common::long_place_geocoding_body()));
    harness.weather_source.finish(Ok(forecast_body()));
    harness.app.tick(&default_instant());

    let terminal = rendered(&harness, &default_instant(), 63, 11);

    assert_eq!(
        row_text(terminal.backend().buffer(), 0),
        "╭─ tue 13 oct ─ llanfairpwllgwyngyllgogerychwyrndro… ─ wk 42 ─╮"
    );
}

#[test]
fn tall_top_border_leaves_the_place_name_out() {
    let harness = named_with_weather("", &default_instant());

    let terminal = rendered(&harness, &default_instant(), 40, 21);

    assert_eq!(
        row_text(terminal.backend().buffer(), 0),
        "╭─ tue 13 oct ───────────────── wk 42 ─╮"
    );
}

#[test]
fn tall_without_a_location_puts_the_first_run_hint_under_the_clock() {
    let harness = Harness::new("");

    let terminal = rendered(&harness, &default_instant(), 40, 21);

    let buffer = terminal.backend().buffer();
    assert_eq!(
        row_text(buffer, 5),
        "│ add location = \"your city\" to        │"
    );
    assert_eq!(
        row_text(buffer, 6),
        "│ ~/.config/alba/config.toml           │"
    );
}

/// Each weather state with the instant to show it at, named for its snapshot.
fn weather_state_cases() -> Vec<(&'static str, (Harness, Zoned))> {
    let offline = || {
        let mut harness = Harness::new(BUENOS_AIRES);
        harness.fetch(
            &default_instant(),
            Err(FetchError::Network("timed out".to_string())),
        );
        (harness, default_instant())
    };
    let fetching = || {
        let mut harness = Harness::new(BUENOS_AIRES);
        harness.app.tick(&default_instant());
        (harness, default_instant().checked_add(1.second()).unwrap())
    };
    let refreshing = |at: Zoned| {
        let mut harness = with_weather("");
        harness.app.key(KeyEvent::from(KeyCode::Char('r')));
        harness.app.tick(&at);
        (harness, at.checked_add(1.second()).unwrap())
    };
    let night = default_instant().with().hour(23).build().unwrap();
    vec![
        ("no_location", (Harness::new(""), default_instant())),
        ("fetching", fetching()),
        ("offline", offline()),
        (
            "stale",
            (
                with_weather(""),
                default_instant().checked_add(2.hours()).unwrap(),
            ),
        ),
        (
            "expired",
            (
                with_weather(""),
                default_instant().checked_add(13.hours()).unwrap(),
            ),
        ),
        ("night", (with_weather_at("", &night), night)),
        ("fetch_indicator", refreshing(default_instant())),
        (
            "stale_fetch_indicator",
            refreshing(default_instant().checked_add(2.hours()).unwrap()),
        ),
    ]
}

#[test]
fn medium_weather_states() {
    for (name, (harness, now)) in weather_state_cases() {
        assert_eq!(
            size_tier(&harness, &now, 40, 15),
            Some(SizeTier::Medium),
            "{name}"
        );
        insta::assert_snapshot!(
            format!("medium_{name}"),
            rendered(&harness, &now, 40, 15).backend()
        );
    }
}

#[test]
fn large_weather_states() {
    for (name, (harness, now)) in weather_state_cases() {
        assert_eq!(
            size_tier(&harness, &now, 85, 26),
            Some(SizeTier::Large),
            "{name}"
        );
        insta::assert_snapshot!(
            format!("large_{name}"),
            rendered(&harness, &now, 85, 26).backend()
        );
    }
}

fn named_with_weather(config_text: &str, now: &Zoned) -> Harness {
    let mut harness = Harness::new(&format!("location = \"buenos aires\"\n{config_text}"));
    harness.fetch(now, Ok(common::geocoding_body()));
    harness.weather_source.finish(Ok(forecast_body()));
    harness.app.tick(now);
    harness
}

#[test]
fn medium_top_border_leaves_the_place_name_out() {
    let harness = named_with_weather("", &default_instant());

    let terminal = rendered(&harness, &default_instant(), 40, 15);

    assert_eq!(
        row_text(terminal.backend().buffer(), 0),
        "╭─ tue 13 oct ───────────────── wk 42 ─╮"
    );
}

#[test]
fn medium_top_border_is_blue_date_and_dim_week_in_spanish_too() {
    let terminal = rendered(
        &with_weather("language = \"es\""),
        &default_instant(),
        40,
        15,
    );

    let buffer = terminal.backend().buffer();
    assert_eq!(
        row_text(buffer, 0),
        "╭─ mar 13 oct ──────────────── sem 42 ─╮"
    );
    assert_eq!(buffer[(3, 0)].fg, Color::LightBlue);
    assert_eq!(buffer[(31, 0)].fg, Color::DarkGray);
    assert_eq!(buffer[(0, 0)].fg, Color::DarkGray);
}

#[test]
fn medium_bottom_border_greets_by_the_hour_in_each_language() {
    let cases = [
        ("", "04:59", "╰» sleepy time ✦"),
        ("", "05:00", "╰» good morning ♥"),
        ("", "12:00", "╰» good afternoon ★"),
        ("", "18:00", "╰» good evening ★"),
        ("", "23:00", "╰» sleepy time ✦"),
        ("language = \"es\"", "05:00", "╰» buenos días ♥"),
        ("language = \"es\"", "19:59", "╰» buenas tardes ★"),
        ("language = \"es\"", "20:00", "╰» buenas noches ★"),
        ("language = \"es\"", "23:00", "╰» a mimir ✦"),
    ];

    for (config_text, time, greeting) in cases {
        let now: Zoned = format!("2026-10-13T{time}[America/Argentina/Buenos_Aires]")
            .parse()
            .unwrap();
        let terminal = rendered(&Harness::new(config_text), &now, 40, 15);

        let bottom = row_text(terminal.backend().buffer(), 14);
        assert!(bottom.starts_with(greeting), "{time}: {bottom}");
        assert_eq!(terminal.backend().buffer()[(3, 14)].fg, Color::Green);
    }
}

#[test]
fn medium_bottom_border_credits_open_meteo_only_with_weather() {
    let with = rendered(&with_weather(""), &default_instant(), 40, 15);
    let without = rendered(&Harness::new(BUENOS_AIRES), &default_instant(), 40, 15);

    assert!(row_text(with.backend().buffer(), 14).ends_with("── open-meteo ─╯"));
    assert!(row_text(without.backend().buffer(), 14).ends_with("─────╯"));
}

#[test]
fn stale_medium_keeps_the_icon_and_timeline_colors() {
    let later = default_instant().checked_add(2.hours()).unwrap();
    let fresh = rendered(&with_weather(""), &default_instant(), 40, 15);
    let stale = rendered(&with_weather(""), &later, 40, 15);

    let buffer = stale.backend().buffer();
    let fresh = fresh.backend().buffer();
    assert_eq!(buffer[(30, 3)], fresh[(30, 3)], "the icon keeps its colors");
    assert_eq!(
        buffer[(20, 8)].fg,
        fresh[(20, 8)].fg,
        "the timeline keeps its colors"
    );
}

#[test]
fn medium_without_a_match_says_location_not_found() {
    let mut harness = Harness::new("location = \"atlantis\"");
    harness.fetch(&default_instant(), Ok(NO_MATCH.to_string()));

    let terminal = rendered(&harness, &default_instant(), 40, 15);

    assert_eq!(
        row_text(terminal.backend().buffer(), 6),
        "│ location not found: atlantis         │"
    );
}

#[test]
fn polar_night_leaves_the_medium_timeline_rows_empty() {
    let now: Zoned = "2025-02-14T10:00[Arctic/Longyearbyen]".parse().unwrap();
    let body = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/forecast_longyearbyen_polar.json"
    ))
    .unwrap();
    let mut harness = Harness::new(BUENOS_AIRES);
    harness.fetch(&now, Ok(body));

    let terminal = rendered(&harness, &now, 40, 15);

    let buffer = terminal.backend().buffer();
    assert_eq!(size_tier(&harness, &now, 40, 15), Some(SizeTier::Medium));
    for y in 8..11 {
        assert_eq!(
            row_text(buffer, y),
            "│                                      │",
            "row {y}"
        );
    }
}

#[test]
fn medium_timeline_counts_down_to_sunset_with_minutes_under_an_hour() {
    let evening = default_instant()
        .with()
        .hour(18)
        .minute(30)
        .build()
        .unwrap();

    let terminal = rendered(&with_weather_at("", &evening), &evening, 40, 15);

    assert_eq!(
        row_text(terminal.backend().buffer(), 10),
        "│ ↑06:14     sunset in 0h36     19:06↓ │"
    );
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(2, 10)].fg, Color::Yellow, "the sunrise arrow");
    assert_eq!(buffer[(3, 10)].fg, Color::Reset, "the start time");
    assert_eq!(buffer[(13, 10)].fg, Color::Reset, "the status");
    assert_eq!(buffer[(32, 10)].fg, Color::Reset, "the end time");
}

#[test]
fn medium_timeline_bar_is_full_behind_the_marker_and_faint_ahead() {
    let midday = default_instant()
        .with()
        .hour(12)
        .minute(40)
        .build()
        .unwrap();

    let terminal = rendered(&with_weather_at("", &midday), &midday, 40, 15);

    let buffer = terminal.backend().buffer();
    let bar = |x: u16| {
        let cell = &buffer[(2 + x, 8)];
        (
            cell.symbol(),
            cell.fg,
            cell.modifier.contains(Modifier::DIM),
        )
    };
    assert_eq!(bar(0), ("▄", Color::LightRed, false), "dawn");
    assert_eq!(bar(4), ("▄", Color::Yellow, false), "behind the marker");
    assert_eq!(bar(12), ("▄", Color::LightYellow, false), "toward noon");
    assert_eq!(
        bar(24),
        ("▄", Color::LightYellow, true),
        "ahead of the marker"
    );
    assert_eq!(bar(35), ("▄", Color::LightRed, true), "dusk");
}

#[test]
fn medium_leaves_blank_rows_under_the_top_border_and_around_the_timeline() {
    let terminal = rendered(&with_weather(""), &default_instant(), 40, 15);

    let blank = "│                                      │";
    let buffer = terminal.backend().buffer();
    assert_eq!(row_text(buffer, 1), blank, "under the top border");
    assert_eq!(
        row_text(buffer, 7),
        blank,
        "between the sparkline and the timeline"
    );
}

#[test]
fn large_fits_seventy_one_by_twenty_three_with_the_default_config() {
    let harness = with_weather("");
    let now = default_instant();

    assert_eq!(
        minimum_size(&harness, &now, SizeTier::Large),
        Some(Size::new(71, 23))
    );
    assert_eq!(size_tier(&harness, &now, 71, 23), Some(SizeTier::Large));
    assert_eq!(size_tier(&harness, &now, 70, 23), Some(SizeTier::Wide));
    assert_eq!(size_tier(&harness, &now, 71, 22), Some(SizeTier::Wide));
}

#[test]
fn large_labels_the_sparkline_in_each_language() {
    let cases = [
        ("", 71, "precip · next 12h"),
        ("language = \"es\"", 72, "precip · próx. 12h"),
    ];

    for (config_text, width, label) in cases {
        let harness = with_weather(config_text);
        let now = default_instant();
        let terminal = rendered(&harness, &now, width, 23);

        let buffer = terminal.backend().buffer();
        assert_eq!(size_tier(&harness, &now, width, 23), Some(SizeTier::Large));
        assert!(
            row_text(buffer, 14).contains(label),
            "{}",
            row_text(buffer, 14)
        );
        assert_eq!(buffer[(29, 14)].fg, Color::DarkGray, "{label}");
    }
}
