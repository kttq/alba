//! The seven-segment clock and the text clock.

use jiff::Zoned;
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Widget,
};

use crate::{
    config::{Config, HourFormat},
    motion::Motion,
    pixels::{Canvas, Pixel},
    sprites,
    strings::Strings,
};

pub struct Clock {
    hour_format: HourFormat,
    ghost_segments: bool,
    strings: &'static Strings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitSize {
    /// 4×7-pixel digits with segments 1 pixel thick.
    Small,
    /// 8×15-pixel digits with segments 2 pixels thick.
    Large,
}

#[derive(Debug)]
pub struct SegmentClock {
    digits: Canvas,
    /// The 12-hour mark: `am` over `pm`, the current one lit.
    mark: Option<[Line<'static>; 2]>,
}

/// In pixels.
struct DigitShape {
    width: u16,
    height: u16,
    thickness: u16,
    /// The top row of each colon dot.
    colon_rows: [u16; 2],
}

const DIGIT_GAP: u16 = 1;
/// In columns.
const MARK_GAP: u16 = 1;

impl Clock {
    pub fn new(config: &Config) -> Self {
        Self {
            hour_format: config.hour_format,
            ghost_segments: config.ghost_segments,
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

    /// A time of day in the clock's format, such as `06:14` or `6:14am`.
    pub fn time_text(&self, at: &Zoned) -> String {
        let minutes = format!("{}:{:02}", self.hour(at), at.minute());
        match self.hour_format {
            HourFormat::TwentyFour => minutes,
            HourFormat::Twelve => minutes + self.am_pm(at),
        }
    }

    pub fn segment_clock(
        &self,
        now: &Zoned,
        motion: Motion,
        digit_size: DigitSize,
    ) -> SegmentClock {
        let shape = digit_size.shape();
        let colon_gap = shape.thickness + 1;
        let mut digits = Canvas::new(
            4 * shape.width + 2 * DIGIT_GAP + shape.thickness + 2 * colon_gap,
            shape.height,
        );
        let colon_x = 2 * shape.width + DIGIT_GAP + colon_gap;
        let minutes_x = colon_x + shape.thickness + colon_gap;
        let digit_xs = [
            0,
            shape.width + DIGIT_GAP,
            minutes_x,
            minutes_x + shape.width + DIGIT_GAP,
        ];
        for (x, digit) in digit_xs.into_iter().zip(self.digits_at(now)) {
            self.paint_digit(&mut digits, x, &shape, digit);
        }
        if let Some(pixel) = self.colon_pixel(motion) {
            for row in shape.colon_rows {
                let size = shape.thickness;
                fill(&mut digits, colon_x, row, size, size, pixel);
            }
        }
        SegmentClock {
            digits,
            mark: self.mark_at(now),
        }
    }

    fn mark_at(&self, now: &Zoned) -> Option<[Line<'static>; 2]> {
        if self.hour_format == HourFormat::TwentyFour {
            return None;
        }
        let lit = Style::reset();
        let unlit = Style::reset().dim();
        let (am, pm) = if now.hour() < 12 {
            (lit, unlit)
        } else {
            (unlit, lit)
        };
        Some([
            Line::styled(self.strings.am, am),
            Line::styled(self.strings.pm, pm),
        ])
    }

    /// None is the empty leading digit of a 12-hour clock.
    fn digits_at(&self, now: &Zoned) -> [Option<usize>; 4] {
        let hour = self.shown_hour(now) as usize;
        let hour_tens = match self.hour_format {
            HourFormat::Twelve if hour < 10 => None,
            _ => Some(hour / 10),
        };
        let minute = now.minute() as usize;
        [
            hour_tens,
            Some(hour % 10),
            Some(minute / 10),
            Some(minute % 10),
        ]
    }

    /// Ghost segments first, so the lit ones cover them.
    fn paint_digit(&self, canvas: &mut Canvas, x: u16, shape: &DigitShape, digit: Option<usize>) {
        if self.ghost_segments {
            paint_segments(
                canvas,
                x,
                shape,
                sprites::DIGIT_SEGMENTS[8],
                sprites::GHOST_SEGMENT,
            );
        }
        if let Some(digit) = digit {
            paint_segments(
                canvas,
                x,
                shape,
                sprites::DIGIT_SEGMENTS[digit],
                sprites::LIT_SEGMENT,
            );
        }
    }

    fn colon_pixel(&self, motion: Motion) -> Option<Pixel> {
        if motion.colon_lit {
            Some(sprites::LIT_SEGMENT)
        } else {
            self.ghost_segments.then_some(sprites::GHOST_SEGMENT)
        }
    }

    fn hour(&self, now: &Zoned) -> String {
        match self.hour_format {
            HourFormat::TwentyFour => format!("{:02}", self.shown_hour(now)),
            HourFormat::Twelve => self.shown_hour(now).to_string(),
        }
    }

    /// 0 to 23, or 1 to 12 on a 12-hour clock.
    fn shown_hour(&self, now: &Zoned) -> i8 {
        match (self.hour_format, now.hour() % 12) {
            (HourFormat::TwentyFour, _) => now.hour(),
            (HourFormat::Twelve, 0) => 12,
            (HourFormat::Twelve, hour) => hour,
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

impl DigitSize {
    fn shape(self) -> DigitShape {
        match self {
            Self::Small => DigitShape {
                width: 4,
                height: 7,
                thickness: 1,
                colon_rows: [2, 5],
            },
            Self::Large => DigitShape {
                width: 8,
                height: 15,
                thickness: 2,
                colon_rows: [4, 9],
            },
        }
    }
}

/// Segments stop short of every corner pixel.
fn paint_segments(canvas: &mut Canvas, x: u16, shape: &DigitShape, segments: &str, pixel: Pixel) {
    let DigitShape {
        width: w,
        height: h,
        thickness: t,
        ..
    } = *shape;
    let middle = (h - t) / 2;
    for segment in segments.chars() {
        let (left, top, right, bottom) = match segment {
            'a' => (t, 0, w - t, t),
            'b' => (w - t, t, w, middle),
            'c' => (w - t, middle + t, w, h - t),
            'd' => (t, h - t, w - t, h),
            'e' => (0, middle + t, t, h - t),
            'f' => (0, t, t, middle),
            'g' => (t, middle, w - t, middle + t),
            other => panic!("no digit segment {other:?}"),
        };
        fill(canvas, x + left, top, right - left, bottom - top, pixel);
    }
}

fn fill(canvas: &mut Canvas, x: u16, y: u16, width: u16, height: u16, pixel: Pixel) {
    for dy in 0..height {
        for dx in 0..width {
            canvas.set(x + dx, y + dy, pixel);
        }
    }
}

impl SegmentClock {
    /// In cells.
    pub fn size(&self) -> Size {
        let digits = self.digits_size();
        match &self.mark {
            None => digits,
            Some(mark) => Size::new(
                digits.width + MARK_GAP + mark_width(mark),
                digits.height.max(mark.len() as u16),
            ),
        }
    }

    fn digits_size(&self) -> Size {
        Size::new(self.digits.width(), self.digits.height().div_ceil(2))
    }
}

fn mark_width(mark: &[Line<'static>; 2]) -> u16 {
    mark.iter().map(Line::width).max().unwrap_or(0) as u16
}

/// The mark sits top-aligned beside the digits.
impl Widget for &SegmentClock {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        self.digits.render(area, buffer);
        let Some(mark) = &self.mark else {
            return;
        };
        let mark_x = self.digits_size().width + MARK_GAP;
        let mark_area = Rect {
            x: area.x + mark_x,
            width: area.width.saturating_sub(mark_x),
            ..area
        };
        for (row, line) in mark.iter().enumerate() {
            let row = row as u16;
            if row < mark_area.height {
                line.render(
                    Rect {
                        y: mark_area.y + row,
                        height: 1,
                        ..mark_area
                    },
                    buffer,
                );
            }
        }
    }
}
