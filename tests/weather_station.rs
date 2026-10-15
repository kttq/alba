mod common;

use alba::{
    config::Language,
    weather::{FetchError, Request, WeatherState},
};
use common::{
    GEOCODED_BUENOS_AIRES, NO_MATCH, WeatherStationHarness, default_instant, forecast_body,
    geocoding_body, temporary_directory,
};
use jiff::{SignedDuration, Zoned};

fn at(seconds: i64) -> Zoned {
    default_instant()
        .checked_add(SignedDuration::from_secs(seconds))
        .unwrap()
}

fn seconds_from_minutes(minutes: i64) -> i64 {
    minutes * 60
}

fn failure() -> Result<String, FetchError> {
    Err(FetchError::Network("timed out".to_string()))
}

/// Asserts that the next fetch starts exactly at `seconds`, not one second earlier.
fn assert_next_fetch_at(harness: &mut WeatherStationHarness, seconds: i64) {
    let fetches = harness.fetches();
    harness.tick(&at(seconds - 1));
    assert_eq!(harness.fetches(), fetches, "fetch before {seconds} s");
    harness.tick(&at(seconds));
    assert_eq!(harness.fetches(), fetches + 1, "no fetch at {seconds} s");
}

#[test]
fn offline_start_retries_at_1_2_5_then_10_minutes_until_weather_arrives() {
    let mut harness = WeatherStationHarness::with_cache(temporary_directory());

    assert_eq!(
        harness.weather_station.weather_state(&at(0)),
        WeatherState::Fetching
    );
    harness.tick(&at(0));
    assert_eq!(harness.fetches(), 1);
    harness.finish(&at(0), failure());
    assert_eq!(
        harness.weather_station.weather_state(&at(0)),
        WeatherState::Offline
    );

    let mut failed_at = 0;
    for wait in [1, 2, 5, 10, 10] {
        assert_next_fetch_at(&mut harness, failed_at + seconds_from_minutes(wait));
        failed_at += seconds_from_minutes(wait);
        assert_eq!(
            harness.weather_station.weather_state(&at(failed_at)),
            WeatherState::Offline,
            "offline while retrying at {failed_at} s"
        );
        harness.finish(&at(failed_at), failure());
    }

    assert_next_fetch_at(&mut harness, failed_at + seconds_from_minutes(10));
    let fetched_at = failed_at + seconds_from_minutes(10);
    harness.finish(&at(fetched_at), Ok(forecast_body()));
    assert!(matches!(
        harness.weather_station.weather_state(&at(fetched_at)),
        WeatherState::Fresh(_)
    ));
    assert_next_fetch_at(&mut harness, fetched_at + seconds_from_minutes(30));
}

#[test]
fn failed_refresh_of_old_cached_weather_keeps_it_and_waits_1_minute() {
    let cache_directory = temporary_directory();
    let mut first_run = WeatherStationHarness::with_cache(cache_directory.clone());
    first_run.tick(&at(-seconds_from_minutes(45)));
    first_run.finish(&at(-seconds_from_minutes(45)), Ok(forecast_body()));
    let mut harness = WeatherStationHarness::with_cache(cache_directory);

    assert!(matches!(
        harness.weather_station.weather_state(&at(0)),
        WeatherState::Fresh(_)
    ));
    harness.tick(&at(0));
    assert_eq!(harness.fetches(), 1, "cache older than 30 minutes");
    harness.finish(&at(0), failure());

    assert!(matches!(
        harness.weather_station.weather_state(&at(0)),
        WeatherState::Fresh(_)
    ));
    assert_next_fetch_at(&mut harness, seconds_from_minutes(1));
}

#[test]
fn cached_weather_up_to_30_minutes_old_is_not_fetched_again() {
    let cache_directory = temporary_directory();
    let mut first_run = WeatherStationHarness::with_cache(cache_directory.clone());
    first_run.tick(&at(-seconds_from_minutes(20)));
    first_run.finish(&at(-seconds_from_minutes(20)), Ok(forecast_body()));
    let mut harness = WeatherStationHarness::with_cache(cache_directory);

    assert_next_fetch_at(&mut harness, seconds_from_minutes(10));
}

fn millis(milliseconds: i64) -> Zoned {
    default_instant()
        .checked_add(SignedDuration::from_millis(milliseconds))
        .unwrap()
}

#[test]
fn fetch_indicator_shows_after_300_ms_until_the_fetch_finishes() {
    let mut harness = WeatherStationHarness::with_cache(temporary_directory());

    harness.tick(&millis(0));

    assert!(!harness.weather_station.shows_fetch_indicator(&millis(299)));
    assert!(harness.weather_station.shows_fetch_indicator(&millis(300)));
    harness.finish(&millis(2000), Ok(forecast_body()));
    assert!(!harness.weather_station.shows_fetch_indicator(&millis(2000)));
}

#[test]
fn refresh_request_fetches_on_the_next_tick() {
    let mut harness = WeatherStationHarness::with_cache(temporary_directory());
    harness.tick(&at(0));
    harness.finish(&at(0), Ok(forecast_body()));

    harness.weather_station.request_refresh();
    harness.tick(&at(5));

    assert_eq!(harness.fetches(), 2);
}

#[test]
fn refresh_request_during_a_fetch_does_nothing() {
    let mut harness = WeatherStationHarness::with_cache(temporary_directory());
    harness.tick(&at(0));

    harness.weather_station.request_refresh();
    harness.finish(&at(5), Ok(forecast_body()));
    harness.tick(&at(6));

    assert_eq!(harness.fetches(), 1);
}

#[test]
fn successful_refresh_resets_the_retry_schedule() {
    let mut harness = WeatherStationHarness::with_cache(temporary_directory());
    harness.tick(&at(0));
    harness.finish(&at(0), failure());

    harness.weather_station.request_refresh();
    harness.tick(&at(10));
    assert_eq!(harness.fetches(), 2);
    harness.finish(&at(10), Ok(forecast_body()));

    assert_next_fetch_at(&mut harness, 10 + seconds_from_minutes(30));
}

#[test]
fn old_cached_weather_is_stale_with_its_age_then_expires() {
    let cache_directory = temporary_directory();
    let mut first_run = WeatherStationHarness::with_cache(cache_directory.clone());
    first_run.tick(&at(0));
    first_run.finish(&at(0), Ok(forecast_body()));
    let harness = WeatherStationHarness::with_cache(cache_directory);

    let state = harness
        .weather_station
        .weather_state(&at(seconds_from_minutes(150)));
    assert!(
        matches!(state, WeatherState::Stale { age, .. } if age == SignedDuration::from_mins(150)),
        "{state:?}"
    );
    assert_eq!(
        harness
            .weather_station
            .weather_state(&at(seconds_from_minutes(12 * 60) + 1)),
        WeatherState::Fetching
    );
}

fn lookup(name: &str, language: Language) -> Request {
    Request::Lookup {
        name: name.to_string(),
        language,
    }
}

#[test]
fn location_name_is_looked_up_then_its_forecast_fetched() {
    let mut harness =
        WeatherStationHarness::named("buenos aires", Language::Es, temporary_directory());

    harness.tick(&at(0));
    assert_eq!(
        harness.weather_source.requests(),
        [lookup("buenos aires", Language::Es)]
    );
    assert_eq!(
        harness.weather_station.weather_state(&at(0)),
        WeatherState::Fetching
    );
    harness.finish(&at(1), Ok(geocoding_body()));

    assert_eq!(
        harness.weather_source.requests()[1..],
        [Request::Forecast(GEOCODED_BUENOS_AIRES)]
    );
    assert_eq!(
        harness.weather_station.place_name(),
        Some("buenos aires, ar")
    );
    harness.finish(&at(2), Ok(forecast_body()));
    assert!(matches!(
        harness.weather_station.weather_state(&at(2)),
        WeatherState::Fresh(_)
    ));
    assert_next_fetch_at(&mut harness, 2 + seconds_from_minutes(30));
    assert_eq!(
        harness.weather_source.requests()[2..],
        [Request::Forecast(GEOCODED_BUENOS_AIRES)],
        "a refresh does not look the name up again"
    );
}

#[test]
fn configured_coordinates_have_no_place_name() {
    let harness = WeatherStationHarness::with_cache(temporary_directory());

    assert_eq!(harness.weather_station.place_name(), None);
}

#[test]
fn fetch_indicator_runs_through_the_lookup_and_the_forecast() {
    let mut harness =
        WeatherStationHarness::named("buenos aires", Language::En, temporary_directory());

    harness.tick(&millis(0));
    harness.finish(&millis(200), Ok(geocoding_body()));

    assert!(harness.weather_station.shows_fetch_indicator(&millis(300)));
}

#[test]
fn no_match_is_location_not_found_until_r_retries_the_lookup() {
    let mut harness = WeatherStationHarness::named("atlantis", Language::En, temporary_directory());
    harness.tick(&at(0));

    harness.finish(&at(0), Ok(NO_MATCH.to_string()));

    assert_eq!(
        harness.weather_station.weather_state(&at(0)),
        WeatherState::LocationNotFound("atlantis")
    );
    harness.tick(&at(seconds_from_minutes(60)));
    assert_eq!(harness.fetches(), 1, "no automatic retry");
    harness.weather_station.request_refresh();
    harness.tick(&at(seconds_from_minutes(61)));
    assert_eq!(
        harness.weather_source.requests()[1..],
        [lookup("atlantis", Language::En)]
    );
    assert_eq!(
        harness
            .weather_station
            .weather_state(&at(seconds_from_minutes(61))),
        WeatherState::Fetching
    );
}

#[test]
fn failed_lookup_is_offline_and_retries_on_the_schedule() {
    let mut harness =
        WeatherStationHarness::named("buenos aires", Language::En, temporary_directory());
    harness.tick(&at(0));

    harness.finish(&at(0), failure());

    assert_eq!(
        harness.weather_station.weather_state(&at(0)),
        WeatherState::Offline
    );
    assert_next_fetch_at(&mut harness, seconds_from_minutes(1));
    assert_eq!(
        harness.weather_source.requests()[1..],
        [lookup("buenos aires", Language::En)]
    );
}

#[test]
fn cached_location_is_not_looked_up_again_until_the_name_changes() {
    let cache_directory = temporary_directory();
    let mut first_run =
        WeatherStationHarness::named("buenos aires", Language::En, cache_directory.clone());
    first_run.tick(&at(0));
    first_run.finish(&at(0), Ok(geocoding_body()));
    first_run.finish(&at(0), Ok(forecast_body()));

    let mut next_start =
        WeatherStationHarness::named("buenos aires", Language::En, cache_directory.clone());
    assert_eq!(
        next_start.weather_station.place_name(),
        Some("buenos aires, ar")
    );
    assert!(matches!(
        next_start.weather_station.weather_state(&at(0)),
        WeatherState::Fresh(_)
    ));
    next_start.tick(&at(seconds_from_minutes(30)));
    assert_eq!(
        next_start.weather_source.requests(),
        [Request::Forecast(GEOCODED_BUENOS_AIRES)]
    );

    let mut renamed = WeatherStationHarness::named("paris", Language::En, cache_directory);
    assert_eq!(renamed.weather_station.place_name(), None);
    assert_eq!(
        renamed.weather_station.weather_state(&at(0)),
        WeatherState::Fetching
    );
    renamed.tick(&at(0));
    assert_eq!(
        renamed.weather_source.requests(),
        [lookup("paris", Language::En)]
    );
}
