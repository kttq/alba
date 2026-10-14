use std::{
    io::{self, Write},
    process::ExitCode,
    time::Duration,
};

use alba::{
    app::{App, Control},
    config, log,
    paths::Paths,
    weather::{BackgroundSource, open_meteo},
};
use jiff::{SignedDuration, Timestamp, Zoned};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};

const HELP: &str = "\
alba: a clock, a month calendar, and the current weather in a terminal pane.

usage: alba [--help | --version | --log]

--log prints the log of the running alba, or of the last run, in logfmt.

keys: q, esc, or ctrl-c quits.";

enum Command {
    Run,
    Help,
    Version,
    Log,
}

fn main() -> ExitCode {
    let result = match parse_args(std::env::args().skip(1)) {
        Ok(Command::Run) => start(),
        Ok(Command::Help) => {
            println!("{HELP}");
            Ok(())
        }
        Ok(Command::Version) => {
            println!("alba {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Ok(Command::Log) => print_log(),
        Err(argument) => Err(format!(
            "error: unexpected argument '{argument}'\n\nrun 'alba --help' for usage"
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Loads the config before raw mode, so its errors print to a normal terminal.
fn start() -> Result<(), String> {
    let paths = paths()?;
    let config = config::load(&paths.config_file).map_err(|error| error.to_string())?;
    let weather_source = Box::new(BackgroundSource::new(open_meteo::fetch));
    let mut app = App::new(
        &config,
        weather_source,
        paths.cache_directory,
        paths.state_directory,
    );
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result.map_err(|error| format!("error: {error}"))
}

fn paths() -> Result<Paths, String> {
    Paths::resolve(|name| std::env::var_os(name)).map_err(|error| format!("error: {error}"))
}

/// A reader that stops early, like `head`, closes the pipe; that is not an error.
fn print_log() -> Result<(), String> {
    let directory = paths()?.state_directory;
    let text = log::read(&directory).map_err(|error| {
        format!(
            "error: cannot read the log in {}: {error}",
            directory.display()
        )
    })?;
    match io::stdout().write_all(text.as_bytes()) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(format!("error: {error}")),
        _ => Ok(()),
    }
}

/// The error is the first argument it rejects.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    let command = match args.next().as_deref() {
        None => Command::Run,
        Some("--help") => Command::Help,
        Some("--version") => Command::Version,
        Some("--log") => Command::Log,
        Some(other) => return Err(other.to_string()),
    };
    match args.next() {
        None => Ok(command),
        Some(extra) => Err(extra),
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut now = Zoned::now();
    app.tick(&now);
    loop {
        terminal.draw(|frame| app.render(frame.area(), frame.buffer_mut(), &now))?;
        let next_tick = next_tick(now.timestamp(), app.tick_interval());
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

fn next_tick(now: Timestamp, interval: Duration) -> Timestamp {
    let interval =
        SignedDuration::try_from(interval).expect("tick interval fits a signed duration");
    let into_interval = now.as_nanosecond().rem_euclid(interval.as_nanos()) as i64;
    now - SignedDuration::from_nanos(into_interval) + interval
}

fn until(deadline: Timestamp) -> Duration {
    Duration::try_from(deadline.duration_since(Timestamp::now())).unwrap_or(Duration::ZERO)
}
