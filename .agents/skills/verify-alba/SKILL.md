---
name: verify-alba
description: Drive the real alba binary, headless in a pseudo-terminal or in a Ghostty window, and capture evidence for a pull request. Use for a ticket's `drive:` line, palette screenshots in light and dark themes, offline and stale checks, quit and terminal restore, resize, tick timing, or the CPU check.
---

# Verify alba

alba is a TUI with no server. Each drive builds the release binary once, writes a scratch `HOME`, and starts its own alba there. Two harnesses drive it:

- **Headless** (`scripts/pty_drive.py`): a pseudo-terminal with a screen model. It reads the text grid, types keys, resizes, and checks quit and terminal restore. It needs no permission. Reach for it first.
- **Ghostty** (`scripts/ghostty.sh`): a real Ghostty window, captured as a PNG. Reach for it when the proof is how the pane looks: colors, faint text, pixel art, a theme.

Read [`features/README.md`](features/README.md), then the feature file that matches the change. The feature files hold the recipes.

All paths below are relative to the repository root. `S=.agents/skills/verify-alba/scripts`.

## Launch

1. Build: `cargo build --release`.
2. Make a scratch `HOME`: `mktemp -d /tmp/alba-verify.XXXXXX`. Note the literal path it prints; later steps call it `HOME`.
3. Write a config and a weather cache into it: `$S/fixture.py HOME [--weather fresh|stale|expired|none] [--age MINUTES] [--code WMO] [--night] [--no-location] [--config 'key = value' ...]`. The cache is `tests/fixtures/forecast.json` (Buenos Aires, rain, 14°, day), shifted to today. `--weather none` writes no cache, for the offline start.
4. Start alba in one harness:
   - Headless: `$S/pty_drive.py HOME --cols 40 --rows 12 [--offline] STEP ...`. It runs the steps, presses `q`, and prints the exit status, `terminal restored: yes|no`, and the CPU seconds alba used. Steps are listed in `$S/pty_drive.py --help`. It is ready when it prints; a failed `expect` or a dirty exit makes it exit 1.
   - Ghostty: `$S/ghostty.sh launch HOME --theme 'Builtin Dark' --cols 85 --rows 26 [--offline]`. It is ready when it prints `alba <pid> in Ghostty <pid>: …`. The window title names the theme, the size, and `offline`.

`--offline` runs alba under `sandbox-exec` with every network call denied. Use it for stale weather and the offline start: without it, the startup fetch replaces the fixture.

## Doctor

Run `$S/ghostty.sh doctor` first, and again whenever a drive looks wrong. It prints the Ghostty and alba versions, says whether `src/` is newer than the release build, and lists each `/tmp/alba-verify.*` run that has not been stopped, with its alba pid if it still runs. Drive only runs you started in this session. If alba misbehaves, read its log: `env HOME=HOME XDG_CONFIG_HOME= XDG_CACHE_HOME= XDG_STATE_HOME= target/release/alba --log`.

## Drive

- **Headless:** pass steps in order, such as `sleep=2 screen key=r resize=60x20 expect=°`. `screen` prints the grid inside a `+---+` frame, one row per line, so a row and column are countable.
- **Ghostty:** alba takes keys only from the focused window, so a Ghostty drive is launch, wait, and capture. Drive keys and resizes headless.
- **CPU:** launch in Ghostty at 85×26 with `--config 'motion = true'`, wait 300 seconds, then run `ps -o time= -p $(cat HOME/alba.pid)`. The spec bound is under 3 seconds.

## Evidence

Put evidence in `target/verify/<ticket>/`. `target/` is ignored by git, and cleanup never touches it. Commit no PNGs.

- **Screenshot:** `$S/ghostty.sh shot HOME target/verify/<ticket>/<theme>-<cols>x<rows>-<state>.png`. Read the PNG before you attach it: it must show the alba window and the state you claim.
- **Headless:** save the printed grid and summary with `| tee target/verify/<ticket>/<name>.txt`. Add `--transcript FILE` to keep alba's output as timed JSON lines, to find which cells a redraw wrote and when.
- **Attach** with `gh pr create|edit|comment --attach 'target/verify/<ticket>/<file>.png#<theme>, <cols>x<rows>, <state>'`. The alt text names the theme, the size, and the state.
- Record each check and its result in the pull request body. A UI pull request attaches four palette shots (Builtin Light and Builtin Dark, 85×26 and 40×12) plus one shot per state or condition it changes.

Prove the user path: the real binary, a config file, and a cache file. A side effect counts as proof only when you read it back, such as the log after an offline run.

## Cleanup

1. Stop each Ghostty run: `$S/ghostty.sh stop HOME`. It kills the alba it started, then that Ghostty if it lingers. A headless run stops itself.
2. Run `$S/ghostty.sh doctor` and confirm no run of yours still runs.
3. Delete each scratch `HOME` by its literal path: `rm -r /tmp/alba-verify.AbC123`.
4. Confirm the evidence is still in `target/verify/<ticket>/`.

## Setup and gotchas

- **Screen Recording:** the terminal that runs the agent needs macOS Screen Recording permission, granted once by the user in System Settings → Privacy & Security → Screen Recording. Without it, a shot can miss the window content, so read every PNG. The headless harness needs no permission.
- **Ghostty config:** each launch is a separate Ghostty process with `--config-default-files=false`. The user's font, theme, and padding never reach a shot, so shots compare across runs. The themes are Ghostty's built-ins; `ghostty +list-themes` lists them.
- **Window managers:** a tiling window manager can resize the new window and move it. `launch` reads alba's pane size from its terminal and prints a warning when the size is not the requested size. Then the shot is not valid. Ask the user to make the window float: it is app `com.mitchellh.ghostty`, and its title starts with `alba · `. `shot` reads the window's bounds each time, so the position does not matter.
- **Ghostty arguments:** pass the command as `sh -c`, never as a bare file path. macOS reads a path argument as a file to open, and Ghostty asks to run it a second time.
- **Headless driver tests:** after you change `$S/pty_drive.py`, run `python3 -I -m unittest discover -s .agents/skills/verify-alba/scripts`. Its tests guard the three driver hangs from #17: a key flood, an endless read after exit, and a deadlock in exit.
