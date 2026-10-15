use serde::Deserialize;

use crate::{
    config::{Coordinates, Language},
    weather::{FetchError, open_meteo},
};

const SEARCH_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// Lowercased, followed by a comma and the country code when geocoding sends one.
    pub name: String,
    pub coordinates: Coordinates,
}

/// Blocks until the response body arrives or the request fails. alba uses only the first
/// result, so it asks for one.
pub fn look_up(name: &str, language: Language) -> Result<String, FetchError> {
    let language = match language {
        Language::En => "en",
        Language::Es => "es",
    };
    open_meteo::get(
        SEARCH_URL,
        &[
            ("name", name),
            ("count", "1"),
            ("language", language),
            ("format", "json"),
        ],
    )
}

#[derive(Deserialize)]
struct Response {
    /// Open-Meteo leaves the key out when nothing matches.
    #[serde(default)]
    results: Vec<Match>,
}

#[derive(Deserialize)]
struct Match {
    name: String,
    latitude: f64,
    longitude: f64,
    country_code: Option<String>,
}

/// The first result, or None when the service found no match.
pub fn parse(body: &str) -> Result<Option<Place>, FetchError> {
    let response: Response =
        serde_json::from_str(body).map_err(|error| FetchError::Invalid(error.to_string()))?;
    let Some(first) = response.results.into_iter().next() else {
        return Ok(None);
    };
    let coordinates = Coordinates::try_from(vec![first.latitude, first.longitude])
        .map_err(FetchError::Invalid)?;
    let name = match first.country_code {
        Some(country_code) => format!("{}, {}", first.name, country_code),
        None => first.name,
    };
    Ok(Some(Place {
        name: shown_text(&name),
        coordinates,
    }))
}

/// The name comes from the API, so a control character could move the cursor or recolor the
/// pane.
fn shown_text(name: &str) -> String {
    name.chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .to_lowercase()
}
