"""Tests for pty_drive.py. Run: python3 -I -m unittest discover -s .agents/skills/verify-alba/scripts"""

import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

from pty_drive import BINARY, Screen

SCRIPTS = Path(__file__).resolve().parent


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


@unittest.skipUnless(BINARY.exists(), "needs cargo build --release")
class DriveAlbaTest(unittest.TestCase):
    """Each case is one hang from #17: it must end well inside its timeout."""

    def drive(self, *arguments):
        with tempfile.TemporaryDirectory(prefix="alba-verify.") as home:
            subprocess.run([sys.executable, "-I", SCRIPTS / "fixture.py", home], check=True, capture_output=True)
            started = time.monotonic()
            result = subprocess.run(
                [sys.executable, "-I", "-u", SCRIPTS / "pty_drive.py", home, *arguments],
                capture_output=True,
                text=True,
                timeout=30,
            )
            return result, time.monotonic() - started

    def test_quit_restores_the_terminal(self):
        result, _ = self.drive("--cols", "40", "--rows", "12", "expect=°")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("terminal restored: yes", result.stdout)

    def test_quit_after_a_key_flood_exits_at_once(self):
        result, seconds = self.drive("flood=2")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertLess(seconds, 10)

    def test_resize_redraws_the_line_on_the_new_middle_row(self):
        result, _ = self.drive("--cols", "40", "--rows", "12", "resize=60x20", "sleep=0.3", "screen")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        rows = result.stdout.split("+" + "-" * 60 + "+")[1].strip().splitlines()
        self.assertEqual(len(rows), 20)
        self.assertIn("°", rows[10])

    def test_a_config_error_exits_without_hanging(self):
        with tempfile.TemporaryDirectory(prefix="alba-verify.") as home:
            subprocess.run(
                [sys.executable, "-I", SCRIPTS / "fixture.py", home, "--config", "colour = 1"],
                check=True,
                capture_output=True,
            )
            result = subprocess.run(
                [sys.executable, "-I", "-u", SCRIPTS / "pty_drive.py", home],
                capture_output=True,
                text=True,
                timeout=30,
            )
        self.assertEqual(result.returncode, 1)
        self.assertIn("exit status: 1", result.stdout)


if __name__ == "__main__":
    unittest.main()
