#!/usr/bin/env bash
# scripts/doctor.sh — "does this machine have what flatus needs."
#
# Reports each prerequisite tool's status. Exits with the count of failures.

set -u
fails=0
ok()    { echo "  ✓ $1"; }
miss()  { echo "  ✘ $1 — $2"; fails=$((fails+1)); }
have()  { command -v "$1" >/dev/null 2>&1; }

echo "→ toolchain"
if have rustc;  then ok "rustc $(rustc --version | awk '{print $2}')"; else miss "rustc"  "install via rustup.rs"; fi
if have cargo;  then ok "cargo $(cargo --version | awk '{print $2}')"; else miss "cargo"  "install via rustup.rs"; fi
if have rustfmt;then ok "rustfmt $(rustfmt --version | awk '{print $2}')"; else miss "rustfmt" "rustup component add rustfmt"; fi
if have cargo-clippy; then ok "clippy $(cargo clippy --version 2>/dev/null | awk '{print $2}')"; else miss "clippy" "rustup component add clippy"; fi

echo "→ tauri prerequisites"
if have node;   then ok "node $(node --version)"; else miss "node" "brew install node"; fi
if have pnpm;   then ok "pnpm $(pnpm --version)"; else miss "pnpm" "brew install pnpm"; fi

if [ "$(uname -s)" = "Darwin" ]; then
  if xcode-select -p >/dev/null 2>&1; then
    ok "xcode-select"
  else
    miss "xcode-select" "run: xcode-select --install"
  fi
fi

echo "→ git / github"
if have git;    then ok "git $(git --version | awk '{print $3}')"; else miss "git" "brew install git"; fi
if have gh;     then
  ok "gh $(gh --version | head -1 | awk '{print $3}')"
  if gh auth status >/dev/null 2>&1; then ok "gh authenticated"; else miss "gh auth" "run: gh auth login"; fi
else
  miss "gh" "brew install gh"
fi

echo "→ audio"
case "$(uname -s)" in
  Darwin)
    # macOS always ships CoreAudio; cpal will find it. Nothing to install.
    ok "CoreAudio (macOS)"
    ;;
  Linux)
    if pkg-config --exists alsa 2>/dev/null; then
      ok "ALSA dev headers"
    else
      miss "libasound2-dev" "sudo apt-get install libasound2-dev"
    fi
    ;;
  *)
    miss "audio" "unknown OS — cpal may or may not find a backend"
    ;;
esac

echo ""
if [ "$fails" -eq 0 ]; then
  echo "✓ flatus is ready to build."
  exit 0
else
  echo "✘ $fails check(s) failed. install the missing pieces and retry."
  exit "$fails"
fi
