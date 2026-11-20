/// Small deterministic PRNG (`SplitMix64`). Good statistical quality for test signals,
/// no dependencies, identical output on every platform.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Seeded generator.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn uniform(&mut self) -> f64 {
        // 53 random mantissa bits.
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// Standard normal sample (Box-Muller).
    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.uniform(); // (0, 1], avoids ln(0)
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    /// Uniform integer in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.uniform() * n as f64) as usize % n.max(1)
    }
}

/// `count` random symbols in `0..order`.
pub fn random_symbols(count: usize, order: usize, seed: u64) -> Vec<usize> {
    let mut rng = Rng::new(seed);
    (0..count).map(|_| rng.below(order)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_per_seed() {
        assert_eq!(random_symbols(32, 8, 7), random_symbols(32, 8, 7));
        assert_ne!(random_symbols(32, 8, 7), random_symbols(32, 8, 8));
    }

    #[test]
    fn symbols_cover_alphabet_evenly() {
        let s = random_symbols(80_000, 8, 1);
        let mut hist = [0_usize; 8];
        for x in s {
            hist[x] += 1;
        }
        for h in hist {
            assert!((9_500..10_500).contains(&h), "uneven histogram {hist:?}");
        }
    }

    #[test]
    fn normal_has_unit_variance() {
        let mut r = Rng::new(3);
        let n = 200_000;
        let xs: Vec<f64> = (0..n).map(|_| r.normal()).collect();
        let mean = xs.iter().sum::<f64>() / f64::from(n);
        let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / f64::from(n);
        assert!(mean.abs() < 0.01, "mean {mean}");
        assert!((var - 1.0).abs() < 0.02, "variance {var}");
    }
}
