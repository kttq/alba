use std::{
    fmt, fs, io,
    marker::PhantomData,
    ops::Range,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer, de};
use toml::Spanned;

/// Written on first run, when no config file exists.
pub const TEMPLATE: &str = r#"# alba config. all options: https://github.com/kttq/alba#config
# language: "en" or "es". units: "metric" or "imperial".
# hour_format: 24 or 12. week_start: "monday" or "sunday".

language    = "en"
units       = "metric"
hour_format = 24
week_start  = "monday"

# location: your city, optionally narrowed, like "paris, united states".
# check a name at https://open-meteo.com/en/docs/geocoding-api
# or set latitude and longitude instead; they win over location.

# location  = "buenos aires"
# latitude  = -34.6037
# longitude = -58.3816

# to stop all motion, uncomment:

# [clock]
# colon = "steady"
# [weather]
# animation_fps = 0
# flash = "off"
"#;

/// `XDG_CONFIG_HOME` is ignored on purpose: the path is the same on macOS and Linux.
pub fn path(home: &Path) -> PathBuf {
    home.join(".config/alba/config.toml")
}

/// Reads the config at `path`. With no file there, writes the template and returns the defaults.
/// A failed template write is ignored, so alba still runs.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let _ = write_template(path);
            return Ok(Config::default());
        }
        Err(error) => {
            return Err(ConfigError::Unreadable {
                reason: format!("cannot read {}: {error}", path.display()),
            });
        }
    };
    let text = String::from_utf8(bytes).map_err(|_| ConfigError::Unreadable {
        reason: format!("{} is not valid UTF-8", path.display()),
    })?;
    Config::parse(&text)
}

/// Writes a temporary file, then links it into place, so an interrupted write never leaves a
/// partial config. The link fails if a file appeared since the read, so nothing is overwritten.
fn write_template(path: &Path) -> io::Result<()> {
    let directory = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(directory)?;
    let temporary = directory.join(format!(".config.toml.{}", std::process::id()));
    let written = fs::write(&temporary, TEMPLATE).and_then(|()| fs::hard_link(&temporary, path));
    let _ = fs::remove_file(&temporary);
    written
}

/// The settings from `config.toml`, parsed and validated.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub language: Language,
    pub units: Units,
    pub hour_format: HourFormat,
    pub week_start: WeekStart,
    pub location: Option<Location>,
    pub clock: ClockConfig,
    pub weather: WeatherConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    Name(String),
    Coordinates { latitude: f64, longitude: f64 },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ClockConfig {
    pub thickness: Thickness,
    pub digit_gap: DigitGap,
    pub colon_gap: ColonGap,
    pub colon: ColonStyle,
    pub colon_dots: ColonDots,
    pub ghost_segments: bool,
    pub tails: bool,
    pub hooked_seven: bool,
    pub am_pm: AmPm,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            thickness: Thickness::default(),
            digit_gap: DigitGap::default(),
            colon_gap: ColonGap::default(),
            colon: ColonStyle::default(),
            colon_dots: ColonDots::default(),
            ghost_segments: true,
            tails: true,
            hooked_seven: false,
            am_pm: AmPm::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WeatherConfig {
    pub animation_fps: AnimationFps,
    pub flash: Flash,
    pub flash_target: FlashTarget,
    pub moon_dark_side: MoonDarkSide,
    pub stars: Stars,
}

/// An option that takes one of a few strings. Unlike a derived enum, it rejects a table.
macro_rules! string_choice {
    ($(#[$meta:meta])* $name:ident { $($(#[$variant_meta:meta])* $variant:ident = $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub enum $name {
            $($(#[$variant_meta])* $variant),+
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                const VALUES: &[&str] = &[$($value),+];
                let value = deserializer.deserialize_any(StringChoiceVisitor(VALUES))?;
                match value.as_str() {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(de::Error::unknown_variant(&value, VALUES)),
                }
            }
        }
    };
}

struct StringChoiceVisitor(&'static [&'static str]);

impl de::Visitor<'_> for StringChoiceVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let values: Vec<_> = self.0.iter().map(|value| format!("`{value}`")).collect();
        write!(formatter, "one of {}", values.join(", "))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<String, E> {
        Ok(value.to_string())
    }
}

string_choice!(Language { #[default] En = "en", Es = "es" });

string_choice!(Units { #[default] Metric = "metric", Imperial = "imperial" });

string_choice!(WeekStart { #[default] Monday = "monday", Sunday = "sunday" });

string_choice!(
    /// The off-beat style of the colon, in the second half of each second.
    ColonStyle { #[default] Ghost = "ghost", Blank = "blank", Steady = "steady" }
);

string_choice!(ColonDots { #[default] Square = "square", Tall = "tall" });

string_choice!(AmPm { #[default] Stack = "stack", Text = "text" });

string_choice!(
    /// How often a thunderstorm flashes.
    Flash { #[default] EightSeconds = "8s", TwentySeconds = "20s", Off = "off" }
);

string_choice!(FlashTarget { #[default] Cloud = "cloud", Bolt = "bolt", Both = "both" });

string_choice!(MoonDarkSide { #[default] Faint = "faint", Solid = "solid", None = "none" });

string_choice!(Stars { #[default] Glyphs = "glyphs", Pixels = "pixels", None = "none" });

/// An option that takes one of a few integers. Each variant's discriminant is its value.
macro_rules! integer_choice {
    ($(#[$meta:meta])* $name:ident, $expected:literal { $($(#[$variant_meta:meta])* $variant:ident = $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
        #[serde(try_from = "i64")]
        #[repr(u8)]
        pub enum $name {
            $($(#[$variant_meta])* $variant = $value),+
        }

        impl TryFrom<i64> for $name {
            type Error = String;

            fn try_from(value: i64) -> Result<Self, String> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(format!("expected {}, found {value}", $expected)),
                }
            }
        }
    };
}

integer_choice!(HourFormat, "24 or 12" {
    #[default] TwentyFour = 24,
    Twelve = 12,
});

integer_choice!(
    /// Segment thickness in pixels, for the Large tier only.
    Thickness, "1 or 2" { One = 1, #[default] Two = 2 }
);

integer_choice!(
    /// Pixels between digits.
    DigitGap, "1 or 2" { #[default] One = 1, Two = 2 }
);

integer_choice!(AnimationFps, "0, 1, 2, or 4" {
    Still = 0,
    One = 1,
    #[default] Two = 2,
    Four = 4,
});

/// Pixels on each side of the colon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColonGap {
    /// Thickness + 1.
    #[default]
    Auto,
    Pixels(u8),
}

impl<'de> Deserialize<'de> for ColonGap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ColonGapVisitor;

        impl de::Visitor<'_> for ColonGapVisitor {
            type Value = ColonGap;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("\"auto\" or 1 to 4")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<ColonGap, E> {
                match value {
                    "auto" => Ok(ColonGap::Auto),
                    _ => Err(E::invalid_value(de::Unexpected::Str(value), &self)),
                }
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<ColonGap, E> {
                match value {
                    1..=4 => Ok(ColonGap::Pixels(value as u8)),
                    _ => Err(E::invalid_value(de::Unexpected::Signed(value), &self)),
                }
            }
        }

        deserializer.deserialize_any(ColonGapVisitor)
    }
}

/// Degrees, checked against a range on parse.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "f64")]
struct Latitude(f64);

impl TryFrom<f64> for Latitude {
    type Error = String;

    fn try_from(degrees: f64) -> Result<Self, String> {
        if (-90.0..=90.0).contains(&degrees) {
            Ok(Self(degrees))
        } else {
            Err(format!("latitude must be -90 to 90, found {degrees}"))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "f64")]
struct Longitude(f64);

impl TryFrom<f64> for Longitude {
    type Error = String;

    fn try_from(degrees: f64) -> Result<Self, String> {
        if (-180.0..=180.0).contains(&degrees) {
            Ok(Self(degrees))
        } else {
            Err(format!("longitude must be -180 to 180, found {degrees}"))
        }
    }
}

/// The file's shape, before the location settings combine into one `Location`.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ConfigFile {
    language: Language,
    units: Units,
    hour_format: HourFormat,
    week_start: WeekStart,
    location: Option<String>,
    latitude: Option<Spanned<Latitude>>,
    longitude: Option<Spanned<Longitude>>,
    #[serde(deserialize_with = "table")]
    clock: ClockConfig,
    #[serde(deserialize_with = "table")]
    weather: WeatherConfig,
}

/// A derived struct also accepts an array, filling its fields by position.
fn table<'de, D: Deserializer<'de>, T: Deserialize<'de>>(deserializer: D) -> Result<T, D::Error> {
    struct TableVisitor<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> de::Visitor<'de> for TableVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a table")
        }

        fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_map(TableVisitor(PhantomData))
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let file: ConfigFile = toml::from_str(text).map_err(|error| ConfigError::Invalid {
            line: line_of(text, error.span()),
            reason: error.message().to_string(),
        })?;
        let location = match (file.latitude, file.longitude) {
            (Some(latitude), Some(longitude)) => Some(Location::Coordinates {
                latitude: latitude.into_inner().0,
                longitude: longitude.into_inner().0,
            }),
            (Some(latitude), None) => {
                return Err(ConfigError::Invalid {
                    line: line_of(text, Some(latitude.span())),
                    reason: "latitude is set without longitude".to_string(),
                });
            }
            (None, Some(longitude)) => {
                return Err(ConfigError::Invalid {
                    line: line_of(text, Some(longitude.span())),
                    reason: "longitude is set without latitude".to_string(),
                });
            }
            (None, None) => file.location.map(Location::Name),
        };
        Ok(Self {
            language: file.language,
            units: file.units,
            hour_format: file.hour_format,
            week_start: file.week_start,
            location,
            clock: file.clock,
            weather: file.weather,
        })
    }
}

/// 1-based. A span-less error points at line 1.
fn line_of(text: &str, span: Option<Range<usize>>) -> usize {
    let start = span.map_or(0, |span| span.start).min(text.len());
    text.as_bytes()[..start]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// A syntax error, unknown key, or bad value.
    Invalid { line: usize, reason: String },
    /// The file exists but cannot be read as text.
    Unreadable { reason: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Invalid { line, reason } => {
                write!(formatter, "config error: line {line}: {reason}")
            }
            Self::Unreadable { reason } => write!(formatter, "config error: {reason}"),
        }
    }
}
