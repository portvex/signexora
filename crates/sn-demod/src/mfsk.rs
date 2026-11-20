//! Universal FSK/MFSK demodulator (SA 6.2.3.8; report sections 7.6, 11.4 and 12.5).
//!
//! The demodulator works on a frequency track: the phase-detector output in Hz, or a
//! discriminator recording in any unit. It
//!
//! 1. optionally filters the track: RRC (APCO-25 uses alpha 0.2) for shaped
//!    signals, or a moving average over part of a symbol (integrate and dump) for
//!    rectangular FSK, where an RRC filter alone would add inter-symbol interference,
//! 2. finds the symbol timing, either from a manual phase or automatically,
//! 3. compares the value at each sampling instant with up to 256 decision levels,
//! 4. optionally lets the level centres follow the data (adaptive levels, new beyond
//!    SA, asked for in article 412).
//!
//! # Automatic synchronisation
//!
//! The track is cut into blocks. For each block and each of 32 trial phases across
//! one symbol, the cost is the mean squared distance of the sampled values from their
//! nearest level centre, normalised to the half gap between levels. The open part of
//! the eye has a low cost; the block phase is the circular centroid of the low-cost
//! region, and its weight is the contrast of the cost curve. A block inside a long run
//! of one symbol has a flat cost curve and so a weight near zero: it neither helps nor
//! misleads.
//!
//! Block phases are unwrapped with an alpha-beta tracker that starts from the most
//! confident block and runs both ways, then smoothed by a weighted local linear fit.
//!
//! The search runs in several passes. A clock error `e` moves the eye by `e * n` symbols
//! over a block of `n` symbols, which smears the cost curve and, near half a symbol,
//! makes the unwrapping ambiguous. The first pass therefore uses short blocks over
//! which the largest allowed drift ([`MfskConfig::max_clock_error`]) moves the eye
//! by at most a fifth of a symbol (but at least 8 symbols). The following passes
//! double the block length each time, up to [`MfskConfig::block_symbols`]: each one
//! measures its blocks on the drifting grid that the previous pass found, so only
//! the small residual drift of that pass smears them, and adds the measured
//! residual. Each pass thus corrects the slope error left by the noisier shorter
//! blocks before it. The plain search with full blocks is run as well, and the timing whose
//! decided values sit closer to their level centres is kept (with little drift the
//! longer blocks are less noisy). The slope of the timing gives the clock error.
//! Clock errors up to [`MfskConfig::max_clock_error`] (0.5 % by default, at most
//! [`MAX_CLOCK_ERROR`], 2.5 %) are followed with any block length. On a selection
//! shorter than four configured blocks the block length is cut to a quarter of the
//! selection (but at least 8 symbols), so that the slope rests on several blocks.
//!
//! # Adaptive levels
//!
//! After each decision the chosen centre moves a fraction
//! [`MfskConfig::adapt_rate`] of the way towards the measured value (a running
//! k-means), and every other centre moves a quarter of that, so unused or rare
//! levels follow a common drift too. Centres never cross. With adaptive levels the
//! timing search is repeated with the level centres tracked in the first pass.
//!
//! Output helpers (character, hex and bit forms, the APCO-25 dibit maps and the
//! inversion check) are in [`output`].

pub mod output;

use std::borrow::Cow;

use sn_core::{SampleKind, Signal};
use sn_dsp::fir::Fir;

pub use output::{
    BitMap, C4FM_MIN_RUN, InversionReport, Polarity, RunCensus, SYMBOL_CHARS, bits_to_string,
    c4fm_level_to_relative_position, detect_c4fm_inversion, detect_inversion, long_runs,
    symbols_to_bits, symbols_to_chars, symbols_to_hex,
};

/// Largest number of decision levels.
pub const MAX_LEVELS: usize = 256;

/// Length of the RRC filter in symbols.
pub const RRC_SPAN: usize = 8;

/// Trial phases per symbol in the automatic timing search.
const PHASE_STEPS: usize = 32;

/// Half-width, in blocks, of the local linear fit that smooths the timing.
const SMOOTH_BLOCKS: usize = 4;

/// Tracker gains (phase and rate) for the block-to-block timing loop.
const TRACK_ALPHA: f64 = 0.5;
const TRACK_BETA: f64 = 0.1;

/// Blocks whose cost-curve contrast is below this carry no timing information.
const MIN_CONTRAST: f64 = 0.05;

/// Fraction of the adaptive step applied to the levels that were not chosen.
const COMMON_SHARE: f64 = 0.25;

/// Largest clock error the automatic timing can follow, as a fraction of the baud.
/// The first timing pass needs blocks of at least [`MIN_COARSE_BLOCK`] symbols over
/// which the eye moves by at most a fifth of a symbol (2.5 %).
pub const MAX_CLOCK_ERROR: f64 = COARSE_DRIFT / MIN_COARSE_BLOCK as f64;

/// Shortest block, in symbols, of the first timing pass.
const MIN_COARSE_BLOCK: usize = 8;

/// Fewest timing blocks the automatic search aims for: the configured block is
/// shortened on a short selection so that the slope of the timing (the clock drift)
/// rests on at least this many points.
const MIN_TIMING_BLOCKS: usize = 4;

/// Largest eye movement, in symbols, over one block of the first timing pass.
const COARSE_DRIFT: f64 = 0.2;

/// Fewest symbols a timing block needs to give a measurement: below this every
/// trial phase has too few samples and the block gets no weight.
const MIN_MEAS_SYMBOLS: usize = 4;

pub(crate) use crate::invalid;
pub use crate::{Error, Result};

/// Where the decision levels sit.
#[derive(Debug, Clone, PartialEq)]
pub enum Levels {
    /// Explicit level centres (Hz for a phase-detector track), in any order.
    Explicit(Vec<f64>),
    /// `count` evenly spaced levels from `lo` to `hi` inclusive (SA: two main markers
    /// on the outermost tones plus a number of levels).
    Grid {
        /// Lowest level centre.
        lo: f64,
        /// Highest level centre.
        hi: f64,
        /// Number of levels, 2 to 256.
        count: usize,
    },
}

impl Levels {
    /// The level centres, sorted from lowest to highest.
    pub fn centres(&self) -> Result<Vec<f64>> {
        match self {
            Self::Explicit(v) => {
                let mut v = v.clone();
                v.sort_by(f64::total_cmp);
                Ok(v)
            }
            Self::Grid { lo, hi, count } => {
                if !(2..=MAX_LEVELS).contains(count) {
                    return Err(invalid(
                        "count",
                        format!("use between 2 and {MAX_LEVELS} levels (got {count})"),
                    ));
                }
                if !(lo.is_finite() && hi.is_finite() && hi > lo) {
                    return Err(invalid(
                        "lo/hi",
                        format!(
                            "the highest level ({hi}) must be above the lowest ({lo}); place the markers on the outermost tones"
                        ),
                    ));
                }
                let step = (hi - lo) / (*count as f64 - 1.0);
                Ok((0..*count).map(|i| lo + i as f64 * step).collect())
            }
        }
    }
}

/// Decision lines derived from level centres: thresholds halfway between adjacent
/// centres. The outermost centres are SA's main markers, the others its extra lines.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionLayout {
    centres: Vec<f64>,
    thresholds: Vec<f64>,
}

impl DecisionLayout {
    /// Build a layout from level centres (sorted here). Needs 2 to 256 distinct,
    /// finite centres.
    pub fn new(mut centres: Vec<f64>) -> Result<Self> {
        if !(2..=MAX_LEVELS).contains(&centres.len()) {
            return Err(invalid(
                "levels",
                format!(
                    "use between 2 and {MAX_LEVELS} levels (got {})",
                    centres.len()
                ),
            ));
        }
        if centres.iter().any(|c| !c.is_finite()) {
            return Err(invalid("levels", "every level must be a finite number"));
        }
        centres.sort_by(f64::total_cmp);
        let min_gap = centres
            .windows(2)
            .map(|w| match w {
                [a, b] => b - a,
                _ => f64::INFINITY,
            })
            .fold(f64::INFINITY, f64::min);
        if min_gap <= 0.0 {
            return Err(invalid(
                "levels",
                "two levels are equal; every level needs its own value",
            ));
        }
        Ok(Self::from_sorted(centres))
    }

    fn from_sorted(centres: Vec<f64>) -> Self {
        let thresholds = midpoints(&centres);
        Self {
            centres,
            thresholds,
        }
    }

    /// Level centres, lowest first.
    pub fn centres(&self) -> &[f64] {
        &self.centres
    }

    /// Decision thresholds (one fewer than the centres), lowest first.
    pub fn thresholds(&self) -> &[f64] {
        &self.thresholds
    }

    /// Number of levels.
    pub fn order(&self) -> usize {
        self.centres.len()
    }

    /// The two main markers: the outermost level centres.
    pub fn main_lines(&self) -> (f64, f64) {
        (
            self.centres.first().copied().unwrap_or(0.0),
            self.centres.last().copied().unwrap_or(0.0),
        )
    }

    /// The extra marker lines: the inner level centres.
    pub fn extra_lines(&self) -> &[f64] {
        let n = self.centres.len();
        self.centres.get(1..n.saturating_sub(1)).unwrap_or(&[])
    }

    /// Level index for value `v`.
    pub fn decide(&self, v: f64) -> usize {
        self.thresholds.partition_point(|t| *t <= v)
    }

    /// Level index for `v` and its signed distance from that centre, normalised to
    /// the half gap towards the neighbouring level on the same side (clamped to
    /// -1..=1). 0 is the centre, +-1 a decision threshold.
    pub fn margin(&self, v: f64) -> (usize, f64) {
        let j = self.decide(v);
        let at = |i: usize| self.centres.get(i).copied();
        let Some(c) = at(j) else {
            return (j, 1.0);
        };
        let up = at(j + 1).map(|n| n - c);
        let down = j.checked_sub(1).and_then(at).map(|p| c - p);
        let gap = if v >= c { up.or(down) } else { down.or(up) }.unwrap_or(1.0);
        let d = if gap > 0.0 {
            (v - c) / (gap / 2.0)
        } else {
            0.0
        };
        (j, d.clamp(-1.0, 1.0))
    }
}

fn midpoints(c: &[f64]) -> Vec<f64> {
    c.windows(2)
        .map(|w| match w {
            [a, b] => 0.5 * (a + b),
            _ => 0.0,
        })
        .collect()
}

/// How the symbol timing is found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncMode {
    /// Automatic, block-wise eye-opening search with drift tracking.
    Auto,
    /// Fixed timing: symbol `k` is sampled at `phase + k * fs / baud` samples.
    Manual {
        /// Position of the first sampling instant in samples.
        phase: f64,
    },
}

/// Filter applied to the frequency track before sampling.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Prefilter {
    /// Sample the track as it is.
    #[default]
    None,
    /// Root-raised-cosine filter with roll-off `alpha` (0 < alpha <= 1), as SA's
    /// demodulator offers. Best for signals shaped at the transmitter (C4FM, GFSK).
    Rrc {
        /// Roll-off.
        alpha: f64,
    },
    /// Moving average over `fraction` of a symbol (0 < fraction <= 1): the matched
    /// filter for rectangular FSK. 0.5 keeps the eye free of interference for
    /// sampling within a quarter symbol of the centre.
    Average {
        /// Window length as a fraction of a symbol.
        fraction: f64,
    },
}

/// Demodulator settings.
#[derive(Debug, Clone, PartialEq)]
pub struct MfskConfig {
    /// Decision levels.
    pub levels: Levels,
    /// Nominal keying rate in baud.
    pub baud: f64,
    /// Filter applied before sampling.
    pub filter: Prefilter,
    /// Symbol timing.
    pub sync: SyncMode,
    /// Let the level centres follow the data.
    pub adaptive: bool,
    /// Adaptive step, the fraction of the error applied per decision (0.001..=0.5).
    pub adapt_rate: f64,
    /// Symbols per block in the automatic timing search (16..=4096).
    pub block_symbols: usize,
    /// Largest clock error followed by the automatic timing, as a fraction of the
    /// baud (0.005 = 0.5 %; 0 to [`MAX_CLOCK_ERROR`]).
    pub max_clock_error: f64,
}

impl MfskConfig {
    /// Settings with automatic sync, no filter, fixed levels and default tracking.
    pub fn new(levels: Levels, baud: f64) -> Self {
        Self {
            levels,
            baud,
            filter: Prefilter::None,
            sync: SyncMode::Auto,
            adaptive: false,
            adapt_rate: 0.02,
            block_symbols: 64,
            max_clock_error: 0.005,
        }
    }

    fn validate(&self, sample_rate: f64) -> Result<()> {
        validate_track_settings(self.baud, sample_rate, self.filter)?;
        if !(0.001..=0.5).contains(&self.adapt_rate) {
            return Err(invalid(
                "adapt_rate",
                format!("must be between 0.001 and 0.5 (got {})", self.adapt_rate),
            ));
        }
        if !(16..=4096).contains(&self.block_symbols) {
            return Err(invalid(
                "block_symbols",
                format!("must be between 16 and 4096 (got {})", self.block_symbols),
            ));
        }
        if !(0.0..=MAX_CLOCK_ERROR).contains(&self.max_clock_error) {
            return Err(invalid(
                "max_clock_error",
                format!(
                    "must be between 0 and {MAX_CLOCK_ERROR} ({} %) (got {}); measure the baud more precisely",
                    MAX_CLOCK_ERROR * 100.0,
                    self.max_clock_error
                ),
            ));
        }
        if let SyncMode::Manual { phase } = self.sync
            && !phase.is_finite()
        {
            return Err(invalid("phase", "must be a finite number of samples"));
        }
        Ok(())
    }
}

/// Checks shared by [`frequency_track`] and the demodulator: sample rate, baud and
/// prefilter.
fn validate_track_settings(baud: f64, sample_rate: f64, filter: Prefilter) -> Result<()> {
    if !(sample_rate.is_finite() && sample_rate > 0.0) {
        return Err(invalid(
            "sample_rate",
            format!("must be a positive finite number of Hz (got {sample_rate})"),
        ));
    }
    if !(baud > 0.0 && baud.is_finite()) {
        return Err(invalid(
            "baud",
            "must be above 0; measure it first (ADP or a clock line)",
        ));
    }
    if sample_rate / baud < 2.0 {
        return Err(invalid(
            "baud",
            format!(
                "{baud} Bd needs at least 2 samples per symbol, but the sample rate is {sample_rate} Hz; resample the signal up or check the baud"
            ),
        ));
    }
    match filter {
        Prefilter::Rrc { alpha } if !(alpha > 0.0 && alpha <= 1.0) => {
            return Err(invalid(
                "alpha",
                format!(
                    "the RRC roll-off must be in 0 < alpha <= 1 (got {alpha}); APCO-25 uses 0.2"
                ),
            ));
        }
        Prefilter::Average { fraction } if !(fraction > 0.0 && fraction <= 1.0) => {
            return Err(invalid(
                "fraction",
                format!(
                    "the averaging window must be in 0 < fraction <= 1 of a symbol (got {fraction}); 0.5 is a good start"
                ),
            ));
        }
        _ => {}
    }
    Ok(())
}

/// Result of a demodulation.
#[derive(Debug, Clone, PartialEq)]
pub struct Demodulated {
    /// Decided level index per symbol (0 = lowest level).
    pub symbols: Vec<u8>,
    /// Sampling instant of each symbol in (fractional) samples.
    pub instants: Vec<f64>,
    /// Track value at each instant (after the prefilter).
    pub values: Vec<f32>,
    /// Position of the first sampling instant within a symbol, in samples
    /// (0 to samples per symbol).
    pub timing_phase: f64,
    /// Measured clock error: measured baud / nominal baud - 1.
    pub clock_error: f64,
    /// Measured baud (nominal baud corrected by the clock error).
    pub measured_baud: f64,
    /// Mean normalised distance from the decision thresholds: 1 when every value
    /// sits on its level centre, 0 when every value sits on a threshold.
    pub quality: f64,
    /// Eye opening: 1 minus the 95th percentile of the normalised distance from the
    /// level centre. 1 is a perfect eye, 0 a closed one.
    pub eye_opening: f64,
    /// Level layout the demodulation started from.
    pub layout: DecisionLayout,
    /// Level centres at the end (equal to the start unless adaptive).
    pub final_levels: Vec<f64>,
    /// Number of symbols decided on each level.
    pub level_counts: Vec<usize>,
}

/// Read a real signal as a frequency track and apply the prefilter. The result is
/// what [`demodulate_track`] and [`eye_diagram`] work on. The baud, the sample rate
/// and the filter settings are checked as in [`demodulate`].
pub fn frequency_track(
    signal: &Signal,
    baud: f64,
    filter: Prefilter,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<f32>> {
    if signal.kind() == SampleKind::Complex {
        return Err(Error::NotFrequencyTrack);
    }
    validate_track_settings(baud, signal.sample_rate(), filter)?;
    let n = signal.len();
    match filter {
        Prefilter::Rrc { alpha } => {
            let fir = Fir::rrc(baud, alpha, RRC_SPAN, signal.sample_rate())?;
            Ok(fir
                .apply(signal, 0, n, should_stop)?
                .iter()
                .map(|c| c.re)
                .collect())
        }
        Prefilter::None | Prefilter::Average { .. } => {
            let mut v = vec![0.0_f32; n];
            let got = signal.read_real(0, &mut v);
            v.truncate(got);
            if let Prefilter::Average { fraction } = filter {
                let w = ((signal.sample_rate() / baud * fraction).round() as usize).max(1);
                v = moving_average(&v, w);
            }
            Ok(v)
        }
    }
}

/// Demodulate a frequency-track signal (phase-detector output in Hz, or a
/// discriminator recording).
pub fn demodulate(
    signal: &Signal,
    config: &MfskConfig,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Demodulated> {
    config.validate(signal.sample_rate())?;
    let track = frequency_track(signal, config.baud, config.filter, should_stop)?;
    demodulate_track(&track, signal.sample_rate(), config, should_stop)
}

/// Demodulate an already prepared track (see [`frequency_track`]). The prefilter in
/// `config` is not applied again here; it only decides whether the timing search
/// smooths the track first (it does when the prefilter is [`Prefilter::None`]).
pub fn demodulate_track(
    track: &[f32],
    sample_rate: f64,
    config: &MfskConfig,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Demodulated> {
    config.validate(sample_rate)?;
    let layout = DecisionLayout::new(config.levels.centres()?)?;
    let period = sample_rate / config.baud;
    let need = (period * 4.0).ceil() as usize;
    if track.len() < need {
        return Err(Error::TooShort {
            have: track.len(),
            need,
        });
    }
    if let Some(i) = track.iter().position(|v| !v.is_finite()) {
        return Err(invalid(
            "signal",
            format!(
                "sample {i} of the selection is not a finite number (NaN or infinite); cut or repair that part of the recording first"
            ),
        ));
    }
    let block = config.block_symbols;
    let adapt = config.adaptive.then_some(config.adapt_rate);

    let (timing, dec) = match config.sync {
        SyncMode::Manual { phase } => {
            let timing = Timing {
                tau: vec![phase],
                block,
                n_sym: symbol_count(track.len(), period),
            };
            let dec = decide_all(track, period, &timing, &layout, adapt, should_stop)?;
            if dec.symbols.is_empty() {
                return Err(invalid(
                    "phase",
                    format!(
                        "no sampling instant falls inside the {} samples of the track; use a phase between 0 and {period:.1} samples",
                        track.len()
                    ),
                ));
            }
            (timing, dec)
        }
        SyncMode::Auto => {
            // Pre-smooth for the timing search when no matched filter was applied.
            let timing_track: Cow<'_, [f32]> = if config.filter == Prefilter::None {
                let w = ((period / 4.0).round() as usize).max(1);
                Cow::Owned(moving_average(track, w))
            } else {
                Cow::Borrowed(track)
            };
            let search = TimingSearch {
                track: &timing_track,
                period,
                block: effective_block(block, track.len(), period),
                max_clock_error: config.max_clock_error,
                should_stop,
            };
            let (timing, dec) = search.decide_best(track, &layout, &[], adapt)?;
            if adapt.is_some() {
                // Second search: timing with the level centres tracked in the first.
                search.decide_best(track, &layout, &dec.block_layouts, adapt)?
            } else {
                (timing, dec)
            }
        }
    };

    // Clock error from the slope of the timing offset against the block centres.
    let actual_period = period + timing.slope_per_symbol();
    let clock_error = if actual_period > 0.0 {
        period / actual_period - 1.0
    } else {
        0.0
    };
    let mut mags: Vec<f64> = dec.margins.iter().map(|d| d.abs()).collect();
    let quality = if mags.is_empty() {
        0.0
    } else {
        1.0 - mags.iter().sum::<f64>() / mags.len() as f64
    };
    mags.sort_by(f64::total_cmp);
    let p95 = mags
        .get(((mags.len() as f64 * 0.95) as usize).min(mags.len().saturating_sub(1)))
        .copied()
        .unwrap_or(1.0);
    let mut level_counts = vec![0_usize; layout.order()];
    for s in &dec.symbols {
        if let Some(c) = level_counts.get_mut(usize::from(*s)) {
            *c += 1;
        }
    }
    Ok(Demodulated {
        timing_phase: timing.offset(0).rem_euclid(period),
        clock_error,
        measured_baud: config.baud * (1.0 + clock_error),
        quality,
        eye_opening: 1.0 - p95,
        final_levels: dec.final_levels,
        level_counts,
        layout,
        symbols: dec.symbols,
        instants: dec.instants,
        values: dec.values,
    })
}

/// Longest timing block that still leaves about [`MIN_TIMING_BLOCKS`] blocks in a
/// track of `len` samples: the configured `block`, shortened on a short selection
/// (normally not below [`MIN_COARSE_BLOCK`]). With a single block the timing has no
/// slope, so any clock drift would be lost and symbols slipped. When even
/// [`MIN_COARSE_BLOCK`] leaves fewer than two blocks long enough to measure (a
/// selection of about 8 to 11 symbols), the selection is cut in two halves instead.
/// Below 2 * [`MIN_MEAS_SYMBOLS`] symbols the drift cannot be followed.
fn effective_block(block: usize, len: usize, period: f64) -> usize {
    let n_sym = symbol_count(len, period);
    let b = block.min((n_sym / MIN_TIMING_BLOCKS).max(MIN_COARSE_BLOCK));
    let measurable = n_sym / b.max(1) + usize::from(n_sym % b.max(1) >= MIN_MEAS_SYMBOLS);
    if measurable < 2 && n_sym / 2 >= MIN_MEAS_SYMBOLS {
        b.min(n_sym / 2)
    } else {
        b
    }
}

/// Whole symbols in a track of `len` samples.
fn symbol_count(len: usize, period: f64) -> usize {
    if period > 0.0 {
        (len as f64 / period).floor() as usize
    } else {
        0
    }
}

/// Centre, in symbols, of timing block `b` of `block` symbols in a track of `n_sym`
/// symbols. The last block may be cut short, and its centre is that of the symbols it
/// actually holds.
fn block_centre(b: usize, block: usize, n_sym: usize) -> f64 {
    let k0 = b.saturating_mul(block);
    let end = k0.saturating_add(block).min(n_sym).max(k0 + 1);
    k0 as f64 + (end - k0 - 1) as f64 / 2.0
}

/// Timing offset per block (unwrapped, in samples): symbol `k` is sampled at
/// `k * period + offset(k)`.
#[derive(Debug, Clone)]
struct Timing {
    tau: Vec<f64>,
    block: usize,
    /// Whole symbols in the track the blocks cover; the last block may hold fewer
    /// than `block` of them.
    n_sym: usize,
}

impl Timing {
    /// Shift the offsets by whole symbols so the first instant lies in
    /// `0..period`: symbol 0 is then the first symbol of the track.
    fn normalised(mut tau: Vec<f64>, block: usize, n_sym: usize, period: f64) -> Self {
        let mut t = Self {
            tau: Vec::new(),
            block,
            n_sym,
        };
        std::mem::swap(&mut t.tau, &mut tau);
        let shift = period * (t.offset(0) / period).floor();
        for v in &mut t.tau {
            *v -= shift;
        }
        t
    }

    /// Centre, in symbols, of the symbols measured in block `b`.
    fn centre(&self, b: usize) -> f64 {
        block_centre(b, self.block, self.n_sym)
    }

    /// Offset at symbol `k`, linear between block centres and extended with the end
    /// slopes.
    fn offset(&self, k: usize) -> f64 {
        self.offset_at(k as f64)
    }

    /// Offset at fractional symbol index `x`.
    fn offset_at(&self, x: f64) -> f64 {
        let n = self.tau.len();
        let at = |b: usize| self.tau.get(b).copied().unwrap_or(0.0);
        if n < 2 {
            return at(0);
        }
        // Every block but the last is full, so the centres are evenly spaced up to
        // the last one, which may sit closer to its predecessor.
        let pos = (x - self.centre(0)) / self.block.max(1) as f64;
        let b = (pos.floor().max(0.0) as usize).min(n - 2);
        let (c0, c1) = (self.centre(b), self.centre(b + 1));
        let f = if c1 > c0 { (x - c0) / (c1 - c0) } else { 0.0 };
        at(b) + f * (at(b + 1) - at(b))
    }

    /// Least-squares slope of the offset against the block centres, in samples per
    /// symbol.
    fn slope_per_symbol(&self) -> f64 {
        let n = self.tau.len();
        if n < 2 {
            return 0.0;
        }
        let mx = (0..n).map(|b| self.centre(b)).sum::<f64>() / n as f64;
        let my = self.tau.iter().sum::<f64>() / n as f64;
        let (mut sxy, mut sxx) = (0.0, 0.0);
        for (i, y) in self.tau.iter().enumerate() {
            let dx = self.centre(i) - mx;
            sxy += dx * (y - my);
            sxx += dx * dx;
        }
        if sxx > 0.0 { sxy / sxx } else { 0.0 }
    }
}

/// Linear interpolation of `track` at fractional sample `t`.
fn sample_at(track: &[f32], t: f64) -> Option<f64> {
    if t.is_nan() || t < 0.0 {
        return None;
    }
    let i = t.floor() as usize;
    let f = t - i as f64;
    let a = f64::from(*track.get(i)?);
    match track.get(i + 1) {
        Some(b) => Some(a + f * (f64::from(*b) - a)),
        None if f < 1e-9 => Some(a),
        None => None,
    }
}

/// Centred moving average with a window exactly `w` samples long. For an even `w`
/// the window covers `w + 1` samples and the two end samples count half, so it stays
/// centred on the output sample. Near the ends the window is cut to the samples that
/// exist and the weights renormalised.
fn moving_average(v: &[f32], w: usize) -> Vec<f32> {
    let w = w.max(1);
    let half = w / 2;
    let even = w.is_multiple_of(2);
    let mut prefix = Vec::with_capacity(v.len() + 1);
    prefix.push(0.0_f64);
    let mut acc = 0.0_f64;
    for x in v {
        acc += f64::from(*x);
        prefix.push(acc);
    }
    let n = v.len();
    let at = |i: usize| v.get(i).copied().map_or(0.0, f64::from);
    (0..n)
        .map(|i| {
            let a = i.saturating_sub(half);
            let b = (i + half + 1).min(n);
            let mut s =
                prefix.get(b).copied().unwrap_or(0.0) - prefix.get(a).copied().unwrap_or(0.0);
            let mut weight = b.saturating_sub(a) as f64;
            if even {
                // Half weight on each end sample of the full window that exists.
                if i >= half {
                    s -= 0.5 * at(a);
                    weight -= 0.5;
                }
                if i + half < n {
                    s -= 0.5 * at(i + half);
                    weight -= 0.5;
                }
            }
            (s / weight.max(0.5)) as f32
        })
        .collect()
}

/// Timing measurement of one block: phase in `0..period` and weight in `0..=1`.
#[derive(Debug, Clone, Copy)]
struct Meas {
    phase: f64,
    weight: f64,
}

/// Summed squared decision distance per trial phase over the symbols `ks`, and the
/// number of symbols summed. Symbol `k` is tried at `phi + k * period +
/// base.offset(k)`. A symbol counts only when every trial instant lies in the track,
/// so all trial phases average exactly the same symbols, also near the end of the
/// track and with a base timing that pushes the last instants past it.
fn phase_costs(
    track: &[f32],
    period: f64,
    ks: std::ops::Range<usize>,
    layout: &DecisionLayout,
    base: Option<&Timing>,
) -> ([f64; PHASE_STEPS], usize) {
    let mut cost = [0.0_f64; PHASE_STEPS];
    let mut count = 0_usize;
    let mut d2 = [0.0_f64; PHASE_STEPS];
    'symbols: for k in ks {
        let start = k as f64 * period + base.map_or(0.0, |b| b.offset(k));
        for (g, slot) in d2.iter_mut().enumerate() {
            let phi = g as f64 * period / PHASE_STEPS as f64;
            match sample_at(track, start + phi) {
                Some(v) => {
                    let (_, d) = layout.margin(v);
                    *slot = d * d;
                }
                None => continue 'symbols,
            }
        }
        for (c, d) in cost.iter_mut().zip(d2) {
            *c += d;
        }
        count += 1;
    }
    (cost, count)
}

/// Measure the eye phase of symbols `k0..(k0 + block).min(n_sym)`, the same symbols
/// [`block_centre`] places the block at. Stopping at `n_sym` keeps the partial symbol
/// at the end of the track out, and [`phase_costs`] drops any symbol that some trial
/// phase cannot sample, so every trial phase averages the same symbols. With a `base`
/// timing the result is the residual phase relative to it.
fn measure_block(
    track: &[f32],
    period: f64,
    k0: usize,
    block: usize,
    n_sym: usize,
    layout: &DecisionLayout,
    base: Option<&Timing>,
) -> Meas {
    let end = k0.saturating_add(block).min(n_sym).max(k0);
    let (mut cost, count) = phase_costs(track, period, k0..end, layout, base);
    if count < MIN_MEAS_SYMBOLS {
        return Meas {
            phase: 0.0,
            weight: 0.0,
        };
    }
    for c in &mut cost {
        *c /= count as f64;
    }
    let lo = cost.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = cost.iter().copied().fold(0.0_f64, f64::max);
    let contrast = if hi > 0.0 { (hi - lo) / hi } else { 0.0 };
    // Circular centroid of the open (low-cost) part of the eye.
    let mid = f64::midpoint(lo, hi);
    let (mut sx, mut sy) = (0.0, 0.0);
    for (g, c) in cost.iter().enumerate() {
        let w = (mid - c).max(0.0);
        let a = std::f64::consts::TAU * g as f64 / PHASE_STEPS as f64;
        sx += w * a.cos();
        sy += w * a.sin();
    }
    if contrast < MIN_CONTRAST || sx.hypot(sy) <= 0.0 {
        return Meas {
            phase: 0.0,
            weight: 0.0,
        };
    }
    let frac = sy.atan2(sx).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU;
    Meas {
        phase: frac * period,
        weight: contrast.min(1.0),
    }
}

/// The automatic timing search (see the module documentation).
struct TimingSearch<'a> {
    track: &'a [f32],
    period: f64,
    block: usize,
    max_clock_error: f64,
    should_stop: &'a (dyn Fn() -> bool + Sync),
}

impl TimingSearch<'_> {
    /// Block length of the first pass: the eye moves by at most [`COARSE_DRIFT`] of
    /// a symbol over it at the largest allowed clock error.
    fn coarse_block(&self) -> usize {
        if self.max_clock_error > 0.0 {
            let n = (COARSE_DRIFT / self.max_clock_error).floor();
            // A block already at or under the coarse minimum is used as it is.
            if n < self.block as f64 && self.block > MIN_COARSE_BLOCK {
                return (n as usize).clamp(MIN_COARSE_BLOCK, self.block);
            }
        }
        self.block
    }

    /// Whole symbols in the track.
    fn n_sym(&self) -> usize {
        symbol_count(self.track.len(), self.period)
    }

    /// Centres of the blocks of `block` symbols, in units of `block`.
    fn positions(&self, block: usize, n_blocks: usize) -> Vec<f64> {
        let n_sym = self.n_sym();
        (0..n_blocks)
            .map(|b| block_centre(b, block, n_sym) / block.max(1) as f64)
            .collect()
    }

    /// Measure every block of `block` symbols, optionally relative to `base`.
    /// `layouts` holds the level layout per configured block (empty: use `layout`).
    fn measure(
        &self,
        block: usize,
        layout: &DecisionLayout,
        layouts: &[DecisionLayout],
        base: Option<&Timing>,
    ) -> Result<Vec<Meas>> {
        let n_sym = self.n_sym();
        let n_blocks = n_sym.div_ceil(block.max(1));
        let mut meas = Vec::with_capacity(n_blocks);
        for b in 0..n_blocks {
            if (self.should_stop)() {
                return Err(Error::Cancelled);
            }
            let k0 = b * block;
            let lay = layouts.get(k0 / self.block).unwrap_or(layout);
            meas.push(measure_block(
                self.track,
                self.period,
                k0,
                block,
                n_sym,
                lay,
                base,
            ));
        }
        Ok(meas)
    }

    /// Timing from blocks of `block` symbols measured on the plain grid.
    fn single_pass(
        &self,
        block: usize,
        layout: &DecisionLayout,
        layouts: &[DecisionLayout],
    ) -> Result<Timing> {
        let meas = self.measure(block, layout, layouts, None)?;
        let max_rate = self.max_clock_error * block as f64 * self.period;
        let pos = self.positions(block, meas.len());
        Ok(Timing::normalised(
            track_phases(&meas, &pos, self.period, max_rate),
            block,
            self.n_sym(),
            self.period,
        ))
    }

    /// Candidate timings: the plain full-block search and, when the first pass needs
    /// shorter blocks, the drift-following progressive search.
    fn candidates(
        &self,
        layout: &DecisionLayout,
        layouts: &[DecisionLayout],
    ) -> Result<Vec<Timing>> {
        let direct = self.single_pass(self.block, layout, layouts)?;
        let coarse = self.coarse_block();
        if coarse == self.block {
            return Ok(vec![direct]);
        }
        // First pass: short blocks, so the drift can neither smear them nor confuse
        // the unwrapping.
        let mut timing = self.single_pass(coarse, layout, layouts)?;
        // Then double the block length step by step up to the configured one. Each
        // step measures on the drifting grid of the previous one, so it only sees
        // the residual drift left by that step, which is small compared with its
        // block, and it measures that residual with the better SNR of longer blocks.
        let mut len = coarse;
        while len < self.block {
            len = len.saturating_mul(2).min(self.block);
            timing = self.refine(&timing, len, layout, layouts)?;
        }
        Ok(vec![timing, direct])
    }

    /// Timing from blocks of `len` symbols measured relative to `base`: the base
    /// offset at each block centre plus the residual measured there.
    fn refine(
        &self,
        base: &Timing,
        len: usize,
        layout: &DecisionLayout,
        layouts: &[DecisionLayout],
    ) -> Result<Timing> {
        let period = self.period;
        // The residual phases lie near 0, so take them in -period/2..period/2.
        let mut meas = self.measure(len, layout, layouts, Some(base))?;
        for m in &mut meas {
            if m.phase > period / 2.0 {
                m.phase -= period;
            }
        }
        let residual = if meas.iter().any(|m| m.weight > 0.0) {
            let max_rate = self.max_clock_error * len as f64 * period;
            let pos = self.positions(len, meas.len());
            track_phases(&meas, &pos, period, max_rate)
        } else {
            vec![0.0; meas.len()]
        };
        let n_sym = self.n_sym();
        let tau = residual
            .iter()
            .enumerate()
            .map(|(b, r)| base.offset_at(block_centre(b, len, n_sym)) + r)
            .collect();
        Ok(Timing::normalised(tau, len, n_sym, period))
    }

    /// Decide with each candidate timing and keep the result whose values sit
    /// closest to their level centres (the short blocks of the first pass are noisier,
    /// so with little drift the plain full-block search can be the better one).
    fn decide_best(
        &self,
        full_track: &[f32],
        layout: &DecisionLayout,
        layouts: &[DecisionLayout],
        adapt: Option<f64>,
    ) -> Result<(Timing, Decisions)> {
        let mut best: Option<(f64, Timing, Decisions)> = None;
        for timing in self.candidates(layout, layouts)? {
            let dec = decide_all(
                full_track,
                self.period,
                &timing,
                layout,
                adapt,
                self.should_stop,
            )?;
            let n = dec.margins.len().max(1) as f64;
            let cost = dec.margins.iter().map(|d| d * d).sum::<f64>() / n;
            if best.as_ref().is_none_or(|(c, _, _)| cost < *c) {
                best = Some((cost, timing, dec));
            }
        }
        best.map(|(_, t, d)| (t, d)).ok_or_else(|| {
            invalid(
                "signal",
                "no symbol timing could be found; select a longer fragment",
            )
        })
    }
}

/// Unwrap and smooth block phases into a timing offset per block. `pos` holds the
/// centre of each block in units of the block length (the last block may be short,
/// so its centre sits less than one unit after its predecessor's); `max_rate` is in
/// samples per unit.
fn track_phases(meas: &[Meas], pos: &[f64], period: f64, max_rate: f64) -> Vec<f64> {
    let at = |b: usize| pos.get(b).copied().unwrap_or(b as f64);
    let n = meas.len();
    let Some((b0, best)) = meas
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.weight.total_cmp(&b.1.weight))
    else {
        return Vec::new();
    };
    if best.weight <= 0.0 {
        // No transitions anywhere (one long run): sample mid-symbol.
        return vec![period / 2.0; n];
    }
    let unwrap = |m: &Meas, pred: f64| m.phase + period * ((pred - m.phase) / period).round();
    let mut filt = vec![0.0_f64; n];
    let mut unwrapped = vec![0.0_f64; n];
    let mut weights: Vec<f64> = meas.iter().map(|m| m.weight).collect();

    // Forward from the most confident block, then backward with the learnt rate.
    let mut rate = 0.0_f64;
    for pass in 0..2 {
        let mut tau = best.phase;
        let mut r_state = rate;
        if let (Some(f), Some(u)) = (filt.get_mut(b0), unwrapped.get_mut(b0)) {
            *f = tau;
            *u = tau;
        }
        for b in b0 + 1..n {
            let pred = tau + r_state * (at(b) - at(b - 1));
            let m = meas.get(b).copied().unwrap_or(Meas {
                phase: 0.0,
                weight: 0.0,
            });
            let u = unwrap(&m, pred);
            let r = u - pred;
            tau = pred + TRACK_ALPHA * m.weight * r;
            r_state = (r_state + TRACK_BETA * m.weight * r).clamp(-max_rate, max_rate);
            if pass == 1
                && let (Some(f), Some(uu)) = (filt.get_mut(b), unwrapped.get_mut(b))
            {
                *f = tau;
                *uu = u;
            }
        }
        if pass == 0 {
            rate = r_state;
            continue;
        }
        let mut tau = best.phase;
        let mut r_state = rate;
        for b in (0..b0).rev() {
            let pred = tau - r_state * (at(b + 1) - at(b));
            let m = meas.get(b).copied().unwrap_or(Meas {
                phase: 0.0,
                weight: 0.0,
            });
            let u = unwrap(&m, pred);
            let r = u - pred;
            tau = pred + TRACK_ALPHA * m.weight * r;
            r_state = (r_state - TRACK_BETA * m.weight * r).clamp(-max_rate, max_rate);
            if let (Some(f), Some(uu)) = (filt.get_mut(b), unwrapped.get_mut(b)) {
                *f = tau;
                *uu = u;
            }
        }
    }

    // Outliers (a measurement far from the tracker) get no weight in the smoothing.
    for ((w, u), f) in weights.iter_mut().zip(&unwrapped).zip(&filt) {
        if (u - f).abs() > period / 4.0 {
            *w = 0.0;
        }
    }

    // Weighted local linear fit of the unwrapped measurements.
    (0..n)
        .map(|b| {
            let lo = b.saturating_sub(SMOOTH_BLOCKS);
            let hi = (b + SMOOTH_BLOCKS + 1).min(n);
            let (mut sw, mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for j in lo..hi {
                let x = at(j) - at(b);
                let k = (1.0 - x.abs() / (SMOOTH_BLOCKS as f64 + 1.0)).max(0.0);
                let w = k * weights.get(j).copied().unwrap_or(0.0);
                let y = unwrapped.get(j).copied().unwrap_or(0.0);
                sw += w;
                sx += w * x;
                sy += w * y;
                sxx += w * x * x;
                sxy += w * x * y;
            }
            let fallback = filt.get(b).copied().unwrap_or(0.0);
            if sw < 0.3 {
                return fallback;
            }
            let det = sw * sxx - sx * sx;
            if det.abs() > 1e-9 * sw * sw {
                // Intercept at x = 0 of the weighted line.
                (sy * sxx - sx * sxy) / det
            } else {
                sy / sw
            }
        })
        .collect()
}

/// Decisions for every symbol whose instant lies inside the track.
#[derive(Debug, Clone)]
struct Decisions {
    symbols: Vec<u8>,
    instants: Vec<f64>,
    values: Vec<f32>,
    margins: Vec<f64>,
    /// Level layout in force at the start of each timing block.
    block_layouts: Vec<DecisionLayout>,
    final_levels: Vec<f64>,
}

fn decide_all(
    track: &[f32],
    period: f64,
    timing: &Timing,
    layout: &DecisionLayout,
    adapt: Option<f64>,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Decisions> {
    // Room for a fast clock (more symbols than nominal); the loop stops at the end.
    let max_k = (track.len() as f64 / period * 1.06).ceil() as usize + 2;
    let mut lay = layout.clone();
    let mut out = Decisions {
        symbols: Vec::with_capacity(max_k),
        instants: Vec::with_capacity(max_k),
        values: Vec::with_capacity(max_k),
        margins: Vec::with_capacity(max_k),
        block_layouts: Vec::new(),
        final_levels: Vec::new(),
    };
    for k in 0..max_k {
        if k % timing.block == 0 {
            if should_stop() {
                return Err(Error::Cancelled);
            }
            out.block_layouts.push(lay.clone());
        }
        let t = k as f64 * period + timing.offset(k);
        if t >= track.len() as f64 {
            break;
        }
        let Some(v) = sample_at(track, t) else {
            continue;
        };
        let (j, d) = lay.margin(v);
        out.symbols.push(u8::try_from(j).unwrap_or(u8::MAX));
        out.instants.push(t);
        out.values.push(v as f32);
        out.margins.push(d);
        if let Some(mu) = adapt {
            adapt_levels(&mut lay, j, v, mu);
        }
    }
    out.final_levels = lay.centres;
    Ok(out)
}

/// One running k-means step: the chosen centre moves `mu` of the way to `v`, the
/// others a quarter of that. Rejected if it would make centres touch or cross.
fn adapt_levels(lay: &mut DecisionLayout, j: usize, v: f64, mu: f64) {
    let Some(c) = lay.centres.get(j).copied() else {
        return;
    };
    let e = v - c;
    let new: Vec<f64> = lay
        .centres
        .iter()
        .enumerate()
        .map(|(i, x)| {
            if i == j {
                x + mu * e
            } else {
                x + mu * COMMON_SHARE * e
            }
        })
        .collect();
    let ordered = new.windows(2).all(|w| match w {
        [a, b] => b > a,
        _ => true,
    });
    if ordered {
        *lay = DecisionLayout::from_sorted(new);
    }
}

/// Eye diagram: the track folded at two symbols, as traces for display.
#[derive(Debug, Clone, PartialEq)]
pub struct EyeDiagram {
    /// Samples per symbol of the track.
    pub samples_per_symbol: f64,
    /// Traces, each `points` values covering two symbols. A trace starts half a
    /// symbol before a sampling instant, so the instants fall at 1/4 and 3/4 of it.
    pub traces: Vec<Vec<f32>>,
    /// Values per trace.
    pub points: usize,
}

impl EyeDiagram {
    /// Horizontal position of point `i`, in symbols from the trace start (0..=2).
    pub fn x_of(&self, i: usize) -> f64 {
        if self.points < 2 {
            0.0
        } else {
            2.0 * i as f64 / (self.points - 1) as f64
        }
    }
}

/// Fold `track` at two symbols around the given sampling `instants` (from
/// [`Demodulated::instants`], so drift is already followed). At most `max_traces`
/// traces are made, spread evenly over the instants, each with at most `max_points`
/// values (at least 2).
pub fn eye_diagram(
    track: &[f32],
    instants: &[f64],
    samples_per_symbol: f64,
    max_traces: usize,
    max_points: usize,
) -> EyeDiagram {
    let t = samples_per_symbol.max(1.0);
    let points = ((2.0 * t).ceil() as usize + 1).clamp(2, max_points.max(2));
    let stride = instants.len().div_ceil(max_traces.max(1)).max(1);
    let traces = instants
        .iter()
        .step_by(stride)
        .filter_map(|&c| {
            let start = c - t / 2.0;
            (0..points)
                .map(|i| {
                    let x = start + 2.0 * t * i as f64 / (points - 1) as f64;
                    sample_at(track, x).map(|v| v as f32)
                })
                .collect::<Option<Vec<f32>>>()
        })
        .take(max_traces)
        .collect();
    EyeDiagram {
        samples_per_symbol: t,
        traces,
        points,
    }
}

/// Levels suggested from the histogram of a track (SA: WF "Sum" plus Count freqs).
#[derive(Debug, Clone, PartialEq)]
pub struct LevelSuggestion {
    /// Histogram peaks (observed tones), lowest first.
    pub tones: Vec<f64>,
    /// Regular grid through the outermost tones that every tone fits.
    pub grid: Levels,
}

/// Suggest decision levels from the histogram of samples `start..end` of a frequency
/// track. `smooth` is the histogram smoothing in value units (Hz), a few percent of
/// the tone spacing.
pub fn suggest_levels(
    signal: &Signal,
    start: usize,
    end: usize,
    smooth: f64,
) -> Result<LevelSuggestion> {
    let h = sn_analysis::histogram::Histogram::build(signal, start, end, 8000, None)?;
    let mut tones = h.peaks(0.15, smooth);
    tones.sort_by(f64::total_cmp);
    // The tones are only as accurate as the histogram resolution or the smoothing.
    let tol_hz = (2.0 * h.bin_width()).max(smooth);
    let grid = sn_analysis::tones::count_grid_hz(&tones, None, None, tol_hz)?;
    Ok(LevelSuggestion {
        tones,
        grid: grid_levels(&grid)?,
    })
}

/// Decision levels for a tone grid found by Count freqs. A grid with more positions
/// than the demodulator has levels is refused rather than re-spaced.
fn grid_levels(grid: &sn_analysis::tones::ToneGrid) -> Result<Levels> {
    if grid.size > MAX_LEVELS {
        return Err(invalid(
            "levels",
            format!(
                "the tones fit a grid of {} positions, but the demodulator supports at most {MAX_LEVELS}; narrow the selection to fewer tones or place the level markers by hand",
                grid.size
            ),
        ));
    }
    Ok(Levels::Grid {
        lo: grid.lowest,
        hi: grid.lowest + grid.spacing * (grid.size as f64 - 1.0),
        count: grid.size,
    })
}

#[cfg(test)]
mod synthetic_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_levels_and_layout() {
        let c = Levels::Grid {
            lo: 1000.0,
            hi: 1300.0,
            count: 4,
        }
        .centres()
        .unwrap();
        assert_eq!(c, vec![1000.0, 1100.0, 1200.0, 1300.0]);
        let l = DecisionLayout::new(c).unwrap();
        assert_eq!(l.thresholds(), &[1050.0, 1150.0, 1250.0]);
        assert_eq!(l.main_lines(), (1000.0, 1300.0));
        assert_eq!(l.extra_lines(), &[1100.0, 1200.0]);
        assert_eq!(l.decide(900.0), 0);
        assert_eq!(l.decide(1149.0), 1);
        assert_eq!(l.decide(1151.0), 2);
        assert_eq!(l.decide(5000.0), 3);
        let (j, d) = l.margin(1125.0);
        assert_eq!(j, 1);
        assert!((d - 0.5).abs() < 1e-12);
        let (j, d) = l.margin(800.0);
        assert_eq!(j, 0);
        assert!((d + 1.0).abs() < 1e-12);
    }

    #[test]
    fn bad_levels_are_rejected() {
        assert!(DecisionLayout::new(vec![1.0]).is_err());
        assert!(DecisionLayout::new(vec![1.0, 1.0]).is_err());
        assert!(DecisionLayout::new(vec![1.0, f64::NAN]).is_err());
        assert!(
            Levels::Grid {
                lo: 2.0,
                hi: 1.0,
                count: 4
            }
            .centres()
            .is_err()
        );
        assert!(
            Levels::Grid {
                lo: 0.0,
                hi: 1.0,
                count: 257
            }
            .centres()
            .is_err()
        );
    }

    #[test]
    fn sample_at_interpolates() {
        let v = [0.0_f32, 2.0, 4.0];
        assert_eq!(sample_at(&v, 0.5), Some(1.0));
        assert_eq!(sample_at(&v, 2.0), Some(4.0));
        assert_eq!(sample_at(&v, 2.5), None);
        assert_eq!(sample_at(&v, -0.1), None);
    }

    #[test]
    fn timing_offset_interpolates_between_blocks() {
        let t = Timing {
            tau: vec![0.0, 10.0],
            block: 10,
            n_sym: 20,
        };
        assert!((t.offset(0) + 4.5).abs() < 1e-9);
        assert!((t.offset(14) - 9.5).abs() < 1e-9);
        assert!((t.slope_per_symbol() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn short_last_block_sits_at_the_centre_of_its_symbols() {
        // Blocks of 8, 8 and 4 symbols: centres 3.5, 11.5 and 17.5.
        assert!((block_centre(2, 8, 20) - 17.5).abs() < 1e-12);
        // A timing drifting by 1 sample per symbol, measured at those centres.
        let t = Timing {
            tau: vec![3.5, 11.5, 17.5],
            block: 8,
            n_sym: 20,
        };
        assert!((t.slope_per_symbol() - 1.0).abs() < 1e-12);
        for k in [0_usize, 5, 12, 16, 19, 25] {
            assert!((t.offset(k) - k as f64).abs() < 1e-9, "offset({k})");
        }
    }

    #[test]
    fn last_block_measures_the_same_symbols_at_every_phase() {
        // A track of 20.7 symbols of 10 samples: n_sym is 20 and symbol 20 lies in
        // the track only for trial phases under 7 samples.
        let period = 10.0;
        let track: Vec<f32> = (0..207).map(|i| ((i / 10) % 2) as f32).collect();
        let n_sym = symbol_count(track.len(), period);
        assert_eq!(n_sym, 20);
        let layout = DecisionLayout::new(vec![0.0, 1.0]).unwrap();
        let end = (16_usize + 8).min(n_sym);
        let (_, count) = phase_costs(&track, period, 16..end, &layout, None);
        assert_eq!(count, 4);
        // Symbols past n_sym are sampled by only some phases and are left out.
        let (_, past) = phase_costs(&track, period, 16..24, &layout, None);
        assert_eq!(past, 4);
        // The measured block holds enough symbols to give a weighted result.
        let m = measure_block(&track, period, 16, 8, n_sym, &layout, None);
        assert!(m.weight >= 0.0 && m.phase.is_finite());
    }

    #[test]
    fn last_symbol_is_dropped_when_some_phase_runs_past_the_end() {
        // Exactly 20 symbols of 10 samples: symbol 19 at phase 9.6875 needs sample
        // 200, past the end, so it is left out for every phase.
        let period = 10.0;
        let track: Vec<f32> = (0..200).map(|i| ((i / 10) % 2) as f32).collect();
        assert_eq!(symbol_count(track.len(), period), 20);
        let layout = DecisionLayout::new(vec![0.0, 1.0]).unwrap();
        let (_, count) = phase_costs(&track, period, 16..20, &layout, None);
        assert_eq!(count, 3);
    }

    #[test]
    fn base_offset_near_a_period_keeps_symbol_sets_equal() {
        // An offset of 9.5 samples moves the late trial instants of the last
        // symbol past the end, so it is dropped for every phase.
        let period = 10.0;
        let track: Vec<f32> = (0..207).map(|i| ((i / 10) % 2) as f32).collect();
        let n_sym = symbol_count(track.len(), period);
        let base = Timing {
            tau: vec![9.5],
            block: 8,
            n_sym,
        };
        let layout = DecisionLayout::new(vec![0.0, 1.0]).unwrap();
        let (_, count) = phase_costs(&track, period, 12..20, &layout, Some(&base));
        // Symbol 18: 189.5 + 9.6875 = 199.19 fits; symbol 19: 209.19 does not.
        assert_eq!(count, 7);
        let (cost, _) = phase_costs(&track, period, 12..20, &layout, Some(&base));
        assert!(cost.iter().all(|c| c.is_finite()));
    }

    #[test]
    fn track_phases_uses_the_short_last_block_spacing() {
        // Constant drift of 8 samples per full block; the last block holds 4 symbols.
        let pos = [3.5 / 8.0, 11.5 / 8.0, 17.5 / 8.0];
        let meas: Vec<Meas> = pos
            .iter()
            .map(|p| Meas {
                phase: 2.0 + 8.0 * p,
                weight: 1.0,
            })
            .collect();
        let tau = track_phases(&meas, &pos, 100.0, 20.0);
        for (t, p) in tau.iter().zip(pos) {
            assert!((t - (2.0 + 8.0 * p)).abs() < 1e-6, "{t} at {p}");
        }
    }

    #[test]
    fn very_short_selections_keep_two_measurable_blocks() {
        // 10 symbols of 10 samples: 8 would leave a 2-symbol block with no weight.
        assert_eq!(effective_block(64, 100, 10.0), 5);
        assert_eq!(effective_block(64, 90, 10.0), 4);
        // 12 symbols: 8 + 4 is measurable.
        assert_eq!(effective_block(64, 120, 10.0), 8);
        // Too short to cut in two measurable halves.
        assert_eq!(effective_block(64, 70, 10.0), 8);
        assert_eq!(effective_block(64, 4000, 10.0), 64);
    }

    #[test]
    fn adaptive_step_keeps_order() {
        let mut l = DecisionLayout::new(vec![0.0, 1.0]).unwrap();
        adapt_levels(&mut l, 0, 0.4, 0.5);
        assert!((l.centres()[0] - 0.2).abs() < 1e-12);
        assert!((l.centres()[1] - 1.05).abs() < 1e-12);
        let before = l.clone();
        adapt_levels(&mut l, 0, 100.0, 0.5);
        assert_eq!(l, before);
    }

    #[test]
    fn eye_diagram_is_bounded() {
        let track: Vec<f32> = (0..10_000).map(|i| ((i / 10) % 2) as f32).collect();
        let instants: Vec<f64> = (1..999).map(|k| f64::from(k) * 10.0 + 5.0).collect();
        let eye = eye_diagram(&track, &instants, 10.0, 50, 400);
        assert!(eye.traces.len() <= 50 && !eye.traces.is_empty());
        assert_eq!(eye.points, 21);
        assert!(eye.traces.iter().all(|t| t.len() == 21));
        assert!((eye.x_of(20) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn moving_average_window_has_the_requested_length() {
        for w in 1..=9 {
            let mut impulse = vec![0.0_f32; 41];
            impulse[20] = 1.0;
            let out = moving_average(&impulse, w);
            let sum: f32 = out.iter().sum();
            let peak = out.iter().copied().fold(0.0_f32, f32::max);
            assert!((sum - 1.0).abs() < 1e-6, "w {w}: gain {sum}");
            // A box of length w has height 1 / w.
            assert!(
                (1.0 / peak - w as f32).abs() < 1e-4,
                "w {w}: length {}",
                1.0 / peak
            );
            // Centred: the response is symmetric about the impulse.
            for d in 1..=w {
                assert!((out[20 - d] - out[20 + d]).abs() < 1e-7, "w {w}");
            }
        }
        // A constant stays constant up to the edges.
        let flat = moving_average(&[2.0_f32; 10], 4);
        assert!(flat.iter().all(|v| (v - 2.0).abs() < 1e-6), "{flat:?}");
    }

    #[test]
    fn grids_beyond_256_levels_are_refused() {
        let tones: Vec<f64> = (0..300).map(|k| 1000.0 + 10.0 * f64::from(k)).collect();
        let grid = sn_analysis::tones::count_grid(&tones, None, None, 0.2).unwrap();
        assert_eq!(grid.size, 300);
        let err = grid_levels(&grid).unwrap_err();
        assert!(
            matches!(&err, Error::Invalid { name: "levels", reason } if reason.contains("300")),
            "{err}"
        );
        let tones: Vec<f64> = (0..256).map(|k| 1000.0 + 10.0 * f64::from(k)).collect();
        let grid = sn_analysis::tones::count_grid(&tones, None, None, 0.2).unwrap();
        let Levels::Grid { lo, hi, count } = grid_levels(&grid).unwrap() else {
            panic!("expected a grid")
        };
        assert_eq!(count, 256);
        assert!((lo - 1000.0).abs() < 1e-9 && (hi - 3550.0).abs() < 1e-6);
    }
}
