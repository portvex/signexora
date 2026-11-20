//! Precise symbol rate, fractional LU and frequency offset (SA: Get Br).
//!
//! 1. **LS, fractionally.** `p[n] = x[n] * conj(x[n + LU])` is folded at a fractional
//!    period L (symbol `k` is shifted by `k L` with linear interpolation). The triangle
//!    peak (the largest sum over LG consecutive folded samples) is highest when the
//!    CPs of all folded symbols line up. The search is hierarchical:
//!    start with `block` symbols over LS +/- 0.6, then fold 4 times as many symbols
//!    over a range matched to the previous precision, until all symbols are used. The
//!    peak width in L is about LG / symbols, so precision grows with the record
//!    length.
//! 2. **LU, fractionally.** Over fixed positions `n` (the inner part of every CP),
//!    the normalised CP correlation between `x[n]` and the partner `x(n + lambda)`
//!    (band-limited interpolation) is 1 exactly at the true, possibly non-integer, LU
//!    of a clean signal and lower elsewhere (Cauchy-Schwarz). Noise biases the plain
//!    `|c| / sqrt(Ex Ey)`: `|c|^2` and the partner energy `Ey` both carry a noise floor
//!    that depends on the fractional part of `lambda` (the interpolator passes about
//!    3 % less white noise at half a sample than at a whole one), which pulled the
//!    peak away from LU as the SNR fell. Two changes remove both floors: the
//!    correlation is computed separately over even and odd symbols and the numerator
//!    is `Re(c_even conj(c_odd))`, whose noise terms are independent and average to
//!    zero; and the partner energies lose `n sigma^2 sum(h^2)`, the noise the
//!    interpolator taps `h` pass, with `sigma^2` estimated from the correlation at
//!    the best whole lag. The search covers LU +/- LG/2 (at least 3 samples) around
//!    Find CT's LU at whole lags, then a fractional grid and golden-section search
//!    within a sample of the best; it stops with an error when the peak sits at the
//!    edge of the range. The taps are built once per trial lag.
//! 3. **Frequency offset.** At the lag LU (rounded to whole samples, the FFT size used
//!    for the LU spectrum) every channel contributes the phase `-2 pi f0 LU / Fd`,
//!    where f0 is the offset of the channels from the grid of spacing `Fd / LU`. That
//!    phase gives f0 within +/- half a spacing. At a rate that is not native the
//!    channels cannot all sit on an integer-LU grid, and f0 is a power-weighted
//!    compromise.

use num_complex::Complex64;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sn_core::Complex32;

use crate::util::{
    TAP_HALF, TAPS, c64, golden_max, lag_product, sliding_sums, taps, triangle_vertex,
};
use crate::{Baseband, CtResult, Error, Result, check_stop, invalid};

/// Settings for [`get_br`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrParams {
    /// Symbols in the first, coarse stage (SA's block size; 20-40 is good, never below
    /// 6-8).
    pub block: usize,
    /// Most symbols used in the final stage.
    pub max_symbols: usize,
}

impl Default for BrParams {
    fn default() -> Self {
        Self {
            block: 40,
            max_symbols: 20_000,
        }
    }
}

/// Result of [`get_br`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrEstimate {
    /// Sample rate of the analysed data.
    pub sample_rate: f64,
    /// Symbol length in samples (fractional).
    pub ls: f64,
    /// Symbol rate `sample_rate / ls` in Bd.
    pub br: f64,
    /// Useful length in samples (fractional).
    pub lu: f64,
    /// Guard length `ls - lu` in samples (fractional).
    pub lg: f64,
    /// `K = lg / lu`.
    pub k: f64,
    /// Channel spacing `sample_rate / lu` in Hz.
    pub spacing: f64,
    /// Offset of the channels from the grid of `sample_rate / lu_int`, in Hz, within
    /// +/- half a spacing. Shift by minus this to put the channels on the grid.
    pub freq_offset: f64,
    /// Start of the first complete symbol (its CP), fractional baseband index.
    pub start: f64,
    /// Symbols used in the final stage.
    pub symbols: usize,
    /// Triangle coherence at the final LS (0..1).
    pub coherence: f64,
    /// LU rounded to whole samples (the FFT size for the LU spectrum).
    pub lu_int: usize,
    /// LG rounded to whole samples.
    pub lg_int: usize,
}

/// Bins per parallel block of [`fold_frac`].
const FOLD_BINS: usize = 64;

/// CP positions per parallel block of [`lag_sums`].
const LAG_BLOCK: usize = 4096;

/// Most CP positions used for the LU search.
const POSITIONS: usize = 60_000;

/// Triangle peak of `p` folded at the fractional period `l` over `k` periods into
/// `width` bins, with windows of `lg`. Each period is shifted into place with linear
/// interpolation, so the peak is a smooth function of `l`. Also returns the folded
/// profile and folded `|p|` when `keep` is set.
fn fold_frac(
    p: &[Complex32],
    l: f64,
    k: usize,
    width: usize,
    lg: usize,
    keep: bool,
) -> (f64, Vec<Complex64>, Vec<f64>) {
    // Period starts and interpolation weights, up to the first period that runs past
    // the data.
    let periods: Vec<(usize, f64)> = (0..k)
        .map(|kk| {
            let pos = kk as f64 * l;
            let base = pos.floor();
            (base as usize, pos - base)
        })
        .take_while(|&(base, _)| base.checked_add(width).is_some_and(|e| e < p.len()))
        .collect();
    // Blocks of bins in parallel; each bin sums the periods in the same order as a
    // single pass, so the result does not depend on the threads.
    let mut f = vec![Complex64::new(0.0, 0.0); width];
    let mut g = if keep { vec![0.0; width] } else { Vec::new() };
    if keep {
        f.par_chunks_mut(FOLD_BINS)
            .zip(g.par_chunks_mut(FOLD_BINS))
            .enumerate()
            .for_each(|(b, (fs, gs))| fold_block(p, &periods, b * FOLD_BINS, fs, Some(gs)));
    } else {
        f.par_chunks_mut(FOLD_BINS)
            .enumerate()
            .for_each(|(b, fs)| fold_block(p, &periods, b * FOLD_BINS, fs, None));
    }
    let mut w: Complex64 = f.iter().take(lg).sum();
    let mut best = w.norm();
    for (head, tail) in f.iter().skip(lg).zip(&f) {
        w += head - tail;
        best = best.max(w.norm());
    }
    (best, if keep { f } else { Vec::new() }, g)
}

/// Adds every period of `p` (start, interpolation weight) into the bins `j0..` held
/// by `fs`, and the magnitudes into `gs` when given, in one pass over the periods.
/// Each bin sums the periods in order, so the result does not depend on how the bins
/// are split between threads.
fn fold_block(
    p: &[Complex32],
    periods: &[(usize, f64)],
    j0: usize,
    fs: &mut [Complex64],
    mut gs: Option<&mut [f64]>,
) {
    for &(base, mu) in periods {
        let Some(seg) = p.get(base + j0..=base + j0 + fs.len()) else {
            continue;
        };
        let pairs = seg.iter().zip(seg.iter().skip(1));
        match gs.as_deref_mut() {
            Some(gs) => {
                for ((a, m), (u, v)) in fs.iter_mut().zip(gs.iter_mut()).zip(pairs) {
                    *a += c64(*u) * (1.0 - mu) + c64(*v) * mu;
                    *m += f64::from(u.norm()) * (1.0 - mu) + f64::from(v.norm()) * mu;
                }
            }
            None => {
                for (a, (u, v)) in fs.iter_mut().zip(pairs) {
                    *a += c64(*u) * (1.0 - mu) + c64(*v) * mu;
                }
            }
        }
    }
}

/// Hierarchical search for the fractional period: `block` symbols over LS +/- 0.6,
/// then 4 times as many symbols per stage over a range matched to the previous
/// precision, until `k_total` symbols are used. Returns the period and the symbols of
/// the last stage.
fn search_period(
    p: &[Complex32],
    ls0: usize,
    lg: usize,
    width: usize,
    (block, k_total): (usize, usize),
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<(f64, usize)> {
    let mut l = ls0 as f64;
    let mut k = block.min(k_total);
    let mut half = 0.6_f64;
    loop {
        check_stop(should_stop)?;
        let peak_width = (lg as f64 / k as f64).max(1e-9);
        let step = (half / 10.0).min(peak_width / 4.0);
        let n = ((half / step).ceil() as i64).min(400);
        let grid: Vec<f64> = (-n..=n).map(|i| l + i as f64 * step).collect();
        let peaks: Vec<f64> = grid
            .par_iter()
            .map(|&li| fold_frac(p, li, k, width, lg, false).0)
            .collect();
        let best = peaks
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map_or(0, |(i, _)| i);
        let centre = grid.get(best).copied().unwrap_or(l);
        l = golden_max(
            |li| fold_frac(p, li, k, width, lg, false).0,
            centre - step,
            centre + step,
            step * 1e-3,
        );
        if k >= k_total {
            return Ok((l, k));
        }
        half = peak_width / 2.0;
        k = (k * 4).min(k_total);
    }
}

/// Refine LS, LU and the frequency offset from a [`CtResult`] (SA: Get Br).
pub fn get_br(
    bb: &Baseband,
    ct: &CtResult,
    params: &BrParams,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<BrEstimate> {
    let fs = bb.sample_rate();
    let x = bb.samples();
    let (ls0, lu0, lg) = (ct.ls, ct.lu, ct.lg);
    if lu0 == 0 || ls0 <= lu0 || lg == 0 {
        return Err(invalid(format!(
            "LS {ls0} / LU {lu0} is not a usable triangle; run Find CT first"
        )));
    }
    if params.block < 3 {
        return Err(invalid(
            "the block must hold at least 3 symbols (20-40 is a good choice)",
        ));
    }
    // Fold origin a little before a CP so the run never leaves the fold window (skip
    // the first symbol when its CP starts too close to the beginning).
    let lead = lg / 2 + 2;
    let first = if ct.start >= lead {
        ct.start
    } else {
        ct.start + ls0
    };
    let origin = first - lead;
    let p = lag_product(x, lu0, origin, x.len());
    let width = ls0 + 2 * lead;
    let k_total = (p.len().saturating_sub(width) as f64 / (ls0 as f64 + 0.6)).floor() as usize;
    let k_total = k_total.min(params.max_symbols.max(params.block));
    if k_total < params.block.clamp(3, 8) {
        return Err(invalid(format!(
            "only {k_total} symbols fit after the triangle start; Get Br needs at least {} \
             (select a longer span or reduce the block)",
            params.block.clamp(3, 8)
        )));
    }

    // 1. Hierarchical fractional-period search.
    let (l, k) = search_period(&p, ls0, lg, width, (params.block, k_total), should_stop)?;

    // 2. Triangle at the final period: start and coherence.
    check_stop(should_stop)?;
    let (_, f, g) = fold_frac(&p, l, k, width, lg, true);
    let sums: Vec<f64> = sliding_sums(&f, lg).iter().map(|c| c.norm()).collect();
    let norms = sliding_sums(&g, lg);
    let (m, peak) = sums
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or((0, 0.0), |(i, v)| (i, *v));
    let at = |i: Option<usize>| i.and_then(|i| sums.get(i)).copied().unwrap_or(peak);
    // The window sums form a triangle (two straight slopes) around the peak.
    let frac = triangle_vertex(at(m.checked_sub(1)), peak, at(m.checked_add(1)));
    let norm = norms.get(m).copied().unwrap_or(0.0);
    let coherence = if norm > 0.0 { peak / norm } else { 0.0 };
    let mut start = origin as f64 + m as f64 + frac;
    // Reduce to the first complete symbol; a start within half a sample of a whole
    // period is the next symbol's start rounded down, so it belongs at about 0.
    start -= (start / l).floor() * l;
    if start > l - 0.5 {
        start = (start - l).max(0.0);
    }

    // 3. Fractional LU and the frequency offset.
    check_stop(should_stop)?;
    let (lu, corr) = refine_lu(x, (start, l), lu0, lg, k_total, should_stop)?;
    let lu_int = lu.round().max(1.0) as usize;
    let freq_offset = -corr.arg() / std::f64::consts::TAU * fs / lu_int as f64;
    Ok(BrEstimate {
        sample_rate: fs,
        ls: l,
        br: fs / l,
        lu,
        lg: l - lu,
        k: (l - lu) / lu,
        spacing: fs / lu,
        freq_offset,
        start,
        symbols: k,
        coherence,
        lu_int,
        lg_int: (l.round() as usize).saturating_sub(lu_int),
    })
}

/// Sums over the CP positions at one lag, split by symbol parity (index 0 even, 1 odd):
/// the CP correlation `c`, the CP and partner energies `ex` and `ey` and the number
/// of positions.
#[derive(Debug, Clone, Copy, Default)]
struct LagSums {
    c: [Complex64; 2],
    ex: [f64; 2],
    ey: [f64; 2],
    n: [f64; 2],
}

impl LagSums {
    /// Adds the CP sample `a` and its partner `y` to the sums of its parity.
    fn push(&mut self, odd: bool, a: Complex64, y: Complex64) {
        let ([c0, c1], [x0, x1], [y0, y1], [n0, n1]) =
            (&mut self.c, &mut self.ex, &mut self.ey, &mut self.n);
        let (c, ex, ey, n) = if odd {
            (c1, x1, y1, n1)
        } else {
            (c0, x0, y0, n0)
        };
        *c += a * y.conj();
        *ex += a.norm_sqr();
        *ey += y.norm_sqr();
        *n += 1.0;
    }

    /// `Re(c_even conj(c_odd)) / sqrt(Ex_even Ex_odd Ey_even Ey_odd)` with the noise
    /// floor `n sigma2 gain` taken off each partner energy.
    fn score(&self, sigma2: f64, gain: f64) -> f64 {
        let [c0, c1] = self.c;
        let e0 = self.ey[0] - self.n[0] * sigma2 * gain;
        let e1 = self.ey[1] - self.n[1] * sigma2 * gain;
        let d = (self.ex[0] * self.ex[1] * e0.max(0.0) * e1.max(0.0)).sqrt();
        if d > 0.0 {
            (c0 * c1.conj()).re / d
        } else {
            0.0
        }
    }
}

impl std::ops::AddAssign<&LagSums> for LagSums {
    fn add_assign(&mut self, o: &LagSums) {
        for (a, b) in self.c.iter_mut().zip(&o.c) {
            *a += b;
        }
        for (a, b) in self.ex.iter_mut().zip(&o.ex) {
            *a += b;
        }
        for (a, b) in self.ey.iter_mut().zip(&o.ey) {
            *a += b;
        }
        for (a, b) in self.n.iter_mut().zip(&o.n) {
            *a += b;
        }
    }
}

/// [`LagSums`] at the fractional lag `lambda`, the partner interpolated with
/// [`taps`] built once for the whole pass. Also returns the taps' noise gain
/// `sum h^2`.
fn lag_sums(x: &[Complex32], positions: &[(usize, bool)], lambda: f64) -> (LagSums, f64) {
    let base = lambda.floor();
    let h = taps(lambda - base);
    let gain: f64 = h.iter().map(|t| t * t).sum();
    let off = base as i64 + 1 - TAP_HALF as i64;
    // A whole lag (the fraction is exactly 0).
    let whole = lambda - base <= 0.0;
    // Fixed blocks of positions in parallel, their partial sums added in block order
    // (deterministic, independent of the thread count).
    let parts: Vec<LagSums> = positions
        .par_chunks(LAG_BLOCK)
        .map(|block| {
            let mut s = LagSums::default();
            for &(n, odd) in block {
                let (Some(a), Some(seg)) = (
                    x.get(n),
                    usize::try_from(n as i64 + off)
                        .ok()
                        .and_then(|m| x.get(m..m + TAPS)),
                ) else {
                    continue;
                };
                // At a whole lag the interpolator is a unit impulse (to rounding):
                // take the sample itself.
                let y: Complex64 = if whole {
                    seg.get(TAP_HALF - 1)
                        .copied()
                        .map_or_else(Complex64::default, c64)
                } else {
                    h.iter().zip(seg).map(|(t, v)| c64(*v) * *t).sum()
                };
                s.push(odd, c64(*a), y);
            }
            s
        })
        .collect();
    let mut s = LagSums::default();
    for p in &parts {
        s += p;
    }
    (s, gain)
}

/// Maximum of `score` within a sample of the whole lag `peak` (and inside
/// `bounds`): a grid of eighth samples, then golden-section search.
fn fine_search(
    score: impl Fn(f64) -> Result<f64> + Sync,
    peak: usize,
    bounds: (f64, f64),
) -> Result<f64> {
    let a = (peak as f64 - 1.0).max(bounds.0);
    let top = (peak as f64 + 1.0).min(bounds.1);
    let step = 0.125;
    let n = ((top - a) / step).floor().max(0.0) as usize;
    let grid: Vec<(f64, f64)> = (0..=n)
        .into_par_iter()
        .map(|i| {
            let lambda = a + i as f64 * step;
            Ok((lambda, score(lambda)?))
        })
        .collect::<Result<_>>()?;
    let coarse = grid
        .iter()
        .max_by(|u, v| u.1.total_cmp(&v.1))
        .map_or(peak as f64, |g| g.0);
    let mut failed = None;
    let best = golden_max(
        |lambda| match score(lambda) {
            Ok(v) => v,
            Err(e) => {
                failed = Some(e);
                f64::NEG_INFINITY
            }
        },
        (coarse - step).max(a),
        (coarse + step).min(top),
        1e-7,
    );
    failed.map_or(Ok(best), Err)
}

/// Fractional LU maximising the split-symbol CP correlation (see the module docs),
/// and the CP correlation at that LU rounded to whole samples. `(start, l)` is the
/// symbol grid, `lu0` Find CT's LU and `lg` its LG.
fn refine_lu(
    x: &[Complex32],
    (start, l): (f64, f64),
    lu0: usize,
    lg: usize,
    symbols: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<(f64, Complex64)> {
    // Lags searched: Find CT's LU +/- LG/2, at least LS/2 (K <= 1) and leaving at
    // least half a sample of guard.
    let w = (lg / 2).clamp(3, 128);
    let lo_phys = ((l / 2.0).ceil() as usize).max(1);
    let hi_phys = (l - 0.5).floor().max(0.0) as usize;
    let lo = lu0.saturating_sub(w).max(lo_phys);
    let hi = lu0.saturating_add(w).min(hi_phys);
    if lo >= hi {
        return Err(invalid(format!(
            "LU {lu0} does not fit a symbol of {l:.2} samples; run Find CT again"
        )));
    }
    let reach_lo = lo as i64 - TAP_HALF as i64 - 2;
    let reach_hi = hi as i64 + TAP_HALF as i64 + 2;
    let margin = if lg >= 4 { (lg / 4).max(1) } else { 0 };
    let core = lg.saturating_sub(2 * margin).max(1);
    let symbols = symbols.min((POSITIONS / core).max(2));
    // (position, odd symbol), every interpolator reach inside the data.
    let positions: Vec<(usize, bool)> = (0..symbols)
        .flat_map(|k| {
            let base = (start + k as f64 * l).round().max(0.0) as usize;
            (margin..margin + core).map(move |i| (base + i, k % 2 == 1))
        })
        .filter(|&(n, _)| n as i64 + reach_lo >= 0 && n as i64 + reach_hi < x.len() as i64)
        .collect();
    let odd = positions.iter().filter(|p| p.1).count();
    if odd < 4 || positions.len() - odd < 4 {
        return Err(Error::NotFound(
            "too few CP samples fit in the span to measure LU; select a longer span".into(),
        ));
    }
    let eval = |lambda: f64| -> Result<(LagSums, f64)> {
        check_stop(should_stop)?;
        Ok(lag_sums(x, &positions, lambda))
    };
    // Whole lags first: no interpolation, so the noise floor is the same at each.
    let ints: Vec<(usize, LagSums)> = (lo..=hi)
        .into_par_iter()
        .map(|lag| Ok((lag, eval(lag as f64)?.0)))
        .collect::<Result<_>>()?;
    let (peak, at_peak) = ints
        .iter()
        .max_by(|a, b| a.1.score(0.0, 1.0).total_cmp(&b.1.score(0.0, 1.0)))
        .copied()
        .unwrap_or((lu0, LagSums::default()));
    if (peak == lo && lo > lo_phys) || (peak == hi && hi < hi_phys) {
        return Err(Error::NotFound(format!(
            "the CP correlation keeps rising toward lag {peak}, the edge of the LU search \
             around Find CT's LU {lu0}; Find CT's LU looks wrong. Check the Find CT result \
             (other Br bounds, more symbols) and run Get Br again"
        )));
    }
    let rho = at_peak.score(0.0, 1.0);
    if rho <= 0.0 {
        return Err(Error::NotFound(format!(
            "the CP correlation near LU {lu0} is not consistent between symbols; the span \
             may hold too little OFDM. Select a longer span or check the Find CT result"
        )));
    }
    // Noise power per sample. At a whole lag the interpolator passes all of the noise,
    // and rho is about the squared fraction of the power that is signal.
    let sigma2 = {
        let n: f64 = at_peak.n.iter().sum();
        let ey: f64 = at_peak.ey.iter().sum();
        (1.0 - rho.min(1.0).sqrt()) * ey / n.max(1.0)
    };
    let lu = fine_search(
        |lambda| {
            let (s, gain) = eval(lambda)?;
            Ok(s.score(sigma2, gain))
        },
        peak,
        (lo as f64, (hi as f64).min(l - 0.5)),
    )?;
    // The offset is measured at the whole-sample lag that the LU spectrum uses, so the
    // channels land on that FFT grid.
    let lag = lu.round().max(1.0) as i64;
    let at_lag = ints.iter().find(|(l, _)| *l as i64 == lag).map_or_else(
        || {
            let s = lag_sums(x, &positions, lag as f64).0;
            s.c[0] + s.c[1]
        },
        |(_, s)| s.c[0] + s.c[1],
    );
    Ok((lu, at_lag))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use sn_synth::{OfdmModulation, OfdmParams, ofdm};

    use super::*;

    /// Narrow-band OFDM (8 of 64 channels, LG 16) at its native rate, symbol 0 at 0.
    fn narrow() -> Vec<Complex32> {
        let p = OfdmParams {
            sample_rate: 8000.0,
            lu: 64,
            lg: 16,
            channels: 8,
            first_bin: -4,
            unused: vec![],
            modulation: OfdmModulation::Qpsk,
            overrides: vec![],
            symbols: 200,
            frequency_offset: 0.0,
            amplitude: 0.3,
            seed: 2,
        };
        let s = ofdm(&p).unwrap().signal;
        let mut x = vec![Complex32::new(0.0, 0.0); s.len()];
        s.read_complex(0, &mut x);
        x
    }

    #[test]
    fn whole_lag_shortcut_matches_the_interpolator() {
        // The shortcut reads `seg[TAP_HALF - 1]`: the interpolator at fraction 0 must
        // be a unit impulse there.
        let h = taps(0.0);
        let peak = h
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i);
        assert_eq!(peak, Some(TAP_HALF - 1));
        assert!((h[TAP_HALF - 1] - 1.0).abs() < 1e-12);
        // And the sums at whole lags match the interpolated path just off them.
        let x = narrow();
        let pos: Vec<(usize, bool)> = (100..x.len() - 200)
            .step_by(3)
            .map(|n| (n, (n / 80) % 2 == 1))
            .collect();
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1e-30);
        for lag in [1usize, 17, 63, 64, 65, 80] {
            let (w, _) = lag_sums(&x, &pos, lag as f64);
            let (f, _) = lag_sums(&x, &pos, lag as f64 + 1e-12);
            for i in 0..2 {
                assert!(close(w.c[i].re, f.c[i].re), "lag {lag} c.re");
                assert!(close(w.c[i].im, f.c[i].im), "lag {lag} c.im");
                assert!(close(w.ex[i], f.ex[i]), "lag {lag} ex");
                assert!(close(w.ey[i], f.ey[i]), "lag {lag} ey");
                assert_eq!(w.n[i], f.n[i], "lag {lag} n");
            }
        }
    }

    #[test]
    fn refine_lu_finds_lu_within_half_lg() {
        let (lu, _) = refine_lu(&narrow(), (0.0, 80.0), 67, 16, 150, &|| false).unwrap();
        assert!((lu - 64.0).abs() < 1e-3, "LU {lu}");
    }

    #[test]
    fn refine_lu_reports_a_peak_at_the_edge() {
        // Find CT's LU 72 with LG 8: the search covers 68..=76 and the correlation
        // keeps rising toward the true LU 64.
        let r = refine_lu(&narrow(), (0.0, 80.0), 72, 8, 150, &|| false);
        assert!(matches!(r, Err(Error::NotFound(_))), "{r:?}");
    }

    #[test]
    fn refine_lu_can_be_cancelled() {
        let r = refine_lu(&narrow(), (0.0, 80.0), 64, 16, 150, &|| true);
        assert!(matches!(r, Err(Error::Cancelled)), "{r:?}");
    }
}
