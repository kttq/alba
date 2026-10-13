use std::{io, process::ExitCode, time::Duration};

use alba::app::{App, Control};
use jiff::{Timestamp, Zoned};
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

/// Ticks and draws on every whole wall-clock second, and at once on key and resize events.
fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        let now = Zoned::now();
        app.tick(&now);
        terminal.draw(|frame| app.render(frame.area(), frame.buffer_mut(), &now))?;
        if event::poll(until_next_second(Timestamp::now()))?
            && let Event::Key(key) = event::read()?
            && app.key(key) == Control::Quit
        {
            return Ok(());
        }
    }
}

fn until_next_second(now: Timestamp) -> Duration {
    const NANOS_PER_SECOND: i128 = 1_000_000_000;
    let elapsed = now.as_nanosecond().rem_euclid(NANOS_PER_SECOND);
    Duration::from_nanos((NANOS_PER_SECOND - elapsed) as u64)
}
