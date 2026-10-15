mod common;

use alba::{
    config::Config,
    face::{Face, FaceSettings},
    icon::Icon,
    weather::{Forecast, ShownWeather, Weather, WeatherState},
};
use common::{default_instant, forecast_body};
use jiff::ToSpan;
use ratatui::text::Line;

fn settings(config_text: &str) -> FaceSettings {
    let config = Config::parse(config_text).unwrap();
    FaceSettings::new(&config, "~/.config/alba/config.toml".to_string())
}

fn weather() -> Weather {
    Weather {
        fetched_at: default_instant().timestamp(),
        forecast: Forecast::parse(&forecast_body()).unwrap(),
    }
}

fn shown(weather_state: WeatherState) -> ShownWeather {
    ShownWeather {
        weather_state,
        shows_fetch_indicator: false,
        place_name: None,
    }
}

fn text(line: &Line) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn without_weather_data_the_face_shows_the_ghost_icon_no_timeline_and_no_credit() {
    for weather_state in [
        WeatherState::NoLocation,
        WeatherState::Fetching,
        WeatherState::Offline,
        WeatherState::LocationNotFound("atlantis"),
    ] {
        let face = Face::at(&settings(""), shown(weather_state), &default_instant());

        assert_eq!(face.icon.map(|icon| icon.icon), Some(Icon::Ghost));
        assert!(face.timeline.is_none(), "{weather_state:?}");
        assert!(!text(&face.bezel.bottom_right).contains("open-meteo"));
    }
}

#[test]
fn with_weather_data_the_face_shows_the_condition_the_timeline_and_the_credit() {
    let weather = weather();
    for weather_state in [
        WeatherState::Fresh(&weather),
        WeatherState::Stale {
            weather: &weather,
            age: jiff::SignedDuration::from_hours(3),
        },
    ] {
        let face = Face::at(&settings(""), shown(weather_state), &default_instant());

        assert!(
            matches!(
                face.icon.map(|icon| icon.icon),
                Some(Icon::Day(_) | Icon::Night(..))
            ),
            "{weather_state:?}"
        );
        assert!(face.timeline.is_some(), "{weather_state:?}");
        assert!(text(&face.bezel.bottom_right).contains("open-meteo"));
    }
}

#[test]
fn the_place_name_goes_to_the_bezel() {
    let weather = weather();
    let shown_weather = ShownWeather {
        place_name: Some("Buenos Aires, AR"),
        ..shown(WeatherState::Fresh(&weather))
    };

    let face = Face::at(&settings(""), shown_weather, &default_instant());

    assert_eq!(face.bezel.place_name.as_deref(), Some("Buenos Aires, AR"));
}

#[test]
fn without_motion_the_icon_stands_still_at_every_instant() {
    let weather = weather();
    for quarter_second in 0..8 {
        let now = default_instant()
            .checked_add((250 * quarter_second).milliseconds())
            .unwrap();

        let face = Face::at(
            &settings("motion = false"),
            shown(WeatherState::Fresh(&weather)),
            &now,
        );

        let motion = face.icon.unwrap().motion;
        assert!(motion.colon_lit && motion.icon_frame == 0 && !motion.flash);
    }
}
