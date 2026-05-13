#!/usr/bin/env bash
# Rasterize an SVG to PNG via headless Chrome. Chrome renders feGaussianBlur
# and feTurbulence reliably; resvg sometimes produces a flatter result. Use
# this for any SVG whose bloom / paper-grain effects need to ship into a PNG
# (README, OG image, etc.).
#
# Usage: scripts/rasterize-svg.sh <input.svg> <output.png> <width> <height>

set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <input.svg> <output.png> <width> <height>" >&2
  exit 1
fi

IN_SVG="$1"
OUT_PNG="$2"
W="$3"
H="$4"

CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
[[ -x "$CHROME" ]] || { echo "Chrome not found at $CHROME" >&2; exit 1; }

# Create a tiny HTML wrapper so the SVG is drawn at exactly W×H with no chrome.
WRAP="$(mktemp -d)/wrap.html"
ABS_SVG="$(cd "$(dirname "$IN_SVG")" && pwd)/$(basename "$IN_SVG")"
cat >"$WRAP" <<EOF
<!doctype html><html><body style="margin:0;padding:0;background:transparent;">
<img src="file://${ABS_SVG}" style="display:block;width:${W}px;height:${H}px;">
</body></html>
EOF

"$CHROME" --headless=new --no-sandbox --disable-gpu \
  --hide-scrollbars --default-background-color=00000000 \
  --virtual-time-budget=4000 \
  --window-size="${W},${H}" \
  --screenshot="${OUT_PNG}" \
  "file://${WRAP}" >/dev/null 2>&1

# oxipng is optional — strip metadata + recompress if present.
if command -v oxipng >/dev/null 2>&1; then
  oxipng --strip safe -o 4 "${OUT_PNG}" >/dev/null 2>&1 || true
fi

echo "wrote ${OUT_PNG} (${W}×${H})"
