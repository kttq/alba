mod common;

use alba::{calendar::Calendar, config::Config};
use common::default_instant;
use jiff::Zoned;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
};

fn week_strip(config_text: &str, now: &Zoned) -> [Line<'static>; 2] {
    Calendar::new(&Config::parse(config_text).unwrap()).week_strip(now)
}

/// Each cell's text and style, without the spaces between the cells.
fn cells(line: &Line) -> Vec<(String, Style)> {
    (line.spans.iter())
        .filter(|span| span.content != " ")
        .map(|span| (span.content.to_string(), span.style))
        .collect()
}

fn texts(line: &Line) -> Vec<String> {
    cells(line).into_iter().map(|(text, _)| text).collect()
}

#[test]
fn week_strip_pills_today_dims_past_days_and_tints_the_weekend() {
    let [header, days] = week_strip("", &default_instant());

    assert_eq!(header.to_string(), "mo tu we th fr sa su");
    assert_eq!(days.to_string(), "12 13 14 15 16 17 18");
    let header = cells(&header);
    let days = cells(&days);
    let dim = Style::new().fg(Color::DarkGray);
    let weekend = Style::new().fg(Color::Green);
    assert_eq!(header[0].1, dim, "a weekday name");
    assert_eq!(header[5].1, weekend, "a weekend name");
    assert_eq!(days[0].1, dim, "a past day");
    assert_eq!(
        days[1].1,
        Style::new()
            .fg(Color::Yellow)
            .add_modifier(Modifier::REVERSED),
        "today's pill"
    );
    assert_eq!(days[2].1, Style::new(), "a coming weekday");
    assert_eq!(days[5].1, weekend, "a coming weekend day");
}

#[test]
fn sunday_first_week_strip_tints_the_weekend_by_the_day_of_the_week() {
    let [header, days] = week_strip("week_start = \"sunday\"", &default_instant());

    assert_eq!(header.to_string(), "su mo tu we th fr sa");
    assert_eq!(days.to_string(), "11 12 13 14 15 16 17");
    let green = Some(Color::Green);
    let header = cells(&header);
    assert_eq!(header[0].1.fg, green, "sunday, first");
    assert_eq!(header[6].1.fg, green, "saturday, last");
    assert_eq!(header[1].1.fg, Some(Color::DarkGray), "monday");
    assert_eq!(cells(&days)[6].1.fg, green);
}

#[test]
fn week_strip_crosses_into_the_next_month() {
    let thursday: Zoned = "2026-10-29T12:00[America/Argentina/Buenos_Aires]"
        .parse()
        .unwrap();

    let [_, days] = week_strip("", &thursday);

    assert_eq!(texts(&days), ["26", "27", "28", "29", "30", "31", " 1"]);
}
