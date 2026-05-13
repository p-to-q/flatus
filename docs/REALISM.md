# Realism research — toward v0.4

> **Status**: research + plan. No synthesis code changes have shipped from
> this document yet. v0.4 will land the **chosen subset** below, listen-
> tested against the v0.3 archive, with before/after WAVs side-by-side.

## The premise

We listened to flatus v0.3 at [flatus.vercel.app](https://flatus.vercel.app/)
and it sounds — to a careful ear — *like a computer rendered it*. The audio
is band-correct, the duration is right, the personalities differ from each
other, but each event has a too-clean periodic signature underneath. A
real fart isn't that orderly.

That's the gap we're closing in v0.4: same architecture, same safety caps,
same WASM bundle size — but a synthesis path that fails the
*"is this generated"* listening test less often.

This document is **prospective**: it lays out what we learned about why
v0.3 sounds synthetic, what knobs would change it, and which knobs we'll
actually turn. It is **not** a writeup of finished work. The companion
[`ACOUSTICS.md`](ACOUSTICS.md) handles the "is the project plausible at
all" question — that one already shipped.

## What "real" sounds like (acoustic principles)

A real fart event, recorded clean, has these audible signatures:

1. **Aperiodic warble.** The pitch wobbles, but it doesn't *oscillate*.
   Biological airflow through a soft sphincter is a random walk modulated
   by lung pressure — the frequency drift looks like a low-pass-filtered
   noise process, not a sine wave.
2. **Bubble bursts.** Wet farts contain individual gas pockets collapsing
   — these read as short, high-frequency transients (Gaussian-ish
   envelopes, 8–20 ms, 400–800 Hz centre). They're sparse but each one is
   audible against the sustained tone.
3. **Body-cavity resonance.** The intestinal cavity is roughly a soft
   Helmholtz resonator. It amplifies a fundamental around 100–150 Hz and
   a second mode near 250–400 Hz — these aren't just bandpass-flat, they
   have peaks. (See ACOUSTICS §5 for the literature trail.)
4. **Brown-noise dominance.** Real fart spectra fall off faster than
   1/f (pink). The low end carries more energy than a pink-noise
   bandpass alone reproduces.
5. **Asymmetric tail.** Real events often have a 200–500 ms exponential
   decay after the main body — sometimes ending with a small final
   "snap" as residual pressure releases. v0.3's release is too clean.
6. **Saturation under load.** At peak amplitude, real airflow goes
   turbulent and the waveform tears — distortion that's nonlinear in a
   way our soft tanh limiter approximates but doesn't reproduce.

Cross-reference against current code:
[`crates/fart-synth/src/graph.rs`](../crates/fart-synth/src/graph.rs) lines
60–145 — the synthesis chain is grain rendering (bandpass + saw/pink mix) →
sine-LFO tremor → asymmetric tanh → 80 ms comb → safety HPF/LPF → soft
clip. Five of the six properties above have nothing in that chain.

## The six knobs

Each row is a parameter we could add or rework to close one of the gaps
above. The "LOC" column is the rough number of lines the change adds to
`graph.rs`/`grain.rs`/`personalities.rs` combined.

| # | What | Closes gap | LOC | Risk | Listenable gain |
|---|---|---|---|---|---|
| 1 | Aperiodic tremor (LFO → LPF random walk) | 1 | ~30 | low | **★★★** |
| 2 | Bubble-burst transients (Poisson pop layer) | 2 | ~50 | medium | **★★★** |
| 3 | Formant resonances (two narrow BPF stages) | 3 | ~40 | low | ★★ |
| 4 | Pink → brown mix on wetness | 4 | ~10 | very low | ★★ |
| 5 | Decay tail + optional final pop | 5 | ~30 | low | ★ |
| 6 | Saturation gating at peak amplitude | 6 | ~20 | low | ★ |

**Why these six and not others?** They're the changes where the
*acoustic principle is well-established* (citations in ACOUSTICS.md), the
*implementation is bounded* (one bullet, < 50 LOC), and they *compose*
(none of them require restructuring grain.rs or rewriting the safety
chain).

## v0.4 scope decision: pick three

The temptation when six knobs are on the table is to turn all of them at
once. That's a mistake — past a certain point we're not making it more
realistic, we're moving it into a different audio space entirely. The
user's words: *"我不希望它太怎么样"* (I don't want it to be too [much]).

**Shipping in v0.4 (this order):**

- **#1 — aperiodic tremor.** Highest single-change return on listenable
  realism. The current sine LFO is the most computer-flagged property of
  v0.3. Cost: ~30 LOC, no new dependencies.
- **#4 — pink/brown noise mix.** Cheapest possible change for visible
  spectral improvement. Real fart spectra are roughly brown at the low
  end; mixing 1/f² into the source raises the bottom register without
  touching anything else. ~10 LOC.
- **#2 — bubble bursts.** The "is this real" tell. Sparse Poisson-rate
  transient layer added on top of grain rendering — keeps wet/dry
  personalities distinct (silent-but-deadly gets dense crackle bursts,
  polite-cough gets almost none). ~50 LOC. This is the substantive one.

Total: ~90 LOC across the three files, all in well-bounded areas. Goldens
regenerate once at the end, not three times.

**Deferring to v0.5+:**

- **#3 — formants.** Worth doing, but the gain is subtler and the right
  Q values need ear-tuning over several iterations. Better as its own
  pass than bundled with v0.4.
- **#5 — decay tail.** Adds a new envelope shape; needs more design
  thought (how does it interact with `refractory_secs`? does the final
  pop need its own personality dimension?). v0.5 territory.
- **#6 — saturation gating.** A quality-of-life refinement once 1, 2, 4
  are settled. Until the source has more material to distort, gating
  doesn't have much to bite into.

## Implementation plan

Each knob lands as its own commit on a `feat/v0.4-realism` branch so the
A/B is preserved in git history:

1. **Archive v0.3 reference.** Copy current
   `fixtures/golden/*.wav` → `fixtures/golden/v0.3/*.wav` (read-only,
   tracked). Adds a paragraph to README explaining the comparison.
2. **Commit A — aperiodic tremor.** Replace the sine LFO in
   `graph.rs:94–103` with a low-pass-filtered (3 Hz cutoff) seeded
   random walk. Reuse the existing `Mulberry32`; no new RNG. Reuse the
   existing `tremor` personality axis to control depth. Add a
   `tremor_chaos` field to `FartParams` if the depth-only knob isn't
   enough — judged after the first listen.
3. **Commit B — brown noise mix.** Add an integrator-based brown-noise
   stage parallel to the existing pink. Mix ratio driven by the
   existing `wetness` axis: wet voices get more brown weight, dry
   voices stay pink-heavy. No new personality params.
4. **Commit C — bubble bursts.** New `pop_density` and `pop_strength`
   fields in `FartParams`. Defaulted via personality:
   silent-but-deadly gets high density, biblical gets sparse + loud,
   polite-cough gets almost none. Render as Gaussian-windowed sine
   bursts at random within-event positions (Poisson process seeded by
   the event's main RNG so deterministic).
5. **Regenerate goldens** + the WASM bundle + the data visualisations
   (waveforms-all, spectrogram-biblical). Update the manifest with
   the new SHA-256s.
6. **Listen test** locally before pushing — for each personality, play
   v0.3 archive and v0.4 fresh back-to-back at three pressures (0.3,
   0.6, 0.9). Note: this is the step where I'll know if any of the
   three changes overshot.
7. **Document.** Append a "Results" section to this file with the
   actual hashes, the listen verdict, and any tuning we had to do that
   diverged from the initial parameter guesses above.
8. **Deploy.** Push the branch, open the v0.4 PR, redeploy to
   flatus.vercel.app on merge.

## Test protocol (how we know it worked)

The realism gain is subjective by definition. We'll validate it three
ways:

1. **Listen panel.** Three people not on the project listen to a 30-second
   playlist alternating v0.3 and v0.4 events (same `(personality, seed,
   pressure)` per pair). Ask: *"which one sounds more like a real fart
   recording?"* Target: at least 2 of 3 prefer v0.4 in each personality.
2. **Spectrogram diff.** Render side-by-side spectrograms of one event
   per personality, v0.3 vs v0.4. The brown-noise mix should be visible
   as more energy below 150 Hz; the bubble bursts as discrete vertical
   spikes; the aperiodic tremor as gentler horizontal banding.
3. **Plausibility test invariants stay passing.** The existing
   `tests/plausibility.rs` checks that each personality concentrates
   energy in the claimed frequency band and stays under the dBFS cap.
   v0.4 must not break either invariant.

## What v0.4 will **not** do

- **Change the safety caps.** −6 / −18 dBFS, 60 Hz HPF, 2 kHz LPF, 30 s
  session ceiling, 60 s cooldown — all stay frozen. v0.4 is an audio
  quality change, not a behaviour change.
- **Introduce new dependencies.** No FFT crates, no resampling crates,
  no `rand` (we keep `Mulberry32`). The wasm bundle stays around 60 KB.
- **Restructure the architecture.** One synth core; CLI / menubar /
  WASM still all consume it. No "v0.4 audio mode" toggle — the new
  output is the new default.
- **Add an LLM, telemetry, or network call.** The frozen invariants
  from PLAN.md §6 stay frozen.

## References

- [`docs/ACOUSTICS.md`](ACOUSTICS.md) — peer-reviewed evidence trail.
  §5 (Fart acoustics) is the load-bearing one for this work.
- [`PLAN.md`](../PLAN.md) — project plan; v0.4 milestone is updated to
  reference this document.
- Granular synthesis reference: Roads, *Microsound* (MIT Press, 2001) —
  the grain-plan-then-render approach in `grain.rs::plan` mirrors his
  asynchronous granular cloud model.

---

*Authored as the v0.4 planning step, not as a post-hoc writeup. Updates
to this file once implementation lands will land in a "Results"
section appended below this line.*
