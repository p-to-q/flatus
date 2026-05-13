#!/usr/bin/env bash
# scripts/release.sh — one-tag release flow for v0.1 (unsigned).
#
# Usage: scripts/release.sh 0.1.0
#
# What this does:
#   1. Verifies the working tree is clean and we're on `main`.
#   2. Verifies CHANGELOG.md has an entry for the target version.
#   3. Runs `cargo fmt --check`, `cargo clippy`, `cargo test`.
#   4. Bumps the version in Cargo.toml, tauri.conf.json, package.json, CHANGELOG.md.
#   5. Commits, tags `vX.Y.Z`, and pushes both to origin.
#   6. CI takes it from there (builds the unsigned .app, uploads as artifact).
#
# v0.2 will extend this to invoke `tauri-action` with Apple Developer ID
# secrets. Until then this is the whole release.

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

echo "→ cargo clippy -p fart-synth -- -D warnings"
cargo clippy -p fart-synth --all-targets -- -D warnings

echo "→ cargo test -p fart-synth"
cargo test -p fart-synth

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
