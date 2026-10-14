use jiff::{SignedDuration, Timestamp};

use super::Weather;

const STALE_AFTER: SignedDuration = SignedDuration::from_mins(60);
const EXPIRED_AFTER: SignedDuration = SignedDuration::from_hours(12);

/// Expired weather counts as no weather.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale(SignedDuration),
    Expired,
}

impl Weather {
    pub fn freshness(&self, now: Timestamp) -> Freshness {
        let age = now.duration_since(self.fetched_at);
        if age > EXPIRED_AFTER || self.forecast.precipitation_after(now).is_empty() {
            Freshness::Expired
        } else if age > STALE_AFTER {
            Freshness::Stale(age)
        } else {
            Freshness::Fresh
        }
    }
}
