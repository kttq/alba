# Large

An 85×26 pane shows the full gadget: the bezel around the 8×15 LCD clock, then the Large icon, a details column, and the month calendar side by side, then the timeline. Large comes before every other tier when it fits.

## Sub-features

- `large-fit` shows Large from 71×23 with the default config, and Wide one column or row below.
- `large-details` shows the temperature with the high and low, the condition name, and the sparkline under its dim label `precip · next 12h` (`precip · próx. 12h` in Spanish).
- `large-first-run-hint` shows the two hint lines in the details column, beside the ghost icon.
- `large-place-name` centres the place name, dim, in the top border, as in Wide.
- `large-conditions` draws each of the nine conditions by day and by night with the Large icon: stars beside the clear and partly cloudy night icons, and tonight's moon for the clear night.

## How to get to it (user POV)

- Run `alba` in a pane of 85×26.

## Driving it with drive and terminal.sh

Preconditions:

- A new `HOME` per state, filled by `$D/fixture`.

- **Large.** `$D/fixture HOME`, then `$D/drive HOME --cols 85 --rows 26 --offline sleep=1 screen expect='precip · next 12h'`.
- **First-run hint.** `$D/fixture HOME --no-location --weather none`, then `$D/drive HOME --cols 85 --rows 26 sleep=1 screen expect='add location'`.
- **Palette.** For each theme, run `$S/terminal.sh launch HOME --theme light --cols 85 --rows 26 --offline`, then `$S/terminal.sh shot HOME target/verify/<ticket>/light-85x26-rain.png`, then `$S/terminal.sh stop HOME`.
- **Full screen.** As in Palette, with `--full-screen` in place of `--cols` and `--rows`. Read the PNG: Large stays centred, with empty space around the bezel. The pane size depends on the theme's font: Clear Light gave 202×56 and Pro 236×60 on a 2560×1600 display.
- **Conditions.** For each WMO code (`0 2 3 45 51 61 65 71 95`), with and without `--night`: `$D/fixture HOME --code CODE [--night]`, then launch and shoot at 85×26 as in Palette. Read each PNG: the icon matches the condition, and the night clear and partly cloudy icons have stars.

## Gotchas

- The Spanish first-run hint is 32 columns wide, so Large without a location needs 86 columns in Spanish; an 85×26 pane then shows Wide.
- The ticket's herdr pane in Ghostty is the owner's eye test; the agent drives Terminal.
