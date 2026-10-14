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

Weather needs only your city. alba looks it up through [Open-Meteo geocoding](https://open-meteo.com/en/docs/geocoding-api) and shows the place it found at the top of the bezel, so you notice a wrong match.

Narrow an ambiguous name with a comma and a full country name, as in `"paris, united states"`. Avoid country codes: Open-Meteo prefers region codes, so `"toronto, ca"` finds nothing.

If your city isn't found, set coordinates instead, as `[latitude, longitude]`. They win over `location`.

```toml
coordinates = [-34.6037, -58.3816]   # latitude -90 to 90, longitude -180 to 180
```

Without a location, the clock and calendar still work, and alba shows the line to add.

### all options

Every option is shown with its default.

```toml
language       = "en"      # "en" | "es"
units          = "metric"  # "metric" | "imperial"
hour_format    = 24        # 24 | 12
week_start     = "monday"  # "monday" | "sunday"
location       = "buenos aires"   # no default; or coordinates
# coordinates  = [-34.6037, -58.3816]   # [latitude -90..90, longitude -180..180]
ghost_segments = true      # unlit clock segments shown faintly
motion         = true      # false: steady colon, still icons, no flash
```

Turn `ghost_segments` off if your terminal has no faint style. Set `motion = false` to stop all motion.

## motivation
