mod common;

use alba::weather::{FetchError, WeatherState};
use common::{StationHarness, default_instant, forecast_body, temporary_directory};
use jiff::{SignedDuration, Zoned};

fn at(seconds: i64) -> Zoned {
    default_instant()
        .checked_add(SignedDuration::from_secs(seconds))
        .unwrap()
}

fn minutes(minutes: i64) -> i64 {
    minutes * 60
}

fn failure() -> Result<String, FetchError> {
    Err(FetchError::Network("timed out".to_string()))
}

/// Asserts that the next fetch starts exactly at `seconds`, not one second earlier.
fn assert_next_fetch_at(station: &mut StationHarness, seconds: i64) {
    let fetches = station.fetches();
    station.tick(&at(seconds - 1));
    assert_eq!(station.fetches(), fetches, "fetch before {seconds} s");
    station.tick(&at(seconds));
    assert_eq!(station.fetches(), fetches + 1, "no fetch at {seconds} s");
}

#[test]
fn offline_start_retries_at_1_2_5_then_10_minutes_until_weather_arrives() {
    let mut station = StationHarness::with_cache(temporary_directory());

    assert_eq!(
        station.weather_station.weather_state(&at(0)),
        WeatherState::Fetching
    );
    station.tick(&at(0));
    assert_eq!(station.fetches(), 1);
    station.finish(&at(0), failure());
    assert_eq!(
        station.weather_station.weather_state(&at(0)),
        WeatherState::Offline
    );

    let mut failed_at = 0;
    for wait in [1, 2, 5, 10, 10] {
        assert_next_fetch_at(&mut station, failed_at + minutes(wait));
        failed_at += minutes(wait);
        assert_eq!(
            station.weather_station.weather_state(&at(failed_at)),
            WeatherState::Offline,
            "offline while retrying at {failed_at} s"
        );
        station.finish(&at(failed_at), failure());
    }

    assert_next_fetch_at(&mut station, failed_at + minutes(10));
    let fetched_at = failed_at + minutes(10);
    station.finish(&at(fetched_at), Ok(forecast_body()));
    assert!(matches!(
        station.weather_station.weather_state(&at(fetched_at)),
        WeatherState::Fresh(_)
    ));
    assert_next_fetch_at(&mut station, fetched_at + minutes(30));
}

#[test]
fn failed_refresh_of_old_cached_weather_keeps_it_and_waits_1_minute() {
    let cache_directory = temporary_directory();
    let mut first_run = StationHarness::with_cache(cache_directory.clone());
    first_run.tick(&at(-minutes(45)));
    first_run.finish(&at(-minutes(45)), Ok(forecast_body()));
    let mut station = StationHarness::with_cache(cache_directory);

    assert!(matches!(
        station.weather_station.weather_state(&at(0)),
        WeatherState::Fresh(_)
    ));
    station.tick(&at(0));
    assert_eq!(station.fetches(), 1, "cache older than 30 minutes");
    station.finish(&at(0), failure());

    assert!(matches!(
        station.weather_station.weather_state(&at(0)),
        WeatherState::Fresh(_)
    ));
    assert_next_fetch_at(&mut station, minutes(1));
}

#[test]
fn cached_weather_up_to_30_minutes_old_is_not_fetched_again() {
    let cache_directory = temporary_directory();
    let mut first_run = StationHarness::with_cache(cache_directory.clone());
    first_run.tick(&at(-minutes(20)));
    first_run.finish(&at(-minutes(20)), Ok(forecast_body()));
    let mut station = StationHarness::with_cache(cache_directory);

    assert_next_fetch_at(&mut station, minutes(10));
}

fn millis(milliseconds: i64) -> Zoned {
    default_instant()
        .checked_add(SignedDuration::from_millis(milliseconds))
        .unwrap()
}

#[test]
fn fetch_indicator_shows_after_300_ms_until_the_fetch_finishes() {
    let mut station = StationHarness::with_cache(temporary_directory());

    station.tick(&millis(0));

    assert!(!station.weather_station.shows_fetch_indicator(&millis(299)));
    assert!(station.weather_station.shows_fetch_indicator(&millis(300)));
    station.finish(&millis(2000), Ok(forecast_body()));
    assert!(!station.weather_station.shows_fetch_indicator(&millis(2000)));
}

#[test]
fn refresh_request_fetches_on_the_next_tick() {
    let mut station = StationHarness::with_cache(temporary_directory());
    station.tick(&at(0));
    station.finish(&at(0), Ok(forecast_body()));

    station.weather_station.request_refresh();
    station.tick(&at(5));

    assert_eq!(station.fetches(), 2);
}

#[test]
fn refresh_request_during_a_fetch_does_nothing() {
    let mut station = StationHarness::with_cache(temporary_directory());
    station.tick(&at(0));

    station.weather_station.request_refresh();
    station.finish(&at(5), Ok(forecast_body()));
    station.tick(&at(6));

    assert_eq!(station.fetches(), 1);
}

#[test]
fn successful_refresh_resets_the_retry_schedule() {
    let mut station = StationHarness::with_cache(temporary_directory());
    station.tick(&at(0));
    station.finish(&at(0), failure());

    station.weather_station.request_refresh();
    station.tick(&at(10));
    assert_eq!(station.fetches(), 2);
    station.finish(&at(10), Ok(forecast_body()));

    assert_next_fetch_at(&mut station, 10 + minutes(30));
}

#[test]
fn old_cached_weather_is_stale_with_its_age_then_expires() {
    let cache_directory = temporary_directory();
    let mut first_run = StationHarness::with_cache(cache_directory.clone());
    first_run.tick(&at(0));
    first_run.finish(&at(0), Ok(forecast_body()));
    let station = StationHarness::with_cache(cache_directory);

    let state = station.weather_station.weather_state(&at(minutes(150)));
    assert!(
        matches!(state, WeatherState::Stale { age, .. } if age == SignedDuration::from_mins(150)),
        "{state:?}"
    );
    assert_eq!(
        station
            .weather_station
            .weather_state(&at(minutes(12 * 60) + 1)),
        WeatherState::Fetching
    );
}
