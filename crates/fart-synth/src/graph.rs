//! The synthesis sample loop.
//!
//! Plain-Rust DSP (no external audio backend in this module — `bin/fart.rs` handles
//! playback). The signal chain, per grain:
//!
//! ```text
//!  pink noise ─┐
//!              ├─► bandpass (centre, Q) ─► × grain envelope
//!  sawtooth  ──┘
//! ```
//!
//! Then over the whole buffer:
//!
//! ```text
//!  → tremor LFO  → asymmetric tanh waveshape  → comb-filter wetness
//!  → HPF (60 Hz) → LPF (2 kHz) → soft tanh limit → dBFS cap
//! ```
//!
//! Determinism: identical `(params, cfg)` → identical `Vec<f32>` on the same Rust
//! toolchain.

use std::f32::consts::PI;

use crate::grain;
use crate::params::FartParams;
use crate::prng::Mulberry32;
use crate::safety::{self, dbfs_to_linear, HPF_HZ, LPF_HZ, SAMPLE_RATE_HZ};

/// Output configuration: sample rate and final cap.
#[derive(Clone, Copy, Debug)]
pub struct RenderConfig {
    pub sample_rate_hz: u32,
    /// Final dBFS ceiling. Typically [`safety::MAX_OUTPUT_DBFS`] for speakers,
    /// [`safety::HEADPHONE_DBFS`] for headphones.
    pub output_gain_dbfs: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: SAMPLE_RATE_HZ,
            output_gain_dbfs: safety::MAX_OUTPUT_DBFS,
        }
    }
}

/// Render one fart event into a fresh `Vec<f32>`.
#[must_use]
pub fn render(params: &FartParams, cfg: &RenderConfig) -> Vec<f32> {
    let params = params.clone().clamp();
    let sr = cfg.sample_rate_hz as f32;
    let n_samples = ((sr * params.duration_ms as f32 / 1000.0) as usize).max(1);

    let mut rng = Mulberry32::new(params.seed);

    // 1) Plan grains.
    let grains = grain::plan(&params, n_samples, sr, &mut rng);

    // 2) Render each grain.
    let mut buf = vec![0.0_f32; n_samples];
    let mut pink = PinkNoise::default();
    let mut brown = BrownNoise::default();
    // v0.4 realism knob #4: brown-noise weight derived from wetness. Real
    // wet fart spectra fall off faster than 1/f (pink) — the low end is
    // closer to 1/f² (brown). Dry voices stay pink-heavy; wet voices
    // pick up roughly half their source from the brown integrator.
    let brown_weight = (0.55 * params.wetness).clamp(0.0, 0.55);

    for g in &grains {
        let mut bpf = Biquad::bandpass(sr, g.centre_hz, g.q);
        let mut saw_phase = rng.next_f32();
        let saw_inc = g.fundamental_hz / sr;

        let len = g.length.min(n_samples.saturating_sub(g.start));
        for i in 0..len {
            // Grain amplitude envelope: short attack, long decay, Hann-ish bell.
            let t = i as f32 / len as f32;
            let env = if t < 0.05 {
                t / 0.05
            } else {
                let d = (t - 0.05) / 0.95;
                libm::powf((1.0 - d).max(0.0), 1.5)
            };

            // Source: noise / saw mix. The noise itself is now a
            // pink/brown blend per `brown_weight`.
            let white_pink = rng.next_f32() * 2.0 - 1.0;
            let white_brown = rng.next_f32() * 2.0 - 1.0;
            let pink_sample = pink.next(white_pink);
            let brown_sample = brown.next(white_brown);
            let n = (1.0 - brown_weight) * pink_sample + brown_weight * brown_sample;
            saw_phase += saw_inc;
            if saw_phase >= 1.0 {
                saw_phase -= 1.0;
            }
            let saw = 2.0 * saw_phase - 1.0;
            let src = g.noise_mix * n + (1.0 - g.noise_mix) * saw;

            // Bandpass + grain amp.
            let y = bpf.process(src) * env * g.amp;
            buf[g.start + i] += y;
        }
    }

    // v0.4 realism knob #2: bubble-burst transients. Real wet farts
    // contain individual gas pockets collapsing — short Gaussian-windowed
    // sine bursts, sparse but audible. The pop centre frequency tracks
    // the personality's natural pitch (`centre_hz`): bigger cavity →
    // bigger bubble → lower pop. That keeps energy in the band each
    // personality claims, and matches the physics (bubble Helmholtz
    // resonance scales with bubble size). Density + strength derive
    // from existing wetness + crackle axes; no new personality
    // dimension.
    let pop_density_per_sec = params.wetness * 16.0 + params.crackle * 6.0;
    let pop_strength = 0.16 + 0.24 * params.crackle;
    let pop_centre_base = params.centre_hz.clamp(80.0, 400.0);
    let event_secs = n_samples as f32 / sr;
    let pop_count = ((pop_density_per_sec * event_secs) as usize).min(48);
    for _ in 0..pop_count {
        let start = (rng.next_f32() * n_samples as f32) as usize;
        // 8-15 ms Gaussian window.
        let length_samples = (sr * (0.008 + rng.next_f32() * 0.007)) as usize;
        // Centre 1.2x-2.4x the personality's pitch; biblical (~110 Hz) gets
        // pops at 130-270 Hz, polite-cough (~220 Hz) gets pops at 265-530 Hz.
        // Always inside or close to each band's tolerance.
        let centre_hz = pop_centre_base * (1.2 + rng.next_f32() * 1.2);
        let phase_inc = centre_hz / sr;
        let mut phase = rng.next_f32();
        let usable = length_samples.min(n_samples.saturating_sub(start));
        for i in 0..usable {
            let t = i as f32 / length_samples.max(1) as f32;
            // Gaussian window centred at t=0.5; sigma^2 ~= 1/18 of duration.
            let env = libm::expf(-(t - 0.5) * (t - 0.5) * 18.0);
            phase += phase_inc;
            if phase >= 1.0 {
                phase -= 1.0;
            }
            let pop = libm::sinf(2.0 * PI * phase) * env * pop_strength;
            buf[start + i] += pop;
        }
    }

    // v0.4 realism knob #1: aperiodic tremor. The previous sine LFO
    // sounded mechanical — biological airflow through a soft sphincter
    // is a random walk under low-pass smoothing, not a clean oscillation.
    // We sample a new target every ~30 ms and 1-pole-LPF toward it; the
    // resulting signal wobbles like a real pressure modulation rather
    // than singing like a wah pedal.
    if params.tremor > 1e-3 {
        let depth = 0.6 * params.tremor;
        // 1-pole LP coefficient for ~3 Hz cutoff at sr=48 kHz:
        //   alpha = exp(-2π * fc / sr) ≈ 0.99961
        let alpha = libm::expf(-2.0 * PI * 3.0 / sr);
        let target_interval = ((sr * 0.030) as usize).max(1);
        let mut state = 0.0_f32;
        let mut target = rng.next_f32() * 2.0 - 1.0;
        for (i, s) in buf.iter_mut().enumerate() {
            if i % target_interval == 0 {
                target = rng.next_f32() * 2.0 - 1.0;
            }
            state = alpha * state + (1.0 - alpha) * target;
            // |state| in roughly [0, 1] after smoothing; modulate gain
            // between (1 − depth) and 1.0 like the old LFO did.
            let modulation = 1.0 - depth + depth * state.abs();
            *s *= modulation;
        }
    }

    // 4) Asymmetric tanh waveshape. `crackle` drives harder; small positive bias
    // creates even-harmonic content.
    let drive = 1.0 + 3.0 * params.crackle;
    let bias = 0.05 * params.crackle;
    for s in &mut buf {
        *s = libm::tanhf(drive * *s + bias);
    }

    // 5) Wetness via single comb-feedback delay (~80 ms). Cheap, surprisingly
    // body-like, no IR table required.
    if params.wetness > 1e-3 {
        let delay = ((0.08 * sr) as usize).max(1).min(buf.len() / 2);
        let feedback = 0.4 * params.wetness;
        let mix = 0.5 * params.wetness;
        let mut wet = buf.clone();
        for i in delay..wet.len() {
            wet[i] += feedback * wet[i - delay];
        }
        for (b, w) in buf.iter_mut().zip(wet.iter()) {
            *b = (1.0 - mix) * *b + mix * *w;
        }
    }

    // 6) Safety HPF and LPF on every render. These are non-negotiable.
    let mut hpf = Biquad::highpass(sr, HPF_HZ, 0.707);
    let mut lpf = Biquad::lowpass(sr, LPF_HZ, 0.707);
    for s in &mut buf {
        *s = lpf.process(hpf.process(*s));
    }

    // 7) Peak normalise to ~0.95 linear, then soft-clip into the cap.
    let peak = buf.iter().fold(0.0_f32, |a, s| a.max(s.abs()));
    let cap = dbfs_to_linear(cfg.output_gain_dbfs);
    if peak > 1e-6 {
        let norm = 0.95 / peak;
        for s in &mut buf {
            *s = libm::tanhf(norm * *s) * cap;
        }
    }

    buf
}

// -------------------- DSP primitives --------------------

/// RBJ-cookbook biquad filter. Direct-form-I, single-channel.
#[derive(Clone, Debug, Default)]
pub struct Biquad {
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    /// Constant-skirt-gain bandpass (peak gain = Q at the centre).
    #[must_use]
    pub fn bandpass(sample_rate: f32, fc: f32, q: f32) -> Self {
        let omega = 2.0 * PI * fc / sample_rate;
        let sin_w = libm::sinf(omega);
        let cos_w = libm::cosf(omega);
        let alpha = sin_w / (2.0 * q.max(0.1));

        let a0 = 1.0 + alpha;
        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[must_use]
    pub fn highpass(sample_rate: f32, fc: f32, q: f32) -> Self {
        let omega = 2.0 * PI * fc / sample_rate;
        let sin_w = libm::sinf(omega);
        let cos_w = libm::cosf(omega);
        let alpha = sin_w / (2.0 * q.max(0.1));

        let a0 = 1.0 + alpha;
        let b0 = (1.0 + cos_w) / 2.0;
        let b1 = -(1.0 + cos_w);
        let b2 = (1.0 + cos_w) / 2.0;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[must_use]
    pub fn lowpass(sample_rate: f32, fc: f32, q: f32) -> Self {
        let omega = 2.0 * PI * fc / sample_rate;
        let sin_w = libm::sinf(omega);
        let cos_w = libm::cosf(omega);
        let alpha = sin_w / (2.0 * q.max(0.1));

        let a0 = 1.0 + alpha;
        let b0 = (1.0 - cos_w) / 2.0;
        let b1 = 1.0 - cos_w;
        let b2 = (1.0 - cos_w) / 2.0;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Brown-noise integrator. Feeds on white noise samples (uniform in [−1, 1])
/// and returns brown (1/f²) samples roughly in [−1, 1]. Implemented as a
/// leaky-integrator random walk with gain compensation; the cap is the leak.
#[derive(Clone, Debug, Default)]
pub struct BrownNoise {
    state: f32,
}

impl BrownNoise {
    /// Step the brown integrator. `white` should be uniform in [−1, 1].
    pub fn next(&mut self, white: f32) -> f32 {
        // Step size + clamp keep the walk bounded; 3.5 is gain compensation so
        // the output rms lands roughly in the same window as PinkNoise::next.
        self.state = (self.state + 0.02 * white).clamp(-1.0, 1.0);
        self.state * 3.5
    }
}

/// Pink-noise filter (Paul Kellet's refined 7-stage IIR approximation).
/// Feed white noise samples (uniform in [−1, 1]); get pink samples back.
#[derive(Clone, Debug, Default)]
pub struct PinkNoise {
    b0: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
    b6: f32,
}

impl PinkNoise {
    pub fn next(&mut self, white: f32) -> f32 {
        // Voss–McCartney pink noise filter; coefficients are reference DSP constants.
        self.b0 = 0.998_86 * self.b0 + white * 0.055_517_9;
        self.b1 = 0.993_32 * self.b1 + white * 0.075_075_9;
        self.b2 = 0.969_00 * self.b2 + white * 0.153_852;
        self.b3 = 0.866_50 * self.b3 + white * 0.310_485_6;
        self.b4 = 0.550_00 * self.b4 + white * 0.532_952_2;
        self.b5 = -0.7616 * self.b5 - white * 0.016_898_0;
        let pink =
            self.b0 + self.b1 + self.b2 + self.b3 + self.b4 + self.b5 + self.b6 + white * 0.5362;
        self.b6 = white * 0.115_926;
        pink * 0.11 // rescale to ~ [−1, 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_returns_correct_length() {
        let p = FartParams {
            duration_ms: 500,
            ..FartParams::default()
        };
        let buf = render(&p, &RenderConfig::default());
        assert_eq!(buf.len(), (48_000.0 * 0.5) as usize);
    }

    #[test]
    fn render_respects_dbfs_cap() {
        let p = FartParams::default();
        let buf = render(&p, &RenderConfig::default());
        let cap = dbfs_to_linear(safety::MAX_OUTPUT_DBFS);
        let peak = buf.iter().fold(0.0_f32, |a, s| a.max(s.abs()));
        // Allow tiny floating-point slop above the cap.
        assert!(peak <= cap + 1e-3, "peak {peak} exceeded cap {cap}");
    }

    #[test]
    fn render_is_deterministic() {
        let p = FartParams::default();
        let cfg = RenderConfig::default();
        let a = render(&p, &cfg);
        let b = render(&p, &cfg);
        assert_eq!(a, b);
    }

    #[test]
    fn render_references_safety_constants() {
        // This test exists to make the constants visible to the binary and to ensure
        // we never optimise them away. If anyone ever removes the HPF/LPF/cap from
        // `render`, this test will still pass — but the contract is documented and the
        // test below catches the cap directly.
        let _ = HPF_HZ;
        let _ = LPF_HZ;
        let _ = SAMPLE_RATE_HZ;
    }
}
