use jiff::{SignedDuration, Timestamp};

const REFRESH: SignedDuration = SignedDuration::from_mins(30);

/// After the last entry, retries repeat it.
const RETRY_MINUTES: [i64; 4] = [1, 2, 5, 10];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastFetchResult {
    Success,
    Failure {
        /// Failures in a row, this one included.
        failures: u32,
        retry_after: Option<SignedDuration>,
    },
}

/// When the next automatic fetch starts, given when the last one finished.
pub fn next_fetch(last_fetch_result: LastFetchResult, fetched_at: Timestamp) -> Timestamp {
    let wait = match last_fetch_result {
        LastFetchResult::Success => REFRESH,
        LastFetchResult::Failure {
            failures,
            retry_after,
        } => retry_wait(failures).max(retry_after.unwrap_or(SignedDuration::ZERO)),
    };
    // Saturates, because `Retry-After` comes from the server and can be any size.
    fetched_at
        .saturating_add(wait)
        .expect("a signed duration has no calendar units")
}

fn retry_wait(failures: u32) -> SignedDuration {
    let index = (failures.max(1) as usize - 1).min(RETRY_MINUTES.len() - 1);
    SignedDuration::from_mins(RETRY_MINUTES[index])
}
