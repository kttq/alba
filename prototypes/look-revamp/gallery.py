#!/usr/bin/env python3
"""PROTOTYPE (issue #55), throwaway: builds gallery.html, the revamped look as it was locked,
from the shots in target/verify/55.

usage: python3 prototypes/look-revamp/gallery.py
The page embeds every image, so it opens with a double-click and can be shared as one file.
"""
import base64
import html
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
SHOTS = ROOT / "target/verify/55"
OUT = pathlib.Path(__file__).resolve().parent / "gallery.html"

SECTIONS = [
    ("The ladder", "alba takes the first rung that fits the pane. C, the week as a column, where there is room; "
     "U, the week under the readings, where C does not fit. The week never sits beside the clock. The sparkline "
     "shows only with its label, in the large rungs; the smaller rungs show the condition instead. Below the "
     "ladder, v1's mini and one-line sizes. Ghostty, dark terminal, the terminal theme.",
     [("final-large-c.png", "1 · large C · 58×24 · pane 85×26"),
      ("final-large-u.png", "2 · large U · 50×24 · pane 50×30"),
      ("final-short-u.png", "3 · short U · 46×21 · pane 70×21"),
      ("final-medium-c.png", "4 · medium C · 50×14 · pane 52×15"),
      ("final-medium-u.png", "5 · medium U · 40×15 · pane 40×15"),
      ("final-compact.png", "6 · compact · pane 32×10"),
      ("final-strip.png", "one line · pane 40×3")]),
    ("Themes", "terminal, the default, uses only the terminal's named colors: the strong accent is bold text, "
     "the second accent is red, and yellow stays in the pictures, because yellow text fades on light themes. "
     "dark and warm fill the pane with their own colors.",
     [("final-large-c-dark.png", "dark"), ("final-large-c-warm.png", "warm"),
      ("final-large-u-warm.png", "warm · large U"), ("final-medium-c-dark.png", "dark · medium C"),
      ("final-large-c-light.png", "terminal on a light terminal"),
      ("final-large-c-dark-on-light.png", "dark on a light terminal"),
      ("final-night-terminal.png", "terminal · night, clear"), ("final-night-dark.png", "dark · night"),
      ("final-night-warm.png", "warm · night")]),
    ("Icons", "24-bit half-block icons where the terminal has 24-bit color; v1's icons elsewhere. Fog: a cloud "
     "over drifting mist. Thunder: the cloud flashes twice and the bolt shows for a second and a half of every "
     "eight seconds.",
     [("final-fog.png", "fog"), ("final-thunder-bolt.png", "thunder · the strike"),
      ("final-thunder-calm.png", "thunder · between strikes")]),
    ("macOS Terminal", "The terminal theme in Terminal. Terminal draws half blocks from the font, so seams split "
     "each pixel row.",
     [("final-terminal-app-dark.png", "Terminal · dark"), ("final-terminal-app-light.png", "Terminal · light")]),
    ("The prototype's bar", "Keys: h theme, t/T time ±1h, c condition, b bar. It shows the theme, the rung, "
     "the time, and the condition.", [("final-bar.png", "the bar")]),
]

CSS = """
body{background:#0e0b08;color:#e6dccb;font:15px/1.5 ui-sans-serif,system-ui,sans-serif;margin:0}
main{max-width:1180px;margin:0 auto;padding:32px 24px 80px}
h1{font-size:28px;color:#e8bd6a;margin:0}
.bar{height:4px;background:linear-gradient(90deg,#dd7357 0 20%,#d49a3a 20% 60%,#e8bd6a 60%);margin:8px 0 24px;max-width:360px}
h2{color:#e8bd6a;border-bottom:1px solid #3a2e20;padding-bottom:6px;margin-top:48px}
p{color:#cfc3ae;max-width:820px}
.grid{display:flex;gap:12px;flex-wrap:wrap;margin-top:12px}
figure{margin:0;background:#0a0806;border-radius:8px;padding:6px}
figure img{display:block;max-height:340px;max-width:100%;border-radius:6px;cursor:zoom-in}
figcaption{font-size:12px;color:#8d7d65;padding-top:4px}
dialog{background:#000;border:0;padding:0}dialog img{max-width:95vw;max-height:95vh;display:block}
"""

JS = """
document.addEventListener('click',e=>{if(e.target.matches('figure img')){const d=document.querySelector('dialog');d.querySelector('img').src=e.target.src;d.showModal()}else if(e.target.closest('dialog')){e.target.closest('dialog').close()}});
"""


def img(name, caption):
    data = base64.b64encode((SHOTS / name).read_bytes()).decode()
    return (f'<figure><img src="data:image/png;base64,{data}" alt="{html.escape(caption)}">'
            f"<figcaption>{html.escape(caption)}</figcaption></figure>")


def main():
    parts = ["<h1>alba · the revamped look</h1><div class='bar'></div>",
             "<p>Prototype for issue #55, as locked. Every shot is the real alba binary with the fixture data "
             "(Buenos Aires), offline. Click a shot to enlarge it.</p>"]
    for title, about, shots in SECTIONS:
        figures = "".join(img(name, caption) for name, caption in shots)
        parts.append(f"<h2>{html.escape(title)}</h2><p>{html.escape(about)}</p><div class='grid'>{figures}</div>")
    OUT.write_text(f"<!doctype html><meta charset='utf-8'><title>alba · the revamped look</title>"
                   f"<style>{CSS}</style><main>{''.join(parts)}</main><dialog><img></dialog><script>{JS}</script>")
    print(OUT)


main()
