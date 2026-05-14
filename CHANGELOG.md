# Changelog

## [v0.1.1] — 2026-05-14

### Fixed
- **DMG background assets now rebuild from the current SVG source all the way through to `@2x` and the bundled TIFF.** The visual pipeline now explicitly writes `apps/desktop/resources/dmg-background@2x.png` alongside the 1x PNG before assembling the multi-rep TIFF, so the packaged DMG matches the latest installer art.
- **README install copy is tighter.** The macOS install path, tray behavior, and current release links were shortened and pointed at the latest patch release.

### Changed
- **Website download entry points now target `v0.1.1`.** The homepage CTA and release URLs follow the latest formal patch release instead of the previous `v0.1.0` tag.

## [v0.1.0] — 2026-05-14

### Changed
- **Formal first release.** Versioning, release links, website CTA metadata, and desktop bundle metadata now point at `v0.1.0` instead of the previous pre-release tags.
- **Desktop shell semantics are now explicit.** The menubar icon opens the native tray menu; `Show window` opens the fuller desktop surface. README, web install copy, and engineering docs were updated to match the actual shipped behavior.
- **Manual desktop preview is now a coherent reference surface.** The desktop window uses the same three-event preview structure as the website instrument, keeps shuffle deterministic for the current seed, and resets to each personality's reference seed when you switch voices.

### Fixed
- **First-launch state can now complete.** The main window now has a real onboarding completion path instead of a persistent "first launch" state that reopened on every start.
- **Desktop no longer depends on a remote UI font at runtime.** The app window now uses local system UI fonts so the bundled `.app` remains visually stable offline.
- **Audio baseline docs now distinguish fixture parity from interactive preview parity.** This closes the gap where `verify_audio_baseline.sh` could pass while the desktop/manual listening reference still drifted from the website instrument.

## [v0.1.0-pre.3] — 2026-05-13

### Added (paper-aesthetic visuals + shippable DMG)
- **Unified paper aesthetic across every visual surface.** All supporting README/site visuals — `docs/screenshots/spectrogram-biblical.png`, `docs/screenshots/waveforms-all.png`, `docs/marks/{wordmark,signature,monogram,og-card}.png` — regenerated on the warm-paper canvas (`#f7f1e3` → `#efe7d2`) with oxblood data layers (`#8c2f1e`), a fingerprint paper-grain `feTurbulence` overlay, and softened `feGaussianBlur` bloom. Replaces the previous dark-canvas SVGs that didn't match the site's light palette.
- **Live site now serves the full visual set.** `apps/web/screenshots/` and `apps/web/marks/` mirror the rasterised PNGs so they're reachable on `flatus.vercel.app`; previously only `banner.png` and `og-card.png` were deployed.
- **Two new figures embedded on the homepage.** Waveform comparison sits between Instrument and Specimens; spectrogram of `biblical.wav` sits inside the Specifications block. New `.figure` CSS in `apps/web/style.css` (~20 lines) — paper mat, hairline frame, italic caption.
- **macOS app icon redrawn from the monogram.** `pnpm tauri icon` regenerated the full icon set (32/64/128/128@2x/Square*, icon.icns, iOS, Android) from `docs/marks/monogram.svg` — italic `f` + three oxblood grains on a rounded paper square. Replaces the placeholder black-circle-with-arrow that previously shipped in `icons/icon.png`.
- **Favicon unified to paper.** `apps/web/favicon.svg` now uses the paper canvas with a single bloomed oxblood grain and ink wisp, matching the homepage palette in both light and dark mode.
- **DMG background renders correctly on all displays.** Previously the 1080×760 PNG was treated as 1x by Finder and only the top-left 540×380 region showed (centred wordmark clipped at the right edge). Replaced with a multi-image TIFF carrying 1x (540×380 @ 72 DPI) + 2x (1080×760 @ 144 DPI); Finder picks the matching rep per display.
- **`scripts/render_all_visuals.sh` is the one-command regen.** Renders every SVG, rasterises via headless Chrome (preserves `feGaussianBlur` + `feTurbulence`), mirrors PNGs into `apps/web/`, and builds the retina TIFF via `tiffutil` + `tiffset`.
- **`.github/workflows/release.yml` cuts releases on tag pushes.** `pnpm tauri build` on `macos-latest` produces both the `.app` and the `.dmg`; `softprops/action-gh-release@v2` uploads both to the GitHub Release for the tag (auto-creating the release if absent).
- **README install section leads with the `.dmg`.** Drag-to-Applications path is the headline; the `.app.zip` fallback is documented for users who prefer to bypass the DMG step. Gatekeeper bypass instructions unchanged.

### Fixed
- `apps/desktop/src-tauri/icons/tray-template.png` is now 8-bit RGBA (was 8-bit gray+alpha, which `tauri::generate_context!()` rejected at compile time during release builds).

### Added (v0.3 web experience, also folded into this tag)
- `crates/fart-synth` now compiles to `wasm32-unknown-unknown` with a `wasm-bindgen` surface (`renderWav`, `listPersonalities`, `version`). Bundle: 62 KB wasm + 10 KB JS. Same Rust DSP the CLI runs — no parity drift.
- New `wav::write_wav_to_vec` builds a 16-bit mono PCM WAV in memory so the browser can obtain bytes without filesystem I/O. Shared core is `write_wav_into<W: Write>`; the path-taking `write_wav` is a thin wrapper.
- `cpal` is now a target-conditional dependency (native only); it was already only used by the `fart` binary.
- `apps/web/` is now a real interactive landing page rather than a brochure. Hero, Instrument (live WASM synth with personality buttons, pressure slider, seed input, headphones cap, waveform canvas, save .wav), Specimens (click to load + play), CLI block, Specifications ledger. Hand-written HTML + CSS + ES module + vendored wasm bundle. No build step.
- Visual language: warm paper (`#f7f1e3` / `#1a1612`), oxblood accent (`#8c2f1e`), Charter display + Inter UI + Berkeley/SF mono. prefers-color-scheme: dark supported.
- Download CTA detects platform/arch (UA + WebGL renderer heuristic), links to the latest release asset, and reveals an inline 4-step first-launch stepper on click.
- Live web prototype deployed at `https://flatus.vercel.app/` via `vercel.json` + `scripts/deploy-vercel.sh`. A redundant `.github/workflows/pages.yml` mirror also ships `apps/web/` to GitHub Pages on every push to main.
- `fart --demo <DIR>` renders all four personalities to a folder and prints a summary table; `fart --list-personalities` now interleaves one-line descriptions with the rhythm params.
- DMG installer: bundle.targets now produces `flatus_0.1.0_aarch64.dmg` with a custom 540×380 background (paper wordmark, `FLATUS` catalog mark, spec label, drag hint along the bottom).
- Template tray icon (`apps/desktop/src-tauri/icons/tray-template.png`) — black-alpha three-grain silhouette, tints to match menubar.
- `docs/banner.png` rasterised from the SVG so the README renders the `feGaussianBlur` bloom and `feTurbulence` paper grain that GitHub strips from inline SVG.

## [v0.1.0-pre.1] — 2026-05-13

Initial scaffold. The comedy ships first; signing comes later.

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
