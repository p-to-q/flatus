//! WebAssembly bindings for the synth core.
//!
//! Compiled only for `wasm32-*` targets — the `wasm-bindgen` glue lives here and
//! is not pulled into native builds. The exported `render_wav` function is the
//! one bridge the browser needs: same inputs as the CLI, same bytes out.

use wasm_bindgen::prelude::*;

use crate::graph::{render, RenderConfig};
use crate::personalities::{lookup_personality, sample_params, PERSONALITIES};
use crate::prng::Mulberry32;
use crate::safety::{HEADPHONE_DBFS, MAX_OUTPUT_DBFS};
use crate::wav::write_wav_to_vec;

/// Install a console panic hook so Rust panics surface in the browser devtools.
/// Call once at page load.
#[wasm_bindgen(start)]
pub fn _init() {
    console_error_panic_hook::set_once();
}

/// Render one fart event to a 16-bit mono PCM WAV byte vector.
///
/// - `personality` — one of the canonical names (`polite-cough`, `default`,
///   `biblical`, `silent-but-deadly`). Unknown names return an empty `Vec` rather
///   than panicking, so the JS side can detect failure with `.length === 0`.
/// - `seed` — PRNG seed for reproducibility. Same `(personality, seed, pressure)`
///   always renders byte-identical output (subject to the same Rust toolchain).
/// - `pressure` — synthesis pressure in `[0, 1]`. Higher = longer + slightly lower.
/// - `headphones` — if `true`, applies the tighter −18 dBFS safety cap instead of
///   the speaker −6 dBFS default.
#[wasm_bindgen(js_name = renderWav)]
#[must_use]
pub fn render_wav(personality: &str, seed: u32, pressure: f32, headphones: bool) -> Vec<u8> {
    let Some(p) = lookup_personality(personality) else {
        return Vec::new();
    };

    let mut rng = Mulberry32::new(u64::from(seed));
    let pressure = pressure.clamp(0.0, 1.0);
    let params = sample_params(p, &mut rng, pressure);

    let cfg = RenderConfig {
        sample_rate_hz: crate::safety::SAMPLE_RATE_HZ,
        output_gain_dbfs: if headphones {
            HEADPHONE_DBFS
        } else {
            MAX_OUTPUT_DBFS
        },
    };

    let samples = render(&params, &cfg);
    write_wav_to_vec(&samples, cfg.sample_rate_hz)
}

/// Comma-separated list of personality names, in declaration order. Cheaper to
/// shuttle than a typed array across the wasm-bindgen boundary; JS splits on `,`.
#[wasm_bindgen(js_name = listPersonalities)]
#[must_use]
pub fn list_personalities() -> String {
    PERSONALITIES
        .iter()
        .map(|p| p.name)
        .collect::<Vec<_>>()
        .join(",")
}

/// Crate version — pinned to the workspace `Cargo.toml`. Surface it in the UI so
/// the page shows what synthesis core it's running.
#[wasm_bindgen(js_name = version)]
#[must_use]
pub fn version() -> String {
    crate::VERSION.to_string()
}
