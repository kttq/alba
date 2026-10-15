# alba verification map

The maintained source for verifying what a user sees in alba. Read this index before a drive, then use the matching feature file as the recipe. When a UI ticket adds a tier, a state, or a key, add or extend its feature file in the same pull request.

## Baseline preconditions

- A release build of alba and the examples that is newer than the sources: `$S/terminal.sh doctor` prints no `stale build` line.
- A fresh scratch `HOME` from `mktemp -d /tmp/alba-verify.XXXXXX`, filled by `$D/fixture`. One `HOME` per run, so runs never share a cache or a log.
- `S=.agents/skills/verify-alba/scripts` and `D=target/release/examples`, from the repository root.

## Driving conventions

- Start each recipe from a new `HOME` with the fixture it names.
- Drive keys, resizes, and timing headless; capture looks in Terminal.
- Match a positioned write (`ESC[row;colH…`) in a transcript, not a whole `HH:MM`: ratatui writes only the cells that change.

## Proof and skip reporting

- Name the feature ID, the harness, the theme, and the size with every artifact.
- A screenshot proves a look; a headless grid proves text and position. Report each with the one that proves it.
- Report a check you could not run with the command you tried and the unmet precondition. Report it as skipped, not as passed through another path.

## Features

- [Strip line](./strip-line.md): the one-line face, its drop order, and its palette in light and dark themes.
- [Mini](./mini.md): the two-row tier with the text clock, the mini icon, and the temperature, and the switch to Strip.
- [Weather states](./weather-states.md): fresh, stale, expired, offline start, no location, and the nine conditions.
- [Terminal lifecycle](./terminal-lifecycle.md): quit keys, terminal restore, resize, the refresh key, tick timing, and idle CPU.
