//! Linear modulations (PSK and QAM) with pulse shaping, real or complex output.
//!
//! The complex envelope is `b(t) = sum_k a_k p(t - t_k)`, where `a_k` is the value of
//! symbol `k` (after differential encoding when asked), `p` the pulse and `t_k` the
//! symbol's reference instant. The pulse is evaluated at the exact time of every output
//! sample, so the samples per symbol (`sample_rate / baud`) need not be an integer.
//!
//! Timing: with [`Shaping::Rectangular`] symbol `k` covers `k / baud` to `(k + 1) / baud`
//! seconds (centre `(k + 0.5) / baud`) and the signal is `count / baud` seconds long.
//! With a raised-cosine or root-raised-cosine pulse the pulse is truncated (without a
//! window) to `span = `[`Shaping::span`] symbols, symbol `k` is centred at
//! `(k + span / 2) / baud` and the signal is `(count + span) / baud` seconds long, so
//! every pulse fits whole. The span is [`PULSE_SPAN`] for a roll-off of 0.2 and above
//! and grows as the roll-off shrinks (up to [`MAX_PULSE_SPAN`]), which keeps the
//! truncation ISI after a matched filter about 40 dB or more below the signal.
//! [`LinearOutput::first_symbol_time`] gives the centre of symbol 0 in every case.
//!
//! Level: `amplitude` is the RMS level of the output (for long random data). The pulse
//! is scaled to unit energy per symbol and every constellation has unit mean power.
//! A real output is `sqrt(2) * amplitude * Re{b(t) exp(j 2 pi fc t)}`, a complex one
//! `amplitude * b(t) exp(j 2 pi fc t)`.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, SQRT_2, TAU};

use rustfft::num_complex::Complex64;
use serde::{Deserialize, Serialize};
use sn_core::{Complex32, Signal};

use crate::{
    Error, Result, StopCheck, check_amplitude, check_sample_rate, check_stop, never_stop,
    output_len, require, require_positive, symbol_index,
};

/// Shortest length of the raised-cosine and root-raised-cosine pulses, in symbols
/// (the length for a roll-off of 0.2 and above; see [`Shaping::span`]).
pub const PULSE_SPAN: usize = 12;

/// Longest length of the raised-cosine and root-raised-cosine pulses, in symbols.
pub const MAX_PULSE_SPAN: usize = 240;

/// Points per symbol of the tabulated pulse.
const PULSE_TABLE_STEPS: usize = 1024;

/// Points per symbol used to integrate the pulse energy.
const ENERGY_STEPS_PER_SYMBOL: usize = 64;

/// Constellation of a linear modulation. Every constellation has unit mean power.
///
/// Symbol indices map to points with Gray coding where it is standard: PSK (adjacent
/// phases differ in one bit) and square QAM (I and Q each Gray coded, I in the high
/// bits). The cross constellations (QAM-32, QAM-128) cannot be fully Gray coded; their
/// indices run over the points row by row (I fastest, from the most negative I and Q).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Constellation {
    /// Two phases, 0 and 180 degrees.
    Bpsk,
    /// Four phases at 45, 135, 225 and 315 degrees.
    Qpsk,
    /// Eight phases from 0 degrees.
    Psk8,
    /// Sixteen phases from 0 degrees.
    Psk16,
    /// pi/4-DQPSK: each symbol advances the phase by 45 + 90 m degrees (always
    /// differential; the `differential` flag does not change it).
    Pi4Dqpsk,
    /// Square 16-QAM.
    Qam16,
    /// Cross 32-QAM (6 x 6 grid without the corners).
    Qam32,
    /// Square 64-QAM.
    Qam64,
    /// Cross 128-QAM (12 x 12 grid without 2 x 2 corners).
    Qam128,
    /// Square 256-QAM.
    Qam256,
}

/// Gray code of `p`.
fn gray(p: usize) -> usize {
    p ^ (p >> 1)
}

/// The position whose Gray code is `g`.
fn inverse_gray(g: usize) -> usize {
    let mut p = g;
    let mut shift = 1;
    while shift < usize::BITS {
        p ^= p >> shift;
        shift <<= 1;
    }
    p
}

impl Constellation {
    /// Every constellation, in a fixed order.
    pub const ALL: [Self; 10] = [
        Self::Bpsk,
        Self::Qpsk,
        Self::Psk8,
        Self::Psk16,
        Self::Pi4Dqpsk,
        Self::Qam16,
        Self::Qam32,
        Self::Qam64,
        Self::Qam128,
        Self::Qam256,
    ];

    /// Number of symbols.
    pub fn order(self) -> usize {
        match self {
            Self::Bpsk => 2,
            Self::Qpsk | Self::Pi4Dqpsk => 4,
            Self::Psk8 => 8,
            Self::Psk16 | Self::Qam16 => 16,
            Self::Qam32 => 32,
            Self::Qam64 => 64,
            Self::Qam128 => 128,
            Self::Qam256 => 256,
        }
    }

    /// True for the phase-only constellations (the ones that can be differential).
    pub fn is_psk(self) -> bool {
        matches!(
            self,
            Self::Bpsk | Self::Qpsk | Self::Psk8 | Self::Psk16 | Self::Pi4Dqpsk
        )
    }

    /// Short English name, for example "QAM-16" or "8-PSK".
    pub fn name(self) -> &'static str {
        match self {
            Self::Bpsk => "BPSK",
            Self::Qpsk => "QPSK",
            Self::Psk8 => "8-PSK",
            Self::Psk16 => "16-PSK",
            Self::Pi4Dqpsk => "pi/4-DQPSK",
            Self::Qam16 => "QAM-16",
            Self::Qam32 => "QAM-32",
            Self::Qam64 => "QAM-64",
            Self::Qam128 => "QAM-128",
            Self::Qam256 => "QAM-256",
        }
    }

    /// Phase of PSK position 0 in radians (45 degrees for QPSK, 0 otherwise).
    fn psk_offset(self) -> f64 {
        if self == Self::Qpsk { FRAC_PI_4 } else { 0.0 }
    }

    /// The points, indexed by symbol. For [`Constellation::Pi4Dqpsk`] these are the four
    /// phase steps `exp(j (pi/4 + m pi/2))`, Gray coded.
    pub fn points(self) -> Vec<Complex64> {
        let m = self.order();
        match self {
            Self::Bpsk | Self::Qpsk | Self::Psk8 | Self::Psk16 => {
                let mut pts = vec![Complex64::new(0.0, 0.0); m];
                for p in 0..m {
                    if let Some(slot) = pts.get_mut(gray(p)) {
                        *slot = Complex64::from_polar(
                            1.0,
                            self.psk_offset() + TAU * p as f64 / m as f64,
                        );
                    }
                }
                pts
            }
            Self::Pi4Dqpsk => {
                let mut pts = vec![Complex64::new(0.0, 0.0); 4];
                for p in 0..4 {
                    if let Some(slot) = pts.get_mut(gray(p)) {
                        *slot = Complex64::from_polar(1.0, FRAC_PI_4 + p as f64 * FRAC_PI_2);
                    }
                }
                pts
            }
            Self::Qam16 | Self::Qam64 | Self::Qam256 => square_qam(m),
            Self::Qam32 => cross_qam(6, 1),
            Self::Qam128 => cross_qam(12, 2),
        }
    }

    /// Index of the point nearest to `z` (`z` on the unit-power scale).
    pub fn nearest(self, z: Complex64) -> usize {
        nearest_in(&self.points(), z)
    }

    /// Phase increment that differential symbol `s` applies: `2 pi p / M` with `p` the
    /// Gray position of `s` (for pi/4-DQPSK, `pi/4 + p pi/2`). `None` when `s` is not
    /// below the order.
    pub fn phase_step(self, s: usize) -> Option<f64> {
        let m = self.order();
        if s >= m {
            return None;
        }
        let p = inverse_gray(s);
        Some(if self == Self::Pi4Dqpsk {
            FRAC_PI_4 + p as f64 * FRAC_PI_2
        } else {
            TAU * p as f64 / m as f64
        })
    }
}

/// Index of the point of `points` nearest to `z` (0 for an empty list).
pub(crate) fn nearest_in(points: &[Complex64], z: Complex64) -> usize {
    points
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - z).norm_sqr().total_cmp(&(*b - z).norm_sqr()))
        .map_or(0, |(i, _)| i)
}

/// Square QAM of order `m` (16, 64 or 256) with Gray-coded I and Q, unit mean power.
fn square_qam(m: usize) -> Vec<Complex64> {
    let side = (m as f64).sqrt().round() as usize;
    let bits = side.trailing_zeros();
    let scale = (2.0 * (m as f64 - 1.0) / 3.0).sqrt();
    let level = |p: usize| (2.0 * p as f64 - (side as f64 - 1.0)) / scale;
    let mut pts = vec![Complex64::new(0.0, 0.0); m];
    for pi in 0..side {
        for pq in 0..side {
            let idx = (gray(pi) << bits) | gray(pq);
            if let Some(slot) = pts.get_mut(idx) {
                *slot = Complex64::new(level(pi), level(pq));
            }
        }
    }
    pts
}

/// Cross QAM: a `side` x `side` grid of odd levels without a `corner` x `corner` square
/// in each corner, unit mean power. Points run row by row (I fastest).
fn cross_qam(side: usize, corner: usize) -> Vec<Complex64> {
    let half = side as f64 - 1.0;
    let mut pts = Vec::with_capacity(side * side);
    for q in 0..side {
        for i in 0..side {
            let in_corner = |k: usize| k < corner || k >= side - corner;
            if in_corner(i) && in_corner(q) {
                continue;
            }
            pts.push(Complex64::new(2.0 * i as f64 - half, 2.0 * q as f64 - half));
        }
    }
    let power = pts.iter().map(Complex64::norm_sqr).sum::<f64>() / pts.len().max(1) as f64;
    let scale = power.sqrt();
    for p in &mut pts {
        *p /= scale;
    }
    pts
}

/// Pulse shape of a linear modulation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shaping {
    /// Rectangular pulses one symbol long (no shaping).
    Rectangular,
    /// Raised cosine (a Nyquist pulse: zero ISI when sampled at the symbol centres).
    RaisedCosine {
        /// Roll-off, 0.01 to 1.
        alpha: f64,
    },
    /// Root raised cosine (zero ISI after a matched RRC receive filter, apart from the
    /// small ISI the truncation to [`Shaping::span`] symbols leaves).
    RootRaisedCosine {
        /// Roll-off, 0.01 to 1.
        alpha: f64,
    },
}

impl Shaping {
    /// Roll-off, or `None` for rectangular pulses.
    pub fn alpha(self) -> Option<f64> {
        match self {
            Self::Rectangular => None,
            Self::RaisedCosine { alpha } | Self::RootRaisedCosine { alpha } => Some(alpha),
        }
    }

    /// Bandwidth the signal occupies in Hz: `baud (1 + alpha)` for shaped pulses, and
    /// the main lobe `2 baud` for rectangular pulses.
    pub fn occupied_bandwidth(self, baud: f64) -> f64 {
        self.alpha().map_or(2.0 * baud, |a| baud * (1.0 + a))
    }

    /// Length of the truncated pulse in symbols: 0 for rectangular pulses, otherwise
    /// `2 ceil(1.2 / alpha)`, at least [`PULSE_SPAN`] and at most [`MAX_PULSE_SPAN`]. The
    /// pulse thus reaches about 1.2 / alpha symbols either side of its centre, where
    /// its tail is small enough for every allowed roll-off.
    pub fn span(self) -> usize {
        match self.alpha() {
            None => 0,
            Some(a) => {
                // The small subtraction keeps 1.2 / 0.2 from rounding up to 7.
                let half = (1.2 / a - 1e-9).ceil();
                if half.is_finite() && half > 0.0 {
                    (2 * half as usize).clamp(PULSE_SPAN, MAX_PULSE_SPAN)
                } else {
                    MAX_PULSE_SPAN
                }
            }
        }
    }

    /// Short English name, for example "RRC 0.35".
    pub fn name(self) -> String {
        match self {
            Self::Rectangular => "rectangular".to_owned(),
            Self::RaisedCosine { alpha } => format!("RC {}", crate::spec::fmt_num(alpha)),
            Self::RootRaisedCosine { alpha } => {
                format!("RRC {}", crate::spec::fmt_num(alpha))
            }
        }
    }

    /// The pulse at `t` symbols from its centre (1 at the centre for raised cosine; the
    /// rectangular pulse is 1 on `-0.5 <= t < 0.5`). Not normalised.
    pub fn pulse(self, t: f64) -> f64 {
        match self {
            Self::Rectangular => {
                if (-0.5..0.5).contains(&t) {
                    1.0
                } else {
                    0.0
                }
            }
            Self::RaisedCosine { alpha } => raised_cosine(t, alpha),
            Self::RootRaisedCosine { alpha } => root_raised_cosine(t, alpha),
        }
    }

    fn check(self) -> Result<()> {
        if let Some(a) = self.alpha() {
            require(
                a.is_finite() && (0.01..=1.0).contains(&a),
                "alpha",
                format!("the roll-off must be between 0.01 and 1 (got {a})"),
            )?;
        }
        Ok(())
    }
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// Raised-cosine pulse at `t` symbols (T = 1).
pub(crate) fn raised_cosine(t: f64, alpha: f64) -> f64 {
    let d = 2.0 * alpha * t;
    if (d.abs() - 1.0).abs() < 1e-9 {
        FRAC_PI_4 * sinc(1.0 / (2.0 * alpha))
    } else {
        sinc(t) * (PI * alpha * t).cos() / (1.0 - d * d)
    }
}

/// Root-raised-cosine pulse at `t` symbols (T = 1), unit energy over all time.
fn root_raised_cosine(t: f64, alpha: f64) -> f64 {
    let a = alpha;
    if t.abs() < 1e-12 {
        return 1.0 - a + 4.0 * a / PI;
    }
    let x = 4.0 * a * t;
    if (x.abs() - 1.0).abs() < 1e-9 {
        let q = PI / (4.0 * a);
        return a / SQRT_2 * ((1.0 + 2.0 / PI) * q.sin() + (1.0 - 2.0 / PI) * q.cos());
    }
    ((PI * t * (1.0 - a)).sin() + x * (PI * t * (1.0 + a)).cos()) / (PI * t * (1.0 - x * x))
}

/// Energy of the truncated pulse over its span, in symbol units (Simpson's rule).
fn pulse_energy(shaping: Shaping) -> f64 {
    let span = shaping.span();
    let half = span as f64 / 2.0;
    let n = span * ENERGY_STEPS_PER_SYMBOL;
    let h = span as f64 / n.max(1) as f64;
    let mut sum = 0.0;
    for i in 0..=n {
        let w = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += w * shaping.pulse(-half + i as f64 * h).powi(2);
    }
    sum * h / 3.0
}

/// The pulse of `shaping` at `t = k / PULSE_TABLE_STEPS` symbols for `t` from 0 to
/// `half` (and one point beyond). RC and RRC pulses are even, so `t >= 0` suffices.
fn pulse_table(shaping: Shaping, half: f64) -> Vec<f64> {
    let len = (half * PULSE_TABLE_STEPS as f64).ceil() as usize + 2;
    (0..len)
        .map(|k| shaping.pulse(k as f64 / PULSE_TABLE_STEPS as f64))
        .collect()
}

/// Linear interpolation in a table made by [`pulse_table`]; 0 beyond it. The error is
/// below about 2e-6 of the peak.
fn table_pulse(table: &[f64], t: f64) -> f64 {
    let x = t.abs() * PULSE_TABLE_STEPS as f64;
    let k = x.floor();
    let frac = x - k;
    let k = k as usize;
    match (table.get(k), table.get(k + 1)) {
        (Some(a), Some(b)) => a + (b - a) * frac,
        _ => 0.0,
    }
}

/// Parameters of a PSK or QAM signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinearParams {
    /// Constellation.
    pub constellation: Constellation,
    /// Symbol rate in baud.
    pub baud: f64,
    /// Output sample rate in Hz.
    pub sample_rate: f64,
    /// Carrier in Hz. With complex output it is the offset from 0 Hz (may be 0 or
    /// negative).
    pub carrier: f64,
    /// RMS amplitude of the output.
    pub amplitude: f64,
    /// Pulse shape.
    pub shaping: Shaping,
    /// Encode each symbol as a phase increment (DPSK). PSK constellations only.
    pub differential: bool,
    /// Produce complex (I/Q) output instead of a real signal.
    pub complex: bool,
}

/// A generated linear-modulation signal and its ground truth.
#[derive(Debug, Clone)]
pub struct LinearOutput {
    /// The signal.
    pub signal: Signal,
    /// The symbol indices sent (the data, before differential encoding).
    pub symbols: Vec<usize>,
    /// The unit-power constellation value sent for each symbol (after differential
    /// encoding).
    pub values: Vec<Complex32>,
    /// Time of the centre of symbol 0 in seconds.
    pub first_symbol_time: f64,
    /// Length of the truncated pulse in symbols ([`Shaping::span`]; 0 for rectangular
    /// pulses).
    pub pulse_span: usize,
}

impl LinearParams {
    /// Check every field.
    pub fn validate(&self) -> Result<()> {
        let p = self;
        check_sample_rate(p.sample_rate)?;
        require_positive(p.baud, "baud", "Bd")?;
        check_amplitude(p.amplitude)?;
        p.shaping.check()?;
        require(
            !p.differential || p.constellation.is_psk(),
            "differential",
            format!(
                "differential encoding applies to PSK only; {} carries amplitude as well \
                 as phase, so it cannot be differential here",
                p.constellation.name()
            ),
        )?;
        require(
            p.carrier.is_finite(),
            "carrier",
            "must be a finite frequency",
        )?;
        // Half the occupied bandwidth: for rectangular pulses the main lobe, +-baud.
        let half_bw = p.shaping.occupied_bandwidth(p.baud) / 2.0;
        let nyquist = p.sample_rate / 2.0;
        if p.complex {
            require(
                p.carrier.abs() + half_bw < nyquist,
                "carrier",
                format!(
                    "the band {:.1} to {:.1} Hz must lie inside -{nyquist} to {nyquist} Hz; \
                     lower the offset or the baud rate",
                    p.carrier - half_bw,
                    p.carrier + half_bw
                ),
            )
        } else {
            require(
                p.carrier > half_bw && p.carrier + half_bw < nyquist,
                "carrier",
                format!(
                    "{} Hz with {} Bd does not fit between 0 Hz and Nyquist ({nyquist} Hz)",
                    p.carrier, p.baud
                ),
            )
        }
    }

    /// Number of output samples for `count` symbols (see the module docs for timing).
    pub fn output_samples(&self, count: usize) -> Result<usize> {
        let symbols = count as f64 + self.shaping.span() as f64;
        output_len(symbols * self.sample_rate / self.baud, "symbols")
    }

    /// Centre of symbol 0 in seconds (see the module docs for timing).
    pub fn first_symbol_time(&self) -> f64 {
        if self.shaping == Shaping::Rectangular {
            0.5 / self.baud
        } else {
            self.shaping.span() as f64 / 2.0 / self.baud
        }
    }

    /// Unit-power values sent for `symbols`, after differential encoding.
    fn encode(&self, symbols: &[usize]) -> Vec<Complex64> {
        let c = self.constellation;
        let points = c.points();
        if c == Constellation::Pi4Dqpsk || self.differential {
            let mut phase = c.psk_offset();
            symbols
                .iter()
                .map(|s| {
                    // `linear` has checked every symbol, so the step always exists.
                    phase = (phase + c.phase_step(*s).unwrap_or(0.0)).rem_euclid(TAU);
                    Complex64::from_polar(1.0, phase)
                })
                .collect()
        } else {
            symbols
                .iter()
                .map(|s| points.get(*s).copied().unwrap_or_default())
                .collect()
        }
    }
}

/// Generate a PSK or QAM signal carrying `symbols` (each below the constellation order).
pub fn linear(params: &LinearParams, symbols: &[usize]) -> Result<LinearOutput> {
    linear_with_stop(params, symbols, &never_stop)
}

/// [`linear`] that calls `stop` now and then and fails with [`Error::Cancelled`] when
/// it returns true.
pub fn linear_with_stop(
    params: &LinearParams,
    symbols: &[usize],
    stop: StopCheck<'_>,
) -> Result<LinearOutput> {
    let p = params;
    p.validate()?;
    require(
        !symbols.is_empty(),
        "symbols",
        "at least one symbol is needed",
    )?;
    let order = p.constellation.order();
    if let Some(bad) = symbols.iter().find(|s| **s >= order) {
        return Err(Error::InvalidParameter {
            name: "symbols",
            reason: format!("symbol {bad} is outside 0..{order}"),
        });
    }
    let n = p.output_samples(symbols.len())?;
    let values = p.encode(symbols);
    let cps = p.carrier / p.sample_rate;
    let shaped = p.shaping != Shaping::Rectangular;
    let span = p.shaping.span();
    let half = span as f64 / 2.0;
    let (gain, table) = if shaped {
        (
            p.amplitude / pulse_energy(p.shaping).sqrt(),
            pulse_table(p.shaping, half),
        )
    } else {
        (p.amplitude, Vec::new())
    };
    let envelope = |i: usize| -> Complex64 {
        if shaped {
            // Symbol units shifted by the ramp: symbol k is centred at u = k, i.e. at
            // time (k + span / 2) / baud.
            let u = i as f64 * p.baud / p.sample_rate - half;
            let lo = (u - half).ceil().max(0.0) as usize;
            let hi = (u + half).floor();
            if hi < 0.0 {
                return Complex64::new(0.0, 0.0);
            }
            let hi = (hi as usize).min(values.len().saturating_sub(1));
            let mut acc = Complex64::new(0.0, 0.0);
            for (k, v) in values.iter().enumerate().take(hi + 1).skip(lo) {
                acc += v * table_pulse(&table, u - k as f64);
            }
            acc
        } else {
            let k = symbol_index(i as f64, p.baud, p.sample_rate);
            values
                .get(k)
                .or_else(|| values.last())
                .copied()
                .unwrap_or_default()
        }
    };
    let rotate = |i: usize, b: Complex64| -> Complex64 {
        if cps == 0.0 {
            b
        } else {
            b * Complex64::from_polar(1.0, TAU * (i as f64 * cps).rem_euclid(1.0))
        }
    };
    let signal = if p.complex {
        let mut out: Vec<Complex32> = Vec::with_capacity(n);
        for i in 0..n {
            check_stop(stop, i)?;
            let z = rotate(i, envelope(i)) * gain;
            out.push(Complex32::new(z.re as f32, z.im as f32));
        }
        Signal::complex(out, p.sample_rate)?
    } else {
        let mut out: Vec<f32> = Vec::with_capacity(n);
        for i in 0..n {
            check_stop(stop, i)?;
            out.push((SQRT_2 * gain * rotate(i, envelope(i)).re) as f32);
        }
        Signal::real(out, p.sample_rate)?
    };
    let first_symbol_time = p.first_symbol_time();
    Ok(LinearOutput {
        signal,
        symbols: symbols.to_vec(),
        values: values
            .iter()
            .map(|v| Complex32::new(v.re as f32, v.im as f32))
            .collect(),
        first_symbol_time,
        pulse_span: span,
    })
}

#[cfg(test)]
mod tests {
    use rustfft::FftPlanner;
    use sn_core::Samples;

    use super::*;
    use crate::random_symbols;

    fn params(c: Constellation, shaping: Shaping, fs: f64, baud: f64) -> LinearParams {
        LinearParams {
            constellation: c,
            baud,
            sample_rate: fs,
            carrier: 0.0,
            amplitude: 1.0,
            shaping,
            differential: false,
            complex: true,
        }
    }

    fn complex_samples(s: &Signal) -> Vec<Complex64> {
        let Samples::Complex(x) = s.samples() else {
            panic!("expected complex")
        };
        x.iter()
            .map(|v| Complex64::new(f64::from(v.re), f64::from(v.im)))
            .collect()
    }

    /// Matched-filter output at the exact centre of symbol k: the correlation of the
    /// samples with the pulse placed at that (fractional) time.
    fn matched(x: &[Complex64], p: &LinearParams, t0: f64, k: usize) -> Complex64 {
        let sps = p.sample_rate / p.baud;
        let centre = (t0 * p.baud + k as f64) * sps; // in samples
        let reach = (p.shaping.span() as f64 / 2.0 + 1.0) * sps;
        let lo = (centre - reach).floor().max(0.0) as usize;
        let hi = ((centre + reach).ceil() as usize).min(x.len());
        let mut acc = Complex64::new(0.0, 0.0);
        let mut norm = 0.0;
        for (i, v) in x.iter().enumerate().take(hi).skip(lo) {
            let w = p.shaping.pulse((i as f64 - centre) / sps);
            acc += v * w;
            norm += w * w;
        }
        acc / norm
    }

    fn decide_all(p: &LinearParams, out: &LinearOutput, use_matched: bool) -> usize {
        let x = complex_samples(&out.signal);
        let sps = p.sample_rate / p.baud;
        let raw: Vec<Complex64> = (0..out.symbols.len())
            .map(|k| {
                if use_matched {
                    matched(&x, p, out.first_symbol_time, k)
                } else {
                    let idx = (out.first_symbol_time * p.baud + k as f64) * sps;
                    x[idx.round() as usize]
                }
            })
            .collect();
        // Scale to unit power.
        let pw = raw.iter().map(Complex64::norm_sqr).sum::<f64>() / raw.len() as f64;
        let points = p.constellation.points();
        raw.iter()
            .zip(&out.symbols)
            .filter(|(r, s)| nearest_in(&points, **r / pw.sqrt()) != **s)
            .count()
    }

    #[test]
    fn constellations_have_unit_power_and_distinct_points() {
        for c in Constellation::ALL {
            let pts = c.points();
            assert_eq!(pts.len(), c.order(), "{c:?}");
            let pw = pts.iter().map(Complex64::norm_sqr).sum::<f64>() / pts.len() as f64;
            assert!((pw - 1.0).abs() < 1e-12, "{c:?} power {pw}");
            for (i, a) in pts.iter().enumerate() {
                assert_eq!(c.nearest(*a), i, "{c:?} point {i}");
            }
        }
    }

    #[test]
    fn psk_and_square_qam_are_gray_coded() {
        for c in [
            Constellation::Qpsk,
            Constellation::Psk8,
            Constellation::Qam16,
            Constellation::Qam64,
        ] {
            let pts = c.points();
            let dmin = pts
                .iter()
                .enumerate()
                .flat_map(|(i, a)| pts.iter().skip(i + 1).map(move |b| (a - b).norm()))
                .fold(f64::INFINITY, f64::min);
            for (i, a) in pts.iter().enumerate() {
                for (j, b) in pts.iter().enumerate() {
                    if i != j && (a - b).norm() < dmin * 1.001 {
                        assert_eq!((i ^ j).count_ones(), 1, "{c:?}: {i} and {j}");
                    }
                }
            }
        }
    }

    #[test]
    fn rc_symbols_recovered_at_integer_sps() {
        for c in Constellation::ALL {
            let p = params(c, Shaping::RaisedCosine { alpha: 0.35 }, 9600.0, 1200.0);
            let syms = random_symbols(400, c.order(), 3);
            let out = linear(&p, &syms).unwrap();
            assert_eq!(out.signal.len(), (400 + PULSE_SPAN) * 8);
            if c == Constellation::Pi4Dqpsk {
                continue; // values are absolute phases, checked below
            }
            assert_eq!(decide_all(&p, &out, false), 0, "{c:?}");
        }
    }

    #[test]
    fn rrc_and_rect_symbols_recovered_at_fractional_sps() {
        // 8000 / 1500 = 5.333 samples per symbol.
        for shaping in [
            Shaping::RootRaisedCosine { alpha: 0.35 },
            Shaping::RootRaisedCosine { alpha: 0.2 },
            Shaping::Rectangular,
        ] {
            for c in [
                Constellation::Qpsk,
                Constellation::Psk8,
                Constellation::Qam16,
                Constellation::Qam32,
                Constellation::Qam64,
            ] {
                let p = params(c, shaping, 8000.0, 1500.0);
                let syms = random_symbols(300, c.order(), 9);
                let out = linear(&p, &syms).unwrap();
                assert_eq!(decide_all(&p, &out, true), 0, "{c:?} {shaping:?}");
            }
        }
    }

    #[test]
    fn real_output_on_a_carrier_demodulates() {
        let mut p = params(
            Constellation::Qam16,
            Shaping::RootRaisedCosine { alpha: 0.35 },
            8000.0,
            1200.0,
        );
        p.complex = false;
        p.carrier = 1800.0;
        let syms = random_symbols(200, 16, 4);
        let out = linear(&p, &syms).unwrap();
        let Samples::Real(x) = out.signal.samples() else {
            panic!("expected real")
        };
        // Mix down; the matched filter rejects the 2 fc image.
        let w = TAU * p.carrier / p.sample_rate;
        let bb: Vec<Complex64> = x
            .iter()
            .enumerate()
            .map(|(i, v)| f64::from(*v) * Complex64::from_polar(SQRT_2, -w * i as f64))
            .collect();
        let raw: Vec<Complex64> = (0..syms.len())
            .map(|k| matched(&bb, &p, out.first_symbol_time, k))
            .collect();
        let pw = raw.iter().map(Complex64::norm_sqr).sum::<f64>() / raw.len() as f64;
        let errors = raw
            .iter()
            .zip(&syms)
            .filter(|(r, s)| Constellation::Qam16.nearest(**r / pw.sqrt()) != **s)
            .count();
        assert_eq!(errors, 0);
        // RMS close to the amplitude.
        let rms = (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        assert!((rms - 1.0).abs() < 0.08, "rms {rms}");
    }

    #[test]
    fn differential_psk_steps_carry_the_symbols() {
        for c in [
            Constellation::Bpsk,
            Constellation::Qpsk,
            Constellation::Psk8,
            Constellation::Pi4Dqpsk,
        ] {
            let mut p = params(c, Shaping::RaisedCosine { alpha: 0.5 }, 9600.0, 1200.0);
            p.differential = true;
            let syms = random_symbols(200, c.order(), 12);
            let out = linear(&p, &syms).unwrap();
            let x = complex_samples(&out.signal);
            let at = |k: usize| x[(out.first_symbol_time * p.baud + k as f64) as usize * 8];
            for (k, sym) in syms.iter().enumerate().skip(1) {
                let step = (at(k) * at(k - 1).conj()).arg().rem_euclid(TAU);
                let want = c.phase_step(*sym).unwrap().rem_euclid(TAU);
                let err = (step - want + PI).rem_euclid(TAU) - PI;
                assert!(err.abs() < 1e-3, "{c:?} symbol {k}: {err}");
            }
        }
    }

    #[test]
    fn phase_step_inverts_the_gray_code() {
        for c in [
            Constellation::Bpsk,
            Constellation::Qpsk,
            Constellation::Psk8,
            Constellation::Psk16,
        ] {
            let m = c.order();
            for p in 0..m {
                let step = c.phase_step(gray(p)).unwrap();
                assert!(
                    (step - TAU * p as f64 / m as f64).abs() < 1e-12,
                    "{c:?} {p}"
                );
            }
            assert_eq!(c.phase_step(m), None);
        }
        assert_eq!(Constellation::Pi4Dqpsk.phase_step(4), None);
        assert!(
            (Constellation::Pi4Dqpsk.phase_step(gray(3)).unwrap() - 7.0 * FRAC_PI_4).abs() < 1e-12
        );
    }

    #[test]
    fn span_grows_as_the_roll_off_shrinks() {
        assert_eq!(Shaping::Rectangular.span(), 0);
        for (alpha, span) in [
            (1.0, 12),
            (0.35, 12),
            (0.2, 12),
            (0.1, 24),
            (0.05, 48),
            (0.01, 240),
        ] {
            assert_eq!(
                Shaping::RootRaisedCosine { alpha }.span(),
                span,
                "alpha {alpha}"
            );
            assert_eq!(
                Shaping::RaisedCosine { alpha }.span(),
                span,
                "alpha {alpha}"
            );
        }
        let p = params(
            Constellation::Qpsk,
            Shaping::RootRaisedCosine { alpha: 0.05 },
            8000.0,
            1000.0,
        );
        let out = linear(&p, &[0, 1, 2, 3]).unwrap();
        assert_eq!(out.pulse_span, 48);
        assert_eq!(out.signal.len(), (4 + 48) * 8);
        assert!((out.first_symbol_time - 24.0 / 1000.0).abs() < 1e-15);
    }

    #[test]
    fn truncation_isi_stays_low_at_small_roll_offs() {
        for alpha in [0.01, 0.05, 0.1, 0.2, 0.35] {
            let p = params(
                Constellation::Qpsk,
                Shaping::RootRaisedCosine { alpha },
                8000.0,
                1000.0,
            );
            let syms = random_symbols(300, 4, 21);
            let out = linear(&p, &syms).unwrap();
            let x = complex_samples(&out.signal);
            let pts = Constellation::Qpsk.points();
            let raw: Vec<Complex64> = (0..syms.len())
                .map(|k| matched(&x, &p, out.first_symbol_time, k))
                .collect();
            let sent: Vec<Complex64> = syms.iter().map(|s| pts[*s]).collect();
            // Least-squares gain, then the residual is the ISI.
            let g = raw
                .iter()
                .zip(&sent)
                .map(|(r, a)| r * a.conj())
                .sum::<Complex64>()
                / sent.iter().map(Complex64::norm_sqr).sum::<f64>();
            let isi: f64 = raw
                .iter()
                .zip(&sent)
                .map(|(r, a)| (r - g * a).norm_sqr())
                .sum();
            let sig: f64 = sent.iter().map(|a| (g * a).norm_sqr()).sum();
            let sir = 10.0 * (sig / isi).log10();
            assert!(sir > 40.0, "alpha {alpha}: signal to ISI {sir:.1} dB");
        }
    }

    #[test]
    fn rectangular_main_lobe_must_fit_above_0_hz() {
        let mut p = params(Constellation::Qpsk, Shaping::Rectangular, 8000.0, 1000.0);
        p.complex = false;
        p.carrier = 1100.0;
        assert!(linear(&p, &[0, 1]).is_ok());
        p.carrier = 600.0;
        assert!(linear(&p, &[0, 1]).is_err());
        p.carrier = 2900.0;
        assert!(linear(&p, &[0, 1]).is_ok());
        p.carrier = 3100.0;
        assert!(linear(&p, &[0, 1]).is_err());
    }

    #[test]
    fn differential_qam_is_rejected() {
        let mut p = params(Constellation::Qam16, Shaping::Rectangular, 8000.0, 1000.0);
        p.differential = true;
        let e = linear(&p, &[0, 1]).unwrap_err().to_string();
        assert!(e.contains("PSK only"), "{e}");
    }

    fn out_of_band_fraction(x: &[Complex64], fs: f64, edge: f64) -> f64 {
        let n = x.len();
        let mut buf: Vec<Complex64> = x.to_vec();
        FftPlanner::new().plan_fft_forward(n).process(&mut buf);
        let (mut inside, mut outside) = (0.0, 0.0);
        for (k, v) in buf.iter().enumerate() {
            let f = if k <= n / 2 {
                k as f64
            } else {
                k as f64 - n as f64
            } * fs
                / n as f64;
            if f.abs() <= edge {
                inside += v.norm_sqr();
            } else {
                outside += v.norm_sqr();
            }
        }
        outside / (inside + outside)
    }

    #[test]
    fn shaping_confines_the_spectrum() {
        let syms = random_symbols(2000, 4, 5);
        let edge = 1000.0 * 1.35 / 2.0 * 1.05;
        let mut fractions = Vec::new();
        for shaping in [
            Shaping::Rectangular,
            Shaping::RootRaisedCosine { alpha: 0.35 },
            Shaping::RaisedCosine { alpha: 0.35 },
        ] {
            let p = params(Constellation::Qpsk, shaping, 8000.0, 1000.0);
            let out = linear(&p, &syms).unwrap();
            fractions.push(out_of_band_fraction(
                &complex_samples(&out.signal),
                8000.0,
                edge,
            ));
        }
        assert!(fractions[0] > 0.03, "rectangular {}", fractions[0]);
        assert!(fractions[1] < 0.003, "RRC {}", fractions[1]);
        assert!(fractions[2] < 0.003, "RC {}", fractions[2]);
    }

    #[test]
    fn validation() {
        let p = params(Constellation::Qpsk, Shaping::Rectangular, 8000.0, 1000.0);
        assert!(linear(&p, &[]).is_err());
        assert!(linear(&p, &[4]).is_err());
        let bad_alpha = LinearParams {
            shaping: Shaping::RootRaisedCosine { alpha: 0.0 },
            ..p.clone()
        };
        assert!(linear(&bad_alpha, &[0]).is_err());
        let off_band = LinearParams {
            carrier: 3800.0,
            ..p.clone()
        };
        assert!(linear(&off_band, &[0]).is_err());
        let real_dc = LinearParams {
            complex: false,
            ..p.clone()
        };
        assert!(linear(&real_dc, &[0]).is_err());
        let slow = LinearParams { baud: 1e-9, ..p };
        assert!(linear(&slow, &[0]).is_err());
    }

    #[test]
    fn rc_pulse_is_nyquist() {
        for alpha in [0.1, 0.25, 0.5, 1.0] {
            for k in 1..6 {
                assert!(raised_cosine(f64::from(k), alpha).abs() < 1e-12);
            }
            assert!((raised_cosine(0.0, alpha) - 1.0).abs() < 1e-12);
            // Continuous at the removable singularity.
            let t = 1.0 / (2.0 * alpha);
            let near = raised_cosine(t + 1e-6, alpha);
            assert!((near - raised_cosine(t, alpha)).abs() < 1e-4);
        }
        let s = Shaping::RootRaisedCosine { alpha: 0.25 };
        assert!((pulse_energy(s) - 1.0).abs() < 0.01);
        let t = 1.0;
        assert!((s.pulse(t + 1e-7) - s.pulse(t)).abs() < 1e-4);
    }
}
