# Compact

A pane of 23×8 or larger that is too small for a larger tier shows Compact: the short date, the 4×7 LCD clock, an empty row, then the mini icon beside the temperature with its high and low, over the 12-bar precipitation sparkline. It has no bezel. The weather sits at the clock's left edge, so the icon keeps its column when the weather returns.

## Sub-features

- `compact-fit` shows Compact from 23×8 with the default config (26×8 with `hour_format = 12`), and Mini or Strip below it.
- `compact-motion` blinks the colon: lit in the first half of each second, a bright black dim ghost in the second.
- `compact-sparkline` draws the next 12 hours in bright blue `▁…█`; a bar below 5% is a dim bright black `▁`.
- `compact-no-data` shows the ghost icon with `no location set`, `fetching…`, or `offline · retrying` beside it. The text after `·` drops when it does not fit: `offline` at 23×8, the full text from 26 columns.
- `compact-stale` turns the text and the sparkline bright black and keeps the icon's colors. The age label and the fetch `…` follow the high and low when they fit, and drop otherwise.

## How to get to it (user POV)

- Run `alba` in a pane of 23×8 or a little larger.

## Driving it with drive and terminal.sh

Preconditions:

- A new `HOME` per state, filled by `$D/fixture`.

- **Fresh and motion.** `$D/fixture HOME`, then `$D/drive HOME --cols 23 --rows 8 --offline sleep=0.6 screen colors sleep=0.5 colors`. Row 6 reads ` ▄██▄   14°  ↑19° ↓13° `, and row 7 holds the icon's lower half and 12 bars with `fg=Idx(12)`. The colon's cells (column 11, rows 2 and 3) are `fg=Default` in one `colors` frame and `fg=Idx(8) dim` in the other.
- **No location.** `$D/fixture HOME --no-location --weather none`, then `$D/drive HOME --cols 23 --rows 8 sleep=1 screen expect='no location set'`.
- **Offline.** `$D/fixture HOME --weather none`, then `$D/drive HOME --cols 23 --rows 8 --offline sleep=2 screen expect=offline resize=26x8 sleep=0.5 screen expect='offline · retrying'`. At both sizes the ghost icon's lower half starts in column 0. The log holds one `fetch failed` line.
- **Stale.** `$D/fixture HOME --weather stale`, then `$D/drive HOME --cols 30 --rows 8 --offline sleep=1 screen colors`. Row 6 ends with the age label, and the temperatures and bars have `fg=Idx(8)`.
- **Palette.** For each theme, run `$S/terminal.sh launch HOME --theme light --cols 23 --rows 8 --offline`, then `$S/terminal.sh shot HOME target/verify/<ticket>/light-23x8-rain.png`, then `$S/terminal.sh stop HOME`; repeat with `--theme dark`. Read each PNG: the date, the clock against its ghosts, the cloud with blue drops, the temperatures, and the bars are legible. `cargo test` checks the fit at every pane size in `tests/size_sweep.rs`.

## Gotchas

- The colon blinks on the wall clock, so one shot shows it lit or as a ghost by chance. Prove the blink headless with two `colors` frames half a second apart.
- A fresh fixture past 30 minutes old makes alba start a fetch; without `--offline` it replaces the fixture.
