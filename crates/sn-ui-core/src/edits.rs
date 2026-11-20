//! Parameters of the Signal menu's operations: the fields of SA's small
//! parameter dialog, their defaults, valid ranges, hints and parsing, and the
//! save formats.

use std::path::{Path, PathBuf};

use sn_core::Signal;
use sn_dsp::ops::{Edit, EqMode};

use crate::format;
use crate::t;

/// Output formats offered by File > Save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFormat {
    /// WAV, 16-bit PCM.
    WavPcm16,
    /// WAV, 32-bit float.
    WavFloat,
    /// SigMF (metadata and data files).
    Sigmf,
    /// Headerless 32-bit float samples.
    RawF32,
}

impl SaveFormat {
    /// Every format, in menu order.
    pub const ALL: [SaveFormat; 4] = [Self::WavPcm16, Self::WavFloat, Self::Sigmf, Self::RawF32];

    /// The menu text.
    pub fn label(self) -> String {
        match self {
            Self::WavPcm16 => t!("edits-save-wav-pcm16"),
            Self::WavFloat => t!("edits-save-wav-float"),
            Self::Sigmf => t!("edits-save-sigmf"),
            Self::RawF32 => t!("edits-save-raw-f32"),
        }
    }

    /// The file name extension.
    pub fn extension(self) -> &'static str {
        match self {
            Self::WavPcm16 | Self::WavFloat => "wav",
            Self::Sigmf => "sigmf-meta",
            Self::RawF32 => "f32",
        }
    }

    /// Write `signal` to `path` in this format; returns the path written (SigMF
    /// writes a data file next to its metadata). `description` goes into the
    /// SigMF metadata; `progress` gets (done, total) and returns true to stop.
    pub fn write(
        self,
        path: &Path,
        signal: &Signal,
        description: &str,
        progress: &dyn Fn(u64, u64) -> bool,
    ) -> Result<PathBuf, sn_io::Error> {
        match self {
            Self::WavPcm16 => {
                sn_io::write_wav_with_progress(path, signal, sn_io::WavFormat::Pcm16, progress)
                    .map(|()| path.to_path_buf())
            }
            Self::WavFloat => {
                sn_io::write_wav_with_progress(path, signal, sn_io::WavFormat::Float32, progress)
                    .map(|()| path.to_path_buf())
            }
            Self::Sigmf => sn_io::write_sigmf_with_progress(path, signal, description, progress),
            Self::RawF32 => sn_io::write_raw_with_progress(
                path,
                signal,
                sn_io::SampleFormat::F32,
                sn_io::Endian::Little,
                progress,
            )
            .map(|()| path.to_path_buf()),
        }
    }
}

/// A file name stem from a window name (`test(0)(c1)`): characters a file
/// name may not hold become `_`.
pub fn file_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '(' | ')') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// A measured value and where it came from, for pre-filling and labelling fields.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredValue {
    /// The value.
    pub value: f64,
    /// E.g. "n4, measure line".
    pub from: String,
    /// The value was measured on a frequency axis that could not be mapped onto the
    /// signal it pre-fills (another branch, or a detector in between): check it.
    pub other_axis: bool,
}

impl MeasuredValue {
    /// Where the value came from, with a warning when it is on another axis.
    pub fn source(&self) -> String {
        if self.other_axis {
            t!("edits-measured-other-axis", from = self.from.as_str())
        } else {
            self.from.clone()
        }
    }
}

/// A line under the dialog fields saying where a pre-filled value came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// The line's text.
    pub text: String,
    /// Shown in the warning colour.
    pub warn: bool,
}

impl From<String> for Note {
    fn from(text: String) -> Self {
        Self { text, warn: false }
    }
}

/// Longest median window the dialog offers.
pub const MAX_MEDIAN: usize = 301;

/// Lowest low-pass cutoff SA derives from a measured Br, in Hz.
const MIN_LOWPASS_HZ: f64 = 30.0;

/// Pre-filled low-pass cutoff and its note, from a measured Br (value, source) if any.
///
/// SA: a measured Br sets the low-pass cutoff after a detector (at least 30 Hz). A
/// raised cutoff is not the Br the method calls for, so its note is a warning. The
/// cutoff always passes the dialog's own check (below half the sample rate): a Br
/// at or above it is lowered, with a warning.
pub fn lowpass_default(fs: f64, br: Option<(f64, &str)>) -> (f64, Note) {
    let below_nyquist = fs / 2.0 * 0.99;
    match br {
        Some((value, from)) if value.max(MIN_LOWPASS_HZ) >= fs / 2.0 => (
            below_nyquist,
            Note {
                text: t!(
                    "edits-note-cutoff-lowered",
                    from = from,
                    br = format::rate(value)
                ),
                warn: true,
            },
        ),
        Some((value, from)) if value < MIN_LOWPASS_HZ => (
            MIN_LOWPASS_HZ,
            Note {
                text: t!(
                    "edits-note-cutoff-raised",
                    from = from,
                    br = format::rate(value)
                ),
                warn: true,
            },
        ),
        Some((value, from)) => (value, t!("edits-note-cutoff-br", from = from).into()),
        None => {
            let guess = (fs / 8.0).round();
            let guess = if guess > 0.0 { guess } else { fs / 8.0 };
            (guess, t!("edits-note-cutoff-guessed").into())
        }
    }
}

/// Pre-filled median window and its note for a measured Br of `br` Bd from `from`.
///
/// A window capped below 1/3 symbol is not what the method calls for, so its note is a
/// warning.
pub fn median_default(fs: f64, br: f64, from: &str) -> Option<(usize, Note)> {
    median_window(fs, br).map(|(w, capped)| {
        let note = if capped {
            Note {
                text: t!("edits-note-window-capped", from = from, count = MAX_MEDIAN),
                warn: true,
            }
        } else {
            t!("edits-note-window-br", from = from).into()
        };
        (w, note)
    })
}

/// Median window covering a third of a symbol at `baud` (odd, 1..=[`MAX_MEDIAN`]), and whether
/// it had to be capped below that third.
pub fn median_window(fs: f64, baud: f64) -> Option<(usize, bool)> {
    let w = fs / baud / 3.0;
    if !w.is_finite() || w <= 0.0 {
        return None;
    }
    let raw = w.round();
    let capped = raw > MAX_MEDIAN as f64;
    let w = (raw as usize).clamp(1, MAX_MEDIAN);
    Some((
        if w.is_multiple_of(2) {
            (w + 1).min(MAX_MEDIAN)
        } else {
            w
        },
        capped,
    ))
}

/// An operation that needs values, with the values to start the fields from.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamKind {
    /// Frequency shift.
    Shift {
        /// Shift in Hz.
        hz: f64,
    },
    /// Resampling.
    Resample {
        /// New sample rate in Hz.
        rate: f64,
    },
    /// Correction of BR.
    Baud {
        /// The measured baud rate.
        existing: f64,
        /// The wanted baud rate.
        required: f64,
    },
    /// Low-pass filter.
    Lowpass {
        /// Cutoff in Hz.
        cutoff: f64,
    },
    /// Median filter.
    Median {
        /// Window length in samples (made odd).
        window: usize,
    },
    /// Root raised cosine filter.
    Rrc {
        /// Baud rate.
        baud: f64,
        /// Roll-off.
        alpha: f64,
    },
    /// Raise to the power N.
    Power {
        /// The power.
        n: u32,
    },
    /// ISD corrector (equalizer).
    Equalize {
        /// Carrier in Hz.
        carrier: f64,
        /// Baud rate.
        baud: f64,
        /// Number of blocks.
        blocks: usize,
        /// The PSK power N, or `None` for the AM mode.
        xn: Option<u32>,
    },
}

/// Common low-pass cutoffs, as in SA's preset list.
pub const LPF_PRESETS: [f64; 10] = [
    30.0, 50.0, 75.0, 100.0, 150.0, 300.0, 600.0, 1200.0, 2400.0, 4800.0,
];

/// Highest power N the dialog accepts.
const MAX_POWER: u32 = 32;
/// Highest sample rate the resampler is asked for, in Hz.
const MAX_RATE: f64 = 1e10;
/// ISD corrector block range.
const BLOCKS: (usize, usize) = (10, 50);

/// The text of a number in a dialog field (the interface's decimal separator).
fn num_text(v: f64) -> String {
    format::dec_trim(v, 4)
}

/// Field `i` as a number.
fn num(texts: &[String], i: usize) -> Option<f64> {
    texts.get(i).and_then(|t| format::parse_number(t))
}

/// Field `i` as a whole number.
fn int(texts: &[String], i: usize) -> Option<i64> {
    texts.get(i).and_then(|t| t.trim().parse().ok())
}

/// The valid range of a field, as its hint.
fn range_hint(lo: f64, hi: f64) -> String {
    t!(
        "edits-range",
        lo = format::dec_trim(lo, 4),
        hi = format::dec_trim(hi, 4)
    )
}

/// The valid range of a numeric field: inclusive (`closed`) or exclusive at both
/// ends (`open`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bound {
    /// Lowest value.
    pub lo: f64,
    /// Highest value.
    pub hi: f64,
    /// Both ends are excluded.
    pub open: bool,
}

impl Bound {
    /// The range `lo..=hi`.
    pub fn closed(lo: f64, hi: f64) -> Self {
        Self {
            lo,
            hi,
            open: false,
        }
    }

    /// The range between `lo` and `hi`, both excluded.
    pub fn open(lo: f64, hi: f64) -> Self {
        Self { lo, hi, open: true }
    }

    /// `v` lies in the range.
    pub fn contains(&self, v: f64) -> bool {
        if self.open {
            v > self.lo && v < self.hi
        } else {
            v >= self.lo && v <= self.hi
        }
    }

    /// The range as the field's hint.
    pub fn hint(&self) -> String {
        if self.open {
            t!(
                "edits-range-open",
                lo = format::dec_trim(self.lo, 4),
                hi = format::dec_trim(self.hi, 4)
            )
        } else {
            range_hint(self.lo, self.hi)
        }
    }
}

impl ParamKind {
    /// The fields' labels.
    pub fn labels(&self) -> Vec<String> {
        match self {
            Self::Shift { .. } => vec![t!("edits-field-shift")],
            Self::Resample { .. } => vec![t!("edits-field-rate")],
            Self::Baud { .. } => vec![t!("edits-field-measured-br"), t!("edits-field-required-br")],
            Self::Lowpass { .. } => vec![t!("edits-field-cutoff")],
            Self::Median { .. } => vec![t!("edits-field-window")],
            Self::Rrc { .. } => vec![t!("edits-field-br"), t!("edits-field-alpha")],
            Self::Power { .. } => vec![t!("edits-field-power")],
            Self::Equalize { xn, .. } => {
                let mut v = vec![
                    t!("edits-field-carrier"),
                    t!("edits-field-br"),
                    t!("edits-field-blocks"),
                ];
                if xn.is_some() {
                    v.push(t!("edits-field-power-psk"));
                }
                v
            }
        }
    }

    /// The fields' initial text.
    pub fn texts(&self) -> Vec<String> {
        match *self {
            Self::Shift { hz } => vec![num_text(hz)],
            Self::Resample { rate } => vec![num_text(rate)],
            // The measured rate keeps its precision: the correction ratio uses it,
            // and a rounded value drifts the symbol clock over a long recording.
            Self::Baud { existing, required } => {
                vec![format::dec_trim(existing, 6), format::dec_trim(required, 6)]
            }
            Self::Lowpass { cutoff } => vec![num_text(cutoff)],
            Self::Median { window } => vec![window.to_string()],
            Self::Rrc { baud, alpha } => vec![num_text(baud), format::dec_trim(alpha, 2)],
            Self::Power { n } => vec![n.to_string()],
            Self::Equalize {
                carrier,
                baud,
                blocks,
                xn,
            } => {
                let mut v = vec![num_text(carrier), num_text(baud), blocks.to_string()];
                if let Some(n) = xn {
                    v.push(n.to_string());
                }
                v
            }
        }
    }

    /// The checked range of each numeric field; `None` for a field with its own
    /// rule (Shift, Median window, Correction of BR). The hints and [`Self::parse`]
    /// both come from this table, so they cannot disagree.
    pub fn bounds(&self, fs: f64) -> Vec<Option<Bound>> {
        let half = fs / 2.0;
        let power = Bound::closed(2.0, f64::from(MAX_POWER));
        match self {
            Self::Shift { .. } | Self::Median { .. } | Self::Baud { .. } => vec![None],
            Self::Resample { .. } => vec![Some(Bound::closed(1.0, MAX_RATE))],
            Self::Lowpass { .. } => vec![Some(Bound::open(0.0, half))],
            Self::Rrc { .. } => vec![Some(Bound::open(0.0, half)), Some(Bound::closed(0.01, 1.0))],
            Self::Power { .. } => vec![Some(power)],
            Self::Equalize { .. } => vec![
                Some(Bound::open(-fs, fs)),
                Some(Bound::open(0.0, fs)),
                Some(Bound::closed(BLOCKS.0 as f64, BLOCKS.1 as f64)),
                Some(power),
            ],
        }
    }

    /// The fields' valid ranges, shown as their hints.
    pub fn hints(&self, fs: f64) -> Vec<String> {
        let bounds = self.bounds(fs);
        (0..self.labels().len())
            .map(|i| match (self, bounds.get(i).copied().flatten()) {
                (_, Some(b)) => b.hint(),
                (Self::Shift { .. }, None) => {
                    t!("edits-hint-shift", max = format::dec_trim(fs, 2))
                }
                (Self::Median { .. }, None) => {
                    t!("edits-hint-window", max = MAX_MEDIAN.to_string())
                }
                _ => t!("sw-brc-max-delta"),
            })
            .collect()
    }

    /// The operation with the typed values, or the error message for the first
    /// field that is not a number or is out of range, with that field's index.
    pub fn parse(&self, texts: &[String], fs: f64) -> Result<Self, (usize, String)> {
        let bounds = self.bounds(fs);
        let labels = self.labels();
        let hints = self.hints(fs);
        let bad = |i: usize| {
            (
                i,
                t!(
                    "edits-field-invalid",
                    field = labels.get(i).cloned().unwrap_or_default(),
                    text = texts.get(i).map_or("", |t| t.trim()).to_owned(),
                    range = hints.get(i).cloned().unwrap_or_default()
                ),
            )
        };
        let real = |i: usize| -> Result<f64, (usize, String)> {
            let v = num(texts, i).ok_or_else(|| bad(i))?;
            match bounds.get(i).copied().flatten() {
                Some(b) if !b.contains(v) => Err(bad(i)),
                _ => Ok(v),
            }
        };
        let whole = |i: usize| -> Result<i64, (usize, String)> {
            let v = int(texts, i).ok_or_else(|| bad(i))?;
            match bounds.get(i).copied().flatten() {
                Some(b) if !b.contains(v as f64) => Err(bad(i)),
                _ => Ok(v),
            }
        };
        Ok(match self {
            Self::Shift { .. } => {
                let hz = real(0)?;
                if hz == 0.0 || hz.abs() > fs {
                    return Err(bad(0));
                }
                Self::Shift { hz }
            }
            Self::Resample { .. } => Self::Resample { rate: real(0)? },
            Self::Baud { .. } => {
                let e = texts.first().map_or("", String::as_str);
                let r = texts.get(1).map_or("", String::as_str);
                match brc_edit(e, r) {
                    Ok(Edit::BaudCorrection { existing, required }) => {
                        Self::Baud { existing, required }
                    }
                    Ok(_) => return Err(bad(0)),
                    Err(e) => return Err((e.field(), e.message())),
                }
            }
            Self::Lowpass { .. } => Self::Lowpass { cutoff: real(0)? },
            Self::Median { .. } => {
                let w = usize::try_from(whole(0)?).map_err(|_| bad(0))?;
                if !(1..=MAX_MEDIAN).contains(&w) {
                    return Err(bad(0));
                }
                Self::Median { window: w | 1 }
            }
            Self::Rrc { .. } => Self::Rrc {
                baud: real(0)?,
                alpha: real(1)?,
            },
            Self::Power { .. } => Self::Power {
                n: u32::try_from(whole(0)?).map_err(|_| bad(0))?,
            },
            Self::Equalize { xn, .. } => {
                let carrier = real(0)?;
                let baud = real(1)?;
                let blocks = usize::try_from(whole(2)?).map_err(|_| bad(2))?;
                let xn = match xn {
                    Some(_) => Some(u32::try_from(whole(3)?).map_err(|_| bad(3))?),
                    None => None,
                };
                Self::Equalize {
                    carrier,
                    baud,
                    blocks,
                    xn,
                }
            }
        })
    }

    /// The operation's name in the dialog title.
    pub fn title(&self) -> String {
        match self {
            Self::Shift { .. } => t!("edits-title-shift"),
            Self::Resample { .. } => t!("edits-title-resample"),
            Self::Baud { .. } => t!("edits-title-baud"),
            Self::Lowpass { .. } => t!("edits-title-lowpass"),
            Self::Median { .. } => t!("edits-title-median"),
            Self::Rrc { .. } => t!("edits-title-rrc"),
            Self::Power { .. } => t!("edits-title-power"),
            Self::Equalize { xn: None, .. } => t!("edits-title-isd-am"),
            Self::Equalize { xn: Some(_), .. } => t!("edits-title-isd-xn"),
        }
    }

    /// The operation with the dialog's values.
    pub fn edit(&self) -> Edit {
        match *self {
            Self::Shift { hz } => Edit::Shift { hz },
            Self::Resample { rate } => Edit::Resample { rate },
            Self::Baud { existing, required } => Edit::BaudCorrection { existing, required },
            Self::Lowpass { cutoff } => Edit::Lowpass { cutoff },
            Self::Median { window } => Edit::Median { window: window | 1 },
            Self::Rrc { baud, alpha } => Edit::Rrc { baud, alpha },
            Self::Power { n } => Edit::Power { n },
            Self::Equalize {
                carrier,
                baud,
                blocks,
                xn,
            } => Edit::Equalize {
                carrier,
                baud,
                blocks,
                mode: xn.map_or(EqMode::Am, |n| EqMode::Xn { n }),
            },
        }
    }
}

/// Largest Correction of BR change, as a fraction.
const MAX_BR_CHANGE: f64 = 0.3;

/// Why typed Correction of BR rates were refused.
#[derive(Debug, Clone, PartialEq)]
pub enum BrcError {
    /// The existing rate is not a number.
    Existing(String),
    /// The required rate is not a number.
    Required(String),
    /// A rate is 0 or negative (the field index).
    NotPositive(usize),
    /// The required rate is more than 30 % from the existing one; the allowed
    /// range.
    TooFar {
        /// Lowest allowed required rate.
        lo: f64,
        /// Highest allowed required rate.
        hi: f64,
    },
}

impl BrcError {
    /// The field the error is about: 0 existing, 1 required.
    pub fn field(&self) -> usize {
        match self {
            Self::Existing(_) => 0,
            Self::NotPositive(i) => *i,
            Self::Required(_) | Self::TooFar { .. } => 1,
        }
    }

    /// SA's "Please check settings!" with what is wrong and how to fix it.
    pub fn message(&self) -> String {
        let why = match self {
            Self::Existing(text) => t!("sw-brc-existing-invalid", text = text.clone()),
            Self::Required(text) => t!("sw-brc-required-invalid", text = text.clone()),
            Self::NotPositive(_) => t!("sw-brc-not-positive"),
            Self::TooFar { lo, hi } => t!(
                "sw-brc-too-far",
                lo = format::dec_trim(*lo, 4),
                hi = format::dec_trim(*hi, 4)
            ),
        };
        format!("{}\n{why}", t!("sw-check-settings"))
    }
}

/// The baud correction for the typed rates, or why they are refused (not
/// numbers, not positive, or more than 30 % apart).
pub fn brc_edit(existing: &str, required: &str) -> Result<Edit, BrcError> {
    let e = format::parse_number(existing)
        .ok_or_else(|| BrcError::Existing(existing.trim().to_owned()))?;
    let r = format::parse_number(required)
        .ok_or_else(|| BrcError::Required(required.trim().to_owned()))?;
    if e <= 0.0 {
        return Err(BrcError::NotPositive(0));
    }
    if r <= 0.0 {
        return Err(BrcError::NotPositive(1));
    }
    if (r / e - 1.0).abs() > MAX_BR_CHANGE + 1e-12 {
        return Err(BrcError::TooFar {
            lo: e * (1.0 - MAX_BR_CHANGE),
            hi: e * (1.0 + MAX_BR_CHANGE),
        });
    }
    Ok(Edit::BaudCorrection {
        existing: e,
        required: r,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stems_keep_sa_names() {
        assert_eq!(file_stem("test(0)(c1)"), "test(0)(c1)");
        assert_eq!(file_stem("a b/c"), "a_b_c");
    }

    #[test]
    fn every_format_writes_a_file_it_names() {
        let dir = std::env::temp_dir().join(format!("sn-ui-core-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let signal = Signal::real(vec![0.25_f32; 32], 8000.0).unwrap();
        for f in SaveFormat::ALL {
            let path = dir.join(format!("s.{}", f.extension()));
            let written = f.write(&path, &signal, "test", &|_, _| false).unwrap();
            assert!(written.is_file(), "{f:?}: {}", written.display());
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    impl ParamKind {
        fn parsed(&self, texts: &[String], fs: f64) -> Option<Self> {
            self.parse(texts, fs).ok()
        }
    }

    #[test]
    fn hints_match_what_parse_accepts() {
        let texts = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let lp = ParamKind::Lowpass { cutoff: 50.0 };
        assert_eq!(lp.hints(8000.0).len(), 1, "one hint per field");
        assert_eq!(lp.bounds(8000.0), vec![Some(Bound::open(0.0, 4000.0))]);
        // The bound of the hint is the bound of the check.
        assert!(lp.parsed(&texts(&["4000"]), 8000.0).is_none());
        let isd = ParamKind::Equalize {
            carrier: 0.0,
            baud: 1.0,
            blocks: 30,
            xn: None,
        };
        assert!(
            isd.parsed(&texts(&["9600", "2400", "30"]), 9600.0)
                .is_none()
        );
        assert!(
            isd.parsed(&texts(&["-9599", "2400", "30"]), 9600.0)
                .is_some()
        );
        assert_eq!(isd.hints(9600.0).len(), isd.labels().len());
    }

    #[test]
    fn parse_names_the_rejected_field() {
        let texts = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let rrc = ParamKind::Rrc {
            baud: 1.0,
            alpha: 0.35,
        };
        let Err((field, why)) = rrc.parse(&texts(&["2400", "1.5"]), 48_000.0) else {
            panic!("alpha 1.5 accepted");
        };
        assert_eq!(field, 1);
        assert!(why.contains("1.5"), "{why}");
        let Err((field, _)) = rrc.parse(&texts(&["x", "0.3"]), 48_000.0) else {
            panic!("bad Br accepted");
        };
        assert_eq!(field, 0);
    }

    #[test]
    fn adjusted_defaults_are_warnings() {
        let (c, n) = lowpass_default(8000.0, Some((10.0, "n2")));
        assert!((c - 30.0).abs() < 1e-9);
        assert!(n.warn, "{n:?}");
        let (c, n) = lowpass_default(8000.0, Some((50.0, "n2")));
        assert!((c - 50.0).abs() < 1e-9);
        assert!(!n.warn, "{n:?}");
        // A Br at or above half the sample rate, or the 30 Hz floor on a slow
        // signal, is lowered so the dialog's own check accepts it.
        for (fs, br) in [(8000.0, 4000.0), (8000.0, 9000.0), (50.0, 10.0)] {
            let (c, n) = lowpass_default(fs, Some((br, "n2")));
            assert!(c > 0.0 && c < fs / 2.0, "{fs} {br}: {c}");
            assert!(n.warn, "{n:?}");
        }
        let (w, n) = median_default(48_000.0, 1.0, "n2").unwrap_or((0, Note::from(String::new())));
        assert_eq!(w, 301);
        assert!(n.warn, "{n:?}");
        let (w, n) = median_default(8000.0, 50.0, "n2").unwrap_or((0, Note::from(String::new())));
        assert_eq!(w, 53);
        assert!(!n.warn, "{n:?}");
    }

    #[test]
    fn median_window_is_a_third_of_a_symbol_and_odd() {
        // 8000 / 50 / 3 = 53.3 -> 53.
        assert_eq!(median_window(8000.0, 50.0), Some((53, false)));
        // 48000 / 100 / 3 = 160 -> 161.
        assert_eq!(median_window(48_000.0, 100.0), Some((161, false)));
        // 48000 / 50 / 3 = 320: capped.
        assert_eq!(median_window(48_000.0, 50.0), Some((301, true)));
        assert_eq!(median_window(48_000.0, 1.0), Some((301, true)));
        assert_eq!(median_window(8000.0, 1e6), Some((1, false)));
        assert_eq!(median_window(8000.0, 0.0), None);
    }

    #[test]
    fn param_kinds_map_to_edits() {
        assert_eq!(
            ParamKind::Median { window: 4 }.edit(),
            Edit::Median { window: 5 }
        );
        assert_eq!(
            ParamKind::Median { window: 7 }.edit(),
            Edit::Median { window: 7 }
        );
        assert_eq!(
            ParamKind::Baud {
                existing: 2373.0,
                required: 2400.0
            }
            .edit(),
            Edit::BaudCorrection {
                existing: 2373.0,
                required: 2400.0
            }
        );
        assert_eq!(
            ParamKind::Equalize {
                carrier: 1800.0,
                baud: 2400.0,
                blocks: 30,
                xn: Some(4),
            }
            .edit(),
            Edit::Equalize {
                carrier: 1800.0,
                baud: 2400.0,
                blocks: 30,
                mode: EqMode::Xn { n: 4 },
            }
        );
        assert_eq!(
            ParamKind::Equalize {
                carrier: 1.0,
                baud: 2.0,
                blocks: 10,
                xn: None,
            }
            .edit(),
            Edit::Equalize {
                carrier: 1.0,
                baud: 2.0,
                blocks: 10,
                mode: EqMode::Am,
            }
        );
    }

    fn texts(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn dialogs_accept_fractions_and_reject_values_out_of_range() {
        let shift = ParamKind::Shift { hz: 0.0 };
        assert_eq!(
            shift.parsed(&texts(&["7777,77"]), 48_000.0),
            Some(ParamKind::Shift { hz: 7777.77 })
        );
        assert_eq!(
            shift.parsed(&texts(&["-12.5"]), 48_000.0),
            Some(ParamKind::Shift { hz: -12.5 })
        );
        assert_eq!(shift.parsed(&texts(&["0"]), 48_000.0), None);
        assert_eq!(shift.parsed(&texts(&["60000"]), 48_000.0), None);
        assert_eq!(shift.parsed(&texts(&["abc"]), 48_000.0), None);
        let rs = ParamKind::Resample { rate: 8000.0 };
        assert_eq!(
            rs.parsed(&texts(&["7982,5"]), 8000.0),
            Some(ParamKind::Resample { rate: 7982.5 })
        );
        assert_eq!(rs.parsed(&texts(&["0"]), 8000.0), None);
        let lp = ParamKind::Lowpass { cutoff: 100.0 };
        assert_eq!(
            lp.parsed(&texts(&["3999"]), 8000.0),
            Some(ParamKind::Lowpass { cutoff: 3999.0 })
        );
        assert_eq!(lp.parsed(&texts(&["4000"]), 8000.0), None);
        let md = ParamKind::Median { window: 5 };
        assert_eq!(
            md.parsed(&texts(&["4"]), 8000.0),
            Some(ParamKind::Median { window: 5 })
        );
        assert_eq!(md.parsed(&texts(&["302"]), 8000.0), None);
        assert_eq!(md.parsed(&texts(&["2.5"]), 8000.0), None);
        let rrc = ParamKind::Rrc {
            baud: 100.0,
            alpha: 0.35,
        };
        assert_eq!(
            rrc.parsed(&texts(&["2400", "0,35"]), 48_000.0),
            Some(ParamKind::Rrc {
                baud: 2400.0,
                alpha: 0.35
            })
        );
        assert_eq!(rrc.parsed(&texts(&["2400", "1.5"]), 48_000.0), None);
        let pw = ParamKind::Power { n: 2 };
        assert_eq!(
            pw.parsed(&texts(&["8"]), 8000.0),
            Some(ParamKind::Power { n: 8 })
        );
        assert_eq!(pw.parsed(&texts(&["1"]), 8000.0), None);
        assert_eq!(pw.parsed(&texts(&["33"]), 8000.0), None);
    }

    #[test]
    fn baud_and_isd_dialogs_follow_their_limits() {
        let brc = ParamKind::Baud {
            existing: 75.15,
            required: 75.0,
        };
        assert_eq!(
            brc.parsed(&texts(&["75,15", "75"]), 8000.0),
            Some(ParamKind::Baud {
                existing: 75.15,
                required: 75.0
            })
        );
        assert_eq!(brc.parsed(&texts(&["100", "131"]), 8000.0), None);
        let isd = ParamKind::Equalize {
            carrier: 1800.0,
            baud: 2400.0,
            blocks: 30,
            xn: Some(8),
        };
        assert_eq!(isd.texts().len(), 4);
        assert_eq!(isd.labels().len(), 4);
        assert_eq!(
            isd.parsed(&texts(&["1800", "2400", "30", "4"]), 9600.0),
            Some(ParamKind::Equalize {
                carrier: 1800.0,
                baud: 2400.0,
                blocks: 30,
                xn: Some(4)
            })
        );
        assert_eq!(
            isd.parsed(&texts(&["1800", "2400", "9", "4"]), 9600.0),
            None
        );
        assert_eq!(
            isd.parsed(&texts(&["1800", "2400", "30", "1"]), 9600.0),
            None
        );
        let am = ParamKind::Equalize {
            carrier: 1800.0,
            baud: 2400.0,
            blocks: 30,
            xn: None,
        };
        assert_eq!(am.texts().len(), 3);
        assert!(am.parsed(&texts(&["1800", "2400", "50"]), 9600.0).is_some());
    }

    #[test]
    fn initial_texts_parse_back_to_the_same_values() {
        for k in [
            ParamKind::Shift { hz: 12.5 },
            ParamKind::Resample { rate: 13_333.0 },
            ParamKind::Lowpass { cutoff: 50.0 },
            ParamKind::Median { window: 53 },
            ParamKind::Rrc {
                baud: 2400.0,
                alpha: 0.35,
            },
            ParamKind::Power { n: 4 },
            ParamKind::Baud {
                existing: 75.1534,
                required: 75.0,
            },
            ParamKind::Equalize {
                carrier: 1800.1234,
                baud: 2400.5678,
                blocks: 30,
                xn: Some(4),
            },
        ] {
            assert_eq!(k.parsed(&k.texts(), 48_000.0), Some(k.clone()), "{k:?}");
            assert_eq!(k.texts().len(), k.labels().len());
        }
    }

    #[test]
    fn baud_correction_is_limited_to_30_percent() {
        assert_eq!(
            brc_edit("75,15", "75"),
            Ok(Edit::BaudCorrection {
                existing: 75.15,
                required: 75.0
            })
        );
        assert!(brc_edit("100", "130").is_ok());
        assert_eq!(
            brc_edit("100", "131"),
            Err(BrcError::TooFar {
                lo: 70.0,
                hi: 130.0
            })
        );
        assert_eq!(brc_edit("0", "75"), Err(BrcError::NotPositive(0)));
        assert_eq!(brc_edit("", "75"), Err(BrcError::Existing(String::new())));
        assert_eq!(brc_edit("100", ""), Err(BrcError::Required(String::new())));
        assert!(
            BrcError::TooFar {
                lo: 70.0,
                hi: 130.0
            }
            .message()
            .contains("130")
        );
    }
}
