mod common;

use alba::{
    bezel::{Bezel, BorderRun},
    calendar::Calendar,
    config::Config,
    layout::SizeTier,
    strings::Strings,
};
use common::default_instant;
use jiff::Zoned;
use ratatui::{buffer::Buffer, layout::Rect, style::Color, text::Line, widgets::Widget};

fn bezel(config_text: &str, now: &Zoned, place_name: Option<&str>, shows_credit: bool) -> Bezel {
    let config = Config::parse(config_text).unwrap();
    let calendar = Calendar::new(&config).at(now);
    Bezel::new(
        Strings::of(config.language),
        &calendar,
        now,
        place_name,
        shows_credit,
    )
}

fn text(line: &Line) -> String {
    line.to_string()
}

fn instant(time: &str) -> Zoned {
    format!("2026-10-13T{time}[America/Argentina/Buenos_Aires]")
        .parse()
        .unwrap()
}

#[test]
fn top_labels_are_the_short_date_and_the_iso_week_in_each_language() {
    let english = bezel("", &default_instant(), None, false);
    let spanish = bezel("language = \"es\"", &default_instant(), None, false);

    assert_eq!(text(&english.top_left), "╭─ tue 13 oct ");
    assert_eq!(text(&english.top_right), " wk 42 ─╮");
    assert_eq!(text(&spanish.top_left), "╭─ mar 13 oct ");
    assert_eq!(text(&spanish.top_right), " sem 42 ─╮");
}

#[test]
fn bottom_left_greets_by_the_hour_in_each_language() {
    let cases = [
        ("", "00:00", "╰» sleepy time ✦ "),
        ("", "04:59", "╰» sleepy time ✦ "),
        ("", "05:00", "╰» good morning ♥ "),
        ("", "11:59", "╰» good morning ♥ "),
        ("", "12:00", "╰» good afternoon ★ "),
        ("", "18:00", "╰» good evening ★ "),
        ("", "23:00", "╰» sleepy time ✦ "),
        ("language = \"es\"", "05:00", "╰» buenos días ♥ "),
        ("language = \"es\"", "19:59", "╰» buenas tardes ★ "),
        ("language = \"es\"", "20:00", "╰» buenas noches ★ "),
        ("language = \"es\"", "23:00", "╰» a mimir ✦ "),
    ];

    for (config_text, time, greeting) in cases {
        let bezel = bezel(config_text, &instant(time), None, false);

        assert_eq!(text(&bezel.bottom_left), greeting, "{config_text:?} {time}");
        assert_eq!(bezel.bottom_left.spans[1].style.fg, Some(Color::Green));
    }
}

#[test]
fn bottom_right_credits_open_meteo_only_with_weather() {
    let with = bezel("", &default_instant(), None, true);
    let without = bezel("", &default_instant(), None, false);

    assert_eq!(text(&with.bottom_right), " open-meteo ─╯");
    assert_eq!(text(&without.bottom_right), "─╯");
}

#[test]
fn place_name_shows_only_in_large_and_wide() {
    let bezel = bezel("", &default_instant(), Some("Buenos Aires, AR"), true);

    for size_tier in SizeTier::IN_ORDER {
        let expected = matches!(size_tier, SizeTier::Large | SizeTier::Wide);
        assert_eq!(
            bezel.place_name(size_tier).is_some(),
            expected,
            "{size_tier:?}"
        );
    }
}

fn border_run(place_name: Option<&str>, width: u16) -> String {
    let area = Rect::new(0, 0, width, 1);
    let mut buffer = Buffer::empty(area);
    BorderRun { place_name }.render(area, &mut buffer);
    (0..width).map(|x| buffer[(x, 0)].symbol()).collect()
}

#[test]
fn border_run_centres_the_place_name_and_shortens_it_to_fit() {
    assert_eq!(border_run(None, 8), "────────");
    assert_eq!(border_run(Some("Lima"), 12), "─── Lima ───");
    assert_eq!(border_run(Some("Buenos Aires"), 13), "─ Buenos A… ─");
}

#[test]
fn border_run_drops_a_place_name_without_room_for_six_columns() {
    assert_eq!(border_run(Some("Buenos Aires"), 9), "─────────");
    assert_eq!(border_run(Some("Buenos Aires"), 10), "─ Bueno… ─");
}
