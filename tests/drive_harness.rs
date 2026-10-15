//! The pseudo-terminal harness itself: it must catch a dirty exit and a dead key, never hang,
//! and parse a drive before alba starts.

mod common;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use common::{
    fixture_home::FixtureHome,
    pty::{
        Alba, Drive, EXITED_WITHIN, Launch, Output, PaneSize, Step, pane_length,
        parse_proc_stat_ticks, screen_restored, transcript_lines,
    },
    temporary_directory,
};
use jiff::Timestamp;
use serde_json::Value;

const MEDIUM: PaneSize = PaneSize { cols: 40, rows: 12 };

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

fn spawn(binary: PathBuf) -> Alba {
    let home = temporary_directory();
    FixtureHome::default()
        .write(&home, Timestamp::now())
        .unwrap();
    Alba::spawn(&Launch {
        binary,
        home,
        pane_size: MEDIUM,
        // As in the binary tests: alba must not fetch, and the sandbox makes sure.
        offline: cfg!(target_os = "macos"),
    })
    .unwrap()
}

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
    let mut alba = spawn(fake_binary(RAW_EXIT));
    alba.quit();
    assert_eq!(alba.exit_code(), Some(0));
    assert!(!alba.restored());
}

#[test]
fn a_key_that_does_not_quit_fails_exited() {
    let mut alba = spawn(fake_binary(DEAF));
    alba.type_keys("\x1b");
    assert!(!alba.exited(EXITED_WITHIN));
}

#[test]
fn a_mode_left_on_is_not_restored() {
    for left_on in [
        &b"\x1b[?25h\x1b[?25l"[..],
        b"\x1b[?1049h",
        b"\x1b[?1000h",
        b"\x1b[?2004h",
        b"\x1b[?1h",
    ] {
        let mut parser = vt100::Parser::new(1, 10, 0);
        parser.process(left_on);
        assert!(!screen_restored(parser.screen()), "{left_on:?}");
    }
}

#[test]
fn every_mode_switched_back_is_restored() {
    let mut parser = vt100::Parser::new(1, 10, 0);
    parser.process(b"\x1b[?1049h\x1b[?25l\x1b[?1000h\x1b[?2004h\x1b[?1h");
    parser.process(b"\x1b[?1l\x1b[?2004l\x1b[?1000l\x1b[?1049l\x1b[?25h");
    assert!(screen_restored(parser.screen()));
}

#[test]
fn transcript_times_each_read_in_order() {
    let started = SystemTime::now();
    let mut alba = spawn(PathBuf::from(env!("CARGO_BIN_EXE_alba")));
    alba.wait_for("°");
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
fn proc_stat_ticks_are_user_plus_system_time_after_the_command_name() {
    let stat =
        "4242 (alba (x) y) S 1 4242 4242 34816 4242 4194304 120 0 0 0 37 12 0 0 20 0 1 0 100";

    assert_eq!(parse_proc_stat_ticks(stat), Some(49));
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
fn bad_steps_and_sizes_fail_to_parse() {
    for step in [
        "resize=wide",
        "resize=0x3",
        "resize=70000x3",
        "flood=nan",
        "sleep=-1",
        "key=\\q",
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
        "key=\\x1bq\\\\".parse::<Step>(),
        Ok(Step::Key("\x1bq\\".to_string()))
    );
}

#[test]
fn a_night_fixture_is_night_with_a_timeline_at_any_hour() {
    for hour in [0, 6, 12, 15, 20] {
        let now: jiff::Zoned = format!("2026-10-15T{hour:02}:00[America/Argentina/Buenos_Aires]")
            .parse()
            .unwrap();
        let home = temporary_directory();
        let fixture_home = FixtureHome {
            night: true,
            ..FixtureHome::default()
        };
        fixture_home.write(&home, now.timestamp()).unwrap();

        let cache_directory = home.join(".cache/alba");
        let location = alba::config::Location::Coordinates(alba::config::Coordinates {
            latitude: -34.622143,
            longitude: -58.40909,
        });
        let (weather, _) = alba::weather::cache::read(&cache_directory, &location).unwrap();
        let sky = alba::sky::Sky::at(&weather.forecast, now.timestamp());

        assert_eq!(
            sky.day_or_night,
            alba::weather::DayOrNight::Night,
            "{hour}:00"
        );
        assert!(sky.timeline.is_some(), "{hour}:00");
    }
}
