//! The real binary on a pseudo-terminal. Each run must end well inside its deadline: a driver
//! that hangs fails these tests instead of the session that uses it.

mod common;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use common::{
    fixture_home::{CachedWeather, FixtureHome},
    pty::{
        Alba, Drive, EXITED_WITHIN, FLOOD_GAP, Launch, Output, PaneSize, Step, colors, pane_length,
        screen_restored, styled_runs, transcript_lines,
    },
    temporary_directory,
};
use jiff::Timestamp;
use serde_json::Value;
use vt100::Color;

const MEDIUM: PaneSize = PaneSize { cols: 40, rows: 12 };

fn alba() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_alba"))
}

/// alba on a fresh fixture home, once its first frame shows the temperature.
fn start(fixture_home: &FixtureHome, pane_size: PaneSize) -> Alba {
    start_binary(alba(), fixture_home, pane_size)
}

fn start_offline(fixture_home: &FixtureHome, pane_size: PaneSize) -> Alba {
    launch(alba(), fixture_home, pane_size, true)
}

fn start_binary(binary: PathBuf, fixture_home: &FixtureHome, pane_size: PaneSize) -> Alba {
    launch(binary, fixture_home, pane_size, false)
}

fn launch(binary: PathBuf, fixture_home: &FixtureHome, pane_size: PaneSize, offline: bool) -> Alba {
    let home = temporary_directory();
    fixture_home.write(&home, Timestamp::now()).unwrap();
    let mut alba = Alba::spawn(&Launch {
        binary,
        home,
        pane_size,
        offline,
    })
    .unwrap();
    alba.wait_for("°");
    alba
}

#[test]
fn each_quit_key_exits_on_its_own_and_restores_the_terminal() {
    for key in ["q", "\x1b", "\x03"] {
        let mut alba = start(&FixtureHome::default(), MEDIUM);
        alba.type_keys(key);
        assert!(alba.exited(EXITED_WITHIN), "{key:?} did not quit");
        assert_eq!(alba.exit_code(), Some(0), "{key:?}");
        assert!(alba.restored(), "{key:?} left the terminal dirty");
    }
}

/// Leaves the alternate screen and shows the cursor, but stays in raw mode.
const RAW_EXIT: &str = "#!/bin/sh
stty raw -echo
printf '\\033[?1049h\\033[?1049l\\033[?25h'
";

/// Draws once, then ignores every key.
const DEAF: &str = "#!/bin/sh
printf '\\033[?1049h'
exec sleep 30
";

fn fake_binary(script: &str) -> PathBuf {
    let directory = temporary_directory();
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("alba");
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn raw_mode_left_behind_is_not_restored() {
    let mut alba = start_binary(fake_binary(RAW_EXIT), &FixtureHome::default(), MEDIUM);
    alba.quit();
    assert_eq!(alba.exit_code(), Some(0));
    assert!(!alba.restored());
}

#[test]
fn a_key_that_does_not_quit_fails_exited() {
    let mut alba = start_binary(fake_binary(DEAF), &FixtureHome::default(), MEDIUM);
    alba.type_keys("\x1b");
    assert!(!alba.exited(EXITED_WITHIN));
}

#[test]
fn cursor_hidden_after_it_was_shown_is_not_restored() {
    let mut parser = vt100::Parser::new(1, 10, 0);
    parser.process(b"\x1b[?1049h\x1b[?25h\x1b[?25l\x1b[?1049l");
    assert!(!screen_restored(parser.screen()));
    parser.process(b"\x1b[?25h");
    assert!(screen_restored(parser.screen()));
}

#[test]
fn quit_after_a_key_flood_exits_at_once() {
    let started = Instant::now();
    let mut alba = start(&FixtureHome::default(), MEDIUM);
    alba.type_keys_apart(&"r".repeat(60), FLOOD_GAP);
    alba.quit();
    assert_eq!(alba.exit_code(), Some(0));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn resize_redraws_the_line_on_the_new_middle_row() {
    let mut alba = start(&FixtureHome::default(), MEDIUM);
    alba.resize(PaneSize { cols: 60, rows: 20 }).unwrap();
    alba.pump(Duration::from_millis(300));
    let rows: Vec<String> = alba.screen().rows(0, 60).collect();
    assert_eq!(rows.len(), 20);
    assert!(rows[10].contains('°'), "{rows:#?}");
}

#[test]
fn a_config_error_exits_without_hanging() {
    let fixture_home = FixtureHome {
        config_lines: vec!["colour = 1".to_string()],
        ..FixtureHome::default()
    };
    let mut alba = start(&fixture_home, MEDIUM);
    alba.quit();
    assert_eq!(alba.exit_code(), Some(1));
}

#[test]
fn transcript_times_each_read_in_order() {
    let started = SystemTime::now();
    let mut alba = start(&FixtureHome::default(), MEDIUM);
    alba.quit();
    let reads: Vec<Value> = transcript_lines(alba.transcript())
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let times: Vec<f64> = reads
        .iter()
        .map(|read| read["time"].as_f64().unwrap())
        .collect();
    assert!(times.is_sorted(), "{times:?}");
    let started = started.duration_since(UNIX_EPOCH).unwrap().as_secs_f64();
    assert!(times[0] > started);
    let output: String = reads
        .iter()
        .map(|read| read["output"].as_str().unwrap())
        .collect();
    assert!(output.contains('°'));
}

const STRIP: PaneSize = PaneSize { cols: 30, rows: 1 };

#[test]
fn the_date_is_bright_blue() {
    let alba = start(&FixtureHome::default(), STRIP);
    let line = alba.screen().contents();
    let date = line.trim().split(" · ").nth(1).unwrap();
    let bright_blue: Vec<String> = styled_runs(alba.screen())
        .into_iter()
        .filter(|run| run.foreground == Color::Idx(12))
        .map(|run| run.text)
        .collect();
    assert_eq!(
        bright_blue.join(" "),
        date,
        "{line}\n{}",
        colors(alba.screen())
    );
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "offline runs need macOS sandbox-exec"
)]
fn a_stale_temperature_is_dark_gray() {
    let fixture_home = FixtureHome {
        cached_weather: CachedWeather::Stale,
        ..FixtureHome::default()
    };
    let alba = start_offline(&fixture_home, STRIP);
    let runs = styled_runs(alba.screen());
    let temperature = runs.iter().find(|run| run.text.ends_with('°')).unwrap();
    assert_eq!(temperature.foreground, Color::Idx(8), "{runs:#?}");
}

#[test]
fn no_cell_uses_an_rgb_or_indexed_color() {
    for pane_size in [STRIP, MEDIUM, PaneSize { cols: 85, rows: 26 }] {
        let alba = start(&FixtureHome::default(), pane_size);
        let screen = alba.screen();
        for (row, col) in
            (0..pane_size.rows).flat_map(|row| (0..pane_size.cols).map(move |col| (row, col)))
        {
            let cell = screen.cell(row, col).unwrap();
            for color in [cell.fgcolor(), cell.bgcolor()] {
                assert!(
                    matches!(color, Color::Default | Color::Idx(0..16)),
                    "{color:?} at {row},{col} in {pane_size:?}"
                );
            }
        }
    }
}

fn drive_arguments(arguments: &str) -> Result<Drive, String> {
    Drive::parse(arguments.split(' ').map(String::from))
}

#[test]
fn bad_drive_arguments_fail_to_parse() {
    for arguments in [
        "home resize=wide",
        "home --cols 0",
        "home --rows",
        "home --colour 3",
        "home jump",
        "--cols 40",
    ] {
        assert!(drive_arguments(arguments).is_err(), "{arguments}");
    }
}

#[test]
fn options_and_steps_mix_after_home() {
    let drive = drive_arguments("home screen --cols 60 key=q --offline exited").unwrap();
    assert_eq!(drive.home, PathBuf::from("home"));
    assert_eq!(drive.pane_size, PaneSize { cols: 60, rows: 12 });
    assert!(drive.offline);
    assert_eq!(
        drive.steps,
        [Step::Screen, Step::Key("q".to_string()), Step::Exited]
    );
}

#[test]
fn a_character_split_across_reads_stays_whole_in_the_transcript() {
    let degree = "°".as_bytes();
    let reads = [&[b'1', b'4', degree[0]][..], &degree[1..]].map(|bytes| Output {
        time: SystemTime::now(),
        bytes: bytes.to_vec(),
    });
    let outputs: Vec<String> = transcript_lines(&reads)
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["output"].to_string())
        .collect();
    assert_eq!(outputs, [r#""14""#, r#""°""#]);
}

#[test]
fn bad_steps_and_sizes_fail_to_parse() {
    for step in [
        "resize=wide",
        "resize=0x3",
        "resize=70000x3",
        "flood=nan",
        "sleep=-1",
        "jump",
        "key",
    ] {
        assert!(step.parse::<Step>().is_err(), "{step}");
    }
    assert!(pane_length("0").is_err());
    assert_eq!(
        "resize=60x20".parse::<Step>(),
        Ok(Step::Resize(PaneSize { cols: 60, rows: 20 }))
    );
    assert_eq!(
        "key=\\x1bq".parse::<Step>(),
        Ok(Step::Key("\x1bq".to_string()))
    );
}
