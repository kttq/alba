#!/usr/bin/env -S python3 -u
"""Drive alba headless in a pseudo-terminal, and print what its screen shows.

usage: pty_drive.py HOME [--cols 40] [--rows 12] [--offline] [--binary PATH]
                         [--transcript FILE] [STEP ...]

Steps run in order, then the driver presses q and waits for alba to exit:
  sleep=SECONDS     read output for that long
  key=CHARS         type the characters, 50 ms apart (esc and ctrl-c: key=\\x1b, key=\\x03)
  resize=COLSxROWS  resize the pane
  screen            print the screen grid
  expect=TEXT       fail unless the screen contains TEXT
  flood=SECONDS     type r at 30 keys a second for that long

It exits 0 only when every expect passed, alba exited 0, and the terminal was restored:
the alternate screen left, the cursor shown, and the line discipline back in cooked mode.
--transcript writes one JSON object per read, {"time": unix seconds, "output": text}.
"""

import argparse
import codecs
import fcntl
import json
import os
import pty
import re
import resource
import select
import signal
import struct
import sys
import termios
import time
import unicodedata
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
BINARY = REPO / "target/release/alba"
# Denies every network call; the terminal is not a network, so drawing still works.
OFFLINE_PROFILE = "(version 1)(allow default)(deny network*)"
KEY_GAP = 0.05
FLOOD_GAP = 1 / 30
EXIT_DEADLINE = 5.0
ENTER_ALTERNATE = b"\x1b[?1049h"
LEAVE_ALTERNATE = b"\x1b[?1049l"
SHOW_CURSOR = b"\x1b[?25h"
STEPS = ("sleep", "key", "flood", "resize", "screen", "expect")


class Screen:
    """Enough of a terminal to read ratatui's output: cursor moves, clears, and text."""

    def __init__(self, cols, rows):
        self.resize(cols, rows)
        self.decoder = codecs.getincrementaldecoder("utf-8")(errors="replace")
        self.pending = ""

    def resize(self, cols, rows):
        self.cols, self.rows = cols, rows
        self.cells = [self._blank_row() for _ in range(rows)]
        self.row = self.col = 0

    def feed(self, data):
        text = self.pending + self.decoder.decode(data)
        self.pending = ""
        index = 0
        while index < len(text):
            char = text[index]
            if char == "\x1b":
                end = self._escape(text, index)
                if end is None:
                    self.pending = text[index:]
                    return
                index = end
                continue
            if char == "\r":
                self.col = 0
            elif char == "\n":
                self.row = min(self.row + 1, self.rows - 1)
            elif char == "\b":
                self.col = max(self.col - 1, 0)
            elif char >= " ":
                self._put(char)
            index += 1

    def lines(self):
        return ["".join(row).rstrip() for row in self.cells]

    def text(self):
        return "\n".join(self.lines())

    def _put(self, char):
        width = 2 if unicodedata.east_asian_width(char) in ("W", "F") else 1
        if self.row < self.rows and self.col < self.cols:
            self.cells[self.row][self.col] = char
            if width == 2 and self.col + 1 < self.cols:
                self.cells[self.row][self.col + 1] = ""
        self.col += width

    def _escape(self, text, start):
        """Applies one escape sequence and returns the index after it, or None if it is cut off."""
        if start + 1 >= len(text):
            return None
        kind = text[start + 1]
        if kind == "[":
            match = re.compile(r"\x1b\[([?>=]?)([0-9;:]*)([ -/]*)([@-~])").match(text, start)
            if match is None:
                return None
            private, params, _, final = match.groups()
            if not private:
                self._csi([int(part) if part else 0 for part in params.split(";")], final)
            return match.end()
        if kind == "]":
            for terminator in ("\x07", "\x1b\\"):
                end = text.find(terminator, start)
                if end != -1:
                    return end + len(terminator)
            return None
        return start + 2

    def _csi(self, params, final):
        first = params[0] if params else 0
        count = max(first, 1)
        if final in "Hf":
            row = params[0] if params and params[0] else 1
            col = params[1] if len(params) > 1 and params[1] else 1
            self.row, self.col = min(row, self.rows) - 1, min(col, self.cols) - 1
        elif final == "A":
            self.row = max(self.row - count, 0)
        elif final == "B":
            self.row = min(self.row + count, self.rows - 1)
        elif final == "C":
            self.col = min(self.col + count, self.cols - 1)
        elif final == "D":
            self.col = max(self.col - count, 0)
        elif final == "G":
            self.col = min(count, self.cols) - 1
        elif final == "J" and first == 2:
            self.cells = [self._blank_row() for _ in range(self.rows)]
        elif final == "J" and first == 0:
            self.cells[self.row][self.col :] = [" "] * (self.cols - self.col)
            for row in range(self.row + 1, self.rows):
                self.cells[row] = self._blank_row()
        elif final == "K" and first == 0:
            self.cells[self.row][self.col :] = [" "] * (self.cols - self.col)
        elif final == "K" and first == 2:
            self.cells[self.row] = self._blank_row()

    def _blank_row(self):
        return [" "] * self.cols


class Alba:
    """One alba process on a pseudo-terminal. Never stops reading, so alba can always exit."""

    def __init__(self, home, cols, rows, offline, binary):
        self.screen = Screen(cols, rows)
        self.reads = []
        self.eof = False
        self.status = None
        command = sandboxed([str(binary)]) if offline else [str(binary)]
        environment = {
            "HOME": str(home),
            "XDG_CONFIG_HOME": "",
            "XDG_CACHE_HOME": "",
            "XDG_STATE_HOME": "",
            "TERM": "xterm-256color",
            "PATH": "/usr/bin:/bin",
        }
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            set_size(0, cols, rows)
            os.execve(command[0], command, environment)
        set_size(self.fd, cols, rows)

    def pump(self, seconds):
        """Reads output for this long. An empty read or an error is the end of the output."""
        deadline = time.monotonic() + seconds
        while (remaining := deadline - time.monotonic()) > 0:
            if self.eof:
                time.sleep(min(remaining, 0.05))
                continue
            ready, _, _ = select.select([self.fd], [], [], min(remaining, 0.05))
            if not ready:
                continue
            try:
                data = os.read(self.fd, 65536)
            except OSError:
                data = b""
            if not data:
                self.eof = True
                continue
            self.reads.append((time.time(), data))
            self.screen.feed(data)

    def output(self):
        return b"".join(data for _, data in self.reads)

    def cooked(self):
        """Reads the line discipline from the master side, which macOS allows after alba exits."""
        try:
            iflag, oflag, _, lflag, *_ = termios.tcgetattr(self.fd)
        except termios.error:
            return False
        lflags = termios.ECHO | termios.ICANON | termios.ISIG
        return lflag & lflags == lflags and bool(oflag & termios.OPOST) and bool(iflag & termios.ICRNL)

    def type(self, chars, gap):
        """Paces keys with sleep: alba draws often, so the pseudo-terminal is always ready."""
        for char in chars:
            os.write(self.fd, char.encode())
            self.pump(gap)

    def resize(self, cols, rows):
        set_size(self.fd, cols, rows)
        self.screen.resize(cols, rows)
        os.kill(self.pid, signal.SIGWINCH)

    def quit(self):
        """Keeps reading after q: alba blocks in exit until its last output drains."""
        if not self.eof:
            try:
                os.write(self.fd, b"q")
            except OSError:
                pass
        deadline = time.monotonic() + EXIT_DEADLINE
        while time.monotonic() < deadline:
            pid, status = os.waitpid(self.pid, os.WNOHANG)
            if pid:
                self.status = os.waitstatus_to_exitcode(status)
                self.pump(0.05)
                return self.status
            self.pump(0.05)
        os.kill(self.pid, signal.SIGKILL)
        os.waitpid(self.pid, 0)
        return None


def sandboxed(command):
    """Runs the command under macOS sandbox-exec with every network call denied."""
    return ["/usr/bin/sandbox-exec", "-p", OFFLINE_PROFILE] + command


def set_size(fd, cols, rows):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def parse_step(text):
    """Turns `name=value` into (name, value), with the value in the type its step uses."""
    name, _, value = text.partition("=")
    try:
        if name in ("sleep", "flood"):
            return name, float(value)
        if name == "key":
            return name, value.encode().decode("unicode_escape")
        if name == "resize":
            cols, rows = (int(part) for part in value.split("x"))
            return name, (cols, rows)
        if name in ("screen", "expect"):
            return name, value
    except ValueError:
        raise argparse.ArgumentTypeError(f"bad value in {text!r}")
    raise argparse.ArgumentTypeError(f"unknown step {text!r}; steps are {', '.join(STEPS)}")


def run(alba, steps, out):
    """Returns the number of failed expects."""
    failures = 0
    for name, value in steps:
        if name == "sleep":
            alba.pump(value)
        elif name == "key":
            alba.type(value, KEY_GAP)
        elif name == "flood":
            alba.type("r" * round(value / FLOOD_GAP), FLOOD_GAP)
        elif name == "resize":
            alba.resize(*value)
        elif name == "screen":
            print_screen(alba.screen, out)
        elif name == "expect":
            failures += not expect(alba.screen, value, out)
    return failures


def expect(screen, text, out):
    found = text in screen.text()
    print(f"expect {text!r}: {'ok' if found else 'MISSING'}", file=out)
    return found


def screen_restored(output):
    """The last alternate-screen entry is followed by its exit and a shown cursor."""
    entered = output.rfind(ENTER_ALTERNATE)
    return entered != -1 and output.rfind(LEAVE_ALTERNATE) > entered and output.rfind(SHOW_CURSOR) > entered


def write_transcript(path, reads):
    lines = (json.dumps({"time": at, "output": data.decode("utf-8", "replace")}) for at, data in reads)
    path.write_text("\n".join(lines) + "\n")


def print_screen(screen, out):
    border = "+" + "-" * screen.cols + "+"
    print(f"screen {screen.cols}x{screen.rows}", file=out)
    print(border, file=out)
    for line in screen.lines():
        print("|" + line.ljust(screen.cols) + "|", file=out)
    print(border, file=out)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("home", type=Path)
    parser.add_argument("--cols", type=int, default=40)
    parser.add_argument("--rows", type=int, default=12)
    parser.add_argument("--offline", action="store_true", help="run under sandbox-exec, network denied")
    parser.add_argument("--binary", type=Path, default=BINARY)
    parser.add_argument("--transcript", type=Path, help="write alba's timed output here, as JSON lines")
    parser.add_argument("steps", nargs="*", type=parse_step)
    args = parser.parse_args()

    alba = Alba(args.home, args.cols, args.rows, args.offline, args.binary)
    alba.pump(1.0)
    failures = run(alba, args.steps, sys.stdout)
    status = alba.quit()
    cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
    if args.transcript:
        write_transcript(args.transcript, alba.reads)

    restored = screen_restored(alba.output()) and alba.cooked()
    print(f"exit status: {'killed after timeout' if status is None else status}")
    print(f"terminal restored: {'yes' if restored else 'no'}")
    print(f"cpu seconds: {cpu.ru_utime + cpu.ru_stime:.2f}")
    if failures or status != 0 or not restored:
        if status not in (0, None):
            print(alba.screen.text(), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
