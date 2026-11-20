//! The K calculator: List of K (stage 1), the orthogonality check (stage 2, new) and
//! the native-rate helpers.
//!
//! **Stage 1.** A native rate SR makes both LS and LU whole numbers. With a precise
//! symbol rate Br and the fractional LU and LS at the current rate Fd, every candidate
//! `SR = n * Br` for `n` from LS to 2 LS has `LS' = n` and `LU' = LU * n / LS`.
//! Candidates where LU' lands within `tolerance` of an integer are kept, and
//! `K = LG' / LU'` is reduced to a fraction. Every rate with the same K is
//! equivalent (report 12.3), so candidates are grouped by K and represented by the
//! lowest rate with an even LU (or the lowest rate when no LU is even). Groups are
//! ranked by how close LU' came to an integer, with a small penalty for odd LU.
//! Like SA's, this stage cannot separate K values whose LU' are all near-integers
//! (17/64 against 15/32): that is stage 2's job.
//!
//! **Stage 2 (orthogonality).** For each of the top candidates, the span is resampled
//! to SR, the triangle is found again at the candidate's LS/LU, the channels are put
//! on the grid with the CP phase, and LU windows of every symbol are transformed.
//! Two measurements are made:
//! - **Guard leakage** (`leakage_db`): at the true native rate the channels are
//!   orthogonal, so all energy sits in as many bins as there are channels; at a wrong
//!   rate it leaks into neighbouring bins. The score is the energy outside the
//!   strongest `N` bins relative to the energy in them, with `N` the smallest channel
//!   count seen over the candidates. Leakage between occupied channels stays inside
//!   the band, so this only sees the two band edges and is a weak test.
//! - **Window consistency** (`ici_db`, the ranking key): two LU windows of the same
//!   symbol, a quarter of LG in from each end of the good slope, hold whole periods of
//!   the symbol only at the true LU. Their spectra must then differ by a pure phase
//!   ramp across the bins. The mismatch energy relative to the window energy measures
//!   inter-carrier interference over the whole band, whatever the modulation.
//!
//! Limits of stage 2: band-limiting in the recording chain (filters, resampling, the
//! analytic filter for real input) cuts the sinc sidelobes of the rectangular symbols,
//! so even the true candidate keeps a floor of about -35 to -50 dB. K values that
//! differ by about 1e-3 or less (17/64 against 72/271) give native grids that are
//! orthogonal to within that floor, so stage 2 cannot separate them; it reliably
//! demotes candidates that are clearly worse and leaves near-ties in stage-1 order.

use std::collections::BTreeMap;

use num_complex::Complex64;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sn_core::{SampleKind, Signal};
use sn_dsp::resample::Resampler;

use crate::grid::{SymbolFft, classify};
use crate::util::{argmax, c64, lag_product, window_sums};
use crate::{Baseband, BrEstimate, Error, Grid, Result, check_stop, invalid};

/// Settings for [`list_of_k`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KParams {
    /// Current sample rate in Hz.
    pub sample_rate: f64,
    /// Measured symbol rate in Bd.
    pub br: f64,
    /// Measured useful length at the current rate, in (fractional) samples.
    pub lu: f64,
    /// Largest distance of LU' from an integer that is still accepted, in samples.
    pub tolerance: f64,
    /// Highest candidate rate as a multiple of the current rate (SA: 2).
    pub max_factor: f64,
    /// Most K values returned.
    pub max_candidates: usize,
}

impl KParams {
    /// Settings from a Get Br estimate: tolerance 0.15 samples, rates up to 2 x Fd,
    /// at most 50 K values.
    pub fn from_br(br: &BrEstimate) -> Self {
        Self {
            sample_rate: br.sample_rate,
            br: br.br,
            lu: br.lu,
            tolerance: 0.15,
            max_factor: 2.0,
            max_candidates: 50,
        }
    }
}

/// One K value from the calculator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KCandidate {
    /// Numerator of K in lowest terms.
    pub k_num: usize,
    /// Denominator of K in lowest terms.
    pub k_den: usize,
    /// `K = LG / LU`.
    pub k: f64,
    /// Symbol length at the representative native rate.
    pub ls: usize,
    /// Useful length there.
    pub lu: usize,
    /// Guard length there.
    pub lg: usize,
    /// The representative native rate `ls * br`, Hz.
    pub sample_rate: f64,
    /// Smallest distance of LU' from an integer among the rates with this K.
    pub lu_error: f64,
    /// True when the representative LU is even.
    pub even_lu: bool,
    /// Number of tested rates that gave this K.
    pub hits: usize,
    /// Stage-1 score, lower is more plausible.
    pub score: f64,
    /// Stage-2 guard leakage in dB: energy outside the strongest N bins relative to the
    /// energy in them (lower is more orthogonal); `None` when not checked.
    pub leakage_db: Option<f64>,
    /// Stage-2 window-consistency error in dB (lower is more orthogonal; the ranking
    /// key); `None` when not checked.
    pub ici_db: Option<f64>,
}

/// One tested rate: (LS, LU, distance of the exact LU from an integer).
type RateHit = (usize, usize, f64);

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Stage 1 of the K calculator (SA: List of K): candidate K values, most plausible
/// first.
pub fn list_of_k(p: &KParams) -> Result<Vec<KCandidate>> {
    let finite_pos = |v: f64| v.is_finite() && v > 0.0;
    if !(finite_pos(p.sample_rate) && finite_pos(p.br) && finite_pos(p.lu)) {
        return Err(invalid(
            "the sample rate, Br and LU must all be above 0; run Get Br first",
        ));
    }
    let ls = p.sample_rate / p.br;
    if p.lu >= ls {
        return Err(invalid(format!(
            "LU {:.3} is not below LS {ls:.3}; the measurement is not a CP signal",
            p.lu
        )));
    }
    if !(p.tolerance > 0.0 && p.tolerance < 0.5) {
        return Err(invalid("the tolerance must be between 0 and 0.5 samples"));
    }
    if !(p.max_factor.is_finite() && p.max_factor >= 1.0) {
        return Err(invalid(
            "the highest rate factor must be a finite number of at least 1 (SA uses 2)",
        ));
    }
    let factor = p.max_factor;
    // From the whole LS nearest the current one (so the current rate is tested even
    // when LS comes out a hair above an integer) up to max_factor * LS.
    let n_lo = ((ls - 0.5).ceil() as usize).max(2);
    let n_hi = (factor * ls + 1e-9).floor() as usize;
    if n_hi.saturating_sub(n_lo) > 1_000_000 {
        return Err(invalid(format!(
            "LS {ls:.1} would need over a million candidate rates; resample to a lower rate \
             first"
        )));
    }
    // (K numerator, K denominator) -> [(LS, LU, distance of LU from an integer)]
    let mut groups: BTreeMap<(usize, usize), Vec<RateHit>> = BTreeMap::new();
    for n in n_lo..=n_hi {
        let lu_est = p.lu * n as f64 / ls;
        let lu = lu_est.round();
        let d = (lu_est - lu).abs();
        let lu = lu as usize;
        if d > p.tolerance || lu == 0 || lu >= n {
            continue;
        }
        let lg = n - lu;
        let g = gcd(lg, lu).max(1);
        groups.entry((lg / g, lu / g)).or_default().push((n, lu, d));
    }
    let mut out: Vec<KCandidate> = groups
        .into_iter()
        .filter_map(|((k_num, k_den), members)| {
            let rep = members
                .iter()
                .find(|(_, lu, _)| lu % 2 == 0)
                .or_else(|| members.first())
                .copied()?;
            let lu_error = members.iter().map(|m| m.2).fold(f64::INFINITY, f64::min);
            let (n, lu, _) = rep;
            let even = lu % 2 == 0;
            Some(KCandidate {
                k_num,
                k_den,
                k: k_num as f64 / k_den as f64,
                ls: n,
                lu,
                lg: n - lu,
                sample_rate: n as f64 * p.br,
                lu_error,
                even_lu: even,
                hits: members.len(),
                score: lu_error / p.tolerance + if even { 0.0 } else { 0.1 },
                leakage_db: None,
                ici_db: None,
            })
        })
        .collect();
    out.sort_by(|a, b| a.score.total_cmp(&b.score).then(a.k_den.cmp(&b.k_den)));
    out.truncate(p.max_candidates.max(1));
    if out.is_empty() {
        return Err(Error::NotFound(format!(
            "no rate between {:.1} and {:.1} Hz makes LU a whole number within {} samples; \
             measure Br and LU again (Get Br over more symbols) or raise the tolerance",
            n_lo as f64 * p.br,
            n_hi as f64 * p.br,
            p.tolerance
        )));
    }
    Ok(out)
}

/// Settings for [`check_orthogonality`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrthoParams {
    /// How many of the leading candidates to check.
    pub top: usize,
    /// Symbols transformed per candidate.
    pub symbols: usize,
    /// Candidates whose window-consistency error is within this many dB of the best
    /// are treated as equally orthogonal and keep their stage-1 order.
    pub tie_db: f64,
}

impl Default for OrthoParams {
    fn default() -> Self {
        Self {
            top: 8,
            symbols: 60,
            tie_db: 1.0,
        }
    }
}

/// Stage-2 measurements of one candidate.
struct Metrics {
    /// Mean power per bin.
    power: Vec<f64>,
    /// Window-consistency error relative to the window energy.
    ici: f64,
}

/// Resample to the candidate's rate, find its triangle and measure it, or `None` if it
/// does not fit.
fn candidate_metrics(
    seg: &Signal,
    c: &KCandidate,
    symbols: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Option<Metrics>> {
    let fs = seg.sample_rate();
    let sr = c.sample_rate;
    if !(sr.is_finite() && sr > 0.0) || c.lu == 0 || c.ls <= c.lu {
        return Ok(None);
    }
    let y = Resampler::new(fs, sr)?.apply(seg, 0, seg.len(), should_stop)?;
    let k = y.len().saturating_sub(c.lu) / c.ls;
    if k < 4 {
        return Ok(None);
    }
    let p = lag_product(&y, c.lu, 0, (k - 1) * c.ls);
    let mut f = vec![Complex64::new(0.0, 0.0); c.ls];
    for chunk in p.chunks_exact(c.ls) {
        for (a, v) in f.iter_mut().zip(chunk) {
            *a += c64(*v);
        }
    }
    let w = window_sums(&f, c.lg);
    let Some((peak, _)) = argmax(&w, |v| v.norm()) else {
        return Ok(None);
    };
    let phase = w.get(peak).map_or(0.0, |v| v.arg());
    // Two windows on the good slope, a quarter of LG in from each end.
    let (d1, d2) = if c.lg >= 4 {
        (c.lg / 4, c.lg - c.lg / 4)
    } else {
        (0, c.lg)
    };
    let grid = Grid {
        sample_rate: sr,
        ls: c.ls as f64,
        lu: c.lu,
        lg: c.lg,
        start: peak as f64,
        window_offset: d1,
        freq_offset: -phase / std::f64::consts::TAU * sr / c.lu as f64,
        virtual_shift: 0,
    };
    let late = Grid {
        window_offset: d2,
        ..grid.clone()
    };
    let early = SymbolFft::new(&y, &grid)?;
    let later = SymbolFft::new(&y, &late)?;
    let last = early.count().min(later.count()).min(symbols);
    if last < 3 {
        return Ok(None);
    }
    // At the true LU both windows hold whole periods of the same symbol, so their
    // spectra differ only by the ramp exp(j 2 pi b (d2 - d1) / LU) (and one common
    // phase, fitted per symbol to absorb a residual frequency offset).
    let shift = (d2 - d1) as f64 / c.lu as f64;
    let (mut err, mut total) = (0.0, 0.0);
    for kk in 0..last {
        check_stop(should_stop)?;
        let (a, b) = (early.values(kk)?, later.values(kk)?);
        let mut cross = Complex64::new(0.0, 0.0);
        let mut energy = 0.0;
        for (i, (u, v)) in a.iter().zip(&b).enumerate() {
            let ramp = Complex64::from_polar(1.0, std::f64::consts::TAU * shift * i as f64);
            cross += v * (u * ramp).conj();
            energy += u.norm_sqr() + v.norm_sqr();
        }
        err += energy - 2.0 * cross.norm();
        total += energy;
    }
    let ici = if total > 0.0 {
        err.max(0.0) / total
    } else {
        1.0
    };
    Ok(Some(Metrics {
        power: early.mean_power(&grid.bins(), 0, last, should_stop)?,
        ici,
    }))
}

/// Stage 2 of the K calculator (new; SA never had it): resample to each of the top
/// candidates' native rates and measure how orthogonal the channels are there.
/// Returns all candidates: the checked ones first (those within `tie_db` of the most
/// orthogonal in stage-1 order, then the rest by increasing error), then the unchecked
/// ones in stage-1 order.
pub fn check_orthogonality(
    bb: &Baseband,
    br: &BrEstimate,
    candidates: &[KCandidate],
    params: &OrthoParams,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<KCandidate>> {
    let x = bb.samples();
    let a = br.start.floor().max(0.0) as usize;
    let want = ((params.symbols.max(4) + 3) as f64 * br.ls).ceil() as usize;
    let b = (a + want).min(x.len());
    let span = x.get(a..b).unwrap_or_default();
    if (span.len() as f64) < 6.0 * br.ls {
        return Err(invalid(format!(
            "the orthogonality check needs at least 6 symbols ({:.0} samples) after the \
             first symbol; select a longer span",
            6.0 * br.ls
        )));
    }
    let seg = Signal::complex(span.to_vec(), bb.sample_rate())?;
    let top = params.top.min(candidates.len());
    let (head, tail) = candidates.split_at(top);
    let metrics: Vec<Option<Metrics>> = head
        .par_iter()
        .map(|c| {
            check_stop(should_stop)?;
            candidate_metrics(&seg, c, params.symbols, should_stop)
        })
        .collect::<Result<_>>()?;
    let channels = metrics
        .iter()
        .flatten()
        .map(|m| {
            let (_, _, thr) = classify(&m.power);
            m.power.iter().filter(|v| **v > thr).count()
        })
        .min()
        .unwrap_or(1)
        .max(1);
    let mut checked: Vec<KCandidate> = head
        .iter()
        .zip(&metrics)
        .map(|(c, m)| {
            let leakage_db = m.as_ref().map(|m| {
                let mut s = m.power.clone();
                s.sort_by(|u, v| v.total_cmp(u));
                let inside: f64 = s.iter().take(channels).sum();
                let outside: f64 = s.iter().skip(channels).sum();
                10.0 * (outside.max(1e-30) / inside.max(1e-30)).log10()
            });
            KCandidate {
                leakage_db,
                ici_db: m.as_ref().map(|m| 10.0 * m.ici.max(1e-30).log10()),
                ..c.clone()
            }
        })
        .collect();
    // Candidates within `tie_db` of the most orthogonal one are indistinguishable at
    // this SNR and keep their stage-1 order; the rest follow, most orthogonal first.
    let best = checked
        .iter()
        .filter_map(|c| c.ici_db)
        .fold(f64::INFINITY, f64::min);
    let tie = params.tie_db.max(0.0);
    let tier = |c: &KCandidate| match c.ici_db {
        Some(v) if v <= best + tie => 0,
        Some(_) => 1,
        None => 2,
    };
    checked.sort_by(|u, v| {
        tier(u).cmp(&tier(v)).then_with(|| match tier(u) {
            0 => u.score.total_cmp(&v.score),
            _ => u
                .ici_db
                .unwrap_or(f64::INFINITY)
                .total_cmp(&v.ici_db.unwrap_or(f64::INFINITY)),
        })
    });
    checked.extend(tail.iter().cloned());
    Ok(checked)
}

/// Native sample rate `SR = Br * (LU + LG)` for a chosen K/LU/LG.
pub fn native_rate(br: f64, lu: usize, lg: usize) -> Result<f64> {
    if !(br.is_finite() && br > 0.0) || lu == 0 {
        return Err(invalid(
            "Br must be above 0 and LU at least 1 to compute a native rate",
        ));
    }
    Ok(br * (lu + lg) as f64)
}

/// Resample a whole signal to `rate` (for example a native rate), keeping it real or
/// complex.
pub fn resample_to_rate(
    signal: &Signal,
    rate: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Signal> {
    let y =
        Resampler::new(signal.sample_rate(), rate)?.apply(signal, 0, signal.len(), should_stop)?;
    let out = match signal.kind() {
        SampleKind::Complex => Signal::complex(y, rate)?,
        SampleKind::Real => Signal::real(y.iter().map(|v| v.re).collect::<Vec<f32>>(), rate)?,
    };
    Ok(out.with_center_frequency(signal.center_frequency()))
}
