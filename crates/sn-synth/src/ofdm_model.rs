//! OFDM signal models A, B and C (the OCG models SA analyses, report 11.6).
//!
//! SA and OCG never published exact definitions of the three models. This module
//! implements our reading of SA's descriptions, chosen so that each model is recovered
//! by the matching view of `sn-ofdm` (Absolute, Relative, PilotReferenced):
//!
//! - **Model A** ("every channel generated as is"): every channel's value is absolute
//!   at its native bin: the data point itself (pi/4-DQPSK keeps its running phase, as
//!   in [`crate::ofdm`], and its data point is that absolute point on the 8-point
//!   pi/4 grid; pilots are the constant 1). The absolute view recovers it.
//! - **Model B** ("every channel can act as a pilot"): every channel is differentially
//!   encoded from symbol to symbol, so each channel is its own phase reference. With
//!   data point `d_k`, the value sent is `x_k = d_k exp(j arg x_{k-1})`, starting from a
//!   random phase per channel, uniform over the circle (so the absolute view fails
//!   also for PSK). Then `x_k conj(x_{k-1}) / |x_{k-1}|` is exactly `d_k`, which is
//!   what the relative view computes, also for QAM. Pilots carry `d = 1` (a constant
//!   phase). For pi/4-DQPSK `d_k` is the phase step.
//! - **Model C** (pilots in model B, data in model A): pilot channels carry a known
//!   reference and data channels are absolute relative to the pilots' common phase
//!   `phi_k`: pilots send `exp(j phi_k)`, data send `d_k exp(j phi_k)`. So
//!   `data conj(pilot) / |pilot|` is `d_k`, which is what the pilot-referenced view
//!   computes. For pi/4-DQPSK, as in model A, `d_k` is the absolute running point on
//!   the 8-point pi/4 grid (its steps carry the data). With [`PilotReference::Constant`] `phi_k = 0` (the pilots are the
//!   constant 1); with [`PilotReference::Differential`] `phi_k` advances by a random
//!   step, uniform over the circle, every symbol (the same on all pilots, recorded in
//!   [`OfdmModelOutput::common_phase`]), so the absolute view fails for every
//!   constellation (a step on a 45 or 90 degree grid would leave PSK and square QAM
//!   looking clean but mislabelled) and only the pilot-referenced (data) and relative
//!   (pilots) views work.
//!
//! Pilots are the positions in [`OfdmModelParams::pilots`] plus any channel whose
//! override (or default modulation) is [`OfdmModulation::Pilot`].

use std::f64::consts::{FRAC_PI_4, TAU};

use rustfft::num_complex::Complex64;
use serde::{Deserialize, Serialize};
use sn_core::{Complex32, Signal};

use crate::ofdm::{check, check_complex_band, check_real_band, fft_indices, synthesize, to_real};
use crate::{
    Constellation, OfdmModulation, OfdmOutput, OfdmParams, Result, Rng, StopCheck, never_stop,
    require,
};

/// Signal model of an OFDM generator. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfdmModel {
    /// Absolute values at the native bins.
    #[default]
    A,
    /// Every channel differential from symbol to symbol.
    B,
    /// Pilots carry the reference; data are absolute relative to the pilots.
    C,
}

impl OfdmModel {
    /// The model's letter.
    pub fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

/// Which channel positions are pilots.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotPattern {
    /// No extra pilots (overrides with [`OfdmModulation::Pilot`] still count).
    #[default]
    None,
    /// These channel positions (0-based, counted from `first_bin`).
    Positions(Vec<usize>),
    /// Positions `offset`, `offset + step`, `offset + 2 step`, ... (unused positions
    /// are skipped).
    Every {
        /// Distance between pilots, at least 1.
        step: usize,
        /// First pilot position.
        offset: usize,
    },
}

/// What the pilots carry in model C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotReference {
    /// The constant 1: the common phase stays 0.
    #[default]
    Constant,
    /// A common phase that advances by a random step (uniform over the circle) every
    /// symbol.
    Differential,
}

/// Parameters of a model A, B or C OFDM signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfdmModelParams {
    /// Grid, timing, level, modulation and seed, as for [`crate::ofdm`].
    pub base: OfdmParams,
    /// Signal model.
    pub model: OfdmModel,
    /// Extra pilot positions.
    pub pilots: PilotPattern,
    /// What the pilots carry in model C (ignored in A and B).
    pub pilot_reference: PilotReference,
}

/// A generated model OFDM signal and its ground truth.
#[derive(Debug, Clone)]
pub struct OfdmModelOutput {
    /// Signal, active bins and the unit-power values sent (`output.symbols[k][c]`).
    pub output: OfdmOutput,
    /// `data[k][c]`: the data point of active channel `c` in symbol `k` (1 for pilots).
    /// For pi/4-DQPSK it is the phase step in model B and the absolute running point
    /// `exp(j m pi/4)` in models A and C. The model's own view (absolute for A,
    /// relative for B, pilot-referenced for C) recovers exactly these.
    pub data: Vec<Vec<Complex32>>,
    /// `data_indices[k][c]`: the symbol index of `data[k][c]` in the channel's
    /// constellation ([`OfdmModulation::constellation`]); 0 for pilots. For pi/4-DQPSK
    /// in models A and C it is `m` (0 to 7) of the absolute point `exp(j m pi/4)`.
    pub data_indices: Vec<Vec<u8>>,
    /// Bins of the pilot channels.
    pub pilot_bins: Vec<i64>,
    /// Common phase `phi_k` of every symbol in radians (all 0 except model C with
    /// differential pilots).
    pub common_phase: Vec<f64>,
}

impl OfdmModelParams {
    /// One flag per channel position, true for pilots. Checks the pilot list.
    fn pilot_mask(&self) -> Result<Vec<bool>> {
        let b = &self.base;
        let active = b.active_mask();
        let table = b.modulation_table();
        let mut mask: Vec<bool> = table.iter().map(|m| *m == OfdmModulation::Pilot).collect();
        match &self.pilots {
            PilotPattern::None => {}
            PilotPattern::Positions(list) => {
                for c in list {
                    let ok = active.get(*c).copied().unwrap_or(false);
                    require(
                        ok,
                        "pilots",
                        format!(
                            "position {c} is not an active channel (positions 0..{}, minus \
                             the unused ones)",
                            b.channels
                        ),
                    )?;
                    if let Some(m) = mask.get_mut(*c) {
                        *m = true;
                    }
                }
            }
            PilotPattern::Every { step, offset } => {
                require(*step >= 1, "pilots", "the pilot step must be at least 1")?;
                require(
                    *offset < b.channels,
                    "pilots",
                    format!(
                        "the first pilot position {offset} is outside 0..{}",
                        b.channels
                    ),
                )?;
                for c in (*offset..b.channels).step_by(*step) {
                    if active.get(c).copied().unwrap_or(false)
                        && let Some(m) = mask.get_mut(c)
                    {
                        *m = true;
                    }
                }
            }
        }
        Ok(mask)
    }

    /// Bins of the pilot channels, in order.
    pub fn pilot_bins(&self) -> Result<Vec<i64>> {
        check(&self.base)?;
        let mask = self.pilot_mask()?;
        Ok(pilot_bins_of(&self.base, &mask))
    }

    /// Check every field.
    pub fn validate(&self) -> Result<()> {
        self.checked_mask().map(drop)
    }

    /// Check every field and return the pilot mask ([`Self::pilot_mask`]).
    fn checked_mask(&self) -> Result<Vec<bool>> {
        check(&self.base)?;
        let mask = self.pilot_mask()?;
        if self.model == OfdmModel::C {
            let active = self.base.active_mask();
            let pilots = mask.iter().zip(&active).filter(|(p, a)| **p && **a).count();
            require(
                pilots > 0,
                "pilots",
                "model C needs at least one pilot channel; list pilot positions or use \
                 model A or B",
            )?;
        }
        Ok(mask)
    }
}

/// Bins of the active channels that `mask` marks as pilots, in order.
fn pilot_bins_of(b: &OfdmParams, mask: &[bool]) -> Vec<i64> {
    b.active_positions()
        .into_iter()
        .filter(|c| mask.get(*c).copied().unwrap_or(false))
        .map(|c| b.first_bin.saturating_add(c as i64))
        .collect()
}

/// What each active channel carries: `None` for a pilot.
fn channel_kinds(p: &OfdmModelParams, mask: &[bool]) -> Vec<Option<Constellation>> {
    let table = p.base.modulation_table();
    p.base
        .active_positions()
        .into_iter()
        .map(|c| {
            if mask.get(c).copied().unwrap_or(false) {
                None
            } else {
                table
                    .get(c)
                    .copied()
                    .unwrap_or(p.base.modulation)
                    .constellation()
            }
        })
        .collect()
}

/// Per-channel state while generating.
struct Channel {
    kind: Option<Constellation>,
    points: Vec<Complex64>,
    /// Running phase (model B reference, or the pi/4-DQPSK phase).
    phase: f64,
}

/// Index `m` (0 to 7) of the point `exp(j m pi/4)` nearest to `phase`.
fn pi4_grid_index(phase: f64) -> u8 {
    ((phase / FRAC_PI_4).round().rem_euclid(8.0)) as u8
}

/// What [`build`] makes, before the samples become a signal.
struct Built {
    samples: Vec<Complex32>,
    sent: Vec<Vec<Complex32>>,
    data: Vec<Vec<Complex32>>,
    data_indices: Vec<Vec<u8>>,
    common_phase: Vec<f64>,
    pilot_bins: Vec<i64>,
}

/// Generate the complex samples and the ground truth. `mask` comes from
/// [`OfdmModelParams::checked_mask`].
fn build(p: &OfdmModelParams, mask: &[bool], stop: StopCheck<'_>) -> Result<Built> {
    let b = &p.base;
    let mut rng = Rng::new(b.seed);
    let mut channels: Vec<Channel> = channel_kinds(p, mask)
        .into_iter()
        .map(|kind| Channel {
            kind,
            points: kind.map(Constellation::points).unwrap_or_default(),
            phase: 0.0,
        })
        .collect();
    // The indices are stored as u8, which holds every constellation (at most 256).
    if let Some(ch) = channels.iter().find(|c| c.points.len() > 256) {
        return Err(crate::Error::InvalidParameter {
            name: "modulation",
            reason: format!("{} points are more than 256", ch.points.len()),
        });
    }
    for ch in &mut channels {
        let u = rng.uniform();
        ch.phase = if p.model == OfdmModel::B {
            // Uniform, so the absolute view of PSK does not stay on a grid.
            u * TAU
        } else {
            // On the pi/4 grid, so pi/4-DQPSK points stay on it.
            (u * 8.0).floor() * FRAC_PI_4
        };
    }
    let mut data = Vec::with_capacity(b.symbols);
    let mut data_indices = Vec::with_capacity(b.symbols);
    let mut common_phase = Vec::with_capacity(b.symbols);
    let mut phi = 0.0_f64;
    let differential_pilots =
        p.model == OfdmModel::C && p.pilot_reference == PilotReference::Differential;
    let (samples, sent) = synthesize(
        b,
        &fft_indices(b),
        || {
            if differential_pilots {
                phi = (phi + rng.uniform() * TAU).rem_euclid(TAU);
            }
            common_phase.push(phi);
            let mut d_row = Vec::with_capacity(channels.len());
            let mut i_row = Vec::with_capacity(channels.len());
            let row = channels
                .iter_mut()
                .map(|ch| {
                    let (idx, d) = if ch.kind.is_some() {
                        let i = rng.below(ch.points.len());
                        (i, ch.points.get(i).copied().unwrap_or_default())
                    } else {
                        (0, Complex64::new(1.0, 0.0))
                    };
                    // Checked above: every index fits in u8.
                    let idx = u8::try_from(idx).unwrap_or(u8::MAX);
                    let pi4 = ch.kind == Some(Constellation::Pi4Dqpsk);
                    // The value sent, and the data point with its index that the
                    // model's view recovers.
                    let (x, truth, truth_idx) = match p.model {
                        OfdmModel::A | OfdmModel::C if pi4 => {
                            ch.phase = (ch.phase + d.arg()).rem_euclid(TAU);
                            let m = pi4_grid_index(ch.phase);
                            let point = Complex64::from_polar(1.0, f64::from(m) * FRAC_PI_4);
                            let x = if p.model == OfdmModel::C {
                                Complex64::from_polar(1.0, ch.phase + phi)
                            } else {
                                Complex64::from_polar(1.0, ch.phase)
                            };
                            (x, point, m)
                        }
                        OfdmModel::A => (d, d, idx),
                        OfdmModel::B => {
                            ch.phase = (ch.phase + d.arg()).rem_euclid(TAU);
                            (Complex64::from_polar(d.norm(), ch.phase), d, idx)
                        }
                        OfdmModel::C => (d * Complex64::from_polar(1.0, phi), d, idx),
                    };
                    d_row.push(Complex32::new(truth.re as f32, truth.im as f32));
                    i_row.push(truth_idx);
                    (x.re, x.im)
                })
                .collect();
            data.push(d_row);
            data_indices.push(i_row);
            Ok(row)
        },
        stop,
    )?;
    Ok(Built {
        samples,
        sent,
        data,
        data_indices,
        common_phase,
        pilot_bins: pilot_bins_of(b, mask),
    })
}

/// Generate model OFDM: complex when `center` is `None`, otherwise real with bin 0 on
/// `center` Hz. Calls `stop` now and then.
pub(crate) fn ofdm_model_with_stop(
    params: &OfdmModelParams,
    center: Option<f64>,
    stop: StopCheck<'_>,
) -> Result<OfdmModelOutput> {
    let mask = params.checked_mask()?;
    match center {
        None => check_complex_band(&params.base)?,
        Some(c) => check_real_band(&params.base, c)?,
    }
    let built = build(params, &mask, stop)?;
    let fs = params.base.sample_rate;
    let signal = match center {
        None => Signal::complex(built.samples, fs)?,
        Some(c) => to_real(&built.samples, fs, c)?,
    };
    Ok(OfdmModelOutput {
        output: OfdmOutput {
            signal,
            bins: params.base.active_bins(),
            symbols: built.sent,
        },
        data: built.data,
        data_indices: built.data_indices,
        pilot_bins: built.pilot_bins,
        common_phase: built.common_phase,
    })
}

/// Generate complex (I/Q) OFDM in model A, B or C. The same band rules as
/// [`crate::ofdm`] apply.
pub fn ofdm_model(params: &OfdmModelParams) -> Result<OfdmModelOutput> {
    ofdm_model_with_stop(params, None, &never_stop)
}

/// Generate real OFDM in model A, B or C with bin 0 on `center` Hz. The same band rules
/// as [`crate::ofdm_real`] apply.
pub fn ofdm_model_real(params: &OfdmModelParams, center: f64) -> Result<OfdmModelOutput> {
    ofdm_model_with_stop(params, Some(center), &never_stop)
}

#[cfg(test)]
mod tests {
    use rustfft::FftPlanner;
    use sn_core::Samples;

    use super::*;

    fn base(modulation: OfdmModulation) -> OfdmParams {
        OfdmParams {
            sample_rate: 7000.0,
            lu: 64,
            lg: 20,
            channels: 24,
            first_bin: -12,
            unused: vec![3],
            modulation,
            overrides: vec![],
            symbols: 40,
            frequency_offset: 0.0,
            amplitude: 0.5,
            seed: 11,
        }
    }

    /// The unit-power value of every active channel in every symbol, read from the
    /// signal by an FFT of each useful part.
    fn demod(o: &OfdmModelOutput, p: &OfdmParams) -> Vec<Vec<Complex64>> {
        let Samples::Complex(x) = o.output.signal.samples() else {
            panic!("expected complex")
        };
        let fft = FftPlanner::<f64>::new().plan_fft_forward(p.lu);
        let gain = p.amplitude / (o.output.bins.len() as f64).sqrt() * p.lu as f64;
        (0..p.symbols)
            .map(|k| {
                let s = k * p.ls() + p.lg;
                let mut buf: Vec<Complex64> = x[s..s + p.lu]
                    .iter()
                    .map(|v| Complex64::new(f64::from(v.re), f64::from(v.im)))
                    .collect();
                fft.process(&mut buf);
                o.output
                    .bins
                    .iter()
                    .map(|b| buf[b.rem_euclid(p.lu as i64) as usize] / gain)
                    .collect()
            })
            .collect()
    }

    fn on_point(z: Complex64, c: Constellation, idx: u8) -> bool {
        (z - c.points()[usize::from(idx)]).norm() < 1e-3
    }

    fn off_constellation_fraction(v: &[Vec<Complex64>], c: Constellation) -> f64 {
        let pts = c.points();
        let all: Vec<&Complex64> = v.iter().flatten().collect();
        let off = all
            .iter()
            .filter(|z| pts.iter().all(|p| (**z - p).norm() > 0.05))
            .count();
        off as f64 / all.len() as f64
    }

    #[test]
    fn model_a_values_are_absolute() {
        for m in [
            OfdmModulation::Bpsk,
            OfdmModulation::Qpsk,
            OfdmModulation::Psk8,
            OfdmModulation::Qam16,
            OfdmModulation::Qam64,
        ] {
            let p = OfdmModelParams {
                base: base(m),
                model: OfdmModel::A,
                pilots: PilotPattern::None,
                pilot_reference: PilotReference::Constant,
            };
            let o = ofdm_model(&p).unwrap();
            let c = m.constellation().unwrap();
            let v = demod(&o, &p.base);
            for (k, row) in v.iter().enumerate() {
                for (ch, z) in row.iter().enumerate() {
                    assert!(on_point(*z, c, o.data_indices[k][ch]), "{m:?} {k} {ch}");
                }
            }
        }
    }

    #[test]
    fn model_b_relative_values_are_on_the_constellation() {
        for m in [
            OfdmModulation::Qpsk,
            OfdmModulation::Psk8,
            OfdmModulation::Qam16,
            OfdmModulation::Pi4Dqpsk,
        ] {
            let p = OfdmModelParams {
                base: base(m),
                model: OfdmModel::B,
                pilots: PilotPattern::Positions(vec![0]),
                pilot_reference: PilotReference::Constant,
            };
            let o = ofdm_model(&p).unwrap();
            let c = m.constellation().unwrap();
            let v = demod(&o, &p.base);
            for k in 1..v.len() {
                for ch in 0..v[k].len() {
                    let r = v[k][ch] * v[k - 1][ch].conj() / v[k - 1][ch].norm();
                    if ch == 0 {
                        assert!((r - Complex64::new(1.0, 0.0)).norm() < 1e-3, "pilot");
                    } else {
                        assert!(on_point(r, c, o.data_indices[k][ch]), "{m:?} {k} {ch}");
                    }
                }
            }
            if m != OfdmModulation::Pi4Dqpsk {
                // The absolute values are rotated off the grid, PSK included.
                let data: Vec<Vec<Complex64>> = v.iter().map(|r| r[1..].to_vec()).collect();
                let off = off_constellation_fraction(&data, c);
                assert!(off > 0.6, "{m:?}: only {off} off the constellation");
            }
        }
    }

    #[test]
    fn model_c_pilot_referenced_values_are_on_the_constellation() {
        let p = OfdmModelParams {
            base: base(OfdmModulation::Qam16),
            model: OfdmModel::C,
            pilots: PilotPattern::Every { step: 6, offset: 0 },
            pilot_reference: PilotReference::Differential,
        };
        let o = ofdm_model(&p).unwrap();
        // Positions 0, 6, 12, 18 are pilots; position 3 is unused.
        assert_eq!(o.pilot_bins, vec![-12, -6, 0, 6]);
        let pilot_cols: Vec<usize> = o
            .output
            .bins
            .iter()
            .enumerate()
            .filter(|(_, b)| o.pilot_bins.contains(b))
            .map(|(i, _)| i)
            .collect();
        let v = demod(&o, &p.base);
        let c = Constellation::Qam16;
        let mut absolute = Vec::new();
        for (k, row) in v.iter().enumerate() {
            let pilot = row[pilot_cols[0]];
            for (ch, z) in row.iter().enumerate() {
                if pilot_cols.contains(&ch) {
                    // Every pilot carries the same common phase.
                    assert!((z - pilot).norm() < 1e-3);
                    continue;
                }
                let r = z * pilot.conj() / pilot.norm();
                assert!(on_point(r, c, o.data_indices[k][ch]), "{k} {ch}");
                absolute.push(*z);
            }
            // The pilot phase is the recorded common phase.
            let err = (pilot.arg() - o.common_phase[k] + std::f64::consts::PI).rem_euclid(TAU)
                - std::f64::consts::PI;
            assert!(err.abs() < 1e-3, "pilot phase {k}: {err}");
        }
        assert!(o.common_phase.iter().any(|p| *p != 0.0));
        // Most absolute values sit off the constellation (rotated by phi_k).
        assert!(off_constellation_fraction(&[absolute], c) > 0.3);
    }

    #[test]
    fn model_c_pi4_dqpsk_truth_is_what_the_pilot_view_recovers() {
        for reference in [PilotReference::Constant, PilotReference::Differential] {
            let p = OfdmModelParams {
                base: base(OfdmModulation::Pi4Dqpsk),
                model: OfdmModel::C,
                pilots: PilotPattern::Positions(vec![0]),
                pilot_reference: reference,
            };
            let o = ofdm_model(&p).unwrap();
            let v = demod(&o, &p.base);
            for (k, row) in v.iter().enumerate() {
                let pilot = row[0];
                for (ch, z) in row.iter().enumerate().skip(1) {
                    let r = z * pilot.conj() / pilot.norm();
                    let want = o.data[k][ch];
                    let want = Complex64::new(f64::from(want.re), f64::from(want.im));
                    assert!((r - want).norm() < 1e-3, "{reference:?} {k} {ch}");
                    let m = f64::from(o.data_indices[k][ch]);
                    assert!((want - Complex64::from_polar(1.0, m * FRAC_PI_4)).norm() < 1e-6);
                }
            }
        }
        // Model A: the absolute view recovers the same kind of point.
        let p = OfdmModelParams {
            base: base(OfdmModulation::Pi4Dqpsk),
            model: OfdmModel::A,
            pilots: PilotPattern::None,
            pilot_reference: PilotReference::Constant,
        };
        let o = ofdm_model(&p).unwrap();
        for (k, row) in demod(&o, &p.base).iter().enumerate() {
            for (ch, z) in row.iter().enumerate() {
                let want = o.data[k][ch];
                let want = Complex64::new(f64::from(want.re), f64::from(want.im));
                assert!((z - want).norm() < 1e-3, "model A {k} {ch}");
            }
        }
    }

    #[test]
    fn model_c_with_constant_pilots_is_absolute() {
        let p = OfdmModelParams {
            base: base(OfdmModulation::Psk8),
            model: OfdmModel::C,
            pilots: PilotPattern::Positions(vec![5]),
            pilot_reference: PilotReference::Constant,
        };
        let o = ofdm_model(&p).unwrap();
        let v = demod(&o, &p.base);
        for (k, row) in v.iter().enumerate() {
            for (ch, z) in row.iter().enumerate() {
                let c = if o.output.bins[ch] == -7 {
                    assert!((z - Complex64::new(1.0, 0.0)).norm() < 1e-3);
                    continue;
                } else {
                    Constellation::Psk8
                };
                assert!(on_point(*z, c, o.data_indices[k][ch]));
            }
        }
    }

    #[test]
    fn validation_and_real_output() {
        let mut p = OfdmModelParams {
            base: base(OfdmModulation::Qpsk),
            model: OfdmModel::C,
            pilots: PilotPattern::None,
            pilot_reference: PilotReference::Constant,
        };
        assert!(ofdm_model(&p).unwrap_err().to_string().contains("pilot"));
        p.pilots = PilotPattern::Positions(vec![3]); // unused
        assert!(ofdm_model(&p).is_err());
        p.pilots = PilotPattern::Positions(vec![24]);
        assert!(ofdm_model(&p).is_err());
        p.pilots = PilotPattern::Every { step: 0, offset: 0 };
        assert!(ofdm_model(&p).is_err());
        p.pilots = PilotPattern::Every { step: 4, offset: 1 };
        let o = ofdm_model_real(&p, 1750.0).unwrap();
        assert_eq!(o.output.signal.kind(), sn_core::SampleKind::Real);
        assert_eq!(o.output.signal.len(), 40 * 84);
        assert!(ofdm_model_real(&p, 100.0).is_err());
    }

    #[test]
    fn same_seed_same_signal() {
        let p = OfdmModelParams {
            base: base(OfdmModulation::Qam64),
            model: OfdmModel::B,
            pilots: PilotPattern::None,
            pilot_reference: PilotReference::Constant,
        };
        let a = ofdm_model(&p).unwrap();
        let b = ofdm_model(&p).unwrap();
        let (Samples::Complex(x), Samples::Complex(y)) =
            (a.output.signal.samples(), b.output.signal.samples())
        else {
            panic!("expected complex")
        };
        assert_eq!(x, y);
    }
}
