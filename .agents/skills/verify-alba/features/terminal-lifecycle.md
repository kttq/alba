# Terminal lifecycle

alba takes over the pane in the alternate screen, redraws on a wall-clock tick and after each resize, and gives the terminal back on quit. `r` fetches the weather now. It stays close to idle between ticks.

## Sub-features

- `quit` leaves on `q`, `esc`, or `ctrl-c`, with exit status 0.
- `restore` leaves the alternate screen and shows the cursor on quit.
- `resize` redraws the line centred in the new pane.
- `refresh` fetches the weather now on `r`.
- `tick` rolls the minute over just after the wall-clock boundary.
- `cpu` stays under 3 s of CPU in 5 minutes at 85×26 with `motion = true`.

## How to get to it (user POV)

- Press `q`, `esc`, or `ctrl-c` in alba; resize the pane; press `r`; leave it running.

## Driving it with drive

Preconditions:

- `$D/fixture HOME` wrote a fresh cache.

- **Quit keys.** Run `$D/drive HOME key=q exited`, then the same with `key='\x1b'` and `key='\x03'`. Each prints `exited: ok`, `exit status: 0`, and `terminal restored: yes`: the alternate screen left, the cursor shown, and the terminal back in cooked mode. `exited` matters: without it, the driver presses `q` at the end, and a dead key would pass.
- **Resize.** Run `$D/drive HOME --cols 40 --rows 12 resize=60x20 sleep=0.15 screen`. The line is on row 10 of 20, centred in 60 columns.
- **Refresh.** Run `$D/drive HOME key=r screen`. The line shows `…` after the temperature until the fetch ends.
- **Tick.** Start the driver about 3 seconds before a minute boundary with `--transcript target/verify/<ticket>/tick.jsonl sleep=6`. Each line is one read, `{"time": unix seconds, "output": text}`. The first read that writes the new minute digit at its position comes just after the boundary; its `time` minus the boundary is the rollover delay.
- **CPU.** Run `$D/drive HOME --cols 85 --rows 26 sleep=300`. The `cpu seconds` line must read under 3. The spec's own check runs in Terminal; see `SKILL.md` → Drive.

## Gotchas

- The driver paces keys with `sleep`. alba writes a cursor sequence on every draw, so the pseudo-terminal always reads as ready.
- A key flood queues `q` behind it; `flood=SECONDS` paces at 30 keys a second, so `q` still lands.
- `cpu seconds` covers every child of the driver, which is only alba.
