# flatus v0.1.0

> _A small apparatus for moving air._

This is the first formal `flatus` release.

It keeps the recent product direction, but tightens the engineering edges
before we call it a real release: the desktop shell behavior is now documented
accurately, first-launch state actually completes, and the manual audio preview
has a clearer relationship to the website reference.

## Highlights

- **Formal release cut**
  `v0.1.0` replaces the earlier pre-release train in the app metadata, README,
  and website download links.
- **Desktop interaction is now honest**
  The menubar icon opens the native tray menu, and `Show window` opens the full
  desktop surface. Release docs now describe that exact shape.
- **Manual audio preview is easier to reason about**
  Desktop preview and manual playback mirror the website instrument's
  three-event preview structure and share the same default reference seeds.
- **First-launch flow is complete**
  The desktop window now includes a real onboarding completion path plus a way
  to re-open that help later.
- **Offline packaging is cleaner**
  The desktop window no longer depends on a remote font CDN at runtime.

## Verification

- `bash scripts/verify_audio_baseline.sh`
- `cargo test`
- `cargo check --workspace`
- `pnpm --dir apps/desktop tauri build`

## Known limits

- The macOS app and DMG are still unsigned.
- Public repository and bundle identifiers are unchanged in this release.
- Windows and Linux remain source-first / CLI-first paths rather than packaged desktop releases.
