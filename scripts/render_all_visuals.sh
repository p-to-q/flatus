#!/usr/bin/env bash
# Regenerate every doc/site visual from sources in one shot.
#
# Pipeline:
#   1. render_visuals.py emits the SVG sources for screenshots + marks.
#   2. rasterize-svg.sh wraps headless Chrome to bake feGaussianBlur +
#      feTurbulence into a PNG (resvg flattens those filters).
#   3. Mirror the rasterized PNGs into apps/web/ so Vercel serves them.
#
# Run from the repo root:
#   bash scripts/render_all_visuals.sh

set -euo pipefail

cd "$(dirname "$0")/.."

# 1. SVG sources
python3 scripts/render_visuals.py

# 2. Rasterize. Each row is: <svg path> <png path> <width> <height>
ras() { scripts/rasterize-svg.sh "$1" "$2" "$3" "$4"; }

ras docs/screenshots/spectrogram-biblical.svg \
    docs/screenshots/spectrogram-biblical.png 1600 520
ras docs/screenshots/waveforms-all.svg \
    docs/screenshots/waveforms-all.png        1600  600
ras docs/marks/wordmark.svg   docs/marks/wordmark.png   1200 360
ras docs/marks/signature.svg  docs/marks/signature.png  1200  300
ras docs/marks/monogram.svg   docs/marks/monogram.png    640  640
ras docs/marks/og-card.svg    docs/marks/og-card.png    1200  630

# 3. Mirror PNGs into apps/web/ (Vercel deploys only apps/web/).
#    docs/* is the editable source; apps/web/* is the deployment.
mkdir -p apps/web/screenshots apps/web/marks
cp docs/screenshots/spectrogram-biblical.png apps/web/screenshots/spectrogram-biblical.png
cp docs/screenshots/waveforms-all.png        apps/web/screenshots/waveforms-all.png
cp docs/marks/wordmark.png                   apps/web/marks/wordmark.png
cp docs/marks/signature.png                  apps/web/marks/signature.png
cp docs/marks/monogram.png                   apps/web/marks/monogram.png
cp docs/marks/og-card.png                    apps/web/marks/og-card.png

# The og:image reference in apps/web/index.html still points at
# apps/web/og-card.png (top-level) — refresh it from the new paper-canvas card.
cp docs/marks/og-card.png apps/web/og-card.png

# 4. DMG background as a multi-rep TIFF (1x + 2x with differential DPI).
#
#    Tauri's bundle_dmg.sh copies the background file as-is into the .dmg and
#    Finder displays it at NATIVE pixel size. A single 1080×760 PNG renders
#    1:1 on non-retina displays — Finder shows only its top-left 540×380
#    region and the centred wordmark lands at the right edge (looks "off
#    centre"). The fix is a multi-image TIFF: frame 0 = 540×380 @ 72 DPI for
#    1x displays, frame 1 = 1080×760 @ 144 DPI for retina. Finder picks the
#    matching rep based on the display's DPI.
#
#    tiffutil (default on macOS) concatenates but normalises both frames to
#    72 DPI, so we need tiffset (from libtiff, `brew install libtiff`) to
#    patch the 2x frame's DPI to 144 after concatenation. If tiffset is
#    missing the script falls back to a 1x-only PNG — the DMG still looks
#    correct, just not retina-sharp.
DMG_RES=apps/desktop/resources
TMP=$(mktemp -d)
scripts/rasterize-svg.sh "$DMG_RES/dmg-background.svg" "$TMP/dmg-1x.png"  540 380
scripts/rasterize-svg.sh "$DMG_RES/dmg-background.svg" "$TMP/dmg-2x.png" 1080 760
cp "$TMP/dmg-1x.png" "$DMG_RES/dmg-background.png"
cp "$TMP/dmg-2x.png" "$DMG_RES/dmg-background@2x.png"

sips -s format tiff "$TMP/dmg-1x.png" --out "$TMP/dmg-1x.tiff" >/dev/null
sips -s format tiff "$TMP/dmg-2x.png" --out "$TMP/dmg-2x.tiff" >/dev/null
tiffutil -cathidpicheck "$TMP/dmg-1x.tiff" "$TMP/dmg-2x.tiff" \
  -out "$DMG_RES/dmg-background.tiff" >/dev/null
if command -v tiffset >/dev/null 2>&1; then
  # tiffutil normalises DPI on both frames to 72; patch frame 1 to 144 so
  # Finder treats it as the @2x rep on retina displays.
  tiffset -d 1 -s 282 144.0 "$DMG_RES/dmg-background.tiff"
  tiffset -d 1 -s 283 144.0 "$DMG_RES/dmg-background.tiff"
  echo "  retina TIFF written (1x 540×380 @ 72 DPI + 2x 1080×760 @ 144 DPI)"
else
  echo "  WARN: tiffset not found — TIFF written with both frames at 72 DPI."
  echo "        Retina users will see the 1x rep upscaled. To fix, install"
  echo "        libtiff (brew install libtiff) and re-run this script."
fi
rm -rf "$TMP"

echo "✓ all visuals regenerated + mirrored into apps/web/ + dmg-background.tiff rebuilt"
