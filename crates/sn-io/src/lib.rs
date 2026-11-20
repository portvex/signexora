//! File input and output for SigNexora.
//!
//! - [`probe`] inspects a file without loading it and returns a [`FileInfo`].
//! - [`open`] turns a file into a [`sn_core::Signal`]. Uncompressed data (WAV, RF64,
//!   BW64, SigMF, raw) is memory-mapped and decoded on demand, so files of any size open
//!   instantly. Compressed audio (MP3, FLAC, Ogg Vorbis, ADPCM WAV) is decoded into
//!   memory.
//! - [`write_wav`], [`write_raw`] and [`write_sigmf`] save signals; SigMF keeps
//!   fractional sample rates. The `*_with_progress` variants report progress and can
//!   be cancelled, and [`open_cancellable`] does the same for loading.
//!
//! Nothing in this crate panics on bad input: every problem is reported as an
//! [`Error`] with a message meant for the user.

mod compressed;
mod format;
mod mapped;
mod riff;
mod sigmf;
mod write;

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use format::{ChannelSelection, DataLayout, Endian, SampleFormat, Scaling};
use sn_core::Signal;
pub use write::{
    Progress, WavFormat, wav_can_store_rate, write_atomically, write_raw, write_raw_with_progress,
    write_sigmf, write_sigmf_with_progress, write_wav, write_wav_with_progress,
};

/// Errors from reading or writing files.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file or directory involved.
        path: PathBuf,
        /// What went wrong.
        source: std::io::Error,
    },
    /// The WAV writer reported a problem.
    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),
    /// The file is damaged or not what its name says.
    #[error("the file is damaged or not in the expected format: {0}")]
    Malformed(String),
    /// WAV headers store integer sample rates only.
    #[error(
        "sample rate {0} Hz is fractional; WAV stores whole numbers only. \
         Resample to an integer rate first"
    )]
    FractionalSampleRate(f64),
    /// The file has no header we recognise; open it as raw data with a [`RawFormat`].
    #[error(
        "the format of this file is not recognised. Open it as raw data and \
         describe the sample format, channels and sample rate"
    )]
    NeedsRawFormat,
    /// The file uses a feature we do not support.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// The caller cancelled the operation.
    #[error("cancelled")]
    Cancelled,
    /// The decoded data did not form a valid signal.
    #[error(transparent)]
    Core(#[from] sn_core::Error),
}

/// Convenience alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Wrap an I/O error with the path it concerns.
pub(crate) fn io_err(path: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_owned(),
        source,
    }
}

/// Description of headerless sample data (SA's "OpenAsData").
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RawFormat {
    /// Sample format.
    pub format: SampleFormat,
    /// Byte order.
    pub endian: Endian,
    /// Interleaved channels per frame.
    pub channels: u16,
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// Bytes to skip at the start of the file (a header we do not understand).
    pub data_offset: u64,
}

/// The kind of file, as detected by [`probe`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Container {
    /// RIFF WAVE.
    Wav,
    /// RF64 or BW64 (WAV for files over 4 GB).
    Rf64,
    /// SigMF metadata plus data file.
    Sigmf,
    /// Headerless data described by a [`RawFormat`].
    Raw,
    /// Compressed audio decoded through Symphonia.
    Compressed,
}

/// Everything known about a file before it is loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct FileInfo {
    /// File that holds the samples.
    pub path: PathBuf,
    /// Detected container.
    pub container: Container,
    /// Codec or sample format name shown to the user, for example `"PCM"` or `"MP3"`.
    pub codec: String,
    /// Decodable sample format, when uncompressed.
    pub sample_format: Option<SampleFormat>,
    /// Number of channels.
    pub channels: u16,
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// Frames (samples per channel), when known before decoding.
    pub frames: Option<u64>,
    /// RF centre frequency in Hz, when the file records one.
    pub center_frequency: f64,
    /// True when the data can be memory-mapped and read on demand.
    pub streamable: bool,
    /// Remarks for the user, such as quality warnings.
    pub notes: Vec<String>,
    layout: Option<DataLayout>,
    /// Channels come in I/Q pairs (complex SigMF), so the default is the first pair.
    iq_pairs: bool,
}

impl FileInfo {
    /// Duration in seconds, when the frame count is known.
    pub fn duration(&self) -> Option<f64> {
        self.frames
            .filter(|_| self.sample_rate > 0.0)
            .map(|f| f as f64 / self.sample_rate)
    }

    /// True when the channels come in I/Q pairs (complex SigMF): channels 0 and 1 are
    /// the I and Q of the first stream, not two separate signals.
    pub fn iq_pairs(&self) -> bool {
        self.iq_pairs
    }

    /// Where and how the samples are stored, for uncompressed files (WAV, RF64, SigMF
    /// and raw data); `None` for compressed files.
    pub fn layout(&self) -> Option<&DataLayout> {
        self.layout.as_ref()
    }

    /// The channel selection [`open`] uses when none is given.
    pub fn default_selection(&self) -> ChannelSelection {
        if self.iq_pairs {
            ChannelSelection::IQ { i: 0, q: 1 }
        } else {
            ChannelSelection::default_for(self.channels)
        }
    }
}

/// A signal from [`open_cancellable`] with remarks about the loading.
#[derive(Debug, Clone)]
pub struct Opened {
    /// The loaded signal.
    pub signal: Signal,
    /// Problems met while loading, such as damaged packets replaced with silence.
    /// These add to [`FileInfo::notes`].
    pub notes: Vec<String>,
}

/// Options for [`open`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LoadOptions {
    /// Which channels form the signal; `None` uses [`FileInfo::default_selection`].
    pub channels: Option<ChannelSelection>,
    /// Pre-load gain and clipping.
    pub scaling: Scaling,
    /// Treat the file as raw data with this layout, ignoring any header.
    pub raw: Option<RawFormat>,
}

fn quality_notes(format: Option<SampleFormat>, codec: &str, compressed: bool) -> Vec<String> {
    let mut notes = Vec::new();
    if compressed {
        notes.push(format!(
            "{codec} is a lossy or compressed format. It distorts the signal, so precise \
             measurements may be unreliable. Record as 16-bit PCM WAV where possible."
        ));
    }
    match format {
        Some(SampleFormat::U8 | SampleFormat::I8) => notes
            .push("8-bit data has coarse quantisation; weak components may be lost.".to_owned()),
        Some(SampleFormat::ALaw | SampleFormat::MuLaw) => notes
            .push("G.711 companded audio is a telephone codec and distorts the signal.".to_owned()),
        _ => {}
    }
    notes
}

fn read_head(path: &Path, max: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let io = io_err(path);
    let mut buf = Vec::new();
    std::fs::File::open(path)
        .map_err(&io)?
        .take(max)
        .read_to_end(&mut buf)
        .map_err(&io)?;
    Ok(buf)
}

/// Bytes of sample data after `data_offset` in a file of `size` bytes, or an error
/// when there is not even one frame.
fn data_after(offset: u64, size: u64, frame_bytes: usize) -> Result<u64> {
    if size == 0 {
        return Err(Error::Unsupported("the file is empty".into()));
    }
    if offset >= size {
        return Err(Error::Unsupported(format!(
            "data offset {offset} is beyond the end of the file ({size} bytes); \
             reduce the offset"
        )));
    }
    let len = size - offset;
    if len < frame_bytes as u64 {
        return Err(Error::Unsupported(format!(
            "only {len} bytes of data follow offset {offset}, less than one frame \
             ({frame_bytes} bytes); check the offset, format and channel count"
        )));
    }
    Ok(len)
}

fn raw_info(path: &Path, raw: &RawFormat) -> Result<FileInfo> {
    if raw.channels == 0 {
        return Err(Error::Unsupported(
            "raw data needs at least one channel".into(),
        ));
    }
    if !raw.sample_rate.is_finite() || raw.sample_rate <= 0.0 {
        return Err(Error::Unsupported(
            "raw data needs a sample rate above 0 Hz".into(),
        ));
    }
    let size = std::fs::metadata(path).map_err(io_err(path))?.len();
    let mut layout = DataLayout {
        format: raw.format,
        endian: raw.endian,
        channels: raw.channels,
        data_offset: raw.data_offset,
        data_len: 0,
    };
    layout.data_len = data_after(raw.data_offset, size, layout.frame_bytes())?;
    Ok(FileInfo {
        path: path.to_owned(),
        container: Container::Raw,
        codec: raw.format.label().to_owned(),
        sample_format: Some(raw.format),
        channels: raw.channels,
        sample_rate: raw.sample_rate,
        frames: Some(layout.frames()),
        center_frequency: 0.0,
        streamable: true,
        notes: quality_notes(Some(raw.format), raw.format.label(), false),
        layout: Some(layout),
        iq_pairs: false,
    })
}

/// Inspect a file without loading its samples.
///
/// Returns [`Error::NeedsRawFormat`] for files without a recognised header; open those
/// with [`LoadOptions::raw`].
pub fn probe(path: impl AsRef<Path>) -> Result<FileInfo> {
    let path = path.as_ref();

    if let Some((meta, data)) = sigmf::pair(path) {
        let m = sigmf::read(&meta, &data)?;
        let channels = if m.complex {
            m.num_channels.saturating_mul(2)
        } else {
            m.num_channels
        };
        let size = match std::fs::metadata(&m.data_path) {
            Ok(md) => md.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Unsupported(format!(
                    "the data file {} is missing; keep the .sigmf-data file next to \
                     the .sigmf-meta file",
                    m.data_path.display()
                )));
            }
            Err(e) => return Err(io_err(&m.data_path)(e)),
        };
        let mut layout = DataLayout {
            format: m.format,
            endian: m.endian,
            channels,
            data_offset: m.header_bytes,
            data_len: 0,
        };
        layout.data_len = data_after(m.header_bytes, size, layout.frame_bytes())?;
        let mut notes = quality_notes(Some(m.format), &m.datatype, false);
        if m.num_channels > 1 {
            notes.push(format!(
                "The recording has {} channels; the first is opened unless you choose \
                 others.",
                m.num_channels
            ));
        }
        if let Some(d) = m.description {
            notes.insert(0, d);
        }
        return Ok(FileInfo {
            path: m.data_path,
            container: Container::Sigmf,
            codec: format!("SigMF {}", m.datatype),
            sample_format: Some(m.format),
            channels,
            sample_rate: m.sample_rate,
            frames: Some(layout.frames()),
            center_frequency: m.center_frequency,
            streamable: true,
            notes,
            layout: Some(layout),
            iq_pairs: m.complex,
        });
    }

    let head = read_head(path, 12)?;
    if riff::looks_like_wav(&head) {
        let h = riff::read_header(path)?;
        if let Some(layout) = h.layout() {
            let mut notes = h.notes.clone();
            notes.extend(quality_notes(h.sample_format, &h.codec, false));
            return Ok(FileInfo {
                path: path.to_owned(),
                container: if h.container == "WAV" {
                    Container::Wav
                } else {
                    Container::Rf64
                },
                codec: h.codec.clone(),
                sample_format: h.sample_format,
                channels: h.channels,
                sample_rate: f64::from(h.sample_rate),
                frames: Some(layout.frames()),
                center_frequency: 0.0,
                streamable: true,
                notes,
                layout: Some(layout),
                iq_pairs: false,
            });
        }
        // A WAV with a compressed codec (ADPCM, MP3-in-WAV, ...): let Symphonia try.
        return compressed_info(path, Some(h.codec));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("mp3" | "flac" | "ogg" | "oga" | "wav") => compressed_info(path, None),
        _ => Err(Error::NeedsRawFormat),
    }
}

fn compressed_info(path: &Path, codec_hint: Option<String>) -> Result<FileInfo> {
    let c = compressed::probe(path)?;
    let codec = codec_hint.unwrap_or(c.codec);
    let mut notes = quality_notes(None, &codec, true);
    if c.channels.is_none() || c.sample_rate.is_none() {
        notes.push(
            "The file does not state its channel count or sample rate; they are \
             determined when it is decoded (shown as 0 until then)."
                .to_owned(),
        );
    }
    Ok(FileInfo {
        path: path.to_owned(),
        container: Container::Compressed,
        notes,
        codec,
        sample_format: None,
        channels: c.channels.unwrap_or(0),
        sample_rate: c.sample_rate.map_or(0.0, f64::from),
        frames: c.frames,
        center_frequency: 0.0,
        streamable: false,
        layout: None,
        iq_pairs: false,
    })
}

/// Open a file as a signal.
pub fn open(path: impl AsRef<Path>, options: &LoadOptions) -> Result<Signal> {
    Ok(open_cancellable(path, options, &|| false)?.signal)
}

/// [`open`] that can be cancelled and also returns notes about the loading.
///
/// `should_stop` is checked regularly while compressed audio is decoded (memory-mapped
/// files open instantly); when it returns true, loading fails with
/// [`Error::Cancelled`].
pub fn open_cancellable(
    path: impl AsRef<Path>,
    options: &LoadOptions,
    should_stop: &dyn Fn() -> bool,
) -> Result<Opened> {
    let path = path.as_ref();
    let info = match &options.raw {
        Some(raw) => raw_info(path, raw)?,
        None => probe(path)?,
    };
    open_probed_cancellable(&info, options, should_stop)
}

/// Open a file that was already inspected with [`probe`] (avoids parsing twice).
pub fn open_probed(info: &FileInfo, options: &LoadOptions) -> Result<Signal> {
    Ok(open_probed_cancellable(info, options, &|| false)?.signal)
}

/// [`open_probed`] that can be cancelled; see [`open_cancellable`].
pub fn open_probed_cancellable(
    info: &FileInfo,
    options: &LoadOptions,
    should_stop: &dyn Fn() -> bool,
) -> Result<Opened> {
    options.scaling.validate()?;
    let Some(layout) = info.layout else {
        // Compressed audio. The decoder picks the default selection from the real
        // channel count, which the probe may not know.
        let d = compressed::decode(&info.path, options.channels, options.scaling, should_stop)?;
        return Ok(Opened {
            signal: d.signal.with_center_frequency(info.center_frequency),
            notes: d.notes,
        });
    };
    let selection = options.channels.unwrap_or_else(|| info.default_selection());
    let map = mapped::map_file(&info.path)?;
    let name = info
        .path
        .file_name()
        .map_or_else(|| "file".to_owned(), |n| n.to_string_lossy().into_owned());
    let description = format!("{name} ({}, memory-mapped)", info.codec);
    let source = mapped::MappedSource::new(map, layout, selection, options.scaling, description)?;
    let signal = Signal::lazy(Arc::new(source), info.sample_rate)?;
    Ok(Opened {
        signal: signal.with_center_frequency(info.center_frequency),
        notes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use sn_core::{Complex32, SampleKind, Samples};

    use super::*;
    use crate::riff::tests::make_wav;

    fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let p = dir.join(name);
        std::fs::File::create(&p).unwrap().write_all(bytes).unwrap();
        p
    }

    fn i16_bytes(values: &[i16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    /// Found by the `open_file` fuzz target: a PCM header with 4096 bits per sample
    /// used to reach the compressed decoder, which panicked on an overflow.
    #[test]
    fn inconsistent_pcm_header_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = [
            0x52, 0x49, 0x46, 0x46, 0x24, 0x04, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d,
            0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0xff, 0xff, 0xff, 0x41, 0x00,
            0x00, 0xee, 0x02, 0x00, 0x04, 0x00, 0x10, 0x00, 0x64, 0x61, 0x74, 0x61, 0x00, 0x00,
            0x00, 0x00, 0x52, 0x00, 0x24, 0x00, 0x40, 0x49, 0x46,
        ];
        let p = write_file(dir.path(), "bad.wav", &bytes);
        let err = probe(&p).unwrap_err();
        assert!(matches!(err, Error::Malformed(_)), "{err}");
        assert!(open(&p, &LoadOptions::default()).is_err());
    }

    #[test]
    fn opens_stereo_wav_as_iq_or_channel_lazily() {
        let dir = tempfile::tempdir().unwrap();
        let data = i16_bytes(&[16_384, -16_384, 8192, 0, 0, 32_767]);
        let p = write_file(
            dir.path(),
            "iq.wav",
            &make_wav(*b"RIFF", 1, false, 2, 48_000, 16, &data),
        );
        let info = probe(&p).unwrap();
        assert_eq!(info.container, Container::Wav);
        assert_eq!(info.frames, Some(3));
        assert!(info.streamable);

        let s = open(&p, &LoadOptions::default()).unwrap();
        assert!(s.is_lazy());
        assert_eq!(s.kind(), SampleKind::Complex);
        let mut buf = [Complex32::new(0.0, 0.0); 4];
        assert_eq!(s.read_complex(0, &mut buf), 3);
        assert!((buf[0].re - 0.5).abs() < 1e-6 && (buf[0].im + 0.5).abs() < 1e-6);
        assert!((buf[1].re - 0.25).abs() < 1e-6);

        let q = open(
            &p,
            &LoadOptions {
                channels: Some(ChannelSelection::Mono(1)),
                ..Default::default()
            },
        )
        .unwrap();
        let mut r = [0.0_f32; 3];
        q.read_real(0, &mut r);
        assert!((r[0] + 0.5).abs() < 1e-6 && r[1].abs() < 1e-6);

        let bad = open(
            &p,
            &LoadOptions {
                channels: Some(ChannelSelection::Mono(5)),
                ..Default::default()
            },
        );
        assert!(bad.is_err());
    }

    #[test]
    fn extensible_float_and_rf64() {
        let dir = tempfile::tempdir().unwrap();
        let f: Vec<u8> = [0.25_f32, -0.75]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let p = write_file(
            dir.path(),
            "f.wav",
            &make_wav(*b"RIFF", 3, true, 1, 96_000, 32, &f),
        );
        let s = open(&p, &LoadOptions::default()).unwrap();
        let mut r = [0.0_f32; 2];
        s.read_real(0, &mut r);
        assert_eq!(r, [0.25, -0.75]);

        let d24 = [0x00, 0x00, 0x40, 0x00, 0x00, 0xC0];
        let p = write_file(
            dir.path(),
            "big.wav",
            &make_wav(*b"RF64", 1, false, 1, 8000, 24, &d24),
        );
        let info = probe(&p).unwrap();
        assert_eq!(info.container, Container::Rf64);
        let s = open(&p, &LoadOptions::default()).unwrap();
        s.read_real(0, &mut r);
        assert!((r[0] - 0.5).abs() < 1e-6 && (r[1] + 0.5).abs() < 1e-6);
    }

    #[test]
    fn truncated_data_is_handled_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = make_wav(*b"RIFF", 1, false, 1, 8000, 16, &i16_bytes(&[1, 2, 3, 4]));
        bytes.truncate(bytes.len() - 3); // header still claims 4 samples
        let p = write_file(dir.path(), "cut.wav", &bytes);
        let s = open(&p, &LoadOptions::default()).unwrap();
        assert_eq!(s.len(), 2);
        let mut r = [9.0_f32; 4];
        assert_eq!(s.read_real(0, &mut r), 2);
    }

    #[test]
    fn raw_and_scaling() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = vec![0xAA; 4]; // junk header
        bytes.extend([100_i8, -100, 10, -10].iter().map(|v| v.to_ne_bytes()[0]));
        let p = write_file(dir.path(), "cap.bin", &bytes);
        assert!(matches!(probe(&p), Err(Error::NeedsRawFormat)));
        let raw = RawFormat {
            format: SampleFormat::I8,
            endian: Endian::Little,
            channels: 2,
            sample_rate: 2_048_000.0,
            data_offset: 4,
        };
        let opts = LoadOptions {
            raw: Some(raw),
            scaling: Scaling {
                gain: 2.0,
                clip: Some(0.5),
            },
            ..Default::default()
        };
        let s = open(&p, &opts).unwrap();
        assert_eq!((s.len(), s.kind()), (2, SampleKind::Complex));
        let mut c = [Complex32::new(0.0, 0.0); 2];
        s.read_complex(0, &mut c);
        // 100/128 = 0.78 clipped to 0.5, rescaled to 1.0, gain 2 = 2.0
        assert!((c[0].re - 2.0).abs() < 1e-6 && (c[0].im + 2.0).abs() < 1e-6);
        // 10/128 = 0.078 / 0.5 * 2 = 0.3125
        assert!((c[1].re - 0.3125).abs() < 1e-6);
    }

    #[test]
    fn sigmf_pair() {
        let dir = tempfile::tempdir().unwrap();
        let data: Vec<u8> = [0.5_f32, -0.5, 0.25, 0.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        write_file(dir.path(), "rec.sigmf-data", &data);
        let meta = r#"{"global":{"core:datatype":"cf32_le","core:sample_rate":250000,
            "core:version":"1.0.0"},"captures":[{"core:sample_start":0,
            "core:frequency":433920000}],"annotations":[]}"#;
        let m = write_file(dir.path(), "rec.sigmf-meta", meta.as_bytes());
        let info = probe(&m).unwrap();
        assert_eq!(info.container, Container::Sigmf);
        assert_eq!(info.frames, Some(2));
        assert!(info.iq_pairs());
        assert_eq!(info.layout().map(|l| l.data_offset), Some(0));
        let s = open(&m, &LoadOptions::default()).unwrap();
        assert!((s.center_frequency() - 433_920_000.0).abs() < 1.0);
        let Samples::Lazy(_) = s.samples() else {
            panic!("expected memory-mapped samples")
        };
        let mut c = [Complex32::new(0.0, 0.0); 2];
        s.read_complex(0, &mut c);
        assert_eq!(c[1], Complex32::new(0.25, 0.0));
    }

    #[test]
    fn raw_offset_past_the_data_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_file(dir.path(), "cap.bin", &[0; 10]);
        let raw = |data_offset, channels| LoadOptions {
            raw: Some(RawFormat {
                format: SampleFormat::I16,
                endian: Endian::Little,
                channels,
                sample_rate: 1000.0,
                data_offset,
            }),
            ..Default::default()
        };
        let err = open(&p, &raw(10, 1)).unwrap_err().to_string();
        assert!(err.contains("beyond the end"), "{err}");
        let err = open(&p, &raw(8, 2)).unwrap_err().to_string();
        assert!(err.contains("less than one frame"), "{err}");
        assert_eq!(open(&p, &raw(2, 2)).unwrap().len(), 2);
        let empty = write_file(dir.path(), "empty.bin", &[]);
        let err = open(&empty, &raw(0, 1)).unwrap_err().to_string();
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn invalid_scaling_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let data = i16_bytes(&[1, 2]);
        let p = write_file(
            dir.path(),
            "m.wav",
            &make_wav(*b"RIFF", 1, false, 1, 8000, 16, &data),
        );
        let opts = LoadOptions {
            scaling: Scaling {
                gain: f32::NAN,
                clip: None,
            },
            ..Default::default()
        };
        assert!(open(&p, &opts).is_err());
    }

    #[test]
    fn sigmf_multichannel_with_header_bytes() {
        let dir = tempfile::tempdir().unwrap();
        // 4-byte header, then frames of two ci16 channels: (I0, Q0, I1, Q1).
        let mut data = vec![0xEE; 4];
        data.extend(i16_bytes(&[
            16_384, -16_384, 8192, 0, 0, 16_384, -8192, 8192,
        ]));
        write_file(dir.path(), "cap.bin", &data);
        let meta = r#"{"global":{"core:datatype":"ci16_le","core:sample_rate":1000,
            "core:num_channels":2,"core:dataset":"cap.bin","core:version":"1.0.0"},
            "captures":[{"core:sample_start":0,"core:header_bytes":4}],"annotations":[]}"#;
        let m = write_file(dir.path(), "rec.sigmf-meta", meta.as_bytes());
        let info = probe(&m).unwrap();
        assert_eq!((info.channels, info.frames), (4, Some(2)));
        assert_eq!(info.path, dir.path().join("cap.bin"));
        assert!(info.iq_pairs());
        let layout = info.layout().unwrap();
        assert_eq!((layout.data_offset, layout.endian), (4, Endian::Little));
        assert_eq!(
            info.default_selection(),
            ChannelSelection::IQ { i: 0, q: 1 }
        );
        let s = open(&m, &LoadOptions::default()).unwrap();
        let mut c = [Complex32::new(0.0, 0.0); 2];
        assert_eq!(s.read_complex(0, &mut c), 2);
        assert_eq!(c, [Complex32::new(0.5, -0.5), Complex32::new(0.0, 0.5)]);
        let second = LoadOptions {
            channels: Some(ChannelSelection::IQ { i: 2, q: 3 }),
            ..Default::default()
        };
        let s = open(&m, &second).unwrap();
        s.read_complex(0, &mut c);
        assert_eq!(c[0], Complex32::new(0.25, 0.0));
    }

    #[test]
    fn sigmf_missing_data_file_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let meta = r#"{"global":{"core:datatype":"cf32_le","core:sample_rate":1000}}"#;
        let m = write_file(dir.path(), "lost.sigmf-meta", meta.as_bytes());
        let err = probe(&m).unwrap_err().to_string();
        assert!(
            err.contains("lost.sigmf-data") && err.contains("next to"),
            "{err}"
        );
    }

    #[test]
    fn missing_file_error_names_the_path() {
        let err = probe("/definitely/not/here.wav").unwrap_err().to_string();
        assert!(err.contains("/definitely/not/here.wav"), "{err}");
    }

    #[test]
    fn wav_with_large_leading_chunk_opens() {
        let dir = tempfile::tempdir().unwrap();
        let base = make_wav(*b"RIFF", 1, false, 1, 8000, 16, &i16_bytes(&[16_384, 0]));
        let mut wav = base[..12].to_vec();
        let big = 2 << 20;
        wav.extend_from_slice(b"iXML");
        wav.extend_from_slice(&(big as u32).to_le_bytes());
        wav.resize(wav.len() + big, b' ');
        wav.extend_from_slice(&base[12..]);
        let p = write_file(dir.path(), "bwf.wav", &wav);
        let s = open(&p, &LoadOptions::default()).unwrap();
        let mut r = [0.0_f32; 2];
        assert_eq!(s.read_real(0, &mut r), 2);
        assert!((r[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn notes_warn_about_poor_formats() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_file(
            dir.path(),
            "a.wav",
            &make_wav(*b"RIFF", 6, false, 1, 8000, 8, &[0xD5; 8]),
        );
        let info = probe(&p).unwrap();
        assert_eq!(info.codec, "A-law");
        assert_eq!(info.notes.len(), 1, "{:?}", info.notes);
        let s = open(&p, &LoadOptions::default()).unwrap();
        let mut r = [0.0_f32; 1];
        s.read_real(0, &mut r);
        assert!((r[0] - 8.0 / 32_768.0).abs() < 1e-7);
    }
}
