//! PROTOTYPE (issue #55), throwaway. Finds out whether the terminal shows 24-bit color, which
//! decides between the 24-bit icons and v1's. The terminal answers a query when it can; the
//! environment decides when it does not. Device attributes go last, because every terminal
//! answers them, so their answer ends the wait.

use std::{
    io::{self, Read, Write},
    time::{Duration, Instant},
};

use ratatui::crossterm::terminal;

#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub colorterm: String,
    /// Whether a 24-bit color set came back as set (DECRQSS), when the terminal answered.
    pub truecolor_query: Option<bool>,
    /// False when the terminal did not answer in time.
    pub answered: bool,
    pub raw: String,
}

impl Profile {
    pub fn truecolor(&self) -> bool {
        let environment = matches!(self.colorterm.as_str(), "truecolor" | "24bit");
        self.truecolor_query.unwrap_or(environment)
    }

    pub fn from_env() -> Self {
        Self {
            colorterm: std::env::var("COLORTERM").unwrap_or_default(),
            ..Self::default()
        }
    }

    /// Raw mode on, queries out, answers in; raw mode back off.
    pub fn detect() -> Self {
        let mut profile = Self::from_env();
        if terminal::enable_raw_mode().is_err() {
            return profile;
        }
        let answer = query();
        let _ = terminal::disable_raw_mode();
        if let Some(answer) = answer {
            profile.parse(&answer);
        }
        profile
    }

    fn parse(&mut self, answer: &str) {
        self.answered = true;
        self.raw = answer.replace('\x1b', "⎋");
        if let Some(start) = answer.find("$r") {
            let rest = &answer[start + 2..];
            let body = rest.find("\x1b\\").map_or(rest, |end| &rest[..end]);
            // Ghostty answers 38:2::1:2:3, with an empty color space; others 38;2;1;2;3.
            self.truecolor_query = Some(body.contains(":1:2:3") || body.contains("38;2;1;2;3"));
        }
    }

    pub fn report(&self) -> String {
        let query = match self.truecolor_query {
            Some(true) => "kept",
            Some(false) => "not kept",
            None => "no answer",
        };
        format!(
            "COLORTERM={}\nterminal: {}\n24-bit color set and read back (DECRQSS): {query}\nicons: {}\nraw: {}",
            self.colorterm,
            if self.answered {
                "answered"
            } else {
                "no answer within 300 ms"
            },
            if self.truecolor() { "24-bit" } else { "v1" },
            self.raw,
        )
    }
}

const QUERIES: &str = concat!(
    "\x1b[38;2;1;2;3m\x1bP$qm\x1b\\\x1b[0m", // set a 24-bit color, read it back
    "\x1b[c",                                // primary device attributes, the sentinel
);

/// Reads a separate, non-blocking open of the terminal until the device attributes arrive or
/// 300 ms pass, so nothing is left reading when the terminal never answers. A blocked reader
/// thread, tried first, ate the first key in the pseudo-terminal tests.
fn query() -> Option<String> {
    use std::os::unix::fs::OpenOptionsExt;
    #[cfg(target_os = "macos")]
    const O_NONBLOCK: i32 = 0x0004;
    #[cfg(not(target_os = "macos"))]
    const O_NONBLOCK: i32 = 0o4000;
    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(O_NONBLOCK)
        .open("/dev/tty")
        .ok()?;
    tty.write_all(QUERIES.as_bytes()).ok()?;
    tty.flush().ok()?;
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut answer = Vec::new();
    let mut chunk = [0u8; 1024];
    while Instant::now() < deadline {
        match tty.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                answer.extend_from_slice(&chunk[..read]);
                if has_device_attributes(&answer) {
                    return Some(String::from_utf8_lossy(&answer).into_owned());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(_) => return None,
        }
    }
    None
}

fn has_device_attributes(answer: &[u8]) -> bool {
    let text = String::from_utf8_lossy(answer);
    text.match_indices("\x1b[?").any(|(start, _)| {
        let rest = &text[start + 3..];
        let end = rest.find(|c: char| !(c.is_ascii_digit() || c == ';'));
        end.is_some_and(|end| rest[end..].starts_with('c'))
    })
}
