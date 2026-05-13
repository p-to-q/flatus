//! The four personalities. Each is a Gaussian distribution over the 7-D fart space,
//! plus the rhythm parameters that drive the pressure state machine.
//!
//! Adding a personality = adding one row to this file. No registries, no plugins.
//! _The directory listing is the bestiary._

use crate::params::FartParams;
use crate::prng::Mulberry32;

/// `(mean, stddev)` for a single axis.
pub type Dist = (f32, f32);

/// A named personality. Holds both colour (the 7-D Gaussians) and rhythm.
#[derive(Clone, Debug)]
pub struct Personality {
    pub name: &'static str,

    // Colour — 7-D Gaussian over `FartParams`.
    pub wetness: Dist,
    pub tightness: Dist,
    pub patter: Dist,
    pub pitch_arc: Dist,
    pub tremor: Dist,
    pub crackle: Dist,

    /// Personality-typical filter centre, in Hz. Pressure modulates around this.
    pub centre_hz: Dist,

    /// Personality-typical duration, in ms. Pressure modulates around this.
    pub duration_ms: (u32, u32),

    // Rhythm — drives `Pressure`.
    /// Average events per hour at the base activity level.
    pub base_rate_per_hour: f32,
    /// Multiplier applied when the user is active (mouse/keyboard).
    pub activity_bonus: f32,
    /// `±` noise around the firing threshold. Larger = more biological irregularity.
    pub threshold_noise: f32,
    /// Hard cooldown after firing, in seconds.
    pub refractory_secs: f32,
}

/// The four canonical personalities. Adding a fifth is a one-row patch.
pub const PERSONALITIES: &[Personality] = &[
    Personality {
        name: "polite-cough",
        wetness: (0.1, 0.05),
        tightness: (0.6, 0.1),
        patter: (0.0, 0.05),
        pitch_arc: (0.0, 0.1),
        tremor: (0.1, 0.05),
        crackle: (0.05, 0.05),
        centre_hz: (220.0, 30.0),
        // Bumped from 80→320 ms. 80 ms was sub-perceptual — registered as a
        // click, not a "pop". 320 ms ± 90 ms lands in the audible-but-still-
        // tight range that matches the personality's name.
        duration_ms: (320, 90),
        base_rate_per_hour: 0.5,
        activity_bonus: 1.2,
        threshold_noise: 0.5,
        refractory_secs: 180.0,
    },
    Personality {
        name: "default",
        wetness: (0.4, 0.1),
        tightness: (0.5, 0.1),
        patter: (0.3, 0.15),
        pitch_arc: (-0.2, 0.2),
        tremor: (0.4, 0.1),
        crackle: (0.2, 0.1),
        centre_hz: (180.0, 30.0),
        // Bumped from 400→900 ms. The canonical voice needs enough length for
        // the patter / tremor / pitch-arc dimensions to actually develop.
        // 900 ± 250 puts most events in the 0.65-1.15 s band — meaty, with
        // audible internal structure, but well short of "long fart" territory.
        duration_ms: (900, 250),
        base_rate_per_hour: 1.0,
        activity_bonus: 1.5,
        threshold_noise: 0.3,
        refractory_secs: 90.0,
    },
    Personality {
        name: "biblical",
        wetness: (0.6, 0.1),
        tightness: (0.4, 0.1),
        patter: (0.2, 0.1),
        pitch_arc: (-0.4, 0.15),
        tremor: (0.5, 0.1),
        crackle: (0.3, 0.1),
        centre_hz: (110.0, 20.0),
        // Bumped from 1500→2400 ms. Biblical is the showpiece — it should
        // feel like an event, not a polite acknowledgement. 2.4 s ± 0.5 is the
        // "uh oh" range; rare enough (refractory 300 s) that it lands as
        // punctuation rather than wallpaper.
        duration_ms: (2400, 500),
        base_rate_per_hour: 1.33,
        activity_bonus: 1.3,
        threshold_noise: 0.2,
        refractory_secs: 300.0,
    },
    Personality {
        name: "silent-but-deadly",
        wetness: (0.3, 0.1),
        tightness: (0.5, 0.1),
        patter: (0.5, 0.15),
        pitch_arc: (-0.1, 0.2),
        tremor: (0.3, 0.1),
        crackle: (0.6, 0.15),
        centre_hz: (140.0, 25.0),
        // Bumped from 600→1100 ms. Crackle dimension needs space to oscillate;
        // a sustained crackle reads as "deniable but pungent", which is the
        // personality's whole bit.
        duration_ms: (1100, 300),
        base_rate_per_hour: 1.5,
        activity_bonus: 2.0,
        threshold_noise: 0.4,
        refractory_secs: 60.0,
    },
];

/// Look up a personality by name. Returns `None` for unknown names.
#[must_use]
pub fn lookup_personality(name: &str) -> Option<&'static Personality> {
    PERSONALITIES.iter().find(|p| p.name == name)
}

/// Draw a `FartParams` from a personality. `pressure` is supplied externally (it comes
/// from the macro-rhythm state machine, not from the personality).
///
/// Determinism: identical `(personality, rng_state, pressure)` produces identical params.
pub fn sample_params(p: &Personality, rng: &mut Mulberry32, pressure: f32) -> FartParams {
    // Sample colour axes from their respective Gaussians.
    let wetness = rng.gauss(p.wetness.0, p.wetness.1);
    let tightness = rng.gauss(p.tightness.0, p.tightness.1);
    let patter = rng.gauss(p.patter.0, p.patter.1);
    let pitch_arc = rng.gauss(p.pitch_arc.0, p.pitch_arc.1);
    let tremor = rng.gauss(p.tremor.0, p.tremor.1);
    let crackle = rng.gauss(p.crackle.0, p.crackle.1);

    // Pressure modulates centre frequency (more pressure = a touch lower) and duration
    // (more pressure = longer).
    let centre_jitter = rng.gauss(0.0, p.centre_hz.1);
    let centre_hz = p.centre_hz.0 + centre_jitter - 30.0 * (pressure - 0.5);

    let dur_jitter = rng.gauss(0.0, p.duration_ms.1 as f32);
    let duration_ms =
        (p.duration_ms.0 as f32 + dur_jitter + 400.0 * (pressure - 0.5)).max(50.0) as u32;

    // We hand a fresh seed downstream so the renderer's RNG state is independent of
    // the sampling RNG state — same `FartParams` → same waveform regardless of how the
    // params were drawn.
    let render_seed = u64::from(rng.next_u32());

    FartParams {
        pressure,
        wetness,
        tightness,
        patter,
        pitch_arc,
        tremor,
        crackle,
        seed: render_seed,
        centre_hz,
        duration_ms,
    }
    .clamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_personalities() {
        assert_eq!(PERSONALITIES.len(), 4);
    }

    #[test]
    fn names_unique() {
        let names: std::collections::HashSet<_> = PERSONALITIES.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), PERSONALITIES.len());
    }

    #[test]
    fn lookup_works() {
        assert!(lookup_personality("biblical").is_some());
        assert!(lookup_personality("definitely-not-a-personality").is_none());
    }

    #[test]
    fn sample_is_deterministic() {
        let p = lookup_personality("default").unwrap();
        let a = sample_params(p, &mut Mulberry32::new(42), 0.5);
        let b = sample_params(p, &mut Mulberry32::new(42), 0.5);
        // FartParams doesn't impl Eq because of f32, but field-by-field is fine.
        assert_eq!(a.seed, b.seed);
        assert_eq!(a.duration_ms, b.duration_ms);
        assert!((a.wetness - b.wetness).abs() < 1e-6);
        assert!((a.centre_hz - b.centre_hz).abs() < 1e-6);
    }
}
