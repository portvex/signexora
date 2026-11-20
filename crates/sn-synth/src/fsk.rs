use sn_core::Signal;

use crate::{
    Result, check_amplitude, check_sample_rate, require, require_positive, samples_for_symbols,
    symbol_at, symbol_index,
};

/// Continuous-phase M-ary FSK.
///
/// Tones sit symmetrically around `center` with spacing `spacing`:
/// tone k = `center + (k - (order - 1) / 2) * spacing`. FSK-2 is `order = 2`.
#[derive(Debug, Clone, PartialEq)]
pub struct MfskParams {
    /// Number of tones (2..=256).
    pub order: usize,
    /// Keying rate in baud.
    pub baud: f64,
    /// Tone spacing in Hz (the shift, for FSK-2).
    pub spacing: f64,
    /// Centre of the tone grid in Hz.
    pub center: f64,
    /// Output sample rate in Hz.
    pub sample_rate: f64,
    /// Peak amplitude.
    pub amplitude: f64,
}

impl MfskParams {
    /// Frequency of tone `k`.
    pub fn tone_frequency(&self, k: usize) -> f64 {
        self.center + (k as f64 - (self.order as f64 - 1.0) / 2.0) * self.spacing
    }
}

/// Generate a real, continuous-phase MFSK signal carrying `symbols`.
///
/// The phase is integrated with the midpoint rule: the step from sample `i` to `i + 1`
/// uses the tone at instant `i + 0.5`, so a discriminator measuring the phase step
/// between two samples sees the frequency of the instant halfway between them.
/// Symbol `k` covers `k / baud` to `(k + 1) / baud` seconds.
pub fn mfsk(params: &MfskParams, symbols: &[usize]) -> Result<Signal> {
    let p = params;
    check_sample_rate(p.sample_rate)?;
    require(
        (2..=256).contains(&p.order),
        "order",
        "must be between 2 and 256",
    )?;
    require_positive(p.baud, "baud", "Bd")?;
    require_positive(p.spacing, "spacing", "Hz")?;
    require(p.center.is_finite(), "center", "must be a finite frequency")?;
    check_amplitude(p.amplitude)?;
    require(
        !symbols.is_empty(),
        "symbols",
        "at least one symbol is needed",
    )?;
    if let Some(bad) = symbols.iter().find(|s| **s >= p.order) {
        return Err(crate::Error::InvalidParameter {
            name: "symbols",
            reason: format!("symbol {bad} is outside 0..{}", p.order),
        });
    }
    let top = p.tone_frequency(p.order - 1) + p.baud;
    let bottom = p.tone_frequency(0) - p.baud;
    require(
        bottom > 0.0 && top < p.sample_rate / 2.0,
        "center",
        format!("the tone grid ({bottom:.1}..{top:.1} Hz) must fit between 0 Hz and Nyquist"),
    )?;

    let n = samples_for_symbols(symbols.len(), p.baud, p.sample_rate)?;
    let mut phase = 0.0_f64;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push((p.amplitude * phase.sin()) as f32);
        let sym = symbol_at(symbols, symbol_index(i as f64 + 0.5, p.baud, p.sample_rate));
        phase += std::f64::consts::TAU * p.tone_frequency(sym) / p.sample_rate;
        phase %= std::f64::consts::TAU;
    }
    Ok(Signal::real(out, p.sample_rate)?)
}

#[cfg(test)]
mod tests {
    use sn_core::Samples;

    use super::*;
    use crate::random_symbols;

    /// Energy of `x` at frequency `f` (single-bin DFT).
    fn energy_at(x: &[f32], f: f64, fs: f64) -> f64 {
        let w = std::f64::consts::TAU * f / fs;
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            re += f64::from(*v) * (w * i as f64).cos();
            im -= f64::from(*v) * (w * i as f64).sin();
        }
        re * re + im * im
    }

    #[test]
    fn each_symbol_sits_on_its_tone() {
        let p = MfskParams {
            order: 4,
            baud: 100.0,
            spacing: 200.0,
            center: 1500.0,
            sample_rate: 8000.0,
            amplitude: 0.8,
        };
        let symbols = random_symbols(200, p.order, 5);
        let s = mfsk(&p, &symbols).unwrap();
        let Samples::Real(x) = s.samples() else {
            panic!("expected real")
        };
        let sps = 80;
        for (k, sym) in symbols.iter().enumerate() {
            let chunk = &x[k * sps..(k + 1) * sps];
            let best = (0..p.order)
                .max_by(|a, b| {
                    energy_at(chunk, p.tone_frequency(*a), p.sample_rate).total_cmp(&energy_at(
                        chunk,
                        p.tone_frequency(*b),
                        p.sample_rate,
                    ))
                })
                .unwrap();
            assert_eq!(best, *sym, "symbol {k}");
        }
    }

    #[test]
    fn rejects_grid_outside_band() {
        let p = MfskParams {
            order: 16,
            baud: 100.0,
            spacing: 400.0,
            center: 1000.0,
            sample_rate: 8000.0,
            amplitude: 1.0,
        };
        assert!(mfsk(&p, &[0, 1]).is_err());
    }

    #[test]
    fn rejects_huge_or_non_finite_values() {
        let p = MfskParams {
            order: 2,
            baud: 100.0,
            spacing: 200.0,
            center: 1500.0,
            sample_rate: 8000.0,
            amplitude: 1.0,
        };
        for sample_rate in [f64::INFINITY, 1e300, f64::NAN, -8000.0] {
            let bad = MfskParams {
                sample_rate,
                ..p.clone()
            };
            assert!(mfsk(&bad, &[0, 1]).is_err(), "rate {sample_rate}");
        }
        let slow = MfskParams {
            baud: 1e-6,
            ..p.clone()
        };
        assert!(mfsk(&slow, &[0, 1]).is_err());
        let loud = MfskParams {
            amplitude: f64::INFINITY,
            ..p
        };
        assert!(mfsk(&loud, &[0]).is_err());
    }
}
