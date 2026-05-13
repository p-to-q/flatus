//! The 7-dimensional fart space.
//!
//! Each fart is a single point in this space. Personalities are Gaussian distributions
//! over this space; the user does not see these axes directly (the popover shows
//! `volume`, `personality`, `base rate`, `quiet hours`, and not much else).
//!
//! All axes are bounded — sampling code clamps after each draw.

use crate::safety;

/// A single point in the 7-D fart space, plus the seed that produced it.
#[derive(Clone, Debug)]
pub struct FartParams {
    /// Master scaler for amplitude, duration, harmonic richness. Comes from the
    /// pressure state machine and is _not_ user-set per shot.
    pub pressure: f32,

    /// Bandpass Q and convolver send. Low = dry / sharp; high = juicy / fleshy.
    pub wetness: f32,

    /// Bandpass bandwidth. High = focused / whistle-like; low = broad / noisy.
    pub tightness: f32,

    /// Grain density. 0.0 = sustained drone; 1.0 = staccato sputter.
    pub patter: f32,

    /// Filter-centre sweep direction over the event. −1.0 = falling; +1.0 = rising.
    pub pitch_arc: f32,

    /// Amplitude LFO depth. Controls "shake" strength of the buzz.
    pub tremor: f32,

    /// High-frequency grain content. The bubbling texture.
    pub crackle: f32,

    /// PRNG seed. Same seed → same waveform.
    pub seed: u64,

    /// Filter centre frequency, derived from personality + pressure. Hz.
    pub centre_hz: f32,

    /// Event duration, derived from pressure + patter. Milliseconds.
    pub duration_ms: u32,
}

impl FartParams {
    /// Clamp every axis into its valid range. Cheap; call after any draw or edit.
    #[must_use]
    pub fn clamp(mut self) -> Self {
        self.pressure = self.pressure.clamp(0.0, 1.0);
        self.wetness = self.wetness.clamp(0.0, 1.0);
        self.tightness = self.tightness.clamp(0.0, 1.0);
        self.patter = self.patter.clamp(0.0, 1.0);
        self.pitch_arc = self.pitch_arc.clamp(-1.0, 1.0);
        self.tremor = self.tremor.clamp(0.0, 1.0);
        self.crackle = self.crackle.clamp(0.0, 1.0);
        // Centre frequency stays inside the band where the LPF/HPF actually let energy through.
        self.centre_hz = self
            .centre_hz
            .clamp(safety::HPF_HZ + 10.0, safety::LPF_HZ - 100.0);
        // Duration is bounded by MAX_SESSION_MS, but most farts are far below that.
        self.duration_ms = self.duration_ms.clamp(50, safety::MAX_SESSION_MS);
        self
    }
}

impl Default for FartParams {
    fn default() -> Self {
        Self {
            pressure: 0.5,
            wetness: 0.4,
            tightness: 0.5,
            patter: 0.3,
            pitch_arc: -0.2,
            tremor: 0.4,
            crackle: 0.2,
            seed: 0,
            centre_hz: 180.0,
            duration_ms: 600,
        }
    }
}
