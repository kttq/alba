use std::time::Duration;

use alba::motion::{self, Motion};
use jiff::Timestamp;

/// 2026-10-13 06:38 in Buenos Aires, on an eight-second boundary.
fn at(time: &str) -> Timestamp {
    format!("2026-10-13T{time}-03:00").parse().unwrap()
}

#[test]
fn the_tick_is_a_quarter_second_with_motion_and_a_second_without() {
    assert_eq!(motion::tick_interval(true), Duration::from_millis(250));
    assert_eq!(motion::tick_interval(false), Duration::from_secs(1));
}

#[test]
fn the_colon_is_lit_for_the_first_two_ticks_of_each_second() {
    let colon_lit = |time: &str| Motion::at(at(time), true).colon_lit;

    assert!(colon_lit("06:38:05"));
    assert!(colon_lit("06:38:05.499"));
    assert!(!colon_lit("06:38:05.5"));
    assert!(!colon_lit("06:38:05.999"));
    assert!(colon_lit("06:38:06"));
}

#[test]
fn the_icon_frame_changes_each_second() {
    let icon_frame = |time: &str| Motion::at(at(time), true).icon_frame;

    assert_eq!(icon_frame("06:38:00"), 0);
    assert_eq!(icon_frame("06:38:00.999"), 0);
    assert_eq!(icon_frame("06:38:01"), 1);
    assert_eq!(icon_frame("06:38:01.999"), 1);
    assert_eq!(icon_frame("06:38:02"), 0);
}

#[test]
fn the_flash_is_the_first_tick_of_each_eight_seconds() {
    let flash = |time: &str| Motion::at(at(time), true).flash;

    assert!(flash("06:38:00"));
    assert!(flash("06:38:00.249"));
    assert!(!flash("06:38:00.25"));
    assert!(!flash("06:38:04"));
    assert!(!flash("06:38:07.999"));
    assert!(flash("06:38:08"));
}

#[test]
fn phases_come_from_the_unix_time_not_the_local_day() {
    let utc: Timestamp = "2026-10-13T09:38:08Z".parse().unwrap();

    assert_eq!(Motion::at(utc, true), Motion::at(at("06:38:08"), true));
    assert!(Motion::at(Timestamp::UNIX_EPOCH, true).flash);
}

#[test]
fn without_motion_the_colon_is_lit_on_frame_one_with_no_flash() {
    let still = Motion {
        colon_lit: true,
        icon_frame: 0,
        flash: false,
    };

    for time in ["06:38:00", "06:38:01", "06:38:05.5", "06:38:08.1"] {
        assert_eq!(Motion::at(at(time), false), still, "{time}");
    }
}
