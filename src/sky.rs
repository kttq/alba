//! Day and night at the forecast location, the timeline, and the moon phase.

use jiff::Timestamp;

use crate::{
    pixels::{Canvas, Pixel, PixelColor},
    weather::{DayOrNight, Forecast, SunEvent},
};

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

    /// Tonight's moon, `diameter` pixels across: lit on the right while waxing, on the left while
    /// waning, and faint on its dark side.
    pub fn disc(&self, diameter: u16) -> Canvas {
        let terminator = self.terminator();
        let radius = f64::from(diameter) / 2.0;
        let centre = (f64::from(diameter) - 1.0) / 2.0;
        let mut disc = Canvas::new(diameter, diameter);
        for y in 0..diameter {
            for x in 0..diameter {
                let across = (f64::from(x) - centre) / radius;
                let down = (f64::from(y) - centre) / radius;
                if across * across + down * down > 1.0 {
                    continue;
                }
                let half_width = (1.0 - down * down).sqrt();
                let lit = if self.fraction <= 0.5 {
                    across > half_width * terminator
                } else {
                    across < -half_width * terminator
                };
                disc.set(x, y, if lit { MOON_LIT } else { MOON_DARK });
            }
        }
        disc
    }

    /// Where the line between the lit and dark sides crosses the middle row, from -1 to 1 of the
    /// radius. Kept 0.45 of the radius from the edge, so a crescent or a gibbous dark slice is at
    /// least 2 pixels wide on an 8-pixel moon, except within 1/16 of a cycle of new or full moon.
    fn terminator(&self) -> f64 {
        let terminator = (std::f64::consts::TAU * self.fraction).cos();
        let from_new = self.fraction.min(1.0 - self.fraction);
        if from_new < 1.0 / 16.0 || (self.fraction - 0.5).abs() < 1.0 / 16.0 {
            terminator
        } else {
            terminator.clamp(-0.45, 0.45)
        }
    }
}

const MOON_LIT: Pixel = Pixel::new(PixelColor::Yellow);
const MOON_DARK: Pixel = Pixel::faint(PixelColor::BrightBlack);

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
