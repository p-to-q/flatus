# Plugin and interaction research

> Status: direction-setting document for the post-cleanup phase.

## Summary

The next expansion should not start from "what else can make a noise?" It
should start from which additional behaviors still feel native to `flatus`
instead of turning it into a novelty soundboard.

This document defines two things:

1. the cadence envelope for `P` itself;
2. the likely shape of future extensions, including non-`P` sounds and
   lightweight motion interactions.

## Cadence model for `P`

### Target range

For a desktop companion that lives in the menubar, the acceptable event rate is
low enough that the app stays surprising, but not so low that it feels broken.

Recommended operating range:

- `polite-cough`: roughly 0.4 to 0.8 events per active hour
- `default`: roughly 0.8 to 1.3 events per active hour
- `biblical`: roughly 0.6 to 1.0 events per active hour
- `silent-but-deadly`: roughly 1.0 to 1.8 events per active hour

The current synth personalities already approximate this band. Future tuning
should stay inside it unless we intentionally introduce a separate mode.

### Single-event duration

Recommended duration band for the shipped personalities:

- short voice floor: `~0.35s`
- default voice center: `~1.1s – 1.8s`
- large voice ceiling: `~4.5s`

Why this range works:

- below the floor, the event stops reading as a body-like interruption;
- above the ceiling, the joke starts to overpower the product and the user
  loses trust in the app's self-restraint.

### Disturbance budget

The product should behave like a mild environmental interruption, not a demand
for attention. That implies:

- no burst clusters by default in the desktop app;
- no notification framing around an event;
- long refractory windows still matter as much as the sound design itself;
- quiet hours are a first-class product feature, not a hidden safety valve.

## Beyond `P`

### Other sounds

Additional sounds are acceptable only if they satisfy at least one of these:

- they reinforce the "inhabited machine" idea;
- they help onboarding, testing, or recovery;
- they open a new mode without making the core product incoherent.

Candidate categories:

- **Core-adjacent body sounds**
  Small creaks, chair-shifts, tiny sigh-like air motions. These are closest to
  the current concept and best suited to optional expansion packs.
- **Functional sounds**
  Preview blips, onboarding test tones, or calibration cues. These support the
  app directly and should stay quiet and short.
- **Decorative novelty sounds**
  Horns, alerts, or exaggerated cartoon effects. These should stay out of the
  core product.

Decision:

- keep the shipped core focused on `P`;
- allow future optional packs for adjacent body-like sounds;
- do not introduce unrelated novelty sounds into the main build.

### Motion and visual interactions

Small visual motion is appropriate when it clarifies state or gives the sound a
little physicality. It should not become a dashboard.

Good candidates:

- a brief waveform or pulse in the quick-controls popover during manual test
  playback;
- a low-key activity pulse in the main window when previewing personalities;
- onboarding motion that teaches where the app lives in the menubar.

Bad candidates:

- idle animations that keep moving when nothing is happening;
- mascot behavior or character UI;
- motion that competes with the acoustic event itself.

Decision:

- motion belongs in the desktop and web surfaces, not in the synthesis core;
- default idle state should stay nearly still.

## Plugin shape

This round should not ship installable plugins yet, but it should define the
extension boundary.

Recommended interpretation of "plugin":

- a **desktop extension point** for optional behavior packs;
- not a code marketplace;
- not a new way to override the synth engine itself;
- not a generic automation system.

### Supported future plugin categories

- **Voice packs**
  Additional curated personality sets built on the same synth core.
- **Behavior packs**
  Alternative cadence presets, quiet-hours defaults, or context-sensitive
  scheduling behavior.
- **Interaction experiments**
  Optional main-window or web-surface presentation layers that leave the core
  deterministic audio path alone.

### Explicit non-goals

- third-party arbitrary code execution inside the desktop app;
- plugin-defined unsafe audio output rules;
- plugin-defined networking in the runtime app.

### Minimum interface boundary

If plugin work begins later, the minimum extension surface should be:

- metadata: `id`, `name`, `version`, `kind`, `description`
- content: display copy, optional imagery, optional cadence presets
- validation: plugin data cannot alter hard safety constants
- runtime loading: disabled by default in the first implementation

## Core vs optional

The next shipping desktop release should treat these as **core**:

- better `P` baseline;
- better menubar interaction;
- onboarding, recovery, and display polish.

The next phase may treat these as **optional**:

- alternate voice packs;
- non-`P` body-adjacent sound packs;
- extra motion treatments in the main window or web preview.
