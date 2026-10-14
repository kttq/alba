# Weather states

alba shows weather from its cache at once, refreshes it in the background, and dims it when it is stale. Expired weather and no weather show only the clock and date. Each of the nine conditions has its own two-cell mark after the temperature.

## Sub-features

- `weather-fresh` shows the temperature and condition mark in the default foreground.
- `weather-stale` keeps cached weather after a failed refresh, with the temperature in dark gray.
- `weather-expired` treats weather older than 12 hours as none.
- `weather-offline-start` starts with no cache and no network, and logs the failed fetch.
- `weather-no-location` runs with no `location` or `coordinates`, and shows no weather.
- `weather-conditions` maps each condition to its mark: clear `✦✦`, partly cloudy `✦~`, cloudy `~~`, fog `≈≈`, drizzle `′′`, rain `‚‚`, heavy rain `┆┆`, snow `**`, thunderstorm `!!`.

## How to get to it (user POV)

- Start alba with an old cache, no cache, or no network. A user cannot pick a condition; the forecast does.

## Driving it with pty_drive.py and terminal.sh

Preconditions:

- A new `HOME` per state. Pass `--offline` for every state but fresh, so the startup fetch keeps the fixture.

- **Fresh.** `$S/fixture.py HOME`, then `$S/pty_drive.py HOME expect='14° ‚‚'`.
- **Stale.** `$S/fixture.py HOME --weather stale`, then `$S/terminal.sh launch HOME --theme dark --cols 40 --rows 12 --offline` and `shot`. The line keeps `14° ‚‚`, and `14°` is dark gray. The log holds `msg="fetch failed"`.
- **Expired.** `$S/fixture.py HOME --weather expired`, then `$S/pty_drive.py HOME --offline screen`. The line is `HH:MM · ddd DD`, with no weather.
- **Offline start.** `$S/fixture.py HOME --weather none`, then `$S/pty_drive.py HOME --offline sleep=2 screen`. The line is `HH:MM · ddd DD`. Read the log with `env HOME=HOME XDG_CONFIG_HOME= XDG_CACHE_HOME= XDG_STATE_HOME= target/release/alba --log`: one `fetch failed` line.
- **No location.** `$S/fixture.py HOME --no-location --weather none`, then `$S/pty_drive.py HOME screen`. The line is `HH:MM · ddd DD`, and alba made no fetch: the log is empty.
- **Conditions.** For each WMO code (clear `0`, partly cloudy `2`, cloudy `3`, fog `45`, drizzle `51`, rain `61`, heavy rain `65`, snow `71`, thunderstorm `95`), with and without `--night`: `$S/fixture.py HOME --code CODE [--night]`, then `$S/pty_drive.py HOME --offline expect='MARK'`.

## Gotchas

- `--offline` denies every network call, so a fresh fixture stays fresh only while its age is under 30 minutes; past that, alba tries a fetch and logs it.
- The cache matches the config's `coordinates` exactly. `fixture.py` writes both from the fixture; a hand-edited `coordinates` line makes alba ignore the cache.
- The fixture shifts by whole days, so sunrise and sunset keep their Buenos Aires clock times. `--night` sets only the current `is_day`.
