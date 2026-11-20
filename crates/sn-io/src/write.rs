//! Writers for WAV (RIFF, or RF64 above 4 GB), raw samples and SigMF.
//!
//! Samples are streamed in blocks, so even memory-mapped signals of any size can be
//! saved without loading them fully. Every file is written to a temporary file in the
//! target directory and renamed over the target only when complete, so a failed or
//! cancelled save never leaves a damaged file behind and never truncates a file that
//! is still memory-mapped.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use sn_core::{Complex32, SampleKind, Signal};

use crate::format::{Endian, SampleFormat};
use crate::{Error, Result, io_err};

/// Frames read from the signal per block; progress is reported once per block.
const CHUNK: usize = 1 << 16;

/// Sample format to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WavFormat {
    /// 16-bit signed PCM, the format SA's author recommends for analysis. Values are
    /// limited to -1.0..=1.0 before scaling.
    #[default]
    Pcm16,
    /// 32-bit IEEE float, lossless for our internal `f32` samples.
    Float32,
}

impl WavFormat {
    fn sample_format(self) -> SampleFormat {
        match self {
            Self::Pcm16 => SampleFormat::I16,
            Self::Float32 => SampleFormat::F32,
        }
    }
}

/// Progress callback for the `*_with_progress` writers: called once per block with
/// `(frames written, total frames)`. Return `true` to cancel the save, which then
/// fails with [`Error::Cancelled`] and leaves any existing target file untouched.
pub type Progress<'a> = &'a dyn Fn(u64, u64) -> bool;

fn no_progress(_: u64, _: u64) -> bool {
    false
}

/// Whether a WAV header can store `rate`: finite, rounding to 1..=`u32::MAX` Hz, and
/// within 1e-9 Hz of that whole number. The WAV writers accept exactly these rates, so
/// a caller can refuse a rate before spending time making the signal.
pub fn wav_can_store_rate(rate: f64) -> bool {
    integer_rate(rate).is_ok()
}

fn integer_rate(rate: f64) -> Result<u32> {
    let rounded = rate.round();
    if !rate.is_finite() || rounded < 1.0 || rounded > f64::from(u32::MAX) {
        return Err(Error::Unsupported(format!(
            "WAV cannot store a sample rate of {rate} Hz; save as SigMF instead"
        )));
    }
    if (rate - rounded).abs() > 1e-9 {
        return Err(Error::FractionalSampleRate(rate));
    }
    Ok(rounded as u32)
}

fn channels_of(signal: &Signal) -> u16 {
    match signal.kind() {
        SampleKind::Real => 1,
        SampleKind::Complex => 2,
    }
}

/// Write a file atomically: `write` fills a uniquely named temporary file in the
/// directory of `path`, which is then synced and renamed over `path` in one step.
///
/// This is the one atomic-save routine for SigNexora; every writer in this crate and
/// the session files use it. It guarantees that:
///
/// - an existing `path` is never left empty or half-written, because when `write`
///   fails or the save is cancelled the temporary file is removed and `path` is
///   untouched;
/// - two saves at once never share a temporary file;
/// - a reader that still has the old file open keeps seeing the old data, since the
///   rename leaves it alive;
/// - on Unix the saved file keeps the mode of the file it replaces (0644 for a new
///   file), not the private mode of temporary files.
///
/// Refuses a path with no file name, and refuses to replace a file that a signal in
/// this process is memory-mapping. Errors from `write` are returned unchanged.
pub fn write_atomically(path: &Path, write: impl FnOnce(&mut File) -> Result<()>) -> Result<()> {
    if path.file_name().is_none() {
        return Err(io_err(path)(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the path has no file name",
        )));
    }
    if crate::mapped::is_mapped(path) {
        return Err(Error::Unsupported(format!(
            "cannot overwrite {}, the file that is open; save under a new name",
            path.display()
        )));
    }
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let mut builder = tempfile::Builder::new();
    builder.prefix(".signexora-save-").suffix(".tmp");
    #[cfg(unix)]
    {
        // Temporary files are private by default; give the saved file the mode of
        // the file it replaces, or the usual one for a new file.
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path).map_or(0o644, |m| m.permissions().mode() & 0o777);
        builder.permissions(std::fs::Permissions::from_mode(mode));
    }
    // The temporary file is removed when `tmp` drops, so every early return below
    // cleans up after itself.
    let mut tmp = builder.tempfile_in(dir).map_err(io_err(dir))?;
    write(tmp.as_file_mut())?;
    tmp.as_file().sync_all().map_err(io_err(tmp.path()))?;
    tmp.persist(path).map_err(|e| io_err(path)(e.error))?;
    Ok(())
}

/// Encode one sample. Non-finite values become silence in every format.
fn encode(v: f32, format: SampleFormat, endian: Endian, out: &mut Vec<u8>) {
    let le = endian == Endian::Little;
    let v = if v.is_finite() { v } else { 0.0 };
    let clamped = v.clamp(-1.0, 1.0);
    macro_rules! put {
        ($v:expr) => {{
            let b = if le {
                $v.to_le_bytes()
            } else {
                $v.to_be_bytes()
            };
            out.extend_from_slice(&b);
        }};
    }
    match format {
        SampleFormat::U8 => out.push(((clamped * 127.0).round() + 128.0) as u8),
        SampleFormat::I8 => out.push(((clamped * 127.0).round() as i8).to_ne_bytes()[0]),
        SampleFormat::I16 => put!((clamped * 32_767.0).round() as i16),
        SampleFormat::I24 => {
            let x = (f64::from(clamped) * 8_388_607.0).round() as i32;
            let [b0, b1, b2, _] = x.to_le_bytes();
            if le {
                out.extend_from_slice(&[b0, b1, b2]);
            } else {
                out.extend_from_slice(&[b2, b1, b0]);
            }
        }
        SampleFormat::I32 => put!((f64::from(clamped) * 2_147_483_647.0).round() as i32),
        SampleFormat::F32 => put!(v),
        SampleFormat::F64 => put!(f64::from(v)),
        // Rejected by `check_writable` before any file is touched.
        SampleFormat::ALaw | SampleFormat::MuLaw => {}
    }
}

fn check_writable(format: SampleFormat) -> Result<()> {
    if matches!(format, SampleFormat::ALaw | SampleFormat::MuLaw) {
        return Err(Error::Unsupported(
            "saving as G.711 is not supported; choose PCM or float".into(),
        ));
    }
    Ok(())
}

/// Stream every sample of `signal` (complex as I then Q) to `w`. Returns the number
/// of bytes written.
fn write_samples(
    w: &mut impl Write,
    path: &Path,
    signal: &Signal,
    format: SampleFormat,
    endian: Endian,
    progress: Progress<'_>,
) -> Result<u64> {
    let io = io_err(path);
    let len = signal.len();
    let total = len as u64;
    let mut bytes = Vec::with_capacity(CHUNK * 2 * format.bytes());
    let mut written = 0_u64;
    let mut real = Vec::new();
    let mut cplx = Vec::new();
    match signal.kind() {
        SampleKind::Real => real.resize(CHUNK, 0.0_f32),
        SampleKind::Complex => cplx.resize(CHUNK, Complex32::new(0.0, 0.0)),
    }
    for start in (0..len).step_by(CHUNK) {
        if progress(start as u64, total) {
            return Err(Error::Cancelled);
        }
        bytes.clear();
        let want = CHUNK.min(len.saturating_sub(start));
        let got = match signal.kind() {
            SampleKind::Real => {
                let buf = real.get_mut(..want).unwrap_or_default();
                let n = signal.read_real(start, buf).min(want);
                for v in buf.iter().take(n) {
                    encode(*v, format, endian, &mut bytes);
                }
                n
            }
            SampleKind::Complex => {
                let buf = cplx.get_mut(..want).unwrap_or_default();
                let n = signal.read_complex(start, buf).min(want);
                for c in buf.iter().take(n) {
                    encode(c.re, format, endian, &mut bytes);
                    encode(c.im, format, endian, &mut bytes);
                }
                n
            }
        };
        if got < want {
            // A truncated save would look complete, so fail instead; the target file
            // stays untouched because the data goes to a temporary file first.
            return Err(io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "the source returned only {got} of {want} samples at sample {start}; \
                     the file may have changed on disk, reopen it and save again"
                ),
            )));
        }
        w.write_all(&bytes).map_err(&io)?;
        written += bytes.len() as u64;
    }
    if progress(total, total) {
        return Err(Error::Cancelled);
    }
    Ok(written)
}

/// A WAV header for 16-bit PCM or 32-bit float. Its length depends only on `rf64` and
/// `format`, so a placeholder can be written first and replaced at the end.
fn wav_header(rf64: bool, channels: u16, rate: u32, format: WavFormat, data_bytes: u64) -> Vec<u8> {
    let (tag, bits): (u16, u16) = match format {
        WavFormat::Pcm16 => (1, 16),
        WavFormat::Float32 => (3, 32),
    };
    let block_align = channels.saturating_mul(bits / 8);
    let byte_rate = u32::try_from(u64::from(rate) * u64::from(block_align)).unwrap_or(u32::MAX);
    let frames = data_bytes / u64::from(block_align.max(1));
    let float = format == WavFormat::Float32;
    let u32_or_max = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);

    let mut fmt = Vec::with_capacity(18);
    fmt.extend_from_slice(&tag.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&rate.to_le_bytes());
    fmt.extend_from_slice(&byte_rate.to_le_bytes());
    fmt.extend_from_slice(&block_align.to_le_bytes());
    fmt.extend_from_slice(&bits.to_le_bytes());
    if float {
        fmt.extend_from_slice(&0_u16.to_le_bytes()); // cbSize
    }

    let ds64_len = if rf64 { 8 + 28 } else { 0 };
    let fact_len = if float { 8 + 4 } else { 0 };
    let header_len = 12 + ds64_len + 8 + fmt.len() as u64 + fact_len + 8;
    let pad = data_bytes & 1;
    let riff_size = (header_len - 8)
        .saturating_add(data_bytes)
        .saturating_add(pad);

    let mut h = Vec::with_capacity(usize::try_from(header_len).unwrap_or(0));
    h.extend_from_slice(if rf64 { b"RF64" } else { b"RIFF" });
    h.extend_from_slice(
        &(if rf64 {
            u32::MAX
        } else {
            u32_or_max(riff_size)
        })
        .to_le_bytes(),
    );
    h.extend_from_slice(b"WAVE");
    if rf64 {
        h.extend_from_slice(b"ds64");
        h.extend_from_slice(&28_u32.to_le_bytes());
        h.extend_from_slice(&riff_size.to_le_bytes());
        h.extend_from_slice(&data_bytes.to_le_bytes());
        h.extend_from_slice(&frames.to_le_bytes());
        h.extend_from_slice(&0_u32.to_le_bytes()); // no extra size table
    }
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    h.extend_from_slice(&fmt);
    if float {
        h.extend_from_slice(b"fact");
        h.extend_from_slice(&4_u32.to_le_bytes());
        let f = if rf64 { u32::MAX } else { u32_or_max(frames) };
        h.extend_from_slice(&f.to_le_bytes());
    }
    h.extend_from_slice(b"data");
    let d = if rf64 {
        u32::MAX
    } else {
        u32_or_max(data_bytes)
    };
    h.extend_from_slice(&d.to_le_bytes());
    h
}

/// Write a signal: real as mono, complex as stereo I/Q (I left, Q right). Files over
/// 4 GB are written as RF64, which [`open`](crate::open) reads.
pub fn write_wav(path: impl AsRef<Path>, signal: &Signal, format: WavFormat) -> Result<()> {
    write_wav_with_progress(path, signal, format, &no_progress)
}

/// [`write_wav`] with progress reporting and cancellation; see [`Progress`].
pub fn write_wav_with_progress(
    path: impl AsRef<Path>,
    signal: &Signal,
    format: WavFormat,
    progress: Progress<'_>,
) -> Result<()> {
    write_wav_impl(path.as_ref(), signal, format, progress, u64::from(u32::MAX))
}

/// `riff_limit` is the largest RIFF size a plain WAV may have; tests lower it to
/// exercise the RF64 path without writing 4 GB.
fn write_wav_impl(
    path: &Path,
    signal: &Signal,
    format: WavFormat,
    progress: Progress<'_>,
    riff_limit: u64,
) -> Result<()> {
    let channels = channels_of(signal);
    let rate = integer_rate(signal.sample_rate())?;
    let data_bytes = (signal.len() as u64)
        .checked_mul(u64::from(channels))
        .and_then(|n| n.checked_mul(format.sample_format().bytes() as u64))
        .ok_or_else(|| Error::Unsupported("the signal is too large to save".into()))?;
    let header_len = wav_header(false, channels, rate, format, 0).len() as u64;
    let rf64 = (header_len - 8).saturating_add(data_bytes) > riff_limit;
    let io = io_err(path);
    write_atomically(path, |file| {
        let mut w = BufWriter::new(file);
        w.write_all(&wav_header(rf64, channels, rate, format, 0))
            .map_err(&io)?;
        let written = write_samples(
            &mut w,
            path,
            signal,
            format.sample_format(),
            Endian::Little,
            progress,
        )?;
        if written & 1 == 1 {
            w.write_all(&[0]).map_err(&io)?;
        }
        let file = w.into_inner().map_err(|e| io(e.into_error()))?;
        file.seek(SeekFrom::Start(0)).map_err(&io)?;
        file.write_all(&wav_header(rf64, channels, rate, format, written))
            .map_err(&io)?;
        Ok(())
    })
}

/// Write raw interleaved samples (real: one channel; I/Q: I then Q) with no header.
pub fn write_raw(
    path: impl AsRef<Path>,
    signal: &Signal,
    format: SampleFormat,
    endian: Endian,
) -> Result<()> {
    write_raw_with_progress(path, signal, format, endian, &no_progress)
}

/// [`write_raw`] with progress reporting and cancellation; see [`Progress`].
pub fn write_raw_with_progress(
    path: impl AsRef<Path>,
    signal: &Signal,
    format: SampleFormat,
    endian: Endian,
    progress: Progress<'_>,
) -> Result<()> {
    let path = path.as_ref();
    check_writable(format)?;
    write_atomically(path, |file| {
        let mut w = BufWriter::new(file);
        write_samples(&mut w, path, signal, format, endian, progress)?;
        w.flush().map_err(io_err(path))
    })
}

/// Write a SigMF recording: `<base>.sigmf-data` (32-bit float, little-endian) and
/// `<base>.sigmf-meta`. SigMF keeps fractional sample rates, which WAV cannot.
/// Returns the path of the metadata file.
pub fn write_sigmf(base: impl AsRef<Path>, signal: &Signal, description: &str) -> Result<PathBuf> {
    write_sigmf_with_progress(base, signal, description, &no_progress)
}

/// [`write_sigmf`] with progress reporting and cancellation; see [`Progress`].
pub fn write_sigmf_with_progress(
    base: impl AsRef<Path>,
    signal: &Signal,
    description: &str,
    progress: Progress<'_>,
) -> Result<PathBuf> {
    let base = base.as_ref();
    let data = base.with_extension("sigmf-data");
    let meta = base.with_extension("sigmf-meta");
    let datatype = match signal.kind() {
        SampleKind::Real => "rf32_le",
        SampleKind::Complex => "cf32_le",
    };
    let mut capture = serde_json::Map::new();
    capture.insert("core:sample_start".into(), serde_json::json!(0));
    if signal.center_frequency() != 0.0 {
        capture.insert(
            "core:frequency".into(),
            serde_json::json!(signal.center_frequency()),
        );
    }
    let json = serde_json::json!({
        "global": {
            "core:datatype": datatype,
            "core:sample_rate": signal.sample_rate(),
            "core:version": "1.0.0",
            "core:description": description,
            "core:recorder": "SigNexora",
        },
        "captures": [capture],
        "annotations": [],
    });
    let text = serde_json::to_string_pretty(&json)
        .map_err(|e| Error::Malformed(format!("cannot encode SigMF metadata: {e}")))?;
    if crate::mapped::is_mapped(&meta) {
        return Err(Error::Unsupported(format!(
            "cannot overwrite {}, the file that is open; save under a new name",
            meta.display()
        )));
    }
    write_raw_with_progress(&data, signal, SampleFormat::F32, Endian::Little, progress)?;
    write_atomically(&meta, |file| {
        file.write_all(text.as_bytes()).map_err(io_err(&meta))
    })?;
    Ok(meta)
}

#[cfg(test)]
mod tests {
    use sn_core::Samples;
    use sn_synth::{PskParams, psk, random_symbols};

    use super::*;
    use crate::{LoadOptions, open, probe};
    use std::path::Path;

    #[test]
    fn pcm16_round_trip_is_within_one_lsb() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.wav");
        let p = PskParams {
            order: 4,
            baud: 2400.0,
            carrier: 1800.0,
            sample_rate: 48_000.0,
            amplitude: 0.5,
        };
        let s = psk(&p, &random_symbols(100, 4, 1)).unwrap();
        write_wav(&path, &s, WavFormat::Pcm16).unwrap();
        let info = probe(&path).unwrap();
        assert_eq!((info.channels, info.sample_rate), (1, 48_000.0));
        assert_eq!(info.frames, Some(s.len() as u64));
        let back = open(&path, &LoadOptions::default()).unwrap();
        let mut b = vec![0.0_f32; s.len()];
        back.read_real(0, &mut b);
        let Samples::Real(a) = s.samples() else {
            panic!("expected real")
        };
        let worst = a
            .iter()
            .zip(&b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0_f32, f32::max);
        assert!(worst <= 1.0 / 32_767.0, "max error {worst}");
    }

    #[test]
    fn complex_round_trips_through_lazy_signal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("iq.wav");
        let iq: Vec<Complex32> = (0..64)
            .map(|i| Complex32::new(i as f32 / 64.0, -0.25))
            .collect();
        let s = Signal::complex(iq.clone(), 96_000.0).unwrap();
        write_wav(&path, &s, WavFormat::Float32).unwrap();
        // Re-save the lazily loaded copy and compare again.
        let lazy = open(&path, &LoadOptions::default()).unwrap();
        let path2 = dir.path().join("iq2.wav");
        write_wav(&path2, &lazy, WavFormat::Float32).unwrap();
        let back = open(&path2, &LoadOptions::default()).unwrap();
        let mut c = vec![Complex32::new(0.0, 0.0); 64];
        assert_eq!(back.read_complex(0, &mut c), 64);
        assert_eq!(c, iq);
    }

    #[test]
    fn raw_and_sigmf_round_trip() {
        use crate::{LoadOptions, RawFormat, open};
        let dir = tempfile::tempdir().unwrap();
        let iq: Vec<Complex32> = (0..100)
            .map(|i| Complex32::new(i as f32 / 200.0, -0.25))
            .collect();
        let s = Signal::complex(iq.clone(), 7014.322)
            .unwrap()
            .with_center_frequency(14_490_000.0);
        for (fmt, endian, tol) in [
            (SampleFormat::I16, Endian::Big, 1.0 / 32_000.0),
            (SampleFormat::I24, Endian::Little, 1e-6),
            (SampleFormat::F64, Endian::Little, 1e-7),
        ] {
            let p = dir.path().join(format!("x_{fmt:?}.raw"));
            write_raw(&p, &s, fmt, endian).unwrap();
            let raw = RawFormat {
                format: fmt,
                endian,
                channels: 2,
                sample_rate: 7014.322,
                data_offset: 0,
            };
            let back = open(
                &p,
                &LoadOptions {
                    raw: Some(raw),
                    ..Default::default()
                },
            )
            .unwrap();
            let mut c = vec![Complex32::new(0.0, 0.0); 100];
            assert_eq!(back.read_complex(0, &mut c), 100);
            for (a, b) in c.iter().zip(&iq) {
                assert!((a - b).norm() < tol, "{fmt:?}");
            }
        }
        let meta = write_sigmf(dir.path().join("rec"), &s, "test").unwrap();
        let back = open(&meta, &LoadOptions::default()).unwrap();
        assert!(
            (back.sample_rate() - 7014.322).abs() < 1e-9,
            "fractional rate kept"
        );
        assert!((back.center_frequency() - 14_490_000.0).abs() < 1e-3);
        let mut c = vec![Complex32::new(0.0, 0.0); 100];
        back.read_complex(0, &mut c);
        assert_eq!(c, iq);
    }

    #[test]
    fn fractional_rate_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let s = Signal::real(vec![0.0_f32; 8], 6999.87).unwrap();
        let err = write_wav(dir.path().join("f.wav"), &s, WavFormat::Pcm16).unwrap_err();
        assert!(matches!(err, Error::FractionalSampleRate(_)));
    }

    #[test]
    fn wav_can_store_rate_matches_the_writer() {
        assert!(wav_can_store_rate(8000.0));
        assert!(wav_can_store_rate(7_999.999_999_999_999));
        for bad in [8000.5, 0.0, 0.4, -1.0, f64::NAN, f64::INFINITY, 1e12] {
            assert!(!wav_can_store_rate(bad), "{bad}");
        }
    }

    #[test]
    fn integer_rate_rejects_out_of_range_and_nan() {
        assert_eq!(integer_rate(48_000.0).unwrap(), 48_000);
        for bad in [f64::NAN, f64::INFINITY, 0.2, -5.0, 1e12] {
            assert!(
                matches!(integer_rate(bad), Err(Error::Unsupported(_))),
                "{bad}"
            );
        }
        assert!(matches!(
            integer_rate(6999.87),
            Err(Error::FractionalSampleRate(_))
        ));
    }

    #[test]
    fn non_finite_samples_encode_as_silence() {
        for fmt in [
            SampleFormat::U8,
            SampleFormat::I8,
            SampleFormat::I16,
            SampleFormat::I24,
            SampleFormat::I32,
        ] {
            let mut b = Vec::new();
            encode(f32::NAN, fmt, Endian::Little, &mut b);
            let v = fmt.decode(&b, Endian::Little).unwrap();
            assert!(v.abs() < 1e-6, "{fmt:?} decoded NaN as {v}");
        }
    }

    #[test]
    fn plain_wav_is_readable_by_hound() {
        let dir = tempfile::tempdir().unwrap();
        let iq: Vec<Complex32> = (0..10)
            .map(|i| Complex32::new(i as f32 / 20.0, -0.5))
            .collect();
        let s = Signal::complex(iq.clone(), 8000.0).unwrap();
        for format in [WavFormat::Pcm16, WavFormat::Float32] {
            let p = dir.path().join(format!("{format:?}.wav"));
            write_wav(&p, &s, format).unwrap();
            let mut r = hound::WavReader::open(&p).unwrap();
            let spec = r.spec();
            assert_eq!((spec.channels, spec.sample_rate), (2, 8000));
            assert_eq!(r.duration(), 10);
            if format == WavFormat::Float32 {
                let v: Vec<f32> = r.samples::<f32>().map(Result::unwrap).collect();
                assert_eq!((v[2], v[3]), (iq[1].re, iq[1].im));
            } else {
                let v: Vec<i16> = r.samples::<i16>().map(Result::unwrap).collect();
                assert_eq!(v[3], -16_384);
            }
        }
    }

    #[test]
    fn large_output_is_written_as_rf64() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.wav");
        let v: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0) - 0.5).collect();
        let s = Signal::real(v.clone(), 1000.0).unwrap();
        // Pretend the RIFF limit is 100 bytes so the RF64 path is taken.
        write_wav_impl(&p, &s, WavFormat::Float32, &no_progress, 100).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        assert_eq!(&bytes[0..4], b"RF64");
        let info = probe(&p).unwrap();
        assert_eq!(info.container, crate::Container::Rf64);
        assert_eq!(info.frames, Some(1000));
        let back = open(&p, &LoadOptions::default()).unwrap();
        let mut r = vec![0.0_f32; 1000];
        assert_eq!(back.read_real(0, &mut r), 1000);
        assert_eq!(r, v);
    }

    /// A lazy source that claims to be enormous; its samples are never read.
    #[derive(Debug)]
    struct Huge;

    impl sn_core::SampleSource for Huge {
        fn len(&self) -> usize {
            usize::MAX
        }
        fn kind(&self) -> SampleKind {
            SampleKind::Complex
        }
        fn read_complex(&self, _: usize, _: &mut [Complex32]) -> usize {
            panic!("the size check must run before any sample is read")
        }
        fn read_real(&self, _: usize, _: &mut [f32]) -> usize {
            panic!("the size check must run before any sample is read")
        }
    }

    #[test]
    fn impossible_size_is_refused_before_creating_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("huge.wav");
        let s = Signal::lazy(std::sync::Arc::new(Huge), 1000.0).unwrap();
        let err = write_wav(&p, &s, WavFormat::Float32).unwrap_err();
        assert!(matches!(err, Error::Unsupported(_)), "{err}");
        assert!(!p.exists());
    }

    /// A lazy source that delivers fewer samples than it claims, as a file truncated
    /// on disk after opening would.
    #[derive(Debug)]
    struct Shrunk {
        claimed: usize,
        real: usize,
    }

    impl sn_core::SampleSource for Shrunk {
        fn len(&self) -> usize {
            self.claimed
        }
        fn kind(&self) -> SampleKind {
            SampleKind::Real
        }
        fn read_complex(&self, _: usize, _: &mut [Complex32]) -> usize {
            0
        }
        fn read_real(&self, start: usize, out: &mut [f32]) -> usize {
            let n = out.len().min(self.real.saturating_sub(start));
            out[..n].fill(0.5);
            n
        }
    }

    #[test]
    fn short_read_fails_and_keeps_the_old_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("keep.wav");
        // Short in the middle of a block, and a source that ends at a block boundary.
        for real in [CHUNK + 10, CHUNK] {
            std::fs::write(&p, b"original").unwrap();
            let src = Shrunk {
                claimed: 3 * CHUNK,
                real,
            };
            let s = Signal::lazy(std::sync::Arc::new(src), 8000.0).unwrap();
            let err = write_wav(&p, &s, WavFormat::Pcm16).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("keep.wav"), "{msg}");
            assert!(
                msg.contains(&format!(
                    "only {} of {CHUNK} samples at sample {CHUNK}",
                    real - CHUNK
                )),
                "{msg}"
            );
            assert!(msg.contains("reopen it and save again"), "{msg}");
            assert_eq!(std::fs::read(&p).unwrap(), b"original");
            assert_eq!(only_file_in(dir.path()), ["keep.wav"]);
            let err = write_raw(&p, &s, SampleFormat::F32, Endian::Little).unwrap_err();
            assert!(matches!(err, Error::Io { .. }), "{err}");
        }
    }

    fn only_file_in(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn cancelling_keeps_the_old_file_and_leaves_no_temporary() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("keep.wav");
        std::fs::write(&p, b"original").unwrap();
        let s = Signal::real(vec![0.25_f32; 3 * CHUNK], 8000.0).unwrap();
        let calls = std::cell::Cell::new(0_u32);
        let stop = |done: u64, total: u64| {
            calls.set(calls.get() + 1);
            assert_eq!(total, 3 * CHUNK as u64);
            done >= CHUNK as u64
        };
        let err = write_wav_with_progress(&p, &s, WavFormat::Pcm16, &stop).unwrap_err();
        assert!(matches!(err, Error::Cancelled));
        assert_eq!(calls.get(), 2);
        assert_eq!(std::fs::read(&p).unwrap(), b"original");
        assert_eq!(only_file_in(dir.path()), ["keep.wav"]);
    }

    #[test]
    fn g711_is_refused_without_touching_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.raw");
        std::fs::write(&p, b"original").unwrap();
        let s = Signal::real(Vec::<f32>::new(), 8000.0).unwrap();
        for fmt in [SampleFormat::ALaw, SampleFormat::MuLaw] {
            assert!(write_raw(&p, &s, fmt, Endian::Little).is_err());
        }
        assert_eq!(std::fs::read(&p).unwrap(), b"original");
    }

    #[test]
    fn the_open_file_cannot_be_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("rec.wav");
        let s = Signal::real(vec![0.5_f32; 100], 8000.0).unwrap();
        write_wav(&p, &s, WavFormat::Pcm16).unwrap();
        let lazy = open(&p, &LoadOptions::default()).unwrap();
        let err = write_wav(&p, &lazy, WavFormat::Float32).unwrap_err();
        assert!(err.to_string().contains("save under a new name"), "{err}");
        let err = write_raw(&p, &lazy, SampleFormat::F32, Endian::Little).unwrap_err();
        assert!(matches!(err, Error::Unsupported(_)));
        // The mapping still reads the original data.
        let mut r = [0.0_f32; 1];
        lazy.read_real(99, &mut r);
        assert!((r[0] - 0.5).abs() < 1e-4);
        // Once the signal is gone the file can be replaced.
        drop(lazy);
        write_wav(&p, &s, WavFormat::Float32).unwrap();
    }

    #[test]
    fn io_errors_name_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("no_such_dir").join("x.wav");
        let s = Signal::real(vec![0.0_f32; 4], 8000.0).unwrap();
        let err = write_wav(&p, &s, WavFormat::Pcm16).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
        assert!(err.to_string().contains("no_such_dir"), "{err}");
    }

    #[test]
    fn write_atomically_replaces_on_success_and_keeps_the_old_file_on_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("notes.json");
        std::fs::write(&p, b"old").unwrap();
        let err = write_atomically(&p, |f| {
            f.write_all(b"half").map_err(io_err(&p))?;
            Err(Error::Malformed("boom".into()))
        })
        .unwrap_err();
        assert!(matches!(err, Error::Malformed(_)), "{err}");
        assert_eq!(std::fs::read(&p).unwrap(), b"old");
        assert_eq!(only_file_in(dir.path()), ["notes.json"]);
        write_atomically(&p, |f| f.write_all(b"new").map_err(io_err(&p))).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"new");
        assert_eq!(only_file_in(dir.path()), ["notes.json"]);
    }

    #[test]
    fn write_atomically_refuses_a_path_without_a_file_name() {
        let err = write_atomically(Path::new("/"), |_| Ok(())).unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert!(err.to_string().contains("no file name"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn write_atomically_keeps_the_mode_of_the_replaced_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("shared.json");
        std::fs::write(&p, b"old").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o640)).unwrap();
        write_atomically(&p, |f| f.write_all(b"new").map_err(io_err(&p))).unwrap();
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }
}
