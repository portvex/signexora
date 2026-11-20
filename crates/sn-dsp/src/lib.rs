//! DSP building blocks for SigNexora.
//!
//! - [`window`]: FFT window functions (SA's default is Kaiser with beta 8).
//! - [`spectrum`]: windowed FFT spectra in dBFS and sonogram columns for any zoom.
//! - [`overview`]: a min/max envelope pyramid for drawing waveforms of any length.
//! - [`fir`], [`mix`], [`resample`]: filters, frequency shifting, resampling.
//! - [`ops`]: SA's editing operations as serialisable [`ops::Edit`] values.
//! - [`detect`]: AM, modified-AM, half-wave and phase detectors, powers, ACF/CCF and
//!   FSK resynthesis.
//! - [`equalize`]: the ISD corrector, a blind adaptive equalizer (CMA and
//!   decision-directed M-PSK).
//!
//! Long computations take a `should_stop` callback so callers can cancel them, and
//! nothing here panics on bad input.

pub mod detect;
pub mod equalize;
pub mod fir;
pub mod mix;
pub mod ops;
pub mod overview;
pub mod resample;
pub mod spectrum;
pub mod window;

/// Errors from DSP setup.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// A parameter was out of range.
    #[error("invalid parameter '{name}': {reason}")]
    InvalidParameter {
        /// Parameter name.
        name: &'static str,
        /// What is wrong with it.
        reason: String,
    },
    /// The computation was cancelled.
    #[error("cancelled")]
    Cancelled,
    /// The result did not form a valid signal.
    #[error("{0}")]
    Signal(String),
}

impl From<sn_core::Error> for Error {
    fn from(e: sn_core::Error) -> Self {
        Self::Signal(e.to_string())
    }
}

/// Convenience alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// True for finite-or-infinite values above zero; false for zero, negatives and NaN
/// (so NaN parameters are rejected rather than slipping through `x <= 0.0` checks).
pub(crate) fn is_positive(x: f64) -> bool {
    x > 0.0
}
