//! The continuous-phase FSK family: CPFSK (M-ary, 2 to 256 tones), GFSK, MSK and GMSK,
//! with real or complex output.
//!
//! The instantaneous frequency is `center + spacing * sum_k d_k g(t - t_k)`, with
//! `d_k = s_k - (order - 1) / 2` the deviation of symbol `k` in tone steps, `t_k` its
//! centre `(k + 0.5) / baud` and `g` the frequency pulse: a one-symbol rectangle for
//! [`FskShaping::None`], or that rectangle through a Gaussian filter of bandwidth
//! `bt * baud` for [`FskShaping::Gaussian`]. Both pulses integrate to one symbol, so
//! every symbol advances the phase by `2 pi spacing d_k / baud` in either case. The
//! phase is integrated sample by sample (midpoint rule, as [`crate::mfsk`]), so it is
//! continuous throughout.
//!
//! The modulation index is `h = spacing / baud`. MSK is binary CPFSK with `h = 0.5`
//! ([`FskParams::msk`]); GMSK adds a Gaussian filter ([`FskParams::gmsk`]).

use std::f64::consts::{LN_2, PI, TAU};

use serde::{Deserialize, Serialize};
use sn_core::{Complex32, Signal};

use crate::{
    Error, Result, StopCheck, check_amplitude, check_sample_rate, check_stop, never_stop, require,
    require_positive, samples_for_symbols, symbol_at, symbol_index,
};

/// Resolution of the tabulated Gaussian frequency pulse, in points per symbol.
const PULSE_STEPS_PER_SYMBOL: usize = 4096;

/// Frequency-pulse shaping of an FSK signal.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FskShaping {
    /// Rectangular frequency pulses (plain CPFSK, as [`crate::mfsk`]).
    None,
    /// Gaussian-filtered pulses (GFSK; GMSK when binary with `h = 0.5`).
    Gaussian {
        /// Bandwidth-time product of the Gaussian filter, 0.1 to 10.
        bt: f64,
    },
}

/// Parameters of a continuous-phase FSK signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FskParams {
    /// Number of tones (2..=256).
    pub order: usize,
    /// Keying rate in baud.
    pub baud: f64,
    /// Tone spacing in Hz (the shift, for FSK-2).
    pub spacing: f64,
    /// Centre of the tone grid in Hz (with complex output, the offset from 0 Hz).
    pub center: f64,
    /// Output sample rate in Hz.
    pub sample_rate: f64,
    /// Peak amplitude (the envelope is constant).
    pub amplitude: f64,
    /// Produce complex (I/Q) output instead of a real signal.
    pub complex: bool,
    /// Frequency-pulse shaping.
    pub shaping: FskShaping,
}

impl FskParams {
    /// MSK: binary CPFSK with spacing `baud / 2` (modulation index 0.5).
    pub fn msk(baud: f64, center: f64, sample_rate: f64, amplitude: f64, complex: bool) -> Self {
        Self {
            order: 2,
            baud,
            spacing: baud / 2.0,
            center,
            sample_rate,
            amplitude,
            complex,
            shaping: FskShaping::None,
        }
    }

    /// GMSK: MSK with a Gaussian filter of bandwidth-time product `bt` (0.3 in GSM).
    pub fn gmsk(
        baud: f64,
        bt: f64,
        center: f64,
        sample_rate: f64,
        amplitude: f64,
        complex: bool,
    ) -> Self {
        Self {
            shaping: FskShaping::Gaussian { bt },
            ..Self::msk(baud, center, sample_rate, amplitude, complex)
        }
    }

    /// Modulation index `spacing / baud`.
    pub fn modulation_index(&self) -> f64 {
        self.spacing / self.baud
    }

    /// Frequency of tone `k`.
    pub fn tone_frequency(&self, k: usize) -> f64 {
        self.center + (k as f64 - (self.order as f64 - 1.0) / 2.0) * self.spacing
    }

    /// Bandwidth the signal occupies in Hz (Carson-like): the tone grid plus one baud on
    /// each side.
    pub fn occupied_bandwidth(&self) -> f64 {
        (self.order as f64 - 1.0) * self.spacing + 2.0 * self.baud
    }

    /// Short English name, for example "GMSK BT 0.3" or "FSK-4".
    pub fn name(&self) -> String {
        let h = self.modulation_index();
        let msk = self.order == 2 && (h - 0.5).abs() < 1e-9;
        match (self.shaping, msk) {
            (FskShaping::None, true) => "MSK".to_owned(),
            (FskShaping::Gaussian { bt }, true) => format!("GMSK BT {}", crate::spec::fmt_num(bt)),
            (FskShaping::None, false) => format!("FSK-{}", self.order),
            (FskShaping::Gaussian { bt }, false) => {
                format!("GFSK-{} BT {}", self.order, crate::spec::fmt_num(bt))
            }
        }
    }

    /// Check every field.
    pub fn validate(&self) -> Result<()> {
        let p = self;
        check_sample_rate(p.sample_rate)?;
        require(
            (2..=256).contains(&p.order),
            "order",
            format!("must be between 2 and 256 (got {})", p.order),
        )?;
        require_positive(p.baud, "baud", "Bd")?;
        require_positive(p.spacing, "spacing", "Hz")?;
        require(p.center.is_finite(), "center", "must be a finite frequency")?;
        check_amplitude(p.amplitude)?;
        if let FskShaping::Gaussian { bt } = p.shaping {
            require(
                bt.is_finite() && (0.1..=10.0).contains(&bt),
                "bt",
                format!("the Gaussian BT must be between 0.1 and 10 (got {bt})"),
            )?;
        }
        let top = p.tone_frequency(p.order - 1) + p.baud;
        let bottom = p.tone_frequency(0) - p.baud;
        let nyquist = p.sample_rate / 2.0;
        let floor = if p.complex { -nyquist } else { 0.0 };
        require(
            bottom > floor && top < nyquist,
            "center",
            format!(
                "the tone grid ({bottom:.1}..{top:.1} Hz) must fit between {floor} Hz and \
                 Nyquist ({nyquist} Hz)"
            ),
        )
    }
}

/// Error function, |error| below 1.2e-7 (Numerical Recipes' Chebyshev fit of erfc).
pub(crate) fn erf(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let poly = -z * z - 1.265_512_23
        + t * (1.000_023_68
            + t * (0.374_091_96
                + t * (0.096_784_18
                    + t * (-0.186_288_06
                        + t * (0.278_868_07
                            + t * (-1.135_203_98
                                + t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
    let erfc = t * poly.exp();
    if x >= 0.0 { 1.0 - erfc } else { erfc - 1.0 }
}

/// Gaussian-filtered rectangular frequency pulse at `t` symbols from its centre; it
/// integrates to 1 over all `t`.
fn gaussian_pulse(t: f64, c: f64) -> f64 {
    0.5 * (erf(c * (t + 0.5)) - erf(c * (t - 0.5)))
}

/// [`gaussian_pulse`] at `t = k / PULSE_STEPS_PER_SYMBOL` for `t` from 0 to `reach`
/// (and one point beyond). The pulse is even, so only `t >= 0` is stored.
fn gaussian_table(c: f64, reach: f64) -> Vec<f64> {
    let len = (reach * PULSE_STEPS_PER_SYMBOL as f64).ceil() as usize + 2;
    (0..len)
        .map(|k| gaussian_pulse(k as f64 / PULSE_STEPS_PER_SYMBOL as f64, c))
        .collect()
}

/// Linear interpolation in a table made by [`gaussian_table`]; 0 beyond it. The error
/// is below about 2e-5 of the peak even at BT 10, where the pulse is steepest.
fn table_pulse(table: &[f64], t: f64) -> f64 {
    let x = t.abs() * PULSE_STEPS_PER_SYMBOL as f64;
    let k = x.floor();
    let frac = x - k;
    let k = k as usize;
    match (table.get(k), table.get(k + 1)) {
        (Some(a), Some(b)) => a + (b - a) * frac,
        _ => 0.0,
    }
}

/// Generate a continuous-phase FSK signal carrying `symbols` (each below `order`).
///
/// With [`FskShaping::None`] and real output this is sample for sample the same signal
/// as [`crate::mfsk`] with the same parameters.
pub fn fsk(params: &FskParams, symbols: &[usize]) -> Result<Signal> {
    fsk_with_stop(params, symbols, &never_stop)
}

/// [`fsk`] that calls `stop` now and then and fails with [`Error::Cancelled`] when it
/// returns true.
pub fn fsk_with_stop(params: &FskParams, symbols: &[usize], stop: StopCheck<'_>) -> Result<Signal> {
    let p = params;
    p.validate()?;
    require(
        !symbols.is_empty(),
        "symbols",
        "at least one symbol is needed",
    )?;
    if let Some(bad) = symbols.iter().find(|s| **s >= p.order) {
        return Err(Error::InvalidParameter {
            name: "symbols",
            reason: format!("symbol {bad} is outside 0..{}", p.order),
        });
    }
    let n = samples_for_symbols(symbols.len(), p.baud, p.sample_rate)?;
    // Gaussian filter constant in symbol units, the pulse half-length beyond which it
    // is below about 1e-8, and the pulse tabulated once over that reach.
    let gauss = match p.shaping {
        FskShaping::None => None,
        FskShaping::Gaussian { bt } => {
            let c = PI * bt * (2.0 / LN_2).sqrt();
            let reach = 0.5 + 4.0 / c;
            Some((reach, gaussian_table(c, reach)))
        }
    };
    let mid = (p.order as f64 - 1.0) / 2.0;
    let frequency = |i: usize| -> f64 {
        match &gauss {
            None => {
                let sym = symbol_at(symbols, symbol_index(i as f64 + 0.5, p.baud, p.sample_rate));
                p.tone_frequency(sym)
            }
            Some((reach, table)) => {
                let reach = *reach;
                // Symbol units; symbol k is centred at k + 0.5.
                let u = (i as f64 + 0.5) * p.baud / p.sample_rate - 0.5;
                let lo = (u - reach).ceil().max(0.0) as usize;
                let hi = (u + reach).floor();
                let mut dev = 0.0;
                if hi >= 0.0 {
                    let hi = (hi as usize).min(symbols.len().saturating_sub(1));
                    for (k, s) in symbols.iter().enumerate().take(hi + 1).skip(lo) {
                        dev += (*s as f64 - mid) * table_pulse(table, u - k as f64);
                    }
                }
                p.center + dev * p.spacing
            }
        }
    };
    let mut phase = 0.0_f64;
    let signal = if p.complex {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            check_stop(stop, i)?;
            let (s, c) = phase.sin_cos();
            out.push(Complex32::new(
                (p.amplitude * c) as f32,
                (p.amplitude * s) as f32,
            ));
            phase += TAU * frequency(i) / p.sample_rate;
            phase %= TAU;
        }
        Signal::complex(out, p.sample_rate)?
    } else {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            check_stop(stop, i)?;
            out.push((p.amplitude * phase.sin()) as f32);
            phase += TAU * frequency(i) / p.sample_rate;
            phase %= TAU;
        }
        Signal::real(out, p.sample_rate)?
    };
    Ok(signal)
}

#[cfg(test)]
mod tests {
    use rustfft::FftPlanner;
    use rustfft::num_complex::Complex64;
    use sn_core::Samples;

    use super::*;
    use crate::{MfskParams, mfsk, random_symbols};

    fn complex(s: &Signal) -> Vec<Complex64> {
        let Samples::Complex(x) = s.samples() else {
            panic!("expected complex")
        };
        x.iter()
            .map(|v| Complex64::new(f64::from(v.re), f64::from(v.im)))
            .collect()
    }

    /// Bandwidth holding 99% of the power, centred on 0 Hz.
    fn bandwidth_99(x: &[Complex64], fs: f64) -> f64 {
        let n = x.len();
        let mut buf = x.to_vec();
        FftPlanner::new().plan_fft_forward(n).process(&mut buf);
        let total: f64 = buf.iter().map(Complex64::norm_sqr).sum();
        let mut half = 0;
        loop {
            let inside: f64 = buf
                .iter()
                .enumerate()
                .filter(|(k, _)| *k <= half || *k >= n - half)
                .map(|(_, v)| v.norm_sqr())
                .sum();
            if inside >= 0.99 * total {
                return 2.0 * half as f64 * fs / n as f64;
            }
            half += 1;
        }
    }

    #[test]
    fn plain_real_matches_mfsk_exactly() {
        let m = MfskParams {
            order: 8,
            baud: 75.0,
            spacing: 125.0,
            center: 1700.0,
            sample_rate: 8000.0,
            amplitude: 0.7,
        };
        let syms = random_symbols(100, 8, 1);
        let a = mfsk(&m, &syms).unwrap();
        let p = FskParams {
            order: 8,
            baud: 75.0,
            spacing: 125.0,
            center: 1700.0,
            sample_rate: 8000.0,
            amplitude: 0.7,
            complex: false,
            shaping: FskShaping::None,
        };
        let b = fsk(&p, &syms).unwrap();
        let (Samples::Real(a), Samples::Real(b)) = (a.samples(), b.samples()) else {
            panic!("expected real")
        };
        assert_eq!(a, b);
    }

    #[test]
    fn phase_is_continuous_and_tones_are_right() {
        for shaping in [FskShaping::None, FskShaping::Gaussian { bt: 0.5 }] {
            let p = FskParams {
                order: 4,
                baud: 100.0,
                spacing: 200.0,
                center: 300.0,
                sample_rate: 8000.0,
                amplitude: 1.0,
                complex: true,
                shaping,
            };
            let syms = random_symbols(300, 4, 2);
            let x = complex(&fsk(&p, &syms).unwrap());
            // Constant envelope and bounded phase steps: no jumps anywhere.
            let max_f = 300.0 + 1.5 * 200.0;
            let max_step = TAU * max_f / 8000.0 + 1e-3;
            for w in x.windows(2) {
                assert!((w[0].norm() - 1.0).abs() < 1e-5);
                let step = (w[1] * w[0].conj()).arg().abs();
                assert!(step <= max_step, "step {step} > {max_step}");
            }
            // The phase advance over each symbol, at its centre region, matches its tone
            // (exactly for rectangular, nearly for a Gaussian pulse at BT 0.5).
            for (k, s) in syms.iter().enumerate().skip(1).take(298) {
                // Five samples: under half a cycle even at the top tone (600 Hz).
                let a = k * 80 + 38;
                let f = (x[a + 5] * x[a].conj()).arg() / TAU * 8000.0 / 5.0;
                let want = p.tone_frequency(*s);
                let tol = if shaping == FskShaping::None {
                    0.01
                } else {
                    60.0
                };
                assert!(
                    (f - want).abs() < tol,
                    "{shaping:?} symbol {k}: {f} vs {want}"
                );
            }
        }
    }

    #[test]
    fn msk_steps_quarter_cycle_per_symbol() {
        let p = FskParams::msk(1000.0, 0.0, 8000.0, 1.0, true);
        assert!((p.modulation_index() - 0.5).abs() < 1e-12);
        let syms = random_symbols(200, 2, 7);
        let x = complex(&fsk(&p, &syms).unwrap());
        for (k, s) in syms.iter().enumerate() {
            let d = (x[(k + 1) * 8 % x.len()] * x[k * 8].conj()).arg();
            if (k + 1) * 8 < x.len() {
                let want = if *s == 1 { PI / 2.0 } else { -PI / 2.0 };
                assert!((d - want).abs() < 1e-3, "symbol {k}: {d}");
            }
        }
    }

    #[test]
    fn gaussian_filter_narrows_the_spectrum() {
        let syms = random_symbols(4000, 2, 3);
        let plain = FskParams::msk(500.0, 0.0, 8000.0, 1.0, true);
        let b_plain = bandwidth_99(&complex(&fsk(&plain, &syms).unwrap()), 8000.0);
        let mut last = b_plain;
        for bt in [1.0, 0.5, 0.3] {
            let g = FskParams::gmsk(500.0, bt, 0.0, 8000.0, 1.0, true);
            let b = bandwidth_99(&complex(&fsk(&g, &syms).unwrap()), 8000.0);
            assert!(b < last, "BT {bt}: {b} Hz not below {last} Hz");
            last = b;
        }
        // GMSK BT 0.3: 99% bandwidth about 0.92 baud; MSK about 1.2 baud.
        assert!(last < 0.8 * b_plain, "{last} vs {b_plain}");
        assert!(
            (400.0..520.0).contains(&last),
            "GMSK 0.3 99% bandwidth {last}"
        );
    }

    #[test]
    fn gaussian_pulse_integrates_to_one() {
        let c = PI * 0.3 * (2.0 / LN_2).sqrt();
        let n = 20_000;
        let sum: f64 = (0..n)
            .map(|i| gaussian_pulse(-10.0 + 20.0 * f64::from(i) / f64::from(n), c))
            .sum::<f64>()
            * 20.0
            / f64::from(n);
        assert!((sum - 1.0).abs() < 1e-5, "{sum}");
        assert!((erf(0.5) - 0.520_499_877_8).abs() < 2e-7);
        assert!((erf(-1.5) + 0.966_105_146_5).abs() < 2e-7);
    }

    #[test]
    fn tabulated_pulse_matches_the_direct_one() {
        for bt in [0.1, 0.3, 1.0, 10.0] {
            let c = PI * bt * (2.0 / LN_2).sqrt();
            let reach = 0.5 + 4.0 / c;
            let table = gaussian_table(c, reach);
            let peak = gaussian_pulse(0.0, c);
            for i in 0..2000 {
                let t = -reach + 2.0 * reach * f64::from(i) / 2000.0 + 1e-4;
                let err = (table_pulse(&table, t) - gaussian_pulse(t, c)).abs();
                assert!(err < 3e-5 * peak, "BT {bt} t {t}: {err}");
            }
        }
    }

    #[test]
    fn stop_cancels() {
        let p = FskParams::gmsk(100.0, 0.3, 1500.0, 8000.0, 1.0, false);
        let e = fsk_with_stop(&p, &[0, 1, 0], &|| true).unwrap_err();
        assert!(matches!(e, Error::Cancelled), "{e}");
    }

    #[test]
    fn validation() {
        let p = FskParams::msk(100.0, 1500.0, 8000.0, 1.0, false);
        assert!(fsk(&p, &[0, 2]).is_err());
        assert!(fsk(&p, &[]).is_err());
        let low = FskParams {
            center: 50.0,
            ..p.clone()
        };
        assert!(fsk(&low, &[0]).is_err());
        // The same low centre is fine for complex output.
        let low_c = FskParams {
            complex: true,
            ..low
        };
        assert!(fsk(&low_c, &[0]).is_ok());
        let bad_bt = FskParams {
            shaping: FskShaping::Gaussian { bt: 0.0 },
            ..p.clone()
        };
        assert!(fsk(&bad_bt, &[0]).is_err());
        let big = FskParams { order: 257, ..p };
        assert!(fsk(&big, &[0]).is_err());
    }
}
