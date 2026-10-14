"""Tests for pty_drive.py. Run: python3 -I -m unittest discover -s .agents/skills/verify-alba/scripts"""

import json
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

from pty_drive import BINARY, Screen

SCRIPTS = Path(__file__).resolve().parent
# Leaves the alternate screen and shows the cursor, but stays in raw mode.
RAW_EXIT = """#!/bin/sh
stty raw -echo
printf '\\033[?1049h\\033[?1049l\\033[?25h'
"""


def drive(*arguments, fixture=()):
    """Runs the driver on a fresh fixture home; returns its result and how long it took."""
    with tempfile.TemporaryDirectory(prefix="alba-verify.") as home:
        subprocess.run(
            [sys.executable, "-I", SCRIPTS / "fixture.py", home, *fixture], check=True, capture_output=True
        )
        started = time.monotonic()
        result = subprocess.run(
            [sys.executable, "-I", "-u", SCRIPTS / "pty_drive.py", home, *arguments],
            capture_output=True,
            text=True,
            timeout=30,
        )
        return result, time.monotonic() - started


class ScreenTest(unittest.TestCase):
    def test_positioned_write_lands_at_its_row_and_column(self):
        screen = Screen(10, 3)
        screen.feed(b"\x1b[2;4Hab\x1b[1;1Hz")
        self.assertEqual(screen.lines(), ["z", "   ab", ""])

    def test_escape_split_across_reads_is_applied_once_whole(self):
        screen = Screen(10, 2)
        screen.feed(b"\x1b[2;")
        screen.feed(b"3H\x1b[1mx\x1b[0m")
        self.assertEqual(screen.lines(), ["", "  x"])

    def test_utf8_split_across_reads_is_one_character(self):
        screen = Screen(10, 1)
        degree = "°".encode()
        screen.feed(b"14" + degree[:1])
        screen.feed(degree[1:])
        self.assertEqual(screen.lines(), ["14°"])

    def test_clear_screen_empties_every_cell(self):
        screen = Screen(4, 2)
        screen.feed(b"abcd\x1b[2J\x1b[1;2Hx")
        self.assertEqual(screen.lines(), [" x", ""])

    def test_private_modes_change_no_cell(self):
        screen = Screen(4, 1)
        screen.feed(b"\x1b[?1049h\x1b[?25lab\x1b]0;title\x07")
        self.assertEqual(screen.lines(), ["ab"])


class StepsTest(unittest.TestCase):
    def test_a_bad_step_fails_before_launch(self):
        result, _ = drive("screen", "resize=wide")
        self.assertEqual(result.returncode, 2)
        self.assertIn("bad value in 'resize=wide'", result.stderr)
        self.assertNotIn("exit status", result.stdout)

    def test_raw_mode_left_behind_is_not_restored(self):
        with tempfile.TemporaryDirectory() as directory:
            raw_exit = Path(directory) / "raw_exit"
            raw_exit.write_text(RAW_EXIT)
            raw_exit.chmod(0o755)
            result, _ = drive("--binary", str(raw_exit))
        self.assertIn("exit status: 0", result.stdout)
        self.assertIn("terminal restored: no", result.stdout)
        self.assertEqual(result.returncode, 1)


@unittest.skipUnless(BINARY.exists(), "needs cargo build --release")
class DriveAlbaTest(unittest.TestCase):
    """Each case is a way a pseudo-terminal driver can hang: it must end well inside its timeout."""

    def test_quit_restores_the_terminal(self):
        result, _ = drive("--cols", "40", "--rows", "12", "expect=°")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("terminal restored: yes", result.stdout)

    def test_quit_after_a_key_flood_exits_at_once(self):
        result, seconds = drive("flood=2")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertLess(seconds, 10)

    def test_resize_redraws_the_line_on_the_new_middle_row(self):
        result, _ = drive("--cols", "40", "--rows", "12", "resize=60x20", "sleep=0.3", "screen")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        rows = result.stdout.split("+" + "-" * 60 + "+")[1].strip().splitlines()
        self.assertEqual(len(rows), 20)
        self.assertIn("°", rows[10])

    def test_a_config_error_exits_without_hanging(self):
        result, _ = drive(fixture=("--config", "colour = 1"))
        self.assertEqual(result.returncode, 1)
        self.assertIn("exit status: 1", result.stdout)

    def test_transcript_times_each_read(self):
        with tempfile.TemporaryDirectory() as directory:
            transcript = Path(directory) / "transcript.jsonl"
            started = time.time()
            result, _ = drive("--transcript", str(transcript))
            reads = [json.loads(line) for line in transcript.read_text().splitlines()]
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        times = [read["time"] for read in reads]
        self.assertEqual(times, sorted(times))
        self.assertGreater(times[0], started)
        self.assertIn("°", "".join(read["output"] for read in reads))


if __name__ == "__main__":
    unittest.main()
