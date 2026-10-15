mod common;

use alba::{
    clock::{Clock, DigitSize},
    config::Config,
    motion::Motion,
};
use common::pixel_snapshot;
use jiff::Zoned;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

fn instant(time: &str) -> Zoned {
    format!("2026-10-13T{time}[America/Argentina/Buenos_Aires]")
        .parse()
        .unwrap()
}

fn colon_lit() -> Motion {
    Motion {
        colon_lit: true,
        icon_frame: 0,
        flash: false,
    }
}

fn rendered(config_text: &str, time: &str, motion: Motion, digit_size: DigitSize) -> String {
    let clock = Clock::new(&Config::parse(config_text).unwrap());
    let segment_clock = clock.segment_clock(&instant(time), motion, digit_size);
    let size = segment_clock.size();
    let area = Rect::new(0, 0, size.width, size.height);
    let mut buffer = Buffer::empty(area);
    segment_clock.render(area, &mut buffer);
    pixel_snapshot(&buffer, area)
}

#[test]
fn small_clock_with_the_colon_lit() {
    insta::assert_snapshot!(rendered("", "06:38", colon_lit(), DigitSize::Small));
}

#[test]
fn large_clock_with_the_colon_lit() {
    insta::assert_snapshot!(rendered("", "06:38", colon_lit(), DigitSize::Large));
}

#[test]
fn the_colon_blinks_to_a_ghost_in_both_sizes() {
    let colon_ghost = Motion {
        colon_lit: false,
        ..colon_lit()
    };
    insta::assert_snapshot!(
        "small_colon_ghost",
        rendered("", "06:38", colon_ghost, DigitSize::Small)
    );
    insta::assert_snapshot!(
        "large_colon_ghost",
        rendered("", "06:38", colon_ghost, DigitSize::Large)
    );
}

#[test]
fn without_motion_the_colon_stays_lit() {
    let second_half = instant("06:38:05.5").timestamp();
    let still = rendered(
        "",
        "06:38:05.5",
        Motion::at(second_half, false),
        DigitSize::Small,
    );

    assert_eq!(still, rendered("", "06:38", colon_lit(), DigitSize::Small));
}

#[test]
fn every_digit_in_both_sizes() {
    for (name, digit_size) in [("small", DigitSize::Small), ("large", DigitSize::Large)] {
        let sheet: String = ["06:38", "19:47", "12:25"]
            .into_iter()
            .map(|time| rendered("", time, colon_lit(), digit_size))
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(format!("every_digit_{name}"), sheet);
    }
}

#[test]
fn without_ghost_segments_unlit_segments_and_the_colon_are_empty() {
    let colon_ghost = Motion {
        colon_lit: false,
        ..colon_lit()
    };
    insta::assert_snapshot!(rendered(
        "ghost_segments = false",
        "06:38",
        colon_ghost,
        DigitSize::Small
    ));
}

#[test]
fn twelve_hour_clock_with_its_mark_in_both_sizes() {
    for (name, digit_size) in [("small", DigitSize::Small), ("large", DigitSize::Large)] {
        let sheet = format!(
            "{}\n{}",
            rendered("hour_format = 12", "06:38", colon_lit(), digit_size),
            rendered("hour_format = 12", "18:38", colon_lit(), digit_size)
        );
        insta::assert_snapshot!(format!("twelve_hour_{name}"), sheet);
    }
}
