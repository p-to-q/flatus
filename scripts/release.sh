#!/usr/bin/env bash
# scripts/release.sh — one-tag release flow for the current unsigned desktop line.
#
# Usage: scripts/release.sh 0.2.1
#
# What this does:
#   1. Verifies the working tree is clean and we're on `main`.
#   2. Verifies CHANGELOG.md has an entry for the target version.
#   3. Runs workspace formatting, clippy, tests, and audio-baseline verification.
#   4. Bumps the version in the workspace, desktop bundle metadata, README, and web release pointers.
#   5. Commits, tags `vX.Y.Z`, and pushes both to origin.
#   6. CI takes it from there (builds the unsigned `.app` / `.dmg`, uploads release assets).

set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <version> (e.g. $0 0.1.0)" >&2
  exit 64
fi

VERSION="$1"
TAG="v${VERSION}"

# Sanity gates --------------------------------------------------------------

if [ -n "$(git status --porcelain)" ]; then
  echo "✘ working tree is not clean. commit or stash before releasing." >&2
  exit 1
fi

BRANCH="$(git symbolic-ref --short HEAD)"
if [ "$BRANCH" != "main" ]; then
  echo "✘ not on main (currently on ${BRANCH}). switch first." >&2
  exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "✘ tag ${TAG} already exists." >&2
  exit 1
fi

if ! grep -q "\[${VERSION}\]" CHANGELOG.md; then
  echo "✘ CHANGELOG.md has no entry for [${VERSION}]. add one and retry." >&2
  exit 1
fi

# Build gates ---------------------------------------------------------------

echo "→ cargo fmt --check"
cargo fmt --all -- --check

echo "→ cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "→ cargo test --workspace"
cargo test --workspace

echo "→ bash scripts/verify_audio_baseline.sh"
bash scripts/verify_audio_baseline.sh

# Bump version --------------------------------------------------------------

echo "→ bumping version to ${VERSION}"

# Workspace Cargo.toml
sed -i.bak -E "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
rm Cargo.toml.bak

# Tauri config
sed -i.bak -E "s/(\"version\": \")[^\"]+(\")/\1${VERSION}\2/" \
  apps/desktop/src-tauri/tauri.conf.json
rm apps/desktop/src-tauri/tauri.conf.json.bak

# Desktop package.json
sed -i.bak -E "s/(\"version\": \")[^\"]+(\")/\1${VERSION}\2/" \
  apps/desktop/package.json
rm apps/desktop/package.json.bak

# README release pointers + asset names
sed -i.bak -E "s/releases\/tag\/v[0-9]+\.[0-9]+\.[0-9]+/releases\/tag\/${TAG}/g" README.md
sed -i.bak -E "s/flatus_[0-9]+\.[0-9]+\.[0-9]+_aarch64\.dmg/flatus_${VERSION}_aarch64.dmg/g" README.md
sed -i.bak -E "s/flatus-v[0-9]+\.[0-9]+\.[0-9]+-aarch64\.app\.zip/flatus-v${VERSION}-aarch64.app.zip/g" README.md
rm README.md.bak

# Website download CTA + release metadata
sed -i.bak -E "s/releases\/tag\/v[0-9]+\.[0-9]+\.[0-9]+/releases\/tag\/${TAG}/g" apps/web/index.html
sed -i.bak -E "s/v[0-9]+\.[0-9]+\.[0-9]+ · unsigned/${TAG} · unsigned/g" apps/web/index.html
rm apps/web/index.html.bak

sed -i.bak -E "s/(const LATEST_TAG = \")v[0-9]+\.[0-9]+\.[0-9]+(\")/\1${TAG}\2/" apps/web/main.js
sed -i.bak -E "s/flatus_[0-9]+\.[0-9]+\.[0-9]+_aarch64\.dmg/flatus_${VERSION}_aarch64.dmg/g" apps/web/main.js
rm apps/web/main.js.bak

# Skill metadata
sed -i.bak -E "s/(version: \")[^\"]+(\")/\1${VERSION}\2/" skills/fart/SKILL.md
rm skills/fart/SKILL.md.bak

# Commit + tag --------------------------------------------------------------

git add -A
git commit -m "release: ${TAG}"
git tag -a "${TAG}" -m "${TAG}"

echo "→ pushing to origin"
git push origin main
git push origin "${TAG}"

echo ""
echo "✓ released ${TAG}"
echo "  CI will now build the unsigned .app on macos-latest."
echo "  Watch:  https://github.com/p-to-q/flatus/actions"
