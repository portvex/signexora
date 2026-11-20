//! Saved sessions, operation log, replay and analysis report export.
//!
//! - [`SessionFile`]: a versioned, JSON-serialisable record of a session: every signal
//!   node with its operation, per-node UI state (opaque to this crate), and the
//!   measurement history. [`save`] and [`load`] write and read it.
//! - [`replay`]: rebuild a [`sn_core::SignalGraph`] from a session by re-opening the
//!   sources and re-running every operation in order. The result is bit-for-bit
//!   identical to the original as long as the source files have not changed.
//! - [`report`]: a Markdown or standalone HTML analysis report.
//!
//! Nothing here panics on bad input: every problem is an [`Error`] or a
//! [`ReplayIssue`] with a message meant for the user.

mod file;
mod replay;
pub mod report;
pub mod tool_id;

pub use file::{
    CaptureOptions, FORMAT_NAME, FORMAT_VERSION, MeasurementRecord, SessionFile, SessionNode,
    SignalSummary, SourceInfo, UI_KEY_ADP, UI_KEY_COUNTERS, UI_KEY_INFO, UI_KEY_MARKERS,
    UI_KEY_NAME, UI_KEY_TONE_GRID, UI_KEY_TONE_TOL, UI_KEY_TONES, UI_KEY_WINDOWS, load, save,
};
pub use replay::{
    GenerateParams, MAX_GENERATED_BYTES, OP_GENERATE, ReplayIssue, ReplayOptions, ReplayOutcome,
    ReplayProgress, generate_operation, replay,
};

/// Errors from saving, loading or replaying sessions.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The session file could not be read or written.
    #[error("{action} {path}: {source}")]
    Io {
        /// What was being done ("cannot read", "cannot write").
        action: &'static str,
        /// File involved.
        path: String,
        /// Underlying error.
        source: std::io::Error,
    },
    /// The file is not a valid session.
    #[error(
        "{0}. The file is not a valid SigNexora session; check that it is the file you meant \
         and that it was not edited by hand into invalid JSON"
    )]
    Invalid(String),
    /// The session cannot be written because it holds a value JSON cannot store.
    #[error("{0}")]
    Unsaveable(String),
    /// The session was written by a newer program.
    #[error(
        "this session uses format version {found}, but this program reads versions up to \
         {supported}. Update SigNexora to open it"
    )]
    NewerVersion {
        /// Version in the file, exactly as written there.
        found: u64,
        /// Highest version this program reads.
        supported: u32,
    },
    /// The replay was cancelled.
    #[error("cancelled")]
    Cancelled,
}

/// Convenience alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;
