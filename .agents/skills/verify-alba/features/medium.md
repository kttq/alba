# Medium

A pane of 40×12 or larger that is too small for a larger tier shows Medium: the bezel around the 4×7 LCD clock and the full icon, the weather row, the timeline, and the current week. The bezel hugs the content, so a larger pane leaves it at its own size and centres it.

## Sub-features

- `medium-fit` shows Medium from 40×12 with the default config (43×12 with `hour_format = 12`), and Compact below it.
- `medium-bezel-top` shows the short date in bright blue on the left and `wk 42` (`sem 42`) dim on the right, with only `─` between them. Medium leaves the place name out; Wide and Large show it.
- `medium-bezel-bottom` shows `» ` and the green greeting on the left, and `open-meteo` on the right only while weather data is shown.
- `medium-timeline` draws the 2-pixel bar with the sun by day or tonight's moon by night, and the start time, status, and end time under it. Medium is too narrow for the moon phase, so the night status is the countdown alone.
- `medium-week` shows the weekday header over the current week: past days bright black, the weekend green, and today a yellow reverse-video pill on its two digits, with a plain space on each side.
- `medium-no-data` shows the ghost icon. Without a location, the two-line first-run hint takes the weather row; otherwise the row says `fetching…`, `offline · retrying`, or `location not found: <name>`, and the timeline rows stay empty.

## How to get to it (user POV)

- Run `alba` in a pane of 40×12 or a little larger.

## Driving it with drive and terminal.sh

Preconditions:

- A new `HOME` per state, filled by `$D/fixture`.

- **First run.** `$D/fixture HOME --no-location --weather none`, then `$D/drive HOME --cols 40 --rows 12 sleep=1 screen expect='add location = "your city" to' expect='~/.config/alba/config.toml'`.
- **Name lookup, online.** `$D/fixture HOME --weather none --location "buenos aires"`, then `$D/drive HOME --cols 40 --rows 12 sleep=6 screen expect=open-meteo`. alba looks the name up and fetches over the network; the weather row fills, and row 0 shows no place name.
- **No match.** `$D/fixture HOME --weather none --location "xqzzyplace"`, then `$D/drive HOME --cols 40 --rows 12 sleep=5 screen expect='location not found: xqzzyplace'`.
- **Offline.** `$D/fixture HOME --weather none`, then `$D/drive HOME --cols 40 --rows 12 --offline sleep=3 screen expect='offline · retrying' colors`. Row 0 has the date at `fg=Idx(12)` and the frame and week at `fg=Idx(8)`; row 10 has today at `fg=Idx(3)` and the weekend at `fg=Idx(2)`. The log holds one `fetch failed` line.
- **Palette.** For each theme, run `$S/terminal.sh launch HOME --theme light --cols 40 --rows 12 --offline`, then `$S/terminal.sh shot HOME target/verify/<ticket>/light-40x12-rain.png`, then `$S/terminal.sh stop HOME`; repeat with `--theme dark` and at 85×26. Read each PNG: the bezel frames the clock, the icon, the timeline, and the week strip, and today's pill is visible. `cargo test` checks the fit at every pane size in `tests/size_sweep.rs`.

## Gotchas

- The fixture writes `coordinates` unless `--location` is given, and coordinates win over a `location` line added with `--config`. Use `--location` to drive a name lookup.
- `colors` prints no reverse-video flag, so prove today's pill with a screenshot.
- The fixture's day follows the real clock, so a drive at night shows the moon on the timeline and the night status.
