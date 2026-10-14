mod common;

use std::fs;

use alba::weather::FetchError;
use common::{
    BUENOS_AIRES, Harness, default_instant, forecast_body, make_read_only, temporary_directory,
};

#[test]
fn failed_fetch_is_logged_as_one_logfmt_line() {
    let mut harness = Harness::new(BUENOS_AIRES);

    harness.fetch(&default_instant(), Err(FetchError::Status(503)));

    assert_eq!(
        harness.log(),
        "time=2026-10-13T06:38:00-03:00 level=warn msg=\"fetch failed\" error=\"http status 503\"\n"
    );
}

#[test]
fn error_text_is_quoted_and_escaped() {
    let mut harness = Harness::new(BUENOS_AIRES);

    harness.fetch(
        &default_instant(),
        Err(FetchError::Network("dns \"lookup\" failed".to_string())),
    );

    assert!(
        harness
            .log()
            .ends_with("error=\"network error: dns \\\"lookup\\\" failed\"\n"),
        "{}",
        harness.log()
    );
}

#[test]
fn each_start_truncates_the_log() {
    let state_directory = temporary_directory();
    fs::create_dir_all(&state_directory).unwrap();
    fs::write(state_directory.join("alba.log"), "an earlier run\n").unwrap();

    let harness = Harness::with_directories(BUENOS_AIRES, temporary_directory(), state_directory);

    assert_eq!(harness.log(), "");
}

#[test]
fn failed_cache_write_is_logged() {
    let cache_directory = temporary_directory();
    fs::create_dir_all(&cache_directory).unwrap();
    make_read_only(&cache_directory);
    let mut harness = Harness::with_cache(BUENOS_AIRES, cache_directory);

    harness.fetch(&default_instant(), Ok(forecast_body()));

    assert!(
        harness.log().starts_with(
            "time=2026-10-13T06:38:00-03:00 level=warn msg=\"cache write failed\" error=\""
        ),
        "{}",
        harness.log()
    );
}

#[test]
fn control_characters_are_escaped_as_logfmt_decoders_expect() {
    let mut harness = Harness::new(BUENOS_AIRES);

    harness.fetch(
        &default_instant(),
        Err(FetchError::Network("bad\u{1b}[0m\nline".to_string())),
    );

    assert!(
        harness
            .log()
            .ends_with("error=\"network error: bad\\u001b[0m\\nline\"\n"),
        "{}",
        harness.log()
    );
}
