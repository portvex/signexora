//! PhasePlane engine, PSK/QAM decisions and the universal FSK/MFSK demodulator.
//!
//! - [`phase`]: carrier and clock recovery, constellations, angles, codes (SA: PhasePlane).
//! - [`mfsk`]: the universal FSK/MFSK demodulator: up to 256 decision levels, RRC or
//!   averaging prefilter, automatic or manual symbol sync with clock-drift tracking,
//!   adaptive decision levels, eye diagrams, and character, hex and bit output
//!   including the APCO-25 C4FM dibit maps.
//!
//! Both modules return the one crate [`Error`] type ([`mfsk::Error`] is the same type
//! re-exported), so a caller matches a single `Cancelled` variant.

pub mod mfsk;
pub mod phase;

/// Errors from demodulation (PhasePlane and the FSK/MFSK demodulator).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// A setting or selection is out of range.
    #[error("invalid setting '{name}': {reason}")]
    Invalid {
        /// Setting name.
        name: &'static str,
        /// What is wrong and how to fix it.
        reason: String,
    },
    /// The input holds I/Q samples instead of a frequency track.
    #[error(
        "the signal holds complex (I/Q) samples; run the phase detector first so the demodulator gets a frequency track in Hz"
    )]
    NotFrequencyTrack,
    /// The input is too short for the requested baud.
    #[error(
        "the signal holds {have} samples, but at least {need} are needed for a few symbols at this baud; select a longer fragment or check the baud rate"
    )]
    TooShort {
        /// Samples available.
        have: usize,
        /// Samples needed.
        need: usize,
    },
    /// The user stopped the job.
    #[error("cancelled")]
    Cancelled,
    /// A filter could not be built or run.
    #[error(transparent)]
    Dsp(sn_dsp::Error),
    /// Level finding failed.
    #[error(transparent)]
    Analysis(sn_analysis::Error),
}

/// A cancelled DSP step becomes [`Error::Cancelled`], so callers see one variant.
impl From<sn_dsp::Error> for Error {
    fn from(e: sn_dsp::Error) -> Self {
        match e {
            sn_dsp::Error::Cancelled => Self::Cancelled,
            other => Self::Dsp(other),
        }
    }
}

/// A cancelled measurement becomes [`Error::Cancelled`], so callers see one variant.
impl From<sn_analysis::Error> for Error {
    fn from(e: sn_analysis::Error) -> Self {
        match e {
            sn_analysis::Error::Cancelled => Self::Cancelled,
            other => Self::Analysis(other),
        }
    }
}

impl From<sn_core::Error> for Error {
    fn from(e: sn_core::Error) -> Self {
        Self::Invalid {
            name: "signal",
            reason: e.to_string(),
        }
    }
}

/// Convenience alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Build an [`Error::Invalid`].
pub(crate) fn invalid(name: &'static str, reason: impl Into<String>) -> Error {
    Error::Invalid {
        name,
        reason: reason.into(),
    }
}
