//! PROTOTYPE (issue #55), throwaway. The 24-bit icons: shaded, animated, drawn on a pixel
//! canvas in the logo palette.

use std::f64::consts::TAU;

use super::rgb::{Px, Rgb, ramp};
use crate::{icon::Icon, sky::MoonPhase, weather::Condition};

pub const GOLD: Rgb = Rgb::hex(0xe8bd6a);
pub const OCHRE: Rgb = Rgb::hex(0xd49a3a);
pub const TERRACOTTA: Rgb = Rgb::hex(0xdd7357);
/// Derived from the logo palette.
pub const PALE: Rgb = Rgb::hex(0xf6e3b4);
pub const RAIN: Rgb = Rgb::hex(0x8fb4cf);
pub const SNOW: Rgb = Rgb::hex(0xf4efe6);

/// Draws a shaded icon into a w×h canvas. `tick` counts quarter seconds, for motion; None
/// without motion keeps every icon still and shows the bolt all the time.
pub fn icon(canvas: &mut Px, w: i32, h: i32, icon: Icon, tick: Option<i64>) {
    let (x, y) = (0, 0);
    let still = tick.is_none();
    let tick = tick.unwrap_or(0);
    let (fw, fh) = (f64::from(w), f64::from(h));
    let (fx, fy) = (f64::from(x), f64::from(y));
    match icon {
        Icon::Ghost => {
            cloud(
                canvas,
                fx + fw * 0.08,
                fy + fh * 0.2,
                fw * 0.84,
                fh * 0.7,
                CloudTone::Ghost,
                0.0,
            );
        }
        Icon::Day(Condition::Clear) => {
            sun(
                canvas,
                fx + fw / 2.0,
                fy + fh / 2.0,
                fh.min(fw) * 0.27,
                tick,
                true,
            );
        }
        Icon::Night(Condition::Clear, phase) => {
            stars(canvas, x, y, w, h, tick);
            moon(
                canvas,
                fx + fw / 2.0,
                fy + fh / 2.0,
                fh.min(fw) * 0.36,
                phase,
            );
        }
        Icon::Day(Condition::PartlyCloudy) | Icon::Night(Condition::PartlyCloudy, _) => {
            if let Icon::Night(_, phase) = icon {
                stars(canvas, x, y, w, h, tick);
                moon(
                    canvas,
                    fx + fw * 0.36,
                    fy + fh * 0.36,
                    fh.min(fw) * 0.3,
                    phase,
                );
            } else {
                sun(
                    canvas,
                    fx + fw * 0.36,
                    fy + fh * 0.36,
                    fh.min(fw) * 0.24,
                    tick,
                    true,
                );
            }
            let drift = (tick as f64 * TAU / 32.0).sin() * fw * 0.03;
            cloud(
                canvas,
                fx + fw * 0.24 + drift,
                fy + fh * 0.42,
                fw * 0.72,
                fh * 0.52,
                CloudTone::Light,
                0.0,
            );
        }
        Icon::Day(condition) | Icon::Night(condition, _) => {
            // A strike: the cloud flashes twice, and the bolt shows for a second and a half of
            // every eight seconds.
            let strike = tick.rem_euclid(32);
            let bolt_shows = still || strike < 6;
            let flash = !still && matches!(strike, 0 | 2);
            let tone = match condition {
                Condition::Cloudy | Condition::Fog | Condition::Drizzle => CloudTone::Light,
                Condition::Thunderstorm if flash => CloudTone::Flash,
                Condition::Thunderstorm | Condition::HeavyRain => CloudTone::Storm,
                _ => CloudTone::Grey,
            };
            match condition {
                Condition::Cloudy => {
                    let drift = (tick as f64 * TAU / 40.0).sin() * fw * 0.03;
                    cloud(
                        canvas,
                        fx + fw * 0.34 - drift,
                        fy + fh * 0.12,
                        fw * 0.62,
                        fh * 0.5,
                        CloudTone::Grey,
                        0.0,
                    );
                    cloud(
                        canvas,
                        fx + fw * 0.04 + drift,
                        fy + fh * 0.32,
                        fw * 0.74,
                        fh * 0.6,
                        CloudTone::Light,
                        0.0,
                    );
                }
                // The usual fog sign: a cloud over bars of mist.
                Condition::Fog => {
                    cloud(
                        canvas,
                        fx + fw * 0.12,
                        fy,
                        fw * 0.76,
                        fh * 0.56,
                        CloudTone::Grey,
                        0.0,
                    );
                    let top = (fh * 0.58).round() as i32;
                    fog(canvas, x, y + top, w, h - top, tick);
                }
                _ => {
                    let fall =
                        |canvas: &mut Px, color: Rgb, length: i32, count: i32, speed: i64| {
                            precipitation(
                                canvas,
                                x,
                                y + (fh * 0.55) as i32,
                                w,
                                h - (fh * 0.55) as i32,
                                color,
                                length,
                                count,
                                tick,
                                speed,
                            );
                        };
                    match condition {
                        Condition::Drizzle => fall(canvas, RAIN, 1, w / 4, 1),
                        Condition::Rain => fall(canvas, RAIN, 2, w / 3, 2),
                        Condition::HeavyRain => fall(canvas, RAIN, 3, w / 2, 3),
                        Condition::Snow => fall(canvas, SNOW, 1, w / 3, 1),
                        _ => {}
                    }
                    cloud(
                        canvas,
                        fx + fw * 0.04,
                        fy + fh * 0.04,
                        fw * 0.92,
                        fh * 0.6,
                        tone,
                        0.0,
                    );
                    if condition == Condition::Thunderstorm && bolt_shows {
                        bolt(
                            canvas,
                            x + w / 2,
                            y + (fh * 0.5) as i32,
                            (fh * 0.5) as i32,
                            flash,
                        );
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CloudTone {
    Light,
    Grey,
    Storm,
    Flash,
    Ghost,
}

/// A cloud of three puffs on a flat base, lit from above and shaded underneath.
pub fn cloud(canvas: &mut Px, x: f64, y: f64, w: f64, h: f64, tone: CloudTone, _seed: f64) {
    let (top, bottom, rim) = match tone {
        CloudTone::Light => (Rgb::hex(0xfbf3e4), Rgb::hex(0x9d958e), Rgb::hex(0xfff7e0)),
        CloudTone::Grey => (Rgb::hex(0xc9c1b8), Rgb::hex(0x6c6670), Rgb::hex(0xe2d8c8)),
        CloudTone::Storm => (Rgb::hex(0x7d7682), Rgb::hex(0x3a3540), Rgb::hex(0x9b91a0)),
        CloudTone::Flash => (Rgb::hex(0xffffff), Rgb::hex(0xd8d0ff), Rgb::hex(0xffffff)),
        CloudTone::Ghost => (Rgb::hex(0x3a332b), Rgb::hex(0x2a241e), Rgb::hex(0x3a332b)),
    };
    // Three puffs, the middle one tallest, over a flat base with round ends. Radii follow the
    // height, so a wide box gets a long cloud, not a flat one.
    let puffs = [(0.28, 0.60, 0.30), (0.52, 0.42, 0.42), (0.76, 0.62, 0.28)];
    let inside = |px: f64, py: f64| {
        let (u, v) = ((px - x) / w, (py - y) / h);
        if !(0.0..=1.0).contains(&u) || !(0.0..=0.95).contains(&v) {
            return false;
        }
        let end = 0.17 * h;
        let (left, right) = (x + w * 0.12 + end, x + w * 0.9 - end);
        let base_y = y + h * 0.76;
        let base = (py - base_y).abs() <= end
            && (px >= left && px <= right
                || (px - left).hypot(py - base_y) <= end
                || (px - right).hypot(py - base_y) <= end);
        base || puffs
            .iter()
            .any(|&(cx, cy, r)| (px - (x + cx * w)).hypot(py - (y + cy * h)) <= r * h)
    };
    for py in (y.floor() as i32)..=((y + h).ceil() as i32) {
        for px in (x.floor() as i32)..=((x + w).ceil() as i32) {
            let (cx, cy) = (f64::from(px) + 0.5, f64::from(py) + 0.5);
            if !inside(cx, cy) {
                continue;
            }
            let v = ((cy - y) / h).clamp(0.0, 1.0);
            let mut color = top.mix(bottom, ((v - 0.35) / 0.6).clamp(0.0, 1.0));
            if !inside(cx, cy - 1.0) || !inside(cx + 1.0, cy - 1.0) {
                color = rim;
            } else if !inside(cx, cy + 1.0) {
                color = bottom.scale(0.85);
            }
            canvas.set(px, py, color);
        }
    }
}

/// A disc lit from its centre, with turning rays and a glow.
pub fn sun(canvas: &mut Px, cx: f64, cy: f64, r: f64, tick: i64, rays: bool) {
    if r >= 2.5 {
        canvas.glow(cx, cy, r * 2.6, GOLD, 0.35);
    }
    if rays {
        let turn = tick as f64 * TAU / 64.0;
        for ray in 0..8 {
            let angle = turn + f64::from(ray) * TAU / 8.0;
            let length = if ray % 2 == 0 { 1.0 } else { 0.6 };
            let steps = (r * 0.9 * length).ceil() as i32 + 1;
            for step in 0..steps {
                let d = r * 1.35 + f64::from(step);
                let (px, py) = (cx + angle.cos() * d, cy + angle.sin() * d);
                canvas.set(
                    px.floor() as i32,
                    py.floor() as i32,
                    if step == 0 { GOLD } else { OCHRE },
                );
            }
        }
    }
    disc(canvas, cx, cy, r, |u, v| {
        let d = ((u + 0.35).powi(2) + (v + 0.35).powi(2)).sqrt();
        ramp(
            &[
                (0.0, Rgb::hex(0xfff4d0)),
                (0.5, GOLD),
                (1.0, OCHRE),
                (1.4, TERRACOTTA),
            ],
            d,
        )
    });
}

/// Tonight's moon: lit side pale gold with two craters, dark side dim, and a cool glow.
pub fn moon(canvas: &mut Px, cx: f64, cy: f64, r: f64, phase: MoonPhase) {
    canvas.glow(cx, cy, r * 2.2, Rgb::hex(0xbfb3d9), 0.18);
    let fraction = phase.fraction();
    let terminator = (TAU * fraction).cos();
    disc(canvas, cx, cy, r, |u, v| {
        let rim = u * u + v * v > (1.0 - 1.4 / r).powi(2);
        let half = (1.0 - v * v).max(0.0).sqrt();
        let lit = if fraction <= 0.5 {
            u > half * terminator
        } else {
            u < -half * terminator
        };
        let crater = ((u - 0.25).powi(2) + (v + 0.2).powi(2)) < 0.05
            || ((u + 0.3).powi(2) + (v - 0.35).powi(2)) < 0.03;
        match (lit, crater) {
            (true, false) => PALE.mix(GOLD, (u * 0.5 + v * 0.5 + 0.3).clamp(0.0, 1.0) * 0.6),
            (true, true) => Rgb::hex(0xcdb98e),
            (false, _) if rim => Rgb::hex(0x5a5270),
            (false, _) => Rgb::hex(0x2f2b3d),
        }
    });
}

/// Fills a disc; `shade` takes the position in radii from the centre.
pub fn disc(canvas: &mut Px, cx: f64, cy: f64, r: f64, shade: impl Fn(f64, f64) -> Rgb) {
    for py in ((cy - r).floor() as i32)..=((cy + r).ceil() as i32) {
        for px in ((cx - r).floor() as i32)..=((cx + r).ceil() as i32) {
            let u = (f64::from(px) + 0.5 - cx) / r;
            let v = (f64::from(py) + 0.5 - cy) / r;
            if u * u + v * v <= 1.0 {
                canvas.set(px, py, shade(u, v));
            }
        }
    }
}

pub fn stars(canvas: &mut Px, x: i32, y: i32, w: i32, h: i32, tick: i64) {
    let count = (w * h / 40).max(3);
    for star in 0..count {
        let seed = hash(star as u64 * 7919 + 13);
        let sx = x + (seed % w.max(1) as u64) as i32;
        let sy = y + ((seed >> 16) % h.max(1) as u64) as i32;
        let twinkle = ((tick / 2 + (seed >> 32) as i64 % 5) % 5) as f64 / 4.0;
        let color = PALE.mix(Rgb::hex(0x6a6380), twinkle);
        if canvas.get(sx, sy).is_none_or(|c| c.luma() < 0.3) {
            canvas.set(sx, sy, color);
        }
    }
}

pub fn hash(mut value: u64) -> u64 {
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51afd7ed558ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ceb9fe1a85ec53);
    value ^ (value >> 33)
}

/// Falling streaks in the box; `speed` is pixels per tick.
#[allow(clippy::too_many_arguments, reason = "prototype")]
pub fn precipitation(
    canvas: &mut Px,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: Rgb,
    length: i32,
    count: i32,
    tick: i64,
    speed: i64,
) {
    if w <= 0 || h <= 0 {
        return;
    }
    for drop in 0..count.max(1) {
        let seed = hash(drop as u64 * 104729 + 7);
        let column = x + (seed % w as u64) as i32;
        let start = ((seed >> 20) % (h as u64 + length as u64)) as i64;
        let drift = if color == SNOW {
            ((tick + drop as i64 * 3) as f64 / 3.0).sin().round() as i32
        } else {
            0
        };
        let head = ((start + tick * speed) % i64::from(h + length)) as i32;
        for k in 0..length {
            let py = y + head - k;
            if py >= y && py < y + h {
                canvas.set(column + drift, py, color.scale(1.0 - f64::from(k) * 0.25));
            }
        }
    }
}

/// Bars of mist, a pixel thick with a pixel between them, long and short in turn, each
/// drifting slowly on its own.
pub fn fog(canvas: &mut Px, x: i32, y: i32, w: i32, h: i32, tick: i64) {
    let bars = ((h + 1) / 2).max(1);
    for bar in 0..bars {
        let by = y + bar * 2;
        let drift = ((tick as f64 / 10.0 + f64::from(bar) * 1.7).sin() * f64::from(w) * 0.06)
            .round() as i32;
        let (start, end) = if bar % 2 == 0 {
            (x + w / 8 + drift, x + w - w / 8 + drift)
        } else {
            (x + w / 4 - drift, x + w - w / 6 - drift)
        };
        let color = if bar % 2 == 0 {
            Rgb::hex(0xd8d0c4)
        } else {
            Rgb::hex(0xa59d94)
        };
        for px in start.max(x)..end.min(x + w) {
            canvas.set(px, by, color);
        }
    }
}

pub fn bolt(canvas: &mut Px, x: i32, y: i32, h: i32, flash: bool) {
    let color = if flash {
        Rgb::hex(0xffffff)
    } else {
        Rgb::hex(0xffe08a)
    };
    let path = [(0, 0), (-1, 1), (-2, 2), (0, 2), (-1, 3), (-2, 4), (-3, 5)];
    let scale = (h / 5).max(1);
    for (dx, dy) in path {
        for k in 0..scale {
            canvas.set(x + dx * scale / 2 + 1, y + dy * scale + k, color);
            canvas.set(x + dx * scale / 2 + 2, y + dy * scale + k, OCHRE);
        }
    }
}
