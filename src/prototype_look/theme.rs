//! PROTOTYPE (issue #55), throwaway. The three themes. `terminal`, the default, keeps the
//! terminal's own named colors; `dark` and `warm` fill the pane and map each named color to one
//! of their own.

use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    style::{Color, Modifier},
};

use super::{icons, rgb::Rgb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Terminal,
    Dark,
    Warm,
}

impl Theme {
    pub const ALL: [Self; 3] = [Self::Terminal, Self::Dark, Self::Warm];

    pub fn key(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Dark => "dark",
            Self::Warm => "warm",
        }
    }
}

fn is_pixel(cell: &Cell) -> bool {
    matches!(cell.symbol(), "▀" | "▄" | "█") || (cell.symbol() == " " && cell.bg != Color::Reset)
}

const SPARK: [&str; 7] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇"];

/// v1 draws the date in light blue and the greeting in green, the two colors that change most
/// between themes. In the terminal theme, yellow text fades on light themes, so the strong
/// accent is bold in the theme's own text color, and red is the second accent; yellow stays in
/// the pixels. The dark and warm themes map yellow to their own accent.
pub fn accents(theme: Theme, area: Rect, buffer: &mut Buffer) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buffer[(x, y)];
            if is_pixel(cell) || SPARK.contains(&cell.symbol()) {
                continue;
            }
            match (cell.fg, theme) {
                (Color::LightBlue | Color::Yellow, Theme::Terminal) => {
                    cell.fg = Color::Reset;
                    cell.modifier.insert(Modifier::BOLD);
                }
                (Color::LightBlue, _) => cell.fg = Color::Yellow,
                (Color::Green, _) => cell.fg = Color::Red,
                _ => {}
            }
        }
    }
}

/// Fills the pane and maps every named color to the theme's own.
pub fn paint(theme: Theme, area: Rect, buffer: &mut Buffer) {
    let Some(palette) = Palette::of(theme) else {
        return;
    };
    for y in area.top()..area.bottom() {
        let t = f64::from(y - area.top()) / f64::from(area.height.max(1));
        let background = palette.background.mix(palette.background_bottom, t);
        for x in area.left()..area.right() {
            let cell = &mut buffer[(x, y)];
            let pixel = is_pixel(cell);
            let dim = cell.modifier.contains(Modifier::DIM);
            cell.fg = palette
                .map(cell.fg, pixel, true, dim)
                .unwrap_or(background)
                .into();
            cell.bg = palette
                .map(cell.bg, pixel, false, dim)
                .unwrap_or(background)
                .into();
            cell.modifier.remove(Modifier::DIM);
        }
    }
}

struct Palette {
    background: Rgb,
    background_bottom: Rgb,
    text: Rgb,
    lit: Rgb,
    label: Rgb,
    ghost: Rgb,
    accent: Rgb,
    second: Rgb,
    warm: bool,
}

impl Palette {
    fn of(theme: Theme) -> Option<Self> {
        match theme {
            Theme::Terminal => None,
            Theme::Dark => Some(Self {
                background: Rgb::hex(0x15110c),
                background_bottom: Rgb::hex(0x100d09),
                text: Rgb::hex(0xeadfc8),
                lit: Rgb::hex(0xf0d9a0),
                label: Rgb::hex(0x8d7d65),
                ghost: Rgb::hex(0x2a2118),
                accent: icons::GOLD,
                second: icons::TERRACOTTA,
                warm: false,
            }),
            Theme::Warm => Some(Self {
                background: Rgb::hex(0xeadba8),
                background_bottom: Rgb::hex(0xd6c088),
                text: Rgb::hex(0x2a1d10),
                lit: Rgb::hex(0x2a1d10),
                label: Rgb::hex(0x7a6440),
                ghost: Rgb::hex(0xcfb986),
                accent: Rgb::hex(0x8a4f12),
                second: Rgb::hex(0x9a3f22),
                warm: true,
            }),
        }
    }

    /// None for the background.
    fn map(&self, color: Color, pixel: bool, foreground: bool, dim: bool) -> Option<Rgb> {
        let named = |dark: u32, warm: u32| Some(Rgb::hex(if self.warm { warm } else { dark }));
        match color {
            Color::Reset if foreground && pixel => Some(self.lit),
            Color::Reset if foreground => Some(self.text),
            Color::Reset | Color::Black => None,
            Color::DarkGray if pixel || dim => Some(self.ghost),
            Color::DarkGray => Some(self.label),
            Color::Yellow if pixel => named(0xe8bd6a, 0x9a6a1e),
            Color::Yellow => Some(self.accent),
            Color::LightYellow => named(0xf3d99a, 0xb58a3a),
            Color::Red | Color::LightRed | Color::Green if !pixel => Some(self.second),
            Color::Red | Color::LightRed => named(0xdd7357, 0x9a3f22),
            Color::Magenta | Color::LightMagenta => named(0x8a5a6a, 0x6a3a48),
            Color::Blue => named(0x3d4a6a, 0x3a4660),
            Color::LightBlue | Color::Cyan | Color::LightCyan => named(0x8fb0d0, 0x4a6a8a),
            Color::Green | Color::LightGreen => named(0x8a9a5a, 0x5a6a2a),
            Color::Gray | Color::White => Some(self.label),
            Color::Rgb(r, g, b) if self.warm => {
                let luma = Rgb(r, g, b).luma();
                match luma {
                    l if l > 0.8 => Some(Rgb::hex(0xbea873).mix(self.label, 0.2)),
                    l if l > 0.55 => Some(self.label),
                    l if l > 0.3 => Some(self.text),
                    _ => None,
                }
            }
            Color::Rgb(r, g, b) => Some(Rgb(r, g, b)),
            Color::Indexed(_) => Some(self.text),
        }
    }
}
