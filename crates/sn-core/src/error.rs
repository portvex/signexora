use crate::NodeId;

/// Errors raised by `sn-core`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A sample rate was zero, negative or not finite.
    #[error("invalid sample rate {0} Hz: it must be a finite number greater than zero")]
    InvalidSampleRate(f64),
    /// A graph operation referred to a node that does not exist.
    #[error(
        "signal node {0} does not exist in this session; it may have been removed, or the \
         session file refers to a signal that was not saved"
    )]
    UnknownNode(NodeId),
    /// A derived node was added without any input.
    #[error("operation '{0}' needs at least one input signal")]
    NoInputs(String),
    /// A sample range fell outside the signal.
    #[error(
        "range {start}..{end} is outside the signal (length {len}); choose a range within 0..{len}"
    )]
    RangeOutOfBounds {
        /// Requested start index.
        start: usize,
        /// Requested end index (exclusive).
        end: usize,
        /// Signal length.
        len: usize,
    },
    /// A sample range had its start after its end.
    #[error("range start {start} is after its end {end}; select the range from left to right")]
    InvalidRange {
        /// Requested start index.
        start: usize,
        /// Requested end index (exclusive).
        end: usize,
    },
    /// A range was too large to hold in memory.
    #[error(
        "the selection or signal ({samples} samples) is too large to load into memory; \
         choose a shorter range"
    )]
    TooLarge {
        /// Number of samples that were requested.
        samples: usize,
    },
    /// A lazy sample source delivered fewer samples than it claims to hold.
    #[error(
        "the signal source returned only {got} of {expected} samples starting at sample \
         {start}; the file may have been truncated or changed on disk, reopen it and try again"
    )]
    ShortRead {
        /// First sample requested.
        start: usize,
        /// Number of samples requested.
        expected: usize,
        /// Number of samples actually delivered.
        got: usize,
    },
    /// A tool produced a NaN or infinite measurement.
    #[error(
        "tool '{tool}' produced an invalid {kind} value ({value}); the measurement was not \
         stored, check the selection and try again"
    )]
    InvalidMeasurement {
        /// Short name of the measurement kind (for example `Br`).
        kind: String,
        /// Tool that produced it.
        tool: String,
        /// The offending value, as text (the value or its uncertainty).
        value: String,
    },
}

/// Convenience alias for results in `sn-core`.
pub type Result<T, E = Error> = std::result::Result<T, E>;
