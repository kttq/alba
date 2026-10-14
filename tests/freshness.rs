mod common;

use alba::weather::{Forecast, Freshness, Weather};
use common::{default_instant, forecast_body};
use jiff::{SignedDuration, Timestamp};
use serde_json::{Value, json};

fn fetched_at() -> Timestamp {
    default_instant().timestamp()
}

fn weather() -> Weather {
    weather_from(&forecast_body())
}

fn weather_from(forecast_body: &str) -> Weather {
    Weather {
        fetched_at: fetched_at(),
        forecast: Forecast::parse(forecast_body).unwrap(),
    }
}

/// The fixture with one 0% hourly value at each of `times`.
fn weather_with_hourly_times(times: &[i64]) -> Weather {
    let mut forecast: Value = serde_json::from_str(&forecast_body()).unwrap();
    forecast["hourly"] = json!({
        "time": times,
        "precipitation_probability": vec![0; times.len()],
    });
    weather_from(&forecast.to_string())
}

fn after(age: SignedDuration) -> Timestamp {
    fetched_at() + age
}

#[test]
fn weather_up_to_60_minutes_old_is_fresh() {
    for minutes in [0, 30, 60] {
        assert_eq!(
            weather().freshness(after(SignedDuration::from_mins(minutes))),
            Freshness::Fresh,
            "{minutes} minutes"
        );
    }
}

#[test]
fn weather_older_than_60_minutes_is_stale_with_its_age() {
    for age in [
        SignedDuration::from_secs(60 * 60 + 1),
        SignedDuration::from_mins(150),
        SignedDuration::from_hours(12),
    ] {
        assert_eq!(
            weather().freshness(after(age)),
            Freshness::Stale(age),
            "{age:?}"
        );
    }
}

#[test]
fn weather_older_than_12_hours_is_expired() {
    let age = SignedDuration::from_hours(12) + SignedDuration::from_secs(1);

    assert_eq!(weather().freshness(after(age)), Freshness::Expired);
}

#[test]
fn weather_whose_sparkline_window_is_entirely_past_is_expired() {
    let now = after(SignedDuration::from_mins(90));
    let now_second = now.as_second();

    let past = weather_with_hourly_times(&[now_second - 3600, now_second]);
    assert_eq!(past.freshness(now), Freshness::Expired);

    let one_ahead = weather_with_hourly_times(&[now_second, now_second + 3600]);
    assert_eq!(
        one_ahead.freshness(now),
        Freshness::Stale(SignedDuration::from_mins(90))
    );
}
