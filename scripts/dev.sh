#!/usr/bin/env bash
# scripts/dev.sh — the inner loop.
#
# Usage:
#   scripts/dev.sh              # build + test + clippy
#   scripts/dev.sh play         # render and play one default fart
#   scripts/dev.sh play <name>  # play a named personality
#   scripts/dev.sh goldens      # regenerate fixtures/golden/*.wav and manifest
#   scripts/dev.sh tauri        # bring up the menubar app in dev mode
#   scripts/dev.sh fmt          # cargo fmt --all
#   scripts/dev.sh ci           # the full CI sequence locally
#
# Run from the repo root.

set -euo pipefail

cmd="${1:-check}"
shift || true

case "$cmd" in
  check|"")
    cargo fmt --all -- --check
    cargo clippy -p fart-synth --all-targets -- -D warnings
    cargo test  -p fart-synth
    ;;

  play)
    name="${1:-default}"
    cargo run -p fart-synth --release --bin fart -- --personality "$name" --print-state
    ;;

  goldens)
    cargo run -p fart-synth --release --bin generate-goldens
    cargo test  -p fart-synth --test golden
    ;;

  tauri)
    cd apps/desktop
    pnpm install
    pnpm tauri dev
    ;;

  fmt)
    cargo fmt --all
    ;;

  ci)
    cargo fmt --all -- --check
    cargo clippy -p fart-synth --all-targets -- -D warnings
    cargo test  -p fart-synth
    cargo build -p fart-synth --release
    cargo run   -p fart-synth --release --bin fart -- --help >/dev/null
    echo "✓ ci sequence passed"
    ;;

  *)
    echo "usage: scripts/dev.sh [check|play|goldens|tauri|fmt|ci]" >&2
    exit 64
    ;;
esac
