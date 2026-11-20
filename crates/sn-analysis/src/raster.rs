//! VMW raster: the signal folded line by line at a chosen period (report 7.7).
//!
//! When the line period matches a period in the signal (a frame, a sync pattern),
//! the structure lines up as vertical stripes. Periods are fractional (samples are
//! interpolated), and strobing shows only every Nth line, which magnifies tiny
//! period errors into visible slopes.

use rayon::prelude::*;
use sn_core::Signal;

use crate::{Error, Result};

/// A raster image of values, row-major.
#[derive(Debug, Clone, PartialEq)]
pub struct Raster {
    /// Number of lines.
    pub rows: usize,
    /// Values per line.
    pub cols: usize,
    /// `rows * cols` values (real part of the signal).
    pub values: Vec<f32>,
}

/// What to fold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterRequest {
    /// First sample of line 0 (may be fractional).
    pub start: f64,
    /// Line period in samples (may be fractional).
    pub period: f64,
    /// Show every `strobe`-th period as a line (1 = every period, at most
    /// [`MAX_STROBE`]).
    pub strobe: usize,
    /// Lines to compute.
    pub rows: usize,
    /// Values per line.
    pub cols: usize,
}

/// Largest raster computed, in values.
pub const MAX_VALUES: usize = 1 << 24;

/// Largest strobe factor.
pub const MAX_STROBE: usize = 1_000_000;

/// Lines up to this many samples are read in one piece; longer lines are read only
/// at the samples their columns need.
const MAX_LINE_READ: usize = 1 << 20;

/// Fold `signal` as requested. Lines past the end of the signal are NaN.
///
/// Same as [`build_with_stop`] without cancellation.
pub fn build(signal: &Signal, r: &RasterRequest) -> Result<Raster> {
    build_with_stop(signal, r, &|| false)
}

/// [`build`] that checks `should_stop` once per line and returns
/// [`Error::Cancelled`] when it fires.
pub fn build_with_stop(
    signal: &Signal,
    r: &RasterRequest,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Raster> {
    let len = signal.len();
    if !(r.period.is_finite() && r.period >= 1.0) {
        return Err(Error::Invalid(
            "the line period must be at least one sample".into(),
        ));
    }
    if r.period > len as f64 {
        return Err(Error::Invalid(
            "the line period exceeds the signal length; shorten it".into(),
        ));
    }
    if !r.start.is_finite() {
        return Err(Error::Invalid(
            "the first line must start at a finite time".into(),
        ));
    }
    if r.rows == 0 || r.cols == 0 || r.rows.saturating_mul(r.cols) > MAX_VALUES {
        return Err(Error::Invalid(format!(
            "the raster must have 1..={MAX_VALUES} values"
        )));
    }
    if !(1..=MAX_STROBE).contains(&r.strobe) {
        return Err(Error::Invalid(format!(
            "the strobe must be between 1 and {MAX_STROBE}"
        )));
    }
    let strobe = r.strobe as f64;
    let step = r.period / r.cols as f64;
    // Each line's span plus one sample for interpolation; period <= len bounds it.
    let span = (r.period.ceil() as usize).saturating_add(2);
    let whole_line = span <= MAX_LINE_READ || span <= 2 * r.cols;
    // Rows are independent: they are filled in parallel, each exactly as in a serial
    // pass.
    let mut values = vec![f32::NAN; r.rows * r.cols];
    values
        .par_chunks_mut(r.cols)
        .enumerate()
        .try_for_each_init(
            || vec![0.0_f32; if whole_line { span } else { 0 }],
            |line, (row, out)| {
                if should_stop() {
                    return Err(Error::Cancelled);
                }
                let t0 = r.start + row as f64 * strobe * r.period;
                let base = t0.floor();
                if t0 < 0.0 || base >= len as f64 {
                    return Ok(());
                }
                let first = base as usize;
                let got = if whole_line {
                    signal.read_real(first, line)
                } else {
                    0
                };
                let mut pair = [0.0_f32; 2];
                for (c, o) in out.iter_mut().enumerate() {
                    let t = t0 - base + c as f64 * step;
                    let i = t.floor() as usize;
                    let frac = (t - t.floor()) as f32;
                    let ab = if whole_line {
                        match (line.get(i), line.get(i + 1)) {
                            (Some(a), Some(b)) if i + 1 < got => Some((*a, *b)),
                            _ => None,
                        }
                    } else {
                        let n = signal.read_real(first.saturating_add(i), &mut pair);
                        let [a, b] = pair;
                        (n == 2).then_some((a, b))
                    };
                    *o = ab.map_or(f32::NAN, |(a, b)| a + (b - a) * frac);
                }
                Ok(())
            },
        )?;
    Ok(Raster {
        rows: r.rows,
        cols: r.cols,
        values,
    })
}

impl Raster {
    /// 1st and 99th percentile of the finite values (a robust display range).
    pub fn auto_range(&self) -> Option<(f32, f32)> {
        let mut s: Vec<f32> = self
            .values
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .collect();
        if s.is_empty() {
            return None;
        }
        let last = s.len() - 1;
        let rank = |q: f64| ((last as f64 * q).round() as usize).min(last);
        let lo = *s.select_nth_unstable_by(rank(0.01), f32::total_cmp).1;
        let hi = *s.select_nth_unstable_by(rank(0.99), f32::total_cmp).1;
        Some((lo, hi))
    }

    /// Variance of each column across lines (finite values only). Low variance marks
    /// a column that is the same in every period, such as a sync pattern. A column
    /// with fewer than two finite values has no variance and gives NaN; skip those.
    pub fn column_variance(&self) -> Vec<f64> {
        (0..self.cols)
            .map(|c| {
                let col: Vec<f64> = (0..self.rows)
                    .filter_map(|r| self.values.get(r * self.cols + c).copied())
                    .filter(|v| v.is_finite())
                    .map(f64::from)
                    .collect();
                if col.len() < 2 {
                    return f64::NAN;
                }
                let m = col.iter().sum::<f64>() / col.len() as f64;
                col.iter().map(|v| (v - m).powi(2)).sum::<f64>() / col.len() as f64
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use sn_synth::{MfskParams, Rng, add_awgn_snr, mfsk, random_symbols};

    use super::*;
    use crate::histogram::Histogram;
    use crate::tones::count_grid;

    /// Phase detector, then a median over a third of a symbol: removes noise but keeps
    /// the frequency plateaus flat (a sharp lowpass would ring at every step).
    fn freq_track(s: &Signal, baud: f64) -> Signal {
        let pd = sn_dsp::ops::Edit::PhaseDetector.run(s, &|| false).unwrap();
        let window = ((s.sample_rate() / baud / 3.0) as usize).clamp(1, 301) | 1;
        sn_dsp::ops::Edit::Median { window }
            .run(&pd, &|| false)
            .unwrap()
    }

    #[test]
    fn eight_tone_mfsk_spacing_to_one_hz() {
        // Exit criterion: 8 tones about 242 Hz apart measured to +/-1 Hz.
        let fs = 16_000.0;
        let p = MfskParams {
            order: 8,
            baud: 100.0,
            spacing: 242.0,
            center: 2500.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let s = add_awgn_snr(&mfsk(&p, &random_symbols(1500, 8, 3)).unwrap(), 20.0, 1).unwrap();
        let f = freq_track(&s, 100.0);
        let h = Histogram::build(&f, 2000, f.len() - 2000, 4000, None).unwrap();
        let tones = h.peaks(0.15, 4.0);
        assert_eq!(tones.len(), 8, "{tones:?}");
        let g = count_grid(&tones, None, None, 0.2).unwrap();
        assert_eq!(g.size, 8);
        assert!((g.spacing - 242.0).abs() < 1.0, "spacing {}", g.spacing);
    }

    #[test]
    fn sparse_32_tone_grid_is_counted() {
        // Exit criterion: a 32-tone grid with unused tones.
        let fs = 16_000.0;
        let p = MfskParams {
            order: 32,
            baud: 50.0,
            spacing: 100.0,
            center: 3000.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let used = [0_usize, 1, 3, 7, 12, 20, 26, 31];
        let mut rng = Rng::new(5);
        let symbols: Vec<usize> = (0..1200).map(|_| used[rng.below(used.len())]).collect();
        let s = add_awgn_snr(&mfsk(&p, &symbols).unwrap(), 25.0, 2).unwrap();
        let f = freq_track(&s, 50.0);
        let h = Histogram::build(&f, 4000, f.len() - 4000, 6000, None).unwrap();
        let tones = h.peaks(0.15, 4.0);
        assert_eq!(tones.len(), used.len(), "{tones:?}");
        let g = count_grid(&tones, None, None, 0.2).unwrap();
        assert_eq!((g.size, g.occupied), (32, used.len()));
    }

    #[test]
    fn raster_reveals_an_eight_bit_sync_pattern() {
        // Exit criterion: frames of 32 bits, the first 8 a fixed sync word.
        let fs = 12_000.0;
        let baud = 600.0;
        let sync = [1_usize, 0, 1, 1, 0, 0, 1, 0];
        let mut rng = Rng::new(9);
        let mut bits = Vec::new();
        for _ in 0..60 {
            bits.extend_from_slice(&sync);
            bits.extend((0..24).map(|_| rng.below(2)));
        }
        let p = MfskParams {
            order: 2,
            baud,
            spacing: 1200.0,
            center: 2400.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let s = add_awgn_snr(&mfsk(&p, &bits).unwrap(), 15.0, 3).unwrap();
        let f = freq_track(&s, baud);
        let sps = fs / baud; // 20 samples per bit
        // Start half a bit in so each column samples the middle of a bit.
        let r = build(
            &f,
            &RasterRequest {
                start: sps / 2.0,
                period: 32.0 * sps,
                strobe: 1,
                rows: 59,
                cols: 32,
            },
        )
        .unwrap();
        let var = r.column_variance();
        // Sample mid-bit columns: bit k sits at column k (32 columns per frame).
        let sync_var: f64 = (1..7).map(|k| var[k]).sum::<f64>() / 6.0;
        let data_var: f64 = (10..30).map(|k| var[k]).sum::<f64>() / 20.0;
        assert!(
            sync_var * 20.0 < data_var,
            "sync {sync_var:.1} vs data {data_var:.1}"
        );
    }

    #[test]
    fn strobe_and_bad_requests() {
        let s = Signal::real((0..1000).map(|i| i as f32).collect::<Vec<_>>(), 1.0).unwrap();
        let r = build(
            &s,
            &RasterRequest {
                start: 0.0,
                period: 10.0,
                strobe: 3,
                rows: 4,
                cols: 10,
            },
        )
        .unwrap();
        assert!(
            (r.values[10] - 30.0).abs() < 1e-6,
            "line 1 starts 3 periods later"
        );
        assert!((r.values[1] - 1.0).abs() < 1e-6);
        let half = build(
            &s,
            &RasterRequest {
                start: 0.5,
                period: 10.0,
                strobe: 1,
                rows: 1,
                cols: 10,
            },
        )
        .unwrap();
        assert!((half.values[0] - 0.5).abs() < 1e-6, "interpolated");
        let past = build(
            &s,
            &RasterRequest {
                start: 995.0,
                period: 10.0,
                strobe: 1,
                rows: 2,
                cols: 10,
            },
        )
        .unwrap();
        assert!(past.values[15].is_nan());
        assert!(
            build(
                &s,
                &RasterRequest {
                    start: 0.0,
                    period: 0.5,
                    strobe: 1,
                    rows: 1,
                    cols: 1
                }
            )
            .is_err()
        );
        assert!(r.auto_range().is_some());
    }

    #[test]
    fn huge_periods_bad_strobes_and_cancellation() {
        let s = Signal::real((0..1000).map(|i| i as f32).collect::<Vec<_>>(), 1.0).unwrap();
        let req = |period: f64, strobe: usize| RasterRequest {
            start: 0.0,
            period,
            strobe,
            rows: 2,
            cols: 4,
        };
        for p in [1e12, 1e15, 1e20, 1001.0] {
            let e = build(&s, &req(p, 1)).unwrap_err();
            assert!(e.to_string().contains("exceeds"), "{e}");
        }
        assert!(build(&s, &req(10.0, 0)).is_err());
        assert!(build(&s, &req(10.0, MAX_STROBE + 1)).is_err());
        assert!(matches!(
            build_with_stop(&s, &req(10.0, 1), &|| true),
            Err(Error::Cancelled)
        ));
        // Rows past the end have no data: their columns have no variance.
        let r = build(
            &s,
            &RasterRequest {
                start: 990.0,
                period: 100.0,
                strobe: 1,
                rows: 3,
                cols: 4,
            },
        )
        .unwrap();
        assert!(r.column_variance().iter().all(|v| v.is_nan()));
    }

    #[test]
    fn long_lines_are_read_column_by_column() {
        let n = 3 * MAX_LINE_READ;
        let s = Signal::real((0..n).map(|i| (i % 4096) as f32).collect::<Vec<_>>(), 1.0).unwrap();
        let period = MAX_LINE_READ as f64 * 1.25 + 0.5;
        let r = build(
            &s,
            &RasterRequest {
                start: 0.25,
                period,
                strobe: 1,
                rows: 3,
                cols: 8,
            },
        )
        .unwrap();
        for row in 0..2 {
            for c in 0..8 {
                let t = 0.25 + row as f64 * period + c as f64 * period / 8.0;
                let i = t.floor() as usize;
                let (a, b) = ((i % 4096) as f64, ((i + 1) % 4096) as f64);
                let want = if b > a { a + (t - t.floor()) } else { f64::NAN };
                let got = f64::from(r.values[row * 8 + c]);
                if want.is_finite() {
                    assert!(
                        (got - want).abs() < 1e-2,
                        "row {row} col {c}: {got} vs {want}"
                    );
                }
            }
        }
        // The third line runs past the end of the signal.
        assert!(r.values[2 * 8 + 7].is_nan());
    }
}
