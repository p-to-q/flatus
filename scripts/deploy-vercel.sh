#!/usr/bin/env bash
# scripts/deploy-vercel.sh
#
# Deploy apps/web/ to Vercel. We stage a temporary directory that contains the
# built site itself as the deployment root; this avoids uploading the whole
# repo and keeps Vercel from trying to infer a project root from unrelated
# files.
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
echo "staging a temporary site root so only public web assets are uploaded."
echo

TMP_SITE="$(mktemp -d "${TMPDIR:-/tmp}/flatus-vercel.XXXXXX")"
trap 'rm -rf "$TMP_SITE"' EXIT

rsync -a "$REPO_ROOT/apps/web/" "$TMP_SITE/"
if [[ -f "$REPO_ROOT/.vercel/project.json" ]]; then
  mkdir -p "$TMP_SITE/.vercel"
  cp "$REPO_ROOT/.vercel/project.json" "$TMP_SITE/.vercel/project.json"
fi
cat > "$TMP_SITE/vercel.json" <<'EOF'
{
  "$schema": "https://openapi.vercel.sh/vercel.json",
  "framework": null,
  "buildCommand": null,
  "installCommand": null,
  "cleanUrls": true,
  "headers": [
    {
      "source": "/(.*)\\.wasm",
      "headers": [
        { "key": "Content-Type", "value": "application/wasm" },
        { "key": "Cache-Control", "value": "public, max-age=31536000, immutable" }
      ]
    },
    {
      "source": "/wasm/(.*)\\.js",
      "headers": [
        { "key": "Content-Type", "value": "text/javascript" },
        { "key": "Cache-Control", "value": "public, max-age=31536000, immutable" }
      ]
    },
    {
      "source": "/(.*)\\.(png|svg|jpg|jpeg|webp|woff2)",
      "headers": [
        { "key": "Cache-Control", "value": "public, max-age=2592000" }
      ]
    },
    {
      "source": "/(.*)\\.wav",
      "headers": [
        { "key": "Content-Type", "value": "audio/wav" },
        { "key": "Cache-Control", "value": "public, max-age=2592000" }
      ]
    },
    {
      "source": "/install.sh",
      "headers": [
        { "key": "Content-Type", "value": "text/x-shellscript; charset=utf-8" },
        { "key": "Cache-Control", "value": "public, max-age=300, must-revalidate" }
      ]
    }
  ]
}
EOF

# --yes accepts defaults so this is safe in CI / agent contexts. Copying the
# repo's `.vercel/project.json` into the temp root keeps production deploys
# attached to the real `flatus` project instead of creating ad-hoc projects.
npx --yes vercel deploy "$TMP_SITE" --yes "${ARGS[@]+"${ARGS[@]}"}"
