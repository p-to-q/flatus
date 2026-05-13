#!/usr/bin/env bash
# flatus installer — Apple Silicon macOS.
#
#   curl -fsSL https://flatus.vercel.app/install.sh | bash
#
# What it does:
#   1. resolves the latest GitHub release of p-to-q/flatus (incl. pre-releases)
#   2. downloads the DMG, mounts it, copies flatus.app into /Applications
#   3. clears the com.apple.quarantine xattr that triggers the misleading
#      "app is damaged and can't be opened" dialog on macOS 15+ for
#      unsigned bundles downloaded through a browser
#
# Manual equivalent if you prefer not to pipe a remote script to bash:
#   - download flatus_*.dmg from https://github.com/p-to-q/flatus/releases
#   - open it, drag flatus.app into /Applications
#   - run: xattr -cr /Applications/flatus.app
#
# The bundle itself is unsigned. Until v0.1 ships notarization, you are
# trusting the GitHub release artifact (and, for the curl|bash path,
# this script). Inspect the source before running.

set -euo pipefail

REPO="p-to-q/flatus"
RELEASES_API="https://api.github.com/repos/${REPO}/releases"

c_dim()   { printf "\033[2m%s\033[0m" "$*"; }
c_bold()  { printf "\033[1m%s\033[0m" "$*"; }
c_red()   { printf "\033[31m%s\033[0m" "$*"; }
c_green() { printf "\033[32m%s\033[0m" "$*"; }

die()  { printf "\n"; c_red "✗ "; echo "$*"; exit 1; }
ok()   { c_green "✓ "; echo "$*"; }
step() { c_dim "·  "; echo "$*"; }

# Preflight ------------------------------------------------------------------

if [[ "$(uname -s)" != "Darwin" ]]; then
  die "flatus only ships a macOS bundle today. On Linux/Windows, build the CLI from source: see https://github.com/${REPO}#install"
fi

if [[ "$(uname -m)" != "arm64" ]]; then
  die "the prebuilt DMG is Apple Silicon only. On Intel Macs, build from source: see https://github.com/${REPO}#install"
fi

for bin in curl hdiutil xattr; do
  command -v "$bin" >/dev/null 2>&1 || die "missing required tool: $bin"
done

# Resolve latest release -----------------------------------------------------

step "resolving latest release on GitHub"
RELEASES_JSON=$(curl -fsSL "$RELEASES_API") \
  || die "could not reach GitHub API ($RELEASES_API)"

TAG=$(printf '%s\n' "$RELEASES_JSON" \
  | grep -E '"tag_name"' | head -1 \
  | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
[[ -n "$TAG" ]] || die "could not parse latest tag from GitHub response"

DMG_URL=$(printf '%s\n' "$RELEASES_JSON" \
  | grep -E '"browser_download_url"' | grep -E '\.dmg"' | head -1 \
  | sed -E 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
[[ -n "$DMG_URL" ]] || die "no DMG asset attached to release $TAG"

DMG_NAME=$(basename "$DMG_URL")
ok "$TAG  ·  $DMG_NAME"

echo
c_bold "flatus installer"; echo "  $TAG  ·  unsigned"
echo

# Download + mount + copy ----------------------------------------------------

WORK=$(mktemp -d -t flatus-install.XXXXXX)
MOUNT=""

cleanup() {
  if [[ -n "$MOUNT" && -d "$MOUNT" ]]; then
    hdiutil detach -quiet "$MOUNT" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

step "downloading $DMG_NAME"
curl -fSL --progress-bar -o "$WORK/$DMG_NAME" "$DMG_URL" \
  || die "download failed ($DMG_URL)"
ok  "downloaded ($(du -h "$WORK/$DMG_NAME" | awk '{print $1}'))"

step "mounting DMG"
MOUNT=$(hdiutil attach -nobrowse -readonly -mountrandom "$WORK" "$WORK/$DMG_NAME" \
  | tail -1 | awk '{$1=""; $2=""; sub(/^[ \t]+/, ""); print}')
[[ -n "$MOUNT" && -d "$MOUNT/flatus.app" ]] \
  || die "flatus.app not found inside DMG (mount: ${MOUNT:-?})"
ok  "mounted at $MOUNT"

if [[ -e "/Applications/flatus.app" ]]; then
  step "replacing existing /Applications/flatus.app"
  rm -rf "/Applications/flatus.app" 2>/dev/null \
    || sudo rm -rf "/Applications/flatus.app"
fi

step "copying flatus.app → /Applications"
if cp -R "$MOUNT/flatus.app" /Applications/ 2>/dev/null; then
  ok "installed"
else
  c_dim "  /Applications not writable as $USER — retrying with sudo"; echo
  sudo cp -R "$MOUNT/flatus.app" /Applications/ \
    || die "copy failed"
  ok "installed (via sudo)"
fi

step "unmounting DMG"
hdiutil detach -quiet "$MOUNT" >/dev/null
MOUNT=""

step "clearing com.apple.quarantine xattr"
if xattr -cr /Applications/flatus.app 2>/dev/null; then
  ok "quarantine cleared"
else
  sudo xattr -cr /Applications/flatus.app
  ok "quarantine cleared (via sudo)"
fi

echo
c_green "✓ "; c_bold "flatus is installed."; echo
echo
echo "  open it now:"
echo "    $(c_dim '$') open /Applications/flatus.app"
echo
echo "  a small icon will appear in your menubar. left-click to fart."
echo
