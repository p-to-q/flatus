# flatus v0.2.0

> _A small apparatus for moving air._

First user-facing release of the desktop shell. If you tried v0.1.x and the
window felt frozen, the first-launch card refused to leave, or the tray menu
felt unresponsive — this is the release that fixes that. The synthesis core
(`fart-synth`, the CLI, the website instrument) is unchanged and still
byte-identical to the published golden fixtures.

## What changed for users

- **First launch behaves.** Click **looks good** once and the card is gone for
  good; it only comes back when you press **Show help again**.
- **`Fart now` is responsive.** From either the tray menu or the window, the
  click returns immediately; audio renders and plays on a background thread
  so the UI never blocks.
- **Audible previews.** Editing the seed, rolling a new seed, or switching
  personality now plays the same three-event session you'd get from `Fart
  now`, so you can audition voices without ever opening the menu.
- **No more "two voices" overlap.** Manual and background fires now share one
  output mutex, so they queue instead of mixing into a confused stack.
- **Larger, brighter preview waveform.** The in-window scope now matches the
  website instrument: same palette, same two-pass glow, fills the visible
  height even on quiet buffers.
- **Menubar icon fits the menubar.** The italic `f` plus three brand grains,
  drawn at the right size for a 22 px template image (the previous glyph was
  getting clipped at the top).

## Install

- macOS Apple Silicon DMG: download `flatus_0.2.0_aarch64.dmg` from this
  release and drag `flatus.app` into `/Applications`.
- One-liner installer (handles quarantine xattr too):

  ```sh
  curl -fsSL https://flatus.vercel.app/install.sh | bash
  ```

The `.app` is still **unsigned**. On macOS 15+ the first launch may need
`xattr -cr /Applications/flatus.app` once; see the README's _First launch_
section.

## Verification

- `cargo check --workspace`
- `cargo test`
- `pnpm --dir apps/desktop tauri build`

## Known limits

- macOS Apple Silicon only for the packaged app.
- Notarization is not yet in the pipeline.
- CLI on Linux/Windows still works from source (`cargo install --path
  crates/fart-synth`).
