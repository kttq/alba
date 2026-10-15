//! The face: everything alba shows at one instant, built from the settings and the weather.

use std::time::Duration;

use jiff::Zoned;

use crate::{
    bezel::Bezel,
    calendar::{Calendar, ShownCalendar},
    clock::{Clock, ShownClock},
    config::{Config, Units},
    icon::{Icon, ShownIcon},
    motion::{self, Motion},
    sky::{ShownTimeline, Sky},
    strings::Strings,
    weather::{ShownWeather, WeatherArea},
};

/// Everything alba shows at one instant. Each size tier asks a part for the size it places.
pub struct Face {
    pub clock: ShownClock,
    pub calendar: ShownCalendar,
    pub weather_area: WeatherArea,
    /// The ghost icon without weather data; None for a weather code with no condition.
    pub icon: Option<ShownIcon>,
    /// None without weather data, and when no sunrise and sunset surround now.
    pub timeline: Option<ShownTimeline>,
    pub bezel: Bezel,
}

/// What the config fixes about every face.
pub struct FaceSettings {
    units: Units,
    motion: bool,
    strings: &'static Strings,
    /// For the first-run hint, with `HOME` shown as `~`.
    shown_config_file: String,
    clock: Clock,
    calendar: Calendar,
}

impl FaceSettings {
    pub fn new(config: &Config, shown_config_file: String) -> Self {
        Self {
            units: config.units,
            motion: config.motion,
            strings: Strings::of(config.language),
            shown_config_file,
            clock: Clock::new(config),
            calendar: Calendar::new(config),
        }
    }

    pub fn tick_interval(&self) -> Duration {
        motion::tick_interval(self.motion)
    }
}

impl Face {
    pub fn at(settings: &FaceSettings, shown_weather: ShownWeather, now: &Zoned) -> Self {
        let motion = Motion::at(now.timestamp(), settings.motion);
        let weather = shown_weather.weather_state.weather();
        let sky = weather.map(|weather| (weather, Sky::at(&weather.forecast, now.timestamp())));
        let icon = match &sky {
            Some((weather, sky)) => Icon::of(&weather.forecast, sky),
            None => Some(Icon::Ghost),
        };
        Self {
            clock: settings.clock.at(now, motion),
            calendar: settings.calendar.at(now),
            weather_area: WeatherArea::of(
                shown_weather.weather_state,
                shown_weather.shows_fetch_indicator,
                settings.units,
                now.timestamp(),
                settings.strings,
                &settings.shown_config_file,
            ),
            icon: icon.map(|icon| ShownIcon { icon, motion }),
            timeline: sky.and_then(|(_, sky)| {
                ShownTimeline::of(&sky, now, &settings.clock, settings.strings)
            }),
            bezel: Bezel::new(
                settings.strings,
                now,
                shown_weather.place_name,
                weather.is_some(),
            ),
        }
    }
}
