# Contributing to `flatus`

Short on purpose.

## The tree

- `crates/fart-synth/` — Rust synthesis core. Pure DSP. Adding a personality is a one-row patch in `src/personalities.rs`. The hard constraints (output cap, HPF, LPF, session ceiling) live in `src/safety.rs` and are referenced from `src/graph.rs`.
- `apps/desktop/` — Tauri v2 menubar shell. The webview is UI only; synthesis is called via `invoke()`.
- `apps/web/` — static landing page. No audio.
- `skills/fart/` — Claude Skill bundle. Wraps the CLI.
- `fixtures/golden/` — pinned WAVs + SHA-256 manifest. Regenerate with `cargo run --bin generate-goldens`.

## Where things are decided

| Question | File |
| -------- | ---- |
| What does a fart sound like? | `crates/fart-synth/src/graph.rs` |
| How often does it happen? | `crates/fart-synth/src/pressure.rs` |
| Inside one fart, what's the rhythm? | `crates/fart-synth/src/grain.rs` |
| What can be changed without a cap? | `crates/fart-synth/src/safety.rs` |
| Which personalities exist? | `crates/fart-synth/src/personalities.rs` |
| What does the tray click do? | `apps/desktop/src-tauri/src/main.rs` |
| How does CI run? | `.github/workflows/ci.yml` |

## Doctrine

[`docs/ENGINEERING.md`](docs/ENGINEERING.md). Read once.

## Working loop

After cloning, make the shell scripts executable once:

```sh
chmod +x scripts/*.sh
```

Then:

```sh
scripts/doctor.sh                 # confirm your machine has everything
scripts/dev.sh                    # fmt --check + clippy + test
scripts/dev.sh play biblical      # render and play one fart
scripts/dev.sh goldens            # regenerate fixtures/golden/

# Tauri shell:
scripts/dev.sh tauri              # pnpm install + tauri dev
```

The longhand also works:

```sh
cargo fmt --all
cargo clippy -p fart-synth -- -D warnings
cargo test  -p fart-synth
cargo run   -p fart-synth --bin fart -- --personality default --print-state
cargo run   --example inspect_distribution -p fart-synth    # 100 samples per personality
```

If you change synthesis: re-pin the goldens in the same PR.

```sh
cargo run --bin generate-goldens   # from repo root
cargo test -p fart-synth --test golden   # confirms the new pins
```

## PRs

- One feature per PR.
- Normal English commits. No Conventional Commits ceremony.
- If you touched `src/safety.rs` constants, mention it in the commit message and add a line to [`PLAN.md`](PLAN.md) §14.
- If you added a personality, add one entry to `personalities.rs` and one fixture to the `CANON` array in `src/bin/generate_goldens.rs`.

## License

Apache-2.0. By contributing, you agree your changes are licensed the same.

_Q.E.D._
