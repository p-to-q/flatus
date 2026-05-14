# Product backlog

> Status: working record for the post-launch cleanup phase.
>
> Goal: finish the current shipped surface before the next round of feature
> expansion. This document records product, interaction, audio-quality, and
> maintenance issues that should be resolved before we treat the app as ready
> for broader distribution.

## Phase judgement

`flatus` is past prototype. The Rust synth core, CLI, Tauri menubar app, Web
Assembly preview, release artifacts, and visual site are all present and
buildable. `cargo test` and `cargo check --workspace` passed locally on
2026-05-14.

The remaining work is mostly product closure:

- make desktop settings real and durable;
- make menubar behavior understandable;
- reconcile local audio output with the web version;
- remove organization-specific branding from public-facing surfaces;
- improve first-run and recovery paths;
- align documentation with the current implementation.

## P0: Audio quality and parity

### Local audio sounds worse than the web version

The web audio is currently the preferred reference. The local desktop output
has been reported as unpleasant and significantly different from the online
version. We should not assume the latest local synthesis is better merely
because it is newer.

Observed concern:

- Web sample / web live synthesis sounds more reasonable.
- Local synthesis sounds strange and overly wet.
- It is unclear whether the desktop build is using a stale synth, a newer
  unreviewed synth, or a playback path that changes the result.

Relevant surfaces:

- `crates/fart-synth/src/graph.rs`
- `crates/fart-synth/src/wasm.rs`
- `crates/fart-synth/src/bin/fart.rs`
- `apps/desktop/src-tauri/src/main.rs`
- `apps/web/samples/v0.3/`
- `apps/web/samples/v0.4/`
- `fixtures/golden/`

Recommended path:

1. Establish an audio reference set.
   Use the current web samples as the reference until we intentionally choose
   otherwise.
2. Render the same `(personality, seed, pressure, headphones)` through CLI,
   desktop, and wasm.
3. Compare WAV bytes where possible, then compare playback paths separately.
4. Decide whether the target is v0.3, v0.4, or a tuned v0.5.
5. Add a short listen-test gate before changing goldens again.

Acceptance criteria:

- For the same synth inputs, CLI and wasm produce the same WAV bytes or the
  difference is explicitly documented.
- Desktop playback uses the same rendered samples as CLI, with only expected
  device resampling.
- The chosen reference samples are pinned and named in the README.
- Any synthesis change updates goldens, web samples, and screenshots together.

## P0: Desktop settings must be real

### Settings are memory-only

The app exposes settings, but they are not persisted across launches. This
contradicts the engineering note that settings should live in one
`settings.json`.

Relevant surfaces:

- `apps/desktop/src-tauri/src/main.rs`
- `apps/desktop/src/main.ts`
- `docs/ENGINEERING.md`

Acceptance criteria:

- Settings load from disk at startup.
- Settings save after changes.
- Corrupt settings recover to defaults without crashing.
- The settings path is documented.

### Volume slider does not affect playback

The UI has a volume slider and sends `volume` to Rust, but the render and
playback path currently only uses the output cap.

Relevant surfaces:

- `apps/desktop/src/index.html`
- `apps/desktop/src/main.ts`
- `apps/desktop/src-tauri/src/main.rs`

Acceptance criteria:

- `volume` scales output after the safety cap.
- `0%` is silent.
- `100%` preserves the existing cap.
- Manual "fart now" and automatic pressure-triggered playback behave the same.

### Quiet hours are not enforced

The UI collects quiet-hours values, but automatic playback does not currently
check them.

Acceptance criteria:

- Automatic pressure-triggered playback is suppressed during quiet hours.
- Manual "fart now" remains available, or explicitly asks for confirmation if
  we decide quiet hours should cover manual actions too.
- Overnight ranges such as `22 -> 7` work.
- Invalid values are clamped or rejected in the UI.

## P0: Menubar interaction

### Tray behavior is not self-explanatory enough

The app lives in the menu bar, but the current interaction depends on the user
learning left-click versus right-click behavior. We need a more discoverable
popover.

Desired behavior:

- Hover or click on the menubar icon should reveal a small window/popover.
- The popover should contain common controls and commands.
- The popover should include a "Show window" command that opens the fuller
  interface.
- The UI should feel intentional, not like a default utility panel.

Recommended popover contents:

- Fart now.
- Personality selector.
- Volume.
- Output cap: speakers / headphones.
- Quiet-hours status.
- Show window.
- Quit.

Relevant surfaces:

- `apps/desktop/src-tauri/src/main.rs`
- `apps/desktop/src/index.html`
- `apps/desktop/src/style.css`

Acceptance criteria:

- Primary action and settings are reachable from the menubar without guessing.
- Right-click menu still works as a fallback.
- The popover closes predictably when focus leaves.
- Keyboard and screen-reader labels are present for core actions.

## P1: Personality experience

### Personality UI is too standardized

Current personality content reads like a fixed catalog. The interface should
feel more alive, with varied lengths and randomized presentation where it helps.

Desired behavior:

- Personality descriptions can vary in length.
- The UI can surface a random personality or random copy variant.
- The randomization should remain tasteful and bounded.
- The canonical personality IDs should remain stable for reproducibility.

Relevant surfaces:

- `crates/fart-synth/src/personalities.rs`
- `crates/fart-synth/src/bin/fart.rs`
- `apps/web/main.js`
- `apps/desktop/src/main.ts`

Acceptance criteria:

- Personality IDs remain unchanged.
- Display copy is allowed to vary independently of synth behavior.
- Random display does not change deterministic render inputs unless the user
  explicitly asks for random audio.

## P1: Remove P2Q branding from public surfaces

### Public-facing text and images still contain P2Q / `[p -> q]`

The current product should stand on its own. Remove organization-specific
branding from screenshots, text, footers, the desktop popover, web navigation,
DMG background, and generated visual assets.

Known text/code locations:

- `README.md`
- `PLAN.md`
- `CHANGELOG.md`
- `docs/DEMO.md`
- `docs/ENGINEERING.md`
- `apps/desktop/src/index.html`
- `apps/web/index.html`
- `apps/web/main.js`
- `apps/web/install.sh`
- `apps/desktop/resources/dmg-background.svg`
- `apps/desktop/src-tauri/tauri.conf.json`

Known visual source locations:

- `docs/banner.svg`
- `apps/web/banner.svg`
- `docs/screenshots/waveforms-all.svg`
- `docs/screenshots/spectrogram-biblical.svg`
- `docs/marks/og-card.svg`

Important note:

Repository URLs and bundle identifiers may still need a deliberate migration
plan. Removing visible branding is easier than changing release hosting,
installer scripts, app identifiers, and GitHub links.

Acceptance criteria:

- No visible P2Q / `[p -> q]` branding in README screenshots, app UI, website,
  DMG artwork, or generated PNGs.
- Technical URLs and identifiers are either migrated or documented as deferred.
- Visual assets are regenerated from sources after SVG edits.

## P1: First launch and recovery

### First-run experience needs a real flow

The app currently expects users to understand a menubar-only app, unsigned
macOS launch friction, output safety, and quiet hours without much guidance.

Recommended first-run flow:

- Explain that the app lives in the menu bar.
- Ask for output mode: headphones / speakers.
- Set default volume.
- Offer quiet hours.
- Provide a test action.
- Explain how to quit and how to restore visibility.

Acceptance criteria:

- First launch opens an onboarding window.
- Onboarding can be dismissed and does not reappear after completion.
- Users can reopen onboarding/help later.

### Recovery after removing the menubar icon is unclear

Users may accidentally remove or hide the app from the menu bar and then not
know how to get it back. We need to verify the exact macOS behavior and design a
user-level recovery path.

Open questions:

- Can the app's icon be removed from the menu bar directly, or is the perceived
  issue caused by quitting/hiding the app?
- Does macOS Control Center / menu bar customization affect third-party tray
  icons in this case?
- Should the app provide a normal window, Dock fallback, or helper launcher for
  recovery?

Acceptance criteria:

- We document the actual macOS behavior.
- Users have a clear recovery path.
- First-run copy explains where the app lives and how to reopen it.

## P1: Hooks for future interaction upgrades

The next phase may add richer desktop and web interactions. We should leave
small extension points now so future work does not require tearing out the
current app shell.

Candidate hooks:

- A stable settings schema with versioning.
- A desktop command boundary for actions such as `show_main_window`,
  `play_preview`, `reset_onboarding`, and `export_sample`.
- A web/desktop shared personality display model separate from synth IDs.
- A documented audio reference manifest.

Acceptance criteria:

- New interaction features can be added without changing synth determinism.
- Settings migrations have a version field from the first persisted release.
- Desktop commands have explicit return errors for UI feedback.

## P2: Documentation drift

Several docs describe older states of the project. This is understandable after
a fast launch, but it will slow future management if left alone.

Known drift:

- `PLAN.md` still lists some completed work as pending.
- `docs/ENGINEERING.md` says the browser audio would be a reimplementation,
  while current wasm uses the Rust synth core.
- `docs/REALISM.md` begins as a prospective plan and later records shipped
  results.
- README status and release links should be checked after audio target is
  chosen.

Acceptance criteria:

- Public README states the current release status plainly.
- Internal docs distinguish shipped behavior from future plans.
- Audio-quality decisions are recorded in one place.

## Suggested order

1. Audio parity investigation.
2. Desktop settings persistence, volume, and quiet hours.
3. Menubar popover redesign.
4. First-run and recovery flow.
5. Public P2Q branding cleanup and asset regeneration.
6. Personality display variation.
7. Documentation alignment.

This order keeps the riskiest product promises first: users should hear the
right thing, control it reliably, and understand where the app went.
