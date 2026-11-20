//! SA's editing operations (report 7.1): each turns a signal into a new signal.
//!
//! Every operation is described by an [`Edit`] value that serialises to JSON, so the
//! session graph records exactly how each signal was produced and can replay it.
//!
//! For real signals, band operations work on the *analytic band*: a complex bandpass
//! that keeps only the positive-frequency copy of the band, with gain 2, so that its
//! real part is exactly the real band-limited signal. Shifts and inversions then act
//! on that complex band and the real part is taken at the end.

use sn_core::{Complex32, SampleKind, SampleUnit, Signal};

use crate::detect;
pub use crate::equalize::EqMode;
use crate::equalize::equalize;

use crate::fir::{Fir, min_transition};
use crate::mix::mix_in_place;
use crate::resample::Resampler;
use crate::{Error, Result};

/// Largest result held in memory, in samples (2^28 complex samples = 2 GB).
pub const MAX_OUTPUT_SAMPLES: usize = 1 << 28;
/// Stopband attenuation of the band filters, dB.
pub const FILTER_ATTEN_DB: f64 = 80.0;

/// One editing operation with its parameters. Frequencies are in Hz relative to the
/// signal's 0 Hz; sample positions are indices into the input.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    /// Keep only `lo..hi` (SA: selection between H markers).
    BandSelect {
        /// Lower band edge.
        lo: f64,
        /// Upper band edge.
        hi: f64,
    },
    /// Remove `lo..hi` (SA: rejection between H markers).
    BandReject {
        /// Lower band edge.
        lo: f64,
        /// Upper band edge.
        hi: f64,
    },
    /// Remove the rectangle `start..end` x `lo..hi` (SA: area delete).
    AreaDelete {
        /// First sample.
        start: usize,
        /// End sample (exclusive).
        end: usize,
        /// Lower band edge.
        lo: f64,
        /// Upper band edge.
        hi: f64,
    },
    /// Shift the whole spectrum by `hz` (fractional values allowed).
    Shift {
        /// Shift in Hz; positive moves up.
        hz: f64,
    },
    /// Move the band `lo..hi` down to 0 Hz, optionally inverted (SA: shift to base).
    ShiftToBase {
        /// Lower band edge.
        lo: f64,
        /// Upper band edge.
        hi: f64,
        /// Flip the band so its upper edge lands at 0 Hz.
        inverted: bool,
    },
    /// Invert the whole spectrum (I/Q: conjugate; real: f becomes fs/2 - f).
    Invert,
    /// Invert only the band `lo..hi` about its centre (SA: in-band inverse).
    InvertBand {
        /// Lower band edge.
        lo: f64,
        /// Upper band edge.
        hi: f64,
    },
    /// Resample to `rate` Hz (fractional rates allowed).
    Resample {
        /// New sample rate.
        rate: f64,
    },
    /// Turn I/Q into a real signal at twice the rate (band moves to 0..fs).
    IqToReal,
    /// First difference `x[n] - x[n-1]` (removes DC and slow trends).
    Derivative,
    /// Hard limiter: unit magnitude (I/Q) or sign (real).
    Limiter,
    /// Copy samples `start..end` into a new signal.
    Copy {
        /// First sample.
        start: usize,
        /// End sample (exclusive).
        end: usize,
    },
    /// Correct a recording whose clock is off: a measured `existing` rate becomes
    /// `required` (SA: Correction of BR, at most +/-30%).
    BaudCorrection {
        /// Rate measured in the recording.
        existing: f64,
        /// Rate it should have.
        required: f64,
    },
    /// Lowpass at `cutoff` Hz (SA's waveform-viewer filter).
    Lowpass {
        /// Cutoff (-6 dB) in Hz.
        cutoff: f64,
    },
    /// Median filter over an odd window of samples (at most 301).
    Median {
        /// Window length in samples.
        window: usize,
    },
    /// Quadrature AM detector: envelope, mean removed (clock line for PSK/QAM/FSK).
    AmDetector,
    /// Modified AM detector: frequency-change pulses (clock line for MFSK).
    ModifiedAm,
    /// Double half-wave rectifier (clock from discriminator recordings).
    HalfWave,
    /// Phase detector: instantaneous frequency in Hz.
    PhaseDetector,
    /// Raise to the power `n` (PSK order and carrier).
    Power {
        /// The power.
        n: u32,
    },
    /// Autocorrelation of samples `start..end` (output index = lag).
    Acf {
        /// First sample.
        start: usize,
        /// End sample (exclusive).
        end: usize,
    },
    /// Cross-correlation of the fragment `start..end` against the whole signal.
    Ccf {
        /// First sample of the fragment.
        start: usize,
        /// End sample (exclusive).
        end: usize,
    },
    /// Regenerate a clean FSK/MFSK signal on `tones` at `baud`.
    Resynthesize {
        /// Tone frequencies in Hz.
        tones: Vec<f64>,
        /// Keying rate.
        baud: f64,
    },
    /// ISD corrector: blind adaptive equalizer (AM = CMA, X^N = decision-directed).
    Equalize {
        /// Carrier in Hz.
        carrier: f64,
        /// Keying rate in Bd.
        baud: f64,
        /// Equalizer mode.
        mode: EqMode,
        /// Training amount (SA: blocks, 10-50).
        blocks: usize,
    },
    /// Root-raised-cosine filter matched to `baud` with roll-off `alpha`.
    Rrc {
        /// Symbol rate.
        baud: f64,
        /// Roll-off, 0 < alpha <= 1.
        alpha: f64,
    },
}

impl Edit {
    /// Short name for the session tree, e.g. `"band select 300-3000 Hz"`.
    pub fn label(&self) -> String {
        let band = |lo: f64, hi: f64| format!("{:.2}-{:.2} Hz", lo.min(hi), lo.max(hi));
        match self {
            Self::BandSelect { lo, hi } => format!("band select {}", band(*lo, *hi)),
            Self::BandReject { lo, hi } => format!("band reject {}", band(*lo, *hi)),
            Self::AreaDelete { lo, hi, .. } => format!("area delete {}", band(*lo, *hi)),
            Self::Shift { hz } => format!("shift {hz:+.3} Hz"),
            Self::ShiftToBase { lo, hi, inverted } => format!(
                "{} to base {}",
                if *inverted { "inverted" } else { "shift" },
                band(*lo, *hi)
            ),
            Self::Invert => "spectrum inverse".to_owned(),
            Self::InvertBand { lo, hi } => format!("in-band inverse {}", band(*lo, *hi)),
            Self::Resample { rate } => format!("resample to {rate} Hz"),
            Self::IqToReal => "I/Q to real".to_owned(),
            Self::Derivative => "first derivative".to_owned(),
            Self::Limiter => "hard limiter".to_owned(),
            Self::Copy { start, end } => format!("copy {start}..{end}"),
            Self::BaudCorrection { existing, required } => {
                format!("baud correction {existing} to {required}")
            }
            Self::Lowpass { cutoff } => format!("lowpass {cutoff} Hz"),
            Self::Median { window } => format!("median {window}"),
            Self::Rrc { baud, alpha } => format!("RRC {baud} Bd a={alpha}"),
            Self::Equalize { mode, blocks, .. } => match mode {
                EqMode::Am => format!("ISD corrector AM ({blocks} blocks)"),
                EqMode::Xn { n } => format!("ISD corrector X^{n} ({blocks} blocks)"),
            },
            Self::AmDetector => "AM detector".to_owned(),
            Self::ModifiedAm => "modified AM detector".to_owned(),
            Self::HalfWave => "double half-wave detector".to_owned(),
            Self::PhaseDetector => "phase detector (Hz)".to_owned(),
            Self::Power { n } => format!("power {n}"),
            Self::Acf { start, end } => format!("ACF {start}..{end}"),
            Self::Ccf { start, end } => format!("CCF of {start}..{end}"),
            Self::Resynthesize { tones, baud } => {
                format!("resynthesis {}-tone {baud} Bd", tones.len())
            }
        }
    }

    /// Run the operation.
    pub fn run(&self, input: &Signal, should_stop: &(dyn Fn() -> bool + Sync)) -> Result<Signal> {
        let out = match self {
            Self::BandSelect { .. }
            | Self::BandReject { .. }
            | Self::AreaDelete { .. }
            | Self::Shift { .. }
            | Self::ShiftToBase { .. }
            | Self::Invert
            | Self::InvertBand { .. } => self.run_spectral(input, should_stop)?,
            Self::Resample { .. }
            | Self::IqToReal
            | Self::Copy { .. }
            | Self::BaudCorrection { .. } => self.run_timebase(input, should_stop)?,
            Self::Derivative
            | Self::Limiter
            | Self::Lowpass { .. }
            | Self::Median { .. }
            | Self::Rrc { .. } => self.run_filter(input, should_stop)?,
            Self::AmDetector
            | Self::ModifiedAm
            | Self::HalfWave
            | Self::PhaseDetector
            | Self::Power { .. }
            | Self::Acf { .. }
            | Self::Ccf { .. }
            | Self::Resynthesize { .. }
            | Self::Equalize { .. } => self.run_detect(input, should_stop)?,
        };
        let unit = match self {
            Self::Copy { .. }
            | Self::Lowpass { .. }
            | Self::Median { .. }
            | Self::Rrc { .. }
            | Self::Resample { .. } => input.unit(),
            Self::PhaseDetector => SampleUnit::Hertz,
            _ => SampleUnit::FullScale,
        };
        Ok(out
            .with_center_frequency(self.output_center_frequency(input))
            .with_unit(unit))
    }

    /// The RF that 0 Hz of the result corresponds to (0 when unknown).
    ///
    /// Operations that move the spectrum move the centre with it. Operations that
    /// mirror the frequency axis or whose output is not a spectrum of the input
    /// (detectors, powers, correlations) have no RF axis, so the result reads 0.
    /// In-band inverse keeps the input's centre: the axis is unchanged and only the
    /// content of the marked band is rearranged.
    fn output_center_frequency(&self, input: &Signal) -> f64 {
        let rf = input.center_frequency();
        if !rf.is_finite() || rf.abs() < f64::MIN_POSITIVE {
            return 0.0;
        }
        match self {
            Self::Shift { hz } => rf - hz,
            Self::ShiftToBase {
                lo,
                hi,
                inverted: false,
            } => band(input, *lo, *hi).map_or(0.0, |b| rf + b.lo),
            Self::IqToReal => rf - input.sample_rate() / 2.0,
            Self::ShiftToBase { inverted: true, .. }
            | Self::Invert
            | Self::Power { .. }
            | Self::AmDetector
            | Self::ModifiedAm
            | Self::HalfWave
            | Self::PhaseDetector
            | Self::Acf { .. }
            | Self::Ccf { .. } => 0.0,
            Self::BandSelect { .. }
            | Self::BandReject { .. }
            | Self::AreaDelete { .. }
            | Self::InvertBand { .. }
            | Self::Resample { .. }
            | Self::Copy { .. }
            | Self::BaudCorrection { .. }
            | Self::Derivative
            | Self::Limiter
            | Self::Lowpass { .. }
            | Self::Median { .. }
            | Self::Rrc { .. }
            | Self::Resynthesize { .. }
            | Self::Equalize { .. } => rf,
        }
    }

    /// Band, shift and inversion operations.
    fn run_spectral(
        &self,
        input: &Signal,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Signal> {
        let fs = input.sample_rate();
        Ok(match *self {
            Self::BandSelect { lo, hi } => {
                let b = band(input, lo, hi)?;
                finish(input, analytic_band(input, &b, should_stop)?)?
            }
            Self::BandReject { lo, hi } => {
                let b = band(input, lo, hi)?;
                subtract(input, &analytic_band(input, &b, should_stop)?, None)?
            }
            Self::AreaDelete { start, end, lo, hi } => {
                let b = band(input, lo, hi)?;
                let x = analytic_band(input, &b, should_stop)?;
                // The filter's impulse response spreads over about fs / t samples.
                let ramp = (fs / b.t) as usize;
                subtract(input, &x, Some((start, end, ramp.max(1))))?
            }
            Self::Shift { hz } => {
                if !hz.is_finite() {
                    return Err(invalid("hz", "the shift must be a finite number of Hz"));
                }
                let mut x = analytic_full(input, should_stop)?;
                mix_in_place(&mut x, 0, hz, fs);
                finish(input, x)?
            }
            Self::ShiftToBase { lo, hi, inverted } => {
                let b = band(input, lo, hi)?;
                // A real result folds negative frequencies onto positive ones, so for
                // real input the filter's stopband edge (not its -6 dB point) sits at
                // the edge that lands on 0 Hz: nothing from outside the band folds in.
                let (f_lo, f_hi) = match (input.kind(), inverted) {
                    (SampleKind::Real, false) => (b.lo + b.t / 2.0, b.hi),
                    (SampleKind::Real, true) => (b.lo, b.hi - b.t / 2.0),
                    (SampleKind::Complex, _) => (b.lo, b.hi),
                };
                let fir = band_fir(input, f_lo, f_hi, b.t)?;
                let mut x = filter_band(input, &fir, should_stop)?;
                if inverted {
                    conj(&mut x);
                    mix_in_place(&mut x, 0, b.hi, fs);
                } else {
                    mix_in_place(&mut x, 0, -b.lo, fs);
                }
                finish(input, x)?
            }
            Self::Invert => match input.kind() {
                SampleKind::Complex => {
                    let mut x = read_all_complex(input)?;
                    conj(&mut x);
                    Signal::complex(x, fs)?
                }
                SampleKind::Real => {
                    let x = read_all_real(input)?;
                    let y: Vec<f32> = x
                        .iter()
                        .enumerate()
                        .map(|(i, v)| if i % 2 == 0 { *v } else { -*v })
                        .collect();
                    Signal::real(y, fs)?
                }
            },
            Self::InvertBand { lo, hi } => {
                let b = band(input, lo, hi)?;
                let mut x = analytic_band(input, &b, should_stop)?;
                let mut y = subtract_values(input, &x)?;
                // Flip the band in place about its centre and put it back.
                let centre = f64::midpoint(b.lo, b.hi);
                mix_in_place(&mut x, 0, -centre, fs);
                conj(&mut x);
                mix_in_place(&mut x, 0, centre, fs);
                add_band(&mut y, &x, input.kind());
                finish(input, y)?
            }
            Self::Resample { .. }
            | Self::IqToReal
            | Self::Copy { .. }
            | Self::BaudCorrection { .. }
            | Self::Derivative
            | Self::Limiter
            | Self::Lowpass { .. }
            | Self::Median { .. }
            | Self::Rrc { .. }
            | Self::AmDetector
            | Self::ModifiedAm
            | Self::HalfWave
            | Self::PhaseDetector
            | Self::Power { .. }
            | Self::Acf { .. }
            | Self::Ccf { .. }
            | Self::Resynthesize { .. }
            | Self::Equalize { .. } => return Err(wrong_family(self)),
        })
    }

    /// Operations that change the sample rate or the length.
    fn run_timebase(
        &self,
        input: &Signal,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Signal> {
        let fs = input.sample_rate();
        let len = input.len();
        Ok(match *self {
            Self::Resample { rate } => {
                let r = Resampler::new(fs, rate)?;
                guard(r.output_len(len))?;
                let y = r.apply(input, 0, len, should_stop)?;
                with_kind(input.kind(), y, rate)?
            }
            Self::IqToReal => {
                if input.kind() == SampleKind::Real {
                    return Err(invalid("signal", "the signal is already real"));
                }
                let up = Resampler::new(fs, 2.0 * fs)?;
                guard(up.output_len(len))?;
                let mut y = up.apply(input, 0, len, should_stop)?;
                // At the doubled rate fs/2 is a quarter of the sample rate.
                mix_in_place(&mut y, 0, fs / 2.0, 2.0 * fs);
                Signal::real(y.iter().map(|c| c.re).collect::<Vec<_>>(), 2.0 * fs)?
            }
            Self::Copy { start, end } => {
                guard(end.saturating_sub(start))?;
                input.slice(start.min(len), end.min(len))?
            }
            Self::BaudCorrection { existing, required } => {
                if !(crate::is_positive(existing) && crate::is_positive(required)) {
                    return Err(invalid("rate", "both rates must be above 0"));
                }
                let ratio = required / existing;
                if !(0.7..=1.3).contains(&ratio) {
                    return Err(invalid(
                        "rate",
                        format!(
                            "a {:.1}% change is outside SA's +/-30% limit",
                            (ratio - 1.0) * 100.0
                        ),
                    ));
                }
                // Reinterpret the recording at fs * ratio, then resample back to fs.
                let relabelled = relabel(input, fs * ratio)?;
                let r = Resampler::new(fs * ratio, fs)?;
                guard(r.output_len(len))?;
                let y = r.apply(&relabelled, 0, len, should_stop)?;
                with_kind(input.kind(), y, fs)?
            }
            Self::BandSelect { .. }
            | Self::BandReject { .. }
            | Self::AreaDelete { .. }
            | Self::Shift { .. }
            | Self::ShiftToBase { .. }
            | Self::Invert
            | Self::InvertBand { .. }
            | Self::Derivative
            | Self::Limiter
            | Self::Lowpass { .. }
            | Self::Median { .. }
            | Self::Rrc { .. }
            | Self::AmDetector
            | Self::ModifiedAm
            | Self::HalfWave
            | Self::PhaseDetector
            | Self::Power { .. }
            | Self::Acf { .. }
            | Self::Ccf { .. }
            | Self::Resynthesize { .. }
            | Self::Equalize { .. } => return Err(wrong_family(self)),
        })
    }

    /// Sample-wise operations and filters.
    fn run_filter(
        &self,
        input: &Signal,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Signal> {
        let fs = input.sample_rate();
        let len = input.len();
        Ok(match *self {
            Self::Derivative => {
                let x = read_all_complex(input)?;
                let mut prev = Complex32::new(0.0, 0.0);
                let y: Vec<Complex32> = x
                    .iter()
                    .map(|v| {
                        let d = v - prev;
                        prev = *v;
                        d
                    })
                    .collect();
                with_kind(input.kind(), y, fs)?
            }
            Self::Limiter => {
                let x = read_all_complex(input)?;
                let y: Vec<Complex32> = match input.kind() {
                    SampleKind::Complex => x
                        .iter()
                        .map(|v| {
                            let m = v.norm();
                            if m > 0.0 { v / m } else { *v }
                        })
                        .collect(),
                    SampleKind::Real => x
                        .iter()
                        .map(|v| {
                            Complex32::new(
                                if v.re > 0.0 {
                                    1.0
                                } else if v.re < 0.0 {
                                    -1.0
                                } else {
                                    0.0
                                },
                                0.0,
                            )
                        })
                        .collect(),
                };
                with_kind(input.kind(), y, fs)?
            }
            Self::Lowpass { cutoff } => {
                guard(len)?;
                // The transition follows the cutoff down to what the tap limit allows,
                // and must fit between 0 Hz and Nyquist around the cutoff.
                let t_min = min_transition(fs, FILTER_ATTEN_DB);
                let t = (cutoff * 0.2)
                    .max(t_min)
                    .min(fs / 8.0)
                    .min(fs - 2.0 * cutoff);
                if !(cutoff.is_finite() && t >= t_min && t < 2.0 * cutoff) {
                    return Err(invalid(
                        "cutoff",
                        format!(
                            "{cutoff} Hz is outside the usable {:.3}-{:.3} Hz range at this \
                             sample rate; resample first for a lower cutoff",
                            t_min / 2.0,
                            (fs - t_min) / 2.0
                        ),
                    ));
                }
                let f = Fir::lowpass(cutoff, t, FILTER_ATTEN_DB, fs)?;
                let y = f.apply(input, 0, len, should_stop)?;
                with_kind(input.kind(), y, fs)?
            }
            Self::Median { window } => {
                if window == 0 || window % 2 == 0 || window > 301 {
                    return Err(invalid("window", "must be odd and between 1 and 301"));
                }
                let x = read_all_complex(input)?;
                with_kind(input.kind(), median(&x, window, should_stop)?, fs)?
            }
            Self::Rrc { baud, alpha } => {
                guard(len)?;
                let f = Fir::rrc(baud, alpha, 8, fs)?;
                let y = f.apply(input, 0, len, should_stop)?;
                with_kind(input.kind(), y, fs)?
            }
            Self::BandSelect { .. }
            | Self::BandReject { .. }
            | Self::AreaDelete { .. }
            | Self::Shift { .. }
            | Self::ShiftToBase { .. }
            | Self::Invert
            | Self::InvertBand { .. }
            | Self::Resample { .. }
            | Self::IqToReal
            | Self::Copy { .. }
            | Self::BaudCorrection { .. }
            | Self::AmDetector
            | Self::ModifiedAm
            | Self::HalfWave
            | Self::PhaseDetector
            | Self::Power { .. }
            | Self::Acf { .. }
            | Self::Ccf { .. }
            | Self::Resynthesize { .. }
            | Self::Equalize { .. } => return Err(wrong_family(self)),
        })
    }
}

impl Edit {
    /// Detectors, correlation and resynthesis.
    fn run_detect(
        &self,
        input: &Signal,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Signal> {
        let fs = input.sample_rate();
        let real = |v: Vec<f32>| Signal::real(v, fs).map_err(Error::from);
        match self {
            Self::AmDetector => real(detect::envelope(input, should_stop)?),
            Self::ModifiedAm => real(detect::modified_am(input, should_stop)?),
            Self::HalfWave => real(detect::half_wave(input)?),
            Self::PhaseDetector => {
                let z = detect::analytic(input, should_stop)?;
                real(detect::inst_freq(&z, fs))
            }
            Self::Power { n } => detect::power(input, *n, should_stop),
            Self::Acf { start, end } => real(detect::acf(input, *start, *end, should_stop)?),
            Self::Ccf { start, end } => real(detect::ccf(input, *start, *end, should_stop)?),
            Self::Resynthesize { tones, baud } => {
                real(detect::resynthesize(input, tones, *baud, should_stop)?)
            }
            Self::Equalize {
                carrier,
                baud,
                mode,
                blocks,
            } => equalize(input, *carrier, *baud, *mode, *blocks, should_stop),
            other => Err(wrong_family(other)),
        }
    }
}

/// Internal dispatch error; [`Edit::run`] never routes an edit to the wrong family.
fn wrong_family(e: &Edit) -> Error {
    invalid("op", format!("internal routing error for {}", e.label()))
}

fn invalid(name: &'static str, reason: impl Into<String>) -> Error {
    Error::InvalidParameter {
        name,
        reason: reason.into(),
    }
}

fn order(a: f64, b: f64) -> (f64, f64) {
    (a.min(b), a.max(b))
}

fn guard(samples: usize) -> Result<()> {
    if samples > MAX_OUTPUT_SAMPLES {
        return Err(invalid(
            "size",
            format!(
                "the result would hold {samples} samples (limit {MAX_OUTPUT_SAMPLES}). \
                 Copy a shorter span, or shift the band down and resample first"
            ),
        ));
    }
    Ok(())
}

fn read_all_complex(s: &Signal) -> Result<Vec<Complex32>> {
    guard(s.len())?;
    let mut v = vec![Complex32::new(0.0, 0.0); s.len()];
    s.read_complex(0, &mut v);
    Ok(v)
}

fn read_all_real(s: &Signal) -> Result<Vec<f32>> {
    guard(s.len())?;
    let mut v = vec![0.0_f32; s.len()];
    s.read_real(0, &mut v);
    Ok(v)
}

fn with_kind(kind: SampleKind, y: Vec<Complex32>, fs: f64) -> Result<Signal> {
    Ok(match kind {
        SampleKind::Complex => Signal::complex(y, fs)?,
        SampleKind::Real => Signal::real(y.iter().map(|c| c.re).collect::<Vec<_>>(), fs)?,
    })
}

/// A copy of `s` reinterpreted at another sample rate (in memory).
fn relabel(s: &Signal, fs: f64) -> Result<Signal> {
    let x = read_all_complex(s)?;
    with_kind(s.kind(), x, fs)
}

/// Lowest frequency a signal of this kind holds: 0 Hz for real, -fs/2 for I/Q.
fn spectrum_floor(fs: f64, kind: SampleKind) -> f64 {
    match kind {
        SampleKind::Real => 0.0,
        SampleKind::Complex => -fs / 2.0,
    }
}

/// The band an in-band edit actually works on: `lo..hi` ordered and clamped to the
/// frequencies a signal of rate `fs` and this kind holds (0..fs/2 for real,
/// -fs/2..fs/2 for I/Q). The result may have no width (`hi <= lo`) when the band lies
/// outside that range; the edits report that as an error.
#[must_use]
pub fn effective_band(lo: f64, hi: f64, fs: f64, kind: SampleKind) -> (f64, f64) {
    let (lo, hi) = order(lo, hi);
    (lo.max(spectrum_floor(fs, kind)), hi.min(fs / 2.0))
}

/// A band clamped to the signal's frequency range, with the transition width of the
/// filter that selects it.
#[derive(Debug, Clone, Copy)]
struct Band {
    lo: f64,
    hi: f64,
    t: f64,
}

/// Order `lo..hi`, clamp it to the frequencies the signal holds (0..fs/2 for real,
/// -fs/2..fs/2 for I/Q) and pick its filter transition. A band with no width left
/// inside that range is an error rather than a silently swapped band.
fn band(input: &Signal, lo: f64, hi: f64) -> Result<Band> {
    if !(lo.is_finite() && hi.is_finite()) {
        return Err(invalid(
            "band",
            "the band edges must be finite numbers of Hz",
        ));
    }
    let fs = input.sample_rate();
    let (lo0, hi0) = order(lo, hi);
    let floor = spectrum_floor(fs, input.kind());
    let (lo, hi) = effective_band(lo, hi, fs, input.kind());
    if hi <= lo {
        return Err(invalid(
            "band",
            format!(
                "{lo0}-{hi0} Hz lies outside the signal's {floor}-{} Hz range; move the H \
                 markers inside it",
                fs / 2.0
            ),
        ));
    }
    let t = transition_for(fs, lo, hi)?;
    Ok(Band { lo, hi, t })
}

/// Transition width for band filters: 5% of the band, at most 1/20 of the rate and
/// half the band (so a passband always exists), and at least what the tap limit
/// allows. Narrow bands therefore keep their edges at the H markers.
fn transition_for(fs: f64, lo: f64, hi: f64) -> Result<f64> {
    let width = hi - lo;
    if !crate::is_positive(width) {
        return Err(invalid("band", "the band must have a non-zero width"));
    }
    let t_min = min_transition(fs, FILTER_ATTEN_DB);
    let t = (width * 0.05)
        .max(t_min)
        .min(fs / 20.0)
        .min(width / 2.0)
        // The filter's stopband must not run past the other side of the spectrum.
        .min(fs - width);
    if t < t_min {
        let reason = if width / 2.0 < t_min {
            format!(
                "the band is {width} Hz wide; at {fs} Hz the narrowest band that can be \
                 filtered is {:.3} Hz. Shift the band down and resample first",
                2.0 * t_min
            )
        } else {
            "the band covers almost the whole spectrum; narrow it with the H markers".to_owned()
        };
        return Err(invalid("band", reason));
    }
    Ok(t)
}

/// Filter passing `lo..hi` with transition `t`: complex, with gain 2 for real input
/// (the analytic band, positive frequencies only) and 1 for I/Q.
fn band_fir(input: &Signal, lo: f64, hi: f64, t: f64) -> Result<Fir> {
    let fs = input.sample_rate();
    let gain = match input.kind() {
        SampleKind::Real => 2.0,
        SampleKind::Complex => 1.0,
    };
    Ok(Fir::complex_bandpass(lo, hi, t, FILTER_ATTEN_DB, fs)?.modulated(0.0, fs, gain))
}

fn filter_band(
    input: &Signal,
    fir: &Fir,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<Complex32>> {
    guard(input.len())?;
    fir.apply(input, 0, input.len(), should_stop)
}

/// The band as a complex signal. For real input: the analytic band (gain 2, positive
/// frequencies), whose real part is the real band. For I/Q: the band itself.
fn analytic_band(
    input: &Signal,
    b: &Band,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<Complex32>> {
    filter_band(input, &band_fir(input, b.lo, b.hi, b.t)?, should_stop)
}

/// The whole signal as a complex signal (analytic for real input).
fn analytic_full(
    input: &Signal,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<Complex32>> {
    match input.kind() {
        SampleKind::Complex => read_all_complex(input),
        SampleKind::Real => {
            guard(input.len())?;
            let fs = input.sample_rate();
            let f = Fir::analytic((fs / 500.0).max(1.0), FILTER_ATTEN_DB, fs)?;
            f.apply(input, 0, input.len(), should_stop)
        }
    }
}

fn conj(x: &mut [Complex32]) {
    for v in x {
        *v = v.conj();
    }
}

/// Build the output signal from a complex result: real part for real input.
fn finish(input: &Signal, y: Vec<Complex32>) -> Result<Signal> {
    with_kind(input.kind(), y, input.sample_rate())
}

/// `input - band` (band given in analytic form for real input).
fn subtract_values(input: &Signal, band: &[Complex32]) -> Result<Vec<Complex32>> {
    let mut x = read_all_complex(input)?;
    match input.kind() {
        SampleKind::Complex => {
            for (v, b) in x.iter_mut().zip(band) {
                *v -= b;
            }
        }
        SampleKind::Real => {
            for (v, b) in x.iter_mut().zip(band) {
                v.re -= b.re;
                v.im = 0.0;
            }
        }
    }
    Ok(x)
}

fn add_band(y: &mut [Complex32], band: &[Complex32], kind: SampleKind) {
    for (v, b) in y.iter_mut().zip(band) {
        match kind {
            SampleKind::Complex => *v += b,
            SampleKind::Real => v.re += b.re,
        }
    }
}

/// `input - band * gate`, where the gate is 1 inside `start..end` with raised-cosine
/// ramps of `ramp` samples (or 1 everywhere when `None`).
fn subtract(
    input: &Signal,
    band: &[Complex32],
    gate: Option<(usize, usize, usize)>,
) -> Result<Signal> {
    let mut x = read_all_complex(input)?;
    let real = input.kind() == SampleKind::Real;
    for (i, (v, b)) in x.iter_mut().zip(band).enumerate() {
        let g = match gate {
            None => 1.0,
            Some((s, e, ramp)) => gate_value(i, s, e, ramp),
        };
        if real {
            v.re -= b.re * g;
            v.im = 0.0;
        } else {
            *v -= b * g;
        }
    }
    finish(input, x)
}

fn gate_value(i: usize, start: usize, end: usize, ramp: usize) -> f32 {
    let (s, e) = (start.min(end), start.max(end));
    if i >= s && i < e {
        return 1.0;
    }
    let d = if i < s { s - i } else { i + 1 - e };
    if d >= ramp {
        0.0
    } else {
        let t = d as f32 / ramp as f32;
        0.5 + 0.5 * (std::f32::consts::PI * t).cos()
    }
}

fn median(
    x: &[Complex32],
    window: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<Complex32>> {
    let half = window / 2;
    let mut re_buf = Vec::with_capacity(window);
    let mut im_buf = Vec::with_capacity(window);
    let mut out = Vec::with_capacity(x.len());
    for i in 0..x.len() {
        if i % 65_536 == 0 && should_stop() {
            return Err(Error::Cancelled);
        }
        let a = i.saturating_sub(half);
        let b = (i + half + 1).min(x.len());
        re_buf.clear();
        im_buf.clear();
        for v in x.get(a..b).unwrap_or(&[]) {
            re_buf.push(v.re);
            im_buf.push(v.im);
        }
        // Lower middle for the shorter, even-length windows at the edges.
        let m = re_buf.len().saturating_sub(1) / 2;
        let re = re_buf.select_nth_unstable_by(m, f32::total_cmp).1;
        let re = *re;
        let im = *im_buf.select_nth_unstable_by(m, f32::total_cmp).1;
        out.push(Complex32::new(re, im));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use sn_synth::tone;

    use super::*;
    use crate::spectrum::SpectrumEngine;
    use crate::window::WindowKind;

    /// Level in dBFS at `hz`, measured in the middle of the signal.
    fn level(s: &Signal, hz: f64) -> f32 {
        let e = SpectrumEngine::new(4096, WindowKind::default(), s.kind()).unwrap();
        let db = e.frame_db(s, s.len() / 2 - 2048);
        let bin = e.frequency_to_bin(hz, s.sample_rate()).round() as usize;
        db[bin.saturating_sub(2)..=bin + 2]
            .iter()
            .copied()
            .fold(f32::MIN, f32::max)
    }

    fn two_tones(fs: f64) -> Signal {
        let a = tone(1000.0, 0.25, fs, 2.0).unwrap();
        let b = tone(3000.0, 0.25, fs, 2.0).unwrap();
        let mut x = vec![0.0_f32; a.len()];
        let mut y = vec![0.0_f32; b.len()];
        a.read_real(0, &mut x);
        b.read_real(0, &mut y);
        Signal::real(x.iter().zip(&y).map(|(p, q)| p + q).collect::<Vec<_>>(), fs).unwrap()
    }

    #[test]
    fn band_select_and_reject_on_real() {
        let fs = 16_000.0;
        let s = two_tones(fs);
        let sel = Edit::BandSelect {
            lo: 800.0,
            hi: 1200.0,
        }
        .run(&s, &|| false)
        .unwrap();
        assert!((level(&sel, 1000.0) - level(&s, 1000.0)).abs() < 0.1);
        assert!(level(&sel, 3000.0) < -70.0);
        let rej = Edit::BandReject {
            lo: 800.0,
            hi: 1200.0,
        }
        .run(&s, &|| false)
        .unwrap();
        assert!(level(&rej, 1000.0) < -60.0);
        assert!((level(&rej, 3000.0) - level(&s, 3000.0)).abs() < 0.1);
    }

    #[test]
    fn shifts_and_base_moves_on_real() {
        let fs = 16_000.0;
        let s = tone(2000.0, 0.5, fs, 2.0).unwrap();
        let up = Edit::Shift { hz: 500.25 }.run(&s, &|| false).unwrap();
        assert!(level(&up, 2500.25) > level(&up, 2000.0) + 60.0);
        let base = Edit::ShiftToBase {
            lo: 1800.0,
            hi: 2600.0,
            inverted: false,
        }
        .run(&s, &|| false)
        .unwrap();
        assert!(level(&base, 200.0) > -7.0);
        let inv = Edit::ShiftToBase {
            lo: 1800.0,
            hi: 2600.0,
            inverted: true,
        }
        .run(&s, &|| false)
        .unwrap();
        assert!(
            level(&inv, 600.0) > -7.0,
            "2000 Hz in 1800..2600 inverted lands at 600 Hz"
        );
    }

    #[test]
    fn inversions() {
        let fs = 16_000.0;
        let s = tone(1000.0, 0.5, fs, 2.0).unwrap();
        let full = Edit::Invert.run(&s, &|| false).unwrap();
        assert!(level(&full, 7000.0) > -7.0);
        let band = Edit::InvertBand {
            lo: 800.0,
            hi: 1600.0,
        }
        .run(&s, &|| false)
        .unwrap();
        // 1000 Hz flips about the 1200 Hz centre to 1400 Hz.
        assert!(level(&band, 1400.0) > -7.0);
        assert!(level(&band, 1000.0) < -60.0);
        let iq = Signal::complex(vec![Complex32::new(0.5, 0.25); 8], fs).unwrap();
        let c = Edit::Invert.run(&iq, &|| false).unwrap();
        let mut v = [Complex32::new(0.0, 0.0)];
        c.read_complex(0, &mut v);
        assert_eq!(v[0], Complex32::new(0.5, -0.25));
    }

    #[test]
    fn iq_to_real_doubles_the_rate_and_maps_the_band() {
        let fs = 8000.0;
        // Complex tone at -1000 Hz; after conversion it sits at fs/2 - 1000 = 3000 Hz.
        let x: Vec<Complex32> = (0..16_000)
            .map(|n| {
                let ph = -std::f64::consts::TAU * 1000.0 * f64::from(n) / fs;
                Complex32::new(ph.cos() as f32 * 0.5, ph.sin() as f32 * 0.5)
            })
            .collect();
        let s = Signal::complex(x, fs).unwrap();
        let r = Edit::IqToReal.run(&s, &|| false).unwrap();
        assert_eq!(r.kind(), SampleKind::Real);
        assert!((r.sample_rate() - 16_000.0).abs() < 1e-9);
        assert!(level(&r, 3000.0) > -7.0);
        assert!(Edit::IqToReal.run(&r, &|| false).is_err());
    }

    #[test]
    fn simple_point_operations() {
        let s = Signal::real(vec![1.0_f32, 3.0, 2.0, -5.0, 0.0], 100.0).unwrap();
        let d = Edit::Derivative.run(&s, &|| false).unwrap();
        let mut v = [0.0_f32; 5];
        d.read_real(0, &mut v);
        assert_eq!(v, [1.0, 2.0, -1.0, -7.0, 5.0]);
        let l = Edit::Limiter.run(&s, &|| false).unwrap();
        l.read_real(0, &mut v);
        assert_eq!(v, [1.0, 1.0, 1.0, -1.0, 0.0]);
        let m = Edit::Median { window: 3 }.run(&s, &|| false).unwrap();
        m.read_real(0, &mut v);
        assert_eq!(v, [1.0, 2.0, 2.0, 0.0, -5.0]);
        assert!(Edit::Median { window: 4 }.run(&s, &|| false).is_err());
        let c = Edit::Copy { start: 1, end: 3 }.run(&s, &|| false).unwrap();
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn baud_correction_rescales_the_time_axis() {
        // A 2373 Hz tone that should be 2400 Hz (SA's sound-card clock example).
        let fs = 48_000.0;
        let s = tone(2373.0, 0.5, fs, 1.0).unwrap();
        let c = Edit::BaudCorrection {
            existing: 2373.0,
            required: 2400.0,
        }
        .run(&s, &|| false)
        .unwrap();
        assert!((c.sample_rate() - fs).abs() < 1e-9);
        // Exact single-frequency DFT: the 27 Hz move is far below one FFT bin.
        let amp = |f: f64| {
            let mut v = vec![0.0_f32; c.len()];
            c.read_real(0, &mut v);
            let (mut re, mut im) = (0.0_f64, 0.0_f64);
            for (i, x) in v.iter().enumerate().skip(4000).take(40_000) {
                let ph = -std::f64::consts::TAU * f * i as f64 / fs;
                re += f64::from(*x) * ph.cos();
                im += f64::from(*x) * ph.sin();
            }
            2.0 * (re * re + im * im).sqrt() / 40_000.0
        };
        assert!(
            (amp(2400.0) - 0.5).abs() < 0.01,
            "tone now at 2400 Hz: {}",
            amp(2400.0)
        );
        assert!(
            amp(2373.0) < 0.05,
            "nothing left at 2373 Hz: {}",
            amp(2373.0)
        );
        assert!(
            Edit::BaudCorrection {
                existing: 1000.0,
                required: 2000.0
            }
            .run(&s, &|| false)
            .is_err()
        );
    }

    #[test]
    fn area_delete_only_touches_the_rectangle() {
        let fs = 16_000.0;
        let s = tone(1000.0, 0.5, fs, 2.0).unwrap();
        let d = Edit::AreaDelete {
            start: 8000,
            end: 24_000,
            lo: 800.0,
            hi: 1200.0,
        }
        .run(&s, &|| false)
        .unwrap();
        let mut v = vec![0.0_f32; d.len()];
        d.read_real(0, &mut v);
        let rms = |a: usize, b: usize| {
            (v[a..b].iter().map(|x| x * x).sum::<f32>() / (b - a) as f32).sqrt()
        };
        assert!(rms(12_000, 20_000) < 1e-3, "deleted inside");
        assert!(
            (rms(28_000, 31_000) - 0.5 / 2_f32.sqrt()).abs() < 1e-3,
            "kept outside"
        );
    }

    #[test]
    fn centre_frequency_follows_the_spectrum() {
        let fs = 48_000.0;
        let rf = 14.49e6;
        let x: Vec<Complex32> = (0..8000)
            .map(|n| {
                let ph = std::f64::consts::TAU * 3000.0 * f64::from(n) / fs;
                Complex32::new(ph.cos() as f32 * 0.5, ph.sin() as f32 * 0.5)
            })
            .collect();
        let s = Signal::complex(x, fs).unwrap().with_center_frequency(rf);
        let cf = |e: Edit| e.run(&s, &|| false).unwrap().center_frequency();
        // A carrier at RF r stays at r: 0 Hz moves by -hz, so the centre does too.
        assert!((cf(Edit::Shift { hz: -1000.0 }) - (rf + 1000.0)).abs() < 1e-6);
        assert!(
            (cf(Edit::ShiftToBase {
                lo: 2000.0,
                hi: 4000.0,
                inverted: false,
            }) - (rf + 2000.0))
                .abs()
                < 1e-6
        );
        // Complex 0 Hz lands at fs/2 of the real result.
        assert!((cf(Edit::IqToReal) - (rf - fs / 2.0)).abs() < 1e-6);
        // Unchanged axis.
        for e in [
            Edit::BandSelect {
                lo: 2000.0,
                hi: 4000.0,
            },
            Edit::Copy { start: 0, end: 100 },
            Edit::Resample { rate: 24_000.0 },
            Edit::Lowpass { cutoff: 5000.0 },
            Edit::Median { window: 3 },
        ] {
            assert!((cf(e.clone()) - rf).abs() < 1e-6, "{}", e.label());
        }
        // Mirrored or non-spectral axes have no RF meaning.
        for e in [
            Edit::Invert,
            Edit::ShiftToBase {
                lo: 2000.0,
                hi: 4000.0,
                inverted: true,
            },
            Edit::Power { n: 2 },
            Edit::AmDetector,
            Edit::PhaseDetector,
            Edit::Acf {
                start: 0,
                end: 1000,
            },
            Edit::Ccf {
                start: 100,
                end: 300,
            },
        ] {
            assert!(cf(e.clone()).abs() < f64::MIN_POSITIVE, "{}", e.label());
        }
        // An unknown centre stays unknown.
        let plain = Signal::complex(vec![Complex32::new(0.5, 0.0); 64], fs).unwrap();
        let out = Edit::Shift { hz: 100.0 }.run(&plain, &|| false).unwrap();
        assert!(out.center_frequency().abs() < f64::MIN_POSITIVE);
    }

    #[test]
    fn bands_outside_the_signal_are_errors() {
        let fs = 8000.0;
        let s = tone(1000.0, 0.5, fs, 0.5).unwrap();
        let e = Edit::BandSelect {
            lo: 5000.0,
            hi: 6000.0,
        }
        .run(&s, &|| false);
        assert!(matches!(
            e,
            Err(Error::InvalidParameter { name: "band", .. })
        ));
        let iq = Signal::complex(vec![Complex32::new(0.5, 0.0); 4000], fs).unwrap();
        for (lo, hi) in [(4500.0, 5000.0), (-6000.0, -4500.0)] {
            assert!(Edit::BandReject { lo, hi }.run(&iq, &|| false).is_err());
        }
        assert!(
            Edit::BandSelect {
                lo: f64::NAN,
                hi: 100.0
            }
            .run(&s, &|| false)
            .is_err()
        );
        assert!(Edit::Shift { hz: f64::NAN }.run(&s, &|| false).is_err());
        assert!(
            Edit::Shift { hz: f64::INFINITY }
                .run(&s, &|| false)
                .is_err()
        );
    }

    #[test]
    fn effective_band_orders_and_clamps() {
        assert_eq!(
            effective_band(900.0, -100.0, 2000.0, SampleKind::Real),
            (0.0, 900.0)
        );
        assert_eq!(
            effective_band(-1500.0, 1500.0, 2000.0, SampleKind::Complex),
            (-1000.0, 1000.0)
        );
        let (lo, hi) = effective_band(1200.0, 1500.0, 2000.0, SampleKind::Real);
        assert!(hi <= lo);
    }

    #[test]
    fn shift_to_base_uses_the_clamped_band() {
        let fs = 16_000.0;
        let s = tone(500.0, 1.0, fs, 2.0).unwrap();
        // lo = -100 Hz is clamped to 0 Hz for a real signal, so nothing moves.
        let base = Edit::ShiftToBase {
            lo: -100.0,
            hi: 1000.0,
            inverted: false,
        }
        .run(&s, &|| false)
        .unwrap();
        assert!(level(&base, 500.0) > -1.0, "{}", level(&base, 500.0));
        assert!(level(&base, 600.0) < level(&base, 500.0) - 30.0);
    }

    #[test]
    fn shift_to_base_on_real_does_not_fold_in_energy_below_the_band() {
        let fs = 16_000.0;
        // A carrier 10 Hz below the band's lower edge (inside the filter skirt).
        let t = transition_for(fs, 2000.0, 3000.0).unwrap();
        assert!(t > 20.0);
        let s = tone(2000.0 - 10.0, 1.0, fs, 2.0).unwrap();
        let base = Edit::ShiftToBase {
            lo: 2000.0,
            hi: 3000.0,
            inverted: false,
        }
        .run(&s, &|| false)
        .unwrap();
        assert!(level(&base, 10.0) < -70.0, "folded {}", level(&base, 10.0));
        let inv = Edit::ShiftToBase {
            lo: 1000.0,
            hi: 2000.0,
            inverted: true,
        };
        let s2 = tone(2000.0 + 10.0, 1.0, fs, 2.0).unwrap();
        let out = inv.run(&s2, &|| false).unwrap();
        assert!(level(&out, 10.0) < -70.0, "folded {}", level(&out, 10.0));
    }

    #[test]
    fn narrow_bands_keep_their_edges_at_the_markers() {
        // 30 Hz at 48 kHz: much narrower than the old fs/1000 transition floor.
        let fs = 48_000.0;
        let iq = Signal::complex(vec![Complex32::new(0.0, 0.0); 16], fs).unwrap();
        let b = band(&iq, 1000.0, 1030.0).unwrap();
        assert!(b.t <= 15.0, "transition {}", b.t);
        let f = band_fir(&iq, b.lo, b.hi, b.t).unwrap();
        for edge in [1000.0, 1030.0] {
            let db = f.response_db(edge, fs);
            assert!((db + 6.0).abs() < 0.5, "{edge} Hz reads {db:.2} dB");
        }
        assert!(f.response_db(1015.0, fs).abs() < 0.05);
        assert!(f.response_db(1000.0 - b.t, fs) < -80.0);
        assert!(f.response_db(1030.0 + b.t, fs) < -80.0);
    }

    #[test]
    fn lowpass_follows_low_cutoffs() {
        let fs = 48_000.0;
        let pass = tone(50.0, 1.0, fs, 1.0).unwrap();
        let lp = Edit::Lowpass { cutoff: 50.0 }
            .run(&pass, &|| false)
            .unwrap();
        let mut v = vec![0.0_f32; lp.len()];
        lp.read_real(0, &mut v);
        let peak = v[20_000..28_000]
            .iter()
            .map(|x| x.abs())
            .fold(0.0, f32::max);
        let db = 20.0 * peak.log10();
        assert!(
            (db + 6.0).abs() < 0.5,
            "50 Hz tone at the 50 Hz cutoff: {db:.2} dB"
        );
        assert!(
            Edit::Lowpass { cutoff: 24_000.0 }
                .run(&pass, &|| false)
                .is_err()
        );
        assert!(
            Edit::Lowpass { cutoff: 1e-6 }
                .run(&pass, &|| false)
                .is_err()
        );
    }

    #[test]
    fn edits_round_trip_through_json() {
        let e = Edit::ShiftToBase {
            lo: 1.0,
            hi: 2.0,
            inverted: true,
        };
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"op\":\"shift_to_base\""));
        assert_eq!(serde_json::from_str::<Edit>(&j).unwrap(), e);
    }
}
