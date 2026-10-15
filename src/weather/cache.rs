//! The last good forecast on disk, so alba shows weather at once on the next start.

use std::{fs, io, path::Path};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use super::{Forecast, ResolvedLocation, Weather};
use crate::config::{Coordinates, Location};

/// Bump this when the file's shape changes.
const FORMAT_VERSION: u32 = 2;

const FILE_NAME: &str = "weather.json";

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    /// Unix seconds.
    fetched_at: i64,
    location: CachedLocation,
    forecast: Forecast,
}

/// Keeps the resolved location with the configured name, so a name is looked up only once.
#[derive(Serialize, Deserialize)]
struct CachedLocation {
    /// The configured `location`, or None for configured coordinates.
    name: Option<String>,
    place_name: Option<String>,
    latitude: f64,
    longitude: f64,
}

impl CachedLocation {
    fn new(location: &Location, resolved_location: &ResolvedLocation) -> Self {
        Self {
            name: match location {
                Location::Name(name) => Some(name.clone()),
                Location::Coordinates(_) => None,
            },
            place_name: resolved_location.place_name.clone(),
            latitude: resolved_location.coordinates.latitude,
            longitude: resolved_location.coordinates.longitude,
        }
    }

    /// The cached coordinates, when they are valid and for the configured location.
    fn coordinates_for(&self, location: &Location) -> Option<Coordinates> {
        let coordinates = Coordinates::try_from(vec![self.latitude, self.longitude]).ok()?;
        let matches = match (location, &self.name) {
            (Location::Name(configured), Some(name)) => configured == name,
            (Location::Coordinates(configured), None) => *configured == coordinates,
            _ => false,
        };
        matches.then_some(coordinates)
    }
}

/// The cached weather and where it is for, when it is for the configured location.
pub fn read(directory: &Path, location: &Location) -> Option<(Weather, ResolvedLocation)> {
    let text = fs::read_to_string(directory.join(FILE_NAME)).ok()?;
    usable(&text, location)
}

fn usable(text: &str, location: &Location) -> Option<(Weather, ResolvedLocation)> {
    let file: CacheFile = serde_json::from_str(text).ok()?;
    if file.version != FORMAT_VERSION {
        return None;
    }
    let coordinates = file.location.coordinates_for(location)?;
    let weather = Weather {
        fetched_at: Timestamp::from_second(file.fetched_at).ok()?,
        forecast: file.forecast,
    };
    let resolved_location = ResolvedLocation {
        coordinates,
        place_name: file.location.place_name,
    };
    Some((weather, resolved_location))
}

/// Writes a temporary file, then renames it into place, so a reader never sees a partial file.
pub fn write(
    directory: &Path,
    location: &Location,
    resolved_location: &ResolvedLocation,
    weather: &Weather,
) -> io::Result<()> {
    let file = CacheFile {
        version: FORMAT_VERSION,
        fetched_at: weather.fetched_at.as_second(),
        location: CachedLocation::new(location, resolved_location),
        forecast: weather.forecast.clone(),
    };
    let text = serde_json::to_string(&file)?;
    fs::create_dir_all(directory)?;
    let temporary = directory.join(format!(".{FILE_NAME}.{}", std::process::id()));
    let written = fs::write(&temporary, text)
        .and_then(|()| fs::rename(&temporary, directory.join(FILE_NAME)));
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written
}
