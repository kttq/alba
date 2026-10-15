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
fn today_pill_leaves_the_spaces_beside_it_plain() {
    let [_, days] = week_strip("", &default_instant());

    let pill = (days.spans.iter())
        .position(|span| span.style.add_modifier.contains(Modifier::REVERSED))
        .expect("today's pill");
    assert_eq!(days.spans[pill].content, "13");
    for beside in [pill - 1, pill + 1] {
        assert_eq!(days.spans[beside].content, " ");
        assert_eq!(days.spans[beside].style, Style::new(), "span {beside}");
    }
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

fn month(config_text: &str, now: &Zoned) -> [Line<'static>; 8] {
    Calendar::new(&Config::parse(config_text).unwrap()).month(now)
}

/// One line per row, without the trailing blanks of the empty days.
fn rows(lines: &[Line]) -> String {
    (lines.iter())
        .map(|line| line.to_string().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn month_shows_the_title_the_weekday_header_and_six_week_rows() {
    let lines = month("", &default_instant());

    assert_eq!(
        rows(&lines),
        [
            "october 2026",
            "mo tu we th fr sa su",
            "          1  2  3  4",
            " 5  6  7  8  9 10 11",
            "12 13 14 15 16 17 18",
            "19 20 21 22 23 24 25",
            "26 27 28 29 30 31",
            "",
        ]
        .join("\n")
    );
    for line in &lines[1..] {
        assert_eq!(line.width(), 20, "{line}");
    }
    assert_eq!(
        cells(&lines[0])[0].1.fg,
        Some(Color::LightBlue),
        "the title"
    );
}

#[test]
fn month_pills_today_dims_past_days_and_tints_the_weekend() {
    let lines = month("", &default_instant());

    let week = cells(&lines[4]);
    assert_eq!(texts(&lines[4]), ["12", "13", "14", "15", "16", "17", "18"]);
    assert_eq!(week[0].1, Style::new().fg(Color::DarkGray), "a past day");
    assert_eq!(
        week[1].1,
        Style::new()
            .fg(Color::Yellow)
            .add_modifier(Modifier::REVERSED),
        "today's pill"
    );
    assert_eq!(week[2].1, Style::new(), "a coming weekday");
    assert_eq!(
        week[5].1,
        Style::new().fg(Color::Green),
        "a coming saturday"
    );
}

#[test]
fn sunday_first_month_starts_the_header_and_grid_on_sunday() {
    let lines = month("week_start = \"sunday\"", &default_instant());

    insta::assert_snapshot!(rows(&lines));
}

#[test]
fn sunday_first_month_tints_the_weekend_by_the_day_of_the_week() {
    let lines = month("week_start = \"sunday\"", &default_instant());

    let green = Some(Color::Green);
    let last_week = cells(&lines[6]);
    assert_eq!(texts(&lines[6]), ["25", "26", "27", "28", "29", "30", "31"]);
    assert_eq!(last_week[0].1.fg, green, "sunday 25, first column");
    assert_eq!(last_week[6].1.fg, green, "saturday 31, last column");
    assert_eq!(last_week[1].1.fg, None, "monday 26");
    let header = cells(&lines[1]);
    assert_eq!(header[0].1.fg, green, "sunday, first");
    assert_eq!(header[6].1.fg, green, "saturday, last");
}
