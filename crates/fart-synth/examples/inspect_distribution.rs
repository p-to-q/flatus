//! `inspect_distribution` — draws 100 `FartParams` per personality and prints
//! mean/stddev per axis. Makes the abstract 7-D space concrete; useful when
//! tuning a personality or adding a new one.
//!
//! Run:
//!
//! ```sh
//! cargo run --example inspect_distribution -p fart-synth
//! ```

use fart_synth::personalities::{sample_params, PERSONALITIES};
use fart_synth::prng::Mulberry32;

const N: usize = 100;

fn main() {
    println!(
        "{:18} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "personality", "wetness", "tightness", "patter", "pitch_arc", "tremor", "crackle", "dur_ms"
    );

    for personality in PERSONALITIES {
        let mut wet = Acc::default();
        let mut tig = Acc::default();
        let mut pat = Acc::default();
        let mut arc = Acc::default();
        let mut tre = Acc::default();
        let mut cra = Acc::default();
        let mut dur = Acc::default();

        for seed in 0..N as u64 {
            let mut rng = Mulberry32::new(seed);
            let p = sample_params(personality, &mut rng, 0.6);
            wet.push(p.wetness);
            tig.push(p.tightness);
            pat.push(p.patter);
            arc.push(p.pitch_arc);
            tre.push(p.tremor);
            cra.push(p.crackle);
            dur.push(p.duration_ms as f32);
        }

        println!(
            "{:18} {} {} {} {} {} {} {}",
            personality.name,
            wet.fmt(),
            tig.fmt(),
            pat.fmt(),
            arc.fmt(),
            tre.fmt(),
            cra.fmt(),
            dur.fmt_int()
        );
    }
}

#[derive(Default)]
struct Acc {
    sum: f64,
    sum_sq: f64,
    n: usize,
}

impl Acc {
    fn push(&mut self, x: f32) {
        self.sum += f64::from(x);
        self.sum_sq += f64::from(x) * f64::from(x);
        self.n += 1;
    }

    fn mean(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.sum / self.n as f64
        }
    }

    fn stddev(&self) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        let mean = self.mean();
        let var = (self.sum_sq / self.n as f64) - mean * mean;
        var.max(0.0).sqrt()
    }

    fn fmt(&self) -> String {
        format!("{:>5.2}±{:.2}", self.mean(), self.stddev())
    }

    fn fmt_int(&self) -> String {
        format!("{:>5}±{:.0}", self.mean() as i32, self.stddev())
    }
}
