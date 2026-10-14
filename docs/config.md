# Config

alba reads `$XDG_CONFIG_HOME/alba/config.toml` once, at startup. Without `XDG_CONFIG_HOME`, that is `~/.config/alba/config.toml`. Restart alba to apply a change. On first run, alba writes a commented template there. It never overwrites an existing file.

## Files

alba follows the XDG Base Directory spec on macOS and Linux. An unset, empty, or relative variable falls back to its default.

| File | Path | Default |
|---|---|---|
| Config | `$XDG_CONFIG_HOME/alba/config.toml` | `~/.config/alba/config.toml` |
| Weather cache | `$XDG_CACHE_HOME/alba/weather.json` | `~/.cache/alba/weather.json` |
| Log | `$XDG_STATE_HOME/alba/alba.log` | `~/.local/state/alba/alba.log` |

The log is empty at each start. It holds one [logfmt](https://brandur.org/logfmt) line per failed fetch or failed cache write:

```text
time=2026-10-13T06:38:00-03:00 level=warn msg="fetch failed" error="http status 503"
```

## All options

Every option, with its default. `location` has no default.

```toml
location       = "buenos aires"   # your city; or coordinates
# coordinates  = [-34.6037, -58.3816]   # [latitude -90..90, longitude -180..180]
language       = "en"      # "en" | "es"
units          = "metric"  # "metric" | "imperial"
hour_format    = 24        # 24 | 12
week_start     = "monday"  # "monday" | "sunday"
ghost_segments = true      # unlit clock segments shown faintly
motion         = true      # false: steady colon, still icons, no flash
```

## Errors

A typo stops alba before it starts, with the line and the reason:

```text
config error: line 3: unknown field `colour`, expected one of `language`, …
```

Unknown keys are errors, so a misspelled option never silently does nothing.

## Location

Weather needs only your city:

```toml
location = "buenos aires"
```

alba looks the name up through [Open-Meteo geocoding](https://open-meteo.com/en/docs/geocoding-api). It shows the place it found at the top of the bezel, so you notice a wrong match.

Narrow an ambiguous name with a comma and a full country name, as in `"paris, united states"`. Avoid country codes: Open-Meteo prefers region codes, so `"toronto, ca"` finds nothing.

If your city isn't found, set coordinates instead. They win over `location`.

```toml
coordinates = [-34.6037, -58.3816]   # [latitude -90..90, longitude -180..180]
```

Without a location, the clock and calendar still work, and alba shows the line to add.

## Display

- `ghost_segments = false` hides the unlit clock segments. Use it if your terminal has no faint style.
- `motion = false` stops all motion: the colon stays lit, icons stay still, and storms don't flash.
