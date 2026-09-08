#!/usr/bin/env python3
"""
draw-dmg-background.py — render the macOS DMG install window background.

The DMG window is the first thing a Mac user sees of Virtues, so it gets the
same register as the airlock: white paper, graphite ink, JJannon for the one
line of serif, Avenir for the instruction. No color, no texture.

Geometry follows `bundle.macOS.dmg` in apps/web/src-tauri/tauri.conf.json
(window 660x400; icon centers at (180,195) and (480,195), 128px icons), so the
title sits above the icons and the arrow sits between them. Change one, change
the other.

Output is drawn at 2x and tagged 144 dpi: Finder reads the dpi and shows it at
660x400 points, crisp on Retina, correctly sized on plain displays.

Needs: pillow, fonttools, brotli (woff2 → ttf happens in a temp dir).

Usage:  python3 tools/draw-dmg-background.py
"""
import tempfile
from pathlib import Path

from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parent.parent
FONTS = REPO / "apps/web/static/fonts"
OUT = REPO / "apps/web/src-tauri/dmg/background.png"

W, H = 660, 400          # window size in points
S = 2                    # render scale
APP_X, FOLDER_X, ICON_Y = 180, 480, 195   # icon centers, from tauri.conf.json
ICON = 128

PAPER = "#FFFFFF"
INK = "#17171A"          # --foreground
MUTED = "#71717A"        # --foreground-subtle
HAIR = "#A1A1AA"         # --foreground-disabled


def ttf(name: str, tmp: Path) -> Path:
    f = TTFont(FONTS / f"{name}.woff2")
    f.flavor = None
    out = tmp / f"{name}.ttf"
    f.save(out)
    return out


def main() -> None:
    with tempfile.TemporaryDirectory() as d:
        tmp = Path(d)
        serif = ImageFont.truetype(str(ttf("JJannon-Display-Regular", tmp)), 40 * S)
        sans = ImageFont.truetype(str(ttf("Avenir-Regular", tmp)), 14 * S)

        im = Image.new("RGB", (W * S, H * S), PAPER)
        dr = ImageDraw.Draw(im)

        # Title and instruction, centered above the icons.
        dr.text((W * S / 2, 62 * S), "Virtues", font=serif, fill=INK, anchor="mm")
        dr.text((W * S / 2, 96 * S), "Drag Virtues to Applications", font=sans, fill=MUTED, anchor="mm")

        # A short dashed hairline between the two icons, with an open chevron.
        # Dashes read as "movement" the way a drawn diagram's arrow does.
        gap = 52
        x0 = (APP_X + ICON / 2 + gap) * S
        x1 = (FOLDER_X - ICON / 2 - gap) * S
        y = ICON_Y * S
        w = int(1.25 * S)
        dash, space = 5 * S, 4 * S
        x = x0
        while x < x1 - 2 * S:
            dr.line([(x, y), (min(x + dash, x1 - 2 * S), y)], fill=HAIR, width=w)
            x += dash + space
        head = 5 * S
        dr.line([(x1 - head, y - head), (x1, y), (x1 - head, y + head)], fill=HAIR, width=w, joint="curve")

        OUT.parent.mkdir(parents=True, exist_ok=True)
        im.save(OUT, dpi=(72 * S, 72 * S), optimize=True)
        print(f"wrote {OUT.relative_to(REPO)} ({W*S}x{H*S} @ {72*S} dpi)")


if __name__ == "__main__":
    main()
