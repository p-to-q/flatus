//! `fart-synth` — synthesis core for `flatus`.
//!
//! Public API:
//! - [`FartParams`] — a single point in the 7-dimensional fart space.
//! - [`Personality`] — a named Gaussian distribution over that space, plus rhythm parameters.
//! - [`PERSONALITIES`] — the four personalities (`polite-cough`, `default`, `biblical`, `silent-but-deadly`).
//! - [`render`] — `(FartParams, RenderConfig) -> Vec<f32>`. Pure, deterministic, single-channel PCM.
//! - [`Pressure`] — the macro-rhythm state machine. Tick it on a timer; it tells you when to fart.
//! - [`Mulberry32`] — seedable PRNG. No global RNG anywhere in this crate.
//! - [`wav::write_wav`] — write a 16-bit mono WAV from a sample buffer.
//!
//! Determinism: same `(FartParams, RenderConfig)` produces the same `Vec<f32>` on the same toolchain.

pub mod grain;
pub mod graph;
pub mod params;
pub mod personalities;
pub mod pressure;
pub mod prng;
pub mod safety;
pub mod wav;

pub use graph::{render, RenderConfig};
pub use params::FartParams;
pub use personalities::{lookup_personality, sample_params, Personality, PERSONALITIES};
pub use pressure::{ActivitySignal, Pressure, TickResult};
pub use prng::Mulberry32;
pub use safety::SAMPLE_RATE_HZ;

/// Crate version, mirrors the workspace `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
