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

alba reads `~/.config/alba/config.toml` at startup, and writes a commented template there on first run. Every option, with its default:

```toml
location       = "buenos aires"   # your city; narrow it with a full country name: "paris, united states"
# coordinates  = [-34.6037, -58.3816]   # instead of location: [latitude, longitude]
language       = "en"      # "en" | "es"
units          = "metric"  # "metric" | "imperial"
hour_format    = 24        # 24 | 12
week_start     = "monday"  # "monday" | "sunday"
ghost_segments = true      # unlit clock segments shown faintly
motion         = true      # false: steady colon, still icons, no flash
```

Errors, location lookup, and each option in detail: [docs/config.md](docs/config.md).

## motivation
