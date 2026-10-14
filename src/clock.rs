use std::time::Duration;

use jiff::Zoned;
use ratatui::{
    style::Stylize,
    text::{Line, Span},
};

use crate::{
    config::{Config, HourFormat},
    strings::Strings,
};

pub struct Clock {
    hour_format: HourFormat,
    motion: bool,
    strings: &'static Strings,
}

impl Clock {
    pub fn new(config: &Config) -> Self {
        Self {
            hour_format: config.hour_format,
            motion: config.motion,
            strings: Strings::of(config.language),
        }
    }

    /// The blinking colon needs a quarter-second tick.
    pub fn tick_interval(&self) -> Duration {
        if self.motion {
            Duration::from_millis(250)
        } else {
            Duration::from_secs(1)
        }
    }

    pub fn text_line(&self, now: &Zoned) -> Line<'static> {
        let mut spans = vec![
            Span::raw(self.hour(now)),
            self.colon(now),
            Span::raw(format!("{:02}", now.minute())),
        ];
        if self.hour_format == HourFormat::Twelve {
            spans.push(self.am_pm(now).dim());
        }
        Line::from(spans)
    }

    fn hour(&self, now: &Zoned) -> String {
        match (self.hour_format, now.hour() % 12) {
            (HourFormat::TwentyFour, _) => format!("{:02}", now.hour()),
            (HourFormat::Twelve, 0) => "12".to_string(),
            (HourFormat::Twelve, hour) => hour.to_string(),
        }
    }

    fn am_pm(&self, now: &Zoned) -> &'static str {
        if now.hour() < 12 {
            self.strings.am
        } else {
            self.strings.pm
        }
    }

    fn colon(&self, now: &Zoned) -> Span<'static> {
        let first_half = now.subsec_nanosecond() < 500_000_000;
        if first_half || !self.motion {
            Span::raw(":")
        } else {
            ":".dim()
        }
    }
}
