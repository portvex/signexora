//! Complex mixing (frequency shifting) with a drift-free oscillator.

use rayon::prelude::*;
use sn_core::Complex32;

/// Multiply `data` (sample index `first_index` onward) by `exp(j 2 pi hz n / fs)`.
///
/// The phase is computed from the absolute sample index in `f64`, so long signals and
/// fractional-Hz shifts stay exact (no accumulated rounding drift), and chunked
/// processing gives the same result as one pass.
///
/// Preconditions: `hz` must be finite and `sample_rate` above 0 Hz. Callers validate
/// both; a non-finite `hz` would turn every sample into NaN, and a non-positive
/// `sample_rate` leaves `data` unchanged.
pub fn mix_in_place(data: &mut [Complex32], first_index: usize, hz: f64, sample_rate: f64) {
    if hz == 0.0 || !crate::is_positive(sample_rate) {
        return;
    }
    // Cycles per sample, reduced so the f64 product keeps full precision.
    let cps = (hz / sample_rate).rem_euclid(1.0);
    let base = (first_index as f64 * cps).rem_euclid(1.0);
    let tau = std::f64::consts::TAU;
    // Every sample's phase depends only on its index, so blocks run in parallel and
    // give the same values as a serial pass.
    data.par_chunks_mut(CHUNK)
        .enumerate()
        .for_each(|(c, block)| {
            for (k, v) in (c * CHUNK..).zip(block.iter_mut()) {
                let ph = tau * (base + k as f64 * cps).rem_euclid(1.0);
                let rot = Complex32::new(ph.cos() as f32, ph.sin() as f32);
                *v *= rot;
            }
        });
}

/// Samples per parallel block.
const CHUNK: usize = 1 << 14;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_a_tone_exactly_and_chunks_agree() {
        let fs = 1000.0;
        let mut a = vec![Complex32::new(1.0, 0.0); 2000];
        mix_in_place(&mut a, 0, 125.0, fs);
        // e^{j 2 pi 125 n / 1000}: period 8 samples.
        assert!((a[8] - Complex32::new(1.0, 0.0)).norm() < 1e-6);
        assert!((a[2] - Complex32::new(0.0, 1.0)).norm() < 1e-6);
        let mut b = vec![Complex32::new(1.0, 0.0); 2000];
        let (x, y) = b.split_at_mut(777);
        mix_in_place(x, 0, 125.0, fs);
        mix_in_place(y, 777, 125.0, fs);
        for (p, q) in a.iter().zip(&b) {
            assert!((p - q).norm() < 1e-6);
        }
    }

    #[test]
    fn every_parallel_chunk_matches_the_closed_form() {
        // Three full chunks and a partial one, starting at a nonzero index, so a wrong
        // block offset in any chunk after the first shows up.
        let fs = 48_000.0;
        let hz = 1234.567;
        let first = 98_765_usize;
        let n = 3 * CHUNK + 123;
        let mut v = vec![Complex32::new(1.0, 0.0); n];
        mix_in_place(&mut v, first, hz, fs);
        for (k, got) in v.iter().enumerate() {
            let ph = std::f64::consts::TAU * ((first + k) as f64 * hz / fs).rem_euclid(1.0);
            let want = Complex32::new(ph.cos() as f32, ph.sin() as f32);
            assert!((got - want).norm() < 1e-5, "sample {k}: {got} vs {want}");
        }
    }

    #[test]
    fn fractional_shift_is_drift_free() {
        // 0.1 Hz at 48 kHz over 10 million samples: phase must match the formula.
        let fs = 48_000.0;
        let hz = 0.1;
        let n = 10_000_000_usize;
        let mut v = [Complex32::new(1.0, 0.0)];
        mix_in_place(&mut v, n, hz, fs);
        let expect = std::f64::consts::TAU * (n as f64 * hz / fs);
        let got = f64::from(v[0].im).atan2(f64::from(v[0].re));
        let err = (got - expect.rem_euclid(std::f64::consts::TAU) + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        assert!(err.abs() < 1e-5, "phase error {err}");
    }
}
