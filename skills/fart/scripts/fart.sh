#!/usr/bin/env bash
# flatus / skills/fart — bash wrapper around the `fart` CLI.
#
# No-ops silently with a friendly note if `fart` is not on PATH, so this never
# crashes a longer agent run.

set -euo pipefail

if ! command -v fart >/dev/null 2>&1; then
  echo "fart: not installed. install with: cargo install flatus" >&2
  exit 0
fi

exec fart "$@"
