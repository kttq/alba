use alba::app::{App, Control};
use jiff::Zoned;
use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};

fn default_instant() -> Zoned {
    "2026-10-13T06:38[America/Argentina/Buenos_Aires]"
        .parse()
        .unwrap()
}

#[test]
fn text_clock_is_centred_in_the_pane() {
    let app = App::new();
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
        assert_eq!(App::new().key(event), Control::Quit, "{event:?}");
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
        assert_eq!(App::new().key(event), Control::Continue, "{event:?}");
    }
}
