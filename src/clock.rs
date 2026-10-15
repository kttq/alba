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

#[derive(Debug, Clone)]
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

/// The clock at one instant, ready to draw at any size.
#[derive(Debug, Clone)]
pub struct ShownClock {
    clock: Clock,
    now: Zoned,
    motion: Motion,
    text_clock: Line<'static>,
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

    pub fn at(&self, now: &Zoned, motion: Motion) -> ShownClock {
        ShownClock {
            clock: self.clone(),
            now: now.clone(),
            motion,
            text_clock: self.text_clock(now, motion),
        }
    }

    pub fn text_clock(&self, now: &Zoned, motion: Motion) -> Line<'static> {
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

    /// A time of day in the clock's hour format, with the am or pm mark on a 12-hour clock.
    pub fn time_text(&self, at: &Zoned) -> String {
        let hour_and_minute = format!("{}:{:02}", self.hour(at), at.minute());
        match self.hour_format {
            HourFormat::TwentyFour => hour_and_minute,
            HourFormat::Twelve => hour_and_minute + self.am_pm(at),
        }
    }

    pub fn segment_clock(
        &self,
        now: &Zoned,
        motion: Motion,
        digit_size: DigitSize,
    ) -> SegmentClock {
        let shape = digit_size.shape();
        let colon_gap = shape.colon_gap();
        let mut digits = Canvas::new(shape.clock_width(), shape.height);
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

    /// In cells, without drawing the digits. The drawing places its mark by the same size.
    pub fn segment_size(&self, digit_size: DigitSize) -> Size {
        let digits = digit_size.shape().cells();
        if self.hour_format == HourFormat::TwentyFour {
            return digits;
        }
        let mark_width = [self.strings.am, self.strings.pm]
            .map(|text| Span::raw(text).width())
            .into_iter()
            .max()
            .unwrap_or(0) as u16;
        Size::new(digits.width + MARK_GAP + mark_width, digits.height.max(2))
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

impl ShownClock {
    pub fn text_clock(&self) -> &Line<'static> {
        &self.text_clock
    }

    pub fn segment_size(&self, digit_size: DigitSize) -> Size {
        self.clock.segment_size(digit_size)
    }

    pub fn segment_clock(&self, digit_size: DigitSize) -> SegmentClock {
        self.clock.segment_clock(&self.now, self.motion, digit_size)
    }
}

impl DigitShape {
    /// The digits and the colon, in cells.
    fn cells(&self) -> Size {
        Size::new(self.clock_width(), self.height.div_ceil(2))
    }

    /// In pixels: four digits and the colon, each with its gaps.
    fn clock_width(&self) -> u16 {
        4 * self.width + 2 * DIGIT_GAP + self.thickness + 2 * self.colon_gap()
    }

    /// On each side of the colon.
    fn colon_gap(&self) -> u16 {
        self.thickness + 1
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

/// The mark sits top-aligned beside the digits.
impl Widget for &SegmentClock {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        self.digits.render(area, buffer);
        let Some(mark) = &self.mark else {
            return;
        };
        let mark_x = self.digits.width() + MARK_GAP;
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
