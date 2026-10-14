//! alba on a pseudo-terminal, read through a `vt100` screen.
//!
//! A background thread reads the terminal until its end, so alba can always finish its last
//! write and exit, whatever the caller waits on.

use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::BorrowedFd,
        unix::{ffi::OsStrExt, fs::OpenOptionsExt, process::CommandExt},
    },
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    str::FromStr,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant, SystemTime},
};

use rustix::{
    io::{FdFlags, fcntl_setfd},
    process::{Pid, Signal, ioctl_tiocsctty, kill_process, setsid},
    pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt},
    termios::{InputModes, LocalModes, OutputModes, Winsize, tcgetattr, tcsetwinsize},
};

pub const EXITED_WITHIN: Duration = Duration::from_secs(2);
const EXIT_DEADLINE: Duration = Duration::from_secs(5);
const KEY_GAP: Duration = Duration::from_millis(50);
/// 30 keys a second.
pub const FLOOD_GAP: Duration = Duration::from_nanos(1_000_000_000 / 30);
/// Generous for a slow runner: a wait ends as soon as its condition holds.
pub const WAIT_DEADLINE: Duration = Duration::from_secs(5);
const DRAIN_DEADLINE: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_millis(50);
/// Denies every network call; the terminal is not a network, so drawing still works.
const OFFLINE_PROFILE: &str = "(version 1)(allow default)(deny network*)";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneSize {
    pub cols: u16,
    pub rows: u16,
}

pub struct Launch {
    pub binary: PathBuf,
    pub home: PathBuf,
    pub pane_size: PaneSize,
    /// Runs alba under macOS `sandbox-exec` with the network denied.
    pub offline: bool,
}

/// One read from the terminal.
pub struct Output {
    pub time: SystemTime,
    pub bytes: Vec<u8>,
}

pub struct Alba {
    child: Child,
    master: File,
    outputs: Receiver<Output>,
    parser: vt100::Parser,
    transcript: Vec<Output>,
    status: Option<ExitStatus>,
    cpu_seconds: Option<f64>,
    output_ended: bool,
}

impl Alba {
    pub fn spawn(launch: &Launch) -> io::Result<Self> {
        let master = open_master()?;
        let terminal = open_terminal(&master)?;
        // macOS sets no size on the master before the terminal side is open.
        tcsetwinsize(&terminal, winsize(launch.pane_size))?;
        // The command holds copies of the terminal; it drops them here, so the terminal ends
        // when alba exits.
        let child = command(launch, &terminal)?.spawn()?;
        drop(terminal);
        let (sender, outputs) = mpsc::channel();
        let reader = master.try_clone()?;
        thread::spawn(move || read_until_end(reader, &sender));
        Ok(Self {
            child,
            master,
            outputs,
            parser: vt100::Parser::new(launch.pane_size.rows, launch.pane_size.cols, 0),
            transcript: Vec::new(),
            status: None,
            cpu_seconds: None,
            output_ended: false,
        })
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn transcript(&self) -> &[Output] {
        &self.transcript
    }

    /// Takes output for this long.
    pub fn pump(&mut self, duration: Duration) {
        let deadline = Instant::now() + duration;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return;
            }
            match self.outputs.recv_timeout(remaining) {
                Ok(output) => self.take(output),
                Err(RecvTimeoutError::Timeout) => return,
                Err(RecvTimeoutError::Disconnected) => {
                    self.output_ended = true;
                    return thread::sleep(remaining);
                }
            }
        }
    }

    /// Takes output until the screen shows the text, for at most the wait deadline.
    pub fn wait_for(&mut self, text: &str) -> bool {
        self.wait_until(|screen| screen.contents().contains(text))
    }

    /// Takes output until the screen shows something, or the output ends.
    pub fn wait_for_first_frame(&mut self) -> bool {
        self.wait_until(|screen| !screen.contents().trim().is_empty())
    }

    /// Takes output until the condition holds for the screen, for at most the wait deadline.
    /// Gives up early when the output has ended, since the screen cannot change any more.
    pub fn wait_until(&mut self, condition: impl Fn(&vt100::Screen) -> bool) -> bool {
        let deadline = Instant::now() + WAIT_DEADLINE;
        while !condition(self.screen()) {
            if self.output_ended || Instant::now() >= deadline {
                return false;
            }
            self.pump(POLL);
        }
        true
    }

    /// Paces keys with a sleep: alba draws often, so the terminal is always ready for input.
    pub fn type_keys(&mut self, keys: &str) {
        self.type_keys_apart(keys, KEY_GAP);
    }

    pub fn type_keys_apart(&mut self, keys: &str, gap: Duration) {
        for key in keys.chars() {
            // A key after alba exited goes nowhere; the steps after it report the exit.
            let _ = self
                .master
                .write_all(key.encode_utf8(&mut [0; 4]).as_bytes());
            self.pump(gap);
        }
    }

    pub fn resize(&mut self, pane_size: PaneSize) -> io::Result<()> {
        tcsetwinsize(&self.master, winsize(pane_size))?;
        self.parser
            .screen_mut()
            .set_size(pane_size.rows, pane_size.cols);
        kill_process(self.pid(), Signal::WINCH)?;
        Ok(())
    }

    /// Keeps taking output while it waits, then takes the rest.
    pub fn exited(&mut self, within: Duration) -> bool {
        if self.status.is_none() {
            self.cpu_seconds = cpu_seconds(self.child.id());
        }
        let deadline = Instant::now() + within;
        while self.status.is_none() && Instant::now() < deadline {
            self.status = self.child.try_wait().ok().flatten();
            self.pump(POLL);
        }
        if self.status.is_some() {
            self.pump_to_end();
        }
        self.status.is_some()
    }

    /// Presses `q` only while alba runs, so a quit key under test gets no help. Kills alba when
    /// it does not exit in time, and then returns `None`.
    pub fn quit(&mut self) -> Option<ExitStatus> {
        if self.status.is_none() {
            let _ = self.master.write_all(b"q");
        }
        if !self.exited(EXIT_DEADLINE) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        self.status
    }

    /// The CPU time alba has used, or had used when the harness began to wait for its exit.
    pub fn cpu_seconds(&self) -> Option<f64> {
        match self.status {
            None => cpu_seconds(self.child.id()),
            Some(_) => self.cpu_seconds,
        }
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.status.and_then(|status| status.code())
    }

    /// The alternate screen left, the cursor shown, and cooked mode back on.
    pub fn restored(&self) -> bool {
        screen_restored(self.screen()) && self.cooked()
    }

    /// Reads the terminal's modes through the master side, which works after alba exits.
    fn cooked(&self) -> bool {
        let Ok(termios) = tcgetattr(&self.master) else {
            return false;
        };
        termios
            .local_modes
            .contains(LocalModes::ECHO | LocalModes::ICANON | LocalModes::ISIG)
            && termios.output_modes.contains(OutputModes::OPOST)
            && termios.input_modes.contains(InputModes::ICRNL)
    }

    fn pid(&self) -> Pid {
        Pid::from_child(&self.child)
    }

    fn take(&mut self, output: Output) {
        self.parser.process(&output.bytes);
        self.transcript.push(output);
    }

    /// Takes output until the reader reaches the end, a poll passes with no read, or the
    /// deadline passes: a process that inherited the terminal can keep it open.
    fn pump_to_end(&mut self) {
        let deadline = Instant::now() + DRAIN_DEADLINE;
        while Instant::now() < deadline {
            let Ok(output) = self.outputs.recv_timeout(POLL) else {
                return;
            };
            self.take(output);
        }
    }
}

impl Drop for Alba {
    fn drop(&mut self) {
        if self.status.is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// A reaped process has no CPU time to read, so this asks `ps` while alba runs.
fn cpu_seconds(pid: u32) -> Option<f64> {
    let output = Command::new("ps")
        .args(["-o", "time=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    parse_cpu_time(&String::from_utf8_lossy(&output.stdout))
}

/// Reads `ps` time: seconds, optionally with a fraction, after optional hours and minutes.
fn parse_cpu_time(text: &str) -> Option<f64> {
    text.trim().split(':').try_fold(0.0, |total, field| {
        Some(total * 60.0 + field.parse::<f64>().ok()?)
    })
}

/// Every mode a program can switch is back at its default; the last command for each wins.
pub fn screen_restored(screen: &vt100::Screen) -> bool {
    !screen.alternate_screen()
        && !screen.hide_cursor()
        && !screen.application_cursor()
        && !screen.application_keypad()
        && !screen.bracketed_paste()
        && screen.mouse_protocol_mode() == vt100::MouseProtocolMode::None
}

/// An empty read or an error is the end: on macOS, a terminal whose process has exited reads
/// as ready and returns nothing.
fn read_until_end(mut master: File, sender: &Sender<Output>) {
    let mut buffer = [0; 65_536];
    loop {
        let read = match master.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Ok(0) | Err(_) => return,
            Ok(read) => read,
        };
        let output = Output {
            time: SystemTime::now(),
            bytes: buffer[..read].to_vec(),
        };
        if sender.send(output).is_err() {
            return;
        }
    }
}

fn open_master() -> io::Result<File> {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    // macOS opens it without close-on-exec, and another test's alba must not inherit it.
    fcntl_setfd(&master, FdFlags::CLOEXEC)?;
    grantpt(&master)?;
    unlockpt(&master)?;
    Ok(File::from(master))
}

/// The terminal side, which alba gets as its standard streams.
fn open_terminal(master: &File) -> io::Result<File> {
    let path = ptsname(master, Vec::new())?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(OpenptFlags::NOCTTY.bits() as i32)
        .open(OsStr::from_bytes(path.as_bytes()))
}

fn command(launch: &Launch, terminal: &File) -> io::Result<Command> {
    let mut command = if launch.offline {
        sandboxed(&launch.binary)?
    } else {
        Command::new(&launch.binary)
    };
    command
        .env_clear()
        .env("HOME", &launch.home)
        .env("XDG_CONFIG_HOME", "")
        .env("XDG_CACHE_HOME", "")
        .env("XDG_STATE_HOME", "")
        .env("TERM", "xterm-256color")
        .env("PATH", "/usr/bin:/bin")
        .stdin(terminal.try_clone()?)
        .stdout(terminal.try_clone()?)
        .stderr(terminal.try_clone()?);
    // SAFETY: the closure runs in the child between fork and exec, where only async-signal-safe
    // calls are sound. It makes two system calls, and it allocates and locks nothing.
    unsafe { command.pre_exec(take_terminal) };
    Ok(command)
}

/// Makes alba a session leader with the terminal as its controlling terminal, so the terminal
/// delivers ctrl-c and resizes to it. The standard streams are already on the terminal here.
fn take_terminal() -> io::Result<()> {
    setsid()?;
    // SAFETY: file descriptor 0 is open for the whole call; it is the terminal.
    ioctl_tiocsctty(unsafe { BorrowedFd::borrow_raw(0) })?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn sandboxed(binary: &Path) -> io::Result<Command> {
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command.args(["-p", OFFLINE_PROFILE]).arg(binary);
    Ok(command)
}

#[cfg(not(target_os = "macos"))]
fn sandboxed(_binary: &Path) -> io::Result<Command> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "offline runs need macOS sandbox-exec",
    ))
}

fn winsize(pane_size: PaneSize) -> Winsize {
    Winsize {
        ws_row: pane_size.rows,
        ws_col: pane_size.cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}

/// The `drive` example's command line, parsed before alba starts.
#[derive(Debug, PartialEq)]
pub struct Drive {
    pub home: PathBuf,
    pub pane_size: PaneSize,
    pub offline: bool,
    /// The alba beside the example when absent.
    pub binary: Option<PathBuf>,
    pub transcript: Option<PathBuf>,
    pub steps: Vec<Step>,
}

impl Drive {
    /// Options and steps may come in any order after HOME.
    pub fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut home = None;
        let mut drive = Self {
            home: PathBuf::new(),
            pane_size: PaneSize { cols: 40, rows: 12 },
            offline: false,
            binary: None,
            transcript: None,
            steps: Vec::new(),
        };
        while let Some(argument) = arguments.next() {
            let mut value = || {
                arguments
                    .next()
                    .ok_or_else(|| format!("{argument} needs a value"))
            };
            match argument.as_str() {
                "--cols" => drive.pane_size.cols = pane_length(&value()?)?,
                "--rows" => drive.pane_size.rows = pane_length(&value()?)?,
                "--offline" => drive.offline = true,
                "--binary" => drive.binary = Some(PathBuf::from(value()?)),
                "--transcript" => drive.transcript = Some(PathBuf::from(value()?)),
                option if option.starts_with("--") => {
                    return Err(format!("unknown option {option}"));
                }
                _ if home.is_none() => home = Some(PathBuf::from(argument)),
                step => drive.steps.push(step.parse()?),
            }
        }
        drive.home = home.ok_or("HOME is missing")?;
        Ok(drive)
    }
}

/// Columns or rows: a window size holds 1 to 65535.
pub fn pane_length(text: &str) -> Result<u16, String> {
    match text.parse() {
        Ok(0) | Err(_) => Err(format!("not a pane length from 1 to 65535: {text:?}")),
        Ok(length) => Ok(length),
    }
}

/// One step of a drive. The help text of the `drive` example lists them.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Sleep(Duration),
    Key(String),
    Flood(Duration),
    Resize(PaneSize),
    Screen,
    Colors,
    Expect(String),
    Exited,
}

impl FromStr for Step {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let bad_value = |reason: String| format!("bad value in {text:?}: {reason}");
        match text.split_once('=') {
            None => match text {
                "screen" => Ok(Self::Screen),
                "colors" => Ok(Self::Colors),
                "exited" => Ok(Self::Exited),
                _ => Err(unknown_step(text)),
            },
            Some(("sleep", value)) => seconds(value).map(Self::Sleep).map_err(bad_value),
            Some(("flood", value)) => seconds(value).map(Self::Flood).map_err(bad_value),
            Some(("key", value)) => unescape(value).map(Self::Key).map_err(bad_value),
            Some(("resize", value)) => pane_size(value).map(Self::Resize).map_err(bad_value),
            Some(("expect", value)) => Ok(Self::Expect(value.to_string())),
            Some(_) => Err(unknown_step(text)),
        }
    }
}

fn unknown_step(text: &str) -> String {
    format!(
        "unknown step {text:?}; steps are sleep=, key=, flood=, resize=, screen, colors, expect=, exited"
    )
}

fn seconds(text: &str) -> Result<Duration, String> {
    let seconds: f64 = text
        .parse()
        .map_err(|_| format!("{text:?} is not a number"))?;
    Duration::try_from_secs_f64(seconds).map_err(|_| format!("{text:?} is not a duration"))
}

fn pane_size(text: &str) -> Result<PaneSize, String> {
    let (cols, rows) = text
        .split_once('x')
        .ok_or_else(|| format!("{text:?} is not COLSxROWS"))?;
    Ok(PaneSize {
        cols: pane_length(cols)?,
        rows: pane_length(rows)?,
    })
}

/// Turns `\xHH` into its character and `\\` into one backslash.
fn unescape(text: &str) -> Result<String, String> {
    let mut keys = String::new();
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('\\') {
        keys.push_str(before);
        let (key, after) = escaped_key(after)?;
        keys.push(key);
        rest = after;
    }
    keys.push_str(rest);
    Ok(keys)
}

fn escaped_key(text: &str) -> Result<(char, &str), String> {
    if let Some(after) = text.strip_prefix('\\') {
        return Ok(('\\', after));
    }
    let code = text
        .strip_prefix('x')
        .and_then(|hex| hex.get(..2))
        .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        .filter(u8::is_ascii)
        .ok_or_else(|| format!("an escape must be \\xHH below \\x80 or \\\\, not \\{text}"))?;
    Ok((char::from(code), &text[3..]))
}

/// Steps run in order. Returns the number of steps that failed.
pub fn run_steps(alba: &mut Alba, steps: &[Step], out: &mut impl Write) -> io::Result<usize> {
    let mut failures = 0;
    for step in steps {
        match step {
            Step::Sleep(duration) => alba.pump(*duration),
            Step::Key(keys) => alba.type_keys(keys),
            Step::Flood(duration) => {
                let count = (duration.as_secs_f64() / FLOOD_GAP.as_secs_f64()).round() as usize;
                alba.type_keys_apart(&"r".repeat(count), FLOOD_GAP);
            }
            Step::Resize(pane_size) => alba.resize(*pane_size)?,
            Step::Screen => write!(out, "{}", framed(alba.screen()))?,
            Step::Colors => write!(out, "{}", colors(alba.screen()))?,
            Step::Expect(text) => failures += usize::from(!expect(alba.screen(), text, out)?),
            Step::Exited => failures += usize::from(!report_exited(alba, out)?),
        }
    }
    Ok(failures)
}

fn expect(screen: &vt100::Screen, text: &str, out: &mut impl Write) -> io::Result<bool> {
    let found = screen.contents().contains(text);
    writeln!(
        out,
        "expect {text:?}: {}",
        if found { "ok" } else { "MISSING" }
    )?;
    Ok(found)
}

fn report_exited(alba: &mut Alba, out: &mut impl Write) -> io::Result<bool> {
    let exited = alba.exited(EXITED_WITHIN);
    writeln!(
        out,
        "exited: {}",
        if exited { "ok" } else { "STILL RUNNING" }
    )?;
    Ok(exited)
}

/// The grid inside a `+---+` frame, one row per line, so a row and a column can be counted.
pub fn framed(screen: &vt100::Screen) -> String {
    let (rows, cols) = screen.size();
    let border = format!("+{}+\n", "-".repeat(usize::from(cols)));
    let lines: String = (0..rows)
        .map(|row| format!("|{}|\n", row_text(screen, row)))
        .collect();
    format!("screen {cols}x{rows}\n{border}{lines}{border}")
}

/// Every column of the row, with a blank for an empty cell; a wide character fills two.
fn row_text(screen: &vt100::Screen, row: u16) -> String {
    let (_, cols) = screen.size();
    (0..cols)
        .filter_map(|col| screen.cell(row, col))
        .filter(|cell| !cell.is_wide_continuation())
        .map(|cell| match cell.contents() {
            "" => " ",
            contents => contents,
        })
        .collect()
}

/// Neighbouring non-blank cells in one row that share a style.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledRun {
    pub row: u16,
    pub col: u16,
    pub text: String,
    pub foreground: vt100::Color,
    pub background: vt100::Color,
    pub dim: bool,
}

pub fn styled_runs(screen: &vt100::Screen) -> Vec<StyledRun> {
    let (rows, cols) = screen.size();
    let mut runs: Vec<StyledRun> = Vec::new();
    for row in 0..rows {
        let mut extends_run = false;
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if cell.is_wide_continuation() {
                continue;
            }
            if cell.contents().trim().is_empty() {
                extends_run = false;
                continue;
            }
            let run = StyledRun {
                row,
                col,
                text: cell.contents().to_string(),
                foreground: cell.fgcolor(),
                background: cell.bgcolor(),
                dim: cell.dim(),
            };
            match runs.last_mut() {
                Some(last) if extends_run && last.has_style_of(&run) => last.text += &run.text,
                _ => runs.push(run),
            }
            extends_run = true;
        }
    }
    runs
}

impl StyledRun {
    fn has_style_of(&self, other: &Self) -> bool {
        (self.foreground, self.background, self.dim)
            == (other.foreground, other.background, other.dim)
    }
}

/// One line per run: its row, column, text, colors, and `dim` when faint.
pub fn colors(screen: &vt100::Screen) -> String {
    styled_runs(screen)
        .iter()
        .map(|run| {
            let dim = if run.dim { " dim" } else { "" };
            format!(
                "row {} col {} {:?} fg={:?} bg={:?}{dim}\n",
                run.row, run.col, run.text, run.foreground, run.background
            )
        })
        .collect()
}

/// One JSON object per read: `{"time": unix seconds, "output": text}`. A character split
/// across reads belongs to the read that completes it.
pub fn transcript_lines(outputs: &[Output]) -> String {
    let mut pending = Vec::new();
    let mut lines = String::new();
    for output in outputs {
        pending.extend_from_slice(&output.bytes);
        let complete = complete_utf8_length(&pending);
        let text = String::from_utf8_lossy(&pending[..complete]).into_owned();
        pending.drain(..complete);
        let time = output
            .time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        lines += &format!("{}\n", serde_json::json!({ "time": time, "output": text }));
    }
    lines
}

/// The length of the bytes before a character that the next read may complete.
fn complete_utf8_length(bytes: &[u8]) -> usize {
    match std::str::from_utf8(bytes) {
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        _ => bytes.len(),
    }
}
