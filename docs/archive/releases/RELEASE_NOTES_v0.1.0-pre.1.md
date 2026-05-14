# flatus v0.1.0-pre.1

> _A small apparatus for moving air._

First public pre-release. This is the **comedy-first** drop: the joke, the architecture, the receipts. No signed `.app`, no notarization, no `crates.io` publish — those come at v0.2, once we know anyone wanted this.

## What ships

- **`fart-synth`** — a Rust synthesis crate. 7-D parameter space, four personality-conditioned Gaussian distributions, mulberry32 seeded PRNG, RBJ biquad BPF/HPF/LPF, Paul Kellet pink noise, granular envelope, asymmetric tanh waveshaper, comb-filter wetness, dBFS-capped soft limiter.
- **`fart`** — a `clap` + `cpal` CLI. `fart --personality biblical`. `fart --render out.wav`. `fart --seed 42`.
- **`flatus-desktop`** — a Tauri v2 menubar shell. `ActivationPolicy::Accessory` + `LSUIElement=true` (no dock icon). Left-click the tray → fart now. Right-click → settings (volume, personality, output, quiet hours).
- **`apps/web`** — a static landing page (no audio yet).
- **`skills/fart`** — a Claude Skill that wraps the CLI.
- **`docs/ACOUSTICS.md`** — citation-backed plausibility writeup. Apple's patent family (US 9,451,354 et seq.), the JASA flatulence paper, Chirone 1988, an honest A/B/C plausibility table. The "we're actually serious" leg.
- **CI** — GitHub Actions matrix (macOS + Linux): `fmt --check`, `clippy -D warnings`, `test`, build, CLI smoke test. The macOS job also builds an unsigned `.app` and uploads it as an artifact.
- **Determinism contract** — `cargo run --bin generate-goldens` then `cargo test --test golden` enforces byte-identical re-rendering across builds. The `tests/plausibility.rs` companion test asserts in-band/above-band spectral energy ratio ≥ 6, backing the headline acoustic claim.

## Personalities (the bestiary)

| Name | Voice |
| --- | --- |
| `polite-cough` | short, dry, plausibly deniable |
| `default` | the canon |
| `biblical` | slow, low, devastating |
| `silent-but-deadly` | exactly what it says |

## Install (from source, this pre-release)

```sh
git clone https://github.com/p-to-q/flatus
cd flatus
chmod +x scripts/*.sh
scripts/doctor.sh        # confirm your machine has the prereqs
cargo install --path crates/fart-synth
fart --personality biblical
```

Menubar app (unsigned — right-click → Open the first time):

```sh
cd apps/desktop
pnpm install
pnpm tauri build
open src-tauri/target/release/bundle/macos/flatus.app
```

## Known limits in this pre-release

- 🔴 **Unsigned / unnotarized.** macOS Gatekeeper will block first launch. Right-click → Open. See [`SECURITY.md`](SECURITY.md).
- 🔴 **macOS Apple Silicon only.** Intel + universal binary in a follow-up.
- ⚠️ **Activity detection is stubbed.** `apps/desktop/src-tauri/src/main.rs::business_hours()` uses UTC wall-clock as a proxy. v0.2 will replace with `IOHIDIdleTime`.
- ⚠️ **The "165 Hz" Apple Watch number is community lore.** Apple has never published a frequency. We don't quote it as fact; see [`docs/ACOUSTICS.md`](docs/ACOUSTICS.md) §1.
- ⚠️ **No clinical efficacy.** The cleaning is a maybe. The comedy is a definitely. See [`docs/ACOUSTICS.md`](docs/ACOUSTICS.md) §8.
- ⚠️ **First `cargo build` on a clean machine may surface 1–2 small fixes.** The scaffold was authored without a local Rust compiler. If you hit something, [open an issue](https://github.com/p-to-q/flatus/issues/new/choose).

## Roadmap

- [x] First release
- [ ] Cease-and-desist from Apple's lawyers (re: US 9,451,354 et seq.)
- [ ] Speaker manufacturer warranty claims department
- [ ] IRB approval for the cleaning-efficacy study
- [ ] Bluetooth headphone hearing-protection litigation
- [ ] Notarized DMG (we'll get to it)
- [ ] Updated fart physics

## Acknowledgements

OpenWhip — voice template for the README, install-path template for the CLI, tray-UX template. Hundred Rabbits, Ink & Switch, Folk Computer — repo discipline. Apple Watch water-eject — the acoustic precedent.

## License

Apache-2.0.

A `[p → q]` project. We're interested in the arrow. _Q.E.D._
