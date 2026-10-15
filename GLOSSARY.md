# alba

alba is a tiny terminal companion: a glanceable widget with a clock, a month calendar, and the current weather, drawn as pixel art in a terminal pane.

## Language

**Pane**:
The terminal area alba draws into, measured in columns and rows.
_Avoid_: Window, screen

**Size tier**:
One of seven fixed layouts (Large, Wide, Tall, Medium, Compact, Mini, Strip), chosen as the largest that fits the pane.
_Avoid_: Breakpoint, mode

**Face**:
Everything alba shows at one instant (the clock, the weather, and the calendar), which the layout fits to the size tier.
_Avoid_: Home view, view, dashboard, screen

**Condition**:
One of nine weather categories (clear, partly cloudy, cloudy, fog, drizzle, rain, heavy rain, snow, thunderstorm) that WMO codes map to.
_Avoid_: Weather type, WMO code

**Pixel**:
One square half of a terminal cell. A cell stacks two pixels, top and bottom, each with its own color.
_Avoid_: Dot, sub-cell, half-block

**Icon**:
The pixel-art drawing of a condition, in a full and a mini size, with two animation frames.
_Avoid_: Sprite (reserve for the const data that encodes icons and digits), emoji

**Bezel**:
An optional frame at the edge of the face, like the edge of a pocket gadget, with labels in its borders.
_Avoid_: Border, box

**Timeline**:
A bar for the current day or night: sunrise to sunset with the sun on it, or sunset to sunrise with tonight's moon on it.
_Avoid_: Sun arc, progress bar, day bar

**Greeting**:
A short, friendly line that changes with the time of day, such as "good morning ♥" or "sleepy time ✦".
_Avoid_: Footer, tagline, message

**Ghost segment**:
An unlit segment of the LCD clock, shown faintly so every digit keeps the shape of an 8.
_Avoid_: Shadow digit, background segment

**Motion**:
The animation alba shows: the blinking colon, the icon frames and star twinkle, and the thunderstorm flash. It stops completely when motion is off.
_Avoid_: Animation (reserve for icon frames), blink

**Weather station**:
The part of alba that holds the latest weather: it reads the cache at start, fetches the forecast, and writes the cache.
_Avoid_: Weather feed, weather service

**Weather state**:
What the weather station has to show now: no location, fetching, offline, fresh, stale, or location not found. Expired weather is never a weather state; it counts as no weather.
_Avoid_: Status, network state

**Weather area**:
The part of the face that shows the weather as text: the readings (temperature, high and low, age, condition, sparkline), or, without weather data, what the weather state says. The icon and the timeline are not part of it.
_Avoid_: Weather row, weather text

**Stale weather**:
Weather data old enough that refreshes have failed; still shown, but dimmed and labelled with its age.
_Avoid_: Old data, outdated weather

**Expired weather**:
Weather data too old to show at all; alba treats it as having no weather.
_Avoid_: Invalid cache, dead data

**Forecast day**:
The day at the forecast location, from midnight to midnight, that today's high and low belong to. It can differ from the calendar's today when the location is far away.
_Avoid_: Location date, local day
