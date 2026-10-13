<div align="center">

<!-- tagline from "once in a lifetime" by talking heads. -->
<img src="assets/alba.svg" alt="alba-logo" width=400>

_letting the days go by..._

[features](#features) · [usage](#usage) · [install](#install) · [motivation](#motivation)

</div>

## features

## usage

## install

## config

alba reads `~/.config/alba/config.toml` once, at startup. Restart alba to apply a change. On first run, alba writes a commented template there. It never overwrites an existing file.

A typo stops alba with the line and the reason, such as `config error: line 3: unknown field 'colour'`. Unknown keys are errors, so a misspelled option never silently does nothing.

### location

```toml
location = "buenos aires"
```

Weather needs only your city. alba looks it up through [Open-Meteo geocoding](https://open-meteo.com/en/docs/geocoding-api) and shows the place it found in the top border, so you notice a wrong match.

Narrow an ambiguous name with a comma and a full country name, as in `"paris, united states"`. Avoid country codes: Open-Meteo prefers region codes, so `"toronto, ca"` finds nothing.

If your city isn't found, set coordinates instead. They win over `location`. Set both or neither.

```toml
latitude  = -34.6037   # -90 to 90
longitude = -58.3816   # -180 to 180
```

Without a location, the clock and calendar still work, and alba shows the line to add.

### all options

Every option is shown with its default.

```toml
language    = "en"        # "en" | "es"
units       = "metric"    # "metric" | "imperial"
hour_format = 24          # 24 | 12
week_start  = "monday"    # "monday" | "sunday"
location    = "buenos aires"   # no default; or latitude + longitude
# latitude  = -34.6037    # -90..90
# longitude = -58.3816    # -180..180

[clock]
thickness      = 2        # Large tier only: 1 | 2
digit_gap      = 1        # 1 | 2
colon_gap      = "auto"   # "auto" (thickness + 1) | 1..4
colon          = "ghost"  # off-beat style: "ghost" | "blank" | "steady"
colon_dots     = "square" # "square" | "tall"
ghost_segments = true
tails          = true     # top bar on 6, bottom bar on 9
hooked_seven   = false
am_pm          = "stack"  # "stack" | "text"

[weather]
animation_fps  = 2        # 0 | 1 | 2 | 4
flash          = "8s"     # "8s" | "20s" | "off"
flash_target   = "cloud"  # "cloud" | "bolt" | "both"
moon_dark_side = "faint"  # "faint" | "solid" | "none"
stars          = "glyphs" # "glyphs" | "pixels" | "none"
```

To stop all motion, set `colon = "steady"`, `animation_fps = 0`, and `flash = "off"`.

## motivation
