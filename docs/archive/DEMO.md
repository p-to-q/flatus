# `flatus/docs/archive/DEMO.md`

> A 90-second demo script for the first recording. Internal. _Read it once,
> rehearse once, then record._

The demo's job is **not** to explain. It's to make the viewer hear the joke
and immediately afterwards realise the joke is acoustically defensible. Two
beats, in that order.

---

## Setup (off-camera)

- macOS, terminal open at `~/code/flatus`.
- `flatus.app` already in `Applications` (or open it via Finder once so
  Gatekeeper is past).
- Sound on at a normal level. Use **speakers**, not headphones — the joke
  needs the small-driver context.
- Recording: QuickTime screen recording + the laptop's built-in mic for
  ambient sound. We want the actual sound of the laptop speaker, not the
  digital signal — that's the whole point.

---

## Beat 1 — the joke (≈ 25 s)

Open the menubar. Click the `flatus` icon. **The laptop farts.**

```text
[screen: clean Finder, top-right of menubar visible]
[click]  →  [a fart]
[no narration]
```

That's the whole first beat. Resist the urge to caption it. Resist the
urge to play it twice.

---

## Beat 2 — the receipt (≈ 60 s)

Switch to the terminal. Open the project, ideally at `docs/banner.svg` and
`docs/ACOUSTICS.md` side-by-side, or read them aloud calmly.

Optional voice-over, three lines, no jokes:

> _"It's a fart."_
>
> _"It's also the same kind of waveform Apple uses in watchOS to push water
> out of the Apple Watch speaker — broadband, 80 to 400 Hz, amplitude-
> modulated, capped well below the driver's excursion limit."_
>
> _"Apple patented it. We synthesise it in 300 lines of Rust."_

Cut to the banner SVG fullscreen for the last sentence. Don't explain the
HPF and LPF lines on the figure — let them be there.

---

## Beat 3 — the punchline (≈ 5 s)

Pull up `README.md`. Scroll to the roadmap. Hold on:

```
[ ] Cease-and-desist from Apple's lawyers (re: US 9,451,354 et seq.)
```

Cut.

---

## What this demo is not

- Not a feature tour. We have no features besides "occasionally farts."
- Not a comparison. OpenWhip is its own thing; don't mention it on camera.
- Not a tutorial. The repo's CONTRIBUTING.md does that better and quieter.
- Not a pitch. Nothing to pitch.

---

## Title and copy

Working title for the recording, in order of preference:

1. **flatus**
2. **flatus — a small apparatus for moving air**
3. **a fart that thinks it's a thesis** _(only if we feel mischievous)_

Description for whatever platform it lands on (kept under 280 chars):

> A desktop companion that occasionally farts. The waveform is the same
> kind Apple uses to eject water from the Apple Watch (US 9,451,354). The
> joke is real. The physics is real. Apache-2.0.

---

_Q.E.D._
