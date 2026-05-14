# flatus v0.2.2

`flatus` is a small thing that lives in your menubar and occasionally farts.

It is also, by acoustic accident, tied to the same general speaker-clearing
story that makes the Apple Watch water-eject reference so hard to ignore.
This release is not about expanding that premise. It is about making the
desktop app feel more recoverable, more legible, more coherent, and easier
to keep shipping.

## Highlights

### Menubar recovery is much safer

The biggest product fix in this release is not glamorous, but it matters:
if the status-bar icon disappears, the app now has real recovery behavior.

- Reopening `flatus` from Applications or Spotlight brings back the main window.
- The app now reasserts or rebuilds the tray icon on resume, reopen, and
  main-window show instead of assuming the original menubar item survived.
- First-launch and in-window help copy now explain the recovery path plainly.

This closes one of the most dangerous user-facing dead ends in the desktop shell.

### Desktop audio now has a proper debug path

This release keeps a real `Export audio debug` path for engineering use,
without leaving it as a front-and-center product control. The export can be
triggered from the menubar menu and from a hidden window affordance. It writes:

- the exact WAV used by desktop manual playback
- a JSON report with personality, seed, pressure, and current output-device info

That gives us a reproducible way to investigate the remaining Apple speaker
listening question without guessing from memory or chat logs.

### Visual polish tightened again

The desktop window and public visuals continue the paper-and-ink direction,
but with cleaner execution:

- the paper grain is scaled up again so it reads as an intentional substrate
  instead of tiny background noise
- the hero banner wordmark and strapline are sharper and easier to read
- `biblical` was pushed further toward an edited, academic-looking figure
- website typography and section hierarchy are more internally consistent
- the desktop title row now sits more calmly against the `single / shuffle`
  control

## Product changes

- Desktop main-window support copy now includes the recovery path for a missing
  tray icon.
- `How to fart` aligns more cleanly with its adjacent play-mode control.
- The desktop window texture is enlarged for a softer, more tactile read.
- Banner text now prioritizes crisp legibility over atmospheric glow.
- Homepage subheads, captions, specimen descriptions, and install notes now
  speak in a more consistent typographic voice.

## Engineering changes

- Added tray recovery logic that:
  - reuses an existing tray when possible
  - forces it visible again
  - rebinds left-click menu behavior
  - rebuilds the tray if it is missing from app state
- Added desktop audio-debug bundle export from the Tauri layer.
- Added `docs/test/` as a place to keep release-relevant investigation notes
  that should survive beyond chat history.
- Regenerated public raster assets and DMG background outputs from the current
  SVG / scripted sources.

## Audio note

We also recorded the current state of the unresolved desktop audio question.

At the moment:

- exported desktop debug WAV playback appears to match desktop real-time
  playback closely enough that this is **not yet isolated** to the real-time
  output path alone
- the app is not currently known to perform deliberate app-level band splitting
- some of the “two-band” character may belong to the current timbre design
  itself, with Apple laptop speakers possibly exaggerating it further

That issue remains open, but it is no longer buried.

See:

- `docs/test/2026-05-14-desktop-audio-notes.md`
- `docs/AUDIO_BASELINE.md`

## Known limits

- macOS only
- Apple Silicon desktop build
- still unsigned
- the desktop audio character for the rougher personalities is under active
  evaluation, even though the rest of the desktop shell is ready to ship

## Assets

- `flatus_0.2.2_aarch64.dmg`
- `flatus-v0.2.2-aarch64.app.zip`
