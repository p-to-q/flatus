# Desktop audio notes — 2026-05-14

## Summary

This note records the current state of the unresolved desktop playback question
before the next release package is cut.

The user reports that some desktop voices can sound like two overlapping
bands or two partially independent lines on Apple hardware, while smoother
voices do not show the same effect as strongly.

## What we tested

- Added a desktop `Export audio debug` path that writes:
  - the exact WAV used by desktop manual playback
  - a JSON report with personality, seed, pressure, and output-device info
- Exported and inspected these runs:
  - `default` seed `17`
  - `polite-cough` seed `7`
  - `silent-but-deadly` arbitrary live seed `794568000`
- Separately rendered two reference WAVs directly from the CLI:
  - `biblical` seed `31`, pressure `0.6`
  - `silent-but-deadly` seed `9`, pressure `0.6`

## What we found

### Device path

The desktop app currently plays through:

- device name: `MacBook Pro扬声器`
- sample format: `F32`
- channels: `2`
- sample rate: `48000`

Desktop code renders mono, then duplicates that mono signal into every output
channel. There is no app-level evidence that we intentionally split high and
low bands into different speakers.

### Reference layers

There are two valid references in the repo:

1. **Canonical fixture / regression layer**
   - `fixtures/golden/*.wav`
   - `apps/web/samples/v0.4/*.wav`
   - example tuple: `default = seed 2, pressure 0.6`

2. **Desktop manual / website specimen layer**
   - fixed preview pressure `0.6`
   - per-personality preview seeds:
     - `polite-cough = 7`
     - `default = 17`
     - `biblical = 31`
     - `silent-but-deadly = 9`

Because of this, a desktop debug export for `default seed 17` should **not**
hash-match `fixtures/golden/default.wav`, and that mismatch is expected.

### Listening outcome

The user reported that the exported reference WAV playback sounds essentially
the same as desktop real-time playback for the tested cases.

That means the current issue is **not yet isolated** to the real-time desktop
playback path alone.

## Current interpretation

The strongest working interpretation is:

- the synth itself can produce multi-band or layered-feeling material for some
  personalities
- Apple laptop speakers may still accentuate that structure
- but the effect is not obviously created only by the desktop real-time output
  path, because exported WAV playback appears similar

This means the two-band quality may be:

- a real and intentional part of the current timbre family, or
- a synthesis choice that still needs curation, but not a simple cpal routing bug

## Release posture

This issue remains open, but it is **not** currently blocking the rest of the
desktop polish / recovery / packaging work for the next patch release.

## Recommended next steps

1. Export and compare `biblical` and `silent-but-deadly` on the same machine,
   using both desktop real-time playback and file playback.
2. If both paths still sound the same, treat this as a timbre-design question
   before treating it as an output-stack bug.
3. If a later pass decides the desktop speaker mode needs smoothing, apply that
   only to `speakers` mode and keep the underlying synth baseline intact.
