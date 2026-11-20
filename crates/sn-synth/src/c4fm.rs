use sn_core::Signal;

use crate::{
    Result, StopCheck, check_amplitude, check_sample_rate, check_stop, never_stop, require,
    require_positive, samples_for_symbols, symbol_at, symbol_index,
};

/// Frequency deviations of the four C4FM levels in Hz, lowest first.
///
/// Level 0 is -1800 Hz (dibit 11), level 1 -600 Hz (10), level 2 +600 Hz (00) and
/// level 3 +1800 Hz (01), as in APCO-25 Phase 1.
pub const C4FM_DEVIATIONS: [f64; 4] = [-1800.0, -600.0, 600.0, 1800.0];

/// Roll-off of the C4FM Nyquist (raised-cosine) frequency pulse.
pub const C4FM_ALPHA: f64 = 0.2;

/// Half-length of the shaped frequency pulse in symbols.
const PULSE_HALF_SPAN: i64 = 8;

/// Resolution of the tabulated APCO-25 pulse, in points per symbol.
const TABLE_STEPS_PER_SYMBOL: usize = 256;

/// Shape of the frequency pulse when [`C4fmParams::shaped`] is true.
///
/// Which receive filter gives zero inter-symbol interference at the symbol centres:
///
/// - [`C4fmPulse::RaisedCosine`]: none. The frequency track is already Nyquist; an RRC
///   receive filter adds ISI (about 28% of a symbol in the worst case for alpha 0.2).
/// - [`C4fmPulse::Apco25`]: an integrate-and-dump (one-symbol moving average) on the
///   frequency track, as the TIA-102 receiver assumes. An RRC filter is only matched to
///   an RRC transmit pulse, which neither option is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum C4fmPulse {
    /// A raised cosine with roll-off [`C4FM_ALPHA`]: ISI-free at the symbol centres
    /// without any receive filter.
    #[default]
    RaisedCosine,
    /// The TIA-102 C4FM pulse: raised cosine (alpha 0.2) cascaded with the shaping
    /// filter P(f) = (pi f / Rs) / sin(pi f / Rs), so that an integrate-and-dump
    /// receiver sees a raised cosine. Without that filter the track overshoots and the
    /// symbol centres carry ISI.
    Apco25,
}

/// Four-level FSK in the style of APCO-25 C4FM.
#[derive(Debug, Clone, PartialEq)]
pub struct C4fmParams {
    /// Keying rate in baud (4800 for APCO-25).
    pub baud: f64,
    /// Carrier (centre) frequency in Hz.
    pub carrier: f64,
    /// Output sample rate in Hz.
    pub sample_rate: f64,
    /// Peak amplitude.
    pub amplitude: f64,
    /// Shape the frequency pulses with [`C4fmParams::pulse`]. When false, the
    /// frequency jumps between levels (rectangular).
    pub shaped: bool,
    /// Which shaped pulse to use (ignored when `shaped` is false). See [`C4fmPulse`]
    /// for the receive filter each one is meant for.
    pub pulse: C4fmPulse,
}

impl Default for C4fmParams {
    fn default() -> Self {
        Self {
            baud: 4800.0,
            carrier: 12_000.0,
            sample_rate: 48_000.0,
            amplitude: 0.5,
            shaped: true,
            pulse: C4fmPulse::RaisedCosine,
        }
    }
}

/// Half-width of the band the generator requires to fit between 0 Hz and Nyquist:
/// the outer deviation plus one baud either side of the carrier.
pub(crate) fn c4fm_band_half_width(baud: f64) -> f64 {
    let max_dev = C4FM_DEVIATIONS.iter().fold(0.0_f64, |a, d| a.max(d.abs()));
    max_dev + baud
}

impl C4fmParams {
    /// Check every field (the symbol levels are checked by [`c4fm`]).
    pub(crate) fn check(&self) -> Result<()> {
        let p = self;
        check_sample_rate(p.sample_rate)?;
        require_positive(p.baud, "baud", "Bd")?;
        check_amplitude(p.amplitude)?;
        require(
            p.sample_rate >= 4.0 * p.baud,
            "sample_rate",
            "must be at least 4 times the baud rate",
        )?;
        let half = c4fm_band_half_width(p.baud);
        let top = p.carrier + half;
        let bottom = p.carrier - half;
        require(
            bottom > 0.0 && top < p.sample_rate / 2.0,
            "carrier",
            format!("the signal ({bottom:.1}..{top:.1} Hz) must fit between 0 Hz and Nyquist"),
        )
    }
}

/// Level (0..=3, lowest frequency first) that carries an APCO-25 dibit (0..=3,
/// first bit in the high position). Values above 3 use their two low bits.
pub fn c4fm_level_of_dibit(dibit: u8) -> usize {
    match dibit & 3 {
        0b01 => 3,
        0b00 => 2,
        0b10 => 1,
        _ => 0,
    }
}

/// Raised-cosine Nyquist pulse at `t` symbols from its centre.
fn raised_cosine(t: f64, alpha: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let sinc = |x: f64| {
        if x.abs() < 1e-12 {
            1.0
        } else {
            (pi * x).sin() / (pi * x)
        }
    };
    let d = 1.0 - (2.0 * alpha * t).powi(2);
    if d.abs() < 1e-9 {
        pi / 4.0 * sinc(1.0 / (2.0 * alpha))
    } else {
        sinc(t) * (pi * alpha * t).cos() / d
    }
}

/// Frequency response of the APCO-25 pulse at `f` (in units of the symbol rate): the
/// raised cosine times the inverse-sinc shaping filter.
fn apco25_spectrum(f: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let a = C4FM_ALPHA;
    let f = f.abs();
    let rc = if f <= (1.0 - a) / 2.0 {
        1.0
    } else if f <= f64::midpoint(1.0, a) {
        f64::midpoint(1.0, (pi / a * (f - (1.0 - a) / 2.0)).cos())
    } else {
        0.0
    };
    let shaping = if f < 1e-12 {
        1.0
    } else {
        (pi * f) / (pi * f).sin()
    };
    rc * shaping
}

/// The APCO-25 frequency pulse at `t = k / TABLE_STEPS_PER_SYMBOL` symbols for
/// `k = 0..=(PULSE_HALF_SPAN + 1) * TABLE_STEPS_PER_SYMBOL`, from the inverse Fourier
/// transform of [`apco25_spectrum`] (Simpson's rule over the occupied band). The pulse
/// is even, so only `t >= 0` is stored.
fn apco25_table() -> Vec<f64> {
    const INTERVALS: usize = 600; // even, as Simpson's rule needs
    let f_max = f64::midpoint(1.0, C4FM_ALPHA);
    let h = f_max / INTERVALS as f64;
    let spectrum: Vec<f64> = (0..=INTERVALS)
        .map(|k| apco25_spectrum(k as f64 * h))
        .collect();
    let len = (PULSE_HALF_SPAN as usize + 1) * TABLE_STEPS_PER_SYMBOL + 1;
    (0..len)
        .map(|n| {
            let t = n as f64 / TABLE_STEPS_PER_SYMBOL as f64;
            let sum: f64 = spectrum
                .iter()
                .enumerate()
                .map(|(k, g)| {
                    let w = if k == 0 || k == INTERVALS {
                        1.0
                    } else if k % 2 == 1 {
                        4.0
                    } else {
                        2.0
                    };
                    w * g * (std::f64::consts::TAU * k as f64 * h * t).cos()
                })
                .sum();
            // Two-sided transform of an even spectrum: 2 * integral over f >= 0.
            2.0 * sum * h / 3.0
        })
        .collect()
}

/// Linear interpolation in a table made by [`apco25_table`]; 0 outside it.
fn table_pulse(table: &[f64], t: f64) -> f64 {
    let x = t.abs() * TABLE_STEPS_PER_SYMBOL as f64;
    let k = x.floor();
    let frac = x - k;
    let k = k as usize;
    match (table.get(k), table.get(k + 1)) {
        (Some(a), Some(b)) => a + (b - a) * frac,
        (Some(a), None) => *a,
        _ => 0.0,
    }
}

/// Generate a real C4FM / 4-level FSK signal carrying `levels` (each 0..=3, see
/// [`C4FM_DEVIATIONS`]). Symbol `k` is centred at `(k + 0.5) / baud` seconds.
///
/// The phase is integrated with the midpoint rule: the step from sample `i` to `i + 1`
/// uses the frequency at instant `i + 0.5`, so a discriminator measuring the phase
/// step between two samples reads the frequency of the instant halfway between them,
/// and the symbol centres above hold for discriminator-based timing.
///
/// Shaped pulses overshoot the nominal deviation between symbol centres. On the worst
/// data pattern the instantaneous deviation reaches about 2.06 times the outer level
/// (about 3710 Hz for +/-1800 Hz) with [`C4fmPulse::RaisedCosine`] and about 2.55 times
/// (about 4580 Hz) with [`C4fmPulse::Apco25`]; random data stays well below that. The
/// band check uses the nominal deviation plus one baud on each side.
pub fn c4fm(params: &C4fmParams, levels: &[usize]) -> Result<Signal> {
    c4fm_with_stop(params, levels, &never_stop)
}

/// [`c4fm`] that calls `stop` now and then and fails with [`crate::Error::Cancelled`]
/// when it returns true.
pub fn c4fm_with_stop(
    params: &C4fmParams,
    levels: &[usize],
    stop: StopCheck<'_>,
) -> Result<Signal> {
    let p = params;
    p.check()?;
    require(
        !levels.is_empty(),
        "levels",
        "at least one symbol is needed",
    )?;
    if let Some(bad) = levels.iter().find(|l| **l > 3) {
        return Err(crate::Error::InvalidParameter {
            name: "levels",
            reason: format!("level {bad} is outside 0..=3"),
        });
    }
    let dev = |k: i64| -> f64 {
        usize::try_from(k)
            .ok()
            .and_then(|k| levels.get(k))
            .and_then(|l| C4FM_DEVIATIONS.get(*l))
            .copied()
            .unwrap_or(0.0)
    };

    let n = samples_for_symbols(levels.len(), p.baud, p.sample_rate)?;
    let sps = p.sample_rate / p.baud;
    let table = if p.shaped && p.pulse == C4fmPulse::Apco25 {
        apco25_table()
    } else {
        Vec::new()
    };
    let pulse = |t: f64| match p.pulse {
        C4fmPulse::RaisedCosine => raised_cosine(t, C4FM_ALPHA),
        C4fmPulse::Apco25 => table_pulse(&table, t),
    };
    let mut phase = 0.0_f64;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        check_stop(stop, i)?;
        out.push((p.amplitude * phase.sin()) as f32);
        // Midpoint of the step from sample i to i + 1.
        let mid = i as f64 + 0.5;
        let f = if p.shaped {
            let t = mid / sps;
            let k = (t - 0.5).round() as i64;
            (k - PULSE_HALF_SPAN..=k + PULSE_HALF_SPAN)
                .map(|j| dev(j) * pulse(t - (j as f64 + 0.5)))
                .sum::<f64>()
        } else {
            let k = symbol_at(levels, symbol_index(mid, p.baud, p.sample_rate));
            C4FM_DEVIATIONS.get(k).copied().unwrap_or(0.0)
        };
        phase += std::f64::consts::TAU * (p.carrier + f) / p.sample_rate;
        phase %= std::f64::consts::TAU;
    }
    Ok(Signal::real(out, p.sample_rate)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raised_cosine_is_nyquist() {
        assert!((raised_cosine(0.0, C4FM_ALPHA) - 1.0).abs() < 1e-12);
        for k in 1..6 {
            assert!(raised_cosine(f64::from(k), C4FM_ALPHA).abs() < 1e-9);
        }
        // The removable singularity at t = 1 / (2 alpha) is finite and continuous.
        let t = 1.0 / (2.0 * C4FM_ALPHA);
        let a = raised_cosine(t, C4FM_ALPHA);
        let b = raised_cosine(t + 1e-6, C4FM_ALPHA);
        assert!((a - b).abs() < 1e-4);
    }

    #[test]
    fn apco25_pulse_is_nyquist_after_integrate_and_dump() {
        let table = apco25_table();
        // Peak above 1: the inverse-sinc filter boosts the high frequencies.
        assert!(table[0] > 1.0, "peak {}", table[0]);
        // A one-symbol moving average of the pulse is 1 at its centre and 0 at the
        // other symbol centres.
        let average = |centre: f64| {
            let steps = 2000_u32;
            (0..steps)
                .map(|k| {
                    table_pulse(
                        &table,
                        centre - 0.5 + (f64::from(k) + 0.5) / f64::from(steps),
                    )
                })
                .sum::<f64>()
                / f64::from(steps)
        };
        assert!((average(0.0) - 1.0).abs() < 2e-3, "centre {}", average(0.0));
        for k in 1..6 {
            let v = average(f64::from(k));
            assert!(v.abs() < 2e-3, "symbol {k}: {v}");
        }
        // Without the moving average the centres are not ISI-free.
        assert!(table_pulse(&table, 1.0).abs() > 0.01);
    }

    #[test]
    fn apco25_signal_is_generated() {
        let p = C4fmParams {
            pulse: C4fmPulse::Apco25,
            ..C4fmParams::default()
        };
        let s = c4fm(&p, &[0, 3, 0, 3, 1, 2]).unwrap();
        assert_eq!(s.len(), 60);
    }

    #[test]
    fn rejects_huge_or_non_finite_values() {
        let p = C4fmParams::default();
        for sample_rate in [f64::INFINITY, 1e300, f64::NAN, 0.0] {
            let bad = C4fmParams {
                sample_rate,
                ..p.clone()
            };
            assert!(c4fm(&bad, &[0, 1]).is_err(), "rate {sample_rate}");
        }
        let loud = C4fmParams {
            amplitude: f64::NAN,
            ..p
        };
        assert!(c4fm(&loud, &[0]).is_err());
        let long = vec![0; (crate::MAX_SAMPLES / 10) + 1];
        assert!(c4fm(&C4fmParams::default(), &long).is_err());
    }

    #[test]
    fn dibit_levels_follow_apco25() {
        let levels: Vec<usize> = [0b01, 0b00, 0b10, 0b11]
            .iter()
            .map(|d| c4fm_level_of_dibit(*d))
            .collect();
        assert_eq!(levels, vec![3, 2, 1, 0]);
    }

    #[test]
    fn length_and_validation() {
        let p = C4fmParams::default();
        let s = c4fm(&p, &[0, 1, 2, 3, 2, 1]).unwrap();
        assert_eq!(s.len(), 60);
        assert!(c4fm(&p, &[4]).is_err());
        assert!(c4fm(&p, &[]).is_err());
        let low = C4fmParams {
            carrier: 1000.0,
            ..C4fmParams::default()
        };
        assert!(c4fm(&low, &[0]).is_err());
    }
}
