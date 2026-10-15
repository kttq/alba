//! The real binary on a pseudo-terminal. Each run must end well inside its deadline: a driver
//! that hangs fails these tests instead of the session that uses it.
//!
//! Every run denies the network where macOS `sandbox-exec` exists, so a fetch shows up as
//! `fetch failed` in the log. The tests that read the log prove that the other tests make no
//! request, on Linux too. A stale or expired cache makes alba fetch at once, so those tests
//! run only where the network can be denied.

mod common;

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use alba::paths::Paths;
use common::{
    fixture_home::{CachedWeather, FixtureHome},
    pty::{Alba, EXITED_WITHIN, FLOOD_GAP, Launch, Output, PaneSize, WAIT_DEADLINE, styled_runs},
    temporary_directory,
};
use jiff::Timestamp;
use vt100::Color;

const STRIP: PaneSize = PaneSize { cols: 30, rows: 1 };
const MEDIUM: PaneSize = PaneSize { cols: 40, rows: 12 };
const LARGE: PaneSize = PaneSize { cols: 85, rows: 26 };
const FULL_SCREEN: PaneSize = PaneSize {
    cols: 250,
    rows: 80,
};
const OFFLINE: bool = cfg!(target_os = "macos");
const BRIGHT_BLUE: Color = Color::Idx(12);
const BRIGHT_BLACK: Color = Color::Idx(8);

fn home_with(fixture_home: &FixtureHome) -> PathBuf {
    let home = temporary_directory();
    fixture_home.write(&home, Timestamp::now()).unwrap();
    home
}

fn spawn(home: &Path, pane_size: PaneSize) -> Alba {
    Alba::spawn(&Launch {
        binary: PathBuf::from(env!("CARGO_BIN_EXE_alba")),
        home: home.to_path_buf(),
        pane_size,
        offline: OFFLINE,
    })
    .unwrap()
}

/// alba on a fresh fixture home, once it has drawn its first frame.
fn start(fixture_home: &FixtureHome, pane_size: PaneSize) -> Alba {
    let mut alba = spawn(&home_with(fixture_home), pane_size);
    assert!(alba.wait_for_first_frame(), "alba drew nothing");
    alba
}

fn log(home: &Path) -> String {
    let paths = Paths::resolve(|name| (name == "HOME").then(|| OsString::from(home))).unwrap();
    fs::read_to_string(paths.state_directory.join("alba.log")).unwrap_or_default()
}

fn no_location() -> FixtureHome {
    FixtureHome {
        has_location: false,
        cached_weather: CachedWeather::NoCache,
        ..FixtureHome::default()
    }
}

fn with_weather(cached_weather: CachedWeather) -> FixtureHome {
    FixtureHome {
        cached_weather,
        ..FixtureHome::default()
    }
}

/// The strip line split into its parts: clock, date, and weather, as far as they fit.
fn strip_parts(alba: &Alba) -> Vec<String> {
    strip_line_parts(alba.screen())
}

fn strip_line_parts(screen: &vt100::Screen) -> Vec<String> {
    match screen.contents().trim() {
        "" => Vec::new(),
        line => line.split(" · ").map(String::from).collect(),
    }
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

#[test]
fn other_keys_do_not_quit() {
    let mut alba = start(&no_location(), STRIP);
    alba.type_keys("aQ \r");
    assert!(!alba.exited(Duration::from_millis(500)));
}

#[test]
fn a_config_error_exits_1_names_its_line_and_restores_the_terminal() {
    let home = home_with(&FixtureHome {
        config_lines: vec!["colour = 1".to_string()],
        ..FixtureHome::default()
    });
    let mut alba = spawn(&home, MEDIUM);
    assert!(alba.exited(EXITED_WITHIN));
    assert_eq!(alba.exit_code(), Some(1));
    assert!(alba.restored());
    assert!(
        alba.screen()
            .contents()
            .contains("config error: line 2: unknown field `colour`"),
        "{}",
        alba.screen().contents()
    );
}

#[test]
fn quit_after_a_key_flood_exits_at_once() {
    let started = Instant::now();
    let mut alba = start(&no_location(), MEDIUM);
    alba.type_keys_apart(&"r".repeat(60), FLOOD_GAP);
    alba.quit();
    assert_eq!(alba.exit_code(), Some(0));
    assert!(alba.restored());
    assert!(started.elapsed() < Duration::from_secs(10));
}

/// Without motion the tick is one second, so a redraw well inside it comes from the resize.
#[test]
fn a_resized_pane_shows_at_once_what_a_fresh_start_at_that_size_shows() {
    const AT_ONCE: Duration = Duration::from_millis(500);
    let still = FixtureHome {
        config_lines: vec!["motion = false".to_string()],
        ..FixtureHome::default()
    };
    for (from, to) in [
        (MEDIUM, PaneSize { cols: 60, rows: 20 }),
        (LARGE, MEDIUM),
        (MEDIUM, STRIP),
    ] {
        let mut fresh = start(&still, to);
        let mut resized = start(&still, from);
        // Startup is over once the cached weather shows.
        assert!(resized.wait_for("°"), "{from:?} drew no weather");
        let resized_at = Instant::now();
        resized.resize(to).unwrap();
        // Both clocks run, so read both until they agree.
        while resized.screen().contents() != fresh.screen().contents()
            && resized_at.elapsed() < WAIT_DEADLINE
        {
            resized.pump(Duration::from_millis(20));
            fresh.pump(Duration::from_millis(5));
        }
        let took = resized_at.elapsed();
        assert_eq!(
            resized.screen().contents(),
            fresh.screen().contents(),
            "{from:?} resized to {to:?}"
        );
        assert!(took < AT_ONCE, "{from:?} resized to {to:?} took {took:?}");
    }
}

#[test]
fn the_strip_drops_the_date_then_the_weather_then_everything() {
    let mut alba = start(&FixtureHome::default(), STRIP);
    for (cols, parts) in [(30, 3), (16, 2), (6, 1), (4, 0)] {
        alba.resize(PaneSize { cols, rows: 1 }).unwrap();
        // The cut-off line from before the resize can have the same number of parts.
        let drawn = alba.wait_until(|screen| has_strip_shape(&strip_line_parts(screen), parts));
        assert!(
            drawn,
            "{cols} columns: {:?}",
            strip_line_parts(alba.screen())
        );
    }
}

/// The clock first, then the date, then the temperature, with the parts that fit.
fn has_strip_shape(shown: &[String], parts: usize) -> bool {
    let is_clock = |part: &String| part.len() == 5 && part.as_bytes()[2] == b':';
    let is_date = |part: &String| part.split(' ').count() == 2 && !part.contains('°');
    let is_weather = |part: &String| part.starts_with("14°");
    shown.len() == parts
        && match shown {
            [] => true,
            [clock] => is_clock(clock),
            [clock, weather] => is_clock(clock) && is_weather(weather),
            [clock, date, weather] => is_clock(clock) && is_date(date) && is_weather(weather),
            _ => false,
        }
}

#[test]
fn the_date_is_bright_blue() {
    let alba = start(&FixtureHome::default(), STRIP);
    let date = strip_parts(&alba)[1].clone();
    let bright_blue: Vec<String> = styled_runs(alba.screen())
        .into_iter()
        .filter(|run| run.foreground == BRIGHT_BLUE)
        .map(|run| run.text)
        .collect();
    assert_eq!(bright_blue.join(" "), date);
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "offline runs need macOS sandbox-exec"
)]
fn a_stale_temperature_is_bright_black_and_a_fresh_one_is_default() {
    for (cached_weather, foreground) in [
        (CachedWeather::Fresh, Color::Default),
        (CachedWeather::Stale, BRIGHT_BLACK),
    ] {
        let alba = start(&with_weather(cached_weather), STRIP);
        let runs = styled_runs(alba.screen());
        let temperature = runs.iter().find(|run| run.text == "14°");
        assert_eq!(
            temperature.map(|run| run.foreground),
            Some(foreground),
            "{cached_weather:?}: {runs:#?}"
        );
    }
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "offline runs need macOS sandbox-exec"
)]
fn an_expired_cache_shows_no_weather() {
    let alba = start(&with_weather(CachedWeather::Expired), STRIP);
    assert_eq!(strip_parts(&alba).len(), 2, "{:?}", strip_parts(&alba));
}

#[test]
fn no_cell_uses_an_rgb_or_an_extended_color() {
    let fixture_homes = [
        FixtureHome::default(),
        FixtureHome {
            weather_code: Some(95),
            ..FixtureHome::default()
        },
        FixtureHome {
            weather_code: Some(0),
            night: true,
            ..FixtureHome::default()
        },
        no_location(),
    ];
    // Without the sandbox, alba would replace stale weather with a real fetch.
    let stale = OFFLINE.then(|| with_weather(CachedWeather::Stale));
    // Start every run first, so they draw at the same time.
    let mut runs = Vec::new();
    for fixture_home in fixture_homes.iter().chain(&stale) {
        for pane_size in [STRIP, MEDIUM, LARGE, FULL_SCREEN] {
            runs.push((spawn(&home_with(fixture_home), pane_size), pane_size));
        }
    }
    for (mut alba, pane_size) in runs {
        assert!(alba.wait_for_first_frame());
        assert_basic_colors_only(alba.screen(), pane_size);
    }
}

/// The 16 colors the user's theme sets, or the default.
fn assert_basic_colors_only(screen: &vt100::Screen, pane_size: PaneSize) {
    for row in 0..pane_size.rows {
        for col in 0..pane_size.cols {
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

/// Whether the text clock's colon was faint after each read, when the screen had one.
fn colon_faint_after_each_read(
    outputs: &[Output],
    pane_size: PaneSize,
) -> Vec<(SystemTime, Option<bool>)> {
    let mut parser = vt100::Parser::new(pane_size.rows, pane_size.cols, 0);
    outputs
        .iter()
        .map(|output| {
            parser.process(&output.bytes);
            let screen = parser.screen();
            let colon = (0..pane_size.cols)
                .filter_map(|col| screen.cell(0, col))
                .find(|cell| cell.contents() == ":");
            (output.time, colon.map(|cell| cell.dim()))
        })
        .collect()
}

fn fraction_of_second(time: SystemTime) -> f64 {
    f64::from(time.duration_since(UNIX_EPOCH).unwrap().subsec_millis()) / 1000.0
}

/// Sleeps until the wall clock is this far into a quarter second, the tick with motion.
fn sleep_until_tick_phase(phase: Duration) {
    let quarter = 250_000_000;
    let into_quarter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        % quarter;
    let wait = (u128::from(phase.subsec_nanos()) + quarter - into_quarter) % quarter;
    std::thread::sleep(Duration::from_nanos(wait as u64));
}

#[test]
fn the_colon_goes_faint_at_each_half_second_and_lights_at_each_second() {
    // A tick counted from the start instead of the wall clock would land about 125 ms late.
    const START_PHASE: Duration = Duration::from_millis(125);
    // The median read lags its boundary by less than this. A busy runner can delay a few reads,
    // but not most of them.
    const TYPICAL_LATENESS: f64 = 0.08;
    sleep_until_tick_phase(START_PHASE);
    let mut alba = start(&no_location(), STRIP);
    alba.pump(Duration::from_millis(3200));
    let states = colon_faint_after_each_read(alba.transcript(), STRIP);
    let mut lateness: Vec<f64> = states
        .windows(2)
        .filter_map(|pair| match (pair[0].1, pair[1]) {
            (Some(before), (time, Some(after))) if before != after => {
                let due = if after { 0.5 } else { 0.0 };
                Some((fraction_of_second(time) - due).rem_euclid(1.0))
            }
            _ => None,
        })
        .collect();
    assert!(lateness.len() >= 5, "{lateness:?}");
    lateness.sort_by(f64::total_cmp);
    let median = lateness[lateness.len() / 2];
    assert!(median < TYPICAL_LATENESS, "median {median} s: {lateness:?}");
}

#[test]
fn without_motion_the_colon_never_goes_faint() {
    let mut alba = start(
        &FixtureHome {
            config_lines: vec!["motion = false".to_string()],
            ..no_location()
        },
        STRIP,
    );
    alba.pump(Duration::from_millis(1500));
    let states = colon_faint_after_each_read(alba.transcript(), STRIP);
    assert!(states.iter().any(|(_, faint)| faint.is_some()));
    assert!(
        states.iter().all(|(_, faint)| *faint != Some(true)),
        "{states:?}"
    );
}

/// Takes output while alba idles, and returns the CPU seconds it used meanwhile.
fn idle_cpu_seconds(fixture_home: &FixtureHome, pane_size: PaneSize, idle: Duration) -> f64 {
    let mut alba = start(fixture_home, pane_size);
    let before = alba.cpu_seconds().unwrap();
    alba.pump(idle);
    alba.cpu_seconds().unwrap() - before
}

/// A fast guard against a busy loop, at the spec's rate: 3 s in 5 minutes is 1% of one core.
#[test]
fn idle_alba_uses_under_one_percent_of_a_core() {
    const IDLE: Duration = Duration::from_secs(5);
    let used = idle_cpu_seconds(&FixtureHome::default(), LARGE, IDLE);
    assert!(
        used <= IDLE.as_secs_f64() / 100.0,
        "{used} s of CPU in {IDLE:?}"
    );
}

/// The spec's CPU check, with rain from the cache and `motion = true`. CI runs it in its own job.
#[test]
#[ignore = "takes five minutes"]
#[expect(clippy::print_stdout, reason = "the CI job reports this line")]
fn five_idle_minutes_use_under_three_cpu_seconds() {
    let rain_in_motion = FixtureHome {
        config_lines: vec!["motion = true".to_string()],
        ..FixtureHome::default()
    };
    let used = idle_cpu_seconds(&rain_in_motion, LARGE, Duration::from_secs(300));
    println!("{used:.2} s of CPU in five minutes");
    assert!(used < 3.0, "{used} s of CPU in five minutes");
}

/// Runs alba offline on the fixture, presses the keys, and returns the log after it quits.
/// A fetch fails within a moment offline, so a second with no log line means no fetch.
fn log_after(fixture_home: &FixtureHome, keys: &str) -> String {
    let home = home_with(fixture_home);
    let mut alba = spawn(&home, MEDIUM);
    assert!(alba.wait_for_first_frame());
    alba.type_keys(keys);
    alba.pump(Duration::from_secs(1));
    alba.quit();
    assert_eq!(alba.exit_code(), Some(0));
    log(&home)
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "offline runs need macOS sandbox-exec"
)]
fn the_offline_log_shows_a_fetch() {
    let home = home_with(&FixtureHome::default());
    let mut alba = spawn(&home, MEDIUM);
    assert!(alba.wait_for_first_frame());
    alba.type_keys("r");
    assert!(
        wait_for_log(&mut alba, &home, "fetch failed"),
        "{}",
        log(&home)
    );
    alba.quit();
    assert_eq!(
        log(&home).matches("fetch failed").count(),
        1,
        "{}",
        log(&home)
    );
}

/// Takes output until the log holds the text, for at most the wait deadline.
fn wait_for_log(alba: &mut Alba, home: &Path, text: &str) -> bool {
    let deadline = Instant::now() + WAIT_DEADLINE;
    while !log(home).contains(text) {
        if Instant::now() >= deadline {
            return false;
        }
        alba.pump(Duration::from_millis(50));
    }
    true
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "offline runs need macOS sandbox-exec"
)]
fn a_fresh_cache_starts_without_a_fetch() {
    assert_eq!(log_after(&FixtureHome::default(), ""), "");
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "offline runs need macOS sandbox-exec"
)]
fn a_cached_location_name_shows_its_weather_without_a_lookup() {
    let named = FixtureHome {
        location_name: Some("buenos aires".to_string()),
        ..FixtureHome::default()
    };

    let alba = start(&named, STRIP);

    assert_eq!(strip_parts(&alba).len(), 3, "{:?}", strip_parts(&alba));
    assert_eq!(log_after(&named, ""), "");
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "offline runs need macOS sandbox-exec"
)]
fn without_a_location_r_fetches_nothing() {
    assert_eq!(log_after(&no_location(), "r"), "");
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "offline runs need macOS sandbox-exec"
)]
fn an_offline_start_shows_the_clock_and_date_and_logs_the_failed_fetch() {
    let home = home_with(&with_weather(CachedWeather::NoCache));
    let mut alba = spawn(&home, STRIP);
    assert!(alba.wait_for_first_frame());
    assert!(wait_for_log(&mut alba, &home, "fetch failed"));
    assert_eq!(
        log(&home).matches("fetch failed").count(),
        1,
        "{}",
        log(&home)
    );
    assert_eq!(strip_parts(&alba).len(), 2, "{:?}", strip_parts(&alba));
    alba.quit();
    assert_eq!(alba.exit_code(), Some(0));
}
