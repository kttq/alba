//! PROTOTYPE (issue #55), throwaway. A 24-bit pixel canvas, drawn in half blocks: two square
//! pixels a cell, each with its own color.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn hex(value: u32) -> Self {
        Self((value >> 16) as u8, (value >> 8) as u8, value as u8)
    }

    pub fn mix(self, other: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let channel =
            |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
        Rgb(
            channel(self.0, other.0),
            channel(self.1, other.1),
            channel(self.2, other.2),
        )
    }

    pub fn scale(self, factor: f64) -> Rgb {
        let channel = |a: u8| (f64::from(a) * factor).round().clamp(0.0, 255.0) as u8;
        Rgb(channel(self.0), channel(self.1), channel(self.2))
    }

    pub fn luma(self) -> f64 {
        (0.299 * f64::from(self.0) + 0.587 * f64::from(self.1) + 0.114 * f64::from(self.2)) / 255.0
    }
}

impl From<Rgb> for Color {
    fn from(rgb: Rgb) -> Self {
        Color::Rgb(rgb.0, rgb.1, rgb.2)
    }
}

/// Picks a color along stops of (position 0..1, color), blending between them.
pub fn ramp(stops: &[(f64, Rgb)], t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    for pair in stops.windows(2) {
        let ((a, ca), (b, cb)) = (pair[0], pair[1]);
        if t <= b {
            return ca.mix(cb, if b > a { (t - a) / (b - a) } else { 0.0 });
        }
    }
    stops[stops.len() - 1].1
}

#[derive(Debug, Clone)]
pub struct Px {
    pub width: i32,
    pub height: i32,
    data: Vec<Option<Rgb>>,
}

impl Px {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            data: vec![None; (width.max(0) * height.max(0)) as usize],
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| (y * self.width + x) as usize)
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        self.index(x, y).and_then(|index| self.data[index])
    }

    pub fn set(&mut self, x: i32, y: i32, color: Rgb) {
        if let Some(index) = self.index(x, y) {
            self.data[index] = Some(color);
        }
    }

    pub fn clear(&mut self, x: i32, y: i32) {
        if let Some(index) = self.index(x, y) {
            self.data[index] = None;
        }
    }

    /// Blends over what is there; over an empty pixel it blends over `under`.
    pub fn blend(&mut self, x: i32, y: i32, color: Rgb, alpha: f64) {
        if alpha <= 0.0 {
            return;
        }
        if let Some(index) = self.index(x, y) {
            let base = self.data[index].unwrap_or(Rgb(0, 0, 0));
            self.data[index] = Some(base.mix(color, alpha));
        }
    }

    /// Soft disc of light, for glows and halos.
    pub fn glow(&mut self, cx: f64, cy: f64, radius: f64, color: Rgb, strength: f64) {
        let (x0, x1) = ((cx - radius).floor() as i32, (cx + radius).ceil() as i32);
        let (y0, y1) = ((cy - radius).floor() as i32, (cy + radius).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((f64::from(x) + 0.5 - cx).powi(2) + (f64::from(y) + 0.5 - cy).powi(2))
                    .sqrt()
                    / radius;
                if d < 1.0 {
                    self.blend(x, y, color, strength * (1.0 - d).powi(2));
                }
            }
        }
    }

    /// Half blocks: two square pixels per cell, each with its own 24-bit color.
    pub fn render_half(&self, area: Rect, buffer: &mut Buffer) {
        let rows = (self.height as u16).div_ceil(2).min(area.height);
        let columns = (self.width as u16).min(area.width);
        for row in 0..rows {
            for column in 0..columns {
                let top = self.get(i32::from(column), i32::from(row) * 2);
                let bottom = self.get(i32::from(column), i32::from(row) * 2 + 1);
                let cell = &mut buffer[(area.x + column, area.y + row)];
                let (symbol, fg, bg) = match (top, bottom) {
                    (None, None) => continue,
                    (Some(top), None) => ("▀", top, None),
                    (None, Some(bottom)) => ("▄", bottom, None),
                    (Some(top), Some(bottom)) if top == bottom => (" ", top, Some(top)),
                    (Some(top), Some(bottom)) => ("▀", top, Some(bottom)),
                };
                cell.set_symbol(symbol);
                cell.set_fg(fg.into());
                cell.set_bg(bg.map_or(Color::Reset, Color::from));
                cell.modifier = Modifier::empty();
            }
        }
    }
}
