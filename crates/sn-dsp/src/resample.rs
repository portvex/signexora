//! Arbitrary-ratio resampling (including fractional rates) with a windowed-sinc
//! polyphase filter.
//!
//! Documented response (checked by the tests):
//! - Passband: 0 to 0.90 x the lower Nyquist frequency, ripple within +/-0.05 dB.
//! - Stopband: from the lower Nyquist frequency upward, at least 80 dB down.
//!
//! Output sample `m` is the band-limited input evaluated at time `m / fs_out`, so the
//! first output sample lines up with the first input sample (no delay).

use std::f64::consts::PI;

use rayon::prelude::*;
use sn_core::{Complex32, Signal};

use crate::fir::{kaiser_beta, read_padded};
use crate::window::WindowKind;
use crate::{Error, Result};

/// Phases in the interpolation table; values in between are linearly interpolated.
const PHASES: usize = 1024;
/// Most coefficients in the table (16 MB of f32). Heavy decimation needs very long
/// filters; their sinc is so wide (thousands of input samples per oscillation) that
/// a few phases interpolate it as accurately as 1024 do for short filters, so long
/// filters get fewer phases instead of gigabytes of table.
const MAX_TABLE: usize = 1 << 22;
/// Most output samples computed per input read (one parallel block).
const OUTPUT_CHUNK: usize = 1 << 15;
/// Fewest output samples per block.
const MIN_OUTPUT_CHUNK: usize = 256;
/// Input samples a block aims to read (besides the filter length).
const INPUT_SPAN: usize = 1 << 16;
/// Stopband attenuation target in dB.
pub const STOPBAND_DB: f64 = 80.0;
/// Passband edge as a fraction of the lower Nyquist frequency.
pub const PASSBAND: f64 = 0.90;

/// A resampler from `fs_in` to `fs_out`.
#[derive(Debug, Clone)]
pub struct Resampler {
    fs_in: f64,
    fs_out: f64,
    /// Half-length of the filter in input samples.
    half: usize,
    /// `table[p][k]`: tap `k` for fractional phase `p / phases` (flattened).
    table: Vec<f32>,
    taps: usize,
    /// Phases in the table (`PHASES`, fewer for very long filters).
    phases: usize,
}

impl Resampler {
    /// Build a resampler. Both rates must be positive and finite.
    pub fn new(fs_in: f64, fs_out: f64) -> Result<Self> {
        let ok = |f: f64| f.is_finite() && f > 0.0;
        if !ok(fs_in) || !ok(fs_out) {
            return Err(Error::InvalidParameter {
                name: "sample_rate",
                reason: "rates must be finite and above 0 Hz".into(),
            });
        }
        let ratio = fs_out / fs_in;
        if !(1e-4..=1e4).contains(&ratio) {
            return Err(Error::InvalidParameter {
                name: "sample_rate",
                reason: format!("ratio {ratio} is outside 1e-4..=1e4; resample in steps"),
            });
        }
        // Cutoff (relative to the input rate) sits between the passband edge and the
        // lower Nyquist frequency.
        let nyq = 0.5 * ratio.min(1.0);
        let pass = PASSBAND * nyq;
        let transition = nyq - pass;
        let cutoff = f64::midpoint(pass, nyq);
        let taps_f = (STOPBAND_DB - 7.95) / (14.36 * transition);
        let half = ((taps_f / 2.0).ceil() as usize).max(4);
        let taps = 2 * half;
        let window = WindowKind::Kaiser {
            beta: kaiser_beta(STOPBAND_DB),
        };
        let phases = (MAX_TABLE / taps).clamp(1, PHASES);
        // Window sampled finely across the filter span.
        let wlen = taps * phases + 1;
        let w = window.coefficients(wlen);
        let mut table = Vec::with_capacity((phases + 1) * taps);
        for p in 0..=phases {
            let frac = p as f64 / phases as f64;
            let mut row: Vec<f64> = (0..taps)
                .map(|k| {
                    // Distance from the output instant to input sample k.
                    let x = k as f64 - (half as f64 - 1.0) - frac;
                    let s = if x.abs() < 1e-12 {
                        2.0 * cutoff
                    } else {
                        (2.0 * PI * cutoff * x).sin() / (PI * x)
                    };
                    let wi = ((x + half as f64) * phases as f64).round();
                    let wv = if wi >= 0.0 {
                        w.get(wi as usize).copied().map_or(0.0, f64::from)
                    } else {
                        0.0
                    };
                    s * wv
                })
                .collect();
            let sum: f64 = row.iter().sum();
            if sum.abs() > 1e-12 {
                for v in &mut row {
                    *v /= sum;
                }
            }
            table.extend(row.iter().map(|v| *v as f32));
        }
        Ok(Self {
            fs_in,
            fs_out,
            half,
            table,
            taps,
            phases,
        })
    }

    /// Number of output samples for `n` input samples.
    pub fn output_len(&self, n: usize) -> usize {
        (n as f64 * self.fs_out / self.fs_in).round() as usize
    }

    fn row(&self, p: usize) -> &[f32] {
        let a = p.saturating_mul(self.taps);
        self.table.get(a..a + self.taps).unwrap_or(&[])
    }

    /// Resample samples `start..end` of `signal`.
    ///
    /// Blocks of output samples are computed in parallel; every output sample is the
    /// same dot product as in a serial pass, so the result does not depend on the
    /// number of threads.
    pub fn apply(
        &self,
        signal: &Signal,
        start: usize,
        end: usize,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<Complex32>> {
        let n_in = end.saturating_sub(start);
        let n_out = self.output_len(n_in);
        let step = self.fs_in / self.fs_out;
        // Keep each block's input span near INPUT_SPAN samples (plus the filter), so
        // heavy decimation does not read megabytes per block and thread.
        let chunk = ((INPUT_SPAN as f64 / step) as usize).clamp(MIN_OUTPUT_CHUNK, OUTPUT_CHUNK);
        let mut out = vec![Complex32::new(0.0, 0.0); n_out];
        out.par_chunks_mut(chunk).enumerate().try_for_each_init(
            Vec::new,
            |window, (c, block)| {
                if should_stop() {
                    return Err(Error::Cancelled);
                }
                self.fill_block(signal, start, step, c * chunk, block, window);
                Ok(())
            },
        )?;
        Ok(out)
    }

    /// Output samples `m0..m0 + block.len()` into `block`, reading the input span
    /// they need into `window`.
    fn fill_block(
        &self,
        signal: &Signal,
        start: usize,
        step: f64,
        m0: usize,
        block: &mut [Complex32],
        window: &mut Vec<Complex32>,
    ) {
        let Some(m_last) = (m0 + block.len()).checked_sub(1) else {
            return;
        };
        let t0 = m0 as f64 * step;
        let first = t0.floor() as i64 - self.half as i64 + 1;
        let last = (m_last as f64 * step).floor() as i64 + self.half as i64;
        let span = usize::try_from(last - first + 1).unwrap_or(0);
        // read_padded overwrites all `span` samples, so only newly grown elements
        // need a value.
        window.resize(span, Complex32::new(0.0, 0.0));
        read_padded(signal, start as i64 + first, window);
        for (m, o) in (m0..).zip(block.iter_mut()) {
            let t = m as f64 * step;
            let base = t.floor();
            let frac = (t - base) * self.phases as f64;
            let p = (frac.floor() as usize).min(self.phases - 1);
            let mu = (frac - p as f64) as f32;
            let (r0, r1) = (self.row(p), self.row(p + 1));
            let offset = usize::try_from(base as i64 - self.half as i64 + 1 - first).unwrap_or(0);
            *o = window
                .get(offset..offset + self.taps)
                .map_or(Complex32::new(0.0, 0.0), |xs| dot(xs, r0, r1, mu));
        }
    }
}

/// `sum x[k] (a[k] + (b[k] - a[k]) mu)` with eight partial sums per component, which
/// the compiler can keep in vector registers.
fn dot(xs: &[Complex32], a: &[f32], b: &[f32], mu: f32) -> Complex32 {
    const L: usize = 8;
    let mut re = [0.0_f32; L];
    let mut im = [0.0_f32; L];
    let (xc, xr) = xs.as_chunks::<L>();
    let (ac, ar) = a.as_chunks::<L>();
    let (bc, br) = b.as_chunks::<L>();
    for ((x, a), b) in xc.iter().zip(ac).zip(bc) {
        for ((((r, i), x), a), b) in re.iter_mut().zip(im.iter_mut()).zip(x).zip(a).zip(b) {
            let w = a + (b - a) * mu;
            *r += x.re * w;
            *i += x.im * w;
        }
    }
    let mut acc = Complex32::new(re.iter().sum(), im.iter().sum());
    for ((x, a), b) in xr.iter().zip(ar).zip(br) {
        acc += x * (a + (b - a) * mu);
    }
    acc
}

#[cfg(test)]
mod tests {
    use sn_synth::tone;

    use super::*;

    fn amplitude_at(x: &[Complex32], f: f64, fs: f64) -> f64 {
        // Single-bin DFT on a Hann-windowed segment; returns the tone amplitude.
        let n = x.len();
        let (mut re, mut im, mut wsum) = (0.0, 0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let w = 0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos();
            let ph = -2.0 * PI * f * i as f64 / fs;
            re += w * f64::from(v.re) * ph.cos();
            im += w * f64::from(v.re) * ph.sin();
            wsum += w;
        }
        2.0 * (re * re + im * im).sqrt() / wsum
    }

    /// Found by the `edit` fuzz target: 8000 Hz to 4 Hz built a 1.6 GB table.
    #[test]
    fn heavy_decimation_keeps_the_table_small_and_the_response() {
        let r = Resampler::new(8000.0, 4.0).unwrap();
        assert!(r.table.len() <= MAX_TABLE + r.taps, "{}", r.table.len());
        // 20 kHz to 10 Hz (ratio 5e-4): a 2 Hz tone passes, 6 Hz (above the new
        // Nyquist) does not alias into the output.
        let (fs_in, fs_out) = (20_000.0, 10.0);
        let r = Resampler::new(fs_in, fs_out).unwrap();
        let pass = tone(2.0, 0.5, fs_in, 120.0).unwrap();
        let y = r.apply(&pass, 0, pass.len(), &|| false).unwrap();
        let mid = &y[y.len() / 4..3 * y.len() / 4];
        let db = 20.0 * (amplitude_at(mid, 2.0, fs_out) / 0.5).log10();
        assert!(db.abs() < 0.05, "passband {db} dB");
        let stop = tone(6.0, 0.5, fs_in, 120.0).unwrap();
        let y = r.apply(&stop, 0, stop.len(), &|| false).unwrap();
        let mid = &y[y.len() / 4..3 * y.len() / 4];
        let alias = amplitude_at(mid, 4.0, fs_out);
        assert!(20.0 * (alias / 0.5).log10() < -80.0, "alias {alias}");
    }

    fn check(fs_in: f64, fs_out: f64) {
        let r = Resampler::new(fs_in, fs_out).unwrap();
        let nyq = fs_in.min(fs_out) / 2.0;
        // Passband tones keep their amplitude (+/-0.05 dB) and frequency.
        for frac in [0.05, 0.4, PASSBAND] {
            let f = frac * nyq;
            let s = tone(f, 0.5, fs_in, 1.0).unwrap();
            let y = r.apply(&s, 0, s.len(), &|| false).unwrap();
            assert_eq!(y.len(), r.output_len(s.len()));
            let mid = &y[y.len() / 4..3 * y.len() / 4];
            let a = amplitude_at(mid, f, fs_out);
            let db = 20.0 * (a / 0.5).log10();
            assert!(
                db.abs() < 0.05,
                "{fs_in}->{fs_out}: {f} Hz passband {db:.3} dB"
            );
        }
        // A tone above the new Nyquist frequency is removed (when downsampling).
        if fs_out < fs_in {
            let f = (nyq * 1.05).min(fs_in / 2.0 * 0.99);
            let s = tone(f, 0.5, fs_in, 1.0).unwrap();
            let y = r.apply(&s, 0, s.len(), &|| false).unwrap();
            let mid = &y[y.len() / 4..3 * y.len() / 4];
            let peak = mid
                .iter()
                .map(|c| f64::from(c.re.abs()))
                .fold(0.0, f64::max);
            let db = 20.0 * (peak / 0.5).max(1e-12).log10();
            assert!(
                db < -STOPBAND_DB + 3.0,
                "{fs_in}->{fs_out}: stopband {db:.1} dB"
            );
        }
    }

    #[test]
    fn integer_and_fractional_ratios_meet_the_spec() {
        check(48_000.0, 8000.0);
        check(8000.0, 48_000.0);
        check(11_025.0, 8000.0);
        check(8000.0, 7_014.322_008);
        check(68_000.0, 60_000.0);
    }

    #[test]
    fn dot_matches_a_plain_sum_including_the_tail() {
        let mut rng = sn_synth::Rng::new(5);
        let mut r = || (rng.uniform() * 2.0 - 1.0) as f32;
        for len in 0..=24 {
            let xs: Vec<Complex32> = (0..len).map(|_| Complex32::new(r(), r())).collect();
            let a: Vec<f32> = (0..len).map(|_| r()).collect();
            let b: Vec<f32> = (0..len).map(|_| r()).collect();
            for mu in [0.0_f32, 0.37, 1.0] {
                let got = dot(&xs, &a, &b, mu);
                let (mut re, mut im, mut mag) = (0.0_f64, 0.0_f64, 0.0_f64);
                for ((x, a), b) in xs.iter().zip(&a).zip(&b) {
                    let w = f64::from(*a) + (f64::from(*b) - f64::from(*a)) * f64::from(mu);
                    re += f64::from(x.re) * w;
                    im += f64::from(x.im) * w;
                    mag += f64::from(x.norm()) * w.abs();
                }
                // Relative to the sum of magnitudes, so cancellation cannot make a
                // correct result look wrong.
                let tol = 1e-5 * mag.max(1e-30);
                assert!(
                    (f64::from(got.re) - re).abs() <= tol && (f64::from(got.im) - im).abs() <= tol,
                    "len {len}, mu {mu}: {got} vs {re} + {im}i"
                );
            }
        }
    }

    #[test]
    fn rejects_bad_rates_and_cancels() {
        assert!(Resampler::new(0.0, 8000.0).is_err());
        assert!(Resampler::new(8000.0, f64::NAN).is_err());
        assert!(Resampler::new(1.0, 1e6).is_err());
        let r = Resampler::new(8000.0, 4000.0).unwrap();
        let s = tone(100.0, 0.5, 8000.0, 1.0).unwrap();
        assert!(matches!(
            r.apply(&s, 0, s.len(), &|| true),
            Err(Error::Cancelled)
        ));
    }
}
