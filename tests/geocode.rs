mod common;

use alba::{
    config::Coordinates,
    geocode::{self, Place},
    weather::FetchError,
};
use common::geocoding_body;
use serde_json::{Value, json};

fn buenos_aires() -> Value {
    serde_json::from_str(&geocoding_body()).unwrap()
}

#[test]
fn first_result_gives_the_lowercased_place_name_and_coordinates() {
    let mut body = buenos_aires();
    body["results"].as_array_mut().unwrap().push(json!({
        "name": "Buenos Aires",
        "latitude": 9.17,
        "longitude": -83.33,
        "country_code": "CR",
    }));

    assert_eq!(
        geocode::parse(&body.to_string()),
        Ok(Some(Place {
            name: "buenos aires, ar".to_string(),
            coordinates: Coordinates {
                latitude: -34.61315,
                longitude: -58.37723,
            },
        }))
    );
}

#[test]
fn response_without_results_is_no_match() {
    for body in [
        json!({ "generationtime_ms": 0.5 }),
        json!({ "results": [], "generationtime_ms": 0.5 }),
    ] {
        assert_eq!(geocode::parse(&body.to_string()), Ok(None), "{body}");
    }
}

#[test]
fn place_name_keeps_letters_outside_ascii_and_drops_control_characters() {
    let mut body = buenos_aires();
    body["results"][0]["name"] = json!("Gött\u{7}ingen\n");
    body["results"][0]["country_code"] = json!("DE");

    let place = geocode::parse(&body.to_string()).unwrap().unwrap();

    assert_eq!(place.name, "göttingen, de");
}

#[test]
fn result_without_a_country_code_shows_the_name_alone() {
    let mut body = buenos_aires();
    body["results"][0]
        .as_object_mut()
        .unwrap()
        .remove("country_code");

    let place = geocode::parse(&body.to_string()).unwrap().unwrap();

    assert_eq!(place.name, "buenos aires");
}

#[test]
fn malformed_responses_are_invalid() {
    let mut out_of_range = buenos_aires();
    out_of_range["results"][0]["latitude"] = json!(123.0);
    let mut no_name = buenos_aires();
    no_name["results"][0]
        .as_object_mut()
        .unwrap()
        .remove("name");
    let bodies = [
        ("html", "<html>bad gateway</html>".to_string()),
        ("empty", String::new()),
        ("latitude out of range", out_of_range.to_string()),
        ("no name", no_name.to_string()),
    ];

    for (case, body) in bodies {
        assert!(
            matches!(geocode::parse(&body), Err(FetchError::Invalid(_))),
            "{case}"
        );
    }
}
