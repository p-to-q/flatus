---
name: fart
description: Plays a procedurally synthesized fart sound for comedic timing. Use when the user asks to "fart", wants comedic punctuation on a punchline, or is testing the flatus CLI itself.
allowed-tools: [Bash]
argument-hint: "[--personality <polite-cough|default|biblical|silent-but-deadly>] [--seed <int>] [--render <out.wav>]"
license: Apache-2.0
metadata:
  homepage: "https://github.com/p-to-q/flatus"
  version: "0.1.0"
---

# fart

Plays one fart through the local audio device by shelling out to the `flatus` CLI.

## Usage

```sh
bash scripts/fart.sh                                # one default fart
bash scripts/fart.sh --personality biblical         # slow, low, devastating
bash scripts/fart.sh --personality polite-cough     # short, dry, plausibly deniable
bash scripts/fart.sh --seed 42                      # reproducible
bash scripts/fart.sh --render out.wav               # no playback; write a WAV
```

If `fart` is not on `PATH`, the wrapper no-ops with a friendly note (so it never crashes a longer agent run).

## Parameter cookbook

| Flag                     | Effect                                                                          |
| ------------------------ | ------------------------------------------------------------------------------- |
| `--personality <name>`   | Pick a Gaussian distribution over the 7-D parameter space. Four are shipped.    |
| `--seed <u64>`           | Same seed → same fart. Useful when the agent wants a specific punchline twice.  |
| `--pressure 0..1`        | Manual pressure override (default 0.6). Higher = louder, longer, more harmonic. |
| `--render <path>`        | Don't play. Write a 16-bit mono WAV. Good for embedding into demo videos.       |
| `--headphones`           | Tighter output cap (−18 dBFS instead of −6).                                    |
| `--print-state`          | Dump the sampled `FartParams` to stderr (debugging).                            |
| `--list-personalities`   | Print the four personalities with their base-rate / refractory and exit.        |

## What this is not

Not a notification API. Not a voice. Not a generative model. Just one CLI call with optional flags. The Skill exists because comedic timing on a punchline is a real and underserved capability and `fart-synth` happens to render one in <200 ms.

See: [github.com/p-to-q/flatus](https://github.com/p-to-q/flatus), `docs/ACOUSTICS.md` for the physics.
