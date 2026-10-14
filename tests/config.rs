mod common;

use std::{fs, path::PathBuf};

use alba::config::{self, Config, Coordinates, HourFormat, Language, Location, Units, WeekStart};
use common::make_read_only;

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
            ghost_segments: true,
            motion: true,
        }
    );
}

#[test]
fn full_schema_parses_every_option() {
    let text = r#"
language       = "es"
units          = "imperial"
hour_format    = 12
week_start     = "sunday"
location       = "buenos aires"
ghost_segments = false
motion         = false
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
            ghost_segments: false,
            motion: false,
        }
    );
}

#[test]
fn coordinates_win_over_location() {
    let text = "location = \"paris\"\ncoordinates = [-34.6037, -58.3816]\n";

    let config = Config::parse(text).unwrap();

    assert_eq!(
        config.location,
        Some(Location::Coordinates(Coordinates {
            latitude: -34.6037,
            longitude: -58.3816,
        }))
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
fn dropped_table_names_its_line() {
    assert_error(
        "motion = true\n\n[clock]\ncolon = \"steady\"\n",
        3,
        "unknown field `clock`",
    );
}

#[test]
fn bad_choice_names_its_line() {
    assert_error("language = \"fr\"\n", 1, "unknown variant `fr`");
}

#[test]
fn bad_hour_format_names_its_line() {
    assert_error(
        "\nhour_format = 13\n",
        2,
        "invalid value: 13, expected 24 or 12",
    );
}

#[test]
fn wrong_type_names_its_line() {
    assert_error(
        "units = \"metric\"\nmotion = \"no\"\n",
        2,
        "expected a boolean",
    );
}

#[test]
fn coordinate_out_of_range_names_its_line() {
    assert_error(
        "\ncoordinates = [91, 0]\n",
        2,
        "latitude must be -90 to 90, found 91",
    );
    assert_error(
        "coordinates = [0, -180.5]\n",
        1,
        "longitude must be -180 to 180, found -180.5",
    );
}

#[test]
fn coordinates_that_are_not_a_pair_name_their_line() {
    let reason = "coordinates must be [latitude, longitude]";
    assert_error("units = \"metric\"\ncoordinates = [48.8]\n", 2, reason);
    assert_error("\n\ncoordinates = [48.8, 2.3, 99]\n", 3, reason);
}

#[test]
fn config_without_a_location_is_not_an_error() {
    assert_eq!(
        Config::parse("units = \"imperial\"\n").unwrap().location,
        None
    );
}

/// The spec's first-run template, copied by hand so the test does not read it from the code.
const SPEC_TEMPLATE: &str = r#"# alba config. docs: https://github.com/kttq/alba/blob/main/docs/config.md
# language: "en" or "es". units: "metric" or "imperial".
# hour_format: 24 or 12. week_start: "monday" or "sunday".

language    = "en"
units       = "metric"
hour_format = 24
week_start  = "monday"

# location: your city, optionally narrowed, like "paris, united states".
# check a name at https://open-meteo.com/en/docs/geocoding-api
# or set coordinates instead, as [latitude, longitude]; they win over location.

# location    = "buenos aires"
# coordinates = [-34.6037, -58.3816]

# ghost_segments: show unlit clock segments faintly.
# turn it off if your terminal has no faint style.
# motion: false stops the blinking colon and all animation.

# ghost_segments = true
# motion         = true
"#;

/// A fresh, empty directory standing in for `HOME`.
fn temporary_home(test: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("alba-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    home
}

#[test]
fn missing_config_writes_the_template_and_loads_defaults() {
    let home = temporary_home("missing");
    let path = home.join(".config/alba/config.toml");

    let loaded = config::load(&path).unwrap();

    assert_eq!(loaded, Config::default());
    assert_eq!(fs::read_to_string(&path).unwrap(), SPEC_TEMPLATE);
}

#[test]
fn existing_config_is_never_overwritten() {
    let home = temporary_home("existing");
    let path = home.join(".config/alba/config.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "units = \"imperial\"\n").unwrap();

    let loaded = config::load(&path).unwrap();

    assert_eq!(loaded.units, Units::Imperial);
    assert_eq!(fs::read_to_string(&path).unwrap(), "units = \"imperial\"\n");
}

#[test]
fn failed_template_write_still_loads_defaults() {
    let home = temporary_home("unwritable");
    let path = home.join(".config/alba/config.toml");
    let directory = path.parent().unwrap();
    fs::create_dir_all(directory).unwrap();
    make_read_only(directory);

    let loaded = config::load(&path).unwrap();

    assert_eq!(loaded, Config::default());
    assert!(!path.exists());
}

#[test]
fn config_that_is_not_utf8_has_no_line() {
    let home = temporary_home("not-utf8");
    let path = home.join(".config/alba/config.toml");
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
    let path = home.join(".config/alba/config.toml");
    fs::create_dir_all(&path).unwrap();

    let message = config::load(&path).unwrap_err().to_string();

    assert!(
        message.starts_with("config error: cannot read ") && !message.contains("line"),
        "{message}"
    );
}
