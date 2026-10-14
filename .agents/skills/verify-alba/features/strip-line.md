# Strip line

alba shows one line, centred in the pane: the clock, the short date, and the weather, joined by ` · `. When the pane is too narrow, the date drops first, then the weather. When even the clock doesn't fit, the pane stays blank.

## Sub-features

- `strip-full` shows `HH:MM · ddd DD · NN° mark` when the pane is wide enough.
- `strip-drop` drops the date, then the weather, as the pane narrows.
- `strip-palette` stays readable in a light and a dark theme: the date in blue, a stale temperature in dark gray.

## How to get to it (user POV)

- Run `alba` in any pane with a config that sets `coordinates` and a usable cache.

## Driving it with pty_drive.py and terminal.sh

Preconditions:

- `$S/fixture.py HOME` wrote a fresh rain cache.

- **Full line.** Run `$S/pty_drive.py HOME --cols 40 --rows 12 screen expect='· 14° ‚‚'`. Row 6 of 12 reads `HH:MM · ddd DD · 14° ‚‚`, centred.
- **Drop order.** Run `$S/pty_drive.py HOME --cols 40 --rows 3 resize=16x3 sleep=0.3 screen resize=6x3 sleep=0.3 screen resize=4x3 sleep=0.3 screen`. At 16 columns the line is `HH:MM · 14° ‚‚`; at 6 it is `HH:MM`; at 4 the grid is blank.
- **Palette.** For each theme, run `$S/terminal.sh launch HOME --theme light --cols 40 --rows 12`, then `$S/terminal.sh shot HOME target/verify/<ticket>/light-40x12-rain.png`, then `$S/terminal.sh stop HOME`; repeat with `--theme dark` and `dark-…`. Read each PNG: the line is legible, the date is blue, and the text uses the theme's colors.

## Gotchas

- The `…` fetch indicator appears after `key=r` and while a fetch runs; a run without `--offline` may show it for a moment.
- The colon dims on alternate ticks while `motion = true`, so two shots of one state can differ in the colon.
