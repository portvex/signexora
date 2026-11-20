//! OFDM analysis for SigNexora (report sections 7.9, 11.6 and 12.3).
//!
//! An OFDM symbol with a cyclic prefix (CP) is `LS = LU + LG` samples long: `LU` is the
//! useful part (the FFT size) and `LG` the guard interval, a copy of the last `LG`
//! samples of the useful part. At a sample rate `Fd`:
//! - channel spacing `Sh = Fd / LU`, symbol rate `Br = Fd / LS`,
//! - `K = LG / LU = Sh / Br - 1`, which does not depend on the recording.
//!
//! Workflow, mirroring SA's OFDM module:
//! 1. [`Baseband::from_signal`]: take a span of a recording as complex samples (real
//!    recordings go through an analytic filter) with the DC removed for the CP search
//!    (the channel tools add it back).
//! 2. [`find_ct`]: search LS within MinBr..MaxBr, find LU and LG and the symbol start,
//!    and return the correlation triangle (CT). [`ct_triangle`] recomputes the triangle
//!    for one LS/LU entry at any start (quick recompute, whole-symbol stepping).
//! 3. [`get_br`]: refine LS fractionally for a precise symbol rate `Br = Fd / LS`, the
//!    fractional LU, and the frequency offset from the phase of the CP correlation.
//! 4. [`Grid`], [`lu_spectrum`], [`occupancy`], [`channel_symbols`]: the LU spectrum,
//!    which channels carry energy, and per-channel symbol values in absolute,
//!    relative or pilot-referenced form, with an optional virtual shift.
//! 5. [`list_of_k`] (stage 1 of SA's K calculator) and [`check_orthogonality`]
//!    (stage 2, which SA never had), then [`native_rate`] and [`resample_to_rate`].
//!
//! # Signal models A, B and C
//!
//! SA analyses each channel in one of three models (report 11.6). Their exact
//! definitions were never published, so this crate does not claim to reproduce them.
//! It offers three views of the same FFT values ([`SymbolView`]) and the closest
//! reading of SA's descriptions is:
//! - **Model A** ("every channel generated as is"): each channel's absolute phase is
//!   meaningful at its native bin. Use [`SymbolView::Absolute`] after a
//!   [`Grid::with_virtual_shift`] that puts the channels back on their native bins
//!   (the report says the virtual shift exists for exactly this). If the shift is
//!   wrong, absolute constellations rotate from symbol to symbol.
//! - **Model B** ("every channel can act as a pilot"): every channel carries its own
//!   phase reference from symbol to symbol. [`SymbolView::Relative`] (value times the
//!   conjugate of the previous symbol on the same channel) is our reading of it.
//! - **Model C** (pilots in model B, data in model A): use
//!   [`SymbolView::PilotReferenced`] for data channels, which rotates every channel by
//!   the phase of a chosen pilot channel in the same symbol, and the relative view for
//!   the pilots themselves.
//!
//! The report's diagnostic still applies: in the right view the constellation
//! orientation changes smoothly (or not at all) from channel to channel.
//!
//! Nothing here panics on bad input; long work takes a `should_stop` callback.

mod baseband;
mod br;
mod ct;
mod grid;
mod kcalc;
mod util;

pub use baseband::{Baseband, MAX_SAMPLES};
pub use br::{BrEstimate, BrParams, get_br};
pub use ct::{CtCandidate, CtParams, CtResult, MAX_LS, Triangle, ct_triangle, find_ct};
pub use grid::{
    ChannelSymbols, Grid, LuSpectrum, Occupancy, SymbolView, channel_symbols,
    channel_symbols_with_stop, lu_spectrum, occupancy, occupancy_with_stop,
};
pub use kcalc::{
    KCandidate, KParams, OrthoParams, check_orthogonality, list_of_k, native_rate, resample_to_rate,
};

/// Errors from OFDM analysis.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// A parameter or selection was unusable.
    #[error("{0}")]
    Invalid(String),
    /// Nothing usable was found in the signal.
    #[error("{0}")]
    NotFound(String),
    /// A DSP step failed.
    #[error(transparent)]
    Dsp(#[from] sn_dsp::Error),
    /// The computation was cancelled.
    #[error("cancelled")]
    Cancelled,
}

impl From<sn_core::Error> for Error {
    fn from(e: sn_core::Error) -> Self {
        Self::Invalid(e.to_string())
    }
}

/// Convenience alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) fn invalid(msg: impl Into<String>) -> Error {
    Error::Invalid(msg.into())
}

pub(crate) fn check_stop(should_stop: &(dyn Fn() -> bool + Sync)) -> Result<()> {
    if should_stop() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
