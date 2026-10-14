#!/usr/bin/env python3
"""Write a config and a weather cache into a scratch HOME for one verification run.

The cache comes from tests/fixtures/forecast.json (Buenos Aires, rain, day), shifted
by whole days so its middle daily row is today at the forecast location.

usage: fixture.py HOME [--weather fresh|stale|expired|none] [--age MINUTES]
                       [--code WMO] [--night] [--no-location] [--config 'key = value' ...]
"""

import argparse
import json
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
FORECAST = REPO / "tests/fixtures/forecast.json"
DAY = 86400
# Ages that land in each freshness band: stale after 60 minutes, expired after 12 hours.
DEFAULT_AGE_MINUTES = {"fresh": 5, "stale": 90, "expired": 13 * 60}


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("home", type=Path)
    parser.add_argument("--weather", choices=["fresh", "stale", "expired", "none"], default="fresh")
    parser.add_argument("--age", type=int, help="minutes since the fetch; overrides the default age")
    parser.add_argument("--code", type=int, help="WMO weather code for the current condition")
    parser.add_argument("--night", action="store_true", help="current is_day = 0")
    parser.add_argument("--no-location", action="store_true", help="config without coordinates")
    parser.add_argument("--config", action="append", default=[], help="extra TOML line")
    args = parser.parse_args()

    forecast = json.loads(FORECAST.read_text())
    latitude, longitude = forecast["latitude"], forecast["longitude"]

    lines = [] if args.no_location else [f"coordinates = [{latitude}, {longitude}]"]
    lines += args.config
    write(args.home / ".config/alba/config.toml", "\n".join(lines) + "\n")

    if args.weather != "none":
        now = int(time.time())
        age = args.age if args.age is not None else DEFAULT_AGE_MINUTES[args.weather]
        fetched_at = now - age * 60
        shift_to_today(forecast, now)
        forecast["current"]["time"] = fetched_at - fetched_at % 900
        if args.code is not None:
            forecast["current"]["weather_code"] = args.code
        if args.night:
            forecast["current"]["is_day"] = 0
        cache = {
            "version": 1,
            "fetched_at": fetched_at,
            "location": {"latitude": latitude, "longitude": longitude},
            "forecast": forecast,
        }
        write(args.home / ".cache/alba/weather.json", json.dumps(cache))

    print(args.home)


def shift_to_today(forecast, now):
    """Moves every time by whole days, so sunrise, sunset, and midnights keep their clock times."""
    offset = timezone(timedelta(seconds=forecast["utc_offset_seconds"]))
    fixture_today = datetime.fromtimestamp(forecast["daily"]["time"][1], offset).date()
    today = datetime.fromtimestamp(now, offset).date()
    shift = (today - fixture_today).days * DAY
    for columns in (forecast["daily"], forecast["hourly"]):
        for name in ("time", "sunrise", "sunset"):
            if name in columns:
                columns[name] = [moment + shift for moment in columns[name]]


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


if __name__ == "__main__":
    main()
