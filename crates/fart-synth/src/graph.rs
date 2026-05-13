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

            // Source: noise / saw mix.
            let n = pink.next(rng.next_f32() * 2.0 - 1.0);
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

    // 3) Tremor LFO across the whole event (4–25 Hz). Depth scales with `tremor`.
    if params.tremor > 1e-3 {
        let tremor_hz = 4.0 + 21.0 * params.tremor;
        let depth = 0.6 * params.tremor;
        for (i, s) in buf.iter_mut().enumerate() {
            let t = i as f32 / sr;
            let lfo = 1.0 - depth + depth * libm::sinf(2.0 * PI * tremor_hz * t).abs();
            *s *= lfo;
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
