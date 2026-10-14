---
name: verify-alba
description: Drive the real alba binary, headless in a pseudo-terminal or in a macOS Terminal window, and capture evidence for a pull request. Use for a ticket's `drive:` line, palette screenshots in light and dark themes, offline and stale checks, quit and terminal restore, resize, tick timing, or the CPU check.
---

# Verify alba

alba is a TUI with no server. Each drive builds the release binary once, writes a scratch `HOME`, and starts its own alba there. Two harnesses drive it:

- **Headless** (`scripts/pty_drive.py`): a pseudo-terminal with a screen model. It reads the text grid, types keys, resizes, and checks quit and terminal restore. It sees the bytes every terminal receives, so its proof holds for any terminal. It needs no permission. Reach for it first.
- **Terminal** (`scripts/terminal.sh`): a window of the macOS Terminal app, captured as a PNG. Terminal ships with every Mac, so it stands for a stock terminal. Reach for it when the proof is how the pane looks: colors, faint text, pixel art, a theme.

Read [`features/README.md`](features/README.md), then the feature file that matches the change. The feature files hold the recipes.

All paths below are relative to the repository root. `S=.agents/skills/verify-alba/scripts`.

## Launch

1. Build: `cargo build --release`.
2. Make a scratch `HOME`: `mktemp -d /tmp/alba-verify.XXXXXX`. Note the literal path it prints; later steps call it `HOME`.
3. Write a config and a weather cache into it: `$S/fixture.py HOME [--weather fresh|stale|expired|none] [--age MINUTES] [--code WMO] [--night] [--no-location] [--config 'key = value' ...]`. The cache is `tests/fixtures/forecast.json` (Buenos Aires, rain, 14°, day), shifted to today. `--weather none` writes no cache, for the offline start.
4. Start alba in one harness:
   - Headless: `$S/pty_drive.py HOME --cols 40 --rows 12 [--offline] STEP ...`. It runs the steps, presses `q`, and prints the exit status, `terminal restored: yes|no`, and the CPU seconds alba used. Steps are listed in `$S/pty_drive.py --help`. It is ready when it prints; a failed `expect` or a dirty exit makes it exit 1.
   - Terminal: `$S/terminal.sh launch HOME --theme light|dark --cols 85 --rows 26 [--offline]`. It is ready when it prints `alba <pid> in Terminal window <id>: …`. A `warning: the pane is …` line means the shot would be at the wrong size; see Setup.

`--offline` runs alba under `sandbox-exec` with every network call denied. Use it for stale weather and the offline start: without it, the startup fetch replaces the fixture.

## Doctor

Run `$S/terminal.sh doctor` first, and again whenever a drive looks wrong. It prints the macOS and alba versions, says whether `src/` is newer than the release build, and lists each `/tmp/alba-verify.*` run that has not been stopped, with its alba pid if it still runs. Drive only runs you started in this session. If alba misbehaves, read its log: `env HOME=HOME XDG_CONFIG_HOME= XDG_CACHE_HOME= XDG_STATE_HOME= target/release/alba --log`.

## Drive

- **Headless:** pass steps in order, such as `sleep=2 screen key=r resize=60x20 expect=°`. `screen` prints the grid inside a `+---+` frame, one row per line, so a row and column are countable.
- **Terminal:** a Terminal drive is launch, wait, and capture. Drive keys and resizes headless.
- **CPU:** launch in Terminal at 85×26 with `--config 'motion = true'`, wait 300 seconds, then run `ps -o time= -p $(cat HOME/alba.pid)`. The spec bound is under 3 seconds.

## Evidence

Put evidence in `target/verify/<ticket>/`. `target/` is ignored by git, and cleanup never touches it. Commit no PNGs.

- **Screenshot:** `$S/terminal.sh shot HOME target/verify/<ticket>/<theme>-<cols>x<rows>-<state>.png`. Read the PNG before you attach it: it must show the alba window and the state you claim.
- **Headless:** save the printed grid and summary with `| tee target/verify/<ticket>/<name>.txt`. Add `--transcript FILE` to keep alba's output as timed JSON lines, to find which cells a redraw wrote and when.
- **Attach** with `gh pr create|edit|comment --attach 'target/verify/<ticket>/<file>.png#<theme>, <cols>x<rows>, <state>'`. The alt text names the theme, the size, and the state.
- Record each check and its result in the pull request body. A UI pull request attaches four palette shots (light and dark, 85×26 and 40×12) plus one shot per state or condition it changes.

Prove the user path: the real binary, a config file, and a cache file. A side effect counts as proof only when you read it back, such as the log after an offline run.

## Cleanup

1. Stop the Terminal run: `$S/terminal.sh stop HOME`. It kills the alba it started, then quits Terminal. A headless run stops itself.
2. Run `$S/terminal.sh doctor` and confirm no run of yours still runs.
3. Delete each scratch `HOME` by its literal path: `rm -r /tmp/alba-verify.AbC123`.
4. Confirm the evidence is still in `target/verify/<ticket>/`.

## Setup and gotchas

- **Permissions:** the terminal that runs the agent needs two macOS permissions, granted once by the user. Screen Recording (System Settings → Privacy & Security → Screen Recording) lets `shot` see the window. Automation for Terminal (Privacy & Security → Automation) lets `launch` and `stop` script it. Without Screen Recording, a shot can miss the window content, so read every PNG. The headless harness needs neither.
- **Themes:** `light` is Terminal's Clear Light profile and `dark` is its Pro profile, each with an opaque background. Both profiles are partly see-through by default, and the desktop would show in a shot. Basic follows the system appearance, and Solid Colors picks a random background for each window, so neither can pin a theme.
- **Window managers:** a tiling window manager can resize the new window. `launch` reads alba's pane size from its terminal and prints a warning when it is not the requested size. Then the shot is not valid: ask the user to make windows of `com.apple.Terminal` float. `shot` captures the window by id, so its position does not matter.
- **One Terminal run at a time:** Terminal's AppleScript can close a window other than the one it names, so the script never closes windows. `launch` refuses while Terminal runs, so every Terminal window belongs to the run, and `stop` quits Terminal. Run shots in sequence. If the user has Terminal open, ask them to quit it.
- **Headless driver tests:** after you change `$S/pty_drive.py`, run `python3 -I -m unittest discover -s .agents/skills/verify-alba/scripts`. CI runs them on Linux and macOS. They guard the three ways a pseudo-terminal driver can hang: a key flood, an endless read after exit, and a deadlock in exit.
