#!/usr/bin/env bash
# scripts/deploy-vercel.sh
#
# Deploy apps/web/ to Vercel. Static site, no build step — Vercel just lifts
# the directory and serves it through its CDN with the headers configured in
# vercel.json (WASM MIME, audio/wav MIME, immutable wasm cache, etc.).
#
# Authentication (any one):
#   - run `npx vercel login` once interactively first, OR
#   - export VERCEL_TOKEN before running this script, OR
#   - put the token in ~/.vercel-token (mode 600); the script will read it.
#
# Usage:
#   scripts/deploy-vercel.sh             # preview deploy (branch URL)
#   scripts/deploy-vercel.sh --prod      # production alias

set -euo pipefail

cd "$(dirname "$0")/.."
REPO_ROOT="$PWD"

# Surface a token from ~/.vercel-token into the env if present and unset.
if [[ -z "${VERCEL_TOKEN:-}" && -r "$HOME/.vercel-token" ]]; then
  VERCEL_TOKEN="$(<"$HOME/.vercel-token")"
  export VERCEL_TOKEN
fi

ARGS=()
if [[ "${1:-}" == "--prod" ]]; then
  ARGS+=("--prod")
fi
if [[ -n "${VERCEL_TOKEN:-}" ]]; then
  ARGS+=("--token" "$VERCEL_TOKEN")
fi

echo "Deploying apps/web/ to Vercel from $REPO_ROOT"
echo "vercel.json drives outputDirectory + headers; no build step runs."
echo

# --yes accepts the auto-generated project name + defaults so this is safe
# in CI / agent contexts.
npx --yes vercel deploy --yes "${ARGS[@]}"
