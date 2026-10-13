# alba

alba is a tiny terminal companion: a glanceable widget with a clock, a month calendar, and the current weather, drawn as pixel art in a terminal pane.

## Language

**Pane**:
The terminal area alba draws into, measured in columns and rows.
_Avoid_: Window, screen

**Size tier**:
One of seven fixed layouts (Large, Wide, Tall, Medium, Compact, Mini, Strip), chosen as the largest that fits the pane.
_Avoid_: Breakpoint, mode

**Home view**:
alba's only view, which composes the clock, weather, and calendar to fit the current size tier.
_Avoid_: Main screen, dashboard, focus view

**Condition**:
One of nine weather categories (clear, partly cloudy, cloudy, fog, drizzle, rain, heavy rain, snow, thunderstorm) that WMO codes map to.
_Avoid_: Weather type, WMO code

**Icon**:
The pixel-art drawing of a condition, in a full and a mini size, with two animation frames.
_Avoid_: Sprite (reserve for the const data that encodes icons and digits), emoji

**Bezel**:
An optional frame around the whole widget, like the edge of a pocket gadget.
_Avoid_: Border, box

**Timeline**:
A bar for the current day or night: sunrise to sunset with the sun on it, or sunset to sunrise with tonight's moon on it.
_Avoid_: Sun arc, progress bar, day bar

**Ghost segment**:
An unlit segment of the LCD clock, shown faintly so every digit keeps the shape of an 8.
_Avoid_: Shadow digit, background segment

**Stale weather**:
Weather data old enough that refreshes have failed; still shown, but dimmed and labelled with its age.
_Avoid_: Old data, outdated weather

**Expired weather**:
Weather data too old to show at all; alba treats it as having no weather.
_Avoid_: Invalid cache, dead data
