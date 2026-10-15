//! Writes a config and a weather cache into a scratch HOME for one verification run.

#[allow(dead_code, reason = "the tests use the parts this example does not")]
#[path = "../tests/common/fixture_home.rs"]
mod fixture_home;

use std::{env, path::PathBuf, process::ExitCode};

use fixture_home::{CachedWeather, FixtureHome};
use jiff::{SignedDuration, Timestamp};

const USAGE: &str = "\
usage: fixture HOME [--weather fresh|stale|expired|none] [--age MINUTES] [--code WMO]
                    [--night] [--no-location] [--location NAME] [--config 'key = value' ...]

The cache is tests/fixtures/forecast.json (Buenos Aires, rain, 14°, day), shifted so its
middle day is today. Default ages: fresh 5 minutes, stale 90 minutes, expired 13 hours.
--weather none writes no cache. --night sets the current is_day to 0. --location sets the
config's location to NAME and caches it as resolved to buenos aires, ar, so alba needs no
lookup. --config adds a line.";

fn main() -> ExitCode {
    if env::args().any(|argument| argument == "-h" || argument == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let (home, fixture_home) = match parse(env::args().skip(1)) {
        Ok(parsed) => parsed,
        Err(reason) => {
            eprintln!("fixture: {reason}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if let Err(error) = fixture_home.write(&home, Timestamp::now()) {
        eprintln!("fixture: {error}");
        return ExitCode::FAILURE;
    }
    println!("{}", home.display());
    ExitCode::SUCCESS
}

fn parse(mut arguments: impl Iterator<Item = String>) -> Result<(PathBuf, FixtureHome), String> {
    let mut home = None;
    let mut fixture_home = FixtureHome::default();
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--weather" => fixture_home.cached_weather = cached_weather(&value()?)?,
            "--age" => fixture_home.age = Some(minutes(&value()?)?),
            "--code" => fixture_home.weather_code = Some(number(&value()?)?),
            "--night" => fixture_home.night = true,
            "--no-location" => fixture_home.has_location = false,
            "--location" => fixture_home.location_name = Some(value()?),
            "--config" => fixture_home.config_lines.push(value()?),
            option if option.starts_with("--") => return Err(format!("unknown option {option}")),
            _ if home.is_none() => home = Some(PathBuf::from(argument)),
            extra => return Err(format!("unexpected argument {extra:?}")),
        }
    }
    Ok((home.ok_or("HOME is missing")?, fixture_home))
}

fn cached_weather(text: &str) -> Result<CachedWeather, String> {
    match text {
        "fresh" => Ok(CachedWeather::Fresh),
        "stale" => Ok(CachedWeather::Stale),
        "expired" => Ok(CachedWeather::Expired),
        "none" => Ok(CachedWeather::NoCache),
        _ => Err(format!(
            "--weather is fresh, stale, expired, or none, not {text:?}"
        )),
    }
}

fn minutes(text: &str) -> Result<SignedDuration, String> {
    Ok(SignedDuration::from_mins(number(text)?))
}

fn number<T: std::str::FromStr>(text: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("{text:?} is not a whole number"))
}
