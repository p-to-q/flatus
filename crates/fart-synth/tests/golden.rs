//! Integration test for the determinism contract.
//!
//! Reads `fixtures/golden/manifest.json`, re-renders each fixture's `(personality,
//! seed, pressure)`, writes it to a temp buffer, hashes, and asserts the hash
//! equals the manifest's `sha256` field.
//!
//! If the manifest has placeholder SHA-256s (`TODO_GENERATED_ON_FIRST_RUN`), the
//! test prints a friendly note and exits early — it doesn't fail, because the
//! correct first action is to run `cargo run --bin generate-goldens` once.

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use fart_synth::personalities::{lookup_personality, sample_params};
use fart_synth::prng::Mulberry32;
use fart_synth::wav::{sha256_hex, write_wav};
use fart_synth::{render, RenderConfig};

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    personality: String,
    seed: u64,
    pressure: f32,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    sample_rate_hz: u32,
    output_gain_dbfs: f32,
    fixtures: Vec<Fixture>,
}

fn manifest_path() -> PathBuf {
    // The test runs from the crate root (CARGO_MANIFEST_DIR). The manifest lives
    // two levels up. We accept both layouts so the test still works if someone
    // copies the crate out.
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        crate_root.join("../../fixtures/golden/manifest.json"),
        crate_root.join("fixtures/golden/manifest.json"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    panic!("could not locate fixtures/golden/manifest.json — run from repo root");
}

#[test]
fn renders_match_manifest() {
    let path = manifest_path();
    let raw = fs::read_to_string(&path).expect("read manifest");
    let manifest: Manifest = serde_json::from_str(&raw).expect("parse manifest");

    // Skip gracefully if the manifest is still placeholder.
    if manifest
        .fixtures
        .iter()
        .any(|f| f.sha256.starts_with("TODO"))
    {
        eprintln!(
            "skip: manifest has placeholder SHA-256s. run `cargo run --bin generate-goldens` first."
        );
        return;
    }

    let cfg = RenderConfig {
        sample_rate_hz: manifest.sample_rate_hz,
        output_gain_dbfs: manifest.output_gain_dbfs,
    };

    for fixture in &manifest.fixtures {
        let personality = lookup_personality(&fixture.personality)
            .unwrap_or_else(|| panic!("unknown personality `{}`", fixture.personality));
        let mut rng = Mulberry32::new(fixture.seed);
        let params = sample_params(personality, &mut rng, fixture.pressure);
        let samples = render(&params, &cfg);

        // Write to an in-memory buffer (via a temp file path, since our wav
        // writer goes through `File`). We use a temp file under the OS temp dir.
        let tmp = std::env::temp_dir().join(format!("flatus-golden-{}.wav", fixture.personality));
        write_wav(&tmp, &samples, cfg.sample_rate_hz).expect("write wav");
        let bytes = fs::read(&tmp).expect("read wav back");
        let hash = sha256_hex(&bytes);

        assert_eq!(
            hash, fixture.sha256,
            "synthesis drifted for `{}` (seed {}, pressure {}). \
             If you intended this, re-run `cargo run --bin generate-goldens`.",
            fixture.personality, fixture.seed, fixture.pressure
        );
    }
}
