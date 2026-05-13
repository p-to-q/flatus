//! Mulberry32 PRNG.
//!
//! ~5 lines of state, deterministic, fast, more than good enough for audio jitter.
//! Every function in this crate that reads randomness takes `&mut Mulberry32` as a
//! parameter — there is no global RNG.

/// Seedable, deterministic PRNG. Same seed → same sequence on any platform.
#[derive(Clone, Debug)]
pub struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    /// Seed from any `u64`. Low 32 bits and high 32 bits are XOR-folded so seeds
    /// that differ only in their upper bits still produce distinct streams.
    pub fn new(seed: u64) -> Self {
        let s = (seed as u32) ^ ((seed >> 32) as u32);
        // Avoid the degenerate zero state.
        let s = if s == 0 { 0x9E37_79B9 } else { s };
        Self { state: s }
    }

    /// Raw 32-bit output.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6D2B_79F5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }

    /// Uniform in `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[min, max)`.
    pub fn uniform(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }

    /// Standard normal via Box–Muller. Returns one value; the second is discarded
    /// (we don't care about the small efficiency win of caching it).
    pub fn next_gauss(&mut self) -> f32 {
        // Avoid taking log of zero.
        let u1 = (self.next_f32() + 1.0e-9).min(1.0 - 1.0e-9);
        let u2 = self.next_f32();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        r * theta.cos()
    }

    /// Gaussian sample with mean and standard deviation.
    pub fn gauss(&mut self, mean: f32, stddev: f32) -> f32 {
        mean + stddev * self.next_gauss()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_with_same_seed() {
        let mut a = Mulberry32::new(42);
        let mut b = Mulberry32::new(42);
        for _ in 0..1024 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn uniform_in_range() {
        let mut r = Mulberry32::new(7);
        for _ in 0..10_000 {
            let v = r.next_f32();
            assert!(v >= 0.0 && v < 1.0);
        }
    }

    #[test]
    fn gauss_rough_stats() {
        let mut r = Mulberry32::new(123);
        let n = 10_000;
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        for _ in 0..n {
            let v = r.next_gauss();
            sum += v;
            sum_sq += v * v;
        }
        let mean = sum / n as f32;
        let var = sum_sq / n as f32 - mean * mean;
        // Loose tolerances; just checking the generator is sane.
        assert!(mean.abs() < 0.1, "mean drifted: {}", mean);
        assert!((var - 1.0).abs() < 0.1, "variance drifted: {}", var);
    }
}
