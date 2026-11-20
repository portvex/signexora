//! FIR filter design (windowed sinc) and streaming FFT convolution.
//!
//! All filters here are linear-phase. [`Fir::apply`] compensates the group delay, so
//! output sample `n` lines up with input sample `n`.

use std::f64::consts::PI;
use std::sync::Arc;

use rayon::prelude::*;
use rustfft::num_complex::Complex32 as FftComplex;
use rustfft::{Fft, FftPlanner};
use sn_core::{Complex32, Signal};

use crate::window::WindowKind;
use crate::{Error, Result};

/// A linear-phase FIR filter with complex taps (real filters have zero imaginary parts).
#[derive(Debug, Clone, PartialEq)]
pub struct Fir {
    taps: Vec<Complex32>,
}

/// Kaiser beta for a stopband attenuation in dB (Kaiser's formula).
pub fn kaiser_beta(atten_db: f64) -> f64 {
    if atten_db > 50.0 {
        0.1102 * (atten_db - 8.7)
    } else if atten_db >= 21.0 {
        0.5842 * (atten_db - 21.0).powf(0.4) + 0.078_86 * (atten_db - 21.0)
    } else {
        0.0
    }
}

/// Odd tap count for a Kaiser-window FIR with transition width `transition` (as a
/// fraction of the sample rate) and stopband attenuation `atten_db`.
///
/// Kaiser's formula gives the filter order M; the length is M + 1, rounded up to odd.
pub fn kaiser_taps(transition: f64, atten_db: f64) -> usize {
    let t = transition.clamp(1e-6, 0.5);
    let order = ((atten_db - 7.95) / (14.36 * t)).ceil().max(2.0) as usize;
    (order + 1) | 1
}

/// Narrowest transition band, in Hz, that a Kaiser FIR with `atten_db` stopband
/// attenuation can have at `sample_rate` without exceeding [`MAX_TAPS`].
pub fn min_transition(sample_rate: f64, atten_db: f64) -> f64 {
    // Leave room for the +1 and the round-up to odd in `kaiser_taps`.
    sample_rate * (atten_db + KAISER_MARGIN_DB - 7.95) / (14.36 * (MAX_TAPS - 3) as f64)
}

/// Extra attenuation, in dB, that [`Fir::lowpass`] designs for. Kaiser's beta and
/// length formulas are estimates that can fall about 1 dB short of the asked-for
/// stopband, so the design aims slightly lower.
pub const KAISER_MARGIN_DB: f64 = 1.5;

fn invalid(name: &'static str, reason: impl Into<String>) -> Error {
    Error::InvalidParameter {
        name,
        reason: reason.into(),
    }
}

/// Longest filter accepted, in taps.
pub const MAX_TAPS: usize = 1 << 20;

fn check_taps(n: usize) -> Result<usize> {
    let n = n | 1;
    if n > MAX_TAPS {
        return Err(invalid(
            "taps",
            format!("{n} taps exceed the limit of {MAX_TAPS}; widen the transition band"),
        ));
    }
    Ok(n)
}

/// Windowed-sinc lowpass prototype: real taps, unity gain at DC.
fn sinc_lowpass(cutoff: f64, taps: usize, window: WindowKind) -> Vec<f64> {
    let w = window.coefficients(taps);
    let mid = (taps / 2) as f64;
    let mut h: Vec<f64> = w
        .iter()
        .enumerate()
        .map(|(i, wi)| {
            let x = i as f64 - mid;
            let s = if x == 0.0 {
                2.0 * cutoff
            } else {
                (2.0 * PI * cutoff * x).sin() / (PI * x)
            };
            s * f64::from(*wi)
        })
        .collect();
    let sum: f64 = h.iter().sum();
    if sum.abs() > 1e-12 {
        for v in &mut h {
            *v /= sum;
        }
    }
    h
}

impl Fir {
    /// Build from explicit taps (must be odd-length for exact delay compensation).
    pub fn from_taps(taps: Vec<Complex32>) -> Result<Self> {
        if taps.is_empty() || taps.len().is_multiple_of(2) {
            return Err(invalid("taps", "need an odd, non-zero number of taps"));
        }
        check_taps(taps.len())?;
        Ok(Self { taps })
    }

    /// Lowpass with cutoff `cutoff_hz` (-6 dB point), transition width
    /// `transition_hz` and stopband attenuation `atten_db`.
    pub fn lowpass(
        cutoff_hz: f64,
        transition_hz: f64,
        atten_db: f64,
        sample_rate: f64,
    ) -> Result<Self> {
        if !crate::is_positive(sample_rate) {
            return Err(invalid("sample_rate", "must be above 0 Hz"));
        }
        if !(cutoff_hz > 0.0 && cutoff_hz < sample_rate / 2.0) {
            return Err(invalid(
                "cutoff",
                format!(
                    "{cutoff_hz} Hz must be between 0 and {} Hz",
                    sample_rate / 2.0
                ),
            ));
        }
        if !crate::is_positive(transition_hz) {
            return Err(invalid("transition", "must be above 0 Hz"));
        }
        let half_t = transition_hz / 2.0;
        if !(cutoff_hz - half_t > 0.0 && cutoff_hz + half_t <= sample_rate / 2.0) {
            return Err(invalid(
                "transition",
                format!(
                    "a {transition_hz} Hz transition does not fit around a {cutoff_hz} Hz \
                     cutoff; the cutoff must lie between {half_t} and {} Hz",
                    sample_rate / 2.0 - half_t
                ),
            ));
        }
        let design_db = atten_db + KAISER_MARGIN_DB;
        let n = check_taps(kaiser_taps(transition_hz / sample_rate, design_db))?;
        let h = sinc_lowpass(
            cutoff_hz / sample_rate,
            n,
            WindowKind::Kaiser {
                beta: kaiser_beta(design_db),
            },
        );
        Ok(Self {
            taps: h.iter().map(|v| Complex32::new(*v as f32, 0.0)).collect(),
        })
    }

    /// Complex bandpass passing `lo_hz..hi_hz` only (positive or negative frequencies
    /// as given). On a real signal this keeps only that side of the spectrum; use
    /// [`Self::real_bandpass`] to keep the mirror image too.
    pub fn complex_bandpass(
        lo_hz: f64,
        hi_hz: f64,
        transition_hz: f64,
        atten_db: f64,
        sample_rate: f64,
    ) -> Result<Self> {
        let (lo, hi) = (lo_hz.min(hi_hz), lo_hz.max(hi_hz));
        let half = (hi - lo) / 2.0;
        if !crate::is_positive(half) {
            return Err(invalid("band", "the band must have a non-zero width"));
        }
        let centre = f64::midpoint(lo, hi);
        let lp = Self::lowpass(half, transition_hz, atten_db, sample_rate)?;
        Ok(lp.modulated(centre, sample_rate, 1.0))
    }

    /// Real bandpass passing `lo_hz..hi_hz` and its mirror image (for real signals).
    pub fn real_bandpass(
        lo_hz: f64,
        hi_hz: f64,
        transition_hz: f64,
        atten_db: f64,
        sample_rate: f64,
    ) -> Result<Self> {
        let (lo, hi) = (lo_hz.min(hi_hz).max(0.0), lo_hz.max(hi_hz));
        if hi.is_nan() || hi > sample_rate / 2.0 {
            return Err(invalid(
                "band",
                format!(
                    "the upper edge {hi} Hz is above the {} Hz Nyquist frequency",
                    sample_rate / 2.0
                ),
            ));
        }
        let half = (hi - lo) / 2.0;
        if !crate::is_positive(half) {
            return Err(invalid("band", "the band must have a non-zero width"));
        }
        let centre = f64::midpoint(lo, hi);
        let lp = Self::lowpass(half, transition_hz, atten_db, sample_rate)?;
        // 2 cos(w n) = e^{jwn} + e^{-jwn}: one copy at +centre, one at -centre.
        let up = lp.modulated(centre, sample_rate, 1.0);
        let down = lp.modulated(-centre, sample_rate, 1.0);
        Ok(Self {
            taps: up
                .taps
                .iter()
                .zip(&down.taps)
                .map(|(a, b)| Complex32::new(a.re + b.re, 0.0))
                .collect(),
        })
    }

    /// Analytic-signal filter: passes `0..fs/2`, rejects `-fs/2..0`, with gain 2 in
    /// the passband so `Re(output)` equals the (band-limited) input. Transition bands
    /// of `transition_hz` sit at 0 Hz and at fs/2.
    pub fn analytic(transition_hz: f64, atten_db: f64, sample_rate: f64) -> Result<Self> {
        let quarter = sample_rate / 4.0;
        let lp = Self::lowpass(
            quarter - transition_hz / 2.0,
            transition_hz,
            atten_db,
            sample_rate,
        )?;
        Ok(lp.modulated(quarter, sample_rate, 2.0))
    }

    /// Root-raised-cosine pulse filter for `baud` symbols/s with roll-off `alpha`,
    /// `span` symbols long, unity gain at DC.
    pub fn rrc(baud: f64, alpha: f64, span: usize, sample_rate: f64) -> Result<Self> {
        if !(baud > 0.0 && baud < sample_rate) {
            return Err(invalid("baud", "must be above 0 and below the sample rate"));
        }
        if !(alpha > 0.0 && alpha <= 1.0) {
            return Err(invalid("alpha", "must be in 0 < alpha <= 1"));
        }
        let sps = sample_rate / baud;
        let n = check_taps(((span.max(2) as f64) * sps).ceil() as usize)?;
        let mid = (n / 2) as f64;
        let mut h: Vec<f64> = (0..n)
            .map(|i| {
                let t = (i as f64 - mid) / sps;
                rrc_impulse(t, alpha)
            })
            .collect();
        let sum: f64 = h.iter().sum();
        if sum.abs() > 1e-12 {
            for v in &mut h {
                *v /= sum;
            }
        }
        Ok(Self {
            taps: h.iter().map(|v| Complex32::new(*v as f32, 0.0)).collect(),
        })
    }

    /// Shift the frequency response by `hz`, scaling the taps by `gain`.
    #[must_use]
    pub fn modulated(&self, hz: f64, sample_rate: f64, gain: f64) -> Self {
        let mid = (self.taps.len() / 2) as f64;
        let w = 2.0 * PI * hz / sample_rate;
        Self {
            taps: self
                .taps
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let ph = w * (i as f64 - mid);
                    let rot = Complex32::new((ph.cos() * gain) as f32, (ph.sin() * gain) as f32);
                    t * rot
                })
                .collect(),
        }
    }

    /// The complementary filter `delta - h`: passes what `self` rejects.
    #[must_use]
    pub fn complement(&self) -> Self {
        let mid = self.taps.len() / 2;
        Self {
            taps: self
                .taps
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    if i == mid {
                        Complex32::new(1.0, 0.0) - t
                    } else {
                        -t
                    }
                })
                .collect(),
        }
    }

    /// Number of taps.
    pub fn len(&self) -> usize {
        self.taps.len()
    }

    /// Never true for a constructed filter; present for API completeness.
    pub fn is_empty(&self) -> bool {
        self.taps.is_empty()
    }

    /// The taps.
    pub fn taps(&self) -> &[Complex32] {
        &self.taps
    }

    /// Magnitude response in dB at `hz`.
    pub fn response_db(&self, hz: f64, sample_rate: f64) -> f64 {
        let w = 2.0 * PI * hz / sample_rate;
        let (mut re, mut im) = (0.0_f64, 0.0_f64);
        for (i, t) in self.taps.iter().enumerate() {
            let ph = -w * i as f64;
            let (c, s) = (ph.cos(), ph.sin());
            re += f64::from(t.re) * c - f64::from(t.im) * s;
            im += f64::from(t.re) * s + f64::from(t.im) * c;
        }
        10.0 * (re * re + im * im).max(1e-30).log10()
    }

    /// Filter samples `start..end` of `signal` (delay-compensated, edges zero-padded).
    /// Returns complex output; take the real part for real signals with real taps.
    pub fn apply(
        &self,
        signal: &Signal,
        start: usize,
        end: usize,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<Complex32>> {
        let len = end.saturating_sub(start);
        if len > crate::ops::MAX_OUTPUT_SAMPLES {
            return Err(invalid(
                "size",
                format!(
                    "filtering {len} samples exceeds the limit of {}. Copy a shorter span, \
                     or shift the band down and resample first",
                    crate::ops::MAX_OUTPUT_SAMPLES
                ),
            ));
        }
        let taps = self.taps.len();
        let delay = taps / 2;
        // Overlap-save block size: a power of two several times the filter length.
        let n = (taps * 4).next_power_of_two().max(4096);
        let step = n - (taps - 1);
        let mut planner = FftPlanner::<f32>::new();
        let fwd: Arc<dyn Fft<f32>> = planner.plan_fft_forward(n);
        let inv: Arc<dyn Fft<f32>> = planner.plan_fft_inverse(n);
        let mut h: Vec<FftComplex> = vec![FftComplex::new(0.0, 0.0); n];
        for (d, t) in h.iter_mut().zip(&self.taps) {
            *d = FftComplex::new(t.re, t.im);
        }
        fwd.process(&mut h);
        let scale = 1.0 / n as f32;

        // Each overlap-save block yields `step` outputs from its own input span, so
        // blocks run in parallel and give the same values as a serial pass.
        let scratch_len = fwd
            .get_inplace_scratch_len()
            .max(inv.get_inplace_scratch_len());
        let mut out = vec![Complex32::new(0.0, 0.0); len];
        out.par_chunks_mut(step).enumerate().try_for_each_init(
            || {
                (
                    vec![Complex32::new(0.0, 0.0); n],
                    vec![FftComplex::new(0.0, 0.0); n],
                    vec![FftComplex::new(0.0, 0.0); scratch_len],
                )
            },
            |(block, buf, scratch), (b, dst)| {
                if should_stop() {
                    return Err(Error::Cancelled);
                }
                // Output y[k] (aligned) = sum_j h[j] x[start + k + delay - j]; the block
                // for outputs k0..k0+step needs input from start + k0 + delay - (taps - 1).
                let k0 = b * step;
                let first = (start + k0 + delay) as i64 - (taps as i64 - 1);
                read_padded(signal, first, block);
                for (d, s) in buf.iter_mut().zip(block.iter()) {
                    *d = FftComplex::new(s.re, s.im);
                }
                fwd.process_with_scratch(buf, scratch);
                for (d, hv) in buf.iter_mut().zip(&h) {
                    *d *= hv;
                }
                inv.process_with_scratch(buf, scratch);
                for (o, c) in dst.iter_mut().zip(buf.iter().skip(taps - 1)) {
                    *o = Complex32::new(c.re * scale, c.im * scale);
                }
                Ok(())
            },
        )?;
        Ok(out)
    }
}

/// Read `out.len()` complex samples starting at `first` (may be negative); samples
/// outside the signal are zero.
pub(crate) fn read_padded(signal: &Signal, first: i64, out: &mut [Complex32]) {
    let skip = usize::try_from(-first).unwrap_or(0).min(out.len());
    for o in out.iter_mut().take(skip) {
        *o = Complex32::new(0.0, 0.0);
    }
    let from = usize::try_from(first.max(0)).unwrap_or(0);
    if let Some(rest) = out.get_mut(skip..) {
        signal.read_complex(from, rest);
    }
}

/// Root-raised-cosine impulse response at time `t` (in symbols).
fn rrc_impulse(t: f64, a: f64) -> f64 {
    if t.abs() < 1e-9 {
        return 1.0 - a + 4.0 * a / PI;
    }
    let edge = 1.0 / (4.0 * a);
    if (t.abs() - edge).abs() < 1e-9 {
        return a / 2_f64.sqrt()
            * ((1.0 + 2.0 / PI) * (PI / (4.0 * a)).sin()
                + (1.0 - 2.0 / PI) * (PI / (4.0 * a)).cos());
    }
    let num = (PI * t * (1.0 - a)).sin() + 4.0 * a * t * (PI * t * (1.0 + a)).cos();
    let den = PI * t * (1.0 - (4.0 * a * t).powi(2));
    num / den
}

#[cfg(test)]
mod tests {
    use sn_synth::tone;

    use super::*;

    #[test]
    fn lowpass_meets_its_spec() {
        let fs = 48_000.0;
        let f = Fir::lowpass(3000.0, 500.0, 80.0, fs).unwrap();
        assert_eq!(f.len() % 2, 1);
        assert!(f.response_db(0.0, fs).abs() < 0.01);
        assert!(f.response_db(2700.0, fs).abs() < 0.05);
        assert!((f.response_db(3000.0, fs) + 6.0).abs() < 0.5);
        // Worst stopband value from the stopband edge up to Nyquist.
        let worst = (0..=4150)
            .map(|i| f.response_db(3250.0 + 5.0 * f64::from(i), fs))
            .fold(f64::MIN, f64::max);
        assert!(worst < -80.0, "worst stopband {worst:.2} dB");
        // A short filter, where the order-versus-length difference matters most.
        let short = Fir::lowpass(5000.0, 2000.0, 80.0, fs).unwrap();
        let worst = (0..=1800)
            .map(|i| short.response_db(6000.0 + 10.0 * f64::from(i), fs))
            .fold(f64::MIN, f64::max);
        assert!(worst < -80.0, "short filter worst stopband {worst:.2} dB");
    }

    #[test]
    fn lowpass_rejects_a_transition_that_does_not_fit() {
        let fs = 48_000.0;
        // No passband: the transition is wider than twice the cutoff.
        assert!(Fir::lowpass(50.0, 200.0, 80.0, fs).is_err());
        // The stopband would run past Nyquist.
        assert!(Fir::lowpass(23_900.0, 400.0, 80.0, fs).is_err());
        assert!(Fir::real_bandpass(1000.0, 30_000.0, 100.0, 80.0, fs).is_err());
    }

    #[test]
    fn tap_counts_stay_within_the_limit() {
        assert!(check_taps(MAX_TAPS).is_err());
        assert_eq!(check_taps(MAX_TAPS - 1).unwrap(), MAX_TAPS - 1);
        let fs = 2_000_000.0;
        let t = min_transition(fs, 80.0);
        let n = kaiser_taps(t / fs, 80.0 + KAISER_MARGIN_DB);
        assert!(n <= MAX_TAPS && n % 2 == 1, "{n}");
    }

    #[test]
    fn complement_and_bandpasses() {
        let fs = 8000.0;
        let bp = Fir::real_bandpass(1000.0, 1500.0, 100.0, 70.0, fs).unwrap();
        assert!(bp.response_db(1250.0, fs).abs() < 0.05);
        assert!(bp.response_db(-1250.0, fs).abs() < 0.05);
        assert!(bp.response_db(500.0, fs) < -69.0);
        let br = bp.complement();
        assert!(br.response_db(1250.0, fs) < -60.0);
        assert!(br.response_db(500.0, fs).abs() < 0.05);
        let cb = Fir::complex_bandpass(1000.0, 1500.0, 100.0, 70.0, fs).unwrap();
        assert!(cb.response_db(1250.0, fs).abs() < 0.05);
        assert!(cb.response_db(-1250.0, fs) < -69.0);
        let an = Fir::analytic(200.0, 70.0, fs).unwrap();
        assert!((an.response_db(2000.0, fs) - 20.0 * 2_f64.log10()).abs() < 0.05);
        assert!(an.response_db(-2000.0, fs) < -60.0);
    }

    #[test]
    fn apply_is_delay_compensated_and_attenuates() {
        let fs = 8000.0;
        let pass = tone(500.0, 0.5, fs, 1.0).unwrap();
        let stop = tone(3000.0, 0.5, fs, 1.0).unwrap();
        let f = Fir::lowpass(1000.0, 300.0, 80.0, fs).unwrap();
        let y = f.apply(&pass, 0, pass.len(), &|| false).unwrap();
        assert_eq!(y.len(), pass.len());
        let mut x = vec![0.0_f32; pass.len()];
        pass.read_real(0, &mut x);
        // Away from the edges the output equals the input (passband, no delay).
        for i in 1000..7000 {
            assert!((y[i].re - x[i]).abs() < 1e-3, "sample {i}");
        }
        let z = f.apply(&stop, 0, stop.len(), &|| false).unwrap();
        let peak = z[1000..7000].iter().map(|c| c.re.abs()).fold(0.0, f32::max);
        assert!(peak < 0.5 * 1e-4, "stopband leak {peak}");
        assert!(matches!(
            f.apply(&pass, 0, pass.len(), &|| true),
            Err(Error::Cancelled)
        ));
    }

    #[test]
    fn apply_on_a_sub_range_matches_the_full_run() {
        let fs = 8000.0;
        let s = tone(700.0, 0.5, fs, 1.0).unwrap();
        let f = Fir::lowpass(1200.0, 400.0, 60.0, fs).unwrap();
        let all = f.apply(&s, 0, s.len(), &|| false).unwrap();
        let part = f.apply(&s, 2000, 3000, &|| false).unwrap();
        for (a, b) in part.iter().zip(&all[2000..3000]) {
            assert!((a - b).norm() < 1e-5);
        }
    }

    #[test]
    fn rrc_is_normalised_and_rejects_bad_input() {
        let f = Fir::rrc(2400.0, 0.35, 8, 48_000.0).unwrap();
        assert!(f.response_db(0.0, 48_000.0).abs() < 1e-6);
        assert!(Fir::rrc(2400.0, 0.0, 8, 48_000.0).is_err());
        assert!(Fir::lowpass(30_000.0, 100.0, 60.0, 48_000.0).is_err());
        assert!(Fir::from_taps(vec![Complex32::new(1.0, 0.0); 4]).is_err());
    }
}
