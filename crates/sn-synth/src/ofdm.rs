//! OFDM with a cyclic prefix (the role of SA's OCG OFDM models).
//!
//! Each symbol is the inverse DFT of `lu` bins, of which `channels` consecutive bins
//! (starting at `first_bin`, minus any listed as unused) carry data. The last `lg`
//! samples of the useful part are copied in front as the cyclic prefix, so a symbol is
//! `ls = lu + lg` samples long and the symbol rate is `sample_rate / ls`.
//!
//! The phase reference of every channel is the first sample of the useful part (the
//! sample right after the prefix), and the frequency offset is applied as a continuous
//! rotation over the whole signal.

use rustfft::{FftPlanner, num_complex::Complex64};
use serde::{Deserialize, Serialize};
use sn_core::{Complex32, Signal};

use crate::{Constellation, MAX_SAMPLES, Result, Rng, StopCheck, check_stop, never_stop, require};

/// Largest useful-part length (FFT size) LU the OFDM generator accepts, in samples
/// (the smallest is 2).
pub const OFDM_MAX_LU: usize = 1 << 16;
/// Largest guard (cyclic prefix) length LG the OFDM generator accepts, in samples.
pub const OFDM_MAX_LG: usize = (1 << 16) - 1;

/// How one channel is modulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfdmModulation {
    /// Absolute QPSK at 45, 135, 225 and 315 degrees.
    Qpsk,
    /// pi/4-DQPSK: each symbol advances the phase by 45 + m * 90 degrees.
    Pi4Dqpsk,
    /// Square 16-QAM with unit mean power.
    Qam16,
    /// An unmodulated pilot (constant value 1).
    Pilot,
    /// BPSK at 0 and 180 degrees.
    Bpsk,
    /// 8-PSK from 0 degrees.
    Psk8,
    /// Square 64-QAM with unit mean power.
    Qam64,
}

impl OfdmModulation {
    /// Short English name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Qpsk => "QPSK",
            Self::Pi4Dqpsk => "pi/4-DQPSK",
            Self::Qam16 => "QAM-16",
            Self::Pilot => "pilot",
            Self::Bpsk => "BPSK",
            Self::Psk8 => "8-PSK",
            Self::Qam64 => "QAM-64",
        }
    }

    /// The constellation the data are drawn from in the model generator
    /// ([`crate::ofdm_model`]); `None` for a pilot. For pi/4-DQPSK the points are the four
    /// phase steps.
    pub fn constellation(self) -> Option<Constellation> {
        match self {
            Self::Qpsk => Some(Constellation::Qpsk),
            Self::Pi4Dqpsk => Some(Constellation::Pi4Dqpsk),
            Self::Qam16 => Some(Constellation::Qam16),
            Self::Pilot => None,
            Self::Bpsk => Some(Constellation::Bpsk),
            Self::Psk8 => Some(Constellation::Psk8),
            Self::Qam64 => Some(Constellation::Qam64),
        }
    }
}

/// Parameters of an OFDM signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfdmParams {
    /// Sample rate in Hz (the native rate: `lu` and `lg` are counted at this rate).
    pub sample_rate: f64,
    /// Useful symbol length (FFT size) in samples. Odd values are allowed.
    pub lu: usize,
    /// Cyclic-prefix (guard) length in samples.
    pub lg: usize,
    /// Number of consecutive channel positions.
    pub channels: usize,
    /// Bin index of the first channel (may be negative; 0 is the centre frequency).
    pub first_bin: i64,
    /// Channel positions (0-based, counted from `first_bin`) that carry nothing.
    pub unused: Vec<usize>,
    /// Modulation of every channel.
    pub modulation: OfdmModulation,
    /// Per-channel overrides: (channel position, modulation).
    pub overrides: Vec<(usize, OfdmModulation)>,
    /// Number of symbols.
    pub symbols: usize,
    /// Frequency offset in Hz applied to the whole signal.
    pub frequency_offset: f64,
    /// RMS amplitude of the output.
    pub amplitude: f64,
    /// Seed for the data.
    pub seed: u64,
}

impl OfdmParams {
    /// Symbol length `lu + lg` in samples.
    pub fn ls(&self) -> usize {
        self.lu + self.lg
    }

    /// Symbol rate in baud.
    pub fn symbol_rate(&self) -> f64 {
        self.sample_rate / self.ls() as f64
    }

    /// Channel spacing in Hz.
    pub fn spacing(&self) -> f64 {
        self.sample_rate / self.lu as f64
    }

    /// Bin indices of the channels that carry something, in order.
    pub fn active_bins(&self) -> Vec<i64> {
        self.active_positions()
            .into_iter()
            .map(|c| self.first_bin.saturating_add(c as i64))
            .collect()
    }

    /// One flag per channel position (0..channels), false where the position is listed
    /// in `unused`. Built from one pass over `unused`, so it is linear in the channel
    /// count plus the list length.
    pub(crate) fn active_mask(&self) -> Vec<bool> {
        let mut active = vec![true; self.channels];
        for u in &self.unused {
            if let Some(a) = active.get_mut(*u) {
                *a = false;
            }
        }
        active
    }

    /// Positions (0..channels) of the channels that carry something, in order.
    pub(crate) fn active_positions(&self) -> Vec<usize> {
        self.active_mask()
            .iter()
            .enumerate()
            .filter_map(|(c, a)| a.then_some(c))
            .collect()
    }

    /// Modulation of every channel position (0..channels): `modulation` unless an
    /// override names the position, in which case the last such override wins. Built
    /// from one pass over `overrides`, so it is linear in the channel count plus the
    /// list length.
    pub(crate) fn modulation_table(&self) -> Vec<OfdmModulation> {
        let mut table = vec![self.modulation; self.channels];
        for (c, m) in &self.overrides {
            if let Some(t) = table.get_mut(*c) {
                *t = *m;
            }
        }
        table
    }
}

/// A generated OFDM signal and the values that went into it.
#[derive(Debug, Clone)]
pub struct OfdmOutput {
    /// The signal.
    pub signal: Signal,
    /// Bin index of each active channel (same order as the inner vectors of `symbols`).
    pub bins: Vec<i64>,
    /// `symbols[k][c]`: the unit-power value sent on active channel `c` in symbol `k`.
    pub symbols: Vec<Vec<Complex32>>,
}

/// Generate complex (I/Q) OFDM with a cyclic prefix.
///
/// Every channel, after the frequency offset, must stay inside -fs/2..fs/2 so the band
/// does not wrap around to the other side of the spectrum.
pub fn ofdm(params: &OfdmParams) -> Result<OfdmOutput> {
    check(params)?;
    check_complex_band(params)?;
    let (values, bins, symbols) = generate(params)?;
    let signal = Signal::complex(values, params.sample_rate)?;
    Ok(OfdmOutput {
        signal,
        bins,
        symbols,
    })
}

/// Generate real OFDM with the band centred on `center` Hz (bin 0 lands on `center`).
/// The output has the same RMS level as the complex version.
pub fn ofdm_real(params: &OfdmParams, center: f64) -> Result<OfdmOutput> {
    let p = params;
    check(p)?;
    check_real_band(p, center)?;
    let (values, bins, symbols) = generate(p)?;
    let signal = to_real(&values, p.sample_rate, center)?;
    Ok(OfdmOutput {
        signal,
        bins,
        symbols,
    })
}

/// Require the channels, after the frequency offset, to stay inside -fs/2..fs/2.
pub(crate) fn check_complex_band(p: &OfdmParams) -> Result<()> {
    let half = p.sample_rate / 2.0;
    let lo = p.first_bin as f64 * p.spacing() + p.frequency_offset;
    let hi = last_bin(p) as f64 * p.spacing() + p.frequency_offset;
    if lo < -half || hi >= half {
        return Err(crate::Error::InvalidParameter {
            name: "frequency_offset",
            reason: format!(
                "the channels would sit at {lo:.1} to {hi:.1} Hz, outside -{half} to {half} \
                 Hz, and wrap around the spectrum; use a smaller offset or fewer channels"
            ),
        });
    }
    Ok(())
}

/// Require a finite `center` and the band, centred on it, to lie between 0 Hz and
/// Nyquist.
pub(crate) fn check_real_band(p: &OfdmParams, center: f64) -> Result<()> {
    require(center.is_finite(), "center", "must be a finite frequency")?;
    let lo = (p.first_bin as f64 - 1.0) * p.spacing() + p.frequency_offset + center;
    let hi = (last_bin(p) + 1) as f64 * p.spacing() + p.frequency_offset + center;
    require(
        lo > 0.0 && hi < p.sample_rate / 2.0,
        "center",
        format!(
            "the band would span {lo:.1} to {hi:.1} Hz, which must lie between 0 Hz and \
             Nyquist ({} Hz); move the centre or use fewer channels",
            p.sample_rate / 2.0
        ),
    )
}

/// Shift complex OFDM up to `center` Hz and keep the real part, at the same RMS level.
pub(crate) fn to_real(values: &[Complex32], sample_rate: f64, center: f64) -> Result<Signal> {
    let w = center / sample_rate;
    let real: Vec<f32> = values
        .iter()
        .enumerate()
        .map(|(n, v)| {
            let ph = std::f64::consts::TAU * (n as f64 * w).rem_euclid(1.0);
            let (s, c) = ph.sin_cos();
            (std::f64::consts::SQRT_2 * (f64::from(v.re) * c - f64::from(v.im) * s)) as f32
        })
        .collect();
    Ok(Signal::real(real, sample_rate)?)
}

pub(crate) fn check(p: &OfdmParams) -> Result<()> {
    require(
        p.sample_rate.is_finite() && p.sample_rate > 0.0,
        "sample_rate",
        "must be a finite number above 0 Hz",
    )?;
    require(
        (2..=OFDM_MAX_LU).contains(&p.lu),
        "lu",
        format!("must be between 2 and {OFDM_MAX_LU} samples"),
    )?;
    require(
        p.lg <= OFDM_MAX_LG,
        "lg",
        format!("must be at most {OFDM_MAX_LG} samples"),
    )?;
    require(p.channels > 0, "channels", "at least one channel is needed")?;
    require(
        p.channels <= p.lu,
        "channels",
        format!(
            "{} channels do not fit in an FFT of {} bins",
            p.channels, p.lu
        ),
    )?;
    let half = p.lu as i64 / 2;
    let top = p.lu as i64 - half - 1;
    // Bound first_bin on its own first, so the last-bin sum below cannot overflow.
    let last = if p.first_bin >= -half && p.first_bin <= top {
        p.first_bin.checked_add(p.channels as i64 - 1)
    } else {
        None
    };
    if last.is_none_or(|l| l > top) {
        return Err(crate::Error::InvalidParameter {
            name: "first_bin",
            reason: format!(
                "{} channels starting at bin {} do not fit in the bins -{half}..{top} of an \
                 FFT of {}",
                p.channels, p.first_bin, p.lu
            ),
        });
    }
    require(p.symbols > 0, "symbols", "at least one symbol is needed")?;
    require(
        p.symbols.saturating_mul(p.ls()) <= MAX_SAMPLES,
        "symbols",
        "the signal would exceed 2^28 samples; generate fewer symbols",
    )?;
    require(
        p.amplitude.is_finite() && p.amplitude >= 0.0,
        "amplitude",
        "must be a finite number of at least 0",
    )?;
    require(
        p.frequency_offset.is_finite(),
        "frequency_offset",
        "must be finite",
    )?;
    if let Some(u) = p.unused.iter().find(|u| **u >= p.channels) {
        return Err(crate::Error::InvalidParameter {
            name: "unused",
            reason: format!(
                "position {u} is outside the channel positions 0..{}",
                p.channels - 1
            ),
        });
    }
    if let Some((c, _)) = p.overrides.iter().find(|(c, _)| *c >= p.channels) {
        return Err(crate::Error::InvalidParameter {
            name: "overrides",
            reason: format!(
                "position {c} is outside the channel positions 0..{}",
                p.channels - 1
            ),
        });
    }
    // One mask for both remaining checks, so they stay linear in the list lengths.
    let active = p.active_mask();
    if let Some((c, _)) = p
        .overrides
        .iter()
        .find(|(c, _)| !active.get(*c).copied().unwrap_or(true))
    {
        return Err(crate::Error::InvalidParameter {
            name: "overrides",
            reason: format!(
                "position {c} is also listed as unused, so its modulation would be ignored; \
                 remove it from one of the two lists"
            ),
        });
    }
    require(
        active.iter().any(|a| *a),
        "unused",
        "every channel is marked unused; leave at least one active",
    )?;
    Ok(())
}

/// Bin index of the last channel position. Only valid after [`check`].
pub(crate) fn last_bin(p: &OfdmParams) -> i64 {
    p.first_bin.saturating_add(p.channels as i64 - 1)
}

type Generated = (Vec<Complex32>, Vec<i64>, Vec<Vec<Complex32>>);

/// FFT index of every active channel (bin b sits at index b mod lu), in order.
pub(crate) fn fft_indices(p: &OfdmParams) -> Vec<usize> {
    p.active_positions()
        .into_iter()
        .map(|c| {
            let bin = p.first_bin.saturating_add(c as i64);
            bin.rem_euclid(p.lu as i64) as usize
        })
        .collect()
}

/// State of one active channel while generating.
struct ChannelState {
    modulation: OfdmModulation,
    /// Constellation points, built once (only for the modulations that draw from
    /// [`Constellation::points`]).
    points: Vec<Complex64>,
    /// Running pi/4-DQPSK phase.
    phase: f64,
}

fn generate(p: &OfdmParams) -> Result<Generated> {
    check(p)?;
    // Modulation of each active channel.
    let table = p.modulation_table();
    let bins = p.active_bins();
    let mut rng = Rng::new(p.seed);
    // pi/4-DQPSK keeps a running phase per channel (random start).
    let mut channels: Vec<ChannelState> = p
        .active_positions()
        .into_iter()
        .map(|c| {
            let modulation = table.get(c).copied().unwrap_or(p.modulation);
            let points = match modulation {
                OfdmModulation::Bpsk | OfdmModulation::Psk8 | OfdmModulation::Qam64 => modulation
                    .constellation()
                    .map(Constellation::points)
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            ChannelState {
                modulation,
                points,
                phase: 0.0,
            }
        })
        .collect();
    for ch in &mut channels {
        ch.phase = rng.below(8) as f64 * std::f64::consts::FRAC_PI_4;
    }
    let (out, sent) = synthesize(
        p,
        &fft_indices(p),
        || {
            channels
                .iter_mut()
                .map(|ch| symbol_value(ch, &mut rng))
                .collect()
        },
        &never_stop,
    )?;
    Ok((out, bins, sent))
}

/// Build the time signal: `next()` gives the unit-power value of every active channel
/// for one symbol (same order as `indices`), which goes through the inverse DFT, gets
/// its cyclic prefix, and the whole signal is then rotated by the frequency offset.
/// Returns the samples and the values sent, or the first error of `next`, or
/// [`crate::Error::Cancelled`] when `stop` fires. Only valid after [`check`].
pub(crate) fn synthesize(
    p: &OfdmParams,
    indices: &[usize],
    mut next: impl FnMut() -> Result<Vec<(f64, f64)>>,
    stop: StopCheck<'_>,
) -> Result<(Vec<Complex32>, Vec<Vec<Complex32>>)> {
    // Check about every STOP_EVERY samples, counted in whole symbols.
    let symbols_per_check = (crate::STOP_EVERY / p.ls().max(1)).max(1);
    let lu = p.lu;
    let ls = p.ls();
    let ifft = FftPlanner::<f64>::new().plan_fft_inverse(lu);
    let mut scratch = vec![Complex64::new(0.0, 0.0); ifft.get_inplace_scratch_len()];
    let gain = p.amplitude / (indices.len() as f64).sqrt();
    let mut out = Vec::with_capacity(p.symbols * ls);
    let mut sent = Vec::with_capacity(p.symbols);
    let mut useful = vec![Complex64::new(0.0, 0.0); lu];
    for k in 0..p.symbols {
        if k.is_multiple_of(symbols_per_check) {
            check_stop(stop, 0)?;
        }
        let values = next()?;
        // Inverse DFT (unnormalised): useful[n] = gain * sum_b v_b e^{j 2 pi b n / lu}.
        useful.fill(Complex64::new(0.0, 0.0));
        for (idx, (vr, vi)) in indices.iter().zip(&values) {
            if let Some(u) = useful.get_mut(*idx) {
                *u = Complex64::new(vr * gain, vi * gain);
            }
        }
        ifft.process_with_scratch(&mut useful, &mut scratch);
        // Prefix sample i repeats useful sample (i - lg) mod lu, which also covers a
        // prefix longer than the useful part.
        let prefix = (0..p.lg).filter_map(|i| {
            let idx = (i as i64 - p.lg as i64).rem_euclid(lu as i64) as usize;
            useful.get(idx)
        });
        for v in prefix.chain(useful.iter()) {
            out.push(Complex32::new(v.re as f32, v.im as f32));
        }
        sent.push(
            values
                .iter()
                .map(|(r, i)| Complex32::new(*r as f32, *i as f32))
                .collect(),
        );
    }
    if p.frequency_offset != 0.0 {
        let cps = p.frequency_offset / p.sample_rate;
        for (n, v) in out.iter_mut().enumerate() {
            let ph = std::f64::consts::TAU * (n as f64 * cps).rem_euclid(1.0);
            let (s, c) = ph.sin_cos();
            let (re, im) = (f64::from(v.re), f64::from(v.im));
            *v = Complex32::new((re * c - im * s) as f32, (re * s + im * c) as f32);
        }
    }
    Ok((out, sent))
}

fn symbol_value(ch: &mut ChannelState, rng: &mut Rng) -> Result<(f64, f64)> {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};
    let phase = &mut ch.phase;
    Ok(match ch.modulation {
        OfdmModulation::Qpsk => {
            let ph = FRAC_PI_4 + rng.below(4) as f64 * FRAC_PI_2;
            (ph.cos(), ph.sin())
        }
        OfdmModulation::Pi4Dqpsk => {
            *phase = (*phase + FRAC_PI_4 + rng.below(4) as f64 * FRAC_PI_2)
                .rem_euclid(std::f64::consts::TAU);
            (phase.cos(), phase.sin())
        }
        OfdmModulation::Qam16 => {
            let level = |k: usize| (2.0 * k as f64 - 3.0) / 10_f64.sqrt();
            (level(rng.below(4)), level(rng.below(4)))
        }
        OfdmModulation::Pilot => (1.0, 0.0),
        OfdmModulation::Bpsk | OfdmModulation::Psk8 | OfdmModulation::Qam64 => {
            let v = ch
                .points
                .get(rng.below(ch.points.len()))
                .copied()
                .ok_or_else(|| crate::Error::InvalidParameter {
                    name: "modulation",
                    reason: format!("{} has no constellation points", ch.modulation.name()),
                })?;
            (v.re, v.im)
        }
    })
}

#[cfg(test)]
mod tests {
    use sn_core::Samples;

    use super::*;

    fn params() -> OfdmParams {
        OfdmParams {
            sample_rate: 7000.0,
            lu: 64,
            lg: 20,
            channels: 24,
            first_bin: -12,
            unused: vec![3],
            modulation: OfdmModulation::Qam16,
            overrides: vec![(0, OfdmModulation::Pilot)],
            symbols: 50,
            frequency_offset: 0.0,
            amplitude: 0.3,
            seed: 5,
        }
    }

    #[test]
    fn prefix_repeats_the_end_and_bins_carry_the_data() {
        let p = params();
        let o = ofdm(&p).unwrap();
        assert_eq!(o.signal.len(), 50 * 84);
        assert_eq!(o.bins.len(), 23);
        let Samples::Complex(x) = o.signal.samples() else {
            panic!("expected complex")
        };
        for k in 0..50 {
            let s = k * 84;
            for i in 0..20 {
                assert!((x[s + i] - x[s + i + 64]).norm() < 1e-6);
            }
            // DFT of the useful part recovers the sent values.
            for (c, b) in o.bins.iter().enumerate() {
                let mut acc = (0.0, 0.0);
                for n in 0..64 {
                    let ph = -std::f64::consts::TAU * (*b as f64) * n as f64 / 64.0;
                    let v = x[s + 20 + n];
                    acc.0 += f64::from(v.re) * ph.cos() - f64::from(v.im) * ph.sin();
                    acc.1 += f64::from(v.re) * ph.sin() + f64::from(v.im) * ph.cos();
                }
                let g = 0.3 / 23_f64.sqrt() * 64.0;
                let want = o.symbols[k][c];
                assert!((acc.0 / g - f64::from(want.re)).abs() < 1e-4);
                assert!((acc.1 / g - f64::from(want.im)).abs() < 1e-4);
            }
        }
        // Pilot is constant, RMS matches the amplitude.
        assert!(o.symbols.iter().all(|s| s[0] == Complex32::new(1.0, 0.0)));
        let rms = (x.iter().map(|v| f64::from(v.norm_sqr())).sum::<f64>() / x.len() as f64).sqrt();
        assert!((rms - 0.3).abs() < 0.03, "rms {rms}");
    }

    #[test]
    fn dqpsk_steps_are_odd_multiples_of_45_degrees() {
        let p = OfdmParams {
            modulation: OfdmModulation::Pi4Dqpsk,
            overrides: vec![],
            ..params()
        };
        let o = ofdm(&p).unwrap();
        for k in 1..o.symbols.len() {
            for c in 0..o.bins.len() {
                let d = o.symbols[k][c] * o.symbols[k - 1][c].conj();
                let deg = f64::from(d.arg()).to_degrees().rem_euclid(90.0);
                assert!((deg - 45.0).abs() < 1e-3, "step {deg}");
            }
        }
    }

    #[test]
    fn real_output_and_validation() {
        let p = params();
        let o = ofdm_real(&p, 1750.0).unwrap();
        assert_eq!(o.signal.kind(), sn_core::SampleKind::Real);
        assert!(ofdm_real(&p, 300.0).is_err());
        assert!(
            ofdm(&OfdmParams {
                first_bin: 30,
                ..params()
            })
            .is_err()
        );
        assert!(
            ofdm(&OfdmParams { lu: 63, ..params() }).is_ok(),
            "odd LU is allowed"
        );
    }

    #[test]
    fn extreme_first_bin_is_an_error_not_an_overflow() {
        for first_bin in [i64::MAX, i64::MIN, i64::MAX - 10, 31, -33] {
            let p = OfdmParams {
                first_bin,
                ..params()
            };
            assert!(ofdm(&p).is_err(), "first_bin {first_bin}");
            assert!(ofdm_real(&p, 1750.0).is_err(), "first_bin {first_bin}");
            let _ = p.active_bins();
        }
        // The full FFT width is accepted.
        let full = OfdmParams {
            channels: 64,
            first_bin: -32,
            unused: vec![],
            overrides: vec![],
            ..params()
        };
        assert!(ofdm(&full).is_ok());
    }

    #[test]
    fn modulation_table_applies_overrides_last_wins() {
        let p = OfdmParams {
            modulation: OfdmModulation::Qpsk,
            overrides: vec![
                (0, OfdmModulation::Pilot),
                (5, OfdmModulation::Qam16),
                (0, OfdmModulation::Pi4Dqpsk),
                (999, OfdmModulation::Pilot),
            ],
            ..params()
        };
        let t = p.modulation_table();
        assert_eq!(t.len(), p.channels);
        assert_eq!(t[0], OfdmModulation::Pi4Dqpsk);
        assert_eq!(t[5], OfdmModulation::Qam16);
        assert!(
            t.iter()
                .enumerate()
                .filter(|(c, _)| *c != 0 && *c != 5)
                .all(|(_, m)| *m == OfdmModulation::Qpsk)
        );
    }

    #[test]
    fn bad_unused_and_override_positions_are_rejected() {
        let p = params();
        assert!(
            ofdm(&OfdmParams {
                unused: vec![24],
                ..p.clone()
            })
            .is_err()
        );
        assert!(
            ofdm(&OfdmParams {
                overrides: vec![(24, OfdmModulation::Pilot)],
                ..p.clone()
            })
            .is_err()
        );
        assert!(
            ofdm(&OfdmParams {
                overrides: vec![(3, OfdmModulation::Pilot)],
                ..p.clone()
            })
            .is_err(),
            "override on an unused channel"
        );
        assert!(
            ofdm(&OfdmParams {
                unused: (0..24).collect(),
                overrides: vec![],
                ..p
            })
            .is_err()
        );
    }

    #[test]
    fn offset_that_wraps_the_band_is_rejected() {
        // Channels -12..11 at 109.4 Hz spacing span about -1313..1203 Hz; 2400 Hz of
        // offset pushes the top past 3500 Hz.
        let p = OfdmParams {
            frequency_offset: 2400.0,
            ..params()
        };
        assert!(ofdm(&p).is_err());
        let ok = OfdmParams {
            frequency_offset: 1000.0,
            ..params()
        };
        assert!(ofdm(&ok).is_ok());
    }
}
