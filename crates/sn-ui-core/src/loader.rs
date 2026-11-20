//! The rules of the `SA Open file` window shared by the front ends: what the
//! probe found about the selected file, the option fields it starts with, the
//! load options and prescaler they make, which preview to build, and the
//! preview job itself. The front ends keep the widgets and draw the window.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sn_core::job::{self, JobError, JobHandle};
use sn_dsp::overview::{MinMax, Overview};
use sn_io::{Endian, FileInfo, LoadOptions, RawFormat, Scaling};

use crate::format;
use crate::loader_info::{self, Bits, Chan, StereoChoice, WavHeader};
use crate::t;

/// Columns of the preview envelope.
pub const PREVIEW_COLUMNS: usize = 760;
/// How long the options must stay unchanged before the file is re-read for a
/// new preview (typing a value would otherwise re-read on every key).
pub const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(300);
/// SA builds no preview for files shorter than 4 KB.
pub const MIN_PREVIEW_BYTES: u64 = 4096;
/// Prescaler range in dB (bottom of the bar: no gain).
pub const PRESCALER_MAX_DB: i32 = 60;

/// What [`probe`](sn_io::probe) said about the selected file.
#[derive(Debug, Clone)]
pub enum Detected {
    /// A format SigNexora reads.
    Known(FileInfo),
    /// Headerless data: only the options can read it.
    NeedsRaw,
    /// The file could not be read; the message says why.
    Failed(String),
}

/// The selected file and what is known about it.
#[derive(Debug, Clone)]
pub struct Selected {
    /// The file chosen in the list.
    pub path: PathBuf,
    /// Its size in bytes.
    pub size: u64,
    /// Modification time when the file was probed (a change re-probes it).
    pub modified: Option<SystemTime>,
    /// What the probe found.
    pub detected: Detected,
    /// The WAV header, for SA's info panel.
    pub wav: Option<WavHeader>,
    /// Size in bytes of the file holding the samples (a SigMF pair's data file,
    /// else the selected file), taken once when probing so the window does no
    /// file system calls while drawing. `size` when it could not be read.
    pub data_size: u64,
}

impl Selected {
    /// Probe `path` (of `size` bytes, modified at `modified`).
    pub fn probe(path: PathBuf, size: u64, modified: Option<SystemTime>) -> Self {
        let detected = match sn_io::probe(&path) {
            Ok(info) => Detected::Known(info),
            Err(sn_io::Error::NeedsRawFormat) => Detected::NeedsRaw,
            Err(err) => Detected::Failed(err.to_string()),
        };
        let wav = loader_info::read_wav_header(&path);
        // A data file that cannot be stat'ed fails with its own message on load;
        // until then the selected file's size stands in for it.
        let data_size = match &detected {
            Detected::Known(info) if info.path != path => {
                std::fs::metadata(&info.path).map_or(size, |m| m.len())
            }
            _ => size,
        };
        Self {
            path,
            size,
            modified,
            detected,
            wav,
            data_size,
        }
    }

    /// The file a load with (`raw`) or without options reads: the data file
    /// of a SigMF pair when the options are used (its `.sigmf-data`, not the
    /// selected `.sigmf-meta`), else the selected file.
    pub fn load_path(&self, raw: bool) -> PathBuf {
        match (&self.detected, raw) {
            (Detected::Known(info), true) => info.path.clone(),
            _ => self.path.clone(),
        }
    }

    /// The sample data's size in bytes: from the WAV header or the detected
    /// layout, else the size of the file holding the samples.
    pub fn data_bytes(&self) -> u64 {
        if let Some(h) = &self.wav {
            return h.data_size;
        }
        match &self.detected {
            Detected::Known(info) => info.layout().map_or(self.data_size, |l| l.data_len),
            _ => self.size,
        }
    }

    /// The channel count the file states, or `None` for headerless data
    /// (the typed channel count then applies).
    pub fn stated_channels(&self) -> Option<u16> {
        match (&self.detected, self.wav) {
            (_, Some(h)) => Some(h.channels.max(1)),
            (Detected::Known(info), None) => Some(info.channels.max(1)),
            _ => None,
        }
    }

    /// The file has no header SigNexora knows.
    pub fn headerless(&self) -> bool {
        self.wav.is_none() && !matches!(self.detected, Detected::Known(_))
    }

    /// `Use options` can be switched: the file has an uncompressed layout the
    /// options can describe (a compressed file read as raw PCM is noise).
    pub fn options_allowed(&self) -> bool {
        self.wav.is_some()
            || matches!(&self.detected, Detected::Known(info) if info.layout().is_some())
    }

    /// Without the options, a stereo file asks which channel to load.
    pub fn is_stereo(&self) -> bool {
        matches!(&self.detected, Detected::Known(i) if is_stereo(i))
    }

    /// The option fields for this file: from its header or detected layout,
    /// else the remembered headerless format `raw`.
    pub fn initial_fields(&self, raw: &RawFormat) -> OptionFields {
        match (&self.detected, &self.wav) {
            (_, Some(h)) => OptionFields {
                data_start: h.data_start.to_string(),
                chan: Chan::All,
                bits: Bits::from_wav(h),
                rate: h.rate.to_string(),
                total: h.channels.max(1),
            },
            (Detected::Known(info), None) => OptionFields {
                data_start: detected_data_start(info).to_string(),
                chan: Chan::All,
                // The byte order the probe found (SigMF names it in its
                // datatype, `cf32_be`), so `Use options` reads it as detected.
                bits: info
                    .sample_format
                    .map_or(Bits::I16, |f| Bits::of(f, detected_endian(info))),
                rate: format::dec_trim(info.sample_rate, 2),
                total: info.channels.max(1),
            },
            _ => OptionFields::raw(raw),
        }
    }
}

/// The option fields (`Data start`, `Channels`, `Bits`, `Freq rate`) as typed.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionFields {
    /// `Data start` in bytes, as typed.
    pub data_start: String,
    /// The channel choice.
    pub chan: Chan,
    /// The sample format.
    pub bits: Bits,
    /// `Freq rate` in Hz, as typed.
    pub rate: String,
    /// Interleaved channels of headerless data (SigNexora addition).
    pub total: u16,
}

impl OptionFields {
    /// The fields for headerless data in `raw`.
    pub fn raw(raw: &RawFormat) -> Self {
        Self {
            data_start: raw.data_offset.to_string(),
            chan: if raw.channels == 2 {
                Chan::All
            } else {
                Chan::One(0)
            },
            bits: Bits::of(raw.format, raw.endian),
            rate: format::dec_trim(raw.sample_rate, 2),
            total: raw.channels.max(1),
        }
    }
}

/// A two-channel file whose channels are left and right (complex SigMF also
/// reports two channels, but they are one I/Q stream).
pub fn is_stereo(info: &FileInfo) -> bool {
    info.channels == 2 && !info.iq_pairs()
}

/// The byte order of a known file without a WAV header, as the probe found it;
/// little-endian when the file has no layout.
pub fn detected_endian(info: &FileInfo) -> Endian {
    info.layout().map_or(Endian::Little, |l| l.endian)
}

/// The byte offset of the first sample of a known file without a WAV header
/// (SigMF's `core:header_bytes`); 0 when the file has no layout.
pub fn detected_data_start(info: &FileInfo) -> u64 {
    info.layout().map_or(0, |l| l.data_offset)
}

/// The prescaler as load scaling: gain, then clipping at full scale. Only with
/// the options; 0 dB is no scaling.
pub fn scaling(use_options: bool, prescale_db: f32) -> Scaling {
    if !use_options || prescale_db <= 0.0 {
        return Scaling::default();
    }
    let gain = 10_f32.powf(prescale_db / 20.0);
    // Clipping at 1/gain and rescaling that threshold to full scale already
    // multiplies by `gain`; a second gain would apply it twice.
    Scaling {
        gain: 1.0,
        clip: Some(1.0 / gain),
    }
}

/// The option fields as the front end holds them (its text boxes).
#[derive(Debug, Clone, Copy)]
pub struct FieldsIn<'a> {
    /// `Data start`, as typed.
    pub data_start: &'a str,
    /// The channel choice.
    pub chan: Chan,
    /// The sample format.
    pub bits: Bits,
    /// `Freq rate`, as typed.
    pub rate: &'a str,
    /// The channel count typed for headerless data.
    pub total: u16,
}

/// The loader's state that decides the load options.
#[derive(Debug, Clone, Copy)]
pub struct OptionsIn<'a> {
    /// The selected file, if any.
    pub file: Option<&'a Selected>,
    /// `Use options` is ticked.
    pub use_options: bool,
    /// The option fields.
    pub fields: FieldsIn<'a>,
    /// The prescaler in dB.
    pub prescale_db: f32,
}

impl OptionsIn<'_> {
    /// The channel count the options refer to.
    pub fn channel_total(&self) -> u16 {
        self.file
            .and_then(Selected::stated_channels)
            .unwrap_or(self.fields.total)
            .max(1)
    }

    /// The load options, or the message naming the invalid option field.
    /// `stereo` is the `SA - select channel` answer for a stereo file read
    /// without options.
    pub fn load_options(&self, stereo: StereoChoice) -> Result<LoadOptions, String> {
        let Some(file) = self.file else {
            return Err(t!("loader-no-file"));
        };
        if self.use_options {
            let total = self.channel_total();
            let (format, endian) = self.fields.bits.format();
            let rate = self.fields.rate;
            let sample_rate = loader_info::parse_rate(rate)
                .ok_or_else(|| t!("loader-rate-invalid", text = rate.trim().to_owned()))?;
            let start = self.fields.data_start;
            let data_offset = loader_info::parse_bytes(start)
                .ok_or_else(|| t!("loader-data-start-invalid", text = start.trim().to_owned()))?;
            let raw = RawFormat {
                format,
                endian,
                channels: total,
                sample_rate,
                data_offset,
            };
            return Ok(LoadOptions {
                channels: Some(self.fields.chan.selection(total)),
                scaling: scaling(true, self.prescale_db),
                raw: Some(raw),
            });
        }
        let Detected::Known(info) = &file.detected else {
            return Err(t!("loader-needs-options"));
        };
        let channels = if is_stereo(info) {
            Some(stereo.selection())
        } else if info.sample_format.is_none() && info.channels == 0 {
            // A compressed file whose header does not state its channels: the
            // decoder chooses from the real stream.
            None
        } else {
            Some(info.default_selection())
        };
        Ok(LoadOptions {
            channels,
            scaling: Scaling::default(),
            raw: None,
        })
    }

    /// The file to load and its options, or the message naming the invalid
    /// option field.
    pub fn load(&self, stereo: StereoChoice) -> Result<(PathBuf, LoadOptions), String> {
        let o = self.load_options(stereo)?;
        let file = self.file.ok_or_else(|| t!("loader-no-file"))?;
        Ok((file.load_path(o.raw.is_some()), o))
    }

    /// The preview to show: with the options, or (`auto_preview`) as the file
    /// loads. It reads the file the load would read (a SigMF pair's data file
    /// with the options), before the prescaler, and none for less than
    /// [`MIN_PREVIEW_BYTES`] of samples.
    pub fn preview_key(&self, auto_preview: bool) -> Option<(PathBuf, LoadOptions)> {
        let file = self.file?;
        if !(self.use_options || auto_preview) {
            return None;
        }
        let mut o = self.load_options(StereoChoice::default()).ok()?;
        let path = file.load_path(o.raw.is_some());
        let bytes = if o.raw.is_some() {
            file.data_size
        } else {
            file.data_bytes()
        };
        if bytes < MIN_PREVIEW_BYTES {
            return None;
        }
        o.scaling = Scaling::default();
        Some((path, o))
    }
}

/// Envelope preview of the whole file, before the prescaler.
#[derive(Debug, Clone)]
pub struct Preview {
    /// [`PREVIEW_COLUMNS`] columns, each the envelope of its frames.
    pub columns: Vec<MinMax>,
}

/// Read the file and build its preview envelope in the background.
pub fn spawn_preview((path, opts): (PathBuf, LoadOptions)) -> JobHandle<Preview> {
    job::spawn("Preview", move |ctx| {
        build_preview(&path, &opts, &|| ctx.is_cancelled())
    })
}

/// The preview envelope of `path` read with `opts`.
fn build_preview(
    path: &Path,
    opts: &LoadOptions,
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<Preview, JobError> {
    let opened = sn_io::open_cancellable(path, opts, stop).map_err(crate::jobs::io_to_job)?;
    let signal = opened.signal;
    let frames = signal.len();
    let block = (frames / PREVIEW_COLUMNS).max(1);
    let ov = Overview::build(&signal, block, stop).map_err(crate::jobs::dsp_to_job)?;
    let columns: Vec<MinMax> = (0..PREVIEW_COLUMNS)
        .map(|c| {
            let (a, b) = column_span(c, frames);
            let env = ov.span(&signal, a, b);
            env.im.map_or(env.re, |q| env.re.merge(q))
        })
        .collect();
    Ok(Preview { columns })
}

/// The frames preview column `c` covers: the columns share the frames
/// proportionally, and every column holds at least one frame (a short file
/// repeats frames across columns instead of leaving them blank).
pub fn column_span(c: usize, frames: usize) -> (usize, usize) {
    let cols = PREVIEW_COLUMNS as u128;
    let at = |i: usize| usize::try_from(i as u128 * frames as u128 / cols).unwrap_or(frames);
    let a = at(c).min(frames.saturating_sub(1));
    let b = at(c.saturating_add(1)).max(a.saturating_add(1)).min(frames);
    (a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preview_column_holds_frames() {
        for frames in [1, 3, 759, 760, 761, 1519, 20_000, 1_000_003] {
            let mut next = 0;
            for c in 0..PREVIEW_COLUMNS {
                let (a, b) = column_span(c, frames);
                assert!(
                    a < b && b <= frames,
                    "{frames} frames, column {c}: {a}..{b}"
                );
                assert!(a <= next, "{frames} frames: gap before column {c}");
                next = next.max(b);
            }
            assert_eq!(
                next, frames,
                "{frames} frames: the last column ends the file"
            );
        }
        assert_eq!(column_span(0, 0), (0, 0));
    }

    #[test]
    fn the_prescaler_clips_only_with_the_options() {
        assert_eq!(scaling(false, 20.0), Scaling::default());
        assert_eq!(scaling(true, 0.0), Scaling::default());
        let s = scaling(true, 20.0);
        assert_eq!(s.gain, 1.0);
        assert!(s.clip.is_some_and(|c| (c - 0.1).abs() < 1e-6));
    }

    #[test]
    fn headerless_files_take_the_remembered_format_and_need_options() {
        let raw = loader_info::DEFAULT_RAW;
        let f = Selected {
            path: PathBuf::from("x.bin"),
            size: 10_000,
            modified: None,
            detected: Detected::NeedsRaw,
            wav: None,
            data_size: 10_000,
        };
        assert!(f.headerless());
        assert!(!f.options_allowed());
        assert_eq!(f.stated_channels(), None);
        let fields = f.initial_fields(&raw);
        assert_eq!(fields, OptionFields::raw(&raw));
        let input = OptionsIn {
            file: Some(&f),
            use_options: false,
            fields: FieldsIn {
                data_start: &fields.data_start,
                chan: fields.chan,
                bits: fields.bits,
                rate: &fields.rate,
                total: fields.total,
            },
            prescale_db: 0.0,
        };
        assert_eq!(
            input.load_options(StereoChoice::default()).unwrap_err(),
            t!("loader-needs-options")
        );
        let with = OptionsIn {
            use_options: true,
            ..input
        };
        let (path, o) = with.load(StereoChoice::default()).unwrap();
        assert_eq!(path, PathBuf::from("x.bin"));
        assert!(o.raw.is_some());
        let bad = OptionsIn {
            fields: FieldsIn {
                rate: "abc",
                ..with.fields
            },
            ..with
        };
        assert!(bad.load_options(StereoChoice::default()).is_err());
    }
}
