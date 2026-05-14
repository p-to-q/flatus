# Audio baseline

> Status: active release baseline.

## Current reference

Until a newer release is manually approved, the working reference is:

- `apps/web/samples/v0.4/*.wav`
- `fixtures/golden/*.wav`
- `apps/web/samples/v0.4/manifest.json`
- `fixtures/golden/manifest.json`

At the moment these files are byte-identical. That gives us a stable technical
fixture baseline for determinism and release regression.

## What this baseline means

For now, the release baseline has two layers:

- **Fixture reference**
  `fixtures/golden/*.wav` and `apps/web/samples/v0.4/*.wav` are the locked
  single-event regression fixtures.
- **Website specimen reference**
  The website specimen cards are the user-facing listening reference for the
  desktop app. They play one event, at fixed preview pressure `0.6`, using
  per-personality default seeds:
  - `polite-cough` -> `7`
  - `default` -> `17`
  - `biblical` -> `31`
  - `silent-but-deadly` -> `9`
- **Website instrument**
  The web instrument can still concatenate three events for exploration. That
  is a browser demo behavior, not the desktop manual playback target.
- CLI must still re-render the canonical fixture tuples to the same WAV bytes.
- Desktop playback must render from the same synth logic, even if the final
  device path still resamples through `cpal`.

This does **not** mean v0.4 is automatically the permanent product target. The
permanent target is chosen only after manual signoff.

## Verification flow

### Technical parity

Run:

```bash
bash scripts/verify_audio_baseline.sh
```

This verifies:

1. fixture WAVs and web sample WAVs hash to the same bytes;
2. fixture manifest and web manifest match;
3. the CLI reproduces the canonical fixture tuples from the manifest.

### Desktop parity

Desktop parity is checked in two layers:

1. **Render path parity**
   Desktop manual playback must call the same Rust render helper as the CLI
   and web specimen path, so differences cannot hide in duplicated parameter
   code. Automatic playback is an opt-in dynamic-pressure path and is not used
   as the website/specimen comparison target.
2. **Playback path parity**
   Any remaining listening difference is treated as a playback issue:
   device sample-rate conversion, output routing, cap mode, or volume scaling.

## Manual signoff checklist

Before freezing a new release baseline, compare the following in order:

1. Web sample playback for each canonical fixture voice.
2. CLI-rendered WAV playback for the same `(personality, seed, pressure)`.
3. Website specimen playback for the same selected voice and preview seed.
4. Desktop manual playback for the same selected voice, seed, and output mode.
5. Desktop automatic playback only after manual parity passes, with auto play
   explicitly enabled and quiet-hours / volume behavior verified separately.

Listen for:

- excessive wetness or crackle in desktop playback relative to the web sample;
- obvious duration drift in the canonical voices;
- output-cap differences that feel larger than the `speakers` / `headphones`
  design intends;
- device artifacts that are clearly not present in the rendered WAV file.

## Canonical fixture tuples

The current canonical tuples are:

- `polite-cough`: seed `1`, pressure `0.4`
- `default`: seed `2`, pressure `0.6`
- `biblical`: seed `3`, pressure `0.8`
- `silent-but-deadly`: seed `4`, pressure `0.7`

These come from `crates/fart-synth/src/bin/generate_goldens.rs`.

## Desktop manual reference

The website specimens and desktop manual playback share this release reference:

- `pressure = 0.6`
- default preview seeds:
  - `polite-cough` -> `7`
  - `default` -> `17`
  - `biblical` -> `31`
  - `silent-but-deadly` -> `9`

## When to change the baseline

Change the baseline only when all of the following are true:

- the desktop render/playback path has been audited;
- the CLI, fixtures, and web samples are technically aligned;
- a manual listening pass accepts the new sound;
- the golden fixtures, web samples, and release notes are updated together.
