//! What `SA Open file` shows about the selected file (the info panel and the
//! `Ext. Info` bar) and its options (`Bits` and `Channels` lists).

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use sn_io::{ChannelSelection, Endian, RawFormat, SampleFormat};

/// Raw sample format the loader starts from when none is remembered.
pub const DEFAULT_RAW: RawFormat = RawFormat {
    format: SampleFormat::I16,
    endian: Endian::Little,
    channels: 2,
    sample_rate: 48_000.0,
    data_offset: 0,
};

/// The `fmt ` and `data` chunks of a WAV (RIFF, RF64 or BW64) file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavHeader {
    /// SA's `Wave tag` (the format tag; 65534 for `WAVE_FORMAT_EXTENSIBLE`).
    pub tag: u16,
    /// The sub-format tag of an extensible file.
    pub sub_tag: Option<u16>,
    /// Number of channels.
    pub channels: u16,
    /// Sample rate in Hz.
    pub rate: u32,
    /// Bits per sample.
    pub bits: u16,
    /// Bytes of sample data.
    pub data_size: u64,
    /// Byte offset of the sample data (SA's `Data start`).
    pub data_start: u64,
}

/// The extensible format tag.
const EXTENSIBLE: u16 = 0xFFFE;

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at + 2)?;
    Some(u16::from_le_bytes([*s.first()?, *s.get(1)?]))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 4)?;
    let mut a = [0_u8; 4];
    a.copy_from_slice(s);
    Some(u32::from_le_bytes(a))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    let s = b.get(at..at + 8)?;
    let mut a = [0_u8; 8];
    a.copy_from_slice(s);
    Some(u64::from_le_bytes(a))
}

/// Read a WAV header from `r`; `None` when it is not a WAV file or has no `fmt `
/// and `data` chunk.
pub fn parse_wav<R: Read + Seek>(r: &mut R) -> Option<WavHeader> {
    let mut head = [0_u8; 12];
    r.read_exact(&mut head).ok()?;
    let riff = head.get(0..4)?;
    let rf64 = riff == b"RF64" || riff == b"BW64";
    if (riff != b"RIFF" && !rf64) || head.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut pos: u64 = 12;
    let mut fmt: Option<Vec<u8>> = None;
    let mut ds64_data: Option<u64> = None;
    // A bounded walk: a damaged file must not loop.
    for _ in 0..256 {
        let mut ch = [0_u8; 8];
        if r.read_exact(&mut ch).is_err() {
            return None;
        }
        let id = ch.get(0..4)?.to_vec();
        let size = u64::from(u32_at(&ch, 4)?);
        let body = pos + 8;
        match id.as_slice() {
            b"fmt " => {
                let mut b = vec![0_u8; usize::try_from(size.min(256)).ok()?];
                r.read_exact(&mut b).ok()?;
                fmt = Some(b);
            }
            b"ds64" => {
                let mut b = [0_u8; 28];
                r.read_exact(&mut b).ok()?;
                ds64_data = u64_at(&b, 8);
            }
            b"data" => {
                let f = fmt?;
                let data_size = if rf64 && size == u64::from(u32::MAX) {
                    ds64_data.unwrap_or(size)
                } else {
                    size
                };
                let tag = u16_at(&f, 0)?;
                let sub_tag = if tag == EXTENSIBLE {
                    u16_at(&f, 24)
                } else {
                    None
                };
                return Some(WavHeader {
                    tag,
                    sub_tag,
                    channels: u16_at(&f, 2)?,
                    rate: u32_at(&f, 4)?,
                    bits: u16_at(&f, 14)?,
                    data_size,
                    data_start: body,
                });
            }
            _ => {}
        }
        // Chunks are padded to an even size.
        pos = body.checked_add(size)?.checked_add(size % 2)?;
        r.seek(SeekFrom::Start(pos)).ok()?;
    }
    None
}

/// The WAV header of the file at `path`, if it is a WAV file.
pub fn read_wav_header(path: &Path) -> Option<WavHeader> {
    let file = std::fs::File::open(path).ok()?;
    parse_wav(&mut std::io::BufReader::new(file))
}

/// The codec name of a format tag (SA 6.2.6.4's `Ext. Info`), as Windows names it.
pub fn codec_name(tag: u16, sub_tag: Option<u16>) -> String {
    let name = |t: u16| -> &'static str {
        match t {
            0x0001 => "PCM",
            0x0002 => "Microsoft ADPCM",
            0x0003 => "IEEE Float",
            0x0006 => "CCITT A-Law",
            0x0007 => "CCITT u-Law",
            0x0011 => "Intel DVI ADPCM (IMA ADPCM)",
            0x0022 => "DSP Group TrueSpeech",
            0x0031 => "GSM 6.10",
            0x0040 => "G.721 ADPCM",
            0x0050 => "MPEG Layer-1/2",
            0x0055 => "MPEG Layer-3",
            0x0092 => "Dolby AC3 SPDIF",
            _ => "",
        }
    };
    match (tag, sub_tag) {
        (EXTENSIBLE, Some(s)) if !name(s).is_empty() => format!("Extensible ({})", name(s)),
        (EXTENSIBLE, _) => "Extensible".to_owned(),
        (t, _) if !name(t).is_empty() => name(t).to_owned(),
        (t, _) => format!("Unknown (0x{t:04X})"),
    }
}

/// SA's standard file (info in white): Wave tag 1, 16 bits, mono.
pub fn is_standard(h: &WavHeader) -> bool {
    h.tag == 1 && h.bits == 16 && h.channels == 1
}

/// The `Bits` list: SA's entries in its order, then SigNexora's additions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bits {
    /// `8`: unsigned 8-bit, as in WAV.
    U8,
    /// `8-S`: signed 8-bit (6.2.5.8; SA's label is not documented).
    S8,
    /// `16`: signed 16-bit.
    I16,
    /// `24-A`: signed 24-bit, little-endian.
    I24A,
    /// `24-B`: signed 24-bit, big-endian.
    I24B,
    /// `32-A`: signed 32-bit, little-endian.
    I32A,
    /// `32-B`: signed 32-bit, big-endian.
    I32B,
    /// `32-F`: 32-bit float.
    F32,
    /// `64-F`: 64-bit float.
    F64,
    /// SigNexora: big-endian 16-bit.
    I16B,
    /// SigNexora: big-endian floats.
    F32B,
    /// SigNexora: big-endian 64-bit floats.
    F64B,
    /// SigNexora: G.711.
    ALaw,
    /// SigNexora: G.711 mu-law.
    MuLaw,
}

impl Bits {
    /// Every choice, in list order.
    pub const ALL: [Bits; 14] = [
        Self::U8,
        Self::S8,
        Self::I16,
        Self::I24A,
        Self::I24B,
        Self::I32A,
        Self::I32B,
        Self::F32,
        Self::F64,
        Self::I16B,
        Self::F32B,
        Self::F64B,
        Self::ALaw,
        Self::MuLaw,
    ];

    /// SA's label (`16`, `24-A`, `32-F`, ...).
    pub fn label(self) -> &'static str {
        match self {
            Self::U8 => "8",
            Self::S8 => "8-S",
            Self::I16 => "16",
            Self::I24A => "24-A",
            Self::I24B => "24-B",
            Self::I32A => "32-A",
            Self::I32B => "32-B",
            Self::F32 => "32-F",
            Self::F64 => "64-F",
            Self::I16B => "16-B",
            Self::F32B => "32-FB",
            Self::F64B => "64-FB",
            Self::ALaw => "A-law",
            Self::MuLaw => "u-law",
        }
    }

    /// The sample format and byte order.
    pub fn format(self) -> (SampleFormat, Endian) {
        use Endian::{Big, Little};
        match self {
            Self::U8 => (SampleFormat::U8, Little),
            Self::S8 => (SampleFormat::I8, Little),
            Self::I16 => (SampleFormat::I16, Little),
            Self::I16B => (SampleFormat::I16, Big),
            Self::I24A => (SampleFormat::I24, Little),
            Self::I24B => (SampleFormat::I24, Big),
            Self::I32A => (SampleFormat::I32, Little),
            Self::I32B => (SampleFormat::I32, Big),
            Self::F32 => (SampleFormat::F32, Little),
            Self::F32B => (SampleFormat::F32, Big),
            Self::F64 => (SampleFormat::F64, Little),
            Self::F64B => (SampleFormat::F64, Big),
            Self::ALaw => (SampleFormat::ALaw, Little),
            Self::MuLaw => (SampleFormat::MuLaw, Little),
        }
    }

    /// The entry for a sample format and byte order. Byte order means nothing
    /// to the 1-byte formats, so every pair has an entry.
    pub fn of(format: SampleFormat, endian: Endian) -> Self {
        let endian = if format.bytes() == 1 {
            Endian::Little
        } else {
            endian
        };
        Self::ALL
            .into_iter()
            .find(|b| b.format() == (format, endian))
            .unwrap_or(Self::I16)
    }

    /// The entry a WAV header suggests (16 when it does not say).
    pub fn from_wav(h: &WavHeader) -> Self {
        let tag = if h.tag == EXTENSIBLE {
            h.sub_tag.unwrap_or(1)
        } else {
            h.tag
        };
        match (tag, h.bits) {
            (3, 32) => Self::F32,
            (3, 64) => Self::F64,
            (6, _) => Self::ALaw,
            (7, _) => Self::MuLaw,
            (_, 8) => Self::U8,
            (_, 24) => Self::I24A,
            (_, 32) => Self::I32A,
            _ => Self::I16,
        }
    }
}

/// SA's `Channels` list entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chan {
    /// `All - I/Q`: channels 1 and 2 as I/Q (or the only channel).
    All,
    /// `Ch - n` (0-based here, shown 1-based).
    One(u16),
    /// SigNexora: channels `n` and `n + 1` (0-based) as I/Q, for files with more than two.
    Pair(u16),
}

/// SA lists the first 16 channels.
const MAX_LISTED_CHANNELS: u16 = 16;

impl Chan {
    /// The text in the Channels list.
    pub fn label(self) -> String {
        match self {
            Self::All => "All - I/Q".to_owned(),
            Self::One(c) => format!("Ch - {}", u32::from(c) + 1),
            Self::Pair(c) => format!("I/Q - {},{}", u32::from(c) + 1, u32::from(c) + 2),
        }
    }

    /// The list for a file of `channels` channels: `All - I/Q`, `Ch - 1` to `Ch - 16`,
    /// then the further channel pairs.
    pub fn list(channels: u16) -> Vec<Chan> {
        let n = channels.clamp(1, MAX_LISTED_CHANNELS);
        let mut v = vec![Self::All];
        v.extend((0..n).map(Self::One));
        v.extend((2..n.saturating_sub(1)).step_by(2).map(Self::Pair));
        v
    }

    /// The channels this entry loads from a file of `channels` channels.
    pub fn selection(self, channels: u16) -> ChannelSelection {
        match self {
            Self::All if channels >= 2 => ChannelSelection::IQ { i: 0, q: 1 },
            Self::One(c) => ChannelSelection::Mono(c.min(channels.saturating_sub(1))),
            Self::Pair(c) if c.saturating_add(1) < channels => {
                ChannelSelection::IQ { i: c, q: c + 1 }
            }
            Self::All | Self::Pair(_) => ChannelSelection::Mono(0),
        }
    }
}

/// The `SA - select channel` choice for a stereo file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StereoChoice {
    /// `1`: the first channel.
    First,
    /// `2`: the second channel.
    Second,
    /// `All samples`: both channels as I/Q.
    #[default]
    All,
}

impl StereoChoice {
    /// The channels to load.
    pub fn selection(self) -> ChannelSelection {
        match self {
            Self::First => ChannelSelection::Mono(0),
            Self::Second => ChannelSelection::Mono(1),
            Self::All => ChannelSelection::IQ { i: 0, q: 1 },
        }
    }
}

/// Parse SA's integer edit fields (`Data start`); `None` for anything else.
pub fn parse_bytes(text: &str) -> Option<u64> {
    text.trim().parse().ok()
}

/// Parse the `Freq rate` field: a positive rate up to 1 GHz (SigNexora accepts a
/// fraction, written with a point or a comma).
pub fn parse_rate(text: &str) -> Option<f64> {
    crate::format::parse_number(text).filter(|r| rate_ok(*r))
}

/// Highest sample rate the loader accepts, Hz.
pub const MAX_RATE_HZ: f64 = 1e9;

/// `r` is a sample rate the loader accepts: above 0 and at most [`MAX_RATE_HZ`].
pub fn rate_ok(r: f64) -> bool {
    r.is_finite() && r > 0.0 && r <= MAX_RATE_HZ
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn one_byte_formats_ignore_the_byte_order() {
        for (f, b) in [
            (SampleFormat::I8, Bits::S8),
            (SampleFormat::U8, Bits::U8),
            (SampleFormat::ALaw, Bits::ALaw),
            (SampleFormat::MuLaw, Bits::MuLaw),
        ] {
            assert_eq!(Bits::of(f, Endian::Big), b);
            assert_eq!(Bits::of(f, Endian::Little), b);
        }
        assert_eq!(Bits::of(SampleFormat::I16, Endian::Big), Bits::I16B);
    }

    fn wav(tag: u16, channels: u16, bits: u16, extra: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&100_u32.to_le_bytes());
        b.extend_from_slice(b"WAVE");
        // An odd-sized chunk before fmt (padded).
        b.extend_from_slice(b"junk");
        b.extend_from_slice(&3_u32.to_le_bytes());
        b.extend_from_slice(&[1, 2, 3, 0]);
        b.extend_from_slice(b"fmt ");
        let ext = if tag == EXTENSIBLE { 40_u32 } else { 16 };
        b.extend_from_slice(&ext.to_le_bytes());
        b.extend_from_slice(&tag.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&11_025_u32.to_le_bytes());
        b.extend_from_slice(&0_u32.to_le_bytes());
        b.extend_from_slice(&0_u16.to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(extra);
        b.extend_from_slice(b"data");
        b.extend_from_slice(&8_u32.to_le_bytes());
        b.extend_from_slice(&[0; 8]);
        b
    }

    #[test]
    fn wav_headers_give_sa_info_values() {
        let b = wav(0x11, 1, 4, &[]);
        let h = parse_wav(&mut Cursor::new(b)).unwrap();
        assert_eq!(h.tag, 17);
        assert_eq!((h.channels, h.rate, h.bits), (1, 11_025, 4));
        assert_eq!(h.data_size, 8);
        assert_eq!(h.data_start, 12 + 12 + 24 + 8);
        assert!(!is_standard(&h));
        assert_eq!(codec_name(h.tag, h.sub_tag), "Intel DVI ADPCM (IMA ADPCM)");
        let pcm = parse_wav(&mut Cursor::new(wav(1, 1, 16, &[]))).unwrap();
        assert!(is_standard(&pcm));
        assert_eq!(Bits::from_wav(&pcm), Bits::I16);
    }

    #[test]
    fn extensible_headers_report_their_sub_format() {
        let mut extra = vec![22, 0, 24, 0, 3, 0, 0, 0];
        extra.extend_from_slice(&3_u16.to_le_bytes());
        extra.extend_from_slice(&[0; 14]);
        let h = parse_wav(&mut Cursor::new(wav(EXTENSIBLE, 2, 32, &extra))).unwrap();
        assert_eq!(h.tag, 65534);
        assert_eq!(h.sub_tag, Some(3));
        assert_eq!(codec_name(h.tag, h.sub_tag), "Extensible (IEEE Float)");
        assert_eq!(Bits::from_wav(&h), Bits::F32);
    }

    #[test]
    fn non_wav_data_is_not_a_header() {
        assert_eq!(parse_wav(&mut Cursor::new(b"ID3 something".to_vec())), None);
        assert_eq!(parse_wav(&mut Cursor::new(Vec::new())), None);
        // RIFF without a data chunk.
        let mut b = wav(1, 1, 16, &[]);
        b.truncate(40);
        assert_eq!(parse_wav(&mut Cursor::new(b)), None);
    }

    #[test]
    fn bits_list_starts_with_sa_entries_in_sa_order() {
        let labels: Vec<&str> = Bits::ALL.iter().map(|b| b.label()).collect();
        assert_eq!(
            labels.get(..9),
            Some(
                &[
                    "8", "8-S", "16", "24-A", "24-B", "32-A", "32-B", "32-F", "64-F"
                ][..]
            )
        );
        for b in Bits::ALL {
            let (f, e) = b.format();
            assert_eq!(Bits::of(f, e), b);
        }
    }

    #[test]
    fn channel_list_follows_sa_and_selects_channels() {
        let labels: Vec<String> = Chan::list(2).iter().map(|c| c.label()).collect();
        assert_eq!(labels, ["All - I/Q", "Ch - 1", "Ch - 2"]);
        let nine = Chan::list(9);
        assert_eq!(nine.len(), 1 + 9 + 3);
        assert_eq!(nine.last().map(|c| c.label()), Some("I/Q - 7,8".to_owned()));
        assert_eq!(Chan::list(40).len(), 1 + 16 + 7);
        assert_eq!(Chan::All.selection(1), ChannelSelection::Mono(0));
        assert_eq!(Chan::All.selection(2), ChannelSelection::IQ { i: 0, q: 1 });
        assert_eq!(Chan::One(5).selection(9), ChannelSelection::Mono(5));
        assert_eq!(Chan::One(5).selection(2), ChannelSelection::Mono(1));
        assert_eq!(
            Chan::Pair(2).selection(9),
            ChannelSelection::IQ { i: 2, q: 3 }
        );
        assert_eq!(
            StereoChoice::default().selection(),
            ChannelSelection::IQ { i: 0, q: 1 }
        );
    }

    #[test]
    fn option_fields_are_validated() {
        assert_eq!(parse_bytes(" 68 "), Some(68));
        assert_eq!(parse_bytes("-1"), None);
        assert_eq!(parse_bytes("x"), None);
        assert_eq!(parse_rate("96000"), Some(96_000.0));
        assert_eq!(parse_rate("7777,77"), Some(7777.77));
        assert_eq!(parse_rate("0"), None);
        assert_eq!(parse_rate(""), None);
    }
}
