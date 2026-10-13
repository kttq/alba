use std::{io, process::ExitCode, time::Duration};

use alba::app::{App, Control};
use jiff::{SignedDuration, Timestamp, Zoned};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};

const HELP: &str = "\
alba: a clock, a month calendar, and the current weather in a terminal pane.

usage: alba [--help | --version]

keys: q, esc, or ctrl-c quits.";

enum Command {
    Run,
    Help,
    Version,
}

fn main() -> ExitCode {
    let command = match parse_args(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(argument) => {
            eprintln!("error: unexpected argument '{argument}'\n\nrun 'alba --help' for usage");
            return ExitCode::FAILURE;
        }
    };
    match command {
        Command::Help => println!("{HELP}"),
        Command::Version => println!("alba {}", env!("CARGO_PKG_VERSION")),
        Command::Run => {
            let mut terminal = ratatui::init();
            let result = run(&mut terminal, &mut App::new());
            ratatui::restore();
            if let Err(error) = result {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

/// Accepts no arguments, or `--help` or `--version` alone. Returns the first argument it rejects.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    let command = match args.next().as_deref() {
        None => Command::Run,
        Some("--help") => Command::Help,
        Some("--version") => Command::Version,
        Some(other) => return Err(other.to_string()),
    };
    match args.next() {
        None => Ok(command),
        Some(extra) => Err(extra),
    }
}

/// Ticks on every whole wall-clock second. Draws after each tick, and at once on key and resize
/// events.
fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut now = Zoned::now();
    app.tick(&now);
    loop {
        terminal.draw(|frame| app.render(frame.area(), frame.buffer_mut(), &now))?;
        let next_tick = next_whole_second(now.timestamp());
        if event::poll(until(next_tick))?
            && let Event::Key(key) = event::read()?
            && app.key(key) == Control::Quit
        {
            return Ok(());
        }
        if Timestamp::now() >= next_tick {
            now = Zoned::now();
            app.tick(&now);
        }
    }
}

fn next_whole_second(now: Timestamp) -> Timestamp {
    const NANOS_PER_SECOND: i128 = 1_000_000_000;
    let into_second = now.as_nanosecond().rem_euclid(NANOS_PER_SECOND) as i64;
    now - SignedDuration::from_nanos(into_second) + SignedDuration::from_secs(1)
}

/// Zero once `deadline` has passed.
fn until(deadline: Timestamp) -> Duration {
    Duration::try_from(deadline.duration_since(Timestamp::now())).unwrap_or(Duration::ZERO)
}
