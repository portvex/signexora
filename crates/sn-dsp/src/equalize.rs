//! ISD corrector (inter-symbol distortion): a blind adaptive equalizer (report 7.5).
//!
//! SA's algorithm was not published. This implementation is a fractionally spaced
//! (T/2) adaptive FIR equalizer on the complex baseband:
//! - **AM mode**: constant-modulus algorithm (CMA). Blind and phase-invariant, so it
//!   needs only the keying rate and an approximate carrier, like SA's AM mode that
//!   "works from the clock rate" and restores PSK-class signals of unknown order.
//! - **X^N mode**: decision-directed LMS for M-PSK with an order-M phase-locked loop,
//!   for the final clean-up once the order and carrier are known.
//!
//! The corrected baseband is moved back to the original carrier and sample rate, so
//! the result can be analysed exactly like the original recording.

use sn_core::{Complex32, SampleKind, Signal};

use crate::detect::analytic;
use crate::fir::Fir;
use crate::mix::mix_in_place;
use crate::resample::Resampler;
use crate::{Error, Result};

/// Equalizer mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum EqMode {
    /// Constant modulus (blind, any PSK order).
    Am,
    /// Decision-directed for M-PSK with an order-`n` PLL.
    Xn {
        /// PSK order.
        n: u32,
    },
}

/// Taps of the T/2-spaced equalizer.
pub const TAPS: usize = 21;

fn invalid(name: &'static str, reason: impl Into<String>) -> Error {
    Error::InvalidParameter {
        name,
        reason: reason.into(),
    }
}

/// Complex baseband of `input` at exactly two samples per symbol, after a lowpass at
/// the signal bandwidth.
pub fn baseband_2sps(
    input: &Signal,
    carrier: f64,
    baud: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<Complex32>> {
    baseband(input, carrier, baud, 2, should_stop)
}

/// Complex baseband of `input` at `sps` samples per symbol (carrier removed,
/// lowpass at the signal bandwidth).
pub fn baseband(
    input: &Signal,
    carrier: f64,
    baud: f64,
    sps: u32,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<Complex32>> {
    let fs = input.sample_rate();
    if !crate::is_positive(baud) || baud * 2.0 > fs {
        return Err(invalid(
            "baud",
            "must be above 0 and at most half the sample rate",
        ));
    }
    let mut z = analytic(input, should_stop)?;
    mix_in_place(&mut z, 0, -carrier, fs);
    let bb = Signal::complex(z, fs)?;
    let cutoff = (baud * 0.75).min(fs * 0.45);
    let lp = Fir::lowpass(cutoff, (baud * 0.25).max(1.0), 60.0, fs)?;
    let filtered = Signal::complex(lp.apply(&bb, 0, bb.len(), should_stop)?, fs)?;
    let r = Resampler::new(fs, f64::from(sps.max(1)) * baud)?;
    r.apply(&filtered, 0, filtered.len(), should_stop)
}

/// Second-order decision-directed carrier loop for the X^N mode.
///
/// The state is kept in f64 and the phase is wrapped into [0, 2*pi) after every
/// update, so it stays precise however long the recording is and however many
/// training passes run (an unwrapped f32 phase reaches 1e5 rad on long files, where
/// its resolution is coarser than the loop's own corrections).
#[derive(Debug, Default, Clone, Copy)]
struct DdPll {
    theta: f64,
    freq: f64,
}

impl DdPll {
    /// Frequency (integral) gain per symbol.
    const KI: f64 = 1e-4;
    /// Phase (proportional) gain per symbol.
    const KP: f64 = 0.02;

    fn theta(&self) -> f64 {
        self.theta
    }

    /// Advance one symbol with the measured phase error `err` (radians).
    fn update(&mut self, err: f64) {
        self.freq += Self::KI * err;
        self.theta = (self.theta + self.freq + Self::KP * err).rem_euclid(std::f64::consts::TAU);
    }
}

/// Train the equalizer on `x` (T/2 samples) for `passes` passes; returns the taps.
fn train(
    x: &[Complex32],
    mode: EqMode,
    passes: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<Complex32>> {
    let mut w = vec![Complex32::new(0.0, 0.0); TAPS];
    if let Some(c) = w.get_mut(TAPS / 2) {
        *c = Complex32::new(1.0, 0.0);
    }
    let power = x
        .iter()
        .map(rustfft::num_complex::Complex::norm_sqr)
        .sum::<f32>()
        / x.len().max(1) as f32;
    if power <= 0.0 {
        return Err(invalid("signal", "the selection is silent"));
    }
    // Normalise the input so the step sizes do not depend on the level.
    let g = 1.0 / power.sqrt();
    let xs: Vec<Complex32> = x.iter().map(|c| c * g).collect();
    let mu = 4e-3_f32;
    let mut pll = DdPll::default();
    let m = match mode {
        EqMode::Am => 1,
        EqMode::Xn { n } => n,
    };
    let mf = m as f32;
    for pass in 0..passes.max(1) {
        if should_stop() {
            return Err(Error::Cancelled);
        }
        // Symbol-rate updates: every second T/2 sample.
        for (step, k) in (TAPS..xs.len()).step_by(2).enumerate() {
            if step % 32_768 == 0 && should_stop() {
                return Err(Error::Cancelled);
            }
            let Some(win) = xs.get(k + 1 - TAPS..=k) else {
                continue;
            };
            let y: Complex32 = w.iter().zip(win.iter().rev()).map(|(a, b)| a * b).sum();
            let e = match mode {
                EqMode::Am => y * (y.norm_sqr() - 1.0),
                EqMode::Xn { .. } => {
                    let rot = Complex32::from_polar(1.0, -(pll.theta() as f32));
                    let yd = y * rot;
                    // Nearest M-PSK point on the unit circle.
                    let step = std::f32::consts::TAU / mf;
                    let a = (yd.arg() / step).round() * step;
                    let d = Complex32::from_polar(1.0, a);
                    let err_phase = (yd * d.conj()).arg();
                    pll.update(f64::from(err_phase));
                    (yd - d) * rot.conj()
                }
            };
            for (wi, xi) in w.iter_mut().zip(win.iter().rev()) {
                *wi -= e * xi.conj() * mu * if pass == 0 { 1.0 } else { 0.5 };
            }
            if !w.iter().all(|c| c.re.is_finite() && c.im.is_finite()) {
                return Err(invalid(
                    "signal",
                    "the equalizer diverged; check the baud rate and carrier, or use fewer blocks",
                ));
            }
        }
    }
    Ok(w)
}

/// Correct `input`. `blocks` (SA: 10-50) sets how much training is done: more is
/// more accurate and slower.
pub fn equalize(
    input: &Signal,
    carrier: f64,
    baud: f64,
    mode: EqMode,
    blocks: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Signal> {
    let fs = input.sample_rate();
    if !carrier.is_finite() {
        return Err(invalid(
            "carrier",
            "the carrier must be a finite number of Hz",
        ));
    }
    if let EqMode::Xn { n } = mode
        && !(2..=64).contains(&n)
    {
        return Err(invalid("n", "the PSK order must be between 2 and 64"));
    }
    let x = baseband_2sps(input, carrier, baud, should_stop)?;
    if x.len() < TAPS * 8 {
        return Err(invalid(
            "signal",
            "too short to train the equalizer; select more signal",
        ));
    }
    let passes = (blocks / 5).clamp(2, 10);
    let w = train(&x, mode, passes, should_stop)?;
    // Apply the trained taps to the whole baseband (delay-compensated).
    let taps: Vec<Complex32> = w.clone();
    let fir = Fir::from_taps(taps)?;
    let bb = Signal::complex(x, 2.0 * baud)?;
    let y = Signal::complex(fir.apply(&bb, 0, bb.len(), should_stop)?, 2.0 * baud)?;
    // Back to the original rate and carrier.
    let up = Resampler::new(2.0 * baud, fs)?;
    let mut z = up.apply(&y, 0, y.len(), should_stop)?;
    // Going down to 2 x baud and back rounds the length twice; keep the result
    // aligned sample for sample with the input.
    z.resize(input.len(), Complex32::new(0.0, 0.0));
    mix_in_place(&mut z, 0, carrier, fs);
    let out = match input.kind() {
        SampleKind::Complex => Signal::complex(z, fs)?,
        SampleKind::Real => Signal::real(z.iter().map(|c| c.re).collect::<Vec<_>>(), fs)?,
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use sn_synth::{PskParams, add_awgn_snr, psk, random_symbols};

    use super::*;

    #[test]
    fn dd_pll_phase_stays_wrapped_and_tracks_a_long_offset() {
        // 1 Hz residual at 2400 Bd over 2 M symbols (about 830 s), where an unwrapped
        // phase would be large: it must stay in [0, 2*pi) and keep tracking precisely.
        let inc = std::f64::consts::TAU * 1.0 / 2400.0;
        let mut pll = DdPll::default();
        let mut true_phase = 0.0_f64;
        let mut worst_late = 0.0_f64;
        let n = 2_000_000_usize;
        for i in 0..n {
            true_phase = (true_phase + inc).rem_euclid(std::f64::consts::TAU);
            let d = true_phase - pll.theta();
            let err = (d.sin()).atan2(d.cos());
            pll.update(err);
            assert!((0.0..std::f64::consts::TAU).contains(&pll.theta()));
            if i > n / 2 {
                worst_late = worst_late.max(err.abs());
            }
        }
        assert!(worst_late < 1e-6, "late phase error {worst_late}");
        assert!((pll.freq - inc).abs() < 1e-9, "freq {}", pll.freq);
    }

    /// Spread of symbol-centre points around the nearest of `m` PSK angles, after
    /// removing a constant rotation (dispersion in radians, RMS).
    fn psk_dispersion(x: &[Complex32], m: usize) -> f32 {
        let step = std::f32::consts::TAU / m as f32;
        // Remove the constant rotation with the M-th power average.
        let avg: Complex32 = x
            .iter()
            .map(|c| (c / c.norm().max(1e-9)).powu(m as u32))
            .sum();
        let rot = Complex32::from_polar(1.0, -avg.arg() / m as f32);
        let errs: Vec<f32> = x
            .iter()
            .map(|c| {
                let a = (c * rot).arg();
                a - (a / step).round() * step
            })
            .collect();
        (errs.iter().map(|e| e * e).sum::<f32>() / errs.len() as f32).sqrt()
    }

    /// 8-PSK at 1200 Bd on `carrier`, with an echo half a symbol later at -4 dB.
    fn multipath_8psk(fs: f64, carrier: f64) -> Signal {
        let p = PskParams {
            order: 8,
            baud: 1200.0,
            carrier,
            sample_rate: fs,
            amplitude: 0.4,
        };
        let clean = psk(&p, &random_symbols(4000, 8, 7)).unwrap();
        let mut v = vec![0.0_f32; clean.len()];
        clean.read_real(0, &mut v);
        let d = (fs / 1200.0 / 2.0) as usize;
        let echo: Vec<f32> = (0..v.len())
            .map(|i| v[i] + if i >= d { -0.63 * v[i - d] } else { 0.0 })
            .collect();
        add_awgn_snr(&Signal::real(echo, fs).unwrap(), 25.0, 3).unwrap()
    }

    /// Best-phase 8-PSK dispersion of `s` measured on its true `carrier`.
    fn dispersion(s: &Signal, carrier: f64) -> f32 {
        let bb = baseband(s, carrier, 1200.0, 8, &|| false).unwrap();
        (0..8)
            .map(|ph| {
                let pts: Vec<Complex32> = bb.iter().skip(800 + ph).step_by(8).copied().collect();
                psk_dispersion(&pts, 8)
            })
            .fold(f32::MAX, f32::min)
    }

    #[test]
    fn cma_removes_multipath_from_8psk() {
        let distorted = multipath_8psk(9600.0, 1800.0);
        let before = dispersion(&distorted, 1800.0);
        let fixed = equalize(&distorted, 1800.0, 1200.0, EqMode::Am, 30, &|| false).unwrap();
        let after_am = dispersion(&fixed, 1800.0);
        let fixed2 = equalize(&fixed, 1800.0, 1200.0, EqMode::Xn { n: 8 }, 30, &|| false).unwrap();
        let after_xn = dispersion(&fixed2, 1800.0);
        assert!(
            after_am < before * 0.7,
            "CMA: {before:.3} -> {after_am:.3} rad"
        );
        // The decision-directed clean-up must improve on CMA, not just keep it.
        assert!(
            after_xn < after_am * 0.95,
            "X^N: {after_am:.3} -> {after_xn:.3} rad"
        );
        assert_eq!(fixed.len(), distorted.len());
    }

    #[test]
    fn xn_mode_converges_with_a_carrier_error() {
        // The signal sits 3 Hz above the carrier the equalizer is given; the PLL
        // must track the rotation during training without slipping cycles.
        let distorted = multipath_8psk(9600.0, 1803.0);
        let before = dispersion(&distorted, 1803.0);
        let am = equalize(&distorted, 1800.0, 1200.0, EqMode::Am, 30, &|| false).unwrap();
        let after_am = dispersion(&am, 1803.0);
        let xn = equalize(&am, 1800.0, 1200.0, EqMode::Xn { n: 8 }, 30, &|| false).unwrap();
        let after_xn = dispersion(&xn, 1803.0);
        let xn_only = equalize(&distorted, 1800.0, 1200.0, EqMode::Xn { n: 8 }, 30, &|| {
            false
        })
        .unwrap();
        let after_xn_only = dispersion(&xn_only, 1803.0);
        assert!(after_am < before * 0.7, "CMA: {before:.3} -> {after_am:.3}");
        assert!(
            after_xn < after_am * 0.95,
            "X^N after CMA: {after_am:.3} -> {after_xn:.3} rad"
        );
        assert!(
            after_xn_only < before * 0.7,
            "X^N alone: {before:.3} -> {after_xn_only:.3} rad"
        );
    }

    #[test]
    fn bad_parameters_are_errors() {
        let s = sn_synth::tone(1000.0, 0.5, 8000.0, 0.01).unwrap();
        assert!(equalize(&s, 1000.0, 0.0, EqMode::Am, 10, &|| false).is_err());
        assert!(equalize(&s, 1000.0, 1000.0, EqMode::Am, 10, &|| false).is_err());
        let long = sn_synth::tone(1000.0, 0.5, 8000.0, 1.0).unwrap();
        for n in [0, 1, 65] {
            let e = equalize(&long, 1000.0, 600.0, EqMode::Xn { n }, 10, &|| false);
            assert!(
                matches!(e, Err(Error::InvalidParameter { name: "n", .. })),
                "X^{n}"
            );
        }
        let e = equalize(&long, f64::NAN, 600.0, EqMode::Am, 10, &|| false);
        assert!(matches!(
            e,
            Err(Error::InvalidParameter {
                name: "carrier",
                ..
            })
        ));
    }

    #[test]
    fn output_length_matches_for_non_integer_rate_ratios() {
        // fs / (2 baud) = 3.33...: the round trip through 2400 Hz rounds twice.
        let fs = 8000.0;
        let x: Vec<f32> = (0..8001)
            .map(|i| (0.4 * (std::f64::consts::TAU * 1800.0 * f64::from(i) / fs).cos()) as f32)
            .collect();
        let s = Signal::real(x, fs).unwrap();
        let out = equalize(&s, 1800.0, 1200.0, EqMode::Am, 10, &|| false).unwrap();
        assert_eq!(out.len(), s.len());
    }
}
