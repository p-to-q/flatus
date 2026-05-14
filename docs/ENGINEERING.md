# `flatus/docs/ENGINEERING.md`

> Short on purpose. Read once, keep open. _Long variable names don't hurt anyone._

---

## 1. Doctrine

These are merge conditions, not aspirations.

- **Flat is better than nested.** Two top-level codebases (`crates/`, `apps/`). No `utils/`, `helpers/`, `core/`, `lib/`.
- **Printability is a great feature.** Every doc fits on a screen.
- **Human-readable interfaces for machine-produced things.** `FartParams` is JSON. Golden fixtures are WAVs. Logs are NDJSON. Settings are a single `settings.json`.
- **Overly abstract means narcissism.** Adding a personality is adding one row in `personalities.rs`. No registries, no plugin systems.
- **Long variable names don't hurt anyone.** `sample_rate_hz`, not `sr`. Type aliases for units (`Hz`, `Ms`, `DbFs`).

---

## 2. Naming

| Thing                     | Convention                       | Example                              |
| ------------------------- | -------------------------------- | ------------------------------------ |
| Repos                     | `lower-kebab`                    | `flatus`                             |
| Rust crates               | `kebab-case`                     | `fart-synth`                         |
| Rust modules              | `snake_case.rs`                  | `personalities.rs`                   |
| TS files                  | `kebab-case.ts`                  | `main.ts`                            |
| Types                     | `PascalCase`                     | `FartParams`, `Personality`          |
| Functions                 | Verbs, full words                | `build_graph`, `render_to_wav`       |
| Variables                 | `snake_case`, full nouns         | `sample_rate_hz`, `master_gain_dbfs` |
| Constants                 | `SCREAMING_SNAKE`                | `MAX_SESSION_MS`, `HPF_HZ`           |

Abbreviations only when they are units the audio world already uses (`Hz`, `dB`, `ms`, `fs`, `Xmax`).

---

## 3. Code rules (compressed)

**Rust** — `cargo fmt`; `cargo clippy -D warnings`; no `unsafe` without a paragraph in PLAN.md explaining why; no `unwrap()` outside tests and `bin/`; any function that reads randomness takes `rng: &mut Mulberry32` as a parameter (no global RNG).

**TypeScript / desktop webview** — keep the Tauri webview build-step free; no frameworks; Web Audio is _not_ used for synthesis (synth lives in Rust); if TS is reintroduced as a build source, it stays strict and mirrors the shipped JS instead of drifting from it.

**Shared** — one feature = one PR; review counts as the second pair of eyes (self-review allowed for the day-one team); commits in normal English (no Conventional Commits ceremony); PRs that touch synthesis re-pin `fixtures/golden/manifest.json` in the same PR.

---

## 4. Determinism contract

Rust-only. There is no cross-implementation parity to maintain, because there is no second implementation (the webview is UI, not synthesis).

- **One seed → one waveform.** `fart-synth` produces byte-identical output for the same `FartParams` (including `seed`) on the same Rust toolchain.
- **CI uses locked seeds.** `fixtures/golden/manifest.json` lists each WAV with `personality`, `seed`, `sha256`. Day-one runtime uses system seed; CI uses the manifest's seed.
- **A failed golden test is a code-change signal, not a test bug.** Re-pin the manifest in the PR that intentionally changes synthesis. Reviewer listens to the diff.

The browser implementation uses the same Rust synthesis core via wasm. When we
change synthesis, we update the web samples and golden fixtures together.

---

## 5. Hard constraints

Live in code, not prose. [`crates/fart-synth/src/safety.rs`](../crates/fart-synth/src/safety.rs):

```rust
pub const MAX_OUTPUT_DBFS: f32 = -6.0;
pub const HEADPHONE_DBFS: f32 = -18.0;
pub const HPF_HZ: f32 = 60.0;
pub const LPF_HZ: f32 = 2_000.0;
pub const MAX_SESSION_MS: u32 = 30_000;
pub const MIN_COOLDOWN_MS: u32 = 60_000;
```

`graph::render` asserts the dBFS cap in a unit test. Changing the numbers is fine; the commit message explains why, the tests still pass, life goes on. No separate `HARD-CONSTRAINTS.md` to keep in sync.

**Headphone routing**: no CoreAudio auto-detection. The desktop window exposes a Speakers / Headphones toggle, defaulting to Speakers so manual preview matches the website reference more closely. One sentence in the README explains the tradeoff.

---

## 6. Docs surface

Keep the docs surface small. Don't add a new file unless it earns its place.

| File                  | Lives at        | Purpose                                                   |
| --------------------- | --------------- | --------------------------------------------------------- |
| `README.md`           | repo root       | Public face. OpenWhip-tier. ~40 lines, joke roadmap.      |
| `docs/PLAN.md`        | `docs/`         | Internal. Philosophy, milestones, architecture, history.  |
| `docs/ACOUSTICS.md`   | `docs/`         | Citation-backed plausibility writeup.                     |
| `docs/AUDIO_BASELINE.md` | `docs/`      | Technical + listening reference for release signoff.      |
| `docs/PRODUCT_BACKLOG.md` | `docs/`    | Post-launch cleanup and product-quality backlog.          |
| `docs/PLUGIN_RESEARCH.md` | `docs/`    | Next-phase extension and cadence research.                |
| `docs/ENGINEERING.md` | `docs/` (this)  | How we write code.                                        |

No `THESIS.md`, no `glossary.md`, no `rfcs/`, no `adrs/`. If a design decision worth recording arises, append a dated paragraph to `PLAN.md`'s history section (§14).

---

## 7. Voice

Two registers, intentional:

- **`README.md`** — OpenWhip-tier. Casual, slightly self-deprecating, joke roadmap, ~40 lines, no philosophy. Comedy on the surface.
- **`docs/PLAN.md` and `docs/ACOUSTICS.md`** — Wittgenstein-key. Indicative mood, short declarative sentences, no exclamation marks, em-dashes used surgically, primary sources quoted without irony. Seriousness in the interior.

If a sentence in the README would look out of place in OpenWhip's README, rewrite it. If a sentence in `PLAN.md` would look out of place next to a quote from the Tractatus, rewrite that one.

---
