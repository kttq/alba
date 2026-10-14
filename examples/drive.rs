//! Drives alba headless in a pseudo-terminal, and prints what its screen shows.

#[allow(dead_code)]
#[path = "../tests/common/pty.rs"]
mod pty;

use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use pty::{Alba, Launch, PaneSize, Step, framed, pane_length, run_steps, transcript_lines};

const USAGE: &str = "\
usage: drive HOME [--cols 40] [--rows 12] [--offline] [--binary PATH] [--transcript FILE] STEP...

Starts alba in HOME, runs the steps in order, and then, unless alba has exited, presses q.
Build alba first with `cargo build --release`.

steps:
  sleep=SECONDS     read output for that long
  key=CHARS         type the characters, 50 ms apart; esc is \\x1b, ctrl-c is \\x03
  flood=SECONDS     type r at 30 keys a second for that long
  resize=COLSxROWS  resize the pane
  screen            print the screen grid
  colors            print each run of styled text with its colors and dim flag
  expect=TEXT       fail unless the screen contains TEXT
  exited            fail unless alba exits within 2 seconds

options:
  --offline         run alba under macOS sandbox-exec, network denied
  --binary PATH     run this binary instead of the alba beside this example
  --transcript FILE write each read as a JSON line: {\"time\": unix seconds, \"output\": text}

The driver exits 0 only when every step passed, alba exited 0, and the terminal was
restored: the alternate screen left, the cursor shown, and cooked mode back on.";

struct Drive {
    launch: Launch,
    transcript: Option<PathBuf>,
    steps: Vec<Step>,
}

fn main() -> ExitCode {
    if env::args().any(|argument| argument == "-h" || argument == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let drive = match parse(env::args().skip(1)) {
        Ok(drive) => drive,
        Err(reason) => {
            eprintln!("drive: {reason}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&drive) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("drive: {error}");
            ExitCode::from(2)
        }
    }
}

/// Options and steps may come in any order after HOME.
fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Drive, String> {
    let mut home = None;
    let mut pane_size = PaneSize { cols: 40, rows: 12 };
    let mut offline = false;
    let mut binary = None;
    let mut transcript = None;
    let mut steps = Vec::new();
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--cols" => pane_size.cols = pane_length(&value()?)?,
            "--rows" => pane_size.rows = pane_length(&value()?)?,
            "--offline" => offline = true,
            "--binary" => binary = Some(PathBuf::from(value()?)),
            "--transcript" => transcript = Some(PathBuf::from(value()?)),
            option if option.starts_with("--") => return Err(format!("unknown option {option}")),
            _ if home.is_none() => home = Some(PathBuf::from(argument)),
            step => steps.push(step.parse()?),
        }
    }
    Ok(Drive {
        launch: Launch {
            binary: binary.map_or_else(release_binary, Ok)?,
            home: home.ok_or("HOME is missing")?,
            pane_size,
            offline,
        },
        transcript,
        steps,
    })
}

/// The alba that `cargo build` puts beside this example's directory, for the same profile.
fn release_binary() -> Result<PathBuf, String> {
    let example = env::current_exe().map_err(|error| error.to_string())?;
    let binary = example
        .parent()
        .and_then(|examples| examples.parent())
        .map(|profile| profile.join("alba"))
        .filter(|binary| binary.exists())
        .ok_or("no alba binary; build it with `cargo build --release`")?;
    Ok(binary)
}

/// Returns whether the drive passed.
fn run(drive: &Drive) -> io::Result<bool> {
    let mut alba = Alba::spawn(&drive.launch)?;
    let mut out = io::stdout().lock();
    alba.wait_for("°");
    let failures = run_steps(&mut alba, &drive.steps, &mut out)?;
    let status = alba.quit();
    if let Some(path) = &drive.transcript {
        fs::write(path, transcript_lines(alba.transcript()))?;
    }
    let restored = alba.restored();
    match status {
        None => writeln!(out, "exit status: killed after timeout")?,
        Some(status) => writeln!(out, "exit status: {}", status.code().unwrap_or(-1))?,
    }
    writeln!(
        out,
        "terminal restored: {}",
        if restored { "yes" } else { "no" }
    )?;
    match alba.cpu_seconds() {
        None => writeln!(out, "cpu seconds: unknown")?,
        Some(seconds) => writeln!(out, "cpu seconds: {seconds:.2}")?,
    }
    let exit_code = alba.exit_code();
    if exit_code.is_some_and(|code| code != 0) {
        eprint!("{}", framed(alba.screen()));
    }
    Ok(failures == 0 && exit_code == Some(0) && restored)
}
