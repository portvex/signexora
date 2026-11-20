//! Test-signal generator for SigNexora.
//!
//! Plays the role SA's companion tool OCG played: produce signals whose parameters are
//! known exactly, so every analysis tool can be tested against ground truth.
//!
//! All generators are deterministic: the same parameters and seed give the same samples.
//! Phase 0 provides tones, white noise, continuous-phase MFSK and rectangular-pulse PSK.
//! Phase 6 adds C4FM (APCO-25 style 4-level FSK, shaped or rectangular).
//! Phase 7 adds OFDM with a cyclic prefix ([`ofdm`], [`ofdm_real`]).
//!
//! The full OCG-style back end adds:
//! - [`linear`]: PSK and QAM (BPSK to QAM-256, pi/4-DQPSK, differential PSK) with
//!   rectangular, raised-cosine or root-raised-cosine pulses, real or complex output,
//!   any (also fractional) number of samples per symbol;
//! - [`fsk`]: continuous-phase FSK with 2 to 256 tones, plain or Gaussian (GFSK), with
//!   MSK and GMSK helpers ([`FskParams::msk`], [`FskParams::gmsk`]);
//! - [`ofdm_model`], [`ofdm_model_real`]: OFDM in OCG's models A, B and C with pilots;
//! - [`apply_channel`]: frequency offset and drift, multipath, sample-clock offset,
//!   AWGN and clipping;
//! - [`calc`]: the OFDM-with-CP parameter calculator and the K list;
//! - [`GeneratorSpec`] and [`generate`]: a serializable description of any of the
//!   above with presets, producing the signal, a description and the ground truth.

mod c4fm;
pub mod calc;
mod channel;
mod cpfsk;
mod fsk;
pub(crate) mod linear;
mod noise;
mod ofdm;
mod ofdm_model;
mod psk;
mod rng;
pub mod spec;
mod tone;

pub use c4fm::{
    C4FM_ALPHA, C4FM_DEVIATIONS, C4fmParams, C4fmPulse, c4fm, c4fm_level_of_dibit, c4fm_with_stop,
};
pub use channel::{
    ChannelHints, ChannelParams, MAX_CLOCK_PPM, MAX_DELAY, MAX_TAPS, NoiseLevel, Tap,
    apply_channel, apply_channel_with_stop,
};
pub use cpfsk::{FskParams, FskShaping, fsk, fsk_with_stop};
pub use fsk::{MfskParams, mfsk};
pub use linear::{
    Constellation, LinearOutput, LinearParams, MAX_PULSE_SPAN, PULSE_SPAN, Shaping, linear,
    linear_with_stop,
};
pub use noise::{add_awgn, add_awgn_esn0, add_awgn_snr, signal_power};
pub use ofdm::{OFDM_MAX_LG, OFDM_MAX_LU, OfdmModulation, OfdmOutput, OfdmParams, ofdm, ofdm_real};
pub use ofdm_model::{
    OfdmModel, OfdmModelOutput, OfdmModelParams, PilotPattern, PilotReference, ofdm_model,
    ofdm_model_real,
};
pub use psk::{PskParams, psk};
pub use rng::{Rng, random_symbols};
pub use spec::{
    C4fmSpec, Common, FskSpec, Generated, GeneratorSpec, GroundTruth, Length, LinearSpec,
    NoiseSpec, OfdmSpec, OfdmTruth, PRESET_NAMES, Plan, SignalKind, ToneSpec, TonesSpec, generate,
    generate_with_stop,
};
pub use tone::tone;

/// Errors from the generators.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A parameter was out of range.
    #[error("invalid parameter '{name}': {reason}")]
    InvalidParameter {
        /// Parameter name.
        name: &'static str,
        /// What is wrong with it.
        reason: String,
    },
    /// The resulting signal was invalid (for example a bad sample rate).
    #[error(transparent)]
    Core(#[from] sn_core::Error),
    /// The stop check passed to a `*_with_stop` function asked to stop.
    #[error("cancelled")]
    Cancelled,
}

/// Convenience alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) fn require(cond: bool, name: &'static str, reason: impl Into<String>) -> Result<()> {
    if cond {
        Ok(())
    } else {
        Err(Error::InvalidParameter {
            name,
            reason: reason.into(),
        })
    }
}

/// A cancellation check for the long-running functions: it returns true when the work
/// should stop, which then fails with [`Error::Cancelled`].
pub type StopCheck<'a> = &'a (dyn Fn() -> bool + Sync);

/// How many samples (or symbols) pass between two calls of a [`StopCheck`].
pub(crate) const STOP_EVERY: usize = 4096;

/// A [`StopCheck`] that never stops.
pub(crate) fn never_stop() -> bool {
    false
}

/// Call `stop` every [`STOP_EVERY`] steps of a loop at step `i`.
pub(crate) fn check_stop(stop: StopCheck<'_>, i: usize) -> Result<()> {
    if i.is_multiple_of(STOP_EVERY) && stop() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

/// Largest signal any generator produces, in samples (2^28, about 1 GiB of real `f32`
/// samples). Also the largest lazy signal [`add_awgn`] reads into memory.
pub const MAX_SAMPLES: usize = 1 << 28;

/// Require `value` to be a finite number above 0.
pub(crate) fn require_positive(value: f64, name: &'static str, unit: &str) -> Result<()> {
    require(
        value.is_finite() && value > 0.0,
        name,
        format!("must be a finite number above 0 {unit} (got {value})"),
    )
}

/// Require a finite sample rate above 0 Hz. Every generator checks this first, so a bad
/// rate is not reported as a misleading Nyquist error.
pub(crate) fn check_sample_rate(sample_rate: f64) -> Result<()> {
    require_positive(sample_rate, "sample_rate", "Hz")
}

/// Require a finite, non-negative amplitude.
pub(crate) fn check_amplitude(amplitude: f64) -> Result<()> {
    require(
        amplitude.is_finite() && amplitude >= 0.0,
        "amplitude",
        format!("must be a finite number of at least 0 (got {amplitude})"),
    )
}

/// Turn a sample count computed in `f64` into a length, refusing anything that is not
/// finite or that exceeds [`MAX_SAMPLES`], before any memory is allocated.
pub(crate) fn output_len(count: f64, name: &'static str) -> Result<usize> {
    let rounded = count.round();
    require(
        rounded.is_finite() && rounded >= 0.0,
        name,
        "the signal length is not a finite number of samples; check the sample rate, \
         duration and symbol count",
    )?;
    require(
        rounded <= MAX_SAMPLES as f64,
        name,
        format!(
            "the signal would be {rounded:.0} samples, above the 2^28 limit; lower the \
             sample rate, duration or symbol count"
        ),
    )?;
    Ok(rounded as usize)
}

/// Number of whole samples covering `count` symbols at `baud` and `sample_rate`, or an
/// error when that exceeds [`MAX_SAMPLES`].
pub(crate) fn samples_for_symbols(count: usize, baud: f64, sample_rate: f64) -> Result<usize> {
    output_len(count as f64 * sample_rate / baud, "symbols")
}

/// Index of the symbol that sample instant `t` (in samples, may be fractional) belongs
/// to. Works for non-integer samples per symbol, so symbol boundaries never drift.
pub(crate) fn symbol_index(t: f64, baud: f64, sample_rate: f64) -> usize {
    (t * baud / sample_rate).floor() as usize
}

/// Symbol `k`, or the last symbol when rounding puts a sample just past the end.
/// `symbols` is never empty here (checked by every generator), so 0 is never used.
pub(crate) fn symbol_at(symbols: &[usize], k: usize) -> usize {
    symbols
        .get(k)
        .or_else(|| symbols.last())
        .copied()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_len_refuses_huge_and_non_finite_counts() {
        assert_eq!(output_len(10.4, "x").unwrap(), 10);
        assert!(output_len(f64::INFINITY, "x").is_err());
        assert!(output_len(f64::NAN, "x").is_err());
        assert!(output_len(1e300, "x").is_err());
        assert!(output_len(MAX_SAMPLES as f64 + 1.0, "x").is_err());
        assert!(samples_for_symbols(10, 1e-300, 48_000.0).is_err());
    }
}
