//! The number fields of the signal generator and its calculators: where each
//! value lives in the spec (or the calculators' inputs), its label, its range
//! and how it is written. `sa-app` edits them with drag values; Windows 95 has
//! edit fields, so a value is read from the text as it is typed and a text
//! that is not a number in range is named under the form until it is fixed.

use sn_synth::{C4fmSpec, FskShaping, GeneratorSpec, Length, NoiseLevel, PilotPattern, Shaping};
use sn_ui_core::generator::{MAX_K, MAX_K_UNCERTAINTY_PCT, trim};
use sn_ui_core::t;

use super::calc::CalcInputs;

/// A number field of the calculators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum CalcNum {
    Fd,
    Lu,
    Lg,
    InvBr,
    InvSpacing,
    InvFd,
    CandBr,
    CandK,
    CandLuMin,
    CandLuMax,
    CandTol,
    KBr,
    KSpacing,
    KValue,
    KPct,
    KMaxDen,
    KCount,
}

/// A number field of the generator window or of the calculators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Num {
    SampleRate,
    Seconds,
    Symbols,
    Seed,
    CenterFrequency,
    Offset,
    Drift,
    Clock,
    NoiseDb,
    Bandwidth,
    Clip,
    Amplitude,
    Baud,
    Spacing,
    Center,
    Carrier,
    Order,
    Bt,
    Alpha,
    Lu,
    Lg,
    Channels,
    FirstBin,
    PilotStep,
    PilotOffset,
    ToneFrequency(usize),
    ToneAmplitude(usize),
    TonePhase(usize),
    TapDelay(usize),
    TapGain(usize),
    TapPhase(usize),
    Calc(CalcNum),
}

/// What kind of number a field takes and its limits.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// A decimal number.
    Real(f64, f64),
    /// A whole number.
    Whole(i64, i64),
    /// A seed (any 64-bit unsigned number).
    Seed,
}

impl Num {
    /// The control id of the field (the focus id; the windows are single).
    pub(crate) fn id(self) -> String {
        format!("ocg.{self:?}")
    }

    /// The label the field is named by in messages.
    pub(crate) fn label(self) -> String {
        match self {
            Self::SampleRate => t!("gen-sample-rate"),
            Self::Seconds => t!("gen-length-seconds"),
            Self::Symbols => t!("gen-length-symbols"),
            Self::Seed => t!("gen-seed"),
            Self::CenterFrequency => t!("gen-center-frequency"),
            Self::Offset => t!("gen-ch-offset"),
            Self::Drift => t!("gen-ch-drift"),
            Self::Clock => t!("gen-ch-clock"),
            Self::NoiseDb => t!("gen-ch-noise"),
            Self::Bandwidth => t!("gen-ch-bandwidth"),
            Self::Clip => t!("gen-ch-clip"),
            Self::Amplitude => t!("gen-amplitude-rms"),
            Self::Baud => t!("gen-baud"),
            Self::Spacing => t!("gen-fsk-spacing"),
            Self::Center => t!("gen-center"),
            Self::Carrier => t!("gen-carrier"),
            Self::Order => t!("gen-fsk-order"),
            Self::Bt => t!("gen-fsk-bt"),
            Self::Alpha => t!("gen-alpha"),
            Self::Lu => t!("gen-ofdm-lu"),
            Self::Lg => t!("gen-ofdm-lg"),
            Self::Channels => t!("gen-ofdm-channels"),
            Self::FirstBin => t!("gen-ofdm-first-bin"),
            Self::PilotStep => t!("gen-ofdm-step"),
            Self::PilotOffset => t!("gen-ofdm-offset"),
            Self::ToneFrequency(i) => format!("{} {}", t!("gen-tones-frequency"), i + 1),
            Self::ToneAmplitude(i) => format!("{} {}", t!("gen-amplitude-peak"), i + 1),
            Self::TonePhase(i) | Self::TapPhase(i) => format!("{} {}", t!("gen-phase-rad"), i + 1),
            Self::TapDelay(i) => format!("{} {}", t!("gen-ch-delay"), i + 1),
            Self::TapGain(i) => format!("{} {}", t!("gen-ch-gain"), i + 1),
            Self::Calc(c) => calc_label(c),
        }
    }

    /// The unit written after the field, if any.
    pub(crate) fn unit(self) -> &'static str {
        match self {
            Self::SampleRate
            | Self::CenterFrequency
            | Self::Offset
            | Self::Bandwidth
            | Self::Spacing
            | Self::Center
            | Self::Carrier
            | Self::ToneFrequency(_)
            | Self::Calc(CalcNum::Fd | CalcNum::InvSpacing | CalcNum::InvFd | CalcNum::KSpacing) => {
                "Hz"
            }
            Self::Seconds => "s",
            Self::Drift => "Hz/s",
            Self::Clock => "ppm",
            Self::NoiseDb => "dB",
            Self::Baud | Self::Calc(CalcNum::InvBr | CalcNum::CandBr | CalcNum::KBr) => "Bd",
            Self::TapDelay(_) => "ms",
            Self::Calc(CalcNum::KPct) => "%",
            _ => "",
        }
    }

    fn kind(self) -> Kind {
        use Kind::{Real, Whole};
        let big = 1e12;
        match self {
            Self::SampleRate => Real(1.0, 1e10),
            Self::Seconds => Real(1e-6, 1e7),
            Self::Symbols => Whole(1, sn_synth::MAX_SAMPLES as i64),
            Self::Seed => Kind::Seed,
            Self::CenterFrequency | Self::Center | Self::Carrier | Self::ToneFrequency(_) => {
                Real(-big, big)
            }
            Self::Offset | Self::Drift => Real(-1e9, 1e9),
            Self::Clock => Real(-sn_synth::MAX_CLOCK_PPM, sn_synth::MAX_CLOCK_PPM),
            Self::NoiseDb => Real(-100.0, 200.0),
            Self::Bandwidth => Real(1e-3, 1e10),
            Self::Clip => Real(1e-6, 1e6),
            Self::Amplitude | Self::ToneAmplitude(_) => Real(0.0, 1e6),
            Self::Baud | Self::Spacing => Real(0.001, 1e9),
            Self::Order => Whole(2, 256),
            Self::Bt => Real(0.1, 10.0),
            Self::Alpha => Real(0.01, 1.0),
            Self::Lu => Whole(2, sn_synth::OFDM_MAX_LU as i64),
            Self::Lg => Whole(0, sn_synth::OFDM_MAX_LG as i64),
            Self::Channels | Self::PilotStep => Whole(1, 65_536),
            Self::FirstBin => Whole(-65_536, 65_536),
            Self::PilotOffset => Whole(0, 65_535),
            Self::TonePhase(_) | Self::TapGain(_) | Self::TapPhase(_) => Real(-1e3, 1e3),
            Self::TapDelay(_) => Real(0.0, sn_synth::MAX_DELAY * 1e3),
            Self::Calc(c) => calc_kind(c),
        }
    }

    /// The field's value as text, or `None` when the spec has no such field.
    pub(crate) fn text(self, spec: &GeneratorSpec, calc: &CalcInputs) -> Option<String> {
        if let Self::Seed = self {
            return Some(spec.common().seed.to_string());
        }
        let v = self.value(spec, calc)?;
        Some(match self.kind() {
            Kind::Whole(..) => format!("{v:.0}"),
            _ => trim(v, 6),
        })
    }

    /// Read `text` into the spec (or the calculators). The error names the
    /// field and says what it takes.
    pub(crate) fn apply(
        self,
        text: &str,
        spec: &mut GeneratorSpec,
        calc: &mut CalcInputs,
    ) -> Result<(), String> {
        let text = text.trim();
        let bad = |why: String| t!("gen-field-error", field = self.label(), error = why);
        match self.kind() {
            Kind::Seed => {
                let s = text
                    .parse::<u64>()
                    .map_err(|_| bad(t!("gui-gen-not-whole", text = text.to_owned())))?;
                spec.common_mut().seed = s;
                Ok(())
            }
            Kind::Whole(lo, hi) => {
                let v = text
                    .parse::<i64>()
                    .map_err(|_| bad(t!("gui-gen-not-whole", text = text.to_owned())))?;
                if !(lo..=hi).contains(&v) {
                    return Err(bad(range_text(lo as f64, hi as f64)));
                }
                self.set(spec, calc, v as f64);
                Ok(())
            }
            Kind::Real(lo, hi) => {
                let v = sn_ui_core::format::parse_number(text)
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| bad(t!("gui-gen-not-number", text = text.to_owned())))?;
                if !(lo..=hi).contains(&v) {
                    return Err(bad(range_text(lo, hi)));
                }
                self.set(spec, calc, v);
                Ok(())
            }
        }
    }

    /// The value in the spec or the calculators.
    fn value(self, spec: &GeneratorSpec, calc: &CalcInputs) -> Option<f64> {
        let c = spec.common();
        let ch = &c.channel;
        Some(match self {
            Self::SampleRate => c.sample_rate,
            Self::Seconds => match c.length {
                Length::Seconds(s) => s,
                Length::Symbols(_) => return None,
            },
            Self::Symbols => match c.length {
                Length::Symbols(n) => n as f64,
                Length::Seconds(_) => return None,
            },
            // The seed is read and written only as a u64 (`text` and
            // `apply`): through f64 it would lose bits above 2^53.
            Self::Seed => return None,
            Self::CenterFrequency => c.center_frequency,
            Self::Offset => ch.frequency_offset,
            Self::Drift => ch.drift,
            Self::Clock => ch.clock_ppm,
            Self::NoiseDb => match ch.noise? {
                NoiseLevel::SnrFullBand { db }
                | NoiseLevel::Snr { db, .. }
                | NoiseLevel::EsN0 { db, .. } => db,
            },
            Self::Bandwidth => match ch.noise? {
                NoiseLevel::Snr { bandwidth, .. } => bandwidth?,
                _ => return None,
            },
            Self::Clip => ch.clip?,
            Self::TapDelay(i) => ch.multipath.get(i)?.delay * 1e3,
            Self::TapGain(i) => ch.multipath.get(i)?.gain,
            Self::TapPhase(i) => ch.multipath.get(i)?.phase,
            Self::Calc(n) => calc.get(n),
            _ => kind_value(self, spec)?,
        })
    }

    /// Write `v` (in range) into the spec or the calculators.
    fn set(self, spec: &mut GeneratorSpec, calc: &mut CalcInputs, v: f64) {
        if let Self::Calc(n) = self {
            calc.set(n, v);
            return;
        }
        if set_kind_value(self, spec, v) {
            return;
        }
        let c = spec.common_mut();
        let ch = &mut c.channel;
        match self {
            Self::SampleRate => c.sample_rate = v,
            Self::Seconds => c.length = Length::Seconds(v),
            Self::Symbols => c.length = Length::Symbols(v as usize),
            Self::CenterFrequency => c.center_frequency = v,
            Self::Offset => ch.frequency_offset = v,
            Self::Drift => ch.drift = v,
            Self::Clock => ch.clock_ppm = v,
            Self::NoiseDb => {
                if let Some(
                    NoiseLevel::SnrFullBand { db }
                    | NoiseLevel::Snr { db, .. }
                    | NoiseLevel::EsN0 { db, .. },
                ) = &mut ch.noise
                {
                    *db = v;
                }
            }
            Self::Bandwidth => {
                if let Some(NoiseLevel::Snr { bandwidth, .. }) = &mut ch.noise {
                    *bandwidth = Some(v);
                }
            }
            Self::Clip => ch.clip = Some(v),
            Self::TapDelay(i) => {
                if let Some(t) = ch.multipath.get_mut(i) {
                    t.delay = v / 1e3;
                }
            }
            Self::TapGain(i) => {
                if let Some(t) = ch.multipath.get_mut(i) {
                    t.gain = v;
                }
            }
            Self::TapPhase(i) => {
                if let Some(t) = ch.multipath.get_mut(i) {
                    t.phase = v;
                }
            }
            _ => {}
        }
    }
}

/// `between { $min } and { $max }`.
fn range_text(lo: f64, hi: f64) -> String {
    t!(
        "gui-gen-range",
        min = sn_ui_core::format::dec_trim(lo, 6),
        max = sn_ui_core::format::dec_trim(hi, 6)
    )
}

/// The values of the kind's own fields.
fn kind_value(n: Num, spec: &GeneratorSpec) -> Option<f64> {
    use Num as N;
    Some(match (spec, n) {
        (GeneratorSpec::Tones(s), N::ToneFrequency(i)) => s.tones.get(i)?.frequency,
        (GeneratorSpec::Tones(s), N::ToneAmplitude(i)) => s.tones.get(i)?.amplitude,
        (GeneratorSpec::Tones(s), N::TonePhase(i)) => s.tones.get(i)?.phase,
        (GeneratorSpec::Noise(s), N::Amplitude) => s.amplitude,
        (GeneratorSpec::Fsk(s), N::Order) => s.order as f64,
        (GeneratorSpec::Fsk(s), N::Baud) => s.baud,
        (GeneratorSpec::Fsk(s), N::Spacing) => s.spacing,
        (GeneratorSpec::Fsk(s), N::Center) => s.center,
        (GeneratorSpec::Fsk(s), N::Amplitude) => s.amplitude,
        (GeneratorSpec::Fsk(s), N::Bt) => match s.shaping {
            FskShaping::Gaussian { bt } => bt,
            FskShaping::None => return None,
        },
        (GeneratorSpec::Linear(s), N::Baud) => s.baud,
        (GeneratorSpec::Linear(s), N::Carrier) => s.carrier,
        (GeneratorSpec::Linear(s), N::Amplitude) => s.amplitude,
        (GeneratorSpec::Linear(s), N::Alpha) => s.shaping.alpha()?,
        (GeneratorSpec::C4fm(s), n) => c4fm_value(s, n)?,
        (GeneratorSpec::Ofdm(o), N::Lu) => o.lu as f64,
        (GeneratorSpec::Ofdm(o), N::Lg) => o.lg as f64,
        (GeneratorSpec::Ofdm(o), N::Channels) => o.channels as f64,
        (GeneratorSpec::Ofdm(o), N::FirstBin) => o.first_bin as f64,
        (GeneratorSpec::Ofdm(o), N::Amplitude) => o.amplitude,
        (GeneratorSpec::Ofdm(o), N::Center) => o.center,
        (GeneratorSpec::Ofdm(o), N::PilotStep) => match o.pilots {
            PilotPattern::Every { step, .. } => step as f64,
            _ => return None,
        },
        (GeneratorSpec::Ofdm(o), N::PilotOffset) => match o.pilots {
            PilotPattern::Every { offset, .. } => offset as f64,
            _ => return None,
        },
        _ => return None,
    })
}

fn c4fm_value(s: &C4fmSpec, n: Num) -> Option<f64> {
    Some(match n {
        Num::Baud => s.baud,
        Num::Carrier => s.carrier,
        Num::Amplitude => s.amplitude,
        _ => return None,
    })
}

/// Write a kind's own field; `false` when `n` is not one of them.
fn set_kind_value(n: Num, spec: &mut GeneratorSpec, v: f64) -> bool {
    use Num as N;
    let whole = v as usize;
    match (spec, n) {
        (GeneratorSpec::Tones(s), N::ToneFrequency(i)) => {
            if let Some(t) = s.tones.get_mut(i) {
                t.frequency = v;
            }
        }
        (GeneratorSpec::Tones(s), N::ToneAmplitude(i)) => {
            if let Some(t) = s.tones.get_mut(i) {
                t.amplitude = v;
            }
        }
        (GeneratorSpec::Tones(s), N::TonePhase(i)) => {
            if let Some(t) = s.tones.get_mut(i) {
                t.phase = v;
            }
        }
        (GeneratorSpec::Noise(s), N::Amplitude) => s.amplitude = v,
        (GeneratorSpec::Fsk(s), N::Order) => s.order = whole,
        (GeneratorSpec::Fsk(s), N::Baud) => s.baud = v,
        (GeneratorSpec::Fsk(s), N::Spacing) => s.spacing = v,
        (GeneratorSpec::Fsk(s), N::Center) => s.center = v,
        (GeneratorSpec::Fsk(s), N::Amplitude) => s.amplitude = v,
        (GeneratorSpec::Fsk(s), N::Bt) => s.shaping = FskShaping::Gaussian { bt: v },
        (GeneratorSpec::Linear(s), N::Baud) => s.baud = v,
        (GeneratorSpec::Linear(s), N::Carrier) => s.carrier = v,
        (GeneratorSpec::Linear(s), N::Amplitude) => s.amplitude = v,
        (GeneratorSpec::Linear(s), N::Alpha) => match &mut s.shaping {
            Shaping::RaisedCosine { alpha } | Shaping::RootRaisedCosine { alpha } => *alpha = v,
            Shaping::Rectangular => {}
        },
        (GeneratorSpec::C4fm(s), N::Baud) => s.baud = v,
        (GeneratorSpec::C4fm(s), N::Carrier) => s.carrier = v,
        (GeneratorSpec::C4fm(s), N::Amplitude) => s.amplitude = v,
        (GeneratorSpec::Ofdm(o), N::Lu) => o.lu = whole,
        (GeneratorSpec::Ofdm(o), N::Lg) => o.lg = whole,
        (GeneratorSpec::Ofdm(o), N::Channels) => o.channels = whole,
        (GeneratorSpec::Ofdm(o), N::FirstBin) => o.first_bin = v as i64,
        (GeneratorSpec::Ofdm(o), N::Amplitude) => o.amplitude = v,
        (GeneratorSpec::Ofdm(o), N::Center) => o.center = v,
        (GeneratorSpec::Ofdm(o), N::PilotStep) => {
            if let PilotPattern::Every { step, .. } = &mut o.pilots {
                *step = whole;
            }
        }
        (GeneratorSpec::Ofdm(o), N::PilotOffset) => {
            if let PilotPattern::Every { offset, .. } = &mut o.pilots {
                *offset = whole;
            }
        }
        _ => return false,
    }
    true
}

fn calc_label(c: CalcNum) -> String {
    match c {
        CalcNum::Fd | CalcNum::InvFd => "Fd".to_owned(),
        CalcNum::Lu => "LU".to_owned(),
        CalcNum::Lg => "LG".to_owned(),
        CalcNum::InvBr | CalcNum::CandBr | CalcNum::KBr => "Br".to_owned(),
        CalcNum::InvSpacing | CalcNum::KSpacing => t!("gen-calc-spacing"),
        CalcNum::CandK | CalcNum::KValue => "K".to_owned(),
        // Technical names, as LU and LG: the refusal says which field.
        CalcNum::CandLuMin => "LU min".to_owned(),
        CalcNum::CandLuMax => "LU max".to_owned(),
        CalcNum::CandTol => t!("gen-calc-tolerance"),
        CalcNum::KPct => t!("gen-calc-uncertainty"),
        CalcNum::KMaxDen => t!("gen-calc-max-den"),
        CalcNum::KCount => t!("gen-calc-count"),
    }
}

fn calc_kind(c: CalcNum) -> Kind {
    use Kind::{Real, Whole};
    match c {
        CalcNum::Fd | CalcNum::InvFd => Real(1e-3, 1e10),
        CalcNum::Lu => Real(1.0, 1e9),
        CalcNum::Lg => Real(0.0, 1e9),
        CalcNum::InvBr
        | CalcNum::InvSpacing
        | CalcNum::CandBr
        | CalcNum::KBr
        | CalcNum::KSpacing => Real(1e-6, 1e9),
        CalcNum::CandK | CalcNum::KValue => Real(0.0, MAX_K),
        CalcNum::CandLuMin | CalcNum::CandLuMax => Whole(1, 1 << 30),
        CalcNum::CandTol => Real(0.0, 0.5),
        CalcNum::KPct => Real(0.0, MAX_K_UNCERTAINTY_PCT),
        CalcNum::KMaxDen => Whole(1, i64::from(sn_synth::calc::MAX_K_DENOMINATOR)),
        CalcNum::KCount => Whole(1, super::calc::TABLE_ROWS as i64),
    }
}

/// The number fields shown for a spec of `kind` in the editor's own section
/// (for the tests: every one has a value).
#[cfg(test)]
pub(crate) fn kind_fields(kind: sn_synth::SignalKind) -> &'static [Num] {
    use sn_synth::SignalKind;
    match kind {
        SignalKind::Tones => &[Num::ToneFrequency(0), Num::ToneAmplitude(0)],
        SignalKind::Noise => &[Num::Amplitude],
        SignalKind::Fsk => &[
            Num::Order,
            Num::Baud,
            Num::Spacing,
            Num::Center,
            Num::Amplitude,
        ],
        SignalKind::Linear | SignalKind::C4fm => &[Num::Baud, Num::Carrier, Num::Amplitude],
        SignalKind::Ofdm => &[
            Num::Lu,
            Num::Lg,
            Num::Channels,
            Num::FirstBin,
            Num::Amplitude,
            Num::Center,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_field_reads_and_writes() {
        let mut calc = CalcInputs::default();
        for kind in sn_synth::SignalKind::ALL {
            let mut spec = GeneratorSpec::default_for(kind);
            for n in kind_fields(kind) {
                let text = n.text(&spec, &calc);
                assert!(text.is_some(), "{n:?} of {kind:?}");
                let text = text.unwrap_or_default();
                assert!(n.apply(&text, &mut spec, &mut calc).is_ok(), "{n:?}");
                assert_eq!(n.text(&spec, &calc), Some(text));
            }
        }
    }

    #[test]
    fn bad_texts_are_named_and_change_nothing() {
        let mut calc = CalcInputs::default();
        let mut spec = GeneratorSpec::default_for(sn_synth::SignalKind::Ofdm);
        let before = spec.clone();
        let e = Num::Lu.apply("1.5", &mut spec, &mut calc).unwrap_err();
        assert!(e.starts_with("LU:"), "{e}");
        let e = Num::SampleRate
            .apply("x", &mut spec, &mut calc)
            .unwrap_err();
        assert!(e.contains("\"x\""), "{e}");
        let e = Num::SampleRate
            .apply("0", &mut spec, &mut calc)
            .unwrap_err();
        assert!(e.contains("between"), "{e}");
        assert_eq!(spec, before);
        assert!(Num::Lu.apply(" 128 ", &mut spec, &mut calc).is_ok());
        assert!(matches!(&spec, GeneratorSpec::Ofdm(o) if o.lu == 128));
        assert!(
            Num::Calc(CalcNum::Fd)
                .apply("8000", &mut spec, &mut calc)
                .is_ok()
        );
        assert!((calc.fd - 8000.0).abs() < 1e-9);
        assert!(
            Num::Seed
                .apply("18446744073709551615", &mut spec, &mut calc)
                .is_ok()
        );
        assert_eq!(spec.common().seed, u64::MAX);
    }
}
