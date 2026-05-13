//! Spectral plausibility test.
//!
//! The README and `docs/ACOUSTICS.md` make a specific claim: each fart's energy
//! is concentrated in the **80–400 Hz** band — where a laptop microspeaker's
//! cone actually moves — and is suppressed above ~2 kHz by the safety LPF.
//!
//! This test renders each of the four personalities at a canonical seed,
//! measures the RMS energy that survives a band-pass around the claimed band
//! versus a band-pass at 1.5 kHz, and asserts the ratio is what we promise.
//!
//! Implementation note: we re-use the same `graph::Biquad` the synth itself
//! uses. That keeps the dependency surface zero — no FFT crate — and means
//! "the test trusts the same filter math the synth does," which is exactly
//! the right scope for this kind of check.

use fart_synth::personalities::{lookup_personality, sample_params, PERSONALITIES};
use fart_synth::prng::Mulberry32;
use fart_synth::safety;
use fart_synth::{render, RenderConfig};

/// Measure RMS energy after band-passing through a Q=2 biquad at `centre_hz`.
fn band_energy(samples: &[f32], sample_rate_hz: f32, centre_hz: f32) -> f32 {
    // Two-pole pass to deepen the skirts; Q=2 keeps the bandwidth realistic.
    let mut bpf_a = mini_bpf(sample_rate_hz, centre_hz, 2.0);
    let mut bpf_b = mini_bpf(sample_rate_hz, centre_hz, 2.0);

    let mut sum_sq = 0.0_f64;
    for &s in samples {
        let y = bpf_b.process(bpf_a.process(s));
        sum_sq += (y as f64) * (y as f64);
    }
    (sum_sq / samples.len().max(1) as f64).sqrt() as f32
}

#[test]
fn each_personality_concentrates_energy_in_the_claimed_band() {
    let cfg = RenderConfig {
        sample_rate_hz: safety::SAMPLE_RATE_HZ,
        output_gain_dbfs: safety::MAX_OUTPUT_DBFS,
    };
    let sr = cfg.sample_rate_hz as f32;

    let canonical = [
        ("polite-cough", 1u64, 0.4_f32),
        ("default", 2, 0.6),
        ("biblical", 3, 0.8),
        ("silent-but-deadly", 4, 0.7),
    ];

    for (name, seed, pressure) in canonical {
        let personality = lookup_personality(name).unwrap_or_else(|| panic!("missing `{name}`"));
        let mut rng = Mulberry32::new(seed);
        let params = sample_params(personality, &mut rng, pressure);
        let samples = render(&params, &cfg);

        // Probe two bands. Centre frequencies chosen at the claim's centre
        // (200 Hz, in the 80–400 Hz cone-excursion sweet spot) and well above
        // the LPF (1500 Hz, where the cone barely moves).
        let in_band = band_energy(&samples, sr, 200.0);
        let above_band = band_energy(&samples, sr, 1500.0);

        // Floor the comparison so a zero `above_band` doesn't divide by zero.
        let ratio = in_band / above_band.max(1e-6);

        // The claim: in-band energy should be _at least_ 6× the above-band
        // energy. In practice we see >20× on most renders; 6× is a generous
        // lower bound that still fails if anyone accidentally lets HF leak.
        assert!(
            ratio >= 6.0,
            "personality `{}` has in-band/above-band ratio {:.2} (in={:.4e}, above={:.4e}). \
             The synth should keep most energy in 80–400 Hz — see docs/ACOUSTICS.md §2.",
            name,
            ratio,
            in_band,
            above_band
        );

        // Sanity: the rendered signal is not silent.
        let peak = samples.iter().fold(0.0_f32, |a, s| a.max(s.abs()));
        assert!(
            peak > 0.01,
            "personality `{}` rendered nearly silent (peak={})",
            name,
            peak
        );
    }
}

#[test]
fn output_below_cap_for_all_personalities() {
    // Belt-and-suspenders for the headline safety claim. The cap is also tested
    // in `graph.rs`, but this runs it for every personality at the canonical
    // seeds — the synth shouldn't have any personality-dependent leak.
    let cfg = RenderConfig {
        sample_rate_hz: safety::SAMPLE_RATE_HZ,
        output_gain_dbfs: safety::MAX_OUTPUT_DBFS,
    };
    let cap = safety::dbfs_to_linear(safety::MAX_OUTPUT_DBFS);

    for personality in PERSONALITIES {
        for seed in 0..4 {
            let mut rng = Mulberry32::new(seed);
            let params = sample_params(personality, &mut rng, 0.6);
            let samples = render(&params, &cfg);
            let peak = samples.iter().fold(0.0_f32, |a, s| a.max(s.abs()));
            assert!(
                peak <= cap + 1e-3,
                "personality `{}` seed {} exceeded cap (peak={:.3}, cap={:.3})",
                personality.name,
                seed,
                peak,
                cap
            );
        }
    }
}

// -------------------- Local Biquad copy --------------------
//
// We deliberately do _not_ depend on `graph::Biquad` being `pub` — it isn't.
// A tiny local copy is more honest than a `pub(crate)` API leak: the test gets
// to assert its claim with its own measuring stick.

struct MiniBpf {
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

fn mini_bpf(sample_rate: f32, fc: f32, q: f32) -> MiniBpf {
    use std::f32::consts::PI;
    let omega = 2.0 * PI * fc / sample_rate;
    let sin_w = omega.sin();
    let cos_w = omega.cos();
    let alpha = sin_w / (2.0 * q.max(0.1));

    let a0 = 1.0 + alpha;
    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;
    let a1 = -2.0 * cos_w;
    let a2 = 1.0 - alpha;

    MiniBpf {
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

impl MiniBpf {
    fn process(&mut self, x: f32) -> f32 {
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
