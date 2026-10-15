mod common;

use alba::layout::{self, SizeTier};
use common::{BUENOS_AIRES, Harness, default_instant, forecast_body};
use jiff::{ToSpan, Zoned};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::{Rect, Size},
    style::Color,
};

fn with_weather(config_text: &str) -> Harness {
    with_weather_at(config_text, &default_instant())
}

fn with_weather_at(config_text: &str, now: &Zoned) -> Harness {
    let mut harness = Harness::new(&format!("{BUENOS_AIRES}\n{config_text}"));
    harness.fetch(now, Ok(forecast_body()));
    harness
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
fn without_weather_mini_does_not_fit_and_strip_takes_the_pane() {
    let harness = Harness::new(BUENOS_AIRES);

    assert_eq!(
        minimum_size(&harness, &default_instant(), SizeTier::Mini),
        None
    );
    assert_eq!(
        size_tier(&harness, &default_instant(), 40, 12),
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
