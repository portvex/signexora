//! Compressed audio import (MP3, FLAC, Ogg Vorbis, ADPCM WAV) through Symphonia.
//!
//! SA's author warns that compressed recordings are unsuitable for serious analysis;
//! we import them anyway and say so in the file notes. The whole file is decoded into
//! memory.

use std::fs::File;
use std::path::Path;

use sn_core::{Complex32, Signal};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::TimeBase;

use crate::format::{ChannelSelection, Scaling};
use crate::mapped::check_channel;
use crate::{Error, Result, io_err};

/// Most memory the decoded signal may use. Compressed files are decoded into memory,
/// and a long multichannel file could otherwise exhaust it.
const MAX_DECODED_BYTES: u64 = 4 << 30;

/// Basic facts about a compressed file, without decoding it. `None` means the
/// container does not say; the decoder finds out.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompressedInfo {
    pub(crate) codec: String,
    pub(crate) channels: Option<u16>,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) frames: Option<u64>,
}

/// A decoded file and remarks about the decoding for the user.
#[derive(Debug, Clone)]
pub(crate) struct Decoded {
    pub(crate) signal: Signal,
    pub(crate) notes: Vec<String>,
}

fn sym_err(e: &SymError) -> Error {
    Error::Unsupported(format!("cannot decode audio: {e}"))
}

fn channel_count(n: usize) -> Result<u16> {
    u16::try_from(n).map_err(|_| {
        Error::Unsupported(format!(
            "the file reports {n} channels; at most {} are supported",
            u16::MAX
        ))
    })
}

fn open_format(path: &Path) -> Result<Box<dyn symphonia::core::formats::FormatReader>> {
    let file = File::open(path).map_err(io_err(path))?;
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| sym_err(&e))
}

/// Run third-party decoder code so that a panic inside it (malformed input that
/// the decoder does not check) becomes an error instead of ending the program.
fn guarded<T>(what: &str, f: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
        Err(Error::Malformed(format!(
            "the audio decoder failed while {what}; the file is probably damaged"
        )))
    })
}

/// Probe a compressed file.
pub(crate) fn probe(path: &Path) -> Result<CompressedInfo> {
    guarded("reading the header", || probe_unguarded(path))
}

fn probe_unguarded(path: &Path) -> Result<CompressedInfo> {
    let format = open_format(path)?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| Error::Unsupported("no audio track found".into()))?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| Error::Unsupported("audio track has no codec parameters".into()))?;
    Ok(CompressedInfo {
        codec: format!("{:?}", params.codec),
        channels: params
            .channels
            .as_ref()
            .map(|c| channel_count(c.count()))
            .transpose()?
            .filter(|&c| c > 0),
        sample_rate: params.sample_rate.filter(|&r| r > 0),
        frames: track.num_frames,
    })
}

/// Decode a compressed file into memory. `should_stop` is checked once per packet;
/// when it returns true, decoding ends with [`Error::Cancelled`].
pub(crate) fn decode(
    path: &Path,
    selection: Option<ChannelSelection>,
    scaling: Scaling,
    should_stop: &dyn Fn() -> bool,
) -> Result<Decoded> {
    guarded("decoding", || {
        decode_unguarded(path, selection, scaling, should_stop)
    })
}

/// Samples of the selected channel(s), decoded so far.
enum Out {
    Real(Vec<f32>),
    Complex(Vec<Complex32>),
}

/// The decoded output and the channel(s) it is taken from.
struct Sink {
    selection: ChannelSelection,
    out: Out,
}

impl Sink {
    /// Start collecting `selection` (or the default) from a file with `channels`.
    fn new(
        selection: Option<ChannelSelection>,
        channels: u16,
        expected_frames: Option<u64>,
    ) -> Result<Self> {
        let selection = selection.unwrap_or_else(|| ChannelSelection::default_for(channels));
        check_channel(selection, channels)?;
        let reserve = expected_frames.map_or(Ok(0), |n| check_size(n, selection))?;
        let out = match selection {
            ChannelSelection::Mono(_) => Out::Real(Vec::with_capacity(reserve)),
            ChannelSelection::IQ { .. } => Out::Complex(Vec::with_capacity(reserve)),
        };
        Ok(Self { selection, out })
    }

    fn len(&self) -> u64 {
        match &self.out {
            Out::Real(v) => v.len() as u64,
            Out::Complex(v) => v.len() as u64,
        }
    }

    /// Append `frames` frames of silence.
    fn silence(&mut self, frames: u64) -> Result<()> {
        let total = check_size(self.len().saturating_add(frames), self.selection)?;
        match &mut self.out {
            Out::Real(v) => v.resize(total, 0.0),
            Out::Complex(v) => v.resize(total, Complex32::new(0.0, 0.0)),
        }
        Ok(())
    }

    /// Append the selected channel(s) of one decoded buffer, given as planes.
    fn append(&mut self, planes: &[Vec<f32>], frames: usize, scaling: Scaling) -> Result<()> {
        check_size(self.len().saturating_add(frames as u64), self.selection)?;
        let plane = |c: u16| planes.get(usize::from(c)).map_or(&[][..], Vec::as_slice);
        match (self.selection, &mut self.out) {
            (ChannelSelection::Mono(c), Out::Real(v)) => {
                v.extend(plane(c).iter().map(|&x| scaling.apply(x)));
            }
            (ChannelSelection::IQ { i, q }, Out::Complex(v)) => {
                v.extend(
                    plane(i)
                        .iter()
                        .zip(plane(q))
                        .map(|(&re, &im)| Complex32::new(scaling.apply(re), scaling.apply(im))),
                );
            }
            // `Sink::new` always pairs Mono with Real and IQ with Complex.
            _ => {}
        }
        Ok(())
    }

    fn into_signal(self, rate: f64) -> Result<Signal> {
        Ok(match self.out {
            Out::Real(v) => Signal::real(v, rate)?,
            Out::Complex(v) => Signal::complex(v, rate)?,
        })
    }
}

fn frame_bytes(selection: ChannelSelection) -> u64 {
    match selection {
        ChannelSelection::Mono(_) => 4,
        ChannelSelection::IQ { .. } => 8,
    }
}

/// Error when `frames` frames of `selection` would use more than the memory limit.
fn check_size(frames: u64, selection: ChannelSelection) -> Result<usize> {
    let bytes = frames.saturating_mul(frame_bytes(selection));
    match usize::try_from(frames) {
        Ok(n) if bytes <= MAX_DECODED_BYTES => Ok(n),
        _ => Err(Error::Unsupported(format!(
            "the decoded audio would need more than {} GB of memory; convert the file \
             to WAV, which opens without loading it into memory",
            MAX_DECODED_BYTES >> 30
        ))),
    }
}

/// Frames covered by a packet duration in track time-base units.
fn dur_frames(dur: u64, time_base: Option<TimeBase>, rate: Option<u32>) -> u64 {
    match (time_base, rate) {
        (Some(tb), Some(rate)) => {
            let f = u128::from(dur) * u128::from(tb.numer.get()) * u128::from(rate)
                / u128::from(tb.denom.get());
            u64::try_from(f).unwrap_or(u64::MAX)
        }
        // Without a time base, audio durations are in frames.
        _ => dur,
    }
}

fn decode_unguarded(
    path: &Path,
    selection: Option<ChannelSelection>,
    scaling: Scaling,
    should_stop: &dyn Fn() -> bool,
) -> Result<Decoded> {
    let mut format = open_format(path)?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| Error::Unsupported("no audio track found".into()))?;
    let track_id = track.id;
    let time_base = track.time_base;
    let num_frames = track.num_frames;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| Error::Unsupported("audio track has no codec parameters".into()))?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| sym_err(&e))?;

    let mut rate = params.sample_rate.filter(|&r| r > 0);
    let mut sink: Option<Sink> = None;
    // Silence owed for damaged packets seen before the channel count was known.
    let mut pending_gap: u64 = 0;
    let mut damaged: u64 = 0;
    let mut decoded: u64 = 0;
    let mut planes: Vec<Vec<f32>> = Vec::new();

    if let Some(c) = params
        .channels
        .as_ref()
        .map(symphonia::core::audio::Channels::count)
        .filter(|&c| c > 0)
    {
        sink = Some(Sink::new(selection, channel_count(c)?, num_frames)?);
    }

    loop {
        if should_stop() {
            return Err(Error::Cancelled);
        }
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(e) => return Err(sym_err(&e)),
        };
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                decoded += 1;
                let channels = channel_count(buf.spec().channels().count())?;
                if rate.is_none() {
                    rate = Some(buf.spec().rate()).filter(|&r| r > 0);
                }
                let s = match &mut sink {
                    Some(s) => s,
                    None => sink.insert(Sink::new(selection, channels, num_frames)?),
                };
                check_channel(s.selection, channels)?;
                s.silence(std::mem::take(&mut pending_gap))?;
                buf.copy_to_vecs_planar(&mut planes);
                s.append(&planes, buf.frames(), scaling)?;
            }
            // A corrupt packet: keep the time axis intact by putting silence in its
            // place, and tell the user.
            Err(SymError::DecodeError(_)) => {
                damaged += 1;
                let gap = dur_frames(packet.dur.get(), time_base, rate);
                match &mut sink {
                    Some(s) => s.silence(gap)?,
                    None => pending_gap = pending_gap.saturating_add(gap),
                }
            }
            Err(e) => return Err(sym_err(&e)),
        }
    }
    if decoded == 0 && damaged > 0 {
        return Err(Error::Malformed(
            "every audio packet in the file is damaged; nothing could be decoded".into(),
        ));
    }
    let (Some(sink), Some(rate)) = (sink, rate) else {
        return Err(Error::Unsupported(
            "decoder reported no channels or no sample rate".into(),
        ));
    };
    let signal = sink.into_signal(f64::from(rate))?;
    let mut notes = Vec::new();
    if damaged > 0 {
        notes.push(format!(
            "{damaged} damaged audio packet(s) were replaced with silence so that the \
             timing stays correct."
        ));
    }
    Ok(Decoded { signal, notes })
}

#[cfg(test)]
mod guard_tests {
    use super::*;
    fn never() -> bool {
        false
    }

    /// Stereo 16-bit PCM written by our own WAV writer (also checks that Symphonia
    /// accepts our headers).
    fn stereo_pcm(dir: &Path) -> std::path::PathBuf {
        let p = dir.join("s.wav");
        let iq = vec![
            Complex32::new(0.5, -0.5),
            Complex32::new(0.25, 0.0),
            Complex32::new(0.0, 1.0),
        ];
        let s = Signal::complex(iq, 8000.0).unwrap();
        crate::write_wav(&p, &s, crate::WavFormat::Pcm16).unwrap();
        p
    }

    #[test]
    fn decodes_mono_iq_and_scaling_through_symphonia() {
        let dir = tempfile::tempdir().unwrap();
        let p = stereo_pcm(dir.path());

        let d = decode(&p, None, Scaling::default(), &never).unwrap();
        assert_eq!(d.notes, Vec::<String>::new());
        let s = d.signal;
        assert_eq!((s.len(), s.sample_rate()), (3, 8000.0));
        let mut c = [Complex32::new(0.0, 0.0); 3];
        assert_eq!(s.read_complex(0, &mut c), 3);
        assert!((c[0].re - 0.5).abs() < 1e-4 && (c[0].im + 0.5).abs() < 1e-4);

        let scaling = Scaling {
            gain: 2.0,
            clip: None,
        };
        let s = decode(&p, Some(ChannelSelection::Mono(1)), scaling, &never)
            .unwrap()
            .signal;
        let mut r = [0.0_f32; 3];
        s.read_real(0, &mut r);
        assert!((r[0] + 1.0).abs() < 1e-4 && r[1].abs() < 1e-6, "{r:?}");

        let err = decode(&p, Some(ChannelSelection::Mono(u16::MAX)), scaling, &never)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("channel 65536") && err.contains("between 1 and 2"),
            "{err}"
        );
    }

    #[test]
    fn decoding_can_be_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let p = stereo_pcm(dir.path());
        let err = decode(&p, None, Scaling::default(), &|| true).unwrap_err();
        assert!(matches!(err, Error::Cancelled));
    }

    /// Mono IMA ADPCM, three 1024-byte blocks of 2041 frames, each held at 0.25.
    /// `bad` gets an invalid step index, which the decoder rejects.
    fn ima_adpcm(bad: Option<usize>) -> Vec<u8> {
        const BLOCK: usize = 1024;
        let frames_per_block: u16 = 2041;
        let mut data = Vec::new();
        for b in 0..3 {
            data.extend_from_slice(&8192_i16.to_le_bytes());
            data.push(if Some(b) == bad { 200 } else { 0 });
            data.push(0);
            data.resize(data.len() + BLOCK - 4, 0);
        }
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&0x11_u16.to_le_bytes());
        fmt.extend_from_slice(&1_u16.to_le_bytes());
        fmt.extend_from_slice(&8000_u32.to_le_bytes());
        fmt.extend_from_slice(&4055_u32.to_le_bytes());
        fmt.extend_from_slice(&(BLOCK as u16).to_le_bytes());
        fmt.extend_from_slice(&4_u16.to_le_bytes());
        fmt.extend_from_slice(&2_u16.to_le_bytes());
        fmt.extend_from_slice(&frames_per_block.to_le_bytes());
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&((4 + 8 + fmt.len() + 8 + data.len()) as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        out.extend_from_slice(&fmt);
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn damaged_packets_become_silence_of_the_same_length() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good.wav");
        std::fs::write(&good, ima_adpcm(None)).unwrap();
        let d = decode(&good, None, Scaling::default(), &never).unwrap();
        assert_eq!(d.signal.len(), 3 * 2041);
        assert_eq!(d.notes, Vec::<String>::new());

        let p = dir.path().join("bad.wav");
        std::fs::write(&p, ima_adpcm(Some(1))).unwrap();
        let opened = crate::open_cancellable(&p, &crate::LoadOptions::default(), &never).unwrap();
        let s = opened.signal;
        assert_eq!(s.len(), 3 * 2041, "the time axis must not shrink");
        assert_eq!(opened.notes.len(), 1, "{:?}", opened.notes);
        assert!(opened.notes[0].starts_with("1 damaged"));
        let mut r = vec![0.0_f32; s.len()];
        s.read_real(0, &mut r);
        assert!(r[..2041].iter().all(|v| (v - 0.25).abs() < 1e-3));
        assert!(r[2041..4082].iter().all(|&v| v == 0.0));
        assert!(r[4082..].iter().all(|v| (v - 0.25).abs() < 1e-3));
    }

    #[test]
    fn a_file_of_only_damaged_packets_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = ima_adpcm(Some(0));
        // Damage the other two blocks too.
        let data_at = bytes.windows(4).position(|w| w == b"data").unwrap() + 8;
        bytes[data_at + 1024 + 2] = 200;
        bytes[data_at + 2048 + 2] = 200;
        let p = dir.path().join("all_bad.wav");
        std::fs::write(&p, bytes).unwrap();
        assert!(decode(&p, None, Scaling::default(), &never).is_err());
    }

    #[test]
    fn size_limit_and_gap_conversion() {
        assert!(check_size(1000, ChannelSelection::Mono(0)).is_ok());
        assert!(check_size(u64::MAX / 2, ChannelSelection::IQ { i: 0, q: 1 }).is_err());
        assert!(
            check_size(
                (MAX_DECODED_BYTES / 8) + 1,
                ChannelSelection::IQ { i: 0, q: 1 }
            )
            .is_err()
        );
        let tb = TimeBase::new(
            std::num::NonZero::new(1).unwrap(),
            std::num::NonZero::new(1000).unwrap(),
        );
        // 20 ms at 48 kHz is 960 frames.
        assert_eq!(dur_frames(20, Some(tb), Some(48_000)), 960);
        assert_eq!(dur_frames(960, None, Some(48_000)), 960);
    }

    #[test]
    fn a_panic_in_decoder_code_becomes_an_error() {
        let r: Result<()> = guarded("testing", || panic!("decoder bug"));
        assert!(matches!(r, Err(Error::Malformed(m)) if m.contains("testing")));
        assert_eq!(guarded("testing", || Ok(3)).unwrap(), 3);
    }
}
