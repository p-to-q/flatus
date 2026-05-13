//! `generate-goldens` — render the four canonical fixtures and write the manifest.
//!
//! Run from the repo root so the relative paths resolve:
//!
//! ```sh
//! cargo run --bin generate-goldens
//! ```
//!
//! Output:
//!
//! - `fixtures/golden/polite-cough.wav`
//! - `fixtures/golden/default.wav`
//! - `fixtures/golden/biblical.wav`
//! - `fixtures/golden/silent-but-deadly.wav`
//! - `fixtures/golden/manifest.json` (with SHA-256s pinned)
//!
//! Re-pin on any intentional change to synthesis. The `tests/golden.rs`
//! integration test then enforces that future builds match these hashes.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use fart_synth::personalities::{lookup_personality, sample_params};
use fart_synth::prng::Mulberry32;
use fart_synth::safety;
use fart_synth::wav::{sha256_hex, write_wav};
use fart_synth::{render, RenderConfig};

/// One fixture in the manifest. `pressure` is the override given to
/// `sample_params`; `seed` seeds the personality sampler and downstream synth.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Fixture {
    personality: String,
    seed: u64,
    pressure: f32,
    file: String,
    sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    comment: String,
    sample_rate_hz: u32,
    output_gain_dbfs: f32,
    channels: u8,
    bits_per_sample: u8,
    fixtures: Vec<Fixture>,
}

/// The four canonical fixtures. Edit here to add a fifth (mirroring a fifth
/// personality in `personalities.rs`).
const CANON: &[(&str, u64, f32)] = &[
    ("polite-cough", 1, 0.4),
    ("default", 2, 0.6),
    ("biblical", 3, 0.8),
    ("silent-but-deadly", 4, 0.7),
];

fn main() -> Result<()> {
    let golden_dir = PathBuf::from("fixtures/golden");
    fs::create_dir_all(&golden_dir)
        .with_context(|| format!("creating {}", golden_dir.display()))?;

    let cfg = RenderConfig {
        sample_rate_hz: safety::SAMPLE_RATE_HZ,
        output_gain_dbfs: safety::MAX_OUTPUT_DBFS,
    };

    let mut fixtures = Vec::new();
    for &(name, seed, pressure) in CANON {
        let personality =
            lookup_personality(name).with_context(|| format!("unknown personality `{}`", name))?;
        let mut rng = Mulberry32::new(seed);
        let params = sample_params(personality, &mut rng, pressure);
        let samples = render(&params, &cfg);

        let file_name = format!("{}.wav", name);
        let path = golden_dir.join(&file_name);
        write_wav(&path, &samples, cfg.sample_rate_hz)
            .with_context(|| format!("writing {}", path.display()))?;

        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let hash = sha256_hex(&bytes);

        println!(
            "{:20} seed={:>2} pressure={:.1}  →  {}  ({})",
            name, seed, pressure, file_name, hash
        );

        fixtures.push(Fixture {
            personality: name.to_string(),
            seed,
            pressure,
            file: file_name,
            sha256: hash,
        });
    }

    let manifest = Manifest {
        version: 1,
        comment: "Golden fixtures. One canonical WAV per personality. Regenerate with `cargo run --bin generate-goldens`. The integration test at `tests/golden.rs` enforces that future builds re-render byte-identically.".into(),
        sample_rate_hz: cfg.sample_rate_hz,
        output_gain_dbfs: cfg.output_gain_dbfs,
        channels: 1,
        bits_per_sample: 16,
        fixtures,
    };

    let manifest_path = golden_dir.join("manifest.json");
    let pretty = serde_json::to_string_pretty(&manifest)? + "\n";
    fs::write(&manifest_path, pretty)
        .with_context(|| format!("writing {}", manifest_path.display()))?;
    println!("\nwrote {}", manifest_path.display());

    Ok(())
}
