use sn_core::Signal;

use crate::{
    Result, check_amplitude, check_sample_rate, require, require_positive, samples_for_symbols,
    symbol_at, symbol_index,
};

/// M-ary PSK on a real carrier with rectangular pulses.
///
/// Symbol k has phase `2 pi k / order`. Pulse shaping (RRC) arrives in a later phase.
#[derive(Debug, Clone, PartialEq)]
pub struct PskParams {
    /// Modulation order (2 = BPSK, 4 = QPSK, 8 = 8-PSK, up to 32).
    pub order: usize,
    /// Keying rate in baud.
    pub baud: f64,
    /// Carrier frequency in Hz.
    pub carrier: f64,
    /// Output sample rate in Hz.
    pub sample_rate: f64,
    /// Peak amplitude.
    pub amplitude: f64,
}

impl PskParams {
    /// Phase of symbol `k` in radians.
    pub fn symbol_phase(&self, k: usize) -> f64 {
        std::f64::consts::TAU * k as f64 / self.order as f64
    }
}

/// Generate a real PSK signal carrying `symbols`.
pub fn psk(params: &PskParams, symbols: &[usize]) -> Result<Signal> {
    let p = params;
    check_sample_rate(p.sample_rate)?;
    require(
        (2..=32).contains(&p.order),
        "order",
        "must be between 2 and 32",
    )?;
    require_positive(p.baud, "baud", "Bd")?;
    check_amplitude(p.amplitude)?;
    require(
        !symbols.is_empty(),
        "symbols",
        "at least one symbol is needed",
    )?;
    require(
        p.carrier > p.baud / 2.0 && p.carrier + p.baud / 2.0 < p.sample_rate / 2.0,
        "carrier",
        format!(
            "{} Hz with {} Bd does not fit between 0 Hz and Nyquist ({} Hz)",
            p.carrier,
            p.baud,
            p.sample_rate / 2.0
        ),
    )?;
    if let Some(bad) = symbols.iter().find(|s| **s >= p.order) {
        return Err(crate::Error::InvalidParameter {
            name: "symbols",
            reason: format!("symbol {bad} is outside 0..{}", p.order),
        });
    }

    let n = samples_for_symbols(symbols.len(), p.baud, p.sample_rate)?;
    let w = std::f64::consts::TAU * p.carrier / p.sample_rate;
    let out: Vec<f32> = (0..n)
        .map(|i| {
            let sym = symbol_at(symbols, symbol_index(i as f64, p.baud, p.sample_rate));
            (p.amplitude * (w * i as f64 + p.symbol_phase(sym)).cos()) as f32
        })
        .collect();
    Ok(Signal::real(out, p.sample_rate)?)
}

#[cfg(test)]
mod tests {
    use sn_core::Samples;

    use super::*;
    use crate::random_symbols;

    #[test]
    fn symbol_phases_are_recovered() {
        // 1200 Hz carrier, 600 Bd, 48 kHz: 80 samples per symbol, and the 2*fc mixing
        // product completes whole cycles per symbol, so integration is exact.
        let p = PskParams {
            order: 8,
            baud: 600.0,
            carrier: 1200.0,
            sample_rate: 48_000.0,
            amplitude: 1.0,
        };
        let symbols = random_symbols(300, p.order, 11);
        let s = psk(&p, &symbols).unwrap();
        let Samples::Real(x) = s.samples() else {
            panic!("expected real")
        };
        let w = std::f64::consts::TAU * p.carrier / p.sample_rate;
        for (k, sym) in symbols.iter().enumerate() {
            let (mut re, mut im) = (0.0, 0.0);
            for i in k * 80..(k + 1) * 80 {
                re += f64::from(x[i]) * (w * i as f64).cos();
                im -= f64::from(x[i]) * (w * i as f64).sin();
            }
            let got = im.atan2(re).rem_euclid(std::f64::consts::TAU);
            let want = p.symbol_phase(*sym);
            let err = (got - want + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            assert!(err.abs() < 1e-3, "symbol {k}: phase error {err}");
        }
    }

    #[test]
    fn rejects_bad_order_and_symbols() {
        let p = PskParams {
            order: 4,
            baud: 2400.0,
            carrier: 1800.0,
            sample_rate: 48_000.0,
            amplitude: 1.0,
        };
        assert!(psk(&p, &[0, 4]).is_err());
        assert!(
            psk(
                &PskParams {
                    order: 1,
                    ..p.clone()
                },
                &[0]
            )
            .is_err()
        );
        assert!(
            psk(
                &PskParams {
                    carrier: 23_500.0,
                    ..p
                },
                &[0]
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_huge_or_non_finite_rates() {
        let p = PskParams {
            order: 4,
            baud: 2400.0,
            carrier: 1800.0,
            sample_rate: 48_000.0,
            amplitude: 1.0,
        };
        for sample_rate in [f64::INFINITY, 1e300, f64::NAN, 0.0, -48_000.0] {
            let bad = PskParams {
                sample_rate,
                ..p.clone()
            };
            assert!(psk(&bad, &[0, 1]).is_err(), "rate {sample_rate}");
        }
        let slow = PskParams {
            baud: 1e-6,
            carrier: 1800.0,
            ..p.clone()
        };
        assert!(psk(&slow, &[0, 1]).is_err());
        let loud = PskParams {
            amplitude: f64::NAN,
            ..p
        };
        assert!(psk(&loud, &[0]).is_err());
    }
}
