//! WAV header parser: RIFF, RF64 and BW64, including `WAVE_FORMAT_EXTENSIBLE`.
//!
//! Hound handles only the common cases; SA analysts get files from every kind of
//! recorder, so we parse the chunk structure ourselves and report the codec by name
//! even when we cannot decode it.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::format::{DataLayout, Endian, SampleFormat};
use crate::{Error, Result, io_err};

/// What the WAV header says about the audio data.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WavHeader {
    pub(crate) container: &'static str,
    pub(crate) format_tag: u16,
    pub(crate) codec: String,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
    pub(crate) bits_per_sample: u16,
    pub(crate) block_align: u16,
    pub(crate) data_offset: u64,
    /// Length of the sample data in bytes, never more than the file holds.
    pub(crate) data_len: u64,
    /// Decodable sample format, or `None` for compressed codecs.
    pub(crate) sample_format: Option<SampleFormat>,
    /// Problems in the header that were worked around, for the user.
    pub(crate) notes: Vec<String>,
}

impl WavHeader {
    pub(crate) fn layout(&self) -> Option<DataLayout> {
        self.sample_format.map(|format| DataLayout {
            format,
            endian: Endian::Little,
            channels: self.channels,
            data_offset: self.data_offset,
            data_len: self.data_len,
        })
    }
}

/// Human-readable name of a WAV format tag.
pub(crate) fn codec_name(tag: u16) -> String {
    match tag {
        0x0001 => "PCM".into(),
        0x0002 => "Microsoft ADPCM".into(),
        0x0003 => "IEEE float".into(),
        0x0006 => "A-law".into(),
        0x0007 => "mu-law".into(),
        0x0011 => "IMA/DVI ADPCM".into(),
        0x0031 => "GSM 6.10".into(),
        0x0050 => "MPEG".into(),
        0x0055 => "MPEG Layer 3 (MP3)".into(),
        0xFFFE => "Extensible".into(),
        other => format!("unknown codec 0x{other:04X}"),
    }
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(o..o + 8)?.try_into().ok()?))
}

/// True when `bytes` starts like a WAV file.
pub(crate) fn looks_like_wav(bytes: &[u8]) -> bool {
    matches!(bytes.get(0..4), Some(b"RIFF" | b"RF64" | b"BW64"))
        && bytes.get(8..12) == Some(b"WAVE".as_slice())
}

fn bad(msg: &str) -> Error {
    Error::Malformed(msg.to_owned())
}

/// The sample format for a decodable format tag stored in `bytes` bytes per sample.
fn format_for_tag(tag: u16, bytes: u32) -> Option<SampleFormat> {
    match (tag, bytes) {
        (0x0001, 1) => Some(SampleFormat::U8),
        (0x0001, 2) => Some(SampleFormat::I16),
        (0x0001, 3) => Some(SampleFormat::I24),
        (0x0001, 4) => Some(SampleFormat::I32),
        (0x0003, 4) => Some(SampleFormat::F32),
        (0x0003, 8) => Some(SampleFormat::F64),
        (0x0006, 1) => Some(SampleFormat::ALaw),
        (0x0007, 1) => Some(SampleFormat::MuLaw),
        _ => None,
    }
}

/// Work out how PCM, float or G.711 samples are stored. Returns `Ok((None, None))`
/// for compressed codecs, whose block align means something else.
///
/// The container size comes from `block_align / channels` and must hold `bits`. When
/// the two disagree (a common writer bug is a block align for one channel only) and
/// the data length is a whole number of `bits`-sized frames, `bits` wins and a note
/// says so. Anything else is rejected rather than decoded as noise.
fn sample_format_for(
    tag: u16,
    bits: u16,
    block_align: u16,
    channels: u16,
    data_len: u64,
) -> Result<(Option<SampleFormat>, Option<String>)> {
    if !matches!(tag, 0x0001 | 0x0003 | 0x0006 | 0x0007) {
        return Ok((None, None));
    }
    let channels32 = u32::from(channels);
    let block32 = u32::from(block_align);
    if channels32 > 0 && block32 % channels32 == 0 {
        let container = block32 / channels32;
        if u32::from(bits) <= container.saturating_mul(8)
            && let Some(f) = format_for_tag(tag, container)
        {
            return Ok((Some(f), None));
        }
    }
    let bytes = u32::from(bits).div_ceil(8);
    let frame = u64::from(bytes) * u64::from(channels);
    if let Some(f) = format_for_tag(tag, bytes)
        && frame > 0
        && data_len.is_multiple_of(frame)
    {
        let note = format!(
            "The header gives a block align of {block_align} bytes, which does not match \
             {bits}-bit samples on {channels} channel(s). The bits per sample were used; \
             check the result."
        );
        return Ok((Some(f), Some(note)));
    }
    Err(bad(&format!(
        "the fmt chunk is inconsistent: {} with {bits} bits per sample, {channels} \
         channel(s) and block align {block_align}",
        codec_name(tag)
    )))
}

/// Fill `buf` from `r`. Returns `false` if the stream ended first.
fn fill<R: Read>(r: &mut R, buf: &mut [u8]) -> std::io::Result<bool> {
    let mut got = 0;
    while let Some(rest) = buf.get_mut(got..) {
        if rest.is_empty() {
            break;
        }
        match r.read(rest) {
            Ok(0) => return Ok(false),
            Ok(n) => got += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(true)
}

/// True when the 8 bytes at `pos` look like the header of a chunk that fits in the
/// file. Used to tell an empty data chunk from one whose size was never written.
fn chunk_follows<R: Read + Seek>(r: &mut R, pos: u64, file_len: u64) -> std::io::Result<bool> {
    r.seek(SeekFrom::Start(pos))?;
    let mut h = [0_u8; 8];
    if !fill(r, &mut h)? {
        return Ok(false);
    }
    let id_ok = h
        .get(0..4)
        .is_some_and(|id| id.iter().all(|b| b.is_ascii_alphanumeric() || *b == b' '));
    let fits = u32_at(&h, 4)
        .is_some_and(|size| pos.saturating_add(8).saturating_add(u64::from(size)) <= file_len);
    Ok(id_ok && fits)
}

/// The length of the data chunk whose body starts at `body`, never more than the
/// file holds. A size of 0 (or 0xFFFFFFFF without a `ds64` value) is what streaming
/// or crashed recorders leave behind; when no chunk follows, the rest of the file is
/// the data.
fn data_len<R: Read + Seek>(
    r: &mut R,
    body: u64,
    size: u64,
    ds64_data_len: Option<u64>,
    file_len: u64,
    notes: &mut Vec<String>,
) -> std::io::Result<u64> {
    let available = file_len.saturating_sub(body);
    let declared = match size {
        0xFFFF_FFFF => ds64_data_len.filter(|&l| l > 0),
        0 => None,
        s => Some(s),
    };
    Ok(match declared {
        Some(l) if l > available => {
            notes.push(
                "The file is shorter than its header says (probably an unfinished \
                 recording); only the data present is used."
                    .to_owned(),
            );
            available
        }
        Some(l) => l,
        None if available > 0 && !chunk_follows(r, body, file_len)? => {
            notes.push(
                "The header gives no data length (probably an unfinished recording); \
                 the rest of the file is used as data."
                    .to_owned(),
            );
            available
        }
        None => 0,
    })
}

/// Parse the header of a WAV file held in `bytes`.
#[cfg(test)]
pub(crate) fn parse(bytes: &[u8]) -> Result<WavHeader> {
    parse_from(&mut std::io::Cursor::new(bytes), Path::new("<memory>"))
}

/// Parse the header of the WAV file at `path`.
pub(crate) fn read_header(path: &Path) -> Result<WavHeader> {
    let mut file = std::fs::File::open(path).map_err(io_err(path))?;
    parse_from(&mut file, path)
}

/// Parse a WAV header by walking the chunk list. Only chunk headers and the small
/// `fmt ` and `ds64` bodies are read; other chunks are skipped with a seek, so large
/// leading metadata chunks (`bext`, `iXML`, `LIST`) cost nothing.
pub(crate) fn parse_from<R: Read + Seek>(r: &mut R, path: &Path) -> Result<WavHeader> {
    let io = io_err(path);
    let file_len = r.seek(SeekFrom::End(0)).map_err(&io)?;
    r.seek(SeekFrom::Start(0)).map_err(&io)?;
    let mut head = [0_u8; 12];
    if !fill(r, &mut head).map_err(&io)? || !looks_like_wav(&head) {
        return Err(bad("not a RIFF/RF64 WAVE file"));
    }
    let container = match head.get(0..4) {
        Some(b"RIFF") => "WAV",
        Some(b"RF64") => "RF64",
        _ => "BW64",
    };
    let mut notes = Vec::new();
    let mut pos = 12_u64;
    let mut ds64_data_len: Option<u64> = None;
    let mut fmt: Option<(u16, u16, u32, u16, u16)> = None;
    let mut data: Option<(u64, u64)> = None;

    while pos < file_len {
        r.seek(SeekFrom::Start(pos)).map_err(&io)?;
        let mut ch = [0_u8; 8];
        if !fill(r, &mut ch).map_err(&io)? {
            break;
        }
        let Some(size) = u32_at(&ch, 4) else {
            break;
        };
        let size = u64::from(size);
        let body = pos.saturating_add(8);
        match ch.get(0..4) {
            Some(b"ds64") => {
                let mut b = [0_u8; 16];
                if fill(r, &mut b).map_err(&io)? {
                    ds64_data_len = u64_at(&b, 8);
                }
            }
            Some(b"fmt ") => {
                // 40 bytes covers WAVE_FORMAT_EXTENSIBLE; the rest is ignored.
                let mut b = vec![0_u8; usize::try_from(size.min(40)).unwrap_or(40)];
                if !fill(r, &mut b).map_err(&io)? {
                    return Err(bad("short fmt chunk"));
                }
                let short = || bad("short fmt chunk");
                let mut tag = u16_at(&b, 0).ok_or_else(short)?;
                let channels = u16_at(&b, 2).ok_or_else(short)?;
                let rate = u32_at(&b, 4).ok_or_else(short)?;
                let block_align = u16_at(&b, 12).ok_or_else(short)?;
                let bits = u16_at(&b, 14).ok_or_else(short)?;
                if tag == 0xFFFE {
                    // WAVE_FORMAT_EXTENSIBLE: the real tag is the first two bytes of the
                    // SubFormat GUID at offset 24 of the fmt body.
                    tag = u16_at(&b, 24).ok_or_else(|| bad("short extensible fmt"))?;
                }
                fmt = Some((tag, channels, rate, block_align, bits));
            }
            Some(b"data") => {
                let len =
                    data_len(r, body, size, ds64_data_len, file_len, &mut notes).map_err(&io)?;
                data = Some((body, len));
                break;
            }
            _ => {}
        }
        // Chunks are padded to an even size.
        pos = body.saturating_add(size + (size & 1));
    }

    let (tag, channels, sample_rate, block_align, bits) =
        fmt.ok_or_else(|| bad("missing fmt chunk"))?;
    let (data_offset, data_len) = data.ok_or_else(|| bad("missing data chunk"))?;
    if channels == 0 || sample_rate == 0 {
        return Err(bad("fmt chunk has zero channels or zero sample rate"));
    }
    let (sample_format, note) = sample_format_for(tag, bits, block_align, channels, data_len)?;
    notes.extend(note);
    Ok(WavHeader {
        container,
        format_tag: tag,
        codec: codec_name(tag),
        channels,
        sample_rate,
        bits_per_sample: bits,
        block_align,
        data_offset,
        data_len,
        sample_format,
        notes,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Build a minimal WAV file in memory.
    pub(crate) fn make_wav(
        magic: [u8; 4],
        tag: u16,
        extensible: bool,
        channels: u16,
        rate: u32,
        bits: u16,
        data: &[u8],
    ) -> Vec<u8> {
        let block_align = channels * bits.div_ceil(8);
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&(if extensible { 0xFFFE } else { tag }).to_le_bytes());
        fmt.extend_from_slice(&channels.to_le_bytes());
        fmt.extend_from_slice(&rate.to_le_bytes());
        fmt.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
        fmt.extend_from_slice(&block_align.to_le_bytes());
        fmt.extend_from_slice(&bits.to_le_bytes());
        if extensible {
            fmt.extend_from_slice(&22_u16.to_le_bytes());
            fmt.extend_from_slice(&bits.to_le_bytes());
            fmt.extend_from_slice(&0_u32.to_le_bytes());
            fmt.extend_from_slice(&tag.to_le_bytes());
            fmt.extend_from_slice(&[0; 14]);
        }
        let mut out = Vec::new();
        out.extend_from_slice(&magic);
        out.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes());
        out.extend_from_slice(b"WAVE");
        if &magic == b"RF64" {
            out.extend_from_slice(b"ds64");
            out.extend_from_slice(&28_u32.to_le_bytes());
            out.extend_from_slice(&0_u64.to_le_bytes());
            out.extend_from_slice(&(data.len() as u64).to_le_bytes());
            out.extend_from_slice(&0_u64.to_le_bytes());
            out.extend_from_slice(&0_u32.to_le_bytes());
        }
        // An odd-sized unknown chunk to exercise padding.
        out.extend_from_slice(b"LIST");
        out.extend_from_slice(&3_u32.to_le_bytes());
        out.extend_from_slice(&[1, 2, 3, 0]);
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        out.extend_from_slice(&fmt);
        out.extend_from_slice(b"data");
        let size = if &magic == b"RF64" {
            0xFFFF_FFFF
        } else {
            data.len() as u32
        };
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn parses_plain_extensible_and_rf64() {
        let data = [0_u8; 12];
        let h = parse(&make_wav(*b"RIFF", 1, false, 2, 48_000, 16, &data)).unwrap();
        assert_eq!(h.sample_format, Some(SampleFormat::I16));
        assert_eq!((h.channels, h.sample_rate, h.data_len), (2, 48_000, 12));

        let h = parse(&make_wav(*b"RIFF", 3, true, 1, 96_000, 32, &data)).unwrap();
        assert_eq!(h.sample_format, Some(SampleFormat::F32));
        assert_eq!(h.codec, "IEEE float");

        let h = parse(&make_wav(*b"RF64", 1, false, 1, 8000, 24, &data)).unwrap();
        assert_eq!(h.container, "RF64");
        assert_eq!(h.data_len, 12);
        assert_eq!(h.sample_format, Some(SampleFormat::I24));
    }

    #[test]
    fn names_codecs_it_cannot_decode() {
        let h = parse(&make_wav(*b"RIFF", 0x11, false, 1, 8000, 4, &[0; 4])).unwrap();
        assert_eq!(h.codec, "IMA/DVI ADPCM");
        assert!(h.sample_format.is_none());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"not a wav file at all").is_err());
        let mut no_data = make_wav(*b"RIFF", 1, false, 1, 8000, 16, &[]);
        no_data.truncate(no_data.len() - 8);
        assert!(parse(&no_data).is_err());
    }

    /// Byte offset of the `fmt ` chunk body in a file from [`make_wav`].
    fn fmt_body(wav: &[u8]) -> usize {
        wav.windows(4).position(|w| w == b"fmt ").unwrap() + 8
    }

    fn set_block_align(wav: &mut [u8], block_align: u16) {
        let o = fmt_body(wav) + 12;
        wav[o..o + 2].copy_from_slice(&block_align.to_le_bytes());
    }

    #[test]
    fn block_align_for_one_channel_falls_back_to_bits() {
        // Stereo 16-bit with block align 2 (a writer bug): must stay 16-bit, 3 frames.
        let mut wav = make_wav(*b"RIFF", 1, false, 2, 8000, 16, &[0; 12]);
        set_block_align(&mut wav, 2);
        let h = parse(&wav).unwrap();
        assert_eq!(h.sample_format, Some(SampleFormat::I16));
        assert_eq!(h.layout().unwrap().frames(), 3);
        assert_eq!(h.notes.len(), 1, "{:?}", h.notes);

        // Same header, but the data is not a whole number of 16-bit frames.
        let mut wav = make_wav(*b"RIFF", 1, false, 2, 8000, 16, &[0; 10]);
        set_block_align(&mut wav, 2);
        assert!(matches!(parse(&wav), Err(Error::Malformed(_))));
    }

    #[test]
    fn block_align_not_divisible_by_channels_is_rejected_or_corrected() {
        // Block align 5 with 2 channels used to give a 4-byte stride for I16.
        let mut wav = make_wav(*b"RIFF", 1, false, 2, 8000, 16, &[0; 8]);
        set_block_align(&mut wav, 5);
        let h = parse(&wav).unwrap();
        assert_eq!(h.sample_format, Some(SampleFormat::I16));
        assert_eq!(h.notes.len(), 1);

        // Bits that do not fit the container and no usable fallback.
        let mut wav = make_wav(*b"RIFF", 1, false, 2, 8000, 16, &[0; 8]);
        set_block_align(&mut wav, 5);
        let o = fmt_body(&wav) + 14;
        wav[o..o + 2].copy_from_slice(&40_u16.to_le_bytes());
        assert!(matches!(parse(&wav), Err(Error::Malformed(_))));
    }

    #[test]
    fn zero_data_size_uses_the_file_length() {
        let mut wav = make_wav(*b"RIFF", 1, false, 1, 8000, 16, &[0; 20]);
        let size_at = wav.windows(4).position(|w| w == b"data").unwrap() + 4;
        wav[size_at..size_at + 4].copy_from_slice(&0_u32.to_le_bytes());
        let h = parse(&wav).unwrap();
        assert_eq!(h.data_len, 20);
        assert_eq!(h.notes.len(), 1, "{:?}", h.notes);

        // A genuinely empty data chunk followed by another chunk stays empty.
        let mut wav = make_wav(*b"RIFF", 1, false, 1, 8000, 16, &[]);
        wav.extend_from_slice(b"LIST");
        wav.extend_from_slice(&4_u32.to_le_bytes());
        wav.extend_from_slice(b"INFO");
        let h = parse(&wav).unwrap();
        assert_eq!(h.data_len, 0);
        assert_eq!(h.notes, Vec::<String>::new());
    }

    #[test]
    fn data_after_a_large_leading_chunk_is_found() {
        let base = make_wav(*b"RIFF", 1, false, 1, 8000, 16, &[1, 0, 2, 0]);
        let mut wav = base[..12].to_vec();
        let big = 3 << 20;
        wav.extend_from_slice(b"bext");
        wav.extend_from_slice(&(big as u32).to_le_bytes());
        wav.resize(wav.len() + big, 0);
        wav.extend_from_slice(&base[12..]);
        let h = parse(&wav).unwrap();
        assert_eq!(h.data_len, 4);
        assert_eq!(h.data_offset as usize, wav.len() - 4);
    }
}
