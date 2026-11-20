//! A serializable description of a test signal, and [`generate`] to make it.
//!
//! A [`GeneratorSpec`] names the kind of signal and its parameters, plus the fields
//! every kind shares ([`Common`]: sample rate, length, seed, label, centre frequency
//! and channel impairments). In JSON the kind is the `"kind"` tag:
//!
//! ```json
//! {
//!   "kind": "linear",
//!   "common": {
//!     "label": "",
//!     "sample_rate": 8000.0,
//!     "length": { "seconds": 10.0 },
//!     "seed": 1,
//!     "center_frequency": 0.0,
//!     "channel": { "noise": { "snr": { "db": 20.0, "bandwidth": null } } }
//!   },
//!   "constellation": "qam16",
//!   "baud": 1200.0,
//!   "carrier": 1800.0,
//!   "amplitude": 0.3,
//!   "shaping": { "root_raised_cosine": { "alpha": 0.35 } },
//!   "differential": false,
//!   "complex": false
//! }
//! ```
//!
//! [`generate`] is fully deterministic: it runs serially, and all randomness comes from
//! the seed (the data from `seed`, the channel noise from a value derived from it), so
//! the same spec gives bit-identical samples on every run. Across machines the samples
//! are identical as long as the platform's `sin`, `cos`, `exp` and `ln` round alike
//! (they do on the usual targets, but Rust does not guarantee it).

use std::f64::consts::TAU;

use rustfft::num_complex::Complex64;
use serde::{Deserialize, Serialize};
use sn_core::{Complex32, Signal};

use crate::channel::clocked_len;
use crate::{
    C4FM_ALPHA, C4FM_DEVIATIONS, C4fmParams, C4fmPulse, ChannelHints, ChannelParams, Constellation,
    FskParams, FskShaping, LinearParams, MAX_SAMPLES, OfdmModel, OfdmModelParams, OfdmModulation,
    OfdmParams, PilotPattern, PilotReference, Result, Rng, Shaping, StopCheck,
    apply_channel_with_stop, c4fm_with_stop, check_amplitude, check_sample_rate, check_stop,
    fsk_with_stop, linear_with_stop, never_stop, output_len, random_symbols, require,
    require_positive,
};

/// Most tones in a [`TonesSpec`].
pub const MAX_TONES: usize = 256;
/// Longest label in characters.
pub const MAX_LABEL: usize = 1024;

/// Format a number with at most three decimals and no trailing zeros ("66.667",
/// "1200", "0.35").
pub fn fmt_num(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    };
    if s == "-0" { "0".to_owned() } else { s }
}

/// How long the signal is.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Length {
    /// A duration in seconds. Symbol-based kinds send as many whole symbols as fit.
    Seconds(f64),
    /// A number of symbols (not for tones and noise).
    Symbols(usize),
}

/// Fields every kind of signal shares.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Common {
    /// Free text naming the signal.
    #[serde(default)]
    pub label: String,
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// Length of the signal.
    pub length: Length,
    /// Seed of the data and the noise.
    #[serde(default)]
    pub seed: u64,
    /// RF frequency that 0 Hz of a complex output stands for (metadata only; real
    /// outputs ignore it).
    #[serde(default)]
    pub center_frequency: f64,
    /// Channel impairments.
    #[serde(default)]
    pub channel: ChannelParams,
}

impl Common {
    fn new(sample_rate: f64, seconds: f64) -> Self {
        Self {
            label: String::new(),
            sample_rate,
            length: Length::Seconds(seconds),
            seed: 1,
            center_frequency: 0.0,
            channel: ChannelParams::default(),
        }
    }

    fn validate(&self) -> Result<()> {
        check_sample_rate(self.sample_rate)?;
        require(
            self.label.chars().count() <= MAX_LABEL,
            "label",
            format!("at most {MAX_LABEL} characters"),
        )?;
        require(
            self.center_frequency.is_finite(),
            "center_frequency",
            "must be a finite frequency",
        )?;
        match self.length {
            Length::Seconds(s) => {
                require_positive(s, "length", "s")?;
                output_len(s * self.sample_rate, "length")?;
            }
            Length::Symbols(n) => require(
                (1..=MAX_SAMPLES).contains(&n),
                "length",
                format!("the number of symbols must be 1 to 2^28 (got {n})"),
            )?,
        }
        self.channel.validate(self.sample_rate)
    }

    /// Number of samples for a duration-only kind.
    fn samples(&self) -> Result<usize> {
        match self.length {
            Length::Seconds(s) => output_len(s * self.sample_rate, "length"),
            Length::Symbols(_) => Err(crate::Error::InvalidParameter {
                name: "length",
                reason: "this kind of signal has no symbols; give the length in seconds".to_owned(),
            }),
        }
    }

    /// Number of symbols at `rate` symbols per second, less `overhead` symbols of
    /// ramp when the length is in seconds.
    fn symbols(&self, rate: f64, overhead: usize) -> Result<usize> {
        match self.length {
            Length::Symbols(n) => Ok(n),
            Length::Seconds(s) => {
                let whole = (s * rate).floor();
                require(
                    whole.is_finite() && whole <= MAX_SAMPLES as f64,
                    "length",
                    "too many symbols; shorten the signal",
                )?;
                let n = (whole as usize).saturating_sub(overhead);
                require(
                    n >= 1,
                    "length",
                    format!(
                        "{s} s holds no whole symbol{} at {} symbols per second",
                        if overhead > 0 {
                            format!(" after the {overhead}-symbol pulse ramp")
                        } else {
                            String::new()
                        },
                        fmt_num(rate)
                    ),
                )?;
                Ok(n)
            }
        }
    }
}

/// One tone of a [`TonesSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToneSpec {
    /// Frequency in Hz (negative allowed for complex output).
    pub frequency: f64,
    /// Peak amplitude.
    pub amplitude: f64,
    /// Starting phase in radians.
    #[serde(default)]
    pub phase: f64,
}

/// A sum of tones: real `a cos(2 pi f t + phase)` or complex `a exp(j (2 pi f t +
/// phase))`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TonesSpec {
    /// Shared fields.
    pub common: Common,
    /// The tones (1 to [`MAX_TONES`]).
    pub tones: Vec<ToneSpec>,
    /// Complex (I/Q) output.
    #[serde(default)]
    pub complex: bool,
}

/// White Gaussian noise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoiseSpec {
    /// Shared fields.
    pub common: Common,
    /// RMS level.
    pub amplitude: f64,
    /// Complex (I/Q) output.
    #[serde(default)]
    pub complex: bool,
}

/// Continuous-phase FSK (see [`FskParams`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FskSpec {
    /// Shared fields.
    pub common: Common,
    /// Number of tones (2..=256).
    pub order: usize,
    /// Keying rate in baud.
    pub baud: f64,
    /// Tone spacing in Hz.
    pub spacing: f64,
    /// Centre of the tone grid in Hz.
    pub center: f64,
    /// Peak amplitude.
    pub amplitude: f64,
    /// Frequency-pulse shaping.
    #[serde(default = "default_fsk_shaping")]
    pub shaping: FskShaping,
    /// Complex (I/Q) output.
    #[serde(default)]
    pub complex: bool,
}

fn default_fsk_shaping() -> FskShaping {
    FskShaping::None
}

/// PSK or QAM (see [`LinearParams`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearSpec {
    /// Shared fields.
    pub common: Common,
    /// Constellation.
    pub constellation: Constellation,
    /// Symbol rate in baud.
    pub baud: f64,
    /// Carrier in Hz (offset from 0 Hz for complex output).
    pub carrier: f64,
    /// RMS amplitude.
    pub amplitude: f64,
    /// Pulse shape.
    pub shaping: Shaping,
    /// Differential (PSK only).
    #[serde(default)]
    pub differential: bool,
    /// Complex (I/Q) output.
    #[serde(default)]
    pub complex: bool,
}

/// C4FM (see [`C4fmParams`]). Always real.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct C4fmSpec {
    /// Shared fields.
    pub common: Common,
    /// Keying rate in baud.
    pub baud: f64,
    /// Carrier in Hz.
    pub carrier: f64,
    /// Peak amplitude.
    pub amplitude: f64,
    /// Shaped frequency pulses.
    pub shaped: bool,
    /// Which shaped pulse.
    #[serde(default)]
    pub pulse: C4fmPulse,
}

/// OFDM with a cyclic prefix in model A, B or C (see [`OfdmModelParams`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfdmSpec {
    /// Shared fields.
    pub common: Common,
    /// Useful length (FFT size) in samples at the sample rate.
    pub lu: usize,
    /// Guard length in samples.
    pub lg: usize,
    /// Number of consecutive channel positions.
    pub channels: usize,
    /// Bin of the first channel (0 is the centre).
    pub first_bin: i64,
    /// Positions that carry nothing.
    #[serde(default)]
    pub unused: Vec<usize>,
    /// Modulation of every channel.
    pub modulation: OfdmModulation,
    /// Per-position modulation overrides.
    #[serde(default)]
    pub overrides: Vec<(usize, OfdmModulation)>,
    /// Signal model.
    #[serde(default)]
    pub model: OfdmModel,
    /// Pilot positions.
    #[serde(default)]
    pub pilots: PilotPattern,
    /// What model C pilots carry.
    #[serde(default)]
    pub pilot_reference: PilotReference,
    /// RMS amplitude.
    pub amplitude: f64,
    /// Where bin 0 lands, in Hz (for complex output, the offset from 0 Hz).
    pub center: f64,
    /// Complex (I/Q) output.
    #[serde(default)]
    pub complex: bool,
}

/// Everything needed to generate a test signal. Serialised with a `"kind"` tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GeneratorSpec {
    /// A sum of tones.
    Tones(TonesSpec),
    /// White Gaussian noise.
    Noise(NoiseSpec),
    /// CPFSK, GFSK, MSK, GMSK.
    Fsk(FskSpec),
    /// PSK and QAM.
    Linear(LinearSpec),
    /// C4FM.
    C4fm(C4fmSpec),
    /// OFDM with a cyclic prefix.
    Ofdm(OfdmSpec),
}

/// The kinds of [`GeneratorSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    /// [`GeneratorSpec::Tones`].
    Tones,
    /// [`GeneratorSpec::Noise`].
    Noise,
    /// [`GeneratorSpec::Fsk`].
    Fsk,
    /// [`GeneratorSpec::Linear`].
    Linear,
    /// [`GeneratorSpec::C4fm`].
    C4fm,
    /// [`GeneratorSpec::Ofdm`].
    Ofdm,
}

impl SignalKind {
    /// Every kind.
    pub const ALL: [Self; 6] = [
        Self::Tones,
        Self::Noise,
        Self::Fsk,
        Self::Linear,
        Self::C4fm,
        Self::Ofdm,
    ];
}

/// Names accepted by [`GeneratorSpec::preset`].
pub const PRESET_NAMES: [&str; 6] = [
    "ofdm_lu64_lg20_7000",
    "ofdm_30ch_pi4dqpsk_6400",
    "mfsk_11",
    "cis12_like",
    "mil188_110b_39tone_like",
    "qam16_rrc_1200",
];

impl GeneratorSpec {
    /// Shared fields.
    pub fn common(&self) -> &Common {
        match self {
            Self::Tones(s) => &s.common,
            Self::Noise(s) => &s.common,
            Self::Fsk(s) => &s.common,
            Self::Linear(s) => &s.common,
            Self::C4fm(s) => &s.common,
            Self::Ofdm(s) => &s.common,
        }
    }

    /// Shared fields, to change.
    pub fn common_mut(&mut self) -> &mut Common {
        match self {
            Self::Tones(s) => &mut s.common,
            Self::Noise(s) => &mut s.common,
            Self::Fsk(s) => &mut s.common,
            Self::Linear(s) => &mut s.common,
            Self::C4fm(s) => &mut s.common,
            Self::Ofdm(s) => &mut s.common,
        }
    }

    /// The kind of signal.
    pub fn kind(&self) -> SignalKind {
        match self {
            Self::Tones(_) => SignalKind::Tones,
            Self::Noise(_) => SignalKind::Noise,
            Self::Fsk(_) => SignalKind::Fsk,
            Self::Linear(_) => SignalKind::Linear,
            Self::C4fm(_) => SignalKind::C4fm,
            Self::Ofdm(_) => SignalKind::Ofdm,
        }
    }

    /// A sensible 10 s signal of `kind` at 8000 Hz (C4FM at 48 kHz, since its fixed
    /// +-1800 Hz deviation at 4800 Bd needs it), seed 1, no impairments.
    pub fn default_for(kind: SignalKind) -> Self {
        let common = Common::new(8000.0, 10.0);
        match kind {
            SignalKind::Tones => Self::Tones(TonesSpec {
                common,
                tones: vec![
                    ToneSpec {
                        frequency: 1000.0,
                        amplitude: 0.5,
                        phase: 0.0,
                    },
                    ToneSpec {
                        frequency: 2200.0,
                        amplitude: 0.25,
                        phase: 0.0,
                    },
                ],
                complex: false,
            }),
            SignalKind::Noise => Self::Noise(NoiseSpec {
                common,
                amplitude: 0.25,
                complex: false,
            }),
            SignalKind::Fsk => Self::Fsk(FskSpec {
                common,
                order: 2,
                baud: 100.0,
                spacing: 200.0,
                center: 1500.0,
                amplitude: 0.5,
                shaping: FskShaping::None,
                complex: false,
            }),
            SignalKind::Linear => Self::Linear(LinearSpec {
                common,
                constellation: Constellation::Qam16,
                baud: 1200.0,
                carrier: 1800.0,
                amplitude: 0.3,
                shaping: Shaping::RootRaisedCosine { alpha: 0.35 },
                differential: false,
                complex: false,
            }),
            SignalKind::C4fm => Self::C4fm(C4fmSpec {
                common: Common::new(48_000.0, 10.0),
                baud: 4800.0,
                carrier: 12_000.0,
                amplitude: 0.5,
                shaped: true,
                pulse: C4fmPulse::RaisedCosine,
            }),
            SignalKind::Ofdm => Self::Ofdm(OfdmSpec {
                common,
                lu: 64,
                lg: 16,
                channels: 16,
                first_bin: -8,
                unused: Vec::new(),
                modulation: OfdmModulation::Qpsk,
                overrides: Vec::new(),
                model: OfdmModel::B,
                pilots: PilotPattern::None,
                pilot_reference: PilotReference::Constant,
                amplitude: 0.3,
                center: 1800.0,
                complex: false,
            }),
        }
    }

    /// A named preset (see [`PRESET_NAMES`]), or `None` for an unknown name:
    /// - `ofdm_lu64_lg20_7000`: OCG's test signal LU 64, LG 20, LS 84 at Fd 7000 Hz
    ///   (83.333 Bd, 109.375 Hz spacing), 24 channels of QPSK in model B.
    /// - `ofdm_30ch_pi4dqpsk_6400`: 30 channels of pi/4-DQPSK, spacing 80 Hz, Fd 6400,
    ///   LU 80, LG 16, LS 96 (66.667 Bd), model B.
    /// - `mfsk_11`: 11-tone MFSK (the tone count of OCG's MFSK-11 example; the rate and
    ///   spacing, 31.25 Bd and 31.25 Hz, are our choice).
    /// - `cis12_like`: 12 channels of pi/4-DQPSK with LU 36, LG 24 at 7200 Hz
    ///   (K = 2/3, 120 Bd, 200 Hz spacing), model B. The real CIS-12 sits on a
    ///   half-step grid; this one uses whole bins.
    /// - `mil188_110b_39tone_like`: 39 channels of QPSK with LU 128, LG 34 (K = 17/64,
    ///   LS 162) at Fd = 162 x 43.298284 = 7014.32 Hz, model B.
    /// - `qam16_rrc_1200`: QAM-16, 1200 Bd, RRC 0.35, on 1800 Hz at 8000 Hz.
    pub fn preset(name: &str) -> Option<Self> {
        let ofdm = |fd: f64,
                    lu: usize,
                    lg: usize,
                    channels: usize,
                    modulation: OfdmModulation,
                    center: f64| {
            let mut common = Common::new(fd, 10.0);
            name.clone_into(&mut common.label);
            Self::Ofdm(OfdmSpec {
                common,
                lu,
                lg,
                channels,
                first_bin: -((channels / 2) as i64),
                unused: Vec::new(),
                modulation,
                overrides: Vec::new(),
                model: OfdmModel::B,
                pilots: PilotPattern::None,
                pilot_reference: PilotReference::Constant,
                amplitude: 0.3,
                center,
                complex: false,
            })
        };
        let spec = match name {
            "ofdm_lu64_lg20_7000" => ofdm(7000.0, 64, 20, 24, OfdmModulation::Qpsk, 1800.0),
            "ofdm_30ch_pi4dqpsk_6400" => ofdm(6400.0, 80, 16, 30, OfdmModulation::Pi4Dqpsk, 1600.0),
            "cis12_like" => ofdm(7200.0, 36, 24, 12, OfdmModulation::Pi4Dqpsk, 1700.0),
            "mil188_110b_39tone_like" => ofdm(
                162.0 * 43.298_284,
                128,
                34,
                39,
                OfdmModulation::Qpsk,
                1800.0,
            ),
            "mfsk_11" => {
                let mut common = Common::new(8000.0, 10.0);
                name.clone_into(&mut common.label);
                Self::Fsk(FskSpec {
                    common,
                    order: 11,
                    baud: 31.25,
                    spacing: 31.25,
                    center: 1500.0,
                    amplitude: 0.5,
                    shaping: FskShaping::None,
                    complex: false,
                })
            }
            "qam16_rrc_1200" => {
                let mut s = Self::default_for(SignalKind::Linear);
                name.clone_into(&mut s.common_mut().label);
                s
            }
            _ => return None,
        };
        Some(spec)
    }
}

/// What a spec would produce, worked out by [`GeneratorSpec::plan`] without making
/// any samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    /// Number of output samples (after the channel's clock offset).
    pub samples: usize,
    /// Number of symbols sent (`None` for tones and noise).
    pub symbols: Option<usize>,
    /// Symbol rate in Bd (`None` for tones and noise).
    pub symbol_rate: Option<f64>,
    /// Bandwidth the clean signal occupies in Hz.
    pub occupied_bandwidth: f64,
    /// Lowest and highest frequency of the clean signal in Hz, before the channel
    /// (`None` for noise, which fills the whole band).
    pub band: Option<(f64, f64)>,
    /// Complex (I/Q) output.
    pub complex: bool,
}

impl Plan {
    /// Duration of the output in seconds at `sample_rate`.
    pub fn duration(&self, sample_rate: f64) -> f64 {
        self.samples as f64 / sample_rate
    }
}

impl GeneratorSpec {
    /// Check every field and work out the size of the signal, without generating it.
    /// Cheap (no sample is made), so a user interface can call it on every change.
    /// [`generate`] can still fail where a check needs the samples (noise relative to
    /// a silent signal), but every parameter error is found here.
    pub fn plan(&self) -> Result<Plan> {
        let common = self.common();
        common.validate()?;
        let fs = common.sample_rate;
        let mut plan = self.clean_plan()?;
        let channel = &common.channel;
        let hints = channel_hints(
            plan.occupied_bandwidth,
            plan.symbol_rate,
            plan.complex,
            fs,
            None,
        );
        channel.check_hints(&hints, fs, plan.complex)?;
        if channel.frequency_offset != 0.0 || channel.drift != 0.0 {
            channel.check_shift(plan.samples, fs)?;
            if let Some(band) = plan.band {
                check_shifted_band(channel, band, plan.samples, fs, plan.complex)?;
            }
        }
        plan.samples = clocked_len(plan.samples, channel.clock_ppm)?;
        Ok(plan)
    }

    /// [`Self::plan`] of the clean signal, before the channel. Only valid after
    /// [`Common::validate`].
    fn clean_plan(&self) -> Result<Plan> {
        let fs = self.common().sample_rate;
        Ok(match self {
            Self::Tones(s) => {
                let samples = tones_plan(s)?;
                let lo = s
                    .tones
                    .iter()
                    .map(|t| t.frequency)
                    .fold(f64::INFINITY, f64::min);
                let hi = s
                    .tones
                    .iter()
                    .map(|t| t.frequency)
                    .fold(f64::NEG_INFINITY, f64::max);
                Plan {
                    samples,
                    symbols: None,
                    symbol_rate: None,
                    occupied_bandwidth: if s.complex { fs } else { fs / 2.0 },
                    band: Some((lo, hi)),
                    complex: s.complex,
                }
            }
            Self::Noise(s) => {
                check_amplitude(s.amplitude)?;
                Plan {
                    samples: s.common.samples()?,
                    symbols: None,
                    symbol_rate: None,
                    occupied_bandwidth: if s.complex { fs } else { fs / 2.0 },
                    band: None,
                    complex: s.complex,
                }
            }
            Self::Fsk(s) => {
                let (p, count, samples) = fsk_plan(s)?;
                Plan {
                    samples,
                    symbols: Some(count),
                    symbol_rate: Some(s.baud),
                    occupied_bandwidth: p.occupied_bandwidth(),
                    band: Some((
                        p.tone_frequency(0) - s.baud,
                        p.tone_frequency(s.order - 1) + s.baud,
                    )),
                    complex: s.complex,
                }
            }
            Self::Linear(s) => {
                let (_, count, samples) = linear_plan(s)?;
                let half = s.shaping.occupied_bandwidth(s.baud) / 2.0;
                Plan {
                    samples,
                    symbols: Some(count),
                    symbol_rate: Some(s.baud),
                    occupied_bandwidth: s.shaping.occupied_bandwidth(s.baud),
                    band: Some((s.carrier - half, s.carrier + half)),
                    complex: s.complex,
                }
            }
            Self::C4fm(s) => {
                let (_, count, samples) = c4fm_plan(s)?;
                // The same edges C4fmParams::check requires of the unshifted signal,
                // so a frequency offset or drift cannot move it closer to 0 Hz or
                // Nyquist than the generator accepts.
                let half = crate::c4fm::c4fm_band_half_width(s.baud);
                Plan {
                    samples,
                    symbols: Some(count),
                    symbol_rate: Some(s.baud),
                    occupied_bandwidth: c4fm_bandwidth(s.baud),
                    band: Some((s.carrier - half, s.carrier + half)),
                    complex: false,
                }
            }
            Self::Ofdm(s) => {
                let p = ofdm_plan(s)?;
                let b = &p.base;
                Plan {
                    samples: b.symbols.saturating_mul(b.ls()),
                    symbols: Some(b.symbols),
                    symbol_rate: Some(b.symbol_rate()),
                    occupied_bandwidth: ofdm_bandwidth(s, b),
                    band: Some(ofdm_band(s, b)),
                    complex: s.complex,
                }
            }
        })
    }

    /// Check every field without generating anything ([`Self::plan`] without the
    /// result).
    pub fn validate(&self) -> Result<()> {
        self.plan().map(drop)
    }
}

/// Ground truth for an OFDM signal.
#[derive(Debug, Clone, PartialEq)]
pub struct OfdmTruth {
    /// Bin of each active channel.
    pub bins: Vec<i64>,
    /// Bins of the pilot channels.
    pub pilot_bins: Vec<i64>,
    /// `values[k][c]`: unit-power value sent on active channel `c` in symbol `k`.
    pub values: Vec<Vec<Complex32>>,
    /// `data[k][c]`: the data point the model's view recovers.
    pub data: Vec<Vec<Complex32>>,
    /// `data_indices[k][c]`: the index of `data[k][c]` in its constellation (see
    /// [`crate::OfdmModelOutput::data_indices`]).
    pub data_indices: Vec<Vec<u8>>,
    /// Common phase of every symbol (model C), radians.
    pub common_phase: Vec<f64>,
    /// Useful length in samples.
    pub lu: usize,
    /// Guard length in samples.
    pub lg: usize,
}

/// What went into a generated signal. Times and symbol rates are those of the
/// generator, before any clock offset of the channel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroundTruth {
    /// Symbol indices sent (FSK tone, constellation index, C4FM level); empty for
    /// tones, noise and OFDM (see `ofdm`).
    pub symbols: Vec<usize>,
    /// Unit-power constellation values sent (PSK and QAM only, after differential
    /// encoding).
    pub values: Vec<Complex32>,
    /// Symbol rate in Bd.
    pub symbol_rate: Option<f64>,
    /// Centre of symbol 0 in seconds (start of the first useful part for OFDM).
    pub first_symbol_time: Option<f64>,
    /// Bandwidth the clean signal occupies in Hz.
    pub occupied_bandwidth: f64,
    /// Length of the truncated pulse in symbols (PSK and QAM only; 0 for rectangular
    /// pulses). See [`crate::Shaping::span`].
    pub pulse_span: Option<usize>,
    /// OFDM details.
    pub ofdm: Option<OfdmTruth>,
}

/// A generated signal.
#[derive(Debug, Clone)]
pub struct Generated {
    /// The samples (real or complex; the centre frequency is set on complex output).
    pub signal: Signal,
    /// Short English description, for example "QAM-16, 1200 Bd, RRC 0.35, 1800 Hz,
    /// SNR 20 dB".
    pub description: String,
    /// What went in.
    pub truth: GroundTruth,
}

/// Seed of the channel noise, derived from the data seed so that in practice the noise
/// is not correlated with the data.
fn noise_seed(seed: u64) -> u64 {
    Rng::new(seed ^ 0x6E6F_6973_655F_7365).next_u64()
}

/// Hints for the channel from what the generator knows about the clean signal.
fn channel_hints(
    occupied_bandwidth: f64,
    symbol_rate: Option<f64>,
    complex: bool,
    sample_rate: f64,
    steady: Option<(f64, f64)>,
) -> ChannelHints {
    let band = if complex {
        sample_rate
    } else {
        sample_rate / 2.0
    };
    ChannelHints {
        bandwidth: Some(occupied_bandwidth.min(band)),
        symbol_rate,
        steady,
    }
}

/// Require the clean band `lo..hi`, moved by the frequency offset and drift over
/// `samples` samples, to stay inside 0 Hz to Nyquist (real) or -Nyquist to Nyquist
/// (complex), so the shift neither folds the spectrum over 0 Hz nor wraps it.
fn check_shifted_band(
    channel: &ChannelParams,
    (lo, hi): (f64, f64),
    samples: usize,
    sample_rate: f64,
    complex: bool,
) -> Result<()> {
    let (slo, shi) = channel.shift_range(samples, sample_rate);
    let (lo, hi) = (lo + slo, hi + shi);
    let nyquist = sample_rate / 2.0;
    let floor = if complex { -nyquist } else { 0.0 };
    require(
        lo > floor && hi < nyquist,
        "frequency_offset",
        format!(
            "with the frequency offset and drift the signal would span {lo:.1} to {hi:.1} \
             Hz, outside {floor} to {nyquist} Hz, so its spectrum would {}; lower the offset \
             or the drift",
            if complex {
                "wrap around"
            } else {
                "fold over 0 Hz or Nyquist"
            }
        ),
    )
}

/// Generate the signal `spec` describes. Every field is checked first ([`GeneratorSpec::plan`]),
/// sizes before anything is allocated.
pub fn generate(spec: &GeneratorSpec) -> Result<Generated> {
    generate_with_stop(spec, &never_stop)
}

/// [`generate`] that calls `stop` now and then (every few thousand samples) and fails
/// with [`crate::Error::Cancelled`] when it returns true.
pub fn generate_with_stop(spec: &GeneratorSpec, stop: StopCheck<'_>) -> Result<Generated> {
    let plan = spec.plan()?;
    let common = spec.common();
    let fs = common.sample_rate;
    let (signal, mut description, truth, complex) = match spec {
        GeneratorSpec::Tones(s) => tones(s, stop)?,
        GeneratorSpec::Noise(s) => noise(s, stop)?,
        GeneratorSpec::Fsk(s) => fsk_signal(s, stop)?,
        GeneratorSpec::Linear(s) => linear_signal(s, stop)?,
        GeneratorSpec::C4fm(s) => c4fm_signal(s, stop)?,
        GeneratorSpec::Ofdm(s) => ofdm_signal(s, stop)?,
    };
    // Shaped linear pulses ramp up and down over the pulse span; the power for a noise
    // level is measured between the first and the last symbol centre.
    let steady = match (truth.pulse_span, truth.first_symbol_time, truth.symbol_rate) {
        (Some(span), Some(t0), Some(rate)) if span > 0 && !truth.symbols.is_empty() => {
            Some((t0, t0 + (truth.symbols.len() - 1) as f64 / rate))
        }
        _ => None,
    };
    let hints = channel_hints(
        truth.occupied_bandwidth,
        truth.symbol_rate,
        complex,
        fs,
        steady,
    );
    let signal = apply_channel_with_stop(
        &signal,
        &common.channel,
        noise_seed(common.seed),
        &hints,
        stop,
    )?;
    require(
        signal.len() == plan.samples,
        "length",
        format!(
            "internal error: {} samples were made where {} were planned",
            signal.len(),
            plan.samples
        ),
    )?;
    let signal = if complex {
        signal.with_center_frequency(common.center_frequency)
    } else {
        signal
    };
    for part in common.channel.describe() {
        description.push_str(", ");
        description.push_str(&part);
    }
    if complex {
        description.push_str(", I/Q");
    }
    Ok(Generated {
        signal,
        description,
        truth,
    })
}

type Made = (Signal, String, GroundTruth, bool);

/// Check a [`TonesSpec`] and return its number of samples.
fn tones_plan(s: &TonesSpec) -> Result<usize> {
    let nyquist = s.common.sample_rate / 2.0;
    require(
        (1..=MAX_TONES).contains(&s.tones.len()),
        "tones",
        format!("give 1 to {MAX_TONES} tones (got {})", s.tones.len()),
    )?;
    for t in &s.tones {
        let ok = t.frequency.is_finite()
            && t.frequency.abs() < nyquist
            && (s.complex || t.frequency >= 0.0);
        require(
            ok,
            "tones",
            format!(
                "{} Hz must be {} below Nyquist ({nyquist} Hz)",
                t.frequency,
                if s.complex {
                    "in size"
                } else {
                    "at least 0 Hz and"
                }
            ),
        )?;
        check_amplitude(t.amplitude)?;
        require(t.phase.is_finite(), "tones", "phases must be finite")?;
    }
    s.common.samples()
}

fn tones(s: &TonesSpec, stop: StopCheck<'_>) -> Result<Made> {
    let c = &s.common;
    let fs = c.sample_rate;
    let nyquist = fs / 2.0;
    let n = tones_plan(s)?;
    let at = |i: usize| -> Complex64 {
        s.tones
            .iter()
            .map(|t| {
                let cycles = (i as f64 * t.frequency / fs).rem_euclid(1.0);
                Complex64::from_polar(t.amplitude, TAU * cycles + t.phase)
            })
            .sum()
    };
    let signal = if s.complex {
        let mut v: Vec<Complex32> = Vec::with_capacity(n);
        for i in 0..n {
            check_stop(stop, i)?;
            let z = at(i);
            v.push(Complex32::new(z.re as f32, z.im as f32));
        }
        Signal::complex(v, fs)?
    } else {
        let mut v: Vec<f32> = Vec::with_capacity(n);
        for i in 0..n {
            check_stop(stop, i)?;
            v.push(at(i).re as f32);
        }
        Signal::real(v, fs)?
    };
    let list: Vec<String> = s
        .tones
        .iter()
        .take(4)
        .map(|t| format!("{} Hz", fmt_num(t.frequency)))
        .collect();
    let more = if s.tones.len() > 4 { ", ..." } else { "" };
    let description = if s.tones.len() == 1 {
        format!("tone {}", list.join(""))
    } else {
        format!("{} tones: {}{more}", s.tones.len(), list.join(", "))
    };
    let truth = GroundTruth {
        occupied_bandwidth: if s.complex { fs } else { nyquist },
        ..GroundTruth::default()
    };
    Ok((signal, description, truth, s.complex))
}

fn noise(s: &NoiseSpec, stop: StopCheck<'_>) -> Result<Made> {
    let c = &s.common;
    check_amplitude(s.amplitude)?;
    let n = c.samples()?;
    let mut rng = Rng::new(c.seed);
    let fs = c.sample_rate;
    let signal = if s.complex {
        let sigma = s.amplitude / std::f64::consts::SQRT_2;
        let mut v: Vec<Complex32> = Vec::with_capacity(n);
        for i in 0..n {
            check_stop(stop, i)?;
            let re = (sigma * rng.normal()) as f32;
            let im = (sigma * rng.normal()) as f32;
            v.push(Complex32::new(re, im));
        }
        Signal::complex(v, fs)?
    } else {
        let mut v: Vec<f32> = Vec::with_capacity(n);
        for i in 0..n {
            check_stop(stop, i)?;
            v.push((s.amplitude * rng.normal()) as f32);
        }
        Signal::real(v, fs)?
    };
    let description = format!("white noise, RMS {}", fmt_num(s.amplitude));
    let truth = GroundTruth {
        occupied_bandwidth: if s.complex { fs } else { fs / 2.0 },
        ..GroundTruth::default()
    };
    Ok((signal, description, truth, s.complex))
}

/// Check an [`FskSpec`]: its parameters, symbol count and sample count.
fn fsk_plan(s: &FskSpec) -> Result<(FskParams, usize, usize)> {
    let c = &s.common;
    let p = FskParams {
        order: s.order,
        baud: s.baud,
        spacing: s.spacing,
        center: s.center,
        sample_rate: c.sample_rate,
        amplitude: s.amplitude,
        complex: s.complex,
        shaping: s.shaping,
    };
    p.validate()?;
    let count = c.symbols(s.baud, 0)?;
    let samples = crate::samples_for_symbols(count, s.baud, c.sample_rate)?;
    Ok((p, count, samples))
}

fn fsk_signal(s: &FskSpec, stop: StopCheck<'_>) -> Result<Made> {
    let c = &s.common;
    let (p, count, _) = fsk_plan(s)?;
    let symbols = random_symbols(count, s.order, c.seed);
    let signal = fsk_with_stop(&p, &symbols, stop)?;
    let description = format!(
        "{}, {} Bd, spacing {} Hz, {} Hz",
        p.name(),
        fmt_num(s.baud),
        fmt_num(s.spacing),
        fmt_num(s.center)
    );
    let truth = GroundTruth {
        symbols,
        symbol_rate: Some(s.baud),
        first_symbol_time: Some(0.5 / s.baud),
        occupied_bandwidth: p.occupied_bandwidth(),
        ..GroundTruth::default()
    };
    Ok((signal, description, truth, s.complex))
}

/// Check a [`LinearSpec`]: its parameters, symbol count and sample count.
fn linear_plan(s: &LinearSpec) -> Result<(LinearParams, usize, usize)> {
    let c = &s.common;
    let p = LinearParams {
        constellation: s.constellation,
        baud: s.baud,
        sample_rate: c.sample_rate,
        carrier: s.carrier,
        amplitude: s.amplitude,
        shaping: s.shaping,
        differential: s.differential,
        complex: s.complex,
    };
    p.validate()?;
    let count = c.symbols(s.baud, s.shaping.span())?;
    let samples = p.output_samples(count)?;
    Ok((p, count, samples))
}

fn linear_signal(s: &LinearSpec, stop: StopCheck<'_>) -> Result<Made> {
    let c = &s.common;
    let (p, count, _) = linear_plan(s)?;
    let symbols = random_symbols(count, s.constellation.order(), c.seed);
    let out = linear_with_stop(&p, &symbols, stop)?;
    let description = format!(
        "{}{}, {} Bd, {}, {} Hz",
        s.constellation.name(),
        if s.differential && s.constellation != Constellation::Pi4Dqpsk {
            " differential"
        } else {
            ""
        },
        fmt_num(s.baud),
        s.shaping.name(),
        fmt_num(s.carrier)
    );
    let truth = GroundTruth {
        symbols: out.symbols,
        values: out.values,
        symbol_rate: Some(s.baud),
        first_symbol_time: Some(out.first_symbol_time),
        occupied_bandwidth: s.shaping.occupied_bandwidth(s.baud),
        pulse_span: Some(out.pulse_span),
        ofdm: None,
    };
    Ok((out.signal, description, truth, s.complex))
}

/// Check a [`C4fmSpec`]: its parameters, symbol count and sample count.
fn c4fm_plan(s: &C4fmSpec) -> Result<(C4fmParams, usize, usize)> {
    let c = &s.common;
    require_positive(s.baud, "baud", "Bd")?;
    let count = c.symbols(s.baud, 0)?;
    let samples = crate::samples_for_symbols(count, s.baud, c.sample_rate)?;
    let p = C4fmParams {
        baud: s.baud,
        carrier: s.carrier,
        sample_rate: c.sample_rate,
        amplitude: s.amplitude,
        shaped: s.shaped,
        pulse: s.pulse,
    };
    p.check()?;
    Ok((p, count, samples))
}

/// Occupied bandwidth of C4FM: the outer deviation either side plus one baud.
fn c4fm_bandwidth(baud: f64) -> f64 {
    let max_dev = C4FM_DEVIATIONS.iter().fold(0.0_f64, |a, d| a.max(d.abs()));
    2.0 * max_dev + baud
}

fn c4fm_signal(s: &C4fmSpec, stop: StopCheck<'_>) -> Result<Made> {
    let c = &s.common;
    let (p, count, _) = c4fm_plan(s)?;
    let levels = random_symbols(count, 4, c.seed);
    let signal = c4fm_with_stop(&p, &levels, stop)?;
    let shape = match (s.shaped, s.pulse) {
        (false, _) => "rectangular".to_owned(),
        (true, C4fmPulse::RaisedCosine) => format!("RC {}", fmt_num(C4FM_ALPHA)),
        (true, C4fmPulse::Apco25) => "APCO-25 pulse".to_owned(),
    };
    let description = format!(
        "C4FM, {} Bd, {shape}, {} Hz",
        fmt_num(s.baud),
        fmt_num(s.carrier)
    );
    let truth = GroundTruth {
        symbols: levels,
        symbol_rate: Some(s.baud),
        first_symbol_time: Some(0.5 / s.baud),
        occupied_bandwidth: c4fm_bandwidth(s.baud),
        ..GroundTruth::default()
    };
    Ok((signal, description, truth, false))
}

/// Check an [`OfdmSpec`] (also the band) and return the model parameters.
fn ofdm_plan(s: &OfdmSpec) -> Result<OfdmModelParams> {
    let c = &s.common;
    let fs = c.sample_rate;
    require(
        (2..=crate::OFDM_MAX_LU).contains(&s.lu),
        "lu",
        format!("must be 2 to {} samples (got {})", crate::OFDM_MAX_LU, s.lu),
    )?;
    require(
        s.lg <= crate::OFDM_MAX_LG,
        "lg",
        format!(
            "must be at most {} samples (got {})",
            crate::OFDM_MAX_LG,
            s.lg
        ),
    )?;
    let ls = s.lu + s.lg;
    let symbols = c.symbols(fs / ls as f64, 0)?;
    require(
        symbols.saturating_mul(ls) <= MAX_SAMPLES,
        "length",
        "the signal would exceed 2^28 samples; generate fewer symbols",
    )?;
    require(s.center.is_finite(), "center", "must be a finite frequency")?;
    let params = OfdmModelParams {
        base: OfdmParams {
            sample_rate: fs,
            lu: s.lu,
            lg: s.lg,
            channels: s.channels,
            first_bin: s.first_bin,
            unused: s.unused.clone(),
            modulation: s.modulation,
            overrides: s.overrides.clone(),
            symbols,
            frequency_offset: if s.complex { s.center } else { 0.0 },
            amplitude: s.amplitude,
            seed: c.seed,
        },
        model: s.model,
        pilots: s.pilots.clone(),
        pilot_reference: s.pilot_reference,
    };
    params.validate()?;
    if s.complex {
        crate::ofdm::check_complex_band(&params.base)?;
    } else {
        crate::ofdm::check_real_band(&params.base, s.center)?;
    }
    Ok(params)
}

/// Lowest and highest frequency of OFDM, with the same edges as the generator's own
/// band check (bin centres for complex output, one spacing beyond for real output).
fn ofdm_band(s: &OfdmSpec, base: &OfdmParams) -> (f64, f64) {
    let first = base.first_bin as f64;
    let last = crate::ofdm::last_bin(base) as f64;
    let sp = base.spacing();
    if s.complex {
        (first * sp + s.center, last * sp + s.center)
    } else {
        ((first - 1.0) * sp + s.center, (last + 1.0) * sp + s.center)
    }
}

/// Occupied bandwidth of OFDM: the channel grid plus one symbol rate either side.
fn ofdm_bandwidth(s: &OfdmSpec, base: &OfdmParams) -> f64 {
    (s.channels.saturating_sub(1)) as f64 * base.spacing() + 2.0 * base.symbol_rate()
}

fn ofdm_signal(s: &OfdmSpec, stop: StopCheck<'_>) -> Result<Made> {
    let c = &s.common;
    let fs = c.sample_rate;
    let params = ofdm_plan(s)?;
    let center = if s.complex { None } else { Some(s.center) };
    let out = crate::ofdm_model::ofdm_model_with_stop(&params, center, stop)?;
    let b = &params.base;
    let n = out.output.bins.len();
    let description = format!(
        "OFDM model {}, {n} x {}, LU {} / LG {}, Fd {} Hz, {} Bd, spacing {} Hz, {} Hz",
        s.model.name(),
        s.modulation.name(),
        s.lu,
        s.lg,
        fmt_num(fs),
        fmt_num(b.symbol_rate()),
        fmt_num(b.spacing()),
        fmt_num(s.center)
    );
    let truth = GroundTruth {
        symbol_rate: Some(b.symbol_rate()),
        first_symbol_time: Some(s.lg as f64 / fs),
        occupied_bandwidth: ofdm_bandwidth(s, b),
        ofdm: Some(OfdmTruth {
            bins: out.output.bins,
            pilot_bins: out.pilot_bins,
            values: out.output.symbols,
            data: out.data,
            data_indices: out.data_indices,
            common_phase: out.common_phase,
            lu: s.lu,
            lg: s.lg,
        }),
        ..GroundTruth::default()
    };
    Ok((out.output.signal, description, truth, s.complex))
}

#[cfg(test)]
mod tests {
    use sn_core::Samples;

    use super::*;
    use crate::{NoiseLevel, PULSE_SPAN, Tap, linear};

    fn bits(s: &Signal) -> Vec<u32> {
        match s.samples() {
            Samples::Real(x) => x.iter().map(|v| v.to_bits()).collect(),
            Samples::Complex(z) => z
                .iter()
                .flat_map(|v| [v.re.to_bits(), v.im.to_bits()])
                .collect(),
            Samples::Lazy(_) => panic!("lazy"),
        }
    }

    fn all_specs() -> Vec<GeneratorSpec> {
        let mut v: Vec<GeneratorSpec> = SignalKind::ALL
            .iter()
            .map(|k| GeneratorSpec::default_for(*k))
            .collect();
        v.extend(
            PRESET_NAMES
                .iter()
                .map(|n| GeneratorSpec::preset(n).unwrap()),
        );
        v
    }

    #[test]
    fn defaults_and_presets_generate() {
        for spec in all_specs() {
            let g = generate(&spec).unwrap_or_else(|e| panic!("{spec:?}: {e}"));
            let secs = g.signal.len() as f64 / g.signal.sample_rate();
            assert!((9.5..=10.01).contains(&secs), "{}: {secs} s", g.description);
            assert_ne!(g.description, "");
        }
        assert!(GeneratorSpec::preset("nope").is_none());
        for k in SignalKind::ALL {
            assert_eq!(GeneratorSpec::default_for(k).kind(), k);
        }
    }

    #[test]
    fn descriptions() {
        let mut spec = GeneratorSpec::default_for(SignalKind::Linear);
        spec.common_mut().channel.noise = Some(NoiseLevel::Snr {
            db: 20.0,
            bandwidth: None,
        });
        let g = generate(&spec).unwrap();
        assert_eq!(
            g.description,
            "QAM-16, 1200 Bd, RRC 0.35, 1800 Hz, SNR 20 dB"
        );
        let g = generate(&GeneratorSpec::preset("ofdm_30ch_pi4dqpsk_6400").unwrap()).unwrap();
        assert_eq!(
            g.description,
            "OFDM model B, 30 x pi/4-DQPSK, LU 80 / LG 16, Fd 6400 Hz, 66.667 Bd, \
             spacing 80 Hz, 1600 Hz"
        );
        let g = generate(&GeneratorSpec::preset("ofdm_lu64_lg20_7000").unwrap()).unwrap();
        assert!(
            g.description.contains("83.333 Bd, spacing 109.375 Hz"),
            "{}",
            g.description
        );
        assert_eq!(fmt_num(-0.0001), "0");
        assert_eq!(fmt_num(1200.0), "1200");
    }

    #[test]
    fn serde_round_trip_every_kind() {
        let mut specs = all_specs();
        // Exercise the channel and the non-default fields too.
        let mut busy = GeneratorSpec::default_for(SignalKind::Ofdm);
        if let GeneratorSpec::Ofdm(o) = &mut busy {
            o.model = OfdmModel::C;
            o.pilots = PilotPattern::Every { step: 4, offset: 0 };
            o.pilot_reference = PilotReference::Differential;
            o.overrides = vec![(1, OfdmModulation::Qam16)];
            o.unused = vec![2];
            o.complex = true;
            o.common.length = Length::Symbols(30);
            o.common.channel = ChannelParams {
                frequency_offset: 12.5,
                drift: 0.25,
                multipath: vec![Tap {
                    delay: 0.001,
                    gain: 0.5,
                    phase: 1.0,
                }],
                clock_ppm: 100.0,
                noise: Some(NoiseLevel::EsN0 {
                    db: 15.0,
                    symbol_rate: None,
                }),
                clip: Some(2.0),
            };
        }
        specs.push(busy);
        for spec in specs {
            let json = serde_json::to_string_pretty(&spec).unwrap();
            let back: GeneratorSpec = serde_json::from_str(&json).unwrap();
            assert_eq!(back, spec, "{json}");
            assert_eq!(serde_json::to_string_pretty(&back).unwrap(), json);
        }
    }

    #[test]
    fn json_shape_is_stable() {
        let json = r#"{
            "kind": "linear",
            "common": { "sample_rate": 8000.0, "length": { "symbols": 100 }, "seed": 3 },
            "constellation": "psk8",
            "baud": 1000.0,
            "carrier": 0.0,
            "amplitude": 0.5,
            "shaping": { "raised_cosine": { "alpha": 0.5 } },
            "differential": true,
            "complex": true
        }"#;
        let spec: GeneratorSpec = serde_json::from_str(json).unwrap();
        let g = generate(&spec).unwrap();
        assert_eq!(g.truth.symbols.len(), 100);
        assert_eq!(g.signal.len(), (100 + PULSE_SPAN) * 8);
        // Unknown fields are refused.
        let bad = json.replace("\"seed\": 3", "\"seed\": 3, \"sede\": 4");
        assert!(serde_json::from_str::<GeneratorSpec>(&bad).is_err());
        let tones = r#"{"kind":"tones","common":{"sample_rate":8000,"length":{"seconds":1}},
            "tones":[{"frequency":1000,"amplitude":0.5}]}"#;
        let g = generate(&serde_json::from_str(tones).unwrap()).unwrap();
        assert_eq!(g.signal.len(), 8000);
        assert_eq!(g.description, "tone 1000 Hz");
    }

    #[test]
    fn generate_is_deterministic() {
        for mut spec in all_specs() {
            spec.common_mut().channel = ChannelParams {
                frequency_offset: 3.0,
                multipath: vec![
                    Tap {
                        delay: 0.0,
                        gain: 1.0,
                        phase: 0.0,
                    },
                    Tap {
                        delay: 0.000_7,
                        gain: 0.4,
                        phase: 0.5,
                    },
                ],
                noise: Some(NoiseLevel::SnrFullBand { db: 20.0 }),
                ..ChannelParams::default()
            };
            spec.common_mut().length = Length::Seconds(0.5);
            let a = generate(&spec).unwrap();
            let b = generate(&spec).unwrap();
            assert_eq!(bits(&a.signal), bits(&b.signal), "{}", a.description);
            assert_eq!(a.truth, b.truth);
            spec.common_mut().seed = 99;
            let c = generate(&spec).unwrap();
            assert_ne!(bits(&a.signal), bits(&c.signal), "{}", a.description);
        }
    }

    #[test]
    fn ground_truth_matches_the_signal() {
        let spec = GeneratorSpec::Linear(LinearSpec {
            common: Common {
                length: Length::Symbols(50),
                ..Common::new(9600.0, 1.0)
            },
            constellation: Constellation::Qpsk,
            baud: 1200.0,
            carrier: 0.0,
            amplitude: 1.0,
            shaping: Shaping::RaisedCosine { alpha: 0.35 },
            differential: false,
            complex: true,
        });
        let g = generate(&spec).unwrap();
        let Samples::Complex(z) = g.signal.samples() else {
            panic!("expected complex")
        };
        let t0 = g.truth.first_symbol_time.unwrap();
        let pts = Constellation::Qpsk.points();
        for (k, s) in g.truth.symbols.iter().enumerate() {
            let i = ((t0 + k as f64 / 1200.0) * 9600.0).round() as usize;
            let v = Complex64::new(f64::from(z[i].re), f64::from(z[i].im));
            assert_eq!(linear::nearest_in(&pts, v), *s);
        }
        let o = generate(&GeneratorSpec::preset("cis12_like").unwrap()).unwrap();
        let t = o.truth.ofdm.unwrap();
        assert_eq!(t.bins.len(), 12);
        assert_eq!((t.lu, t.lg), (36, 24));
        assert!((o.truth.symbol_rate.unwrap() - 120.0).abs() < 1e-12);
    }

    #[test]
    fn ofdm_lengths_are_limited_by_the_shared_constants() {
        let with = |lu, lg| {
            let mut s = GeneratorSpec::default_for(SignalKind::Ofdm);
            if let GeneratorSpec::Ofdm(o) = &mut s {
                o.lu = lu;
                o.lg = lg;
            }
            s.plan()
        };
        let lu = with(crate::OFDM_MAX_LU + 1, 16).unwrap_err().to_string();
        assert!(lu.contains("'lu'") && lu.contains("65536"), "{lu}");
        let lg = with(64, crate::OFDM_MAX_LG + 1).unwrap_err().to_string();
        assert!(lg.contains("'lg'") && lg.contains("65535"), "{lg}");
        assert!(with(1, 0).is_err());
    }

    #[test]
    fn plan_matches_what_generate_makes() {
        let mut specs = all_specs();
        let mut ppm = GeneratorSpec::default_for(SignalKind::Fsk);
        ppm.common_mut().channel.clock_ppm = 2000.0;
        specs.push(ppm);
        let mut iq = GeneratorSpec::default_for(SignalKind::Linear);
        if let GeneratorSpec::Linear(l) = &mut iq {
            l.complex = true;
            l.carrier = 0.0;
            l.common.length = Length::Symbols(77);
        }
        specs.push(iq);
        for mut spec in specs {
            if let Length::Seconds(_) = spec.common().length {
                spec.common_mut().length = Length::Seconds(0.7);
            }
            let plan = spec.plan().unwrap();
            let g = generate(&spec).unwrap();
            assert_eq!(plan.samples, g.signal.len(), "{}", g.description);
            assert_eq!(plan.symbol_rate, g.truth.symbol_rate, "{}", g.description);
            assert!(
                (plan.occupied_bandwidth - g.truth.occupied_bandwidth).abs() < 1e-9,
                "{}",
                g.description
            );
            let sent = g
                .truth
                .ofdm
                .as_ref()
                .map_or(g.truth.symbols.len(), |o| o.values.len());
            assert_eq!(plan.symbols.unwrap_or(0), sent, "{}", g.description);
            assert_eq!(
                plan.complex,
                g.signal.kind() == sn_core::SampleKind::Complex
            );
        }
    }

    #[test]
    fn c4fm_shifted_band_uses_the_generator_edges() {
        // Default: 4800 Bd at 12 kHz, so the generator needs 12000 +- (1800 + 4800)
        // = 5400..18600 Hz between 0 Hz and Nyquist.
        let shifted = |offset: f64| {
            let mut s = GeneratorSpec::default_for(SignalKind::C4fm);
            s.common_mut().channel.frequency_offset = offset;
            s.validate()
        };
        // An unshifted carrier at 6000 Hz is refused (-600 Hz), so a -6000 Hz shift
        // of the 12 kHz carrier must be refused too.
        let mut s = GeneratorSpec::default_for(SignalKind::C4fm);
        if let GeneratorSpec::C4fm(c) = &mut s {
            c.carrier = 6000.0;
        }
        assert!(s.validate().is_err());
        let e = shifted(-6000.0).unwrap_err().to_string();
        assert!(e.contains("frequency_offset"), "{e}");
        assert!(shifted(6000.0).is_err());
        // Inside the edges the shift is accepted.
        assert!(shifted(-5000.0).is_ok());
        assert!(shifted(5000.0).is_ok());
    }

    #[test]
    fn validate_finds_what_generate_refuses() {
        let mut bad = Vec::new();
        let mut s = GeneratorSpec::default_for(SignalKind::Linear);
        if let GeneratorSpec::Linear(l) = &mut s {
            l.differential = true;
        }
        bad.push(s);
        let mut s = GeneratorSpec::default_for(SignalKind::C4fm);
        if let GeneratorSpec::C4fm(c) = &mut s {
            c.carrier = 100.0;
        }
        bad.push(s);
        let mut s = GeneratorSpec::default_for(SignalKind::Ofdm);
        if let GeneratorSpec::Ofdm(o) = &mut s {
            o.center = 100.0;
        }
        bad.push(s);
        let mut s = GeneratorSpec::default_for(SignalKind::Ofdm);
        if let GeneratorSpec::Ofdm(o) = &mut s {
            o.model = OfdmModel::C;
        }
        bad.push(s);
        let mut s = GeneratorSpec::default_for(SignalKind::Tones);
        s.common_mut().channel.noise = Some(NoiseLevel::Snr {
            db: 10.0,
            bandwidth: Some(1e6),
        });
        bad.push(s);
        // Es/N0 needs a symbol rate, which tones and noise do not have.
        for kind in [SignalKind::Tones, SignalKind::Noise] {
            let mut s = GeneratorSpec::default_for(kind);
            s.common_mut().channel.noise = Some(NoiseLevel::EsN0 {
                db: 10.0,
                symbol_rate: None,
            });
            bad.push(s);
        }
        // An in-band SNR wider than a complex signal can be.
        let mut s = GeneratorSpec::default_for(SignalKind::Noise);
        if let GeneratorSpec::Noise(n) = &mut s {
            n.complex = true;
        }
        s.common_mut().channel.noise = Some(NoiseLevel::Snr {
            db: 10.0,
            bandwidth: Some(9000.0),
        });
        bad.push(s);
        // A real 1000 Hz tone moved down by 1500 Hz would fold over 0 Hz.
        let mut s = GeneratorSpec::default_for(SignalKind::Tones);
        s.common_mut().channel.frequency_offset = -1500.0;
        bad.push(s);
        // 800 Hz/s over 10 s takes the shift far past Nyquist, even for noise.
        let mut s = GeneratorSpec::default_for(SignalKind::Noise);
        s.common_mut().channel.drift = 800.0;
        bad.push(s);
        for spec in bad {
            let v = spec.validate().unwrap_err().to_string();
            let g = generate(&spec).unwrap_err().to_string();
            assert_eq!(v, g);
        }
        // The same checks as plan() run on their own when the channel is applied.
        let s = GeneratorSpec::default_for(SignalKind::Tones);
        let mut clean = s.clone();
        clean.common_mut().channel = ChannelParams::default();
        let g = generate(&clean).unwrap();
        let p = ChannelParams {
            noise: Some(NoiseLevel::EsN0 {
                db: 10.0,
                symbol_rate: None,
            }),
            ..ChannelParams::default()
        };
        let e = crate::apply_channel(&g.signal, &p, 1, &ChannelHints::default()).unwrap_err();
        assert!(e.to_string().contains("symbol rate"), "{e}");
    }

    #[test]
    fn oversized_clock_output_is_refused_before_generating() {
        let fs = 8000.0;
        let mut spec = GeneratorSpec::default_for(SignalKind::Fsk);
        if let GeneratorSpec::Fsk(f) = &mut spec {
            f.shaping = FskShaping::Gaussian { bt: 0.1 };
        }
        spec.common_mut().length = Length::Seconds((MAX_SAMPLES - 1000) as f64 / fs);
        spec.common_mut().channel.clock_ppm = crate::MAX_CLOCK_PPM;
        let e = spec.plan().unwrap_err().to_string();
        assert!(e.contains("clock_ppm") && e.contains("2^28"), "{e}");
        // generate checks the plan first, so it fails at once instead of after
        // minutes of GFSK synthesis.
        let t = std::time::Instant::now();
        assert_eq!(generate(&spec).unwrap_err().to_string(), e);
        assert!(t.elapsed().as_secs() < 5);
    }

    #[test]
    fn stop_cancels_generation() {
        for spec in all_specs() {
            let e = generate_with_stop(&spec, &|| true).unwrap_err();
            assert!(matches!(e, crate::Error::Cancelled), "{spec:?}: {e}");
        }
        let mut spec = GeneratorSpec::default_for(SignalKind::Tones);
        spec.common_mut().channel.frequency_offset = 10.0;
        let calls = std::sync::atomic::AtomicUsize::new(0);
        // Let the generator finish, then stop in the channel.
        let n = 8000 * 10 / crate::STOP_EVERY + 1;
        let stop = || calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= n;
        let e = generate_with_stop(&spec, &stop).unwrap_err();
        assert!(matches!(e, crate::Error::Cancelled), "{e}");
    }

    #[test]
    fn esn0_uses_the_steady_power_of_a_short_shaped_signal() {
        // 50 symbols with a 12-symbol RRC ramp: the mean over every sample would
        // understate Es by 10 log10(62 / 50) = 0.93 dB.
        let fs = 64_000.0;
        let baud = 1000.0;
        let mut spec = GeneratorSpec::Linear(LinearSpec {
            common: Common {
                length: Length::Symbols(50),
                ..Common::new(fs, 1.0)
            },
            constellation: Constellation::Qpsk,
            baud,
            carrier: 0.0,
            amplitude: 1.0,
            shaping: Shaping::RootRaisedCosine { alpha: 0.35 },
            differential: false,
            complex: true,
        });
        let clean = generate(&spec).unwrap();
        spec.common_mut().channel.noise = Some(NoiseLevel::EsN0 {
            db: 10.0,
            symbol_rate: None,
        });
        let noisy = generate(&spec).unwrap();
        let (Samples::Complex(a), Samples::Complex(b)) =
            (clean.signal.samples(), noisy.signal.samples())
        else {
            panic!("expected complex")
        };
        // Steady power from the clean samples between the first and last symbol centre.
        let t0 = clean.truth.first_symbol_time.unwrap();
        let lo = (t0 * fs) as usize;
        let hi = ((t0 + 49.0 / baud) * fs) as usize;
        let p: f64 = a[lo..=hi]
            .iter()
            .map(|v| f64::from(v.norm_sqr()))
            .sum::<f64>()
            / (hi - lo + 1) as f64;
        let want = p / baud / 10.0 * fs / 2.0; // N0 fs / 2 per component
        let var = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| f64::from((y - x).norm_sqr()) / 2.0)
            .sum::<f64>()
            / a.len() as f64;
        let err_db = 10.0 * (var / want).log10();
        assert!(err_db.abs() < 0.25, "noise off by {err_db:.2} dB");
    }

    #[test]
    fn complex_output_gets_the_centre_frequency() {
        let mut spec = GeneratorSpec::default_for(SignalKind::Noise);
        if let GeneratorSpec::Noise(n) = &mut spec {
            n.complex = true;
            n.common.center_frequency = 7_100_000.0;
        }
        let g = generate(&spec).unwrap();
        assert!((g.signal.center_frequency() - 7_100_000.0).abs() < 1e-6);
        assert!(g.description.ends_with("I/Q"));
    }

    #[test]
    fn validation_errors_are_clear() {
        let mut spec = GeneratorSpec::default_for(SignalKind::Tones);
        spec.common_mut().length = Length::Seconds(1e12);
        assert!(generate(&spec).unwrap_err().to_string().contains("2^28"));
        spec.common_mut().length = Length::Symbols(10);
        assert!(generate(&spec).unwrap_err().to_string().contains("seconds"));
        let mut spec = GeneratorSpec::default_for(SignalKind::Linear);
        if let GeneratorSpec::Linear(l) = &mut spec {
            l.differential = true;
        }
        assert!(
            generate(&spec)
                .unwrap_err()
                .to_string()
                .contains("PSK only")
        );
        let mut spec = GeneratorSpec::default_for(SignalKind::Linear);
        spec.common_mut().length = Length::Seconds(0.005);
        assert!(generate(&spec).unwrap_err().to_string().contains("ramp"));
        let mut spec = GeneratorSpec::default_for(SignalKind::Ofdm);
        if let GeneratorSpec::Ofdm(o) = &mut spec {
            o.lu = usize::MAX;
        }
        assert!(generate(&spec).is_err());
        let mut spec = GeneratorSpec::default_for(SignalKind::Fsk);
        spec.common_mut().sample_rate = f64::NAN;
        assert!(generate(&spec).is_err());
        let mut spec = GeneratorSpec::default_for(SignalKind::Fsk);
        spec.common_mut().length = Length::Symbols(usize::MAX);
        assert!(generate(&spec).is_err());
        let mut spec = GeneratorSpec::default_for(SignalKind::Noise);
        spec.common_mut().channel.clock_ppm = 1e9;
        assert!(generate(&spec).is_err());
    }
}
