//! Day and night at the forecast location, the timeline, and the moon phase.

use jiff::{Timestamp, Zoned};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::{
    clock::Clock,
    pixels::{Canvas, Pixel, PixelColor},
    strings::Strings,
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
                let lit = self.is_lit(across, down, terminator);
                disc.set(x, y, if lit { MOON_LIT } else { MOON_DARK });
            }
        }
        disc
    }

    /// `across` and `down` are in radii from the disc's centre.
    fn is_lit(&self, across: f64, down: f64, terminator: f64) -> bool {
        let half_width = (1.0 - down * down).sqrt();
        if self.fraction <= 0.5 {
            across > half_width * terminator
        } else {
            across < -half_width * terminator
        }
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

/// The timeline as drawn at one instant: a 2-pixel bar with the sun or tonight's moon on it, and
/// the start time, status, and end time under it. It stretches to the width it is given.
#[derive(Debug, Clone, PartialEq)]
pub struct ShownTimeline {
    /// How far now is from the start to the end, from 0 to 1.
    progress: f64,
    /// Each color holds up to its fraction of the bar.
    gradient: &'static [(f64, PixelColor)],
    /// The sun by day, tonight's moon by night.
    marker: Canvas,
    start_label: Line<'static>,
    end_label: Line<'static>,
    /// Each way to say the status, widest first.
    statuses: Vec<String>,
}

/// In pixels: the marker is a 4-pixel square, with a pixel of air on each side of it.
const MARKER_SIZE: u16 = 4;
/// Pixel rows of the bar inside the marker's height.
const BAR_ROWS: [u16; 2] = [1, 2];

const DAY_GRADIENT: [(f64, PixelColor); 5] = [
    (0.08, PixelColor::BrightRed),
    (0.22, PixelColor::Yellow),
    (0.78, PixelColor::BrightYellow),
    (0.92, PixelColor::Yellow),
    (1.0, PixelColor::BrightRed),
];
const NIGHT_GRADIENT: [(f64, PixelColor); 3] = [
    (0.08, PixelColor::Magenta),
    (0.92, PixelColor::Blue),
    (1.0, PixelColor::Magenta),
];

const SUN: Pixel = Pixel::new(PixelColor::Yellow);
const SUN_CORE: Pixel = Pixel::new(PixelColor::BrightYellow);

impl ShownTimeline {
    /// None when no sunrise and sunset surround now, so the timeline rows stay empty.
    pub fn of(forecast: &Forecast, now: &Zoned, clock: &Clock, strings: &Strings) -> Option<Self> {
        let sky = Sky::at(forecast, now.timestamp());
        let timeline = sky.timeline?;
        let time_text = |at: Timestamp| clock.time_text(&at.to_zoned(now.time_zone().clone()));
        let (start, end) = (time_text(timeline.start), time_text(timeline.end));
        let accent = Style::new().yellow();
        let dim = Style::new().dark_gray();
        let countdown = duration_text(minutes_between(now.timestamp(), timeline.end));
        let (start_label, end_label, statuses, gradient, marker) = match sky.day_or_night {
            DayOrNight::Day => (
                Line::from(vec![Span::styled("↑", accent), Span::styled(start, dim)]),
                Line::from(vec![Span::styled(end, dim), Span::styled("↓", accent)]),
                vec![format!("{} {countdown}", strings.sunset_in)],
                &DAY_GRADIENT[..],
                sun(),
            ),
            DayOrNight::Night => {
                let countdown = format!("{} {countdown}", strings.sunrise_in);
                let phase = format!(
                    "{} {}%",
                    strings.moon_phase(sky.moon_phase.name()),
                    sky.moon_phase.illumination_percent()
                );
                (
                    Line::from(vec![Span::styled("↓", accent), Span::styled(start, dim)]),
                    Line::from(vec![Span::styled(end, dim), Span::styled("↑", accent)]),
                    vec![format!("{phase} · {countdown}"), countdown],
                    &NIGHT_GRADIENT[..],
                    sky.moon_phase.disc(MARKER_SIZE),
                )
            }
        };
        let elapsed = now.timestamp().duration_since(timeline.start).as_secs_f64();
        let length = timeline.end.duration_since(timeline.start).as_secs_f64();
        Some(Self {
            progress: (elapsed / length).clamp(0.0, 1.0),
            gradient,
            marker,
            start_label,
            end_label,
            statuses,
        })
    }

    /// The narrowest it can draw: both time labels with a column between them.
    pub fn required_size(&self) -> Size {
        let labels = self.start_label.width() + self.end_label.width() + 1;
        Size::new((labels as u16).max(MARKER_SIZE + 2), 3)
    }

    fn bar(&self, width: u16) -> Canvas {
        let marker_x = (self.progress * f64::from(width - MARKER_SIZE)).round() as u16;
        let mut bar = Canvas::new(width, MARKER_SIZE);
        for x in 0..width {
            let Some(pixel) = self.bar_pixel(x, width, marker_x) else {
                continue;
            };
            for y in BAR_ROWS {
                bar.set(x, y, pixel);
            }
        }
        bar.paint(marker_x, 0, &self.marker);
        bar
    }

    /// Full color up to the marker's middle and faint ahead; None in the air beside the marker.
    fn bar_pixel(&self, x: u16, width: u16, marker_x: u16) -> Option<Pixel> {
        if x + 1 == marker_x || x == marker_x + MARKER_SIZE {
            return None;
        }
        let fraction = f64::from(x) / f64::from((width - 1).max(1));
        let (_, last) = self.gradient[self.gradient.len() - 1];
        let color = (self.gradient.iter())
            .find(|(up_to, _)| fraction <= *up_to)
            .map_or(last, |(_, color)| *color);
        Some(if x < marker_x + MARKER_SIZE / 2 {
            Pixel::new(color)
        } else {
            Pixel::faint(color)
        })
    }
}

/// The sun marker: a round disc with a bright core.
fn sun() -> Canvas {
    let mut sun = Canvas::new(MARKER_SIZE, MARKER_SIZE);
    for y in 0..MARKER_SIZE {
        for x in 0..MARKER_SIZE {
            let corner = (x == 0 || x == MARKER_SIZE - 1) && (y == 0 || y == MARKER_SIZE - 1);
            if !corner {
                sun.set(x, y, SUN);
            }
        }
    }
    sun.set(1, 1, SUN_CORE);
    sun
}

/// Whole minutes on the clock, so the countdown agrees with the times shown.
fn minutes_between(from: Timestamp, to: Timestamp) -> i64 {
    (to.as_second().div_euclid(60) - from.as_second().div_euclid(60)).max(0)
}

/// Hours, `h`, and two-digit minutes; the hours stay even when zero.
fn duration_text(minutes: i64) -> String {
    format!("{}h{:02}", minutes / 60, minutes % 60)
}

/// The bar over its labels. The status takes the widest form that fits between the times,
/// centred in the room between them.
impl Widget for &ShownTimeline {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.width < self.required_size().width || area.height < 3 {
            return;
        }
        self.bar(area.width).render(area, buffer);
        let labels_y = area.y + 2;
        let start_width = self.start_label.width() as u16;
        let end_width = self.end_label.width() as u16;
        (&self.start_label).render(Rect::new(area.x, labels_y, start_width, 1), buffer);
        let end_x = area.x + area.width - end_width;
        (&self.end_label).render(Rect::new(end_x, labels_y, end_width, 1), buffer);
        let room = area.width.saturating_sub(start_width + end_width + 2);
        let status = self
            .statuses
            .iter()
            .map(|status| Span::styled(status.as_str(), Style::new().dark_gray()))
            .find(|status| status.width() as u16 <= room);
        if let Some(status) = status {
            let status_width = status.width() as u16;
            let x = area.x + start_width + 1 + (room - status_width) / 2;
            status.render(Rect::new(x, labels_y, status_width, 1), buffer);
        }
    }
}
