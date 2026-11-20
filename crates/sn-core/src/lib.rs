//! Core types for SigNexora.
//!
//! - [`Signal`]: an immutable block of real or complex samples with its sample rate.
//! - [`SignalGraph`]: the derived-signal DAG. Every operation adds a node that records
//!   which signals it came from and with which parameters, so analyses can be undone,
//!   compared and replayed.
//! - [`MeasurementContext`]: named measurements (baud rate, carrier, period, ...) that
//!   later tools pre-fill from, with provenance.
//! - [`job`]: background jobs with progress reporting and cancellation.

mod error;
mod graph;
pub mod job;
mod measure;
mod signal;

pub use error::{Error, Result};
pub use graph::{Node, NodeId, Operation, SignalGraph};
pub use measure::{Measurement, MeasurementContext, MeasurementKind};
pub use num_complex::Complex32;
pub use signal::{SampleKind, SampleSource, SampleUnit, Samples, Signal};
