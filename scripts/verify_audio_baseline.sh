#!/usr/bin/env bash
# Verify the current flatus audio baseline across fixtures, web samples, and the CLI.
#
# What it checks:
#   1. `fixtures/golden/*.wav` and `apps/web/samples/v0.4/*.wav` have identical hashes.
#   2. The fixture manifest and the web manifest are identical.
#   3. The CLI re-renders the canonical `(personality, seed, pressure)` tuples to the
#      same WAV bytes listed in `fixtures/golden/manifest.json`.
#   4. The interactive web reference in `apps/web/main.js` still points at the
#      expected preview seeds / pressure / session shape used for release signoff.
#
# Run from the repo root:
#   bash scripts/verify_audio_baseline.sh

set -euo pipefail

cd "$(dirname "$0")/.."

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "== fixture vs web sample parity =="
for name in polite-cough default biblical silent-but-deadly; do
  fixture="fixtures/golden/${name}.wav"
  web="apps/web/samples/v0.4/${name}.wav"
  fixture_hash=$(shasum -a 256 "$fixture" | awk '{print $1}')
  web_hash=$(shasum -a 256 "$web" | awk '{print $1}')
  if [[ "$fixture_hash" != "$web_hash" ]]; then
    echo "hash mismatch for ${name}:"
    echo "  fixture: $fixture_hash"
    echo "  web:     $web_hash"
    exit 1
  fi
  echo "  ${name}: ${fixture_hash}"
done

echo
echo "== manifest parity =="
cmp -s fixtures/golden/manifest.json apps/web/samples/v0.4/manifest.json
echo "  fixture and web manifests match"

echo
echo "== CLI render parity =="
jq -r '.fixtures[] | [.personality, (.seed|tostring), (.pressure|tostring), .sha256] | @tsv' \
  fixtures/golden/manifest.json |
while IFS=$'\t' read -r personality seed pressure expected_hash; do
  out="$TMP_DIR/${personality}.wav"
  cargo run --quiet --bin fart -- \
    --personality "$personality" \
    --seed "$seed" \
    --pressure "$pressure" \
    --render "$out" >/dev/null
  actual_hash=$(shasum -a 256 "$out" | awk '{print $1}')
  if [[ "$actual_hash" != "$expected_hash" ]]; then
    echo "CLI mismatch for ${personality}:"
    echo "  expected: $expected_hash"
    echo "  actual:   $actual_hash"
    exit 1
  fi
  echo "  ${personality}: ${actual_hash}"
done

echo
echo "== interactive web reference =="
python3 - <<'PY'
import pathlib
import re
import sys

text = pathlib.Path("apps/web/main.js").read_text()

expected = {
    "polite-cough": "7",
    "default": "17",
    "biblical": "31",
    "silent-but-deadly": "9",
}

for name, seed in expected.items():
    pattern = rf'"{re.escape(name)}":\s*{seed}\b'
    if not re.search(pattern, text):
        print(f"missing or changed preview seed for {name}: expected {seed}", file=sys.stderr)
        sys.exit(1)

checks = {
    "DEFAULT_PRESSURE": r"const DEFAULT_PRESSURE = 0\.6;",
    "SESSION_EVENTS": r"const SESSION_EVENTS = 3;",
    "SESSION_GAP_MS": r"const SESSION_GAP_MS = 280;",
}

for label, pattern in checks.items():
    if not re.search(pattern, text):
        print(f"missing or changed web preview constant {label}", file=sys.stderr)
        sys.exit(1)

print("  web preview constants match release reference")
PY

echo
echo "audio baseline verified"
