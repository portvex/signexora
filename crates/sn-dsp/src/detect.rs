//! Detectors (report 7.2-7.3, 7.8): signals whose spectrum shows the keying rate or
//! the carrier as a line, plus correlation and FSK resynthesis.

use rayon::prelude::*;
use rustfft::FftPlanner;
use rustfft::num_complex::Complex32 as FftComplex;
use sn_core::{Complex32, SampleKind, Signal};

use crate::fir::Fir;
use crate::resample::Resampler;
use crate::{Error, Result};

/// Largest selection accepted for an ACF, in samples.
pub const MAX_ACF_SAMPLES: usize = 1 << 24;
/// Largest CCF template (fragment), in samples.
pub const MAX_CCF_TEMPLATE: usize = 1 << 20;

fn invalid(name: &'static str, reason: impl Into<String>) -> Error {
    Error::InvalidParameter {
        name,
        reason: reason.into(),
    }
}

fn guard(n: usize, limit: usize, what: &str) -> Result<()> {
    if n > limit {
        return Err(invalid(
            "size",
            format!("{what} needs {n} samples; the limit is {limit}. Select a shorter span"),
        ));
    }
    Ok(())
}

/// The signal as complex samples: I/Q as is, real signals through an analytic filter
/// (positive frequencies, gain 2, so the magnitude is the true envelope).
pub fn analytic(input: &Signal, should_stop: &(dyn Fn() -> bool + Sync)) -> Result<Vec<Complex32>> {
    guard(input.len(), crate::ops::MAX_OUTPUT_SAMPLES, "the signal")?;
    match input.kind() {
        SampleKind::Complex => {
            let mut v = vec![Complex32::new(0.0, 0.0); input.len()];
            input.read_complex(0, &mut v);
            Ok(v)
        }
        SampleKind::Real => {
            let fs = input.sample_rate();
            Fir::analytic((fs / 500.0).max(1.0), 70.0, fs)?.apply(
                input,
                0,
                input.len(),
                should_stop,
            )
        }
    }
}

fn remove_mean(v: &mut [f32]) {
    if v.is_empty() {
        return;
    }
    let mean = v.iter().map(|x| f64::from(*x)).sum::<f64>() / v.len() as f64;
    for x in v {
        *x -= mean as f32;
    }
}

/// Quadrature AM detector: the envelope `|z|` with its mean removed. Its spectrum
/// shows the keying rate of PSK/QAM/FSK as a line.
pub fn envelope(input: &Signal, should_stop: &(dyn Fn() -> bool + Sync)) -> Result<Vec<f32>> {
    let z = analytic(input, should_stop)?;
    let mut e: Vec<f32> = z.iter().map(|c| c.norm()).collect();
    remove_mean(&mut e);
    Ok(e)
}

/// Double half-wave (full-wave) rectifier: `|x|` with the mean removed. Fast; suits
/// recordings already passed through a discriminator.
pub fn half_wave(input: &Signal) -> Result<Vec<f32>> {
    guard(input.len(), crate::ops::MAX_OUTPUT_SAMPLES, "the signal")?;
    let mut v = vec![0.0_f32; input.len()];
    input.read_real(0, &mut v);
    for x in &mut v {
        *x = x.abs();
    }
    remove_mean(&mut v);
    Ok(v)
}

/// Instantaneous frequency in Hz from adjacent-sample phase differences (SA's phase
/// detector). The first sample repeats the second.
pub fn inst_freq(z: &[Complex32], sample_rate: f64) -> Vec<f32> {
    let k = sample_rate / std::f64::consts::TAU;
    let diff = |a: &Complex32, b: &Complex32| {
        let d = b * a.conj();
        (f64::from(d.im).atan2(f64::from(d.re)) * k) as f32
    };
    let mut out = Vec::with_capacity(z.len());
    if let [a, b, ..] = z {
        out.push(diff(a, b));
    } else if !z.is_empty() {
        out.push(0.0);
    }
    // Independent per sample: computed in parallel, same values as a serial pass.
    out.par_extend(z.par_windows(2).map(|w| match w {
        [a, b] => diff(a, b),
        _ => 0.0,
    }));
    out
}

fn moving_average(v: &[f32], w: usize) -> Vec<f32> {
    let w = w.max(1);
    let half = w / 2;
    let mut prefix = Vec::with_capacity(v.len() + 1);
    prefix.push(0.0_f64);
    let mut acc = 0.0_f64;
    for x in v {
        acc += f64::from(*x);
        prefix.push(acc);
    }
    let n = v.len();
    (0..n)
        .map(|i| {
            let a = i.saturating_sub(half);
            let b = (i + half + 1).min(n);
            let s = prefix.get(b).copied().unwrap_or(0.0) - prefix.get(a).copied().unwrap_or(0.0);
            (s / (b - a).max(1) as f64) as f32
        })
        .collect()
}

/// Modified AM detector for MFSK clocks.
///
/// SA's algorithm was never published. This implementation measures how much the
/// instantaneous frequency changes over a short delay: every symbol transition gives
/// a pulse, so the spectrum shows the keying rate even when the envelope is constant
/// (MFSK) or carries unrelated amplitude modulation (mains hum, fading).
pub fn modified_am(input: &Signal, should_stop: &(dyn Fn() -> bool + Sync)) -> Result<Vec<f32>> {
    let fs = input.sample_rate();
    let z = analytic(input, should_stop)?;
    let f = inst_freq(&z, fs);
    // Smooth over ~1 ms so noise in the phase difference does not dominate.
    let w = ((fs / 1000.0).round() as usize).max(3) | 1;
    let s = moving_average(&f, w);
    let mut out: Vec<f32> = (0..s.len())
        .map(|i| {
            let a = s.get(i).copied().unwrap_or(0.0);
            let b = s.get(i.saturating_sub(w)).copied().unwrap_or(a);
            (a - b).abs()
        })
        .collect();
    remove_mean(&mut out);
    Ok(out)
}

/// Raise the analytic signal to the power `n` (SA's harmonics module). The signal is
/// first upsampled `n` times so the `n`-fold frequencies cannot alias; the output is
/// complex at `n` times the input rate. Magnitudes are kept (`z^n / |z|^(n-1)`).
pub fn power(input: &Signal, n: u32, should_stop: &(dyn Fn() -> bool + Sync)) -> Result<Signal> {
    if !(1..=64).contains(&n) {
        return Err(invalid("n", "the power must be between 1 and 64"));
    }
    let fs = input.sample_rate();
    let z = analytic(input, should_stop)?;
    let base = Signal::complex(z, fs)?;
    let up_rate = fs * f64::from(n);
    let up = if n > 1 {
        let r = Resampler::new(fs, up_rate)?;
        guard(
            r.output_len(base.len()),
            crate::ops::MAX_OUTPUT_SAMPLES,
            "the upsampled signal",
        )?;
        r.apply(&base, 0, base.len(), should_stop)?
    } else {
        let mut v = vec![Complex32::new(0.0, 0.0); base.len()];
        base.read_complex(0, &mut v);
        v
    };
    // Polar form: |z| * e^{j n arg z}. Computing z^n / |z|^(n-1) directly in f32
    // underflows to 0/0 = NaN for small samples.
    let nf = f64::from(n);
    let out: Vec<Complex32> = up
        .iter()
        .map(|c| {
            let m = f64::from(c.norm());
            let ph = f64::from(c.im).atan2(f64::from(c.re)) * nf;
            Complex32::new((m * ph.cos()) as f32, (m * ph.sin()) as f32)
        })
        .collect();
    Ok(Signal::complex(out, up_rate)?)
}

/// Autocorrelation of samples `start..end`, normalised so lag 0 is 1. Output sample
/// `k` is lag `k` (same sample rate, so the time axis reads in lag seconds).
pub fn acf(
    input: &Signal,
    start: usize,
    end: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<f32>> {
    let end = end.min(input.len());
    let len = end.saturating_sub(start);
    if len < 2 {
        return Err(invalid(
            "range",
            "select at least two samples with the V markers",
        ));
    }
    guard(len, MAX_ACF_SAMPLES, "the ACF")?;
    let m = (2 * len).next_power_of_two();
    let mut x = vec![Complex32::new(0.0, 0.0); len];
    input.read_complex(start, &mut x);
    // Remove the mean so a DC offset does not dominate (summed in f64: an f32 sum
    // over millions of samples leaves a DC residue that biases every lag).
    let (re, im) = x.iter().fold((0.0_f64, 0.0_f64), |(r, i), c| {
        (r + f64::from(c.re), i + f64::from(c.im))
    });
    let mean = Complex32::new((re / len as f64) as f32, (im / len as f64) as f32);
    let mut buf: Vec<FftComplex> = vec![FftComplex::new(0.0, 0.0); m];
    for (b, v) in buf.iter_mut().zip(&x) {
        let d = v - mean;
        *b = FftComplex::new(d.re, d.im);
    }
    if should_stop() {
        return Err(Error::Cancelled);
    }
    let mut planner = FftPlanner::<f32>::new();
    planner.plan_fft_forward(m).process(&mut buf);
    for b in &mut buf {
        *b = FftComplex::new(b.norm_sqr(), 0.0);
    }
    planner.plan_fft_inverse(m).process(&mut buf);
    let r0 = buf.first().map_or(0.0, |c| c.re);
    if r0 <= 0.0 {
        return Err(invalid(
            "signal",
            "the selection is silent or constant (no variation once its mean is \
             removed); select a span that contains signal",
        ));
    }
    let real = input.kind() == SampleKind::Real;
    Ok(buf
        .iter()
        .take(len / 2)
        .map(|c| if real { c.re / r0 } else { c.norm() / r0 })
        .collect())
}

/// Normalised cross-correlation of the fragment `start..end` against the whole
/// signal (0..=1). Output sample `n` is the match of the fragment *centred* at `n`;
/// the original position scores 1.
pub fn ccf(
    input: &Signal,
    start: usize,
    end: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<f32>> {
    let end = end.min(input.len());
    let mut len = end.saturating_sub(start);
    if len < 3 {
        return Err(invalid(
            "range",
            "select a fragment of at least three samples with the V markers",
        ));
    }
    guard(len, MAX_CCF_TEMPLATE, "the CCF fragment")?;
    guard(input.len(), crate::ops::MAX_OUTPUT_SAMPLES, "the CCF")?;
    // Odd template for exact centring: drop the last sample rather than reading one
    // past the selection.
    if len.is_multiple_of(2) {
        len -= 1;
    }
    let mut t = vec![Complex32::new(0.0, 0.0); len];
    input.read_complex(start, &mut t);
    let energy_t: f64 = t.iter().map(|c| f64::from(c.norm_sqr())).sum();
    if energy_t <= 0.0 {
        return Err(invalid("signal", "the fragment is silent"));
    }
    let taps: Vec<Complex32> = t
        .iter()
        .rev()
        .map(rustfft::num_complex::Complex::conj)
        .collect();
    let y = Fir::from_taps(taps)?.apply(input, 0, input.len(), should_stop)?;
    // Local energy of the input under the template, from prefix sums.
    let n = input.len();
    let mut prefix = Vec::with_capacity(n + 1);
    prefix.push(0.0_f64);
    let mut acc = 0.0_f64;
    let mut buf = vec![Complex32::new(0.0, 0.0); 1 << 16];
    for s in (0..n).step_by(buf.len()) {
        let got = input.read_complex(s, &mut buf);
        for c in buf.iter().take(got) {
            acc += f64::from(c.norm_sqr());
            prefix.push(acc);
        }
    }
    let half = len / 2;
    Ok(y.iter()
        .enumerate()
        .map(|(i, c)| {
            let a = i.saturating_sub(half);
            let b = (i + half + 1).min(n);
            let e = prefix.get(b).copied().unwrap_or(0.0) - prefix.get(a).copied().unwrap_or(0.0);
            let d = (energy_t * e).sqrt();
            if d > 0.0 {
                (f64::from(c.norm()) / d).min(1.0) as f32
            } else {
                0.0
            }
        })
        .collect())
}

/// Per-sample tone decision for FSK/MFSK: instantaneous frequency smoothed over a
/// third of a symbol, snapped to the nearest of `tones`.
pub fn tone_decisions(
    input: &Signal,
    tones: &[f64],
    baud: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<f64>> {
    if tones.len() < 2 {
        return Err(invalid("tones", "need at least two tone frequencies"));
    }
    if !crate::is_positive(baud) {
        return Err(invalid("baud", "must be above 0"));
    }
    let fs = input.sample_rate();
    let z = analytic(input, should_stop)?;
    let f = inst_freq(&z, fs);
    let w = ((fs / baud / 3.0).round() as usize).max(1);
    Ok(moving_average(&f, w)
        .iter()
        .map(|v| {
            let fv = f64::from(*v);
            tones
                .iter()
                .copied()
                .min_by(|a, b| (a - fv).abs().total_cmp(&(b - fv).abs()))
                .unwrap_or(fv)
        })
        .collect())
}

/// Regenerate a clean continuous-phase FSK/MFSK signal on the given tones (SA's
/// inverse synthesis), removing in-band and out-of-band noise.
pub fn resynthesize(
    input: &Signal,
    tones: &[f64],
    baud: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<f32>> {
    let fs = input.sample_rate();
    let tau = std::f64::consts::TAU;
    let mut phase = 0.0_f64;
    Ok(tone_decisions(input, tones, baud, should_stop)?
        .iter()
        .map(|tone| {
            let out = (0.5 * phase.sin()) as f32;
            phase = (phase + tau * tone / fs).rem_euclid(tau);
            out
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use sn_synth::{MfskParams, PskParams, add_awgn_snr, mfsk, psk, random_symbols, tone};

    use super::*;
    use crate::spectrum::{Combine, SpectrumEngine};
    use crate::window::WindowKind;

    /// Prominence (dB above the median) of the strongest line near `hz` in `x`.
    fn line_prominence(x: &[f32], fs: f64, hz: f64) -> f32 {
        let s = Signal::real(x.to_vec(), fs).unwrap();
        let e = SpectrumEngine::new(8192, WindowKind::default(), SampleKind::Real).unwrap();
        let frames: Vec<usize> = (0..s.len().saturating_sub(8192)).step_by(4096).collect();
        let db = e.column_db(&s, &frames, Combine::Mean, &|| false).unwrap();
        let k = e.frequency_to_bin(hz, fs).round() as usize;
        let peak = db[k - 2..=k + 2].iter().copied().fold(f32::MIN, f32::max);
        let mut sorted = db.clone();
        sorted.sort_by(f32::total_cmp);
        peak - sorted[sorted.len() / 2]
    }

    #[test]
    fn envelope_shows_the_psk_clock() {
        let fs = 9600.0;
        let p = PskParams {
            order: 2,
            baud: 600.0,
            carrier: 1800.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let s = psk(&p, &random_symbols(6000, 2, 3)).unwrap();
        // Rectangular PSK has envelope dips at phase flips: a clean 600 Hz line.
        let e = envelope(&s, &|| false).unwrap();
        assert!(line_prominence(&e, fs, 600.0) > 20.0);
    }

    #[test]
    fn modified_am_finds_the_mfsk_clock_where_the_envelope_cannot() {
        let fs = 8000.0;
        let p = MfskParams {
            order: 8,
            baud: 50.0,
            spacing: 100.0,
            center: 1500.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let s = mfsk(&p, &random_symbols(500, 8, 9)).unwrap();
        let s = add_awgn_snr(&s, 15.0, 4).unwrap();
        let m = modified_am(&s, &|| false).unwrap();
        let e = envelope(&s, &|| false).unwrap();
        let pm = line_prominence(&m, fs, 50.0);
        let pe = line_prominence(&e, fs, 50.0);
        assert!(pm > 15.0, "modified detector line {pm} dB");
        assert!(pm > pe + 10.0, "modified {pm} dB vs envelope {pe} dB");
    }

    #[test]
    fn phase_detector_reads_tone_frequencies_in_hz() {
        let s = tone(1234.5, 0.5, 8000.0, 0.5).unwrap();
        let z = analytic(&s, &|| false).unwrap();
        let f = inst_freq(&z, 8000.0);
        let mid = &f[1000..3000];
        let mean = mid.iter().map(|v| f64::from(*v)).sum::<f64>() / mid.len() as f64;
        assert!((mean - 1234.5).abs() < 0.05, "{mean}");
        assert_eq!(f.len(), z.len());
        assert_eq!(inst_freq(&[], 8000.0), Vec::<f32>::new());
        assert_eq!(inst_freq(&[Complex32::new(1.0, 0.0)], 8000.0), vec![0.0]);
    }

    #[test]
    fn acf_mean_removal_is_accurate_on_long_selections() {
        // DC 0.7 plus a weak 0.1 sine: after removing the mean, lag 0 holds only the
        // sine's power, and a quarter period later the ACF is close to 0.
        let n = 1 << 22;
        let x: Vec<f32> = (0..n)
            .map(|i| 0.7 + 0.1 * (std::f64::consts::TAU * i as f64 / 64.0).sin() as f32)
            .collect();
        let s = Signal::real(x, 1000.0).unwrap();
        let a = acf(&s, 0, n, &|| false).unwrap();
        assert!(a[16].abs() < 1e-3, "quarter-period lag {}", a[16]);
        assert!((a[32] + 1.0).abs() < 1e-2, "half-period lag {}", a[32]);
        let dc = Signal::real(vec![0.5_f32; 100], 1000.0).unwrap();
        assert!(acf(&dc, 0, 100, &|| false).is_err());
    }

    #[test]
    fn power_of_four_collapses_qpsk_to_a_carrier_line() {
        let fs = 8000.0;
        let p = PskParams {
            order: 4,
            baud: 500.0,
            carrier: 1300.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let s = psk(&p, &random_symbols(4000, 4, 5)).unwrap();
        let p4 = power(&s, 4, &|| false).unwrap();
        assert!((p4.sample_rate() - 32_000.0).abs() < 1e-9);
        let e = SpectrumEngine::new(8192, WindowKind::default(), SampleKind::Complex).unwrap();
        let frames: Vec<usize> = (0..p4.len() - 8192).step_by(4096).collect();
        let db = e.column_db(&p4, &frames, Combine::Mean, &|| false).unwrap();
        let k = db
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert!(
            (e.bin_frequency(k, 32_000.0) - 5200.0).abs() < 5.0,
            "4 x 1300 Hz"
        );
        assert!(power(&s, 0, &|| false).is_err());
        // Near-silent samples must not turn into NaN (z^8 underflows in f32).
        let tiny = Signal::complex(vec![Complex32::new(1e-8, 1e-8); 64], fs).unwrap();
        let p8 = power(&tiny, 8, &|| false).unwrap();
        let mut v = vec![Complex32::new(0.0, 0.0); p8.len()];
        p8.read_complex(0, &mut v);
        assert!(v.iter().all(|c| c.re.is_finite() && c.im.is_finite()));
    }

    #[test]
    fn acf_and_ccf_find_repetitions() {
        // A random 200-sample burst repeated every 1000 samples.
        let burst: Vec<f32> = random_symbols(200, 2, 1)
            .iter()
            .map(|b| if *b == 0 { -1.0 } else { 1.0 })
            .collect();
        let mut x = vec![0.0_f32; 10_000];
        for k in 0..10 {
            for (i, v) in burst.iter().enumerate() {
                x[k * 1000 + 100 + i] = *v;
            }
        }
        let s = Signal::real(x, 1000.0).unwrap();
        let a = acf(&s, 0, 10_000, &|| false).unwrap();
        assert!((a[0] - 1.0).abs() < 1e-5);
        let best = (500..1500).max_by(|i, j| a[*i].total_cmp(&a[*j])).unwrap();
        assert_eq!(best, 1000);
        // An even 200-sample selection becomes the odd 199-sample template
        // 3100..3299, centred at 3199; nothing past the selection is read.
        let c = ccf(&s, 3100, 3300, &|| false).unwrap();
        assert!((c[3199] - 1.0).abs() < 1e-3, "{}", c[3199]);
        assert!(c[7199] > 0.99, "repeat found");
        assert!(ccf(&s, 3100, 3102, &|| false).is_err());
        assert!(c[7700] < 0.5);
        assert!(acf(&s, 5, 5, &|| false).is_err());
    }

    #[test]
    fn resynthesis_snaps_to_the_tones() {
        let fs = 8000.0;
        let p = MfskParams {
            order: 2,
            baud: 100.0,
            spacing: 400.0,
            center: 1500.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let symbols = random_symbols(200, 2, 2);
        let clean = mfsk(&p, &symbols).unwrap();
        let noisy = add_awgn_snr(&clean, 6.0, 8).unwrap();
        let tones = [1300.0, 1700.0];
        let d = tone_decisions(&noisy, &tones, 100.0, &|| false).unwrap();
        // Decision at each symbol centre (80 samples per symbol).
        let right = symbols
            .iter()
            .enumerate()
            .filter(|(k, s)| (d[k * 80 + 40] - tones[**s]).abs() < 1e-9)
            .count();
        assert!(right >= 198, "{right} of 200 symbols");
        let y = resynthesize(&noisy, &tones, 100.0, &|| false).unwrap();
        assert_eq!(y.len(), noisy.len());
        assert!(y.iter().all(|v| v.abs() <= 0.5 + 1e-6));
        assert!(tone_decisions(&noisy, &[1.0], 100.0, &|| false).is_err());
    }
}
