# Changelog

## [Unreleased]

## [0.1.0] — 2026-05-13 (target)

Initial release. The comedy ships first; signing comes later.

### Added
- `crates/fart-synth` — Rust synthesis core. Pressure state machine, granular envelope, 7-dimensional parameter space, four personality distributions (`polite-cough`, `default`, `biblical`, `silent-but-deadly`), mulberry32 seeded PRNG, RBJ biquad BPF/HPF/LPF, asymmetric tanh waveshaper, comb-filter wetness, dBFS-capped limiter.
- `bin/fart` — CLI. Flags: `--personality`, `--seed`, `--render <out.wav>`, `--print-state`. Plays via cpal default device.
- `apps/desktop` — Tauri v2 menubar shell. `ActivationPolicy::Accessory` (no dock icon). Tray click fires a fart; right-click (or ⌘-click) opens the settings popover. **Unsigned** `.app` only; no DMG, no notarization in v0.1.
- `apps/web` — static landing page placeholder.
- `skills/fart` — Claude Skill bundle with bash wrapper around the CLI.
- `fixtures/golden` — manifest scaffold; canonical WAVs generated at first `cargo test`.
- Docs: `README.md` (OpenWhip-tier public face), `PLAN.md` (internal plan, philosophy and architecture), `docs/ACOUSTICS.md` (citation-backed plausibility writeup), `docs/ENGINEERING.md` (conventions).
- License: Apache-2.0.

### Hard constraints (frozen)
- Output cap −6 dBFS (speakers) / −18 dBFS (headphones; user toggle, default Headphones).
- HPF at 60 Hz; LPF at 2 kHz on every render.
- Max session 30 s; min cooldown 60 s.
- No telemetry. No network at runtime. No LLM at runtime.

### Known limits
- macOS only on day one. Windows/Linux deferred.
- Apple Silicon only (no universal binary yet).
- Unsigned `.app`; first launch requires Gatekeeper bypass. See `SECURITY.md` when it lands.
- "165 Hz" Apple Watch frequency is community lore, not Apple-disclosed. See `docs/ACOUSTICS.md`.
