mod common;

use alba::weather::{LastFetchResult, next_fetch};
use common::default_instant;
use jiff::{SignedDuration, Timestamp};

fn fetched_at() -> Timestamp {
    default_instant().timestamp()
}

fn minutes(minutes: i64) -> SignedDuration {
    SignedDuration::from_mins(minutes)
}

#[test]
fn success_refreshes_after_30_minutes() {
    assert_eq!(
        next_fetch(LastFetchResult::Success, fetched_at()),
        fetched_at() + minutes(30)
    );
}

#[test]
fn failures_retry_at_1_2_5_then_every_10_minutes() {
    let table = [(1, 1), (2, 2), (3, 5), (4, 10), (5, 10), (40, 10)];

    for (failures, wait) in table {
        let last_fetch_result = LastFetchResult::Failure {
            failures,
            retry_after: None,
        };

        assert_eq!(
            next_fetch(last_fetch_result, fetched_at()),
            fetched_at() + minutes(wait),
            "failure {failures}"
        );
    }
}

#[test]
fn longer_retry_after_wins_over_the_schedule() {
    let last_fetch_result = LastFetchResult::Failure {
        failures: 1,
        retry_after: Some(SignedDuration::from_secs(90)),
    };

    assert_eq!(
        next_fetch(last_fetch_result, fetched_at()),
        fetched_at() + SignedDuration::from_secs(90)
    );
}

#[test]
fn shorter_retry_after_keeps_the_schedule() {
    let last_fetch_result = LastFetchResult::Failure {
        failures: 3,
        retry_after: Some(SignedDuration::from_secs(30)),
    };

    assert_eq!(
        next_fetch(last_fetch_result, fetched_at()),
        fetched_at() + minutes(5)
    );
}
