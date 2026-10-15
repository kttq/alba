mod common;

use alba::{
    clock::{Clock, DigitSize},
    config::Config,
    motion::Motion,
};
use common::pixel_snapshot;
use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::Modifier,
    widgets::Widget,
};

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

fn buffer(config_text: &str, time: &str, motion: Motion, digit_size: DigitSize) -> Buffer {
    let clock = Clock::new(&Config::parse(config_text).unwrap());
    let segment_clock = clock.segment_clock(&instant(time), motion, digit_size);
    let size = clock.segment_size(digit_size);
    let mut buffer = Buffer::empty(Rect::new(0, 0, size.width, size.height));
    segment_clock.render(buffer.area, &mut buffer);
    buffer
}

fn rendered(config_text: &str, time: &str, motion: Motion, digit_size: DigitSize) -> String {
    let buffer = buffer(config_text, time, motion, digit_size);
    pixel_snapshot(&buffer, buffer.area)
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
fn without_motion_the_colon_stays_lit_in_both_sizes() {
    let second_half = instant("06:38:05.5").timestamp();
    for digit_size in [DigitSize::Small, DigitSize::Large] {
        let still = rendered("", "06:38:05.5", Motion::at(second_half, false), digit_size);

        assert_eq!(still, rendered("", "06:38", colon_lit(), digit_size));
    }
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

/// The pixel snapshots show the mark's color but not its faint style.
#[test]
fn the_current_mark_is_lit_and_the_other_faint() {
    let faint_rows = |time: &str| {
        let buffer = buffer("hour_format = 12", time, colon_lit(), DigitSize::Small);
        let mark_column = buffer.area.width - 1;
        [0, 1].map(|row| buffer[(mark_column, row)].modifier.contains(Modifier::DIM))
    };

    assert_eq!(faint_rows("06:38"), [false, true]);
    assert_eq!(faint_rows("18:38"), [true, false]);
}

#[test]
fn segment_size_is_the_drawn_clock_extent() {
    for config_text in [
        "",
        "hour_format = 12",
        "hour_format = 12\nlanguage = \"es\"",
    ] {
        let clock = Clock::new(&Config::parse(config_text).unwrap());
        for digit_size in [DigitSize::Small, DigitSize::Large] {
            let drawn = clock.segment_clock(&instant("18:38"), colon_lit(), digit_size);
            let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 20));
            (&drawn).render(buffer.area, &mut buffer);

            assert_eq!(
                drawn_extent(&buffer),
                clock.segment_size(digit_size),
                "{config_text:?}, {digit_size:?}"
            );
        }
    }
}

/// From the top-left corner to the last column and row with a glyph.
fn drawn_extent(buffer: &Buffer) -> Size {
    let drawn = |x, y| buffer[(x, y)].symbol() != " ";
    let area = buffer.area;
    let width = (0..area.width)
        .filter(|&x| (0..area.height).any(|y| drawn(x, y)))
        .max()
        .map_or(0, |x| x + 1);
    let height = (0..area.height)
        .filter(|&y| (0..area.width).any(|x| drawn(x, y)))
        .max()
        .map_or(0, |y| y + 1);
    Size::new(width, height)
}
