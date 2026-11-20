//! Mapping the errors of the lower crates onto job errors, keeping cancellation
//! distinct from failure. Each mapping lists every error kind without a
//! catch-all, so a new kind is flagged by the compiler instead of being
//! mis-reported.

use sn_core::job::JobError;

/// Map a generator error to a job error, keeping cancellation distinct from failure.
pub fn synth_to_job(e: &sn_synth::Error) -> JobError {
    match e {
        sn_synth::Error::Cancelled => JobError::Cancelled,
        e @ (sn_synth::Error::InvalidParameter { .. } | sn_synth::Error::Core(_)) => {
            JobError::Failed(e.to_string())
        }
    }
}

/// Map a file error to a job error, keeping cancellation distinct from failure.
pub fn io_to_job(e: sn_io::Error) -> JobError {
    match e {
        sn_io::Error::Cancelled => JobError::Cancelled,
        e @ (sn_io::Error::Io { .. }
        | sn_io::Error::Wav(_)
        | sn_io::Error::Malformed(_)
        | sn_io::Error::FractionalSampleRate(_)
        | sn_io::Error::NeedsRawFormat
        | sn_io::Error::Unsupported(_)
        | sn_io::Error::Core(_)) => JobError::Failed(e.to_string()),
    }
}

/// Map a measurement error to a job error, keeping cancellation distinct from failure.
pub fn analysis_to_job(e: sn_analysis::Error) -> JobError {
    match e {
        sn_analysis::Error::Cancelled => JobError::Cancelled,
        e @ (sn_analysis::Error::Invalid(_)
        | sn_analysis::Error::NotFound(_)
        | sn_analysis::Error::Dsp(_)) => JobError::Failed(e.to_string()),
    }
}

/// Map a DSP error to a job error without a catch-all, so new error kinds are not
/// silently mis-reported.
pub fn dsp_to_job(e: sn_dsp::Error) -> JobError {
    match e {
        sn_dsp::Error::Cancelled => JobError::Cancelled,
        e @ (sn_dsp::Error::InvalidParameter { .. } | sn_dsp::Error::Signal(_)) => {
            JobError::Failed(e.to_string())
        }
    }
}

/// Map a demodulator error to a job error without a catch-all, so a new error kind
/// is flagged by the compiler instead of being mis-reported.
pub fn demod_to_job(e: sn_demod::Error) -> JobError {
    match e {
        sn_demod::Error::Cancelled => JobError::Cancelled,
        sn_demod::Error::Dsp(d) => dsp_to_job(d),
        e @ (sn_demod::Error::Invalid { .. }
        | sn_demod::Error::NotFrequencyTrack
        | sn_demod::Error::TooShort { .. }
        | sn_demod::Error::Analysis(_)) => JobError::Failed(e.to_string()),
    }
}

/// Map an OFDM error to a job error without a catch-all.
pub fn ofdm_to_job(e: &sn_ofdm::Error) -> JobError {
    match e {
        sn_ofdm::Error::Cancelled => JobError::Cancelled,
        sn_ofdm::Error::Dsp(d) => dsp_to_job(d.clone()),
        e @ (sn_ofdm::Error::Invalid(_) | sn_ofdm::Error::NotFound(_)) => {
            JobError::Failed(e.to_string())
        }
    }
}

/// Map a session error to a job error, so a stopped replay reads as cancelled
/// rather than as a failure.
pub fn session_to_job(e: sn_session::Error) -> JobError {
    match e {
        sn_session::Error::Cancelled => JobError::Cancelled,
        e @ (sn_session::Error::Io { .. }
        | sn_session::Error::Invalid(_)
        | sn_session::Error::Unsaveable(_)
        | sn_session::Error::NewerVersion { .. }) => JobError::Failed(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_errors_keep_cancellation_apart_from_failure() {
        assert!(matches!(
            session_to_job(sn_session::Error::Cancelled),
            JobError::Cancelled
        ));
        assert!(matches!(
            session_to_job(sn_session::Error::Invalid("bad".into())),
            JobError::Failed(m) if m.starts_with("bad")
        ));
    }
}
