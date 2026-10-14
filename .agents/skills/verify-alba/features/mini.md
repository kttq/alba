# Mini

With weather shown, a pane of two rows that is too small for a larger tier shows Mini: the text clock, the mini icon, and the temperature, 2 columns apart and centred as one block. Without weather, Mini does not fit and Strip takes the pane.

## Sub-features

- `mini-fit` shows Mini from 18×2 with the default config (20×2 at `12:38pm` with `hour_format = 12`), and Strip below it.
- `mini-motion` animates the icon at 1 frame per second; the rain drops move one column.
- `mini-stale` dims the temperature to dark gray, with no age label.

## How to get to it (user POV)

- Run `alba` in a pane of 18×2 or a little larger, with a config that sets `coordinates` and a usable cache.

## Driving it with drive and terminal.sh

Preconditions:

- `$D/fixture HOME` wrote a fresh rain cache.

- **Fit and switch.** Run `$D/drive HOME --cols 19 --rows 2 --offline sleep=0.5 screen resize=19x1 sleep=0.3 screen`. At 19×2 row 0 reads `HH:MM   ▄██▄   14°` and row 1 holds the icon's lower half; at 19×1 the row reads `HH:MM · 14° ‚‚`.
- **Motion.** Run `$D/drive HOME --cols 19 --rows 2 --offline sleep=0.5 colors sleep=1 colors`. The grid text is the same in both frames; the cells on row 1 with `bg=Idx(12)` (bright blue drops) move one column.
- **Stale.** Write the cache with `$D/fixture HOME --weather stale`, then run `$D/drive HOME --cols 19 --rows 2 --offline colors`. The temperature has `fg=Idx(8)`.
- **Palette.** For each theme, run `$S/terminal.sh launch HOME --theme light --cols 20 --rows 2 --offline`, then `$S/terminal.sh shot HOME target/verify/<ticket>/light-20x2-rain.png`, then `$S/terminal.sh stop HOME`; repeat with `--theme dark`. Read each PNG: the clock, the cloud with blue drops, and the temperature are legible. `cargo test` checks the fit at every pane size in `tests/size_sweep.rs`.

## Gotchas

- A 19×2 Terminal launch has come out as 20×2, with the `launch` warning. Drive 19 columns headless, and shoot at 20.
- The fetch indicator `…` widens the temperature by one column while a fetch runs, so an 18×2 pane shows Strip during a fetch. Use `--offline` for a still pane.
