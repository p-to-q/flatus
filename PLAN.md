# `flatus` — The Plan (v0.1)

> _Pneuma is the wind of speech._ — Aristotle, _De Anima_ II.8 (paraphr.)
>
> **A small apparatus for moving air.**
>
> The second proof in the `[p → q]` lineage. After **`wittgenstein`** comes **`flatus`** — a desktop companion that vibrates a laptop speaker near its resonance, _in the manner of a fart_.

This document is the **internal** plan — design, philosophy, milestones. The **public** face of the repo is the OpenWhip-tier `README.md`; the two-tier voice is intentional (see §10).

---

## 1. The arrow

```
p   surface form          a fart sound on your desktop, intermittent
→   pressure → release    a stateful biological model exhales through
q   consequence           a diaphragm at fs displaces air at the grille
```

Read the output as audio and your friends laugh. Read the same node graph as a function and you have the engineering Apple did for water in the Apple Watch, scaled to a laptop microspeaker. The waveform that makes the joke and the waveform that does the work are the same waveform. That is what `[p → q]` means here.

---

## 2. Philosophy: a body that isn't there

Most desktop pets work by being _seen_ — Clippy in the corner, Tamagotchi asking to be fed, a sprite walking across the screen. They are visual and usually need-driven; failing to attend to them is a state they reach.

`flatus` is not that. It has no form, no image, no demand, no scoreboard. It is _acoustic_ and _anti-performative_ — it does not ask for your attention; it just occasionally happens.

> **`flatus` is a body that isn't there.**
>
> The software environment is gravity-less. Nothing in it has weight, secretions, or digestion. `flatus` is the small piece of body-ness smuggled into that environment — it does not mimic the _shape_ of a body (visual) but does the _things_ a body does (a fart being the lowest-status, least-controllable, most-honestly-bodily of those things).
>
> Its purpose is not to keep you company. It is to make your laptop seem _inhabited_.

This connects to the org's stated drive — _AI should serve as a deeper connective tissue that strengthens humans._ Here the connective tissue is inverted: a small physical presence imported into the disembodied environment, reminding you that bodies are real. The joke and the metaphysics are the same waveform.

Three operational consequences:

1. **Does not request, does not notify.** No "time to fart" popup. No "today's count" stat page. No state that needs caretaking.
2. **Has internal state, but state is not exposed.** It has its own pressure, its own mood — the UI does not show them. You can only infer from how it farts, the way you can only infer your housemate's hunger from the noises in the kitchen.
3. **Invisible but adjustable.** Settings expose volume, personality, output (speakers / headphones), quiet hours. _Not_ pressure values, mood scalars, or internal timers. The hiding is part of the aesthetic.

---

## 3. Rhythm: macro and micro

### 3.1 Macro: a pressure model

Forget "interval + jitter, cron-style." Use a **biological pressure-accumulation model** — coincidentally the same dynamics as actual intestinal gas. See [`crates/fart-synth/src/pressure.rs`](crates/fart-synth/src/pressure.rs).

```
pressure(t) = pressure(t-1) + base_rate + activity_bonus - decay
if pressure > threshold + uniform_noise:
    fart()
    pressure = residual
    enter refractory period
```

| Personality         | base_rate | activity_bonus | threshold_noise | refractory |
| ------------------- | --------- | -------------- | --------------- | ---------- |
| `polite-cough`      | 0.5 / hr  | ×1.2           | ±0.5            | 180 s      |
| `default`           | 1.0 / hr  | ×1.5           | ±0.3            | 90 s       |
| `biblical`          | 1.33 / hr | ×1.3           | ±0.2            | 300 s      |
| `silent-but-deadly` | 1.5 / hr  | ×2.0           | ±0.4            | 60 s       |

### 3.2 Micro: granular structure inside one fart

A fart is not one continuous note — it has internal phrasing. Acoustic studies of human flatulence show clear granularity (JASA 2021). Each event = a sequence of N **grains**, where each grain is a short BPF-filtered noise+saw burst with its own centre frequency, Q, amplitude, and fundamental. The `patter` axis controls grain count and density. See [`crates/fart-synth/src/grain.rs`](crates/fart-synth/src/grain.rs).

The elegance: **grain count and density are derived from the same `pressure` variable as the macro rhythm.** High pressure → more grains, longer, denser. Just-over-threshold pressure → fewer grains, sparser, shorter. The macro and micro share one variable — the result is that each fart's _shape_ encodes the history that produced it.

---

## 4. Sound: a seven-dimensional fart-space

Synthesis chain in [`crates/fart-synth/src/graph.rs`](crates/fart-synth/src/graph.rs):

```
pink noise + saw ─► BPF(centre, Q) ─► grain envelope
                                            │
                                            ▼
                                      sum over grains
                                            │
                                            ▼
                                tremor LFO ─► asymmetric tanh
                                            │
                                            ▼
                                comb-filter wetness
                                            │
                                            ▼
                              HPF (60 Hz) ─► LPF (2 kHz) ─► soft-limit ─► dBFS cap
```

| Parameter      | Range  | Meaning                                                                       |
| -------------- | ------ | ----------------------------------------------------------------------------- |
| **pressure**   | 0–1    | Master scaler for amplitude, duration, harmonic richness. From pressure model. |
| **wetness**    | 0–1    | BPF Q + comb-filter send. Low = dry/sharp, high = juicy/fleshy.                |
| **tightness**  | 0–1    | BPF bandwidth. High = focused whistle-like, low = broad noisy.                 |
| **patter**     | 0–1    | Grain density. 0 = sustained, 1 = staccato scatter.                            |
| **pitch_arc**  | −1…+1  | Filter-centre sweep direction. − = falling, + = rising, 0 = flat.              |
| **tremor**     | 0–1    | Amplitude LFO depth. Controls "shake" strength.                                |
| **crackle**    | 0–1    | Waveshaper drive. The "bubbling" texture.                                      |

Each fart is a single point sampled from this space. Sampling is a personality-conditioned Gaussian (see [`personalities.rs`](crates/fart-synth/src/personalities.rs)). Combined with mulberry32 PRNG + seed: every output reproducible, every output different.

**We deliberately do _not_ ship a named "sound library"** (squeak / trombone / etc.). That sort of enumeration freezes a continuous space into discrete boxes — the result is wooden. Keep the parameter space continuous; personality is a distribution; every fart is a fresh draw.

---

## 5. Architecture

```
                  ┌────────────────────────────────────────┐
                  │  crates/fart-synth (Rust)              │
                  │  pressure state machine                │
                  │  7D parameter sampler                  │
                  │  render(params, cfg) → Vec<f32>        │
                  └──────────────────┬─────────────────────┘
                                     │
        ┌────────────────────────────┼────────────────────────────┐
        │                            │                            │
        ▼                            ▼                            ▼
┌────────────────┐         ┌────────────────┐         ┌────────────────┐
│ apps/desktop   │         │ bin/fart       │         │ skills/fart    │
│ Tauri menubar  │         │ CLI via cpal   │         │ SKILL.md +     │
│ no dock icon   │         │ clap flags     │         │ bash wrapper   │
│ tray click =   │         │ --personality  │         │ around CLI     │
│ fart;          │         │ --seed         │         │                │
│ cmd-click =    │         │ --render <wav> │         │                │
│ settings       │         │ --print-state  │         │                │
└────────────────┘         └────────────────┘         └────────────────┘
```

Architectural commitments:

- **`fart-synth` is the single source of truth.** Synthesis happens in Rust via a plain sample-loop driving `cpal`. The Tauri webview is _just UI_ — it calls `invoke("fart_now", …)` and Rust does the work. No isomorphic TS Web Audio mirror.
- **State machine lives in Rust.** The `Pressure` struct ticks once per second in a background thread inside the Tauri shell. The webview reads/writes settings only; it does not drive timing.
- **Tray UX is OpenWhip-shaped.** Left-click fires a fart immediately. Right-click (or ⌘-click on a one-button mouse) opens the settings popover. The default action is _to do the thing_, not _to open dashboards_.
- **Three shells, all thin.** Tauri = window + tray + UI + invoke. CLI = parameter-parse + one call. Skill = bash wrapping CLI. Each shell well under 500 lines.

---

## 6. Hard constraints

These live as named constants in [`crates/fart-synth/src/safety.rs`](crates/fart-synth/src/safety.rs), with rationale comments. Tests in the same file verify ordering invariants. Changing values is a real change worth a CHANGELOG line.

```rust
pub const MAX_OUTPUT_DBFS: f32 = -6.0;       // speakers; protects driver Xmax
pub const HEADPHONE_DBFS: f32 = -18.0;       // tighter cap for ear-level output
pub const HPF_HZ: f32 = 60.0;                // prevents sub-fs excursion runaway
pub const LPF_HZ: f32 = 2_000.0;             // energy above this does not move the cone
pub const MAX_SESSION_MS: u32 = 30_000;
pub const MIN_COOLDOWN_MS: u32 = 60_000;
```

**Headphone detection** is a **user-facing toggle** in the popover (Speakers / Headphones, default Headphones). CoreAudio's transport-type API cannot distinguish Bluetooth headphones from Bluetooth speakers, and the failure mode is in the dangerous direction (under-attenuation). One UI toggle, one sentence in the README explains why.

---

## 7. Repo layout (as scaffolded)

```
flatus/
├── crates/
│   └── fart-synth/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs            # public API
│           ├── params.rs         # FartParams, the 7D point
│           ├── prng.rs           # mulberry32
│           ├── safety.rs         # hard constants + tests
│           ├── personalities.rs  # the four Gaussian distributions
│           ├── pressure.rs       # macro-rhythm state machine
│           ├── grain.rs          # granular envelope planner
│           ├── graph.rs          # the synthesis sample loop + biquad/pink noise
│           ├── wav.rs            # WAV writer + self-contained SHA-256
│           └── bin/fart.rs       # CLI (clap + cpal)
├── apps/
│   ├── desktop/                  # Tauri v2 shell
│   │   ├── package.json
│   │   ├── src/                  # webview UI: index.html + main.ts + style.css
│   │   └── src-tauri/
│   │       ├── Cargo.toml
│   │       ├── build.rs
│   │       ├── tauri.conf.json   # bundle.targets ["app"]; no DMG/signing yet
│   │       ├── capabilities/default.json
│   │       ├── icons/.gitkeep    # drop icon.png here
│   │       └── src/main.rs       # tray + invoke handlers + pressure background loop
│   └── web/                      # static landing page (apps/web/index.html)
├── skills/fart/                  # Claude Skill bundle
│   ├── SKILL.md
│   └── scripts/fart.sh
├── fixtures/golden/manifest.json # SHA-256 pinned WAVs; generated on first run
├── docs/
│   ├── ACOUSTICS.md
│   └── ENGINEERING.md
├── README.md
├── PLAN.md
├── CHANGELOG.md
├── LICENSE                       # Apache-2.0
├── .gitignore
└── Cargo.toml                    # workspace
```

---

## 8. Docs surface

Four files. Don't invent a fifth without a real reason.

| File              | Audience  | Voice                                                              |
| ----------------- | --------- | ------------------------------------------------------------------ |
| `README.md`       | public    | OpenWhip-tier. ~40 lines. Casual, joke roadmap, self-deprecating.  |
| `PLAN.md`         | internal  | Indicative mood, philosophy, milestones. Wittgenstein-key.         |
| `docs/ACOUSTICS.md` | both    | Citation-backed plausibility. The "we're actually serious" leg.    |
| `docs/ENGINEERING.md` | internal | Short. Conventions that actually constrain code.                 |

Cut: `THESIS.md`, `HARD-CONSTRAINTS.md`, `glossary.md`, `rfcs/`, `adrs/`. Hard constraints live as named code constants; their rationale is one-line comments. If a real architectural decision arises later, append a dated paragraph to this PLAN.md's history section.

---

## 9. Milestones

**v0.1 — ship the comedy** (current scaffold):

- [x] `crates/fart-synth` with `FartParams`, pressure state machine, grain envelope, four personalities
- [x] `bin/fart` plays via cpal; flags for `--personality`, `--seed`, `--render`, `--print-state`, `--headphones`, `--list-personalities`
- [x] `apps/desktop` Tauri menubar shell — Accessory mode, tray click = fart, right-click = settings popover (volume, personality, output, quiet hours)
- [x] **Unsigned `.app` only.** No DMG, no notarization, no Apple Developer ID required.
- [x] `apps/web/` static landing page (deploy-ready; subdomain TBD)
- [x] `fixtures/golden/manifest.json` placeholder
- [x] `skills/fart/SKILL.md` + bash wrapper
- [x] All four docs + LICENSE + CHANGELOG
- [ ] Drop a real `icons/icon.png` for the menubar
- [ ] Generate the four golden WAVs and pin their SHA-256s

**v0.2 — make it spreadable** (when we decide to broadcast):

- Signed `.app` + notarized `.dmg` via `tauri-action`
- Apple Developer ID setup (cert + App Store Connect API key)
- Optional Homebrew tap (`brew install flatus`)
- Universal binary (Apple Silicon + Intel)
- Real macOS `IOHIDIdleTime` polling to replace the `business_hours()` stub in `apps/desktop/src-tauri/src/main.rs`

**v0.3+ — stretch (not promised):**

- Web app with audio (synth re-implemented in TS for the browser, with an honest parity disclosure)
- Windows / Linux
- More personalities (one row each in `personalities.rs`)
- "Snitch log" mode for corporate IT

---

## 10. The two-tier voice (and why)

`README.md` is OpenWhip-style — short, casual, slightly self-deprecating, joke roadmap, no philosophy. The surface where the project introduces itself to a stranger on GitHub, and on that surface seriousness reads as overcompensation.

`PLAN.md` (this) and `docs/ACOUSTICS.md` are indicative-mood, Wittgenstein-key, citation-bearing. Where the project earns the right to the joke — by being correct about the physics and rigorous about its own limits.

The two voices reinforce each other. **The comedy is a side effect of taking the physics seriously, not the other way around.**

The org's lineage already has both registers — `wittgenstein` is austere, but the canon includes Hundred Rabbits and Pauline Oliveros, which are not. The two-tier voice is native to `[p → q]`, just spread across two files.

---

## 11. Open decisions

Two, both deferable:

1. **Apple Developer ID.** Required only at v0.2.
2. **Repo visibility.** Public from day 1 (your call; OpenWhip-style argues yes — the joke needs an audience).

---

## 12. What we borrow

Conceptual, lightly investigated:

- **OpenWhip** — voice template for the README, install-path template for the CLI, tray-UX template (click = do the thing).
- **Claude Code's master loop** — single loop, well-named events, no hidden orchestration. Our pressure tick has the same shape.
- **Hundred Rabbits, Ink & Switch, Folk Computer** — repo discipline. One README, code carries its own rationale.
- **Apple Watch water-eject** — the acoustic precedent (see `docs/ACOUSTICS.md`).
- **JASA "Physics of flatulence" (2021), Chirone 1988, NASA Yiin et al.** — the literature backing the plausibility argument.

---

## 13. What to do first, post-scaffold

1. `cargo build` in the workspace root — fix any first-run warts.
2. `cargo run --bin fart -- --personality default --print-state` to hear it.
3. Drop a real `icons/icon.png` into `apps/desktop/src-tauri/icons/` (or run `pnpm tauri icon path/to/source.png`).
4. `cd apps/desktop && pnpm install && pnpm tauri dev` to bring up the menubar shell.
5. Once it all hangs together, generate the four golden WAVs and pin their SHA-256s in `fixtures/golden/manifest.json`.

---

## 14. History

_(append-only log of meaningful design changes, dated; the substitute for an `adrs/` directory.)_

- **2026-05-13** — v0.1 scaffold landed. Rust-only synthesis (no isomorphic TS mirror). Unsigned `.app` ship strategy adopted from OpenWhip. Tray-click = fart confirmed as default UX.

---

_Q.E.D._
