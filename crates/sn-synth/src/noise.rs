use sn_core::{Complex32, SampleKind, Signal};

use crate::{MAX_SAMPLES, Result, Rng, require, require_positive};

const CHUNK: usize = 1 << 16;

/// Mean power of a signal (`|x|^2` averaged). Works on lazy signals in chunks.
pub fn signal_power(signal: &Signal) -> f64 {
    let n = signal.len();
    if n == 0 {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    match signal.kind() {
        SampleKind::Real => {
            let mut buf = vec![0.0_f32; CHUNK];
            for start in (0..n).step_by(CHUNK) {
                let got = signal.read_real(start, &mut buf);
                sum += buf
                    .iter()
                    .take(got)
                    .map(|v| f64::from(*v).powi(2))
                    .sum::<f64>();
            }
        }
        SampleKind::Complex => {
            let mut buf = vec![Complex32::new(0.0, 0.0); CHUNK];
            for start in (0..n).step_by(CHUNK) {
                let got = signal.read_complex(start, &mut buf);
                sum += buf
                    .iter()
                    .take(got)
                    .map(|v| f64::from(v.norm_sqr()))
                    .sum::<f64>();
            }
        }
    }
    sum / n as f64
}

/// Read the whole of `signal` into memory in chunks, checking that every sample arrived.
/// `purpose` names what needs the samples in memory, for the error message (for
/// example "adding noise").
pub(crate) fn read_all<T: Copy>(
    signal: &Signal,
    zero: T,
    read: impl Fn(&Signal, usize, &mut [T]) -> usize,
    purpose: &str,
) -> Result<Vec<T>> {
    let n = signal.len();
    require(
        !signal.is_lazy() || n <= MAX_SAMPLES,
        "signal",
        format!(
            "is {n} samples long; {purpose} needs it in memory, which is limited to \
             2^28 samples; select a shorter part first"
        ),
    )?;
    let mut v = vec![zero; n];
    for (k, chunk) in v.chunks_mut(CHUNK).enumerate() {
        let start = k * CHUNK;
        let got = read(signal, start, chunk);
        require(
            got == chunk.len(),
            "signal",
            format!(
                "could only read {} of {n} samples; the file may be truncated or unreadable",
                start + got
            ),
        )?;
    }
    Ok(v)
}

fn check_sigma(sigma: f64) -> Result<()> {
    require(
        sigma.is_finite() && sigma >= 0.0,
        "sigma",
        format!("must be a finite number of at least 0 (got {sigma})"),
    )
}

/// Add white Gaussian noise of standard deviation `sigma` to `v` in place, drawn from
/// `seed`. The same noise sequence as [`add_awgn`] on a real signal.
pub(crate) fn awgn_real_in_place(v: &mut [f32], sigma: f64, seed: u64) -> Result<()> {
    check_sigma(sigma)?;
    let mut rng = Rng::new(seed);
    for s in v {
        *s += (sigma * rng.normal()) as f32;
    }
    Ok(())
}

/// Add complex white Gaussian noise of standard deviation `sigma` per component to `v`
/// in place, drawn from `seed`. The same noise sequence as [`add_awgn`] on a complex
/// signal.
pub(crate) fn awgn_complex_in_place(v: &mut [Complex32], sigma: f64, seed: u64) -> Result<()> {
    check_sigma(sigma)?;
    let mut rng = Rng::new(seed);
    for s in v {
        let re = (sigma * rng.normal()) as f32;
        let im = (sigma * rng.normal()) as f32;
        *s += Complex32::new(re, im);
    }
    Ok(())
}

/// Add white Gaussian noise with standard deviation `sigma` (per component for complex).
///
/// The result is held in memory. A lazy (file-backed) signal is read in chunks and must
/// be at most [`MAX_SAMPLES`] long; a short read is reported as an error rather than
/// leaving silence in the output.
pub fn add_awgn(signal: &Signal, sigma: f64, seed: u64) -> Result<Signal> {
    check_sigma(sigma)?;
    let out = match signal.kind() {
        SampleKind::Real => {
            let mut v = read_all(signal, 0.0_f32, Signal::read_real, "adding noise")?;
            awgn_real_in_place(&mut v, sigma, seed)?;
            Signal::real(v, signal.sample_rate())?
        }
        SampleKind::Complex => {
            let mut v = read_all(
                signal,
                Complex32::new(0.0, 0.0),
                Signal::read_complex,
                "adding noise",
            )?;
            awgn_complex_in_place(&mut v, sigma, seed)?;
            Signal::complex(v, signal.sample_rate())?
        }
    };
    Ok(out.with_center_frequency(signal.center_frequency()))
}

/// Mean power of `signal`, required to be finite and above 0 so a noise level can be
/// derived from it.
fn usable_power(signal: &Signal) -> Result<f64> {
    let power = signal_power(signal);
    require(
        power.is_finite(),
        "signal",
        "contains infinite or NaN samples, so an SNR cannot be applied",
    )?;
    require(
        power > 0.0,
        "signal",
        "is silent, so an SNR cannot be applied",
    )?;
    Ok(power)
}

/// Add white Gaussian noise for a given signal-to-noise ratio over the full bandwidth.
///
/// Convention: the total noise power over the whole Nyquist band equals the signal
/// power divided by the SNR. For a real signal that band is 0 to fs/2, for a complex
/// one -fs/2 to fs/2, so at the same `snr_db` a real signal has twice the noise
/// density, which is 3 dB less Es/N0 than its complex equivalent. The in-band SNR
/// also depends on the sample rate: Es/N0 = SNR * fs / Rs for complex signals and
/// SNR * fs / (2 Rs) for real ones. Use [`add_awgn_esn0`] to state Es/N0 directly,
/// independently of the sample rate and sample kind.
pub fn add_awgn_snr(signal: &Signal, snr_db: f64, seed: u64) -> Result<Signal> {
    require(
        snr_db.is_finite(),
        "snr_db",
        format!("must be a finite number of dB (got {snr_db})"),
    )?;
    let power = usable_power(signal)?;
    let noise_power = power / 10_f64.powf(snr_db / 10.0);
    let sigma = match signal.kind() {
        SampleKind::Real => noise_power.sqrt(),
        SampleKind::Complex => (noise_power / 2.0).sqrt(),
    };
    add_awgn(signal, sigma, seed)
}

/// Add white Gaussian noise for a given Es/N0 (energy per symbol over noise density).
///
/// With signal power P and symbol rate Rs, Es = P / Rs and
/// `N0 = P / (Rs * 10^(esn0_db / 10))`. The noise variance is N0 * fs / 2 for a real
/// signal (one-sided density over 0 to fs/2) and N0 * fs / 2 per component for a
/// complex one (density N0 over -fs/2 to fs/2), so the same `esn0_db` means the same
/// detection difficulty whatever the sample rate or sample kind. For Eb/N0 pass
/// `ebn0_db + 10 log10(bits per symbol)`.
pub fn add_awgn_esn0(signal: &Signal, esn0_db: f64, symbol_rate: f64, seed: u64) -> Result<Signal> {
    require(
        esn0_db.is_finite(),
        "esn0_db",
        format!("must be a finite number of dB (got {esn0_db})"),
    )?;
    require_positive(symbol_rate, "symbol_rate", "Bd")?;
    let power = usable_power(signal)?;
    let n0 = power / (symbol_rate * 10_f64.powf(esn0_db / 10.0));
    let sigma = (n0 * signal.sample_rate() / 2.0).sqrt();
    add_awgn(signal, sigma, seed)
}

#[cfg(test)]
mod tests {
    use sn_core::Samples;

    use super::*;
    use crate::tone;

    #[test]
    fn snr_is_respected() {
        let clean = tone(1000.0, 0.5, 48_000.0, 2.0).unwrap();
        let noisy = add_awgn_snr(&clean, 10.0, 42).unwrap();
        let (Samples::Real(a), Samples::Real(b)) = (clean.samples(), noisy.samples()) else {
            panic!("expected real")
        };
        let noise_power = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| f64::from(y - x).powi(2))
            .sum::<f64>()
            / a.len() as f64;
        let snr = 10.0 * (signal_power(&clean) / noise_power).log10();
        assert!((snr - 10.0).abs() < 0.1, "measured SNR {snr} dB");
    }

    #[test]
    fn same_seed_same_noise() {
        let clean = tone(1000.0, 0.5, 8000.0, 0.1).unwrap();
        let a = add_awgn(&clean, 0.1, 9).unwrap();
        let b = add_awgn(&clean, 0.1, 9).unwrap();
        let (Samples::Real(a), Samples::Real(b)) = (a.samples(), b.samples()) else {
            panic!("expected real")
        };
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_non_finite_snr_and_sigma() {
        let clean = tone(1000.0, 0.5, 8000.0, 0.1).unwrap();
        for snr in [f64::NAN, f64::NEG_INFINITY, f64::INFINITY] {
            let err = add_awgn_snr(&clean, snr, 1).unwrap_err().to_string();
            assert!(err.contains("snr_db"), "{err}");
        }
        assert!(add_awgn(&clean, f64::INFINITY, 1).is_err());
        assert!(add_awgn(&clean, f64::NAN, 1).is_err());
        assert!(add_awgn(&clean, -1.0, 1).is_err());
        let silent = Signal::real(vec![0.0_f32; 100], 8000.0).unwrap();
        assert!(add_awgn_snr(&silent, 10.0, 1).is_err());
        let broken = Signal::real(vec![f32::INFINITY; 100], 8000.0).unwrap();
        assert!(add_awgn_snr(&broken, 10.0, 1).is_err());
    }

    #[test]
    fn esn0_sets_noise_density_independently_of_kind() {
        // Real tone: P = 0.125, Rs = 100 Bd, Es/N0 = 10 dB gives N0 = 1.25e-4 and
        // sigma^2 = N0 * fs / 2 = 0.5 at 8 kHz.
        let clean = tone(1000.0, 0.5, 8000.0, 2.0).unwrap();
        let noisy = add_awgn_esn0(&clean, 10.0, 100.0, 4).unwrap();
        let (Samples::Real(a), Samples::Real(b)) = (clean.samples(), noisy.samples()) else {
            panic!("expected real")
        };
        let var = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| f64::from(y - x).powi(2))
            .sum::<f64>()
            / a.len() as f64;
        assert!((var - 0.5).abs() < 0.02, "variance {var}");
        // Complex signal of the same power: the same variance per component.
        let c =
            Signal::complex(vec![Complex32::new(0.125_f32.sqrt(), 0.0); 16_000], 8000.0).unwrap();
        let nc = add_awgn_esn0(&c, 10.0, 100.0, 4).unwrap();
        let Samples::Complex(z) = nc.samples() else {
            panic!("expected complex")
        };
        let var_im = z.iter().map(|v| f64::from(v.im).powi(2)).sum::<f64>() / z.len() as f64;
        assert!((var_im - 0.5).abs() < 0.02, "variance {var_im}");
        assert!(add_awgn_esn0(&clean, f64::NAN, 100.0, 1).is_err());
        assert!(add_awgn_esn0(&clean, 10.0, 0.0, 1).is_err());
    }

    #[test]
    fn short_lazy_read_is_an_error() {
        use std::sync::Arc;

        #[derive(Debug)]
        struct Truncated;
        impl sn_core::SampleSource for Truncated {
            fn len(&self) -> usize {
                1000
            }
            fn kind(&self) -> SampleKind {
                SampleKind::Real
            }
            fn read_complex(&self, start: usize, out: &mut [Complex32]) -> usize {
                let n = out.len().min(500_usize.saturating_sub(start));
                out.iter_mut()
                    .take(n)
                    .for_each(|v| *v = Complex32::new(1.0, 0.0));
                n
            }
            fn read_real(&self, start: usize, out: &mut [f32]) -> usize {
                let n = out.len().min(500_usize.saturating_sub(start));
                out.iter_mut().take(n).for_each(|v| *v = 1.0);
                n
            }
        }
        let s = Signal::lazy(Arc::new(Truncated), 8000.0).unwrap();
        let err = add_awgn(&s, 0.1, 1).unwrap_err().to_string();
        assert!(err.contains("could only read 500 of 1000"), "{err}");
    }
}
