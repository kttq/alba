mod common;

use alba::{
    sky::{MoonPhase, MoonPhaseName, Sky, Timeline},
    strings::Strings,
    weather::{DayOrNight, Forecast},
};
use common::{
    DAY, HOUR, MIDNIGHT, at, edited_fixture_forecast, fixture_forecast, small_forecast,
    three_day_forecast,
};
use jiff::Timestamp;
use serde_json::json;

fn utc(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn day_or_night_and_timeline(sky: Sky) -> (DayOrNight, Option<(Timestamp, Timestamp)>) {
    let timeline = sky.timeline.map(|Timeline { start, end }| (start, end));
    (sky.day_or_night, timeline)
}

#[test]
fn day_runs_from_the_last_sunrise_to_the_next_sunset() {
    let sky = Sky::at(&three_day_forecast(), at(MIDNIGHT + 10 * HOUR));

    assert_eq!(
        day_or_night_and_timeline(sky),
        (
            DayOrNight::Day,
            Some((at(MIDNIGHT + 6 * HOUR), at(MIDNIGHT + 18 * HOUR)))
        )
    );
}

#[test]
fn night_runs_from_the_last_sunset_to_the_next_sunrise() {
    let tonight = (
        DayOrNight::Night,
        Some((at(MIDNIGHT + 18 * HOUR), at(MIDNIGHT + DAY + 6 * HOUR))),
    );
    let before_midnight = Sky::at(&three_day_forecast(), at(MIDNIGHT + 23 * HOUR));
    let after_midnight = Sky::at(&three_day_forecast(), at(MIDNIGHT + DAY + 2 * HOUR));

    assert_eq!(day_or_night_and_timeline(before_midnight), tonight);
    assert_eq!(day_or_night_and_timeline(after_midnight), tonight);
}

#[test]
fn an_event_exactly_at_now_has_passed() {
    let sky = Sky::at(&three_day_forecast(), at(MIDNIGHT + 18 * HOUR));

    assert_eq!(
        day_or_night_and_timeline(sky),
        (
            DayOrNight::Night,
            Some((at(MIDNIGHT + 18 * HOUR), at(MIDNIGHT + DAY + 6 * HOUR)))
        )
    );
}

#[test]
fn is_day_decides_when_no_event_pair_surrounds_now() {
    let forecast = small_forecast(|forecast| forecast["current"]["is_day"] = json!(0));
    let before_sunrise = Sky::at(&forecast, at(MIDNIGHT + HOUR));
    let after_sunset = Sky::at(&forecast, at(MIDNIGHT + 20 * HOUR));

    assert_eq!(
        day_or_night_and_timeline(before_sunrise),
        (DayOrNight::Night, None)
    );
    assert_eq!(
        day_or_night_and_timeline(after_sunset),
        (DayOrNight::Night, None)
    );
}

/// Longyearbyen, 14 to 17 February 2025: polar night on the 14th, then the first sunrise.
fn polar_with_is_day(is_day: u8) -> Forecast {
    edited_fixture_forecast("forecast_longyearbyen_polar.json", |forecast| {
        forecast["current"]["is_day"] = json!(is_day);
    })
}

#[test]
fn polar_sentinel_times_are_never_sunrise_or_sunset() {
    let polar_night = utc("2025-02-14T10:00Z");
    let morning_before_the_first_sunrise = utc("2025-02-15T09:00Z");

    for now in [polar_night, morning_before_the_first_sunrise] {
        for (is_day, day_or_night) in [(0, DayOrNight::Night), (1, DayOrNight::Day)] {
            let sky = Sky::at(&polar_with_is_day(is_day), now);
            assert_eq!(
                day_or_night_and_timeline(sky),
                (day_or_night, None),
                "{now}"
            );
        }
    }
}

#[test]
fn the_first_day_after_polar_night_has_a_timeline() {
    let sky = Sky::at(
        &fixture_forecast("forecast_longyearbyen_polar.json"),
        utc("2025-02-15T11:00Z"),
    );

    assert_eq!(
        day_or_night_and_timeline(sky),
        (
            DayOrNight::Day,
            Some((utc("2025-02-15T10:38:34Z"), utc("2025-02-15T11:46:05Z")))
        )
    );
}

#[test]
fn madrid_night_crosses_the_end_of_daylight_saving() {
    let sky = Sky::at(
        &fixture_forecast("forecast_madrid_dst.json"),
        utc("2025-10-25T23:00Z"),
    );

    assert_eq!(
        day_or_night_and_timeline(sky),
        (
            DayOrNight::Night,
            Some((at(1_761_412_830), at(1_761_460_686)))
        )
    );
}

fn moon_at(text: &str) -> (MoonPhaseName, u8) {
    let moon_phase = MoonPhase::at(utc(text));
    (moon_phase.name(), moon_phase.illumination_percent())
}

/// Published phases of October 2025, in UTC.
const FULL_MOON: &str = "2025-10-07T03:47Z";
const LAST_QUARTER: &str = "2025-10-13T18:13Z";
const NEW_MOON: &str = "2025-10-21T12:25Z";
const FIRST_QUARTER: &str = "2025-10-29T16:21Z";

#[test]
fn moon_phase_matches_the_published_phases() {
    assert_eq!(moon_at(FULL_MOON), (MoonPhaseName::FullMoon, 100));
    assert_eq!(moon_at(LAST_QUARTER), (MoonPhaseName::LastQuarter, 50));
    assert_eq!(moon_at(NEW_MOON), (MoonPhaseName::NewMoon, 0));
    assert_eq!(moon_at(FIRST_QUARTER), (MoonPhaseName::FirstQuarter, 50));
}

/// Each name covers an eighth of the cycle, about 3.7 days, centred on its point.
#[test]
fn each_phase_name_covers_the_days_around_its_point() {
    let a_day_and_a_half = jiff::SignedDuration::from_hours(36);
    let points = [
        (FULL_MOON, MoonPhaseName::FullMoon),
        (LAST_QUARTER, MoonPhaseName::LastQuarter),
        (NEW_MOON, MoonPhaseName::NewMoon),
        (FIRST_QUARTER, MoonPhaseName::FirstQuarter),
    ];
    for (point, name) in points {
        for now in [utc(point) - a_day_and_a_half, utc(point) + a_day_and_a_half] {
            assert_eq!(MoonPhase::at(now).name(), name, "{now}");
        }
    }

    let between_the_points = [
        ("2025-10-10T11:00Z", MoonPhaseName::WaningGibbous),
        ("2025-10-17T15:00Z", MoonPhaseName::WaningCrescent),
        ("2025-10-25T14:00Z", MoonPhaseName::WaxingCrescent),
        ("2025-11-02T15:00Z", MoonPhaseName::WaxingGibbous),
    ];
    for (now, name) in between_the_points {
        assert_eq!(moon_at(now).0, name, "{now}");
    }
}

#[test]
fn moon_phase_names_exist_in_both_languages() {
    let table = [
        (MoonPhaseName::NewMoon, "new moon", "luna nueva"),
        (
            MoonPhaseName::WaxingCrescent,
            "waxing crescent",
            "luna creciente",
        ),
        (
            MoonPhaseName::FirstQuarter,
            "first quarter",
            "cuarto creciente",
        ),
        (
            MoonPhaseName::WaxingGibbous,
            "waxing gibbous",
            "gibosa creciente",
        ),
        (MoonPhaseName::FullMoon, "full moon", "luna llena"),
        (
            MoonPhaseName::WaningGibbous,
            "waning gibbous",
            "gibosa menguante",
        ),
        (
            MoonPhaseName::LastQuarter,
            "last quarter",
            "cuarto menguante",
        ),
        (
            MoonPhaseName::WaningCrescent,
            "waning crescent",
            "luna menguante",
        ),
    ];

    for (name, english, spanish) in table {
        assert_eq!(Strings::EN.moon_phase(name), english);
        assert_eq!(Strings::ES.moon_phase(name), spanish);
    }
}

#[test]
fn no_timeline_spans_a_polar_sentinel_day() {
    let forecast = small_forecast(|forecast| {
        let midnights = [MIDNIGHT - DAY, MIDNIGHT, MIDNIGHT + DAY];
        forecast["current"]["is_day"] = json!(0);
        forecast["daily"] = json!({
            "time": midnights,
            "sunrise": [MIDNIGHT - DAY + 11 * HOUR, MIDNIGHT, MIDNIGHT + DAY + 11 * HOUR],
            "sunset": [MIDNIGHT - DAY + 13 * HOUR, MIDNIGHT, MIDNIGHT + DAY + 13 * HOUR],
            "temperature_2m_max": [0.0, 0.0, 0.0],
            "temperature_2m_min": [0.0, 0.0, 0.0],
        });
    });

    for now in [
        MIDNIGHT - 2 * HOUR,
        MIDNIGHT + 12 * HOUR,
        MIDNIGHT + DAY + 2 * HOUR,
    ] {
        let sky = Sky::at(&forecast, at(now));
        assert_eq!(
            day_or_night_and_timeline(sky),
            (DayOrNight::Night, None),
            "{}",
            at(now)
        );
    }
}
