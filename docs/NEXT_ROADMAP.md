# Next roadmap

> Status: post-`v0.2.2` execution plan.
>
> Goal: stop treating `flatus` as a chain of local fixes and start advancing it
> through a small number of explicit product tracks.

## Baseline

`v0.2.2` is the current stable baseline.

That means:

- the desktop shell is releasable;
- the DMG, GitHub release, README, and website are aligned;
- tray recovery and onboarding recovery paths exist;
- the Vercel deployment path is working again.

The next phase should not start by expanding scope. It should start by
preserving this baseline and moving one track at a time.

## Global judgement

`flatus` is now in a transition state between launch cleanup and deliberate
product development.

The product no longer needs broad, unfocused polishing. It needs a tighter
sequence:

1. decide the desktop audio target;
2. upgrade the menubar interaction model;
3. define the next expansion surfaces only after those two foundations are in
   place.

## Track 1: Audio baseline decision

This is the highest-priority track.

The unresolved question is no longer “is desktop broken?” in the simple sense.
The stronger question is:

- do we want the desktop output to sound closer to the current web specimen;
- do we want it to sound closer to a smoother, more single-line “Apple Watch”
  speaker-clearing effect;
- or do we want to preserve the present layered / two-band character and tune
  it intentionally?

### Objectives

- choose one target listening character for desktop output;
- separate synth design questions from playback-path questions;
- keep `headphones` and `speakers` as distinct product targets if needed;
- freeze the next accepted reference set only after explicit listening review.

### Suggested execution

1. Use `v0.2.2` as the frozen starting point.
2. Compare `default`, `biblical`, and `silent-but-deadly` as the representative
   personalities.
3. Use exported debug WAVs, CLI renders, and desktop manual playback on the
   same machine.
4. Decide whether any smoothing belongs only to `speakers` mode.
5. If the baseline changes, update `goldens`, web specimens, and docs together.

### Exit criteria

- one documented desktop audio target;
- one accepted reference set;
- one explicit decision on whether speaker-mode compensation exists.

## Track 2: Menubar product shape

This is the second-priority track.

The current native menu is functional, but it is still a utility-shell
interaction, not the final product shape.

### Direction

Move from:

- native tray menu as the main experience

to:

- `Popover + main window` as the main desktop model,
- native menu retained as fallback and recovery.

### Popover target

The popover should hold the low-friction controls:

- `Fart now`
- personality
- volume
- output mode
- quiet-hours status
- `Show window`
- `Quit`

### Exit criteria

- the app feels menubar-native without asking the user to infer hidden behavior;
- common actions are reachable from the popover;
- the fuller window becomes setup / help / recovery, not the only serious UI.

## Track 3: Expansion planning

This track should not begin until Track 1 and Track 2 are materially clearer.

The point is not to build more things immediately. The point is to define what
the next surfaces actually are.

### Scope

- plugin form and boundaries
- non-`P` sounds or motion interactions
- personality copy / presentation expansion
- future desktop and web hooks

### Principle

Do not open expansion work by default.

Only promote work into implementation after the core desktop product and audio
identity are more stable.

## Recommended sequence

### Phase A

Freeze `v0.2.2` as the operational baseline and use it for all follow-up audio
tests.

### Phase B

Run an audio-focused pass and make the next audio decision explicit.

### Phase C

Build the popover model on top of the accepted audio baseline.

### Phase D

Open plugin / interaction expansion only after the first three phases settle.

## What not to do next

- do not mix audio redesign, visual redesign, and menubar architecture in the
  same pass;
- do not reopen broad release cleanup unless a concrete regression appears;
- do not treat every small refinement as a roadmap item.

The next phase should be track-driven, not drift-driven.
