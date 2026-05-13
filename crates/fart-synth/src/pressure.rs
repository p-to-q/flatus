//! Macro-rhythm pressure state machine.
//!
//! Forget cron + jitter. This is the biological model — pressure accumulates over time
//! and faster while you're at the keyboard; once it crosses a noisy threshold, the
//! event fires and pressure drops to a small residual. A hard refractory period
//! prevents back-to-back fires faster than `MIN_COOLDOWN_MS`.
//!
//! Tick this on a timer (a few times per second is plenty; we only resolve to seconds).
//! The state is plain data, so it survives serialisation to `settings.json` if the
//! shell wants to checkpoint across restarts.

use crate::personalities::Personality;
use crate::prng::Mulberry32;
use crate::safety;

/// One bit of state about whether the user is currently active. The shell passes this
/// in on every tick. (`flatus` deliberately does not introspect the OS for activity;
/// the shell makes that judgement.)
#[derive(Clone, Copy, Debug, Default)]
pub struct ActivitySignal {
    pub user_is_active: bool,
}

/// What happens during a tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TickResult {
    /// Pressure built up; nothing fires.
    Building,
    /// In refractory; ignoring all signals.
    Refractory,
    /// The event fires. Render a fart with the supplied pressure value.
    Fart { pressure: f32 },
}

/// Pressure state machine. Owned by the desktop shell; ticked at ~1 Hz.
#[derive(Clone, Debug)]
pub struct Pressure {
    /// Current pressure value, ~0..1 typically (can briefly exceed 1).
    pub level: f32,
    /// Seconds remaining in the current refractory window.
    pub refractory_remaining_secs: f32,
    /// Personality currently driving the rhythm parameters.
    pub personality_name: String,
    /// Internal RNG for threshold noise and residual draws.
    pub rng: Mulberry32,
}

impl Pressure {
    #[must_use]
    pub fn new(personality: &Personality, seed: u64) -> Self {
        Self {
            level: 0.0,
            refractory_remaining_secs: 0.0,
            personality_name: personality.name.to_string(),
            rng: Mulberry32::new(seed),
        }
    }

    /// Advance the state by `dt_secs` (real wall-clock seconds since the last tick).
    /// Returns whether anything fired.
    pub fn tick(
        &mut self,
        dt_secs: f32,
        activity: ActivitySignal,
        personality: &Personality,
    ) -> TickResult {
        // Refractory wins everything.
        if self.refractory_remaining_secs > 0.0 {
            self.refractory_remaining_secs = (self.refractory_remaining_secs - dt_secs).max(0.0);
            return TickResult::Refractory;
        }

        // Pressure accumulates at base_rate per hour, ×activity_bonus when the user is
        // active. A tiny constant decay keeps idle pressure bounded.
        let base_per_sec = personality.base_rate_per_hour / 3600.0;
        let bonus = if activity.user_is_active {
            personality.activity_bonus
        } else {
            1.0
        };
        // Decay must stay well below `base_per_sec` (typically ~3e-4 for 1/hr personalities)
        // or pressure can never accumulate to threshold under active gain.
        let decay_per_sec = 0.0001;
        self.level += base_per_sec * bonus * dt_secs - decay_per_sec * dt_secs;
        self.level = self.level.max(0.0);

        // Noisy threshold. Threshold is centred at 1.0 with the personality's noise width.
        let threshold = 1.0 + self.rng.gauss(0.0, personality.threshold_noise);

        if self.level >= threshold {
            // Fire. Pressure carries a snapshot of how built-up things were.
            let pressure_snapshot = self.level.clamp(0.3, 1.5);
            // Drop to a small residual so back-to-back is possible but unusual.
            self.level = self.rng.uniform(0.1, 0.3);
            // Enter refractory — bounded below by MIN_COOLDOWN_MS in the safety module.
            let min_cooldown = safety::MIN_COOLDOWN_MS as f32 / 1000.0;
            self.refractory_remaining_secs = personality.refractory_secs.max(min_cooldown);
            return TickResult::Fart {
                pressure: (pressure_snapshot / 1.5).clamp(0.0, 1.0),
            };
        }

        TickResult::Building
    }

    /// Force an immediate fire — used by the tray "click = fart" path. Bypasses
    /// threshold but still enters refractory and snapshots whatever pressure was.
    pub fn force_fire(&mut self, personality: &Personality) -> TickResult {
        let pressure_snapshot = self.level.clamp(0.3, 1.5);
        self.level = self.rng.uniform(0.1, 0.3);
        let min_cooldown = safety::MIN_COOLDOWN_MS as f32 / 1000.0;
        self.refractory_remaining_secs = personality.refractory_secs.max(min_cooldown);
        TickResult::Fart {
            pressure: (pressure_snapshot / 1.5).clamp(0.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::personalities::PERSONALITIES;

    #[test]
    fn refractory_blocks_firing() {
        let p = &PERSONALITIES[1]; // default
        let mut state = Pressure::new(p, 7);
        state.refractory_remaining_secs = 30.0;
        let r = state.tick(
            1.0,
            ActivitySignal {
                user_is_active: true,
            },
            p,
        );
        assert_eq!(r, TickResult::Refractory);
    }

    #[test]
    fn pressure_builds_under_activity() {
        let p = &PERSONALITIES[1];
        let mut state = Pressure::new(p, 7);
        for _ in 0..60 {
            state.tick(
                60.0,
                ActivitySignal {
                    user_is_active: true,
                },
                p,
            );
        }
        assert!(
            state.level > 0.0
                || matches!(
                    state.tick(
                        1.0,
                        ActivitySignal {
                            user_is_active: true
                        },
                        p
                    ),
                    TickResult::Fart { .. }
                )
        );
    }

    #[test]
    fn force_fire_always_fires() {
        let p = &PERSONALITIES[1];
        let mut state = Pressure::new(p, 7);
        let r = state.force_fire(p);
        assert!(matches!(r, TickResult::Fart { .. }));
        assert!(state.refractory_remaining_secs > 0.0);
    }
}
