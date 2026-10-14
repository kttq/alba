mod common;

use alba::{
    sky::{MoonPhase, MoonPhaseName, Sky, Timeline},
    strings::Strings,
    weather::{DayOrNight, Forecast},
};
use common::{MIDNIGHT, edited_fixture_forecast, fixture_forecast, small_forecast};
use jiff::Timestamp;
use serde_json::json;

const HOUR: i64 = 3600;
const DAY: i64 = 24 * HOUR;

fn at(second: i64) -> Timestamp {
    Timestamp::from_second(second).unwrap()
}

fn utc(text: &str) -> Timestamp {
    text.parse().unwrap()
}

/// Days 12 to 14 October at UTC, each with sunrise 06:00 and sunset 18:00.
fn three_days() -> Forecast {
    small_forecast(|forecast| {
        let midnights = [MIDNIGHT - DAY, MIDNIGHT, MIDNIGHT + DAY];
        forecast["daily"] = json!({
            "time": midnights,
            "sunrise": midnights.map(|midnight| midnight + 6 * HOUR),
            "sunset": midnights.map(|midnight| midnight + 18 * HOUR),
            "temperature_2m_max": [0.0, 0.0, 0.0],
            "temperature_2m_min": [0.0, 0.0, 0.0],
        });
    })
}

fn day_or_night_and_timeline(sky: Sky) -> (DayOrNight, Option<(Timestamp, Timestamp)>) {
    let timeline = sky.timeline.map(|Timeline { start, end }| (start, end));
    (sky.day_or_night, timeline)
}

#[test]
fn day_runs_from_the_last_sunrise_to_the_next_sunset() {
    let sky = Sky::at(&three_days(), at(MIDNIGHT + 10 * HOUR));

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
    let before_midnight = Sky::at(&three_days(), at(MIDNIGHT + 23 * HOUR));
    let after_midnight = Sky::at(&three_days(), at(MIDNIGHT + DAY + 2 * HOUR));

    assert_eq!(day_or_night_and_timeline(before_midnight), tonight);
    assert_eq!(day_or_night_and_timeline(after_midnight), tonight);
}

#[test]
fn an_event_exactly_at_now_has_passed() {
    let sky = Sky::at(&three_days(), at(MIDNIGHT + 18 * HOUR));

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
    (moon_phase.name, moon_phase.illumination_percent())
}

/// Published phases of October 2025. The mean synodic month puts the quarters off by up to a
/// day, so only their names are checked.
#[test]
fn moon_phase_matches_the_published_phases() {
    assert_eq!(moon_at("2025-10-07T03:47Z"), (MoonPhaseName::FullMoon, 100));
    assert_eq!(moon_at("2025-10-21T12:25Z"), (MoonPhaseName::NewMoon, 0));
    assert_eq!(moon_at("2025-10-13T18:13Z").0, MoonPhaseName::LastQuarter);
    assert_eq!(moon_at("2025-10-29T16:21Z").0, MoonPhaseName::FirstQuarter);
}

#[test]
fn each_phase_name_covers_an_eighth_of_the_cycle_centred_on_its_point() {
    let reference_new_moon = utc("2000-01-06T18:14Z");
    let sixteenth = jiff::SignedDuration::from_secs_f64(29.530589 * 86400.0 / 16.0);
    let minute = jiff::SignedDuration::from_mins(1);
    let names = [
        MoonPhaseName::NewMoon,
        MoonPhaseName::WaxingCrescent,
        MoonPhaseName::FirstQuarter,
        MoonPhaseName::WaxingGibbous,
        MoonPhaseName::FullMoon,
        MoonPhaseName::WaningGibbous,
        MoonPhaseName::LastQuarter,
        MoonPhaseName::WaningCrescent,
    ];

    for (index, &name) in names.iter().enumerate() {
        let point = reference_new_moon + sixteenth * (2 * index as i32);
        let first = point - sixteenth + minute;
        let last = point + sixteenth - minute;
        for now in [first, point, last] {
            assert_eq!(MoonPhase::at(now).name, name, "{now}");
        }
    }
    assert_eq!(
        MoonPhase::at(reference_new_moon - minute).name,
        MoonPhaseName::NewMoon
    );
}

#[test]
fn moon_phase_names_exist_in_both_languages() {
    let (english, spanish) = (&Strings::EN, &Strings::ES);

    assert_eq!(
        english.moon_phase(MoonPhaseName::WaxingGibbous),
        "waxing gibbous"
    );
    assert_eq!(
        spanish.moon_phase(MoonPhaseName::WaxingGibbous),
        "gibosa creciente"
    );
    assert_eq!(
        english.moon_phase(MoonPhaseName::WaningCrescent),
        "waning crescent"
    );
    assert_eq!(
        spanish.moon_phase(MoonPhaseName::WaningCrescent),
        "luna menguante"
    );
}
