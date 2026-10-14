mod common;

use alba::weather::{Forecast, Freshness, Weather};
use common::{default_instant, forecast_body};
use jiff::{SignedDuration, Timestamp};

fn fetched_at() -> Timestamp {
    default_instant().timestamp()
}

fn weather() -> Weather {
    Weather {
        fetched_at: fetched_at(),
        forecast: Forecast::parse(&forecast_body()).unwrap(),
    }
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
    let mut weather = weather();
    let now = after(SignedDuration::from_mins(90));
    let hourly_times = &mut weather.forecast.hourly.time;
    hourly_times.retain(|&time| time <= now.as_second());

    assert_eq!(weather.freshness(now), Freshness::Expired);
    let one_hour_ahead = now.as_second() + 3600;
    weather.forecast.hourly.time.push(one_hour_ahead);
    weather.forecast.hourly.precipitation_probability.push(0);
    assert_eq!(
        weather.freshness(now),
        Freshness::Stale(SignedDuration::from_mins(90))
    );
}
