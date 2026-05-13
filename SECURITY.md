# SECURITY.md

## What `flatus` does at runtime

- Reads / writes its own settings file in your OS config directory.
- Opens the default audio output device via `cpal` and writes 16-bit PCM into it.
- Runs a single background thread that ticks once a second, evaluates a pressure state machine, and occasionally renders an audio buffer.

That's it. **No network calls. No telemetry. No file access outside its own settings directory. No LLM at runtime. No code download or auto-update.**

## v0.1 is unsigned

The macOS `.app` ships **unsigned and unnotarized**. On first launch macOS Gatekeeper will say *"flatus cannot be opened because the developer cannot be verified."* This is expected.

To open it the first time:

1. Right-click (or ⌘-click) `flatus.app` in Finder.
2. Choose **Open**.
3. In the dialog, click **Open** again.

macOS will remember this choice; subsequent launches don't prompt.

Signed and notarized builds land at v0.2. Until then, if your security team needs a SHA-256 of every released artifact, the GitHub Release page lists them and they match what's in `CHANGELOG.md`.

## Reporting a vulnerability

If you find a real security issue (not Gatekeeper warnings; those are expected), email `hi@ptoq.io`. We'll confirm receipt within a week.

## What we won't do

- Auto-update.
- Phone home with usage data.
- Make any health, medical, or device-warranty claim. See [`docs/ACOUSTICS.md`](docs/ACOUSTICS.md) §8.
