#!/usr/bin/env python3
"""Render the macOS menubar tray icon.

A template image: only the alpha channel is used; macOS tints it. Output is a
22x22 base (44x44 @2x): Charter italic **f** scaled to fit inside the slot with
padding, plus three small grains under the baseline (same vocabulary as
`docs/marks/monogram.svg`, simplified for template silhouette).

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

    # Reserve bottom band for three “grain” dots; keep the ascender + descender
    # of italic f inside the remaining height. (Older builds used ~1.65× size,
    # which blew past 22px and only the middle of the stroke showed in the bar.)
    reserve_bottom = max(4, int(round(size * 0.22)))
    content_h = size - reserve_bottom

    font_px = max(9, int(round(size * 0.56)))
    font = ImageFont.truetype(CHARTER, font_px, index=CHARTER_ITALIC_INDEX)

    bbox = draw.textbbox((0, 0), "f", font=font)
    glyph_w = bbox[2] - bbox[0]
    glyph_h = bbox[3] - bbox[1]
    x = (size - glyph_w) // 2 - bbox[0]
    y = (content_h - glyph_h) // 2 - bbox[1]
    draw.text((x, y), "f", font=font, fill=(0, 0, 0, 255))

    ink = (0, 0, 0, 255)
    dot_r = max(1, int(round(size * 0.10)))
    dot_y = size - int(round(size * 0.12))
    step = size * 0.19
    cx0 = size / 2
    for dx in (-step, 0.0, step):
        cx = cx0 + dx
        draw.ellipse(
            [cx - dot_r, dot_y - dot_r, cx + dot_r, dot_y + dot_r],
            fill=ink,
        )

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
