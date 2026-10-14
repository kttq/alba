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
    fraction: f64,
    name: MoonPhaseName,
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

/// J2000.0, 2000-01-01 12:00. The minute between UTC and Terrestrial Time is too small to matter.
const J2000: Timestamp = Timestamp::constant(946_728_000, 0);
const JULIAN_CENTURY_SECONDS: f64 = 36525.0 * 24.0 * 60.0 * 60.0;

impl MoonPhase {
    pub fn at(now: Timestamp) -> Self {
        let elongation = elongation_degrees(now);
        let fraction = (elongation / 360.0).rem_euclid(1.0);
        let eighth = (fraction * 8.0).round() as usize % 8;
        Self {
            fraction,
            name: MoonPhaseName::IN_CYCLE_ORDER[eighth],
        }
    }

    /// Of the cycle since new moon, from 0 up to 1.
    pub fn fraction(&self) -> f64 {
        self.fraction
    }

    pub fn name(&self) -> MoonPhaseName {
        self.name
    }

    pub fn illumination_percent(&self) -> u8 {
        let illumination = (1.0 - (std::f64::consts::TAU * self.fraction).cos()) / 2.0;
        (illumination * 100.0).round() as u8
    }
}

/// The Moon's angle from the Sun, seen from the Earth: Meeus's low-precision formula, within
/// about an hour of the true phase.
fn elongation_degrees(now: Timestamp) -> f64 {
    let centuries = now.duration_since(J2000).as_secs_f64() / JULIAN_CENTURY_SECONDS;
    let mean_elongation = 297.8501921 + 445267.1114034 * centuries;
    let sun_anomaly = 357.5291092 + 35999.0502909 * centuries;
    let moon_anomaly = 134.9633964 + 477198.8675055 * centuries;
    let sine = |degrees: f64| degrees.to_radians().sin();
    mean_elongation + 6.289 * sine(moon_anomaly) - 2.100 * sine(sun_anomaly)
        + 1.274 * sine(2.0 * mean_elongation - moon_anomaly)
        + 0.658 * sine(2.0 * mean_elongation)
        + 0.214 * sine(2.0 * moon_anomaly)
        + 0.110 * sine(mean_elongation)
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
