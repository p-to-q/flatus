//! `fart` — the CLI front-end for `fart-synth`.
//!
//! Examples:
//!
//! ```sh
//! fart                                # one default fart, played out loud
//! fart --personality biblical         # biblical, immediately
//! fart --seed 42 --personality default
//! fart --render out.wav               # don't play; write a WAV
//! fart --print-state                  # dump the chosen FartParams to stderr
//! fart --headphones                   # tighter cap for ear-level output
//! ```

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use fart_synth::personalities::{lookup_personality, sample_params, PERSONALITIES};
use fart_synth::prng::Mulberry32;
use fart_synth::safety;
use fart_synth::wav::write_wav;
use fart_synth::{render, RenderConfig, VERSION};

#[derive(Parser, Debug)]
#[command(
    name = "fart",
    version = VERSION,
    about = "A small apparatus for moving air.",
    long_about = "Synthesize and play one fart. Part of `flatus` — see flatus.ptoq.io."
)]
struct Cli {
    /// Personality name. One of: polite-cough, default, biblical, silent-but-deadly.
    #[arg(short, long, default_value = "default")]
    personality: String,

    /// PRNG seed. Same seed → same fart. Defaults to OS time-derived randomness.
    #[arg(short, long)]
    seed: Option<u64>,

    /// Manual pressure value (0–1). If unset, uses 0.6 (a healthy mid-pressure draw).
    /// Pressure normally comes from the macro state machine; this flag is the manual
    /// override for one-shot CLI use.
    #[arg(long)]
    pressure: Option<f32>,

    /// Apply the tighter headphone cap (−18 dBFS) instead of the speaker cap (−6 dBFS).
    /// Default if you do not pass anything is speakers; the desktop app uses Headphones
    /// as its safer default.
    #[arg(long)]
    headphones: bool,

    /// Don't play. Render to this path as a 16-bit mono WAV.
    #[arg(long)]
    render: Option<PathBuf>,

    /// Render all four personalities into the given directory at a fixed seed
    /// and print a short summary. Useful for `samples/` and for hearing the
    /// whole bestiary without a shell loop.
    #[arg(long, value_name = "DIR")]
    demo: Option<PathBuf>,

    /// Print the sampled `FartParams` to stderr.
    #[arg(long)]
    print_state: bool,

    /// List the available personalities and exit.
    #[arg(long)]
    list_personalities: bool,
}

/// One-line marketing description per personality. Mirrored in apps/web/main.js
/// and README; kept here so `--list-personalities` is a useful CLI affordance.
const DESCRIPTIONS: &[(&str, &str)] = &[
    ("polite-cough", "short, dry, plausibly deniable"),
    ("default", "the canon. wet enough, not so wet"),
    ("biblical", "slow, low, devastating"),
    ("silent-but-deadly", "exactly what it says"),
];

fn description_for(name: &str) -> &'static str {
    DESCRIPTIONS
        .iter()
        .find_map(|(n, d)| (*n == name).then_some(*d))
        .unwrap_or("—")
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.list_personalities {
        for p in PERSONALITIES {
            println!(
                "{:18} {:36}  base_rate={}/hr  refractory={}s",
                p.name,
                description_for(p.name),
                p.base_rate_per_hour,
                p.refractory_secs
            );
        }
        return Ok(());
    }

    if let Some(dir) = cli.demo {
        return run_demo(&dir, cli.headphones);
    }

    let personality = lookup_personality(&cli.personality).ok_or_else(|| {
        let names: Vec<_> = PERSONALITIES.iter().map(|p| p.name).collect();
        anyhow!(
            "unknown personality `{}`. known: {}",
            cli.personality,
            names.join(", ")
        )
    })?;

    let seed = cli.seed.unwrap_or_else(time_seed);
    let pressure = cli.pressure.unwrap_or(0.6).clamp(0.0, 1.0);
    let mut rng = Mulberry32::new(seed);
    let params = sample_params(personality, &mut rng, pressure);

    if cli.print_state {
        eprintln!("personality: {}", personality.name);
        eprintln!("seed:        {seed}");
        eprintln!("params:      {params:#?}");
    }

    let cfg = RenderConfig {
        sample_rate_hz: safety::SAMPLE_RATE_HZ,
        output_gain_dbfs: if cli.headphones {
            safety::HEADPHONE_DBFS
        } else {
            safety::MAX_OUTPUT_DBFS
        },
    };

    let samples = render(&params, &cfg);

    if let Some(path) = cli.render {
        write_wav(&path, &samples, cfg.sample_rate_hz)
            .with_context(|| format!("writing wav to {}", path.display()))?;
        eprintln!(
            "wrote {} samples ({} ms) to {}",
            samples.len(),
            params.duration_ms,
            path.display()
        );
        return Ok(());
    }

    play(&samples, cfg.sample_rate_hz)
}

/// Render every canonical personality to `<dir>/{name}.wav` at a fixed seed and
/// pressure. Prints one summary line per file. Used by `fart --demo`.
fn run_demo(dir: &std::path::Path, headphones: bool) -> Result<()> {
    use std::fs;

    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;

    let cfg = RenderConfig {
        sample_rate_hz: safety::SAMPLE_RATE_HZ,
        output_gain_dbfs: if headphones {
            safety::HEADPHONE_DBFS
        } else {
            safety::MAX_OUTPUT_DBFS
        },
    };

    println!(
        "rendered {} personalities → {}",
        PERSONALITIES.len(),
        dir.display()
    );
    let seed: u64 = 42;
    let pressure: f32 = 0.6;
    for p in PERSONALITIES {
        let mut rng = Mulberry32::new(seed);
        let params = sample_params(p, &mut rng, pressure);
        let samples = render(&params, &cfg);
        let path = dir.join(format!("{}.wav", p.name));
        write_wav(&path, &samples, cfg.sample_rate_hz)
            .with_context(|| format!("writing wav to {}", path.display()))?;
        let bytes = std::fs::metadata(&path).map_or(0, |m| m.len());
        let secs = samples.len() as f32 / cfg.sample_rate_hz as f32;
        println!(
            "  {:18} {:6.2}s  {:>6.1} KB  {}",
            p.name,
            secs,
            bytes as f32 / 1024.0,
            path.display()
        );
    }
    eprintln!(
        "seed={seed}, pressure={pressure}, cap={} dBFS",
        cfg.output_gain_dbfs
    );
    Ok(())
}

/// OS-derived seed; not cryptographically strong, but plenty for "every fart different."
fn time_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0xdead_beef_dead_beef, |d| d.as_nanos() as u64)
}

/// Play `samples` through the default output device. Blocks until playback completes.
fn play(samples: &[f32], sample_rate_hz: u32) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no default output device"))?;

    let supported = device
        .default_output_config()
        .context("no default output config")?;
    let device_channels = supported.channels() as usize;
    let device_sample_rate = supported.sample_rate().0;

    // Simple linear resample if the device runs at a different rate (rare for CoreAudio
    // defaults; common on Linux). Built-in to keep deps minimal.
    let mono: Vec<f32> = if device_sample_rate == sample_rate_hz {
        samples.to_vec()
    } else {
        resample_linear(samples, sample_rate_hz, device_sample_rate)
    };

    let total_frames = mono.len();
    let mut cursor = 0usize;

    let config: StreamConfig = StreamConfig {
        channels: device_channels as u16,
        sample_rate: cpal::SampleRate(device_sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_output_stream(
            &config,
            move |out: &mut [f32], _| {
                feed_mono_to_interleaved(&mono, &mut cursor, device_channels, out);
            },
            err_fn,
            None,
        ),
        SampleFormat::I16 => {
            let mono = mono.clone();
            device.build_output_stream(
                &config,
                move |out: &mut [i16], _| {
                    let mut tmp = vec![0.0_f32; out.len()];
                    feed_mono_to_interleaved(&mono, &mut cursor, device_channels, &mut tmp);
                    for (o, s) in out.iter_mut().zip(tmp.iter()) {
                        *o = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
                    }
                },
                err_fn,
                None,
            )
        }
        SampleFormat::U16 => {
            let mono = mono.clone();
            device.build_output_stream(
                &config,
                move |out: &mut [u16], _| {
                    let mut tmp = vec![0.0_f32; out.len()];
                    feed_mono_to_interleaved(&mono, &mut cursor, device_channels, &mut tmp);
                    for (o, s) in out.iter_mut().zip(tmp.iter()) {
                        let v = (s.clamp(-1.0, 1.0) * 0.5 + 0.5) * f32::from(u16::MAX);
                        *o = v as u16;
                    }
                },
                err_fn,
                None,
            )
        }
        other => return Err(anyhow!("unsupported sample format: {other:?}")),
    }
    .context("could not build output stream")?;

    stream.play().context("could not start stream")?;

    // Wait for playback to drain. Add a small tail so the last frames make it out.
    let duration = Duration::from_secs_f32(total_frames as f32 / device_sample_rate as f32 + 0.25);
    std::thread::sleep(duration);
    drop(stream);

    Ok(())
}

/// Feed the mono buffer (with a moving cursor) into an interleaved multi-channel output,
/// duplicating the mono sample across all device channels. Pads with zero once exhausted.
fn feed_mono_to_interleaved(mono: &[f32], cursor: &mut usize, channels: usize, out: &mut [f32]) {
    for frame in out.chunks_mut(channels) {
        let s = if *cursor < mono.len() {
            let v = mono[*cursor];
            *cursor += 1;
            v
        } else {
            0.0
        };
        for ch in frame.iter_mut() {
            *ch = s;
        }
    }
}

/// Linear resampler. Good enough for a fart; not good enough for anything else.
fn resample_linear(samples: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz {
        return samples.to_vec();
    }
    let ratio = f64::from(from_hz) / f64::from(to_hz);
    let out_len = ((samples.len() as f64) / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        let lo = src.floor() as usize;
        let hi = (lo + 1).min(samples.len().saturating_sub(1));
        let frac = (src - lo as f64) as f32;
        let a = samples.get(lo).copied().unwrap_or(0.0);
        let b = samples.get(hi).copied().unwrap_or(0.0);
        out.push(a + (b - a) * frac);
    }
    out
}
