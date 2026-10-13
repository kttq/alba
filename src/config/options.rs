//! The value of each config option that takes one of a few choices.

use serde::Deserialize;
use serde_repr::Deserialize_repr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Es,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    #[default]
    Metric,
    Imperial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize_repr)]
#[repr(u8)]
pub enum HourFormat {
    #[default]
    TwentyFour = 24,
    Twelve = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
}

/// Segment thickness in pixels, for the Large tier only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize_repr)]
#[repr(u8)]
pub enum Thickness {
    One = 1,
    #[default]
    Two = 2,
}

/// Pixels between digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize_repr)]
#[repr(u8)]
pub enum DigitGap {
    #[default]
    One = 1,
    Two = 2,
}

/// Pixels on each side of the colon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(from = "ColonGapValue")]
pub enum ColonGap {
    /// Thickness + 1.
    #[default]
    Auto,
    Pixels(ColonGapPixels),
}

/// `colon_gap` as written in TOML: the string `"auto"` or an integer.
#[derive(Deserialize)]
#[serde(untagged, expecting = r#"expected "auto" or 1 to 4"#)]
enum ColonGapValue {
    Auto(AutoKeyword),
    Pixels(ColonGapPixels),
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum AutoKeyword {
    Auto,
}

impl From<ColonGapValue> for ColonGap {
    fn from(value: ColonGapValue) -> Self {
        match value {
            ColonGapValue::Auto(AutoKeyword::Auto) => Self::Auto,
            ColonGapValue::Pixels(pixels) => Self::Pixels(pixels),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr)]
#[repr(u8)]
pub enum ColonGapPixels {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
}

/// The off-beat style of the colon, in the second half of each second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColonStyle {
    #[default]
    Ghost,
    Blank,
    Steady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColonDots {
    #[default]
    Square,
    Tall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AmPm {
    #[default]
    Stack,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize_repr)]
#[repr(u8)]
pub enum AnimationFps {
    Still = 0,
    One = 1,
    #[default]
    Two = 2,
    Four = 4,
}

/// How often a thunderstorm flashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum Flash {
    #[default]
    #[serde(rename = "8s")]
    EightSeconds,
    #[serde(rename = "20s")]
    TwentySeconds,
    #[serde(rename = "off")]
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlashTarget {
    #[default]
    Cloud,
    Bolt,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoonDarkSide {
    #[default]
    Faint,
    Solid,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stars {
    #[default]
    Glyphs,
    Pixels,
    None,
}
