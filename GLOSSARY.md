# pana

pana is a tiny terminal companion: a glanceable widget with a clock, a month calendar, and the current weather, drawn as pixel art in a terminal pane.

## Language

**Pane**:
The terminal area pana draws into, measured in columns and rows.
_Avoid_: Window, screen

**Size tier**:
One of four fixed layouts (Full, Compact, Mini, Strip), chosen as the largest that fits the pane.
_Avoid_: Breakpoint, mode

**Home view**:
The default view, which composes the clock, weather, and calendar to fit the current size tier.
_Avoid_: Main screen, dashboard

**Focus view**:
A view that shows one widget only (clock, weather, or calendar) and fills the pane.
_Avoid_: Detail screen, page

**Condition**:
One of nine weather categories (clear, partly cloudy, cloudy, fog, drizzle, rain, heavy rain, snow, thunderstorm) that WMO codes map to.
_Avoid_: Weather type, WMO code

**Icon**:
The pixel-art drawing of a condition, in a full and a mini size, with two animation frames.
_Avoid_: Sprite (reserve for the const data that encodes icons and digits), emoji

**Bezel**:
An optional frame around the whole widget, like the edge of a pocket gadget.
_Avoid_: Border, box
