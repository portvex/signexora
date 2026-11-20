use sn_core::Signal;

use crate::{Result, check_amplitude, check_sample_rate, output_len, require, require_positive};

/// A real sine tone `amplitude * sin(2 pi f t)`.
///
/// The length is limited to [`crate::MAX_SAMPLES`] samples.
pub fn tone(frequency: f64, amplitude: f64, sample_rate: f64, duration: f64) -> Result<Signal> {
    check_sample_rate(sample_rate)?;
    require_positive(duration, "duration", "s")?;
    check_amplitude(amplitude)?;
    require(
        frequency.abs() < sample_rate / 2.0,
        "frequency",
        format!(
            "{frequency} Hz is not below the Nyquist frequency {} Hz",
            sample_rate / 2.0
        ),
    )?;
    let n = output_len(duration * sample_rate, "duration")?;
    let w = std::f64::consts::TAU * frequency / sample_rate;
    let samples: Vec<f32> = (0..n)
        .map(|i| (amplitude * (w * i as f64).sin()) as f32)
        .collect();
    Ok(Signal::real(samples, sample_rate)?)
}

#[cfg(test)]
mod tests {
    use sn_core::Samples;

    use super::*;

    #[test]
    fn zero_crossings_match_frequency() {
        let s = tone(1000.0, 0.5, 48_000.0, 1.0).unwrap();
        let Samples::Real(x) = s.samples() else {
            panic!("expected real")
        };
        let crossings = x
            .windows(2)
            .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
            .count();
        assert!((1999..=2001).contains(&crossings), "{crossings} crossings");
        assert!(x.iter().all(|v| v.abs() <= 0.5 + 1e-6));
    }

    #[test]
    fn rejects_frequency_above_nyquist() {
        assert!(tone(5000.0, 1.0, 8000.0, 1.0).is_err());
    }

    #[test]
    fn rejects_huge_or_non_finite_lengths_and_values() {
        assert!(tone(1000.0, 0.5, f64::INFINITY, 1.0).is_err());
        assert!(tone(1000.0, 0.5, 1e300, 1.0).is_err());
        assert!(tone(1000.0, 0.5, 8000.0, f64::INFINITY).is_err());
        assert!(tone(1000.0, 0.5, 8000.0, 1e12).is_err());
        assert!(tone(1000.0, 0.5, -8000.0, 1.0).is_err());
        assert!(tone(1000.0, f64::NAN, 8000.0, 1.0).is_err());
        assert!(tone(1000.0, f64::INFINITY, 8000.0, 1.0).is_err());
    }
}
