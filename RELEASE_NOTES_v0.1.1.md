# flatus v0.1.1

> _A small apparatus for moving air._

This is a packaging and copy-tightening patch release.

It does not change the product direction. It fixes the DMG background pipeline
so the shipped installer uses the latest SVG source, and it tightens the public
install / release copy around the current desktop shape.

## Highlights

- **DMG background pipeline corrected**
  The installer assets now regenerate from the current `dmg-background.svg`
  through 1x PNG, 2x PNG, and bundled TIFF before packaging.
- **README install path tightened**
  The macOS install instructions are shorter and more direct.
- **Website release pointers advanced**
  The homepage download CTA and release URLs now point at `v0.1.1`.

## Verification

- `bash scripts/render_all_visuals.sh`
- `cargo test`
- `cargo check --workspace`
- `bash scripts/verify_audio_baseline.sh`
- `pnpm --dir apps/desktop tauri build`

## Known limits

- The macOS app and DMG are still unsigned.
- Windows and Linux remain source-first / CLI-first paths rather than packaged desktop releases.
