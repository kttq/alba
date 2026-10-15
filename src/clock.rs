use jiff::Zoned;
use ratatui::{
    style::Stylize,
    text::{Line, Span},
};

use crate::{
    config::{Config, HourFormat},
    motion::Motion,
    strings::Strings,
};

pub struct Clock {
    hour_format: HourFormat,
    strings: &'static Strings,
}

impl Clock {
    pub fn new(config: &Config) -> Self {
        Self {
            hour_format: config.hour_format,
            strings: Strings::of(config.language),
        }
    }

    pub fn text_line(&self, now: &Zoned, motion: Motion) -> Line<'static> {
        let mut spans = vec![
            Span::raw(self.hour(now)),
            colon(motion),
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
}

fn colon(motion: Motion) -> Span<'static> {
    if motion.colon_lit {
        Span::raw(":")
    } else {
        ":".dim()
    }
}
