//! The symbol grid, the LU spectrum, channel occupancy and per-channel symbols.

use std::sync::Arc;

use num_complex::Complex64;
use rayon::prelude::*;
use rustfft::{Fft, FftPlanner};
use serde::{Deserialize, Serialize};
use sn_core::Complex32;

use crate::util::c64;
use crate::{BrEstimate, CtResult, Error, Result, invalid};

/// Where the symbols are and how to read them: the state behind SA's yellow marker,
/// frequency shift and virtual shift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// Symbol length in samples (fractional, so symbols do not drift).
    pub ls: f64,
    /// Useful length: the FFT size.
    pub lu: usize,
    /// Guard length in samples.
    pub lg: usize,
    /// Start of symbol 0 (its CP), fractional baseband index.
    pub start: f64,
    /// Where the LU window starts inside a symbol, 0..=lg. Every value on this "good
    /// slope" gives a valid window; `lg` (the default) skips the whole prefix, so the
    /// window is exactly the useful part.
    pub window_offset: usize,
    /// Frequency removed before the FFT, in Hz (puts the channels on the grid).
    pub freq_offset: f64,
    /// Virtual shift in whole channel steps: the channel seen at FFT bin `b + shift`
    /// is reported as bin `b`, with the per-symbol phase that a real frequency shift
    /// of `shift` channels would remove. Use it to put channels on their native bins.
    pub virtual_shift: i64,
}

impl Grid {
    /// Grid from a Find CT result at integer LS (no Get Br yet).
    pub fn from_ct(ct: &CtResult) -> Self {
        let fs = ct.sample_rate;
        Self {
            sample_rate: fs,
            ls: ct.ls as f64,
            lu: ct.lu,
            lg: ct.lg,
            start: ct.start as f64,
            window_offset: ct.lg,
            freq_offset: -ct.triangle.phase / std::f64::consts::TAU * fs / ct.lu.max(1) as f64,
            virtual_shift: 0,
        }
    }

    /// Grid from a Get Br estimate (fractional LS, frequency positioned on the grid).
    pub fn from_br(br: &BrEstimate) -> Self {
        Self {
            sample_rate: br.sample_rate,
            ls: br.ls,
            lu: br.lu_int,
            lg: br.lg_int,
            start: br.start,
            window_offset: br.lg_int,
            freq_offset: br.freq_offset,
            virtual_shift: 0,
        }
    }

    /// The same grid with a virtual shift of `bins` channel steps.
    #[must_use]
    pub fn with_virtual_shift(mut self, bins: i64) -> Self {
        self.virtual_shift = bins;
        self
    }

    /// Channel spacing `sample_rate / lu` in Hz.
    pub fn spacing(&self) -> f64 {
        self.sample_rate / self.lu.max(1) as f64
    }

    /// First sample of the LU window of symbol `k` (saturating at `usize::MAX` for an
    /// index far past any data).
    pub fn window_start(&self, k: usize) -> usize {
        ((self.start + k as f64 * self.ls).round().max(0.0) as usize)
            .saturating_add(self.window_offset)
    }

    /// True when the LU window of symbol `k` ends within `len` samples.
    fn fits(&self, k: usize, len: usize) -> bool {
        self.window_start(k)
            .checked_add(self.lu)
            .is_some_and(|end| end <= len)
    }

    /// Number of symbols whose LU window fits in `len` samples. Counted with the same
    /// rounding as [`Grid::window_start`], so the last symbol counted always fits.
    pub fn symbol_count(&self, len: usize) -> usize {
        if !(self.ls.is_finite() && self.ls > 0.0 && self.start.is_finite()) {
            return 0;
        }
        let room = len as f64 - (self.start + (self.window_offset + self.lu) as f64);
        let estimate = if room < 0.0 {
            0
        } else {
            ((room / self.ls).floor() as usize).min(len)
        };
        // `fits` is monotone in k: binary search for the first symbol that does not fit
        // (the estimate is within a symbol of it).
        let (mut lo, mut hi) = (0_usize, estimate.saturating_add(2));
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.fits(mid, len) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// Bin indices in display order: `-lu/2 .. lu - lu/2` (rounded down).
    pub fn bins(&self) -> Vec<i64> {
        let lu = self.lu as i64;
        (-(lu / 2)..lu - lu / 2).collect()
    }

    /// Frequency in the analysed signal of channel `bin`, in Hz.
    pub fn bin_frequency(&self, bin: i64) -> f64 {
        (bin + self.virtual_shift) as f64 * self.spacing() + self.freq_offset
    }

    fn check(&self) -> Result<()> {
        let ok = self.lu >= 1
            && self.sample_rate.is_finite()
            && self.sample_rate > 0.0
            && self.ls.is_finite()
            && self.ls >= self.lu as f64
            && self
                .lu
                .checked_add(self.lg)
                .is_some_and(|s| s as f64 <= self.ls + 0.5)
            && self.start.is_finite()
            && self.start >= 0.0
            && self.window_offset <= self.lg
            && self.freq_offset.is_finite();
        if ok {
            Ok(())
        } else {
            Err(invalid(format!(
                "the symbol grid (LS {}, LU {}, LG {}, start {}, window offset {}) is not \
                 consistent: LS must be finite, LU + LG at most LS (to half a sample), the \
                 start finite and at least 0, and the window offset within 0..=LG. Run Find \
                 CT and Get Br again",
                self.ls, self.lu, self.lg, self.start, self.window_offset
            )))
        }
    }
}

/// Symbols transformed per parallel batch (and per run of [`channel_symbols`]).
const SYMBOL_BATCH: usize = 64;

/// FFT of LU windows on a grid.
pub(crate) struct SymbolFft<'a> {
    x: &'a [Complex32],
    grid: &'a Grid,
    fft: Arc<dyn Fft<f64>>,
    count: usize,
    /// Added to every sample before the transform (the DC the baseband removed).
    dc: Complex64,
}

impl<'a> SymbolFft<'a> {
    pub(crate) fn new(x: &'a [Complex32], grid: &'a Grid) -> Result<Self> {
        grid.check()?;
        let fft = FftPlanner::<f64>::new().plan_fft_forward(grid.lu);
        let count = grid.symbol_count(x.len());
        Ok(Self {
            x,
            grid,
            fft,
            count,
            dc: Complex64::new(0.0, 0.0),
        })
    }

    /// Engine on a baseband's samples with its removed DC added back.
    fn for_baseband(bb: &'a crate::Baseband, grid: &'a Grid) -> Result<Self> {
        Ok(Self {
            dc: c64(bb.dc()),
            ..Self::new(bb.samples(), grid)?
        })
    }

    /// FFT bins of symbol `k` in FFT order, scaled by `1/lu`.
    pub(crate) fn values(&self, k: usize) -> Result<Vec<Complex64>> {
        let g = self.grid;
        let past = || {
            invalid(format!(
                "symbol {k} runs past the end of the span ({} symbols fit)",
                self.count
            ))
        };
        if k >= self.count {
            return Err(past());
        }
        let n0 = g.window_start(k);
        let seg = n0
            .checked_add(g.lu)
            .and_then(|end| self.x.get(n0..end))
            .ok_or_else(past)?;
        let hz = g.freq_offset + g.virtual_shift as f64 * g.spacing();
        let cps = (hz / g.sample_rate).rem_euclid(1.0);
        let base = (n0 as f64 * cps).rem_euclid(1.0);
        let scale = 1.0 / g.lu as f64;
        let mut buf: Vec<Complex64> = seg
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let ph = -std::f64::consts::TAU * (base + i as f64 * cps).rem_euclid(1.0);
                (c64(*v) + self.dc) * Complex64::from_polar(scale, ph)
            })
            .collect();
        self.fft.process(&mut buf);
        Ok(buf)
    }

    /// Mean power of `bins` over symbols `first..last`.
    pub(crate) fn mean_power(
        &self,
        bins: &[i64],
        first: usize,
        last: usize,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<f64>> {
        let mut power = vec![0.0_f64; bins.len()];
        // Batches of symbols are transformed in parallel; their powers are added in
        // symbol order, so the sums are those of a serial pass.
        let ks: Vec<usize> = (first..last).collect();
        for batch in ks.chunks(SYMBOL_BATCH) {
            let rows: Vec<Vec<f64>> = batch
                .par_iter()
                .map(|&k| -> Result<Vec<f64>> {
                    crate::check_stop(should_stop)?;
                    let v = self.values(k)?;
                    Ok(bins.iter().map(|b| self.bin(&v, *b).norm_sqr()).collect())
                })
                .collect::<Result<_>>()?;
            for row in rows {
                for (p, r) in power.iter_mut().zip(row) {
                    *p += r;
                }
            }
        }
        let n = last.saturating_sub(first).max(1) as f64;
        for p in &mut power {
            *p /= n;
        }
        Ok(power)
    }

    /// Symbols that fit.
    pub(crate) fn count(&self) -> usize {
        self.count
    }

    fn bin(&self, values: &[Complex64], b: i64) -> Complex64 {
        let i = b.rem_euclid(self.grid.lu as i64) as usize;
        values.get(i).copied().unwrap_or_default()
    }
}

fn to_db(p: f64) -> f32 {
    (10.0 * p.max(1e-30).log10()) as f32
}

/// The LU spectrum of one symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuSpectrum {
    /// Symbol index.
    pub symbol: usize,
    /// First sample of the LU window (baseband index).
    pub window_start: usize,
    /// Channel spacing in Hz.
    pub spacing: f64,
    /// Bin indices in display order.
    pub bins: Vec<i64>,
    /// Frequency of each bin in the analysed signal, Hz.
    pub frequencies: Vec<f64>,
    /// Complex value of each bin (scaled by 1/LU, so a channel of amplitude A reads A).
    pub values: Vec<Complex32>,
    /// Power of each bin in dB.
    pub power_db: Vec<f32>,
}

/// FFT of the LU window of `symbol` (the window starts `window_offset` samples into the
/// symbol, on the good slope).
pub fn lu_spectrum(bb: &crate::Baseband, grid: &Grid, symbol: usize) -> Result<LuSpectrum> {
    let engine = SymbolFft::for_baseband(bb, grid)?;
    let v = engine.values(symbol)?;
    let bins = grid.bins();
    let values: Vec<Complex64> = bins.iter().map(|b| engine.bin(&v, *b)).collect();
    Ok(LuSpectrum {
        symbol,
        window_start: grid.window_start(symbol),
        spacing: grid.spacing(),
        frequencies: bins.iter().map(|b| grid.bin_frequency(*b)).collect(),
        power_db: values.iter().map(|c| to_db(c.norm_sqr())).collect(),
        values: values
            .iter()
            .map(|c| Complex32::new(c.re as f32, c.im as f32))
            .collect(),
        bins,
    })
}

/// Which channels carry energy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Occupancy {
    /// Bin indices in display order.
    pub bins: Vec<i64>,
    /// Mean power of each bin over the symbols, dB.
    pub power_db: Vec<f32>,
    /// Noise-floor estimate, dB: the mean level of the weaker of the two classes the
    /// bins split into.
    pub floor_db: f32,
    /// Channel level estimate, dB: the mean level of the stronger class.
    pub peak_db: f32,
    /// Decision threshold, dB: in the gap between the two classes, or very low (every
    /// bin occupied) when they are less than 6 dB apart.
    pub threshold_db: f32,
    /// Bins above the threshold, ascending.
    pub occupied: Vec<i64>,
    /// Symbols averaged.
    pub symbols: usize,
}

impl Occupancy {
    /// Number of occupied channels.
    pub fn count(&self) -> usize {
        self.occupied.len()
    }

    /// Number of grid positions from the first to the last occupied channel.
    pub fn span(&self) -> usize {
        match (self.occupied.first(), self.occupied.last()) {
            (Some(a), Some(b)) => (b - a) as usize + 1,
            _ => 0,
        }
    }
}

/// Classes closer than this (dB) are one class: every bin is occupied.
const MIN_SPLIT_DB: f64 = 6.0;

/// Floor, peak and threshold (linear power) for a set of bin powers. The bins are
/// split into a weak and a strong class on their dB values by Otsu's method (the
/// split with the largest between-class variance), so the result does not depend on
/// what fraction of the bins hold channels. Floor and peak are the class means; the
/// threshold sits in the gap at the split. When the classes are less than 6 dB apart
/// every bin counts as occupied (threshold 0).
pub(crate) fn classify(power: &[f64]) -> (f64, f64, f64) {
    let mut db: Vec<f64> = power
        .iter()
        .filter(|p| p.is_finite())
        .map(|p| 10.0 * p.max(1e-30).log10())
        .collect();
    db.sort_by(f64::total_cmp);
    let lin = |d: f64| 10_f64.powf(d / 10.0);
    let n = db.len();
    let total: f64 = db.iter().sum();
    let mut below = 0.0;
    // (between-class variance, split index, lower mean, upper mean)
    let mut best: Option<(f64, usize, f64, f64)> = None;
    for (i, d) in db.iter().enumerate().take(n.saturating_sub(1)) {
        below += d;
        let n0 = (i + 1) as f64;
        let n1 = (n - i - 1) as f64;
        let (m0, m1) = (below / n0, (total - below) / n1);
        let var = n0 * n1 * (m1 - m0).powi(2);
        if best.is_none_or(|b| var > b.0) {
            best = Some((var, i + 1, m0, m1));
        }
    }
    let Some((_, split, m0, m1)) = best else {
        let only = db.first().copied().map_or(1e-30, lin);
        return (only, only, 0.0);
    };
    if m1 - m0 < MIN_SPLIT_DB {
        return (lin(m0), lin(m1), 0.0);
    }
    let gap_lo = split
        .checked_sub(1)
        .and_then(|i| db.get(i))
        .copied()
        .unwrap_or(m0);
    let gap_hi = db.get(split).copied().unwrap_or(m1);
    (lin(m0), lin(m1), lin(f64::midpoint(gap_lo, gap_hi)))
}

/// Mean power per bin over `count` symbols from `first`, and the occupied channels.
///
/// Same as [`occupancy_with_stop`] without cancellation.
pub fn occupancy(
    bb: &crate::Baseband,
    grid: &Grid,
    first: usize,
    count: usize,
) -> Result<Occupancy> {
    occupancy_with_stop(bb, grid, first, count, &|| false)
}

/// [`occupancy`] that checks `should_stop` once per symbol and returns
/// [`Error::Cancelled`] when it fires.
pub fn occupancy_with_stop(
    bb: &crate::Baseband,
    grid: &Grid,
    first: usize,
    count: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Occupancy> {
    let engine = SymbolFft::for_baseband(bb, grid)?;
    let last = first.saturating_add(count.max(1)).min(engine.count);
    if first >= last {
        return Err(invalid(format!(
            "symbol {first} is past the end of the span ({} symbols fit)",
            engine.count
        )));
    }
    let bins = grid.bins();
    let power = engine.mean_power(&bins, first, last, should_stop)?;
    let (floor, peak, thr) = classify(&power);
    let occupied = bins
        .iter()
        .zip(&power)
        .filter(|(_, p)| **p > thr)
        .map(|(b, _)| *b)
        .collect();
    Ok(Occupancy {
        power_db: power.iter().map(|p| to_db(*p)).collect(),
        floor_db: to_db(floor),
        peak_db: to_db(peak),
        threshold_db: to_db(thr),
        occupied,
        symbols: last - first,
        bins,
    })
}

/// How channel values are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolView {
    /// The FFT value itself (model A reading).
    Absolute,
    /// Value times the conjugate of the same channel in the previous symbol, divided by
    /// that symbol's magnitude: the phase step, with the current amplitude (model B
    /// reading).
    Relative,
    /// Value rotated by the phase of `pilot` in the same symbol (model C reading).
    PilotReferenced {
        /// Bin of the pilot channel.
        pilot: i64,
    },
}

/// Per-channel symbol values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelSymbols {
    /// The view used.
    pub view: SymbolView,
    /// Channel bins, as requested.
    pub bins: Vec<i64>,
    /// Symbol indices shown.
    pub symbols: Vec<usize>,
    /// `values[c][j]`: channel `bins[c]` in symbol `symbols[j]`.
    pub values: Vec<Vec<Complex32>>,
}

fn rotate_by(v: Complex64, reference: Complex64) -> Complex64 {
    let m = reference.norm();
    if m > 0.0 { v * reference.conj() / m } else { v }
}

/// Values of channels `bins` for symbols `first, first + every, ...` (`count` of them,
/// SA's Each and Start+). Relative views skip symbol 0, which has no predecessor.
///
/// Same as [`channel_symbols_with_stop`] without cancellation.
pub fn channel_symbols(
    bb: &crate::Baseband,
    grid: &Grid,
    bins: &[i64],
    first: usize,
    count: usize,
    every: usize,
    view: SymbolView,
) -> Result<ChannelSymbols> {
    channel_symbols_with_stop(bb, grid, bins, first, count, every, view, &|| false)
}

/// [`channel_symbols`] that checks `should_stop` once per symbol and returns
/// [`Error::Cancelled`] when it fires.
#[allow(clippy::too_many_arguments)]
pub fn channel_symbols_with_stop(
    bb: &crate::Baseband,
    grid: &Grid,
    bins: &[i64],
    first: usize,
    count: usize,
    every: usize,
    view: SymbolView,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<ChannelSymbols> {
    if every == 0 {
        return Err(invalid("'every' must be at least 1 (1 shows every symbol)"));
    }
    let engine = SymbolFft::for_baseband(bb, grid)?;
    let all = grid.bins();
    let (lo, hi) = (
        all.first().copied().unwrap_or(0),
        all.last().copied().unwrap_or(0),
    );
    let check_bin = |b: i64| -> Result<()> {
        if (lo..=hi).contains(&b) {
            Ok(())
        } else {
            Err(invalid(format!(
                "channel {b} is outside the LU spectrum ({lo}..={hi}); use a virtual shift to \
                 reach it"
            )))
        }
    };
    for b in bins {
        check_bin(*b)?;
    }
    if let SymbolView::PilotReferenced { pilot } = view {
        check_bin(pilot)?;
    }
    let first = if view == SymbolView::Relative {
        first.max(1)
    } else {
        first
    };
    let ks: Vec<usize> = (0..count)
        .map_while(|j| j.checked_mul(every).and_then(|o| first.checked_add(o)))
        .take_while(|k| *k < engine.count)
        .collect();
    if ks.is_empty() {
        return Err(Error::NotFound(format!(
            "no symbol from {first} fits in the span ({} symbols)",
            engine.count
        )));
    }
    // Runs of symbols are read in parallel, each run in order so the relative view
    // with `every == 1` transforms each symbol once per run; every value is the same
    // as in a single serial pass.
    // Each run returns its values symbol-major (one row of `bins.len()` per symbol).
    let runs: Vec<Vec<Complex32>> = ks
        .par_chunks(SYMBOL_BATCH)
        .map(|run| -> Result<Vec<Complex32>> {
            let mut out = Vec::with_capacity(run.len() * bins.len());
            // The last spectrum computed.
            let mut last: Option<(usize, Vec<Complex64>)> = None;
            for &k in run {
                crate::check_stop(should_stop)?;
                let prev = match (view, k.checked_sub(1)) {
                    (SymbolView::Relative, Some(kp)) => match last.take() {
                        Some((i, v)) if i == kp => Some(v),
                        _ => Some(engine.values(kp)?),
                    },
                    _ => None,
                };
                let v = engine.values(k)?;
                out.extend(bins.iter().map(|b| {
                    let x = engine.bin(&v, *b);
                    let y = match (view, &prev) {
                        (SymbolView::Relative, Some(p)) => rotate_by(x, engine.bin(p, *b)),
                        (SymbolView::PilotReferenced { pilot }, _) => {
                            rotate_by(x, engine.bin(&v, pilot))
                        }
                        _ => x,
                    };
                    Complex32::new(y.re as f32, y.im as f32)
                }));
                last = Some((k, v));
            }
            Ok(out)
        })
        .collect::<Result<_>>()?;
    // Transpose to channel-major, run by run so each run is freed once copied.
    let mut values = vec![Vec::with_capacity(ks.len()); bins.len()];
    for run in runs {
        for row in run.chunks_exact(bins.len().max(1)) {
            for (out, v) in values.iter_mut().zip(row) {
                out.push(*v);
            }
        }
    }
    Ok(ChannelSymbols {
        view,
        bins: bins.to_vec(),
        symbols: ks,
        values,
    })
}
