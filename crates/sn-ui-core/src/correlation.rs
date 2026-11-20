//! SA 6.2.4.2's question before an ACF or CCF and its size limits, as plain
//! data: what the computation is expected to cost, and the text of the
//! question or of the error box that refuses it.

use sn_core::{SampleKind, Signal};
use sn_dsp::detect::{MAX_ACF_SAMPLES, MAX_CCF_TEMPLATE};
use sn_dsp::ops::Edit;

use crate::format;
use crate::t;

/// SA's limit on the memory an ACF may need, in MB.
pub const ACF_LIMIT_MB: f64 = 70.0;
/// SA's limit on the CCF fragment, in MB.
pub const CCF_LIMIT_MB: f64 = 50.0;

/// Seconds per `n log2 n` unit of an FFT, for the running-time estimate.
const FFT_SECONDS_PER_UNIT: f64 = 2e-9;

/// Bytes in SA's `mBytes`.
const MB: f64 = 1024.0 * 1024.0;

/// What an ACF or CCF is expected to cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    /// Expected running time in seconds.
    pub seconds: f64,
    /// Memory the computation needs, in MB.
    pub megabytes: f64,
    /// The size compared with SA's limit, in MB: the needed memory for an ACF,
    /// the fragment for a CCF.
    pub limited: f64,
    /// SA's limit for `limited`, in MB.
    pub limit: f64,
}

fn fft_seconds(m: usize) -> f64 {
    let m = m as f64;
    m * m.log2().max(1.0) * FFT_SECONDS_PER_UNIT
}

/// The cost of an ACF of `len` samples (SigNexora's method: one zero-padded FFT pair).
pub fn acf_estimate(len: usize) -> Estimate {
    let m = len.saturating_mul(2).next_power_of_two();
    // Input (complex f32), the FFT buffer and the real output.
    let bytes = 8.0 * len as f64 + 8.0 * m as f64 + 4.0 * len as f64;
    Estimate {
        seconds: 2.0 * fft_seconds(m),
        megabytes: bytes / MB,
        limited: bytes / MB,
        limit: ACF_LIMIT_MB,
    }
}

/// The cost of a CCF of a `len`-sample fragment over a `total`-sample recording
/// (SigNexora's method: an FFT convolution, then the local energy).
pub fn ccf_estimate(len: usize, total: usize, complex: bool) -> Estimate {
    let block = len.saturating_mul(4).next_power_of_two().max(4096);
    let step = block.saturating_sub(len).max(1);
    let blocks = total.div_ceil(step).max(1);
    let fragment = len as f64 * if complex { 8.0 } else { 4.0 };
    // Template and its spectrum, the block buffers, the output and the f64 prefix sums.
    let bytes = 16.0 * len as f64 + 16.0 * block as f64 + 12.0 * total as f64 + 8.0;
    Estimate {
        seconds: 2.0 * blocks as f64 * fft_seconds(block),
        megabytes: bytes / MB,
        limited: fragment / MB,
        limit: CCF_LIMIT_MB,
    }
}

/// The selection an ACF or CCF works on, if `edit` is one: start, end and
/// whether it is a CCF.
fn span(edit: &Edit) -> Option<(usize, usize, bool)> {
    match *edit {
        Edit::Acf { start, end } => Some((start, end, false)),
        Edit::Ccf { start, end } => Some((start, end, true)),
        _ => None,
    }
}

/// SA's check before an ACF or CCF on `signal`: `None` when `edit` is neither;
/// otherwise `Ok` with the question to ask (`OK` runs it), or `Err` with the
/// error box text when SA's limit (or SigNexora's own sample limit) is exceeded.
pub fn check(edit: &Edit, signal: &Signal) -> Option<Result<String, String>> {
    let (start, end, ccf) = span(edit)?;
    let len = end.min(signal.len()).saturating_sub(start);
    let complex = signal.kind() == SampleKind::Complex;
    let est = if ccf {
        ccf_estimate(len, signal.len(), complex)
    } else {
        acf_estimate(len)
    };
    let max = if ccf {
        MAX_CCF_TEMPLATE
    } else {
        MAX_ACF_SAMPLES
    };
    Some(if est.limited > est.limit {
        Err(t!(
            "dialogs-corr-limit",
            megabytes = format::dec(est.limited, 1),
            limit = format::dec(est.limit, 0)
        ))
    } else if len > max {
        Err(t!(
            "dialogs-corr-too-long",
            count = max,
            samples = format::count(max as u64)
        ))
    } else {
        Ok(t!(
            "dialogs-corr-confirm",
            seconds = format::dec(est.seconds, 2),
            megabytes = format::dec(est.megabytes, 1)
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acf_estimate_grows_with_the_selection_and_meets_sa_limit() {
        let small = acf_estimate(8000);
        assert!(small.megabytes < 1.0, "{small:?}");
        assert!(small.limited <= small.limit);
        // Two million samples fit in 70 MB, three million do not.
        assert!(acf_estimate(2_000_000).limited <= ACF_LIMIT_MB);
        let big = acf_estimate(3_000_000);
        assert!(big.limited > ACF_LIMIT_MB, "{big:?}");
        assert!(big.seconds > small.seconds);
    }

    #[test]
    fn ccf_limit_is_on_the_fragment() {
        let e = ccf_estimate(1000, 10_000_000, false);
        assert!(e.megabytes > 100.0, "the recording dominates: {e:?}");
        assert!(e.limited < 0.01, "{e:?}");
        assert!((e.limit - CCF_LIMIT_MB).abs() < f64::EPSILON);
        assert!(ccf_estimate(14_000_000, 20_000_000, false).limited > CCF_LIMIT_MB);
        assert!(ccf_estimate(1000, 1000, true).limited > e.limited);
    }

    #[test]
    fn only_correlations_are_asked_about() {
        let s = Signal::real(vec![0.5_f32; 1000], 8000.0).unwrap();
        assert!(check(&Edit::Derivative, &s).is_none());
        let acf = Edit::Acf { start: 0, end: 500 };
        assert!(matches!(check(&acf, &s), Some(Ok(_))));
        let ccf = Edit::Ccf { start: 0, end: 900 };
        assert!(matches!(check(&ccf, &s), Some(Ok(_))));
    }
}
