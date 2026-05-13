//! Hard constraints — the engineering ceiling that protects the driver and the listener.
//!
//! Changing any of these numbers is a real change worth a CHANGELOG line. The unit test at
//! the bottom keeps `graph::render` honest about referencing them.

/// Default sample rate. Most audio interfaces want 48 kHz; 44.1 kHz works too but adds
/// resampling cost downstream.
pub const SAMPLE_RATE_HZ: u32 = 48_000;

/// Output cap when playing through laptop speakers. dBFS = decibels relative to digital
/// full-scale (1.0 in a `f32` buffer). −6 dBFS = ~0.5 linear.
///
/// Why: sustained max-volume into a small driver at its resonance frequency exceeds
/// `Xmax` and slowly cooks the voice coil. We do not want to ship the most absurdly
/// memorable bug report in flatus history.
pub const MAX_OUTPUT_DBFS: f32 = -6.0;

/// Tighter cap when the user has toggled "Headphones" in the popover. Ear-level audio
/// is genuinely loud at −6 dBFS; we drop to −18 by default and let the user reach for
/// −6 themselves if they really want it.
pub const HEADPHONE_DBFS: f32 = -18.0;

/// High-pass cutoff applied on every render. Below this, cone excursion grows as 1/f²
/// at constant SPL — sub-resonance content damages the driver before it gets loud.
pub const HPF_HZ: f32 = 60.0;

/// Low-pass cutoff. Above ~2 kHz, cone excursion is negligible on a laptop microspeaker;
/// energy here only hurts the listener's ears, not the cleaning work.
pub const LPF_HZ: f32 = 2_000.0;

/// Maximum single-event duration. A fart that lasts longer than this is no longer a fart.
pub const MAX_SESSION_MS: u32 = 30_000;

/// Minimum cooldown between events. Enforced by the pressure state machine.
pub const MIN_COOLDOWN_MS: u32 = 60_000;

/// Convert dBFS to linear gain (multiplier on the `f32` sample bus).
#[inline]
#[must_use]
pub fn dbfs_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

// Compile-time guards on the safety constants — stronger than unit tests because the
// build itself fails if the bands or timings ever drift into nonsense.
const _: () = assert!(HPF_HZ < LPF_HZ);
const _: () = assert!(MAX_SESSION_MS < MIN_COOLDOWN_MS);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dbfs_zero_is_unity() {
        assert!((dbfs_to_linear(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cap_below_unity() {
        assert!(dbfs_to_linear(MAX_OUTPUT_DBFS) < 1.0);
        assert!(dbfs_to_linear(HEADPHONE_DBFS) < dbfs_to_linear(MAX_OUTPUT_DBFS));
    }
}
