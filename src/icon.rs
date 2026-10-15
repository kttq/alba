//! The icon for a condition, in three sizes and two animation frames, with its night stars.

use jiff::Zoned;
use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

use crate::{
    motion::Motion,
    pixels::{Canvas, Legend},
    sky::{MoonPhase, Sky},
    sprites::{self, Frames},
    weather::{Condition, DayOrNight, Weather},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Icon {
    Day(Condition),
    Night(Condition, MoonPhase),
    /// No weather data.
    Ghost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    /// 6×4 pixels.
    Mini,
    /// 12×8 pixels.
    Full,
    /// The full icon at twice the scale.
    Large,
}

impl IconSize {
    /// Width and height in cells.
    pub fn cells(self) -> (u16, u16) {
        match self {
            Self::Mini => (6, 2),
            Self::Full => (12, 4),
            Self::Large => (24, 8),
        }
    }
}

impl Icon {
    /// None for a weather code with no condition.
    pub fn of(weather: &Weather, now: &Zoned) -> Option<Self> {
        let condition = weather.forecast.condition()?;
        let sky = Sky::at(&weather.forecast, now.timestamp());
        Some(match sky.day_or_night {
            DayOrNight::Day => Self::Day(condition),
            DayOrNight::Night => Self::Night(condition, sky.moon_phase),
        })
    }

    pub fn canvas(self, size: IconSize, motion: Motion) -> Canvas {
        if size == IconSize::Large {
            return self.canvas(IconSize::Full, motion).scaled(2);
        }
        if let Self::Night(Condition::Clear, moon_phase) = self {
            return moon(size, moon_phase);
        }
        let (frames, legend) = self.sprite(size, motion);
        Canvas::from_rows(frames[icon_frame(self, motion)], legend)
    }

    /// Paints the canvas, then the stars into empty cells.
    pub fn render(self, size: IconSize, motion: Motion, area: Rect, buffer: &mut Buffer) {
        let canvas = self.canvas(size, motion);
        canvas.render(area, buffer);
        if !self.is_starry() {
            return;
        }
        let stars = match size {
            IconSize::Mini => return,
            IconSize::Full => sprites::FULL_STARS,
            IconSize::Large => sprites::LARGE_STARS,
        };
        let glyphs = sprites::STAR_GLYPHS[motion.icon_frame];
        for ((column, row), glyph) in stars.into_iter().zip(glyphs) {
            let empty = canvas.pixel(column, row * 2).is_none()
                && canvas.pixel(column, row * 2 + 1).is_none();
            if empty && glyph != " " && column < area.width && row < area.height {
                buffer[(area.x + column, area.y + row)]
                    .set_symbol(glyph)
                    .set_style(Style::reset());
            }
        }
    }

    fn is_starry(self) -> bool {
        matches!(
            self,
            Self::Night(Condition::Clear | Condition::PartlyCloudy, _)
        )
    }

    fn sprite(self, size: IconSize, motion: Motion) -> (Frames, &'static Legend) {
        let condition = match self {
            Self::Ghost => return (frames(Condition::Cloudy, size), sprites::GHOST_LEGEND),
            Self::Night(Condition::PartlyCloudy, _) => {
                return (partly_cloudy_night_frames(size), sprites::ICON_LEGEND);
            }
            Self::Day(condition) | Self::Night(condition, _) => condition,
        };
        let legend = if condition == Condition::Thunderstorm && motion.flash {
            sprites::FLASH_LEGEND
        } else {
            sprites::ICON_LEGEND
        };
        (frames(condition, size), legend)
    }
}

/// The ghost icon stands still.
fn icon_frame(icon: Icon, motion: Motion) -> usize {
    match icon {
        Icon::Ghost => 0,
        _ => motion.icon_frame,
    }
}

/// Tonight's moon, as tall as the icon and one pixel in from the left.
fn moon(size: IconSize, moon_phase: MoonPhase) -> Canvas {
    let (width, height) = size.cells();
    let mut canvas = Canvas::new(width, height * 2);
    canvas.paint(1, 0, &moon_phase.disc(height * 2));
    canvas
}

fn partly_cloudy_night_frames(size: IconSize) -> Frames {
    match size {
        IconSize::Mini => sprites::MINI_PARTLY_CLOUDY_NIGHT,
        _ => sprites::FULL_PARTLY_CLOUDY_NIGHT,
    }
}

fn frames(condition: Condition, size: IconSize) -> Frames {
    match (condition, size) {
        (Condition::Clear, IconSize::Mini) => sprites::MINI_CLEAR,
        (Condition::PartlyCloudy, IconSize::Mini) => sprites::MINI_PARTLY_CLOUDY,
        (Condition::Cloudy, IconSize::Mini) => sprites::MINI_CLOUDY,
        (Condition::Fog, IconSize::Mini) => sprites::MINI_FOG,
        (Condition::Drizzle, IconSize::Mini) => sprites::MINI_DRIZZLE,
        (Condition::Rain, IconSize::Mini) => sprites::MINI_RAIN,
        (Condition::HeavyRain, IconSize::Mini) => sprites::MINI_HEAVY_RAIN,
        (Condition::Snow, IconSize::Mini) => sprites::MINI_SNOW,
        (Condition::Thunderstorm, IconSize::Mini) => sprites::MINI_THUNDERSTORM,
        (Condition::Clear, _) => sprites::FULL_CLEAR,
        (Condition::PartlyCloudy, _) => sprites::FULL_PARTLY_CLOUDY,
        (Condition::Cloudy, _) => sprites::FULL_CLOUDY,
        (Condition::Fog, _) => sprites::FULL_FOG,
        (Condition::Drizzle, _) => sprites::FULL_DRIZZLE,
        (Condition::Rain, _) => sprites::FULL_RAIN,
        (Condition::HeavyRain, _) => sprites::FULL_HEAVY_RAIN,
        (Condition::Snow, _) => sprites::FULL_SNOW,
        (Condition::Thunderstorm, _) => sprites::FULL_THUNDERSTORM,
    }
}
