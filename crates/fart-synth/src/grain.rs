//! Granular-synthesis micro-rhythm.
//!
//! A single fart event is rendered as a sequence of `Grain`s. Each grain is a short
//! bandpass-filtered noise burst with its own centre frequency, Q, amplitude, and
//! fundamental for the saw component. The `patter` parameter controls grain count
//! and density: low patter → one long sustained grain; high patter → many short
//! staccato grains.
//!
//! `plan` returns the grain list; `graph::render` does the actual rendering.

use crate::params::FartParams;
use crate::prng::Mulberry32;
use crate::safety;

/// One grain in a fart event.
#[derive(Clone, Debug)]
pub struct Grain {
    /// Start position in the output buffer, in samples.
    pub start: usize,
    /// Length of the grain, in samples.
    pub length: usize,
    /// Bandpass centre frequency, in Hz.
    pub centre_hz: f32,
    /// Bandpass Q (1 = wide, 10 = narrow / whistle-like).
    pub q: f32,
    /// Peak amplitude (before final limiter / cap), in linear units.
    pub amp: f32,
    /// Sawtooth fundamental for the tonal component, in Hz.
    pub fundamental_hz: f32,
    /// Noise-to-saw mix, 0.0 = pure saw, 1.0 = pure noise.
    pub noise_mix: f32,
}

/// Plan the grain sequence for one event.
///
/// Determinism: identical `(params, n_samples, sample_rate_hz, rng_state)` → identical
/// `Vec<Grain>`.
pub fn plan(
    params: &FartParams,
    n_samples: usize,
    sample_rate_hz: f32,
    rng: &mut Mulberry32,
) -> Vec<Grain> {
    let _ = sample_rate_hz;
    let mut grains = Vec::new();

    // Grain count from patter. Low patter = one sustained drone; high patter = many
    // staccato pops.
    let count = if params.patter < 0.1 {
        1
    } else if params.patter < 0.5 {
        2 + (params.patter * 8.0) as usize
    } else {
        4 + (params.patter * 16.0) as usize
    }
    .max(1);

    let q_base = 2.0 + 8.0 * params.tightness;
    let n_samples_f = n_samples.max(1) as f32;

    for i in 0..count {
        let t = (i as f32 + 0.5) / count as f32; // 0..1, centred per grain

        // Grain length scales inversely with count and gets some jitter.
        let max_len = (n_samples_f / count as f32).max(1.0);
        let length_f = if count == 1 {
            n_samples_f
        } else {
            max_len * (0.6 + 0.5 * rng.next_f32())
        };
        let length = (length_f as usize).max(1);

        // Start position: evenly distributed across the buffer with a touch of jitter.
        let nominal_start = (t * n_samples_f) as usize;
        let jitter_range = (max_len * 0.15) as usize;
        let jitter = (rng.next_f32() * jitter_range as f32) as usize;
        let start = nominal_start
            .saturating_sub(jitter_range / 2)
            .saturating_add(jitter)
            .min(n_samples.saturating_sub(length).max(0));

        // Pitch arc — sweep the centre frequency across the whole event.
        let arc_offset = params.pitch_arc * 60.0 * (t - 0.5);
        let centre_jitter = rng.gauss(0.0, 8.0);
        let centre_hz = (params.centre_hz + arc_offset + centre_jitter)
            .clamp(safety::HPF_HZ + 10.0, safety::LPF_HZ - 100.0);

        let q = (q_base + rng.gauss(0.0, 1.0).abs()).clamp(0.5, 30.0);

        // Bell-curve over the grain sequence so the first and last grains are a touch
        // quieter than the middle ones.
        let position_envelope = if count == 1 {
            1.0
        } else {
            1.0 - (t * 2.0 - 1.0).abs() * 0.5
        };
        let amp = position_envelope * (0.6 + 0.4 * rng.next_f32()) * (0.4 + 0.6 * params.pressure);

        // Fundamental for the saw component: below the bandpass, so it gets harmonics
        // pushed up through the BPF passband.
        let fundamental_hz = (centre_hz * 0.5 + rng.gauss(0.0, 4.0)).max(40.0);

        // Wetter farts have more noise content; tighter / crackle-heavier ones lean
        // saw-ward.
        let noise_mix = (0.4 + 0.4 * params.wetness - 0.2 * params.crackle).clamp(0.0, 1.0);

        grains.push(Grain {
            start,
            length,
            centre_hz,
            q,
            amp,
            fundamental_hz,
            noise_mix,
        });
    }

    grains
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_grain_when_patter_low() {
        let mut p = FartParams::default();
        p.patter = 0.0;
        let grains = plan(&p, 48_000, 48_000.0, &mut Mulberry32::new(0));
        assert_eq!(grains.len(), 1);
    }

    #[test]
    fn many_grains_when_patter_high() {
        let mut p = FartParams::default();
        p.patter = 0.9;
        let grains = plan(&p, 48_000, 48_000.0, &mut Mulberry32::new(0));
        assert!(grains.len() >= 4);
    }

    #[test]
    fn all_grains_fit_inside_buffer() {
        let mut p = FartParams::default();
        for patter in [0.0, 0.3, 0.6, 0.9] {
            p.patter = patter;
            let n = 24_000;
            let grains = plan(&p, n, 48_000.0, &mut Mulberry32::new(7));
            for g in &grains {
                assert!(g.start + g.length <= n, "grain past end of buffer");
            }
        }
    }
}
