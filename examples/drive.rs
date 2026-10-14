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

use pty::{Alba, Drive, Launch, framed, run_steps, transcript_lines};

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

fn main() -> ExitCode {
    if env::args().any(|argument| argument == "-h" || argument == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let drive = match Drive::parse(env::args().skip(1)) {
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
    let binary = match &drive.binary {
        Some(binary) => binary.clone(),
        None => release_binary().map_err(io::Error::other)?,
    };
    let mut alba = Alba::spawn(&Launch {
        binary,
        home: drive.home.clone(),
        pane_size: drive.pane_size,
        offline: drive.offline,
    })?;
    let mut out = io::stdout().lock();
    alba.wait_for_first_frame();
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
