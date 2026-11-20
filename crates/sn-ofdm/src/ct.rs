//! The correlation triangle (CT) and the LS/LU/LG search (SA: Find CT).
//!
//! Method:
//! 1. LU from the autocorrelation: OFDM with a CP has an ACF side peak near lag LU, of
//!    height about LG/LS. Up to six local maxima between half the shortest and the
//!    longest LS that reach half the strongest one are kept, each with its neighbours
//!    +/-1, as LU candidates. With few channels the side peak sits a few lags below
//!    LU (the CP part is skewed by the broad main lobe), which step 4 corrects.
//! 2. For each LU candidate, `p[n] = x[n] * conj(x[n + LU])` is real and positive
//!    inside every CP and random elsewhere. Folding `p` at a candidate period LS over
//!    `symbols` periods and summing it over sliding windows of `LG = LS - LU` gives
//!    the triangle: its peak sits at the symbol start, its base is `2 LG` wide. LS
//!    runs from LU + 1 to about 2 LU: LU >= LS/2 means K <= 1 (the largest K in the
//!    report is 7/8); a little slack covers an LU candidate that sits low.
//! 3. Each (LU, LS) pair is scored by its triangle peak divided by the noise level of
//!    the same sum (a detection statistic, so a short window cannot win by chance).
//!    The best pair gives LS.
//! 4. The detection score grows with the window, so on an oversampled signal (x[n]
//!    and x[n-1] nearly equal) the pair (LS, LU - 1) can outscore the true one. LU is
//!    therefore chosen again for the winning LS, over every lag from 2 below to LG/2
//!    above the winner, by the "coherence" (peak over what a perfect CP match would
//!    give, 0..1): it reaches 1 only at the exact LU, because a window one sample too
//!    long holds a sample that does not match and one too short compares samples a
//!    lag apart. LG is `LS - LU`, cross-checked against the width of the coherent run
//!    in the folded profile.
//! 5. The margin compares the winner with the best pair at another LS, leaving out
//!    multiples and fractions of the winning LS (2 LS, 3 LS, LS/2, LS/3), which fold
//!    the same CPs and are not a rival answer.

use num_complex::Complex64;
use rayon::prelude::*;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};
use sn_core::Complex32;

use crate::util::{argmax, c64, lag_product, window_sums};
use crate::{Baseband, Error, Result, check_stop, invalid};

/// Longest symbol searched, in samples (SA went to about 32000).
pub const MAX_LS: usize = 1 << 16;
/// Longest span used for the ACF, in samples.
const ACF_SPAN: usize = 1 << 19;
/// Candidate pairs kept in the result list.
const LIST_LEN: usize = 10;

/// Settings for [`find_ct`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CtParams {
    /// Lowest symbol rate to consider, in Bd (SA: MinBr). Sets the longest LS.
    pub min_br: f64,
    /// Highest symbol rate to consider, in Bd (SA: MaxBr). Sets the shortest LS.
    pub max_br: f64,
    /// Symbols folded into the triangle (SA: Symbols, default 40). More symbols give a
    /// cleaner triangle at the native rate but smear it when LS is fractional.
    pub symbols: usize,
    /// First sample of the search, as an index into the [`Baseband`].
    pub start: usize,
}

impl CtParams {
    /// Bounds with the default of 40 symbols, starting at sample 0.
    pub fn new(min_br: f64, max_br: f64) -> Self {
        Self {
            min_br,
            max_br,
            symbols: 40,
            start: 0,
        }
    }
}

/// One correlation triangle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Triangle {
    /// Symbol length in samples.
    pub ls: usize,
    /// Useful length in samples.
    pub lu: usize,
    /// Guard length `ls - lu` in samples.
    pub lg: usize,
    /// Baseband index that `values[0]` refers to.
    pub origin: usize,
    /// `values[m]`: CP correlation magnitude for a symbol starting at `origin + m`
    /// (`ls` values), scaled so the peak equals the coherence.
    pub values: Vec<f32>,
    /// The same with polarity: the real part, normalised the same way. It is negative
    /// when the channels sit half a grid step off (report 12.3).
    pub signed: Vec<f32>,
    /// Index of the peak in `values` (the symbol start is `origin + peak`).
    pub peak: usize,
    /// Peak height, 0..1: the coherent CP correlation divided by the sum of the
    /// magnitudes it is made of. Noise-free OFDM at its native rate gives 1.
    pub coherence: f64,
    /// Phase of the CP correlation at the peak, in radians.
    pub phase: f64,
    /// Symbols folded.
    pub symbols: usize,
    /// Width in samples of the coherent run in the folded profile: a measurement of LG
    /// that does not use `ls - lu`.
    pub lg_width: usize,
}

/// One (LS, LU) pair from the search.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CtCandidate {
    /// Symbol length.
    pub ls: usize,
    /// Useful length.
    pub lu: usize,
    /// Guard length.
    pub lg: usize,
    /// Triangle coherence (0..1).
    pub coherence: f64,
    /// Detection score: the coherent CP sum over its noise level. It grows with the
    /// square root of the folded CP samples; noise gives about 1 to 4.
    pub score: f64,
}

/// Result of [`find_ct`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CtResult {
    /// Sample rate of the analysed data.
    pub sample_rate: f64,
    /// Symbol length in samples.
    pub ls: usize,
    /// Useful length in samples.
    pub lu: usize,
    /// Guard length in samples.
    pub lg: usize,
    /// Start of the first complete symbol (its CP), as a baseband index.
    pub start: usize,
    /// The triangle for the winning pair (starting at the search start).
    pub triangle: Triangle,
    /// Best score divided by the best score at an LS more than 2 samples away (not
    /// counting multiples or fractions of LS). Above about 2 the answer is
    /// unambiguous.
    pub margin: f64,
    /// Confidence 0..1: the coherence, reduced when the margin is small or the measured
    /// triangle width disagrees with `ls - lu`.
    pub confidence: f64,
    /// The ACF side-peak height at LU (about LG/LS for clean OFDM).
    pub acf_peak: f64,
    /// Best pairs, strongest first (one entry per LS).
    pub candidates: Vec<CtCandidate>,
}

impl CtResult {
    /// Symbol rate `fs / ls` at integer LS (use [`crate::get_br`] for precision).
    pub fn br(&self) -> f64 {
        self.sample_rate / self.ls as f64
    }

    /// `K = LG / LU`.
    pub fn k(&self) -> f64 {
        self.lg as f64 / self.lu.max(1) as f64
    }
}

/// `p` folded at a period: the coherent sum, the sum of `|p|` and of `|p|^2` per
/// position.
struct Folded {
    f: Vec<Complex64>,
    g: Vec<f64>,
    q: Vec<f64>,
    /// Mean of `p` over everything folded.
    mean: Complex64,
    /// Periods folded.
    k: usize,
}

/// Fold `p` at period `ls` over `k` periods.
fn fold(p: &[Complex32], ls: usize, k: usize) -> Folded {
    let mut f = vec![Complex64::new(0.0, 0.0); ls];
    let mut g = vec![0.0; ls];
    let mut q = vec![0.0; ls];
    let mut folded = 0;
    for chunk in p.chunks_exact(ls).take(k) {
        folded += 1;
        for (((a, b), c), v) in f.iter_mut().zip(g.iter_mut()).zip(q.iter_mut()).zip(chunk) {
            *a += c64(*v);
            let m = f64::from(v.norm_sqr());
            *b += m.sqrt();
            *c += m;
        }
    }
    let total: Complex64 = f.iter().sum();
    let n = (folded * ls).max(1) as f64;
    Folded {
        f,
        g,
        q,
        mean: total / n,
        k: folded,
    }
}

/// Peak of the triangle (window sums over `lg`) of a folded profile.
#[derive(Debug, Clone, Copy)]
struct Peak {
    index: usize,
    /// Coherent sum over sum of magnitudes: 1 for a perfect CP match.
    coherence: f64,
    /// Coherent sum, less its expectation at a wrong period (the mean of `p` times the
    /// window size), over the root of the summed squared magnitudes: a detection
    /// statistic that grows as sqrt(symbols * LG) for a real CP and stays near 1 for
    /// noise, so neither short nor long windows win by chance.
    z: f64,
}

fn real_window_sums(v: &[f64], lg: usize) -> Vec<f64> {
    let c: Vec<Complex64> = v.iter().map(|x| Complex64::new(*x, 0.0)).collect();
    window_sums(&c, lg).iter().map(|c| c.re).collect()
}

/// The triangle (window sums of the profile) and its peak.
fn triangle_peak(fd: &Folded, lg: usize) -> (Vec<Complex64>, Vec<f64>, Peak) {
    let w = window_sums(&fd.f, lg);
    let gw = real_window_sums(&fd.g, lg);
    let qw = real_window_sums(&fd.q, lg);
    let bias = fd.mean * (fd.k * lg) as f64;
    let peak = match argmax(&w, |c| (c - bias).norm()) {
        Some((i, v)) => {
            let n = gw.get(i).copied().unwrap_or(0.0);
            let e = qw.get(i).copied().unwrap_or(0.0);
            let c = w.get(i).map_or(0.0, |c| c.norm());
            Peak {
                index: i,
                coherence: if n > 0.0 { c / n } else { 0.0 },
                z: if e > 0.0 { v / e.sqrt() } else { 0.0 },
            }
        }
        None => Peak {
            index: 0,
            coherence: 0.0,
            z: 0.0,
        },
    };
    (w, gw, peak)
}

/// Width of the coherent run around the triangle peak (folded profile above half the
/// mean level of the run).
fn run_width(f: &[Complex64], peak: usize, lg: usize) -> usize {
    let ls = f.len();
    if ls == 0 || lg == 0 {
        return 0;
    }
    let mag = |i: usize| f.get(i % ls).map_or(0.0, |c| c.norm());
    let mean = (0..lg).map(|i| mag(peak + i)).sum::<f64>() / lg as f64;
    let thr = 0.5 * mean;
    let centre = peak + lg / 2;
    if mag(centre) < thr {
        return 0;
    }
    let mut width = 1;
    let mut i = 1;
    while width < ls && mag(centre + i) >= thr {
        width += 1;
        i += 1;
    }
    let mut j = 1;
    while width < ls && mag(centre + ls - j) >= thr {
        width += 1;
        j += 1;
    }
    width
}

/// The triangle for one (LS, LU) pair, folding `symbols` symbols from baseband index
/// `origin`. Use it for SA's quick recompute and for stepping by whole symbols
/// (`origin + j * ls`): at the native rate the peak does not move.
pub fn ct_triangle(
    bb: &Baseband,
    ls: usize,
    lu: usize,
    origin: usize,
    symbols: usize,
) -> Result<Triangle> {
    if lu == 0 || ls <= lu || ls > MAX_LS {
        return Err(invalid(format!(
            "LS {ls} and LU {lu} are not a valid pair: LU must be at least 1 and below LS, \
             and LS at most {MAX_LS}"
        )));
    }
    let x = bb.samples();
    let avail = x.len().saturating_sub(origin).saturating_sub(lu) / ls;
    let k = symbols.min(avail);
    if k == 0 {
        return Err(invalid(format!(
            "no complete symbol of {ls} samples (plus LU {lu}) fits after sample {origin}; \
             move the start back or select a longer span"
        )));
    }
    let p = lag_product(x, lu, origin, origin + k * ls);
    Ok(triangle_from(&p, ls, lu, origin, k))
}

fn triangle_from(p: &[Complex32], ls: usize, lu: usize, origin: usize, k: usize) -> Triangle {
    let lg = ls - lu;
    let fd = fold(p, ls, k);
    let (w, gw, pk) = triangle_peak(&fd, lg);
    let (peak, coherence) = (pk.index, pk.coherence);
    let norm = gw.get(peak).copied().unwrap_or(0.0);
    let scale = if norm > 0.0 { 1.0 / norm } else { 0.0 };
    let phase = w.get(peak).map_or(0.0, |c| c.arg());
    Triangle {
        ls,
        lu,
        lg,
        origin,
        values: w.iter().map(|c| (c.norm() * scale) as f32).collect(),
        signed: w.iter().map(|c| (c.re * scale) as f32).collect(),
        peak,
        coherence,
        phase,
        symbols: k,
        lg_width: run_width(&fd.f, peak, lg),
    }
}

/// Normalised ACF magnitude for lags `0..max_lag` of `x` (unbiased: each lag is
/// divided by the number of overlapping samples).
fn acf_magnitude(x: &[Complex32], max_lag: usize) -> Vec<f64> {
    let n = x.len();
    let size = (n + max_lag).next_power_of_two();
    let mut buf: Vec<Complex64> = x.iter().map(|v| c64(*v)).collect();
    buf.resize(size, Complex64::new(0.0, 0.0));
    let mut planner = FftPlanner::<f64>::new();
    planner.plan_fft_forward(size).process(&mut buf);
    for b in &mut buf {
        *b = Complex64::new(b.norm_sqr(), 0.0);
    }
    planner.plan_fft_inverse(size).process(&mut buf);
    let r0 = buf.first().map_or(0.0, |c| c.re / n as f64);
    if r0 <= 0.0 {
        return vec![0.0; max_lag];
    }
    buf.iter()
        .take(max_lag.min(n))
        .enumerate()
        .map(|(lag, c)| c.norm() / (n - lag) as f64 / r0)
        .collect()
}

/// Up to `count` local maxima of `v` within `lo..hi`, strongest first.
fn local_maxima(v: &[f64], lo: usize, hi: usize, count: usize) -> Vec<(usize, f64)> {
    let at = |i: usize| v.get(i).copied().unwrap_or(0.0);
    let mut peaks: Vec<(usize, f64)> = (lo.max(1)..hi.min(v.len().saturating_sub(1)))
        .filter(|&i| at(i) >= at(i - 1) && at(i) > at(i + 1))
        .map(|i| (i, at(i)))
        .collect();
    peaks.sort_by(|a, b| b.1.total_cmp(&a.1));
    peaks.truncate(count);
    peaks
}

/// Shortest and longest LS for the Br bounds, after checking them.
fn ls_bounds(fs: f64, params: &CtParams) -> Result<(usize, usize)> {
    let (lo_br, hi_br) = (params.min_br, params.max_br);
    if !(lo_br.is_finite() && hi_br.is_finite() && lo_br > 0.0 && hi_br > lo_br) {
        return Err(invalid(format!(
            "symbol-rate bounds {lo_br}..{hi_br} Bd are invalid; set MinBr above 0 and \
             MaxBr above MinBr"
        )));
    }
    if params.symbols < 3 {
        return Err(invalid(
            "fold at least 3 symbols (SA's default is 40; do not go below 6-8)",
        ));
    }
    let ls_min = ((fs / hi_br).floor() as usize).max(3);
    let ls_max = (fs / lo_br).ceil() as usize;
    if ls_max > MAX_LS {
        return Err(invalid(format!(
            "MinBr {lo_br} Bd means symbols of up to {ls_max} samples at {fs} Hz; the limit \
             is {MAX_LS}. Raise MinBr or resample to a lower rate first"
        )));
    }
    if ls_min >= ls_max {
        return Err(invalid(format!(
            "the bounds {lo_br}..{hi_br} Bd leave no whole symbol length at {fs} Hz; widen them"
        )));
    }
    Ok((ls_min, ls_max))
}

/// ACF side peaks kept as LU candidates.
const LU_PEAKS: usize = 6;

/// LU candidates: up to [`LU_PEAKS`] ACF side peaks that reach half the strongest one,
/// and their neighbours.
fn lu_candidates(acf: &[f64], ls_min: usize, ls_max: usize) -> Result<Vec<usize>> {
    let peaks = local_maxima(acf, (ls_min / 2).max(2), ls_max, LU_PEAKS);
    let strongest = peaks.first().map_or(0.0, |p| p.1);
    let mut lus: Vec<usize> = Vec::new();
    for (lag, _) in peaks.iter().filter(|p| p.1 >= 0.5 * strongest) {
        for lu in [lag.saturating_sub(1), *lag, lag + 1] {
            if lu >= 1 && lu < ls_max && !lus.contains(&lu) {
                lus.push(lu);
            }
        }
    }
    if lus.is_empty() {
        return Err(Error::NotFound(
            "the ACF shows no side peak between half the shortest and the longest symbol, so \
             there is no cyclic prefix in range; check the Br bounds and that the span holds \
             OFDM"
                .into(),
        ));
    }
    Ok(lus)
}

/// Longest LS tried with `lu`: K <= 1, plus slack for an LU candidate that sits low.
fn ls_limit(lu: usize) -> usize {
    2 * lu + lu / 8 + 2
}

/// One lag product per LU candidate.
type Products = Vec<(usize, Vec<Complex32>)>;

/// Score every (LU, LS) pair in parallel.
fn score_pairs(
    products: &Products,
    ls_range: (usize, usize),
    k: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<CtCandidate>> {
    let pairs: Vec<(usize, usize)> = products
        .iter()
        .enumerate()
        .flat_map(|(i, (lu, _))| {
            (ls_range.0.max(lu + 1)..=ls_range.1.min(ls_limit(*lu))).map(move |ls| (i, ls))
        })
        .collect();
    Ok(pairs
        .par_iter()
        .map(|&(i, ls)| -> Result<Option<CtCandidate>> {
            check_stop(should_stop)?;
            let Some((lu, p)) = products.get(i) else {
                return Ok(None);
            };
            Ok(Some(score_pair(p, ls, *lu, k)))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect())
}

/// Score of one (LS, LU) pair from its lag product.
fn score_pair(p: &[Complex32], ls: usize, lu: usize, k: usize) -> CtCandidate {
    let lg = ls - lu;
    let (_, _, pk) = triangle_peak(&fold(p, ls, k), lg);
    CtCandidate {
        ls,
        lu,
        lg,
        coherence: pk.coherence,
        score: pk.z,
    }
}

/// Step 4: LU chosen again for the winning LS by coherence, from 2 below the winner
/// to LG/2 above it. Returns the pair and its lag product.
fn refine_pair(
    x: &[Complex32],
    (start, k): (usize, usize),
    best: CtCandidate,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<(CtCandidate, Vec<Complex32>)> {
    let ls = best.ls;
    let lo = best.lu.saturating_sub(2).max(ls.div_ceil(2)).max(1);
    let hi = (best.lu + best.lg / 2 + 2).min(ls - 1);
    let tried: Vec<(CtCandidate, Vec<Complex32>)> = (lo..=hi)
        .into_par_iter()
        .map(|lu| -> Result<(CtCandidate, Vec<Complex32>)> {
            check_stop(should_stop)?;
            let p = lag_product(x, lu, start, start + k * ls);
            Ok((score_pair(&p, ls, lu, k), p))
        })
        .collect::<Result<_>>()?;
    tried
        .into_iter()
        .max_by(|a, b| a.0.coherence.total_cmp(&b.0.coherence))
        .ok_or_else(|| Error::NotFound(format!("no LU fits a symbol of {ls} samples")))
}

/// True when `ls` is within 2 samples of 2 or 3 times `of`, or of a half or a third.
fn harmonic(ls: usize, of: usize) -> bool {
    (2..=3).any(|m| ls.abs_diff(m * of) <= 2 || ls.abs_diff(of / m) <= 2)
}

/// The strongest pairs, one per LS.
fn short_list(mut scored: Vec<CtCandidate>) -> Vec<CtCandidate> {
    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut list: Vec<CtCandidate> = Vec::new();
    for c in scored {
        if list.len() >= LIST_LEN {
            break;
        }
        if !list.iter().any(|l| l.ls == c.ls) {
            list.push(c);
        }
    }
    list
}

/// Search LS within the Br bounds and find LU, LG and the symbol start (SA: Find CT).
pub fn find_ct(
    bb: &Baseband,
    params: &CtParams,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<CtResult> {
    let fs = bb.sample_rate();
    let (ls_min, ls_max) = ls_bounds(fs, params)?;
    let x = bb.samples();
    let start = params.start;
    let after = x.len().saturating_sub(start);
    let k = params
        .symbols
        .min(after.saturating_sub(ls_max) / ls_max.max(1));
    if k < 3 {
        return Err(invalid(format!(
            "the span after the start holds {after} samples; Find CT needs at least 4 symbols \
             of the longest length ({} samples). Raise MinBr or select a longer span",
            4 * ls_max
        )));
    }

    // 1. LU candidates from the ACF side peak.
    check_stop(should_stop)?;
    let seg = x
        .get(start..start + after.min(ACF_SPAN))
        .unwrap_or_default();
    let acf = acf_magnitude(seg, ls_max);
    let lus = lu_candidates(&acf, ls_min, ls_max)?;

    // 2. Score every (LU, LS) pair.
    let products: Products = lus
        .iter()
        .map(|&lu| (lu, lag_product(x, lu, start, start + k * ls_max)))
        .collect();
    let scored = score_pairs(&products, (ls_min, ls_max), k, should_stop)?;
    let winner = scored
        .iter()
        .copied()
        .max_by(|a, b| a.score.total_cmp(&b.score))
        .ok_or_else(|| Error::NotFound("no LS/LU pair could be scored".into()))?;
    let second = scored
        .iter()
        .filter(|c| c.ls.abs_diff(winner.ls) > 2 && !harmonic(c.ls, winner.ls))
        .map(|c| c.score)
        .fold(0.0_f64, f64::max);
    let margin = if second > 0.0 {
        (winner.score / second).min(1e3)
    } else {
        1e3
    };

    // 3. LU again for the winning LS, by coherence, and the winning triangle.
    check_stop(should_stop)?;
    let (best, p) = refine_pair(x, (start, k), winner, should_stop)?;
    let triangle = triangle_from(&p, best.ls, best.lu, start, k);
    let mut list = vec![best];
    list.extend(short_list(scored).into_iter().filter(|c| c.ls != best.ls));
    list.truncate(LIST_LEN);
    let width_ok = triangle.lg_width.abs_diff(best.lg) as f64 <= (best.lg as f64 * 0.25).max(2.0);
    let confidence = (best.coherence.clamp(0.0, 1.0)
        * (1.0_f64 - 1.0 / margin.max(1.0)).clamp(0.0, 1.0)
        * if width_ok { 1.0 } else { 0.5 })
    .clamp(0.0, 1.0);
    let acf_peak = acf.get(best.lu).copied().unwrap_or(0.0);
    // First complete symbol: the peak position reduced to the first period.
    let first = start + triangle.peak % best.ls;
    Ok(CtResult {
        sample_rate: fs,
        ls: best.ls,
        lu: best.lu,
        lg: best.lg,
        start: first,
        triangle,
        margin,
        confidence,
        acf_peak,
        candidates: list,
    })
}
