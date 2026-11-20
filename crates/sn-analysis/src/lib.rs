//! Measurements for SigNexora.
//!
//! - [`line`]: precise frequency of a spectral line (SA: Measurement of Freq).
//! - [`adp`]: automatic keying-rate estimation (SA: ADP).
//! - [`histogram`]: value histograms (SA: Sum) and their peaks (tone frequencies).
//! - [`tones`]: tone-grid size (SA: Count freqs).
//! - [`raster`]: the VMW raster with fractional periods and strobing.
//!
//! Nothing here panics on bad input; long work takes a `should_stop` callback.

pub mod adp;
pub mod histogram;
pub mod line;
pub mod raster;
pub mod tones;

/// Errors from measurements.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// A parameter or selection was unusable.
    #[error("{0}")]
    Invalid(String),
    /// No line or peak could be found.
    #[error("{0}")]
    NotFound(String),
    /// A DSP step failed.
    #[error(transparent)]
    Dsp(sn_dsp::Error),
    /// The computation was cancelled.
    #[error("cancelled")]
    Cancelled,
}

impl From<sn_dsp::Error> for Error {
    /// A DSP step that was cancelled reports [`Error::Cancelled`], like the
    /// measurement's own checks, so callers need to match only one variant.
    fn from(e: sn_dsp::Error) -> Self {
        match e {
            sn_dsp::Error::Cancelled => Self::Cancelled,
            e => Self::Dsp(e),
        }
    }
}

impl From<sn_core::Error> for Error {
    fn from(e: sn_core::Error) -> Self {
        Self::Invalid(e.to_string())
    }
}

/// Convenience alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;
