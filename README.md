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

alba reads `~/.config/alba/config.toml` (or `$XDG_CONFIG_HOME/alba/config.toml`) at startup, and writes a commented template there on first run. A few highlights:

```toml
location    = "paris, united states"   # your city, narrowed by country
language    = "es"                     # english or neutral spanish
hour_format = 12                       # 6:38pm
motion      = false                    # a completely still widget
```

Every option, with defaults and details: [docs/config.md](docs/config.md).

## weather data

Weather data by [Open-Meteo.com](https://open-meteo.com/), under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). See its [licence](https://open-meteo.com/en/license).

## motivation
