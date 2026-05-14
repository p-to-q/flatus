#!/usr/bin/env python3
"""Render the macOS menubar tray icon.

A template image: only the alpha channel is used; macOS tints it. Output is a
single italic 'f' (Charter Italic) centred in a 22x22 base, with a 44x44 @2x
companion. The glyph reads as the brand monogram at menubar size.

Run from the repo root:
  python3 scripts/render_tray_icon.py
"""

from __future__ import annotations

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parent.parent
OUT_DIR = REPO / "apps" / "desktop" / "src-tauri" / "icons"

CHARTER = "/System/Library/Fonts/Supplemental/Charter.ttc"
CHARTER_ITALIC_INDEX = 1


def render_tray(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Pick a font size that lets the italic 'f' descender + ascender fit
    # comfortably with a hairline of breathing room. Empirically: ~1.7x the
    # canvas height for italic 'f' to land cleanly at menubar density.
    font_px = int(size * 1.65)
    font = ImageFont.truetype(CHARTER, font_px, index=CHARTER_ITALIC_INDEX)

    # Use a single-character bbox to centre the glyph optically rather than
    # by font metrics (which include side-bearings that visually push the
    # italic 'f' off-centre).
    bbox = draw.textbbox((0, 0), "f", font=font)
    glyph_w = bbox[2] - bbox[0]
    glyph_h = bbox[3] - bbox[1]
    x = (size - glyph_w) // 2 - bbox[0]
    y = (size - glyph_h) // 2 - bbox[1]
    draw.text((x, y), "f", font=font, fill=(0, 0, 0, 255))

    return img


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    base = render_tray(22)
    retina = render_tray(44)

    base_path = OUT_DIR / "tray-template.png"
    retina_path = OUT_DIR / "tray-template@2x.png"
    base.save(base_path)
    retina.save(retina_path)
    print(f"wrote {base_path}")
    print(f"wrote {retina_path}")


if __name__ == "__main__":
    main()
