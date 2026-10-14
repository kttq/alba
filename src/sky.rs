//! Day and night at the forecast location, the timeline, and the moon phase.

use jiff::Timestamp;

use crate::weather::{DayOrNight, Forecast, SunEvent};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sky {
    pub day_or_night: DayOrNight,
    /// None when no sunrise and sunset surround now, as in polar day and night.
    pub timeline: Option<Timeline>,
    pub moon_phase: MoonPhase,
}

/// From sunrise to sunset by day, or from sunset to sunrise by night.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeline {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl Sky {
    pub fn at(forecast: &Forecast, now: Timestamp) -> Self {
        let (day_or_night, timeline) = match surrounding(&forecast.sun_events(), now) {
            Some((SunEvent::Sunrise(start), SunEvent::Sunset(end))) => {
                (DayOrNight::Day, Some(Timeline { start, end }))
            }
            Some((SunEvent::Sunset(start), SunEvent::Sunrise(end))) => {
                (DayOrNight::Night, Some(Timeline { start, end }))
            }
            _ => (forecast.is_day(), None),
        };
        Self {
            day_or_night,
            timeline,
            moon_phase: MoonPhase::at(now),
        }
    }
}

/// The last event at or before now, and the first after it.
fn surrounding(sun_events: &[SunEvent], now: Timestamp) -> Option<(SunEvent, SunEvent)> {
    let next = sun_events.partition_point(|event| event.at() <= now);
    Some((sun_events[next.checked_sub(1)?], *sun_events.get(next)?))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoonPhase {
    /// Of the synodic month since new moon, from 0 up to 1.
    pub fraction: f64,
    pub name: MoonPhaseName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoonPhaseName {
    NewMoon,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    FullMoon,
    WaningGibbous,
    LastQuarter,
    WaningCrescent,
}

/// 2000-01-06 18:14 UTC.
const REFERENCE_NEW_MOON: Timestamp = Timestamp::constant(947_182_440, 0);
const SYNODIC_MONTH_SECONDS: f64 = 29.530589 * 24.0 * 60.0 * 60.0;

impl MoonPhase {
    /// The mean synodic month, so a phase can be up to about a day off the true one.
    pub fn at(now: Timestamp) -> Self {
        let age = now.duration_since(REFERENCE_NEW_MOON).as_secs_f64();
        let fraction = (age / SYNODIC_MONTH_SECONDS).rem_euclid(1.0);
        let eighth = (fraction * 8.0).round() as usize % 8;
        Self {
            fraction,
            name: MoonPhaseName::IN_CYCLE_ORDER[eighth],
        }
    }

    pub fn illumination_percent(&self) -> u8 {
        let illumination = (1.0 - (std::f64::consts::TAU * self.fraction).cos()) / 2.0;
        (illumination * 100.0).round() as u8
    }
}

impl MoonPhaseName {
    const IN_CYCLE_ORDER: [Self; 8] = [
        Self::NewMoon,
        Self::WaxingCrescent,
        Self::FirstQuarter,
        Self::WaxingGibbous,
        Self::FullMoon,
        Self::WaningGibbous,
        Self::LastQuarter,
        Self::WaningCrescent,
    ];
}
