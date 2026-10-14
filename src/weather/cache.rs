//! The last good forecast on disk, so alba shows weather at once on the next start.

use std::{fs, io, path::Path};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use super::{Forecast, Weather};
use crate::config::Coordinates;

/// Bump this when the file's shape changes. An old file then counts as no cache.
const FORMAT_VERSION: u32 = 1;

const FILE_NAME: &str = "weather.json";

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    /// Unix seconds.
    fetched_at: i64,
    location: CachedLocation,
    forecast: Forecast,
}

/// The coordinates the forecast was fetched for.
#[derive(PartialEq, Serialize, Deserialize)]
struct CachedLocation {
    latitude: f64,
    longitude: f64,
}

impl From<Coordinates> for CachedLocation {
    fn from(coordinates: Coordinates) -> Self {
        Self {
            latitude: coordinates.latitude,
            longitude: coordinates.longitude,
        }
    }
}

/// The cached weather for `coordinates`. A missing or unreadable file counts as no cache.
pub fn read(directory: &Path, coordinates: Coordinates) -> Option<Weather> {
    let text = fs::read_to_string(directory.join(FILE_NAME)).ok()?;
    usable(&text, coordinates)
}

/// A malformed file, another format version, or another location all count as no cache.
fn usable(text: &str, coordinates: Coordinates) -> Option<Weather> {
    let file: CacheFile = serde_json::from_str(text).ok()?;
    if file.version != FORMAT_VERSION || file.location != CachedLocation::from(coordinates) {
        return None;
    }
    file.forecast.validate().ok()?;
    Some(Weather {
        fetched_at: Timestamp::from_second(file.fetched_at).ok()?,
        forecast: file.forecast,
    })
}

/// Writes a temporary file, then renames it into place, so a reader never sees a partial file.
/// Creates `directory` when missing.
pub fn write(directory: &Path, coordinates: Coordinates, weather: &Weather) -> io::Result<()> {
    let file = CacheFile {
        version: FORMAT_VERSION,
        fetched_at: weather.fetched_at.as_second(),
        location: coordinates.into(),
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
