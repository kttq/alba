//! The tick interval and every animation phase, from the Unix time and the `motion` setting.

use std::time::Duration;

use jiff::Timestamp;

const TICK: Duration = Duration::from_millis(250);
const TICKS_PER_SECOND: i64 = 4;
const FLASH_WINDOW_SECONDS: i64 = 8;

/// The colon blinks every half second, so motion needs the quarter-second tick.
pub fn tick_interval(motion: bool) -> Duration {
    if motion { TICK } else { Duration::from_secs(1) }
}

/// Every animation phase at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Motion {
    /// Lit for the first two ticks of each second; a ghost in the other two.
    pub colon_lit: bool,
    /// 0 or 1, changing each second.
    pub icon_frame: usize,
    /// The thunderstorm flash: the first tick of each eight-second window.
    pub flash: bool,
}

impl Motion {
    /// Without motion: the colon lit, frame 1, and no flash.
    pub fn at(now: Timestamp, motion: bool) -> Self {
        if !motion {
            return Self {
                colon_lit: true,
                icon_frame: 0,
                flash: false,
            };
        }
        let tick = now.as_millisecond().div_euclid(TICK.as_millis() as i64);
        let second = tick.div_euclid(TICKS_PER_SECOND);
        let tick_in_second = tick.rem_euclid(TICKS_PER_SECOND);
        Self {
            colon_lit: tick_in_second < 2,
            icon_frame: second.rem_euclid(2) as usize,
            flash: second.rem_euclid(FLASH_WINDOW_SECONDS) == 0 && tick_in_second == 0,
        }
    }
}
