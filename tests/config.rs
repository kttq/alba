use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use alba::config::{
    self, AmPm, AnimationFps, ClockConfig, ColonDots, ColonGap, ColonStyle, Config, DigitGap,
    Flash, FlashTarget, HourFormat, Language, Location, MoonDarkSide, Stars, Thickness, Units,
    WeatherConfig, WeekStart,
};

#[test]
fn empty_config_has_every_default() {
    let config = Config::parse("").unwrap();

    assert_eq!(
        config,
        Config {
            language: Language::En,
            units: Units::Metric,
            hour_format: HourFormat::TwentyFour,
            week_start: WeekStart::Monday,
            location: None,
            clock: ClockConfig {
                thickness: Thickness::Two,
                digit_gap: DigitGap::One,
                colon_gap: ColonGap::Auto,
                colon: ColonStyle::Ghost,
                colon_dots: ColonDots::Square,
                ghost_segments: true,
                tails: true,
                hooked_seven: false,
                am_pm: AmPm::Stack,
            },
            weather: WeatherConfig {
                animation_fps: AnimationFps::Two,
                flash: Flash::EightSeconds,
                flash_target: FlashTarget::Cloud,
                moon_dark_side: MoonDarkSide::Faint,
                stars: Stars::Glyphs,
            },
        }
    );
}

#[test]
fn full_schema_parses_every_option() {
    let text = r#"
language    = "es"
units       = "imperial"
hour_format = 12
week_start  = "sunday"
location    = "buenos aires"

[clock]
thickness      = 1
digit_gap      = 2
colon_gap      = 4
colon          = "steady"
colon_dots     = "tall"
ghost_segments = false
tails          = false
hooked_seven   = true
am_pm          = "text"

[weather]
animation_fps  = 4
flash          = "off"
flash_target   = "both"
moon_dark_side = "none"
stars          = "pixels"
"#;

    let config = Config::parse(text).unwrap();

    assert_eq!(
        config,
        Config {
            language: Language::Es,
            units: Units::Imperial,
            hour_format: HourFormat::Twelve,
            week_start: WeekStart::Sunday,
            location: Some(Location::Name("buenos aires".to_string())),
            clock: ClockConfig {
                thickness: Thickness::One,
                digit_gap: DigitGap::Two,
                colon_gap: ColonGap::Pixels(4),
                colon: ColonStyle::Steady,
                colon_dots: ColonDots::Tall,
                ghost_segments: false,
                tails: false,
                hooked_seven: true,
                am_pm: AmPm::Text,
            },
            weather: WeatherConfig {
                animation_fps: AnimationFps::Four,
                flash: Flash::Off,
                flash_target: FlashTarget::Both,
                moon_dark_side: MoonDarkSide::None,
                stars: Stars::Pixels,
            },
        }
    );
}

#[test]
fn coordinates_win_over_location() {
    let text = "location = \"paris\"\nlatitude = -34.6037\nlongitude = -58.3816\n";

    let config = Config::parse(text).unwrap();

    assert_eq!(
        config.location,
        Some(Location::Coordinates {
            latitude: -34.6037,
            longitude: -58.3816,
        })
    );
}

/// Asserts the message names `line` and contains `reason`, which comes partly from the TOML parser.
fn assert_error(text: &str, line: usize, reason: &str) {
    let message = Config::parse(text).unwrap_err().to_string();
    let prefix = format!("config error: line {line}: ");
    assert!(
        message.starts_with(&prefix) && message.contains(reason),
        "expected '{prefix}…{reason}…', got '{message}'"
    );
}

#[test]
fn syntax_error_names_its_line() {
    assert_error("units = \"metric\"\nlanguage = \"en\n", 2, "");
}

#[test]
fn unknown_key_names_its_line() {
    assert_error(
        "units = \"metric\"\nunit = \"metric\"\n",
        2,
        "unknown field `unit`",
    );
}

#[test]
fn unknown_key_in_a_table_names_its_line() {
    assert_error(
        "[clock]\ncolon = \"ghost\"\ncolour = 1\n",
        3,
        "unknown field `colour`",
    );
}

#[test]
fn unknown_table_names_its_line() {
    assert_error(
        "language = \"en\"\n\n[theme]\nx = 1\n",
        3,
        "unknown field `theme`",
    );
}

#[test]
fn bad_choice_names_its_line() {
    assert_error("language = \"fr\"\n", 1, "unknown variant `fr`");
}

#[test]
fn bad_integer_choice_names_its_line() {
    assert_error("\nhour_format = 13\n", 2, "expected 24 or 12, found 13");
    assert_error(
        "[weather]\nanimation_fps = 3\n",
        2,
        "expected 0, 1, 2, or 4, found 3",
    );
}

#[test]
fn bad_colon_gap_names_its_line() {
    assert_error("[clock]\ncolon_gap = 5\n", 2, "expected \"auto\" or 1 to 4");
    assert_error(
        "[clock]\ncolon_gap = \"wide\"\n",
        2,
        "expected \"auto\" or 1 to 4",
    );
}

#[test]
fn wrong_type_names_its_line() {
    assert_error("[clock]\nghost_segments = \"yes\"\n", 2, "");
}

#[test]
fn coordinate_out_of_range_names_its_line() {
    assert_error(
        "latitude = 91\nlongitude = 0\n",
        1,
        "latitude must be -90 to 90",
    );
    assert_error(
        "latitude = 0\nlongitude = -180.5\n",
        2,
        "longitude must be -180 to 180",
    );
}

#[test]
fn latitude_without_longitude_names_its_line() {
    assert_error(
        "location = \"paris\"\nlatitude = 48.8\n",
        2,
        "latitude is set without longitude",
    );
}

#[test]
fn longitude_without_latitude_names_its_line() {
    assert_error(
        "\n\nlongitude = 2.3\n",
        3,
        "longitude is set without latitude",
    );
}

#[test]
fn config_without_a_location_is_not_an_error() {
    assert_eq!(
        Config::parse("units = \"imperial\"\n").unwrap().location,
        None
    );
}

/// The spec's first-run template, copied by hand so the test does not read it from the code.
const SPEC_TEMPLATE: &str = r#"# alba config. all options: https://github.com/kttq/alba#config
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

/// A fresh, empty directory standing in for `HOME`.
fn temporary_home(test: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("alba-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    home
}

#[test]
fn config_lives_under_home() {
    assert_eq!(
        config::path(&PathBuf::from("/home/ana")),
        PathBuf::from("/home/ana/.config/alba/config.toml")
    );
}

#[test]
fn missing_config_writes_the_template_and_loads_defaults() {
    let home = temporary_home("missing");
    let path = config::path(&home);

    let loaded = config::load(&path).unwrap();

    assert_eq!(loaded, Config::default());
    assert_eq!(fs::read_to_string(&path).unwrap(), SPEC_TEMPLATE);
}

#[test]
fn existing_config_is_never_overwritten() {
    let home = temporary_home("existing");
    let path = config::path(&home);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "units = \"imperial\"\n").unwrap();

    let loaded = config::load(&path).unwrap();

    assert_eq!(loaded.units, Units::Imperial);
    assert_eq!(fs::read_to_string(&path).unwrap(), "units = \"imperial\"\n");
}

#[test]
fn failed_template_write_still_loads_defaults() {
    let home = temporary_home("unwritable");
    let path = config::path(&home);
    let directory = path.parent().unwrap();
    fs::create_dir_all(directory).unwrap();
    fs::set_permissions(directory, fs::Permissions::from_mode(0o555)).unwrap();

    let loaded = config::load(&path).unwrap();

    assert_eq!(loaded, Config::default());
    assert!(!path.exists());
}

#[test]
fn config_that_is_not_utf8_has_no_line() {
    let home = temporary_home("not-utf8");
    let path = config::path(&home);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"language = \"\xff\"\n").unwrap();

    let message = config::load(&path).unwrap_err().to_string();

    assert_eq!(
        message,
        format!("config error: {} is not valid UTF-8", path.display())
    );
}

#[test]
fn config_that_is_a_directory_is_unreadable() {
    let home = temporary_home("directory");
    let path = config::path(&home);
    fs::create_dir_all(&path).unwrap();

    let message = config::load(&path).unwrap_err().to_string();

    assert!(
        message.starts_with("config error: cannot read ") && !message.contains("line"),
        "{message}"
    );
}
