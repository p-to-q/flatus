# `flatus/docs/ACOUSTICS.md`

> **Receipts, not claims.** The mechanism is real. The clinical efficacy claim is not. Both facts ship in the same file.

This document is the plausibility argument for `flatus`. It does two things: (1) lays out _why_ a particular waveform is a defensible engineering choice for moving air near a laptop microspeaker; (2) honestly bounds what is _not_ supported.

---

## 1. The Apple Watch precedent (Class A: documented, patented, filmed)

Apple Watch Series 2 (2016) shipped the "Water Eject" feature in watchOS. The mechanism is documented across a granted patent family:

- **US 9,451,354** — _Liquid expulsion from an orifice_ (Zadesky, Rothkopf, Fletcher, et al., 2016). The primary patent.
- **US 10,063,977** — continuation; reported to contemplate "a pulse of acoustic energy at a frequency that is less than 20 Hz or greater than 20,000 Hz." (We have not personally diffed this quote against the granted patent text — verify before quoting in print.)
- **US 10,595,107** — _Speaker module architecture_.
- **US 10,750,287** — _Evacuation of liquid from acoustic space_; references multiple frequencies played per ejection cycle.

The Slow Mo Guys filmed the mechanism at 2,000 fps and confirmed it visually ([YouTube](https://www.youtube.com/watch?v=EIEwy8rPik4), 2018): the diaphragm acts as a near-field pump, alternately tensing and relaxing so that water re-pooled during the relaxation phase is ejected on the next stroke.

**What Apple actually publishes:** "A series of tones plays to clear any water" ([apple.com/en-us/108352](https://support.apple.com/en-us/108352)). **Apple has not disclosed a frequency.** The widely-circulated "**165 Hz**" figure is community lore — traceable to a 2016 Reddit experiment, propagated by BGR / Yahoo / SEO-driven "fix my speaker" sites — and is _not_ confirmed by any reputable spectrogram analysis. It is a plausible order of magnitude for a Watch-sized driver, and that is the most that can honestly be said. We do not repeat the number as fact anywhere in this project; if it appears in conversation, it appears with this caveat attached.

---

## 2. Cone mechanics (Class A: textbook)

For any moving-coil loudspeaker:

| Quantity              | Definition                                                   | Implication for `flatus`                           |
| --------------------- | ------------------------------------------------------------ | -------------------------------------------------- |
| `fs`                  | Driver free-air resonance                                    | Maximum excursion per watt occurs here             |
| `Xmax`                | Peak linear cone excursion before non-linear distortion      | The hard ceiling we must stay under                |
| Excursion vs. f       | ∝ 1/f² below `fs` at constant SPL                            | Sub-resonance content damages the driver           |
| Excursion vs. f       | drops 12 dB/oct above `fs`                                   | High-frequency cleaning is acoustic theater        |

For laptop microspeakers, `fs` typically falls in the **150–400 Hz** band. Tight enclosures push it higher; small open-back drivers push it lower. The population spread is the reason `flatus` distributes energy across **80–400 Hz** rather than committing to a single tone — whatever the user's actual `fs` is, the broadband content crosses it.

---

## 3. Peer-reviewed evidence for sound-driven particle removal (Class A: real, but at SPLs we cannot reach)

- **Chirone, Massimilla & Russo (1988), _Powder Technology_** — 150 dB SPL at 120 Hz overcomes van der Waals forces between micron-scale powder agglomerates.
- **Yiin et al., NASA technical reports** — 128 dB at 13.8 kHz standing waves dislodge >2 μm particles from surfaces.
- **NASA NTRS 20110016660; Acta Astronautica S0094576524007677; _Nature Sci. Reports_ 2025 / s41598-025-86363-7** — lunar / Mars dust-mitigation work on solar panels. **Almost all use piezoelectric or structure-borne vibration coupled to the substrate**, not free-field airborne sound.
- **Semiconductor "megasonic cleaning"** — ~1 MHz in liquid bath, not air.
- **Gor'kov 1962; Settnes & Bruus 2012** — acoustic radiation force; typically requires MHz ultrasound at high SPL for microparticle manipulation.

**What the literature supports:** sound _can_ overcome adhesion forces; the SPL required is well beyond what a consumer laptop speaker produces at the listener's position (typical max ≈85–95 dB at 30 cm).

---

## 4. The consumer "speaker cleaner" app landscape (Class C: zero peer-reviewed validation)

Survey of public apps and sites (FixMySpeaker, fixphonespeaker.com, waterkick.net, onlinesound.net's Speaker Cleaner, Wave Clean iOS, PaoApps Speaker Cleaner, Google Play "Speaker Cleaner (dust, water)") shows:

- Vendors converge on **100–250 Hz tones** with occasional 0–80 Hz sweeps.
- Some include "ultrasonic" modes >10 kHz that are physically theatrical (cone barely moves at HF).
- **Zero peer-reviewed studies validate any of them.** Every efficacy number is self-reported / anecdotal.

This is the company `flatus` keeps. We acknowledge it honestly.

---

## 5. Fart acoustics (Class A: actually studied)

This matters because the audio output is, in fact, a fart. Acoustic analyses of human flatulence agree:

- **Peak fundamental ≈ 200–300 Hz**; one study reports the main peak at 258 Hz, with the third harmonic at 764 Hz and smaller odd-harmonic peaks at 1308 Hz and 1804 Hz (Flatology survey of 356 recorded events).
- **Generation mechanism:** vibration of the anal sphincter skin, behaving like the buzzing lips in a brass instrument's mouthpiece. Larger anatomy → lower fundamentals (the rectum acts as a closed-tube resonator).
- _The Journal of the Acoustical Society of America_ has published serious work on "physics of flatulence" (JASA 150(4) Supplement, A164, 2021).

The spectral profile of a human fart — **fundamental in 80–250 Hz, rich odd-and-even harmonics decaying through the low-mids, slow amplitude / pitch modulation, duration ~0.5–3 s** — is, by physical accident, almost exactly the spectrum we want for laptop-driver excursion work.

---

## 6. Why "fart-shaped" is, unironically, a defensible waveform

Hand an acoustics engineer this brief: _"Maximize cone excursion of a small laptop driver within Xmax, distribute energy so the same waveform works across the population of driver `fs` values, and avoid wasted energy below 30 Hz or above 2 kHz."_

They will return:

- Fundamental in the **80–250 Hz** band.
- Rich harmonics blanketing **300 Hz – 1 kHz** to catch whichever `fs` the specific driver has.
- A **slow amplitude envelope** (3–10 Hz modulation depth) so the steady-state SPL is converted into mechanical "shaking" rather than sustained pressure.
- A **1–3 s burst** with **0.5–1 s gaps** in repetition, so displaced matter can settle rather than be re-aspirated.
- Hard **high-pass at 60–80 Hz**.
- **Low-pass at ~2 kHz**.

That is a fart.

---

## 7. Plausibility classification

| Claim                                                                            | Class | Why                                                              |
| -------------------------------------------------------------------------------- | ----- | ---------------------------------------------------------------- |
| Apple Watch ejects water via diaphragm vibration                                 | **A** | Patented, documented, filmed at 2000 fps                         |
| Apple uses specifically 165 Hz                                                   | **C** | Community lore, never Apple-confirmed                            |
| Audio at sufficient SPL can dislodge dust from a surface                         | **A** | Chirone 1988, Yiin NASA — but needs ≥128 dB                      |
| A laptop speaker can shake loose lint _near its own grille_                      | **B** | Plausible mechanism (same as Watch), no rigorous validation      |
| A laptop speaker can dislodge adhered sub-50 μm dust                             | **C** | van der Waals dominates; airborne SPL from a laptop is too low   |
| Consumer "speaker cleaner" apps are clinically validated                         | **C** | Zero peer-reviewed studies exist                                 |
| Ultrasonic (>10 kHz) modes do useful mechanical work via a laptop driver         | **C** | Cone excursion negligible at HF                                  |
| A broadband 80–300 Hz modulated buzz is a _reasonable_ engineering choice        | **B** | Energy concentrated where the cone actually moves; not validated |

Class A = published & confirmed. Class B = mechanism plausible, no controlled study. Class C = lore / unsupported.

---

## 8. What `flatus` claims (and refuses to claim)

**Claims:**

- It vibrates a laptop microspeaker near its resonance, the way Apple's water-eject does on a Watch.
- The waveform is, by acoustic accident, both funny and well-shaped for that work.
- It is safe within the hard constraints in [`crates/fart-synth/src/safety.rs`](../crates/fart-synth/src/safety.rs): −6 dBFS cap (−18 dBFS on headphones), 60 Hz HPF, 2 kHz LPF, 30 s session ceiling, 60 s cooldown.

**Refuses to claim:**

- Clinical efficacy at dust removal.
- That it removes adhered fines (van der Waals owns that regime).
- That it is a substitute for compressed air or a Q-tip.
- Any health, medical, or device-warranty implication.

**The honest line:**

> _"Treat the cleaning as a **maybe** and the comedy as a **definitely**. Both are valid outcomes."_

---

## 9. Sources

- Apple patents: [US 9,451,354](https://patents.google.com/patent/US9451354), [US 10,063,977](https://patents.google.com/patent/US10063977), [US 10,595,107](https://patents.google.com/patent/US10595107), [US 10,750,287](https://patents.google.com/patent/US10750287)
- Apple Support: [Use Water Lock and eject water from your Apple Watch](https://support.apple.com/en-us/108352)
- The Slow Mo Guys: [How the Apple Watch Ejects Water in Slow Mo](https://www.youtube.com/watch?v=EIEwy8rPik4)
- Chirone, Massimilla, Russo — _Powder Technology_ (1988)
- JASA — [Physics of flatulence](https://pubs.aip.org/asa/jasa/article/150/4_Supplement/A164/563870/Physics-of-flatulence) (2021)
- ICEF Flatology — [Fart Overtones](https://flatology.com/fart-overtones/), [Harmonies of the World](https://flatology.com/harmonies-of-the-world/)
- `soundgen` R package — [`fart()` function](https://rdrr.io/cran/soundgen/man/fart.html) (academic synthesis reference)

---

_Q.E.D._
