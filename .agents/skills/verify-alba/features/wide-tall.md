# Wide and Tall

A wide, short pane shows Wide; a narrow, tall pane shows Tall. Both put the bezel around the 4×7 LCD clock, the full icon, the weather row, and the timeline, and add the month calendar: beside the timeline in Wide, below it in Tall. Wide comes before Tall when both fit.

## Sub-features

- `wide-fit` shows Wide from 63×11 with the default config (66×11 with `hour_format = 12`).
- `tall-fit` shows Tall from 40×21 (43×21 with `hour_format = 12`) when Wide does not fit, and Medium below 21 rows.
- `wide-place-name` centres the place name, dim, in Wide's top border between the date and the week, truncated with `…`. Tall leaves it out.
- `tall-condition` shows the condition name under the weather row in Tall.
- `month-calendar` shows the month title in bright blue, the weekday header, and always six week rows: past days bright black, the weekend green by the day of the week, and today a yellow reverse-video pill on its two digits.
- `month-sunday-first` starts the header and the grid on Sunday with `week_start = "sunday"`.

## How to get to it (user POV)

- Run `alba` in a pane of 66×11, or 43×21.

## Driving it with drive and terminal.sh

Preconditions:

- A new `HOME` per state, filled by `$D/fixture`.

- **Wide.** `$D/fixture HOME`, then `$D/drive HOME --cols 66 --rows 11 --offline sleep=1 screen colors`. The month title and the header sit right of the clock; today's digits are at `fg=Idx(3)`.
- **Tall.** `$D/fixture HOME`, then `$D/drive HOME --cols 43 --rows 21 --offline sleep=1 screen`. The condition name is under the weather row; the calendar is under the timeline.
- **Sunday first.** `$D/fixture HOME --config 'week_start = "sunday"'`, then `$D/drive HOME --cols 43 --rows 21 --offline sleep=1 screen expect='su mo tu we th fr sa'`. The 1st sits under its own weekday.
- **Place name.** `$D/fixture HOME --location "buenos aires"`, then `$D/drive HOME --cols 66 --rows 11 --offline sleep=1 screen expect='buenos aires, ar'`. The fixture caches the name as resolved, so alba needs no lookup. At 43×21 the same `HOME` shows no place name.
- **Palette.** For each theme and each size (66×11 and 43×21), run `$S/terminal.sh launch HOME --theme light --cols 66 --rows 11 --offline`, then `$S/terminal.sh shot HOME target/verify/<ticket>/light-66x11-rain.png`, then `$S/terminal.sh stop HOME`. Read each PNG: today's pill is visible, and the calendar sits beside or below.

## Gotchas

- `colors` prints no reverse-video flag, so prove today's pill with a screenshot.
- The fixture's day follows the real clock, so the calendar shows the real month and today.
