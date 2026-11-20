//! Automatic keying-rate estimation (SA: ADP, "Auto Define Params").
//!
//! The AM (envelope) and modified-AM (frequency-change) detectors turn the symbol
//! clock into a spectral line. ADP ranks the lines of both detectors by prominence,
//! adds support from the second harmonic (when a stronger line at twice the rate is
//! not just the true clock above a sub-harmonic), and refines the best candidates
//! with
//! [`crate::line::measure_line`]. As SA's author warned, this is a starting point:
//! always confirm the result by eye.

use rayon::prelude::*;
use sn_core::Signal;
use sn_dsp::detect;
use sn_dsp::spectrum::{Combine, SpectrumEngine};
use sn_dsp::window::WindowKind;

use crate::line::{self, measure_line};
use crate::{Error, Result};

/// Which detector produced a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Detector {
    /// Quadrature AM (envelope).
    Am,
    /// Modified AM (frequency changes).
    ModifiedAm,
}

/// One keying-rate candidate.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BaudCandidate {
    /// Estimated rate in Bd.
    pub baud: f64,
    /// Detector whose output showed the line.
    pub detector: Detector,
    /// Line prominence above the median, dB (plus harmonic support).
    pub score: f64,
    /// True when the rate was refined by [`crate::line::measure_line`]; false when
    /// the refinement failed and `baud` is the centre of a coarse spectrum bin,
    /// [`Self::resolution`] wide. That width is fs / FFT size, and the FFT is
    /// capped at half the selection length rounded up to a power of two, so on a
    /// short selection the bin can be far wider than [`MIN_BAUD`].
    ///
    /// Sessions saved before this field existed have no way to mark an unrefined
    /// rate, and refinement nearly always succeeded, so a missing field reads as true.
    #[serde(default = "refined_default")]
    pub refined: bool,
    /// Width of the coarse spectrum bin the line was found in, Bd (fs / FFT size).
    /// An unrefined `baud` is within half of this of the bin's line. Zero means
    /// unknown, for sessions saved before this field existed.
    #[serde(default)]
    pub resolution: f64,
}

/// Value of [`BaudCandidate::refined`] for candidates saved without that field.
const fn refined_default() -> bool {
    true
}

/// Lowest rate searched, Bd.
pub const MIN_BAUD: f64 = 5.0;

/// Estimate the keying rate of samples `start..end`. Returns up to five candidates,
/// best first.
pub fn estimate_baud(
    signal: &Signal,
    start: usize,
    end: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<BaudCandidate>> {
    let end = end.min(signal.len());
    if end.saturating_sub(start) < 1024 {
        return Err(Error::Invalid(
            "select at least 1024 samples (SA recommends 1-20 s of signal)".into(),
        ));
    }
    let part = signal.slice(start, end)?;
    let fs = signal.sample_rate();
    let max_baud = fs / 4.0;
    let mut found: Vec<BaudCandidate> = Vec::new();
    for detector in [Detector::Am, Detector::ModifiedAm] {
        if should_stop() {
            return Err(Error::Cancelled);
        }
        let d = match detector {
            Detector::Am => detect::envelope(&part, should_stop)?,
            Detector::ModifiedAm => detect::modified_am(&part, should_stop)?,
        };
        let d = Signal::real(d, fs)?;
        found.extend(candidates(&d, detector, max_baud, should_stop)?);
    }
    found.sort_by(|a, b| b.score.total_cmp(&a.score));
    // Merge near-duplicates (the same line seen by both detectors).
    let mut merged: Vec<BaudCandidate> = Vec::new();
    for c in found {
        if !merged
            .iter()
            .any(|m| (m.baud - c.baud).abs() < 0.005 * c.baud.max(1.0))
        {
            merged.push(c);
        }
    }
    merged.truncate(5);
    if merged.is_empty() {
        return Err(Error::NotFound(
            "no clock line found; try another detector or a cleaner selection".into(),
        ));
    }
    Ok(merged)
}

/// Lines of one detector output between `MIN_BAUD` and `max_baud`, refined.
fn candidates(
    d: &Signal,
    detector: Detector,
    max_baud: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<BaudCandidate>> {
    let fs = d.sample_rate();
    let n = d.len();
    // Aim for a resolution of MIN_BAUD / 4; the selection length caps the FFT, so
    // a short selection gets a coarser bin.
    let want = (4.0 * fs / MIN_BAUD) as usize;
    let fft = want
        .next_power_of_two()
        .min(n.next_power_of_two() / 2)
        .clamp(1024, 1 << 20);
    let e = SpectrumEngine::new(fft, WindowKind::Hann, d.kind())?;
    // At most 2048 frames, spread evenly over the whole selection.
    let room = n.saturating_sub(fft);
    let hop = (fft / 2).max(room / 2047).max(1);
    let frames: Vec<usize> = (0..=room).step_by(hop).collect();
    let db = e.column_db(d, &frames, Combine::Mean, should_stop)?;
    let at = |hz: f64| -> f32 {
        let k = e.frequency_to_bin(hz, fs).round();
        if k < 0.0 {
            return f32::MIN;
        }
        let k = k as usize;
        db.get(k.saturating_sub(1)..=k + 1)
            .map_or(f32::MIN, |s| s.iter().copied().fold(f32::MIN, f32::max))
    };
    let lo_bin = e.frequency_to_bin(MIN_BAUD, fs).ceil().max(1.0) as usize;
    let hi_bin =
        (e.frequency_to_bin(max_baud, fs).floor() as usize).min(db.len().saturating_sub(2));
    let mut band: Vec<f32> = db.get(lo_bin..=hi_bin).unwrap_or(&[]).to_vec();
    band.sort_by(f32::total_cmp);
    let median = band.get(band.len() / 2).copied().unwrap_or(0.0);
    let mut peaks: Vec<(usize, f32)> = (lo_bin..=hi_bin)
        .filter_map(|k| {
            let (a, b, c) = (db.get(k - 1)?, db.get(k)?, db.get(k + 1)?);
            (b >= a && b >= c && *b - median > 6.0).then_some((k, *b))
        })
        .collect();
    peaks.sort_by(|a, b| b.1.total_cmp(&a.1));
    peaks.truncate(4);
    // Refine on a centred span the line measurement accepts.
    let span = n.min(line::MAX_SAMPLES);
    let first = (n - span) / 2;
    let bin = fs / fft as f64;
    let refine_peak = |&(k, level): &(usize, f32)| -> Result<BaudCandidate> {
        if should_stop() {
            return Err(Error::Cancelled);
        }
        let f = e.bin_frequency(k, fs);
        let fine = match measure_line(
            d,
            first,
            first + span,
            f - 2.0 * bin,
            f + 2.0 * bin,
            should_stop,
        ) {
            Ok(l) if (l.frequency - f).abs() <= 1.5 * bin => Some(l.frequency),
            Ok(_) | Err(Error::NotFound(_) | Error::Invalid(_)) => None,
            Err(e) => return Err(e),
        };
        let refined = fine.unwrap_or(f);
        // A true clock usually has a second harmonic, which earns support. But a
        // sub-harmonic of a strong clock also "has" a second harmonic: the clock
        // itself. When the line at 2f is the stronger one, the support counts only
        // if a line at 3f confirms the series (a sub-harmonic at Br/2 has nothing at
        // 1.5 Br).
        let own = at(refined);
        let second = at(2.0 * refined);
        let third = at(3.0 * refined);
        let series = second <= own || third - median > 6.0;
        let harmonic = if series {
            0.5 * f64::from((second - median).clamp(0.0, 20.0))
        } else {
            0.0
        };
        Ok(BaudCandidate {
            baud: refined,
            detector,
            score: f64::from(level - median) + harmonic,
            refined: fine.is_some(),
            resolution: bin,
        })
    };
    // Each refinement holds a copy of the span. When the copies fit in
    // PARALLEL_REFINE_BYTES the peaks are refined in parallel; either way the
    // results keep the order of the peaks.
    let bytes = span
        .saturating_mul(size_of::<sn_core::Complex32>())
        .saturating_mul(peaks.len());
    if bytes <= PARALLEL_REFINE_BYTES {
        peaks.par_iter().map(refine_peak).collect()
    } else {
        peaks.iter().map(refine_peak).collect()
    }
}

/// Most bytes of span copies refined at once (see [`candidates`]).
const PARALLEL_REFINE_BYTES: usize = 256 << 20;

#[cfg(test)]
mod tests {
    use sn_synth::{MfskParams, PskParams, add_awgn_snr, mfsk, psk, random_symbols};

    use super::*;

    #[test]
    fn candidates_saved_without_the_refined_flag_read_as_refined() {
        let old = r#"{"baud":50.0,"detector":"Am","score":12.0}"#;
        let c: BaudCandidate = serde_json::from_str(old).unwrap();
        assert!(c.refined);
        let coarse = r#"{"baud":50.0,"detector":"Am","score":12.0,"refined":false}"#;
        let c: BaudCandidate = serde_json::from_str(coarse).unwrap();
        assert!(!c.refined);
        assert_eq!(c.resolution, 0.0);
        let sized =
            r#"{"baud":50.0,"detector":"Am","score":12.0,"refined":false,"resolution":3.9}"#;
        let c: BaudCandidate = serde_json::from_str(sized).unwrap();
        assert_eq!(c.resolution, 3.9);
    }

    #[test]
    fn finds_the_psk_rate() {
        let fs = 9600.0;
        let p = PskParams {
            order: 2,
            baud: 1200.0,
            carrier: 2400.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let s = add_awgn_snr(&psk(&p, &random_symbols(12_000, 2, 1)).unwrap(), 15.0, 2).unwrap();
        let c = estimate_baud(&s, 0, s.len(), &|| false).unwrap();
        assert!((c[0].baud - 1200.0).abs() < 0.5, "{c:?}");
    }

    #[test]
    fn finds_the_mfsk_rate_with_the_modified_detector() {
        let fs = 8000.0;
        let p = MfskParams {
            order: 8,
            baud: 62.5,
            spacing: 125.0,
            center: 1500.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let s = add_awgn_snr(&mfsk(&p, &random_symbols(800, 8, 4)).unwrap(), 15.0, 6).unwrap();
        let c = estimate_baud(&s, 0, s.len(), &|| false).unwrap();
        assert!((c[0].baud - 62.5).abs() < 0.2, "{c:?}");
        assert_eq!(c[0].detector, Detector::ModifiedAm);
    }

    #[test]
    fn short_selection_is_refused() {
        let s = sn_synth::tone(100.0, 0.5, 8000.0, 0.01).unwrap();
        assert!(estimate_baud(&s, 0, s.len(), &|| false).is_err());
    }

    #[test]
    fn cancellation_inside_refinement_is_reported() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let fs = 9600.0;
        let p = PskParams {
            order: 2,
            baud: 1200.0,
            carrier: 2400.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let s = add_awgn_snr(&psk(&p, &random_symbols(4000, 2, 1)).unwrap(), 15.0, 2).unwrap();
        // Count the checks of a full run, then stop at every point along the way.
        let total = AtomicUsize::new(0);
        estimate_baud(&s, 0, s.len(), &|| {
            total.fetch_add(1, Ordering::Relaxed);
            false
        })
        .unwrap();
        let total = total.into_inner();
        assert!(total > 4);
        for stop_at in (0..total).step_by((total / 25).max(1)) {
            let calls = AtomicUsize::new(0);
            let r = estimate_baud(&s, 0, s.len(), &|| {
                calls.fetch_add(1, Ordering::Relaxed) >= stop_at
            });
            assert!(
                matches!(r, Err(Error::Cancelled)),
                "stop at {stop_at}: {r:?}"
            );
        }
    }

    #[test]
    fn sub_harmonic_does_not_outrank_the_clock() {
        // A carrier whose envelope has a strong clock line at 100 Bd and a spurious
        // line at 50 Bd 8 dB weaker; no lines at 150 or 200 Bd.
        let fs = 4000.0;
        let n = 80_000;
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let t = f64::from(i) / fs;
                let env = 1.0
                    + 0.3 * (std::f64::consts::TAU * 100.0 * t).cos()
                    + 0.12 * (std::f64::consts::TAU * 50.0 * t).cos();
                (0.4 * env * (std::f64::consts::TAU * 1000.0 * t).cos()) as f32
            })
            .collect();
        let s = add_awgn_snr(&Signal::real(x, fs).unwrap(), 20.0, 4).unwrap();
        let c = estimate_baud(&s, 0, s.len(), &|| false).unwrap();
        assert!((c[0].baud - 100.0).abs() < 0.1, "{c:?}");
        assert!(c[0].refined);
        // fs / FFT size, with the FFT at 4096 for this rate and length.
        assert!((c[0].resolution - fs / 4096.0).abs() < 1e-9, "{c:?}");
    }
}
