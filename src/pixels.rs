//! Square pixels in named colors, packed two per cell with half blocks.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
    widgets::Widget,
};

/// The 16 named terminal colors and the default foreground, so pictures follow the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelColor {
    Foreground,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixel {
    pub color: PixelColor,
    pub faint: bool,
}

/// Maps each sprite letter to its pixel. `.` is always empty.
pub type Legend = [(char, Pixel)];

/// A picture of `width` by `height` pixels. An empty pixel is transparent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    width: u16,
    height: u16,
    pixels: Vec<Option<Pixel>>,
}

impl Pixel {
    pub const fn new(color: PixelColor) -> Self {
        Self {
            color,
            faint: false,
        }
    }

    pub const fn faint(color: PixelColor) -> Self {
        Self { color, faint: true }
    }
}

impl Canvas {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixels: vec![None; usize::from(width) * usize::from(height)],
        }
    }

    /// Panics on a letter missing from the legend: sprites are const data.
    pub fn from_rows(rows: &[&str], legend: &Legend) -> Self {
        let width = rows
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0);
        let mut canvas = Self::new(width as u16, rows.len() as u16);
        for (y, row) in rows.iter().enumerate() {
            for (x, letter) in row.chars().enumerate() {
                if letter != '.' {
                    canvas.set(x as u16, y as u16, legend_pixel(legend, letter));
                }
            }
        }
        canvas
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// None when empty or outside the canvas.
    pub fn pixel(&self, x: u16, y: u16) -> Option<Pixel> {
        self.index(x, y).and_then(|index| self.pixels[index])
    }

    /// Clipped at the edges.
    pub fn set(&mut self, x: u16, y: u16, pixel: Pixel) {
        if let Some(index) = self.index(x, y) {
            self.pixels[index] = Some(pixel);
        }
    }

    /// Paints `source` with its top left pixel at (x, y). Its empty pixels leave this canvas as it was.
    pub fn paint(&mut self, x: u16, y: u16, source: &Canvas) {
        for source_y in 0..source.height {
            for source_x in 0..source.width {
                if let Some(pixel) = source.pixel(source_x, source_y) {
                    self.set(
                        x.saturating_add(source_x),
                        y.saturating_add(source_y),
                        pixel,
                    );
                }
            }
        }
    }

    pub fn scaled(&self, factor: u16) -> Canvas {
        let mut scaled = Canvas::new(self.width * factor, self.height * factor);
        for y in 0..scaled.height {
            for x in 0..scaled.width {
                if let Some(pixel) = self.pixel(x / factor, y / factor) {
                    scaled.set(x, y, pixel);
                }
            }
        }
        scaled
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| usize::from(y) * usize::from(self.width) + usize::from(x))
    }
}

fn legend_pixel(legend: &Legend, letter: char) -> Pixel {
    legend
        .iter()
        .find(|(legend_letter, _)| *legend_letter == letter)
        .map(|(_, pixel)| *pixel)
        .unwrap_or_else(|| panic!("no pixel for sprite letter {letter:?}"))
}

/// Draws from the area's top left corner, clipped to the area. Cells with no lit pixel stay as they were.
impl Widget for &Canvas {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let columns = self.width.min(area.width);
        let rows = self.height.div_ceil(2).min(area.height);
        for row in 0..rows {
            for column in 0..columns {
                let top = self.pixel(column, row * 2);
                let bottom = self.pixel(column, row * 2 + 1);
                if let Some(packed) = PackedCell::of(top, bottom) {
                    packed.render(&mut buffer[(area.x + column, area.y + row)]);
                }
            }
        }
    }
}

struct PackedCell {
    symbol: &'static str,
    foreground: Pixel,
    background: Option<PixelColor>,
}

impl PackedCell {
    /// The faint style and the default foreground exist only on the foreground side, so a faint
    /// or default foreground pixel takes the foreground glyph. When both pixels need it, the
    /// default foreground wins, then the top. The loser draws at full strength, or not at all
    /// when it is the default foreground too.
    fn of(top: Option<Pixel>, bottom: Option<Pixel>) -> Option<Self> {
        let packed = match (top, bottom) {
            (None, None) => return None,
            (Some(top), None) => Self::half("▀", top, None),
            (None, Some(bottom)) => Self::half("▄", bottom, None),
            (Some(top), Some(bottom)) if top == bottom => Self::half("█", top, None),
            (Some(top), Some(bottom)) if takes_foreground(bottom) && !takes_foreground(top) => {
                Self::half("▄", bottom, Some(top.color))
            }
            (Some(top), Some(bottom))
                if bottom.color == PixelColor::Foreground
                    && top.color != PixelColor::Foreground =>
            {
                Self::half("▄", bottom, Some(top.color))
            }
            (Some(top), Some(bottom)) => Self::half("▀", top, Some(bottom.color)),
        };
        Some(packed)
    }

    fn half(symbol: &'static str, foreground: Pixel, background: Option<PixelColor>) -> Self {
        Self {
            symbol,
            foreground,
            background,
        }
    }

    fn render(&self, cell: &mut ratatui::buffer::Cell) {
        cell.set_symbol(self.symbol);
        cell.set_fg(self.foreground.color.into());
        cell.set_bg(self.background.map_or(Color::Reset, Color::from));
        if self.foreground.faint {
            cell.modifier.insert(Modifier::DIM);
        } else {
            cell.modifier.remove(Modifier::DIM);
        }
    }
}

fn takes_foreground(pixel: Pixel) -> bool {
    pixel.faint || pixel.color == PixelColor::Foreground
}

impl From<PixelColor> for Color {
    fn from(color: PixelColor) -> Self {
        match color {
            PixelColor::Foreground => Color::Reset,
            PixelColor::Black => Color::Black,
            PixelColor::Red => Color::Red,
            PixelColor::Green => Color::Green,
            PixelColor::Yellow => Color::Yellow,
            PixelColor::Blue => Color::Blue,
            PixelColor::Magenta => Color::Magenta,
            PixelColor::Cyan => Color::Cyan,
            PixelColor::White => Color::Gray,
            PixelColor::BrightBlack => Color::DarkGray,
            PixelColor::BrightRed => Color::LightRed,
            PixelColor::BrightGreen => Color::LightGreen,
            PixelColor::BrightYellow => Color::LightYellow,
            PixelColor::BrightBlue => Color::LightBlue,
            PixelColor::BrightMagenta => Color::LightMagenta,
            PixelColor::BrightCyan => Color::LightCyan,
            PixelColor::BrightWhite => Color::White,
        }
    }
}
