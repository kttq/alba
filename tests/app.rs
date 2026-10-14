use std::time::Duration;

use alba::{
    app::{App, Control},
    config::Config,
};
use jiff::Zoned;
use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    style::Stylize,
    text::{Line, Span},
};

fn default_instant() -> Zoned {
    "2026-10-13T06:38[America/Argentina/Buenos_Aires]"
        .parse()
        .unwrap()
}

#[test]
fn text_clock_is_centred_in_the_pane() {
    let app = App::new(&Config::default());
    let now = default_instant();
    let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();

    terminal
        .draw(|frame| app.render(frame.area(), frame.buffer_mut(), &now))
        .unwrap();

    insta::assert_snapshot!(terminal.backend());
}

#[test]
fn q_esc_and_ctrl_c_quit() {
    let quit_keys = [
        KeyEvent::from(KeyCode::Char('q')),
        KeyEvent::from(KeyCode::Esc),
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    ];

    for event in quit_keys {
        assert_eq!(
            App::new(&Config::default()).key(event),
            Control::Quit,
            "{event:?}"
        );
    }
}

#[test]
fn other_keys_do_nothing() {
    let other_keys = [
        KeyEvent::from(KeyCode::Char('c')),
        KeyEvent::from(KeyCode::Char('x')),
        KeyEvent::from(KeyCode::Enter),
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
    ];

    for event in other_keys {
        assert_eq!(
            App::new(&Config::default()).key(event),
            Control::Continue,
            "{event:?}"
        );
    }
}

#[test]
fn pane_narrower_than_the_clock_renders_nothing() {
    let app = App::new(&Config::default());
    let now = default_instant();
    let mut terminal = Terminal::new(TestBackend::new(4, 3)).unwrap();

    terminal
        .draw(|frame| app.render(frame.area(), frame.buffer_mut(), &now))
        .unwrap();

    terminal.backend().assert_buffer_lines(["    "; 3]);
}

/// The default instant, at another wall-clock time.
fn instant(time: &str) -> Zoned {
    format!("2026-10-13T{time}[America/Argentina/Buenos_Aires]")
        .parse()
        .unwrap()
}

fn app_with(config_text: &str) -> App {
    App::new(&Config::parse(config_text).unwrap())
}

/// Renders into a pane exactly as wide as `width`, one row high.
fn render_row(app: &App, now: &Zoned, width: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
    terminal
        .draw(|frame| app.render(frame.area(), frame.buffer_mut(), now))
        .unwrap();
    terminal
}

#[test]
fn twelve_hour_clock_has_no_leading_zero_and_a_dim_mark() {
    let app = app_with("hour_format = 12");

    let terminal = render_row(&app, &instant("18:38"), 6);

    terminal
        .backend()
        .assert_buffer_lines([Line::from(vec![Span::raw("6:38"), "pm".dim()])]);
}

#[test]
fn twelve_hour_clock_reads_midnight_and_noon_as_twelve() {
    let app = app_with("hour_format = 12");

    let midnight = render_row(&app, &instant("00:05"), 7);
    let noon = render_row(&app, &instant("12:05"), 7);

    midnight
        .backend()
        .assert_buffer_lines([Line::from(vec![Span::raw("12:05"), "am".dim()])]);
    noon.backend()
        .assert_buffer_lines([Line::from(vec![Span::raw("12:05"), "pm".dim()])]);
}

#[test]
fn spanish_twelve_hour_clock_uses_the_same_mark() {
    let app = app_with("language = \"es\"\nhour_format = 12");

    let terminal = render_row(&app, &instant("06:38"), 6);

    terminal
        .backend()
        .assert_buffer_lines([Line::from(vec![Span::raw("6:38"), "am".dim()])]);
}

#[test]
fn colon_is_lit_for_the_first_half_of_each_second() {
    let terminal = render_row(&app_with(""), &instant("06:38:07.499"), 5);

    terminal.backend().assert_buffer_lines(["06:38"]);
}

#[test]
fn colon_is_faint_in_the_second_half() {
    let terminal = render_row(&app_with(""), &instant("06:38:07.500"), 5);

    terminal.backend().assert_buffer_lines([Line::from(vec![
        Span::raw("06"),
        ":".dim(),
        Span::raw("38"),
    ])]);
}

#[test]
fn colon_never_blinks_without_motion() {
    let terminal = render_row(&app_with("motion = false"), &instant("06:38:07.999"), 5);

    terminal.backend().assert_buffer_lines(["06:38"]);
}

#[test]
fn motion_ticks_every_quarter_second() {
    assert_eq!(app_with("").tick_interval(), Duration::from_millis(250));
}

#[test]
fn still_clock_ticks_every_second() {
    assert_eq!(
        app_with("motion = false").tick_interval(),
        Duration::from_secs(1)
    );
}
