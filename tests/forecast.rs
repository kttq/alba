mod common;

use alba::weather::{Forecast, SunEvent};
use common::{
    DAY, HOUR, MIDNIGHT, at, fixture_forecast, small_forecast as forecast, three_day_forecast,
};
use serde_json::json;

/// Hourly values 0, 1, 2, … at each hour from midnight.
fn forecast_with_hours(hours: i64) -> Forecast {
    forecast(|forecast| {
        forecast["hourly"] = json!({
            "time": (0..hours).map(|hour| MIDNIGHT + hour * HOUR).collect::<Vec<_>>(),
            "precipitation_probability": (0..hours).collect::<Vec<_>>(),
        });
    })
}

#[test]
fn precipitation_is_the_next_12_hourly_values_strictly_after_now() {
    let forecast = forecast_with_hours(24);

    assert_eq!(
        forecast.precipitation_after(at(MIDNIGHT + 2 * HOUR)),
        [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
    );
    assert_eq!(
        forecast.precipitation_after(at(MIDNIGHT + 2 * HOUR + 1)),
        [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
    );
}

#[test]
fn precipitation_near_the_end_of_the_forecast_has_fewer_values() {
    let forecast = forecast_with_hours(24);

    assert_eq!(
        forecast.precipitation_after(at(MIDNIGHT + 20 * HOUR)),
        [21, 22, 23]
    );
    assert_eq!(
        forecast.precipitation_after(at(MIDNIGHT + 23 * HOUR)),
        [] as [u8; 0]
    );
}

#[test]
fn high_and_low_come_from_the_forecast_day_row() {
    let forecast = three_day_forecast();

    let cases = [
        (MIDNIGHT - 1, Some((21.0, 11.0))),
        (MIDNIGHT, Some((22.0, 12.0))),
        (MIDNIGHT + 12 * HOUR, Some((22.0, 12.0))),
        (MIDNIGHT + 2 * DAY - 1, Some((23.0, 13.0))),
        (MIDNIGHT + 2 * DAY, None),
        (MIDNIGHT - DAY - 1, None),
    ];
    for (now, expected) in cases {
        let high_and_low = forecast
            .high_and_low(at(now))
            .map(|high_and_low| (high_and_low.high, high_and_low.low));
        assert_eq!(high_and_low, expected, "{}", at(now));
    }
}

/// Open-Meteo keeps the summer offset, +2h, for every row, so the row for 27 October starts at
/// 23:00 on the 26th in Madrid's real time, after daylight saving ended.
#[test]
fn high_and_low_follow_the_response_offset_on_a_daylight_saving_day() {
    let forecast = fixture_forecast("forecast_madrid_dst.json");
    let late_on_the_26th: jiff::Zoned = "2025-10-26T23:30[Europe/Madrid]".parse().unwrap();
    let evening_of_the_26th: jiff::Zoned = "2025-10-26T22:30[Europe/Madrid]".parse().unwrap();

    let high_and_low = forecast.high_and_low(late_on_the_26th.timestamp()).unwrap();
    assert_eq!((high_and_low.high, high_and_low.low), (19.3, 3.9));
    let high_and_low = forecast
        .high_and_low(evening_of_the_26th.timestamp())
        .unwrap();
    assert_eq!((high_and_low.high, high_and_low.low), (18.1, 10.7));
}

#[test]
fn a_real_sunset_after_the_next_midnight_is_kept() {
    let forecast = forecast(|forecast| {
        forecast["daily"]["sunrise"] = json!([MIDNIGHT + 2 * HOUR]);
        forecast["daily"]["sunset"] = json!([MIDNIGHT + DAY + 16 * 60]);
    });

    assert_eq!(
        forecast.sun_events(),
        [
            SunEvent::Sunrise(at(MIDNIGHT + 2 * HOUR)),
            SunEvent::Sunset(at(MIDNIGHT + DAY + 16 * 60)),
        ]
    );
}

#[test]
fn both_polar_sentinel_forms_are_a_polar_marker() {
    let polar_night = [MIDNIGHT, MIDNIGHT];
    let polar_day = [MIDNIGHT, MIDNIGHT + DAY];

    for [sunrise, sunset] in [polar_night, polar_day] {
        let forecast = forecast(|forecast| {
            forecast["daily"]["sunrise"] = json!([sunrise]);
            forecast["daily"]["sunset"] = json!([sunset]);
        });
        assert_eq!(
            forecast.sun_events(),
            [SunEvent::Polar(at(MIDNIGHT))],
            "sunset {}",
            at(sunset)
        );
    }
}

fn parses_with(edit: impl FnOnce(&mut serde_json::Value)) -> bool {
    let mut body: serde_json::Value = serde_json::from_str(&common::forecast_body()).unwrap();
    edit(&mut body);
    Forecast::parse(&body.to_string()).is_ok()
}

#[test]
fn times_out_of_range_or_order_fail_the_response() {
    assert!(parses_with(|_| {}));
    assert!(!parses_with(
        |body| body["daily"]["sunrise"][0] = json!(i64::MAX)
    ));
    assert!(!parses_with(
        |body| body["daily"]["time"][2] = json!(i64::MAX)
    ));
    assert!(!parses_with(|body| body["hourly"]["time"][1] = json!(0)));
}
