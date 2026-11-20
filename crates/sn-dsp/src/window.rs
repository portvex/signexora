//! FFT window functions.

use std::f64::consts::PI;
use std::fmt;

/// A window function. SA's default is `Kaiser { beta: 8.0 }` (report 6).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WindowKind {
    /// No window (best resolution, worst leakage).
    Rectangular,
    /// Hann.
    Hann,
    /// Hamming.
    Hamming,
    /// Blackman.
    Blackman,
    /// 4-term Blackman-Harris (very low sidelobes).
    BlackmanHarris,
    /// Flat-top (accurate amplitudes, wide main lobe).
    FlatTop,
    /// Kaiser with shape parameter beta (0 = rectangular; 8 is SA's default).
    Kaiser {
        /// Shape parameter.
        beta: f64,
    },
}

impl Default for WindowKind {
    fn default() -> Self {
        Self::Kaiser { beta: 8.0 }
    }
}

impl fmt::Display for WindowKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rectangular => f.write_str("Rectangular"),
            Self::Hann => f.write_str("Hann"),
            Self::Hamming => f.write_str("Hamming"),
            Self::Blackman => f.write_str("Blackman"),
            Self::BlackmanHarris => f.write_str("Blackman-Harris"),
            Self::FlatTop => f.write_str("Flat-top"),
            Self::Kaiser { beta } => write!(f, "Kaiser, b={beta}"),
        }
    }
}

impl WindowKind {
    /// Every window, for selection lists (Kaiser at SA's default beta).
    pub const ALL: [WindowKind; 7] = [
        Self::Kaiser { beta: 8.0 },
        Self::Rectangular,
        Self::Hann,
        Self::Hamming,
        Self::Blackman,
        Self::BlackmanHarris,
        Self::FlatTop,
    ];

    /// `n` window coefficients (symmetric). `n == 0` gives an empty vector.
    pub fn coefficients(self, n: usize) -> Vec<f32> {
        if n <= 1 {
            return vec![1.0; n];
        }
        let m = (n - 1) as f64;
        (0..n)
            .map(|i| {
                let x = i as f64 / m; // 0..=1
                let c = |k: f64| (2.0 * PI * k * x).cos();
                let w = match self {
                    Self::Rectangular => 1.0,
                    Self::Hann => 0.5 - 0.5 * c(1.0),
                    Self::Hamming => 0.54 - 0.46 * c(1.0),
                    Self::Blackman => 0.42 - 0.5 * c(1.0) + 0.08 * c(2.0),
                    Self::BlackmanHarris => {
                        0.358_75 - 0.488_29 * c(1.0) + 0.141_28 * c(2.0) - 0.011_68 * c(3.0)
                    }
                    Self::FlatTop => {
                        0.215_578_95 - 0.416_631_58 * c(1.0) + 0.277_263_158 * c(2.0)
                            - 0.083_578_947 * c(3.0)
                            + 0.006_947_368 * c(4.0)
                    }
                    Self::Kaiser { beta } => {
                        let r = 2.0 * x - 1.0; // -1..=1
                        bessel_i0(beta * (1.0 - r * r).max(0.0).sqrt()) / bessel_i0(beta)
                    }
                };
                w as f32
            })
            .collect()
    }
}

/// Modified Bessel function of the first kind, order 0 (series expansion).
fn bessel_i0(x: f64) -> f64 {
    let half = x / 2.0;
    let mut term = 1.0;
    let mut sum = 1.0;
    for k in 1..64 {
        let kf = f64::from(k);
        term *= (half / kf) * (half / kf);
        sum += term;
        if term < sum * 1e-16 {
            break;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_are_symmetric_and_peak_at_centre() {
        for w in WindowKind::ALL {
            let c = w.coefficients(65);
            for i in 0..32 {
                assert!((c[i] - c[64 - i]).abs() < 1e-6, "{w} not symmetric");
            }
            let peak = c.iter().copied().fold(f32::MIN, f32::max);
            assert!((c[32] - peak).abs() < 1e-6, "{w} peak not at centre");
        }
    }

    #[test]
    fn known_values() {
        let h = WindowKind::Hann.coefficients(5);
        assert!(h[0].abs() < 1e-7 && (h[2] - 1.0).abs() < 1e-7);
        let k0 = WindowKind::Kaiser { beta: 0.0 }.coefficients(9);
        assert!(k0.iter().all(|v| (v - 1.0).abs() < 1e-6));
        let k8 = WindowKind::default().coefficients(9);
        // Kaiser beta 8 edge value = 1 / I0(8) = 1 / 427.564...
        assert!((f64::from(k8[0]) - 1.0 / 427.564_115_721_804_7).abs() < 1e-7);
        assert_eq!(WindowKind::Hann.coefficients(0).len(), 0);
        assert_eq!(WindowKind::Hann.coefficients(1), vec![1.0]);
    }

    #[test]
    fn bessel_reference_points() {
        assert!((bessel_i0(0.0) - 1.0).abs() < 1e-12);
        assert!((bessel_i0(1.0) - 1.266_065_877_752_008).abs() < 1e-12);
        assert!((bessel_i0(8.0) - 427.564_115_721_804_7).abs() < 1e-9);
    }
}
