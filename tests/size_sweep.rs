mod common;

use alba::{
    app::App,
    layout::{self, Face, LayoutReport, SizeTier},
    weather::FetchError,
};
use common::{BUENOS_AIRES, Harness, NO_MATCH, forecast_body, geocoding_body};
use jiff::{ToSpan, Zoned};
use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyEvent},
    layout::Rect,
    style::Color,
};
use serde_json::{Value, json};

const WIDEST_PANE: u16 = 250;
const TALLEST_PANE: u16 = 80;

/// Beyond printable ASCII: the Spanish letters and the Departure Mono "Available" list.
const APPROVED_GLYPHS: &str = "áéíóúñüÁÉÍÓÚÑ¡¿\
    █▀▄▌▐░▒▓■□▪•·─│╭╮╰╯┄┈═»«›‹↑↓←→★☆✦✧♪♫♥∙°…▁▂▃▄▅▆▇╷╵┊┆▼♦˙‚′″≈~";

#[derive(Debug, Clone, Copy)]
enum WeatherStateFixture {
    NoLocation,
    Fetching,
    Offline,
    FreshDay,
    FreshNight,
    Stale,
    LocationNotFound,
}

const WEATHER_STATES: [WeatherStateFixture; 7] = [
    WeatherStateFixture::NoLocation,
    WeatherStateFixture::Fetching,
    WeatherStateFixture::Offline,
    WeatherStateFixture::FreshDay,
    WeatherStateFixture::FreshNight,
    WeatherStateFixture::Stale,
    WeatherStateFixture::LocationNotFound,
];

fn config_variants() -> Vec<String> {
    let mut variants = Vec::new();
    for language in ["en", "es"] {
        for hour_format in [24, 12] {
            for units in ["metric", "imperial"] {
                variants.push(format!(
                    "language = \"{language}\"\nhour_format = {hour_format}\nunits = \"{units}\""
                ));
            }
        }
    }
    variants
}

fn instant(time: &str) -> Zoned {
    format!("2026-10-13T{time}[America/Argentina/Buenos_Aires]")
        .parse()
        .unwrap()
}

/// The widest temperatures, `-12°` and `104°`, and the longest condition name in each language.
fn worst_case_forecast(config_text: &str) -> String {
    let celsius = if config_text.contains("imperial") {
        40.0
    } else {
        -12.0
    };
    let weather_code = if config_text.contains("\"es\"") {
        65
    } else {
        2
    };
    let mut forecast: Value = serde_json::from_str(&forecast_body()).unwrap();
    forecast["current"]["temperature_2m"] = json!(celsius);
    forecast["current"]["weather_code"] = json!(weather_code);
    forecast.to_string()
}

/// A place name longer than the bezel's free space.
fn long_place_geocoding() -> String {
    let mut geocoding: Value = serde_json::from_str(&geocoding_body()).unwrap();
    geocoding["results"][0]["name"] =
        json!("Llanfairpwllgwyngyllgogerychwyrndrobwllllantysiliogogogoch");
    geocoding["results"][0]["country_code"] = json!("GB");
    geocoding.to_string()
}

/// The app in the weather state, and the instant to show it at. Every state with weather has a
/// refresh running long enough to show the fetch indicator, the widest temperature.
fn app_in(weather_state: WeatherStateFixture, config_text: &str) -> (App, Zoned) {
    let noon = instant("12:38");
    let located = format!("{BUENOS_AIRES}\n{config_text}");
    let named = format!("location = \"llanfair\"\n{config_text}");
    let mut harness = match weather_state {
        WeatherStateFixture::NoLocation => Harness::new(config_text),
        WeatherStateFixture::LocationNotFound => {
            Harness::new(&format!("location = \"atlantis\"\n{config_text}"))
        }
        WeatherStateFixture::FreshDay
        | WeatherStateFixture::FreshNight
        | WeatherStateFixture::Stale => Harness::new(&named),
        _ => Harness::new(&located),
    };
    let body = worst_case_forecast(config_text);
    let shown_at = match weather_state {
        WeatherStateFixture::NoLocation => noon,
        WeatherStateFixture::Fetching => {
            harness.app.tick(&noon);
            noon.checked_add(1.second()).unwrap()
        }
        WeatherStateFixture::Offline => {
            harness.fetch(&noon, Err(FetchError::Network("timed out".to_string())));
            noon
        }
        WeatherStateFixture::FreshDay => refreshing(&mut harness, &noon, &noon, body),
        WeatherStateFixture::FreshNight => {
            let night = instant("23:38");
            refreshing(&mut harness, &night, &night, body)
        }
        WeatherStateFixture::Stale => refreshing(&mut harness, &instant("01:38"), &noon, body),
        WeatherStateFixture::LocationNotFound => {
            harness.fetch(&noon, Ok(NO_MATCH.to_string()));
            noon
        }
    };
    (harness.app, shown_at)
}

/// Looks up the long place name first, then fetches the forecast.
fn refreshing(harness: &mut Harness, fetched_at: &Zoned, now: &Zoned, body: String) -> Zoned {
    harness.fetch(fetched_at, Ok(long_place_geocoding()));
    harness.weather_source.finish(Ok(body));
    harness.app.tick(fetched_at);
    harness.app.key(KeyEvent::from(KeyCode::Char('r')));
    harness.app.tick(now);
    now.checked_add(1.second()).unwrap()
}

/// Formatted only when an assert fails, so a million sizes stay fast.
struct Case<'a> {
    config_text: &'a str,
    weather_state: WeatherStateFixture,
    pane: Rect,
}

impl std::fmt::Display for Case<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "{:?}, {:?}, {}×{}",
            self.config_text, self.weather_state, self.pane.width, self.pane.height
        )
    }
}

fn largest_fitting(face: &Face, pane: Rect) -> Option<SizeTier> {
    SizeTier::IN_ORDER.into_iter().find(|&size_tier| {
        layout::minimum_size(size_tier, face)
            .is_some_and(|size| size.width <= pane.width && size.height <= pane.height)
    })
}

fn assert_items_fit_without_overlap(report: &LayoutReport, pane: Rect, case: &Case) {
    for (index, item) in report.items.iter().enumerate() {
        assert!(
            item.required_size.width <= item.area.width
                && item.required_size.height <= item.area.height,
            "{case}: {item:?} does not fit its area"
        );
        assert_eq!(
            pane.intersection(item.area),
            item.area,
            "{case}: {item:?} leaves the pane"
        );
        for other in &report.items[index + 1..] {
            assert!(
                !item.area.intersects(other.area),
                "{case}: {item:?} overlaps {other:?}"
            );
        }
    }
}

fn assert_approved_glyphs_and_named_colors(app: &App, now: &Zoned, pane: Rect, case: &Case) {
    let mut terminal = Terminal::new(TestBackend::new(pane.width, pane.height)).unwrap();
    terminal
        .draw(|frame| app.render(frame.area(), frame.buffer_mut(), now))
        .unwrap();
    for cell in terminal.backend().buffer().content() {
        for glyph in cell.symbol().chars() {
            assert!(
                glyph.is_ascii_graphic() || glyph == ' ' || APPROVED_GLYPHS.contains(glyph),
                "{case}: {glyph:?} is not an approved glyph"
            );
        }
        for color in [cell.fg, cell.bg] {
            assert!(
                !matches!(color, Color::Rgb(..) | Color::Indexed(_)),
                "{case}: {color:?} is not a named color"
            );
        }
    }
}

/// A tier draws the same at every pane size apart from its offset, so each tier and Strip
/// variant renders once, at the first size that picks it.
#[test]
fn every_pane_size_gets_the_largest_fitting_tier_without_clipping() {
    for config_text in config_variants() {
        for weather_state in WEATHER_STATES {
            let (app, now) = app_in(weather_state, &config_text);
            let face = app.face(&now);
            let mut rendered = Vec::new();
            for width in 1..=WIDEST_PANE {
                for height in 1..=TALLEST_PANE {
                    let pane = Rect::new(0, 0, width, height);
                    let case = Case {
                        config_text: &config_text,
                        weather_state,
                        pane,
                    };
                    let report = layout::lay_out(pane, &face);

                    assert_eq!(report.size_tier, largest_fitting(&face, pane), "{case}");
                    assert_items_fit_without_overlap(&report, pane, &case);

                    let drawn = (report.size_tier, report.items.len());
                    if !rendered.contains(&drawn) {
                        rendered.push(drawn);
                        assert_approved_glyphs_and_named_colors(&app, &now, pane, &case);
                    }
                }
            }
        }
    }
}
