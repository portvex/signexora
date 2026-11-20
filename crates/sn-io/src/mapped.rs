//! Memory-mapped sample source: the file stays on disk and is decoded as it is read,
//! so recordings of any size open instantly (SA capped files at about 150 MB).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

use memmap2::Mmap;
use sn_core::{Complex32, SampleKind, SampleSource};

use crate::format::{ChannelSelection, DataLayout, Endian, SampleFormat, Scaling};
use crate::{Error, Result, io_err};

/// A [`SampleSource`] over a memory-mapped file.
pub(crate) struct MappedSource {
    map: Arc<Mmap>,
    layout: DataLayout,
    selection: ChannelSelection,
    scaling: Scaling,
    frames: usize,
    description: String,
}

impl std::fmt::Debug for MappedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedSource")
            .field("layout", &self.layout)
            .field("selection", &self.selection)
            .field("scaling", &self.scaling)
            .field("frames", &self.frames)
            .finish_non_exhaustive()
    }
}

/// Files mapped by this process, so that writers can refuse to replace one that a
/// signal still reads from. Entries whose map has been dropped are ignored.
static OPEN_MAPS: Mutex<Vec<(PathBuf, Weak<Mmap>)>> = Mutex::new(Vec::new());

fn open_maps() -> std::sync::MutexGuard<'static, Vec<(PathBuf, Weak<Mmap>)>> {
    // The list stays consistent even if a holder panicked, so a poisoned lock is fine.
    OPEN_MAPS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// True when `path` is a file that a live memory map in this process reads from.
pub(crate) fn is_mapped(path: &Path) -> bool {
    let Ok(target) = std::fs::canonicalize(path) else {
        return false; // a file that does not exist cannot be mapped
    };
    let mut maps = open_maps();
    maps.retain(|(_, m)| m.strong_count() > 0);
    maps.iter().any(|(p, _)| *p == target)
}

/// Map a file read-only.
pub(crate) fn map_file(path: &Path) -> Result<Arc<Mmap>> {
    let file = File::open(path).map_err(io_err(path))?;
    // SAFETY: the map is read-only and we never create a mutable view. If another
    // process truncates or rewrites the file while it is open, reads may see changed
    // data or fault; this is the documented trade-off of memory-mapping and the same
    // behaviour as every mmap-based viewer. Analysis results never depend on the map
    // staying valid after the user closes the file. Our own writers never truncate a
    // file in place: they write a temporary file and rename it, and they refuse to
    // replace a file that is registered here (see `is_mapped`).
    #[allow(unsafe_code)]
    let map = Arc::new(unsafe { Mmap::map(&file) }.map_err(io_err(path))?);
    if let Ok(canonical) = std::fs::canonicalize(path) {
        let mut maps = open_maps();
        maps.retain(|(_, m)| m.strong_count() > 0);
        maps.push((canonical, Arc::downgrade(&map)));
    }
    Ok(map)
}

/// Error unless every channel in `selection` exists in a file with `channels`.
pub(crate) fn check_channel(selection: ChannelSelection, channels: u16) -> Result<()> {
    if selection.max_channel() >= channels {
        return Err(Error::Unsupported(format!(
            "channel {} requested but the file has {channels} channel(s); choose a \
             channel between 1 and {channels}",
            u32::from(selection.max_channel()) + 1,
        )));
    }
    Ok(())
}

impl MappedSource {
    pub(crate) fn new(
        map: Arc<Mmap>,
        mut layout: DataLayout,
        selection: ChannelSelection,
        scaling: Scaling,
        description: String,
    ) -> Result<Self> {
        check_channel(selection, layout.channels)?;
        // Headers written by crashed or streaming recorders often claim more data than
        // the file holds; trust the file length.
        let available = (map.len() as u64).saturating_sub(layout.data_offset);
        layout.data_len = layout.data_len.min(available);
        let frames = usize::try_from(layout.frames())
            .map_err(|_| Error::Unsupported("file is too large for this platform".into()))?;
        Ok(Self {
            map,
            layout,
            selection,
            scaling,
            frames,
            description,
        })
    }

    /// Decode channel `ch` for frames `start..start + n` into the first `n` elements
    /// of `out`, calling `put(&mut out[k], value)` for each. Returns how many frames
    /// were decoded (at most `min(n, out.len())`). This is less than that only if the
    /// header and the mapped length disagree in a way not caught in
    /// [`MappedSource::new`]; a file truncated by another program while mapped faults
    /// instead (see the SAFETY note in [`map_file`]).
    #[inline]
    fn read_channel<T>(
        &self,
        start: usize,
        n: usize,
        ch: u16,
        out: &mut [T],
        put: impl Fn(&mut T, f32),
    ) -> usize {
        let fmt = self.layout.format;
        let endian = self.layout.endian;
        let bytes = fmt.bytes();
        let frame = self.layout.frame_bytes();
        let Ok(data_offset) = usize::try_from(self.layout.data_offset) else {
            return 0;
        };
        let base = data_offset.saturating_add(usize::from(ch).saturating_mul(bytes));
        let first = base.saturating_add(start.saturating_mul(frame));
        let span = self.map.get(first..).unwrap_or_default();
        // The common formats get a loop of their own, with the format and byte order
        // fixed, so the decoding is inlined; the values are those of `decode`.
        match (fmt, endian) {
            (SampleFormat::I16, Endian::Little) => self.decode_run(span, n, out, &put, |b| {
                SampleFormat::I16.decode(b, Endian::Little)
            }),
            (SampleFormat::F32, Endian::Little) => self.decode_run(span, n, out, &put, |b| {
                SampleFormat::F32.decode(b, Endian::Little)
            }),
            (SampleFormat::U8, _) => self.decode_run(span, n, out, &put, |b| {
                SampleFormat::U8.decode(b, Endian::Little)
            }),
            (SampleFormat::I8, _) => self.decode_run(span, n, out, &put, |b| {
                SampleFormat::I8.decode(b, Endian::Little)
            }),
            (SampleFormat::I24, Endian::Little) => self.decode_run(span, n, out, &put, |b| {
                SampleFormat::I24.decode(b, Endian::Little)
            }),
            (SampleFormat::I32, Endian::Little) => self.decode_run(span, n, out, &put, |b| {
                SampleFormat::I32.decode(b, Endian::Little)
            }),
            _ => self.decode_run(span, n, out, &put, |b| fmt.decode(b, endian)),
        }
    }

    /// Decode up to `n` frames from `span` (which starts at the channel's sample in
    /// the first frame) with `decode`, scaled, into the first elements of `out` via
    /// `put`. Returns how many were decoded.
    #[inline]
    fn decode_run<T>(
        &self,
        span: &[u8],
        n: usize,
        out: &mut [T],
        put: &impl Fn(&mut T, f32),
        decode: impl Fn(&[u8]) -> Option<f32>,
    ) -> usize {
        let frame = self.layout.frame_bytes().max(1);
        let identity = self.scaling.is_identity();
        let mut done = 0;
        for (o, b) in out.iter_mut().take(n).zip(span.chunks(frame)) {
            let Some(v) = decode(b) else {
                return done;
            };
            put(o, if identity { v } else { self.scaling.apply(v) });
            done += 1;
        }
        done
    }

    fn clamp_count(&self, start: usize, want: usize) -> usize {
        self.frames.saturating_sub(start).min(want)
    }
}

impl SampleSource for MappedSource {
    fn len(&self) -> usize {
        self.frames
    }

    fn kind(&self) -> SampleKind {
        match self.selection {
            ChannelSelection::Mono(_) => SampleKind::Real,
            ChannelSelection::IQ { .. } => SampleKind::Complex,
        }
    }

    fn read_complex(&self, start: usize, out: &mut [Complex32]) -> usize {
        let n = self.clamp_count(start, out.len());
        match self.selection {
            ChannelSelection::Mono(c) => {
                self.read_channel(start, n, c, out, |o, v| *o = Complex32::new(v, 0.0))
            }
            ChannelSelection::IQ { i, q } => {
                let got_i = self.read_channel(start, n, i, out, |o, v| o.re = v);
                let got_q = self.read_channel(start, n, q, out, |o, v| o.im = v);
                got_i.min(got_q)
            }
        }
    }

    fn read_real(&self, start: usize, out: &mut [f32]) -> usize {
        let n = self.clamp_count(start, out.len());
        let ch = match self.selection {
            ChannelSelection::Mono(c) => c,
            ChannelSelection::IQ { i, .. } => i,
        };
        self.read_channel(start, n, ch, out, |o, v| *o = v)
    }

    fn describe(&self) -> String {
        self.description.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMATS: [SampleFormat; 9] = [
        SampleFormat::U8,
        SampleFormat::I8,
        SampleFormat::I16,
        SampleFormat::I24,
        SampleFormat::I32,
        SampleFormat::F32,
        SampleFormat::F64,
        SampleFormat::ALaw,
        SampleFormat::MuLaw,
    ];

    /// Header bytes before the samples, chosen so no sample is aligned.
    const OFFSET: usize = 7;

    /// Deterministic bytes that cover the whole range of every format.
    fn bytes(n: usize) -> Vec<u8> {
        let mut x: u32 = 0x1234_5678;
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x.to_le_bytes()[0]
            })
            .collect()
    }

    fn selections(channels: u16) -> Vec<ChannelSelection> {
        match channels {
            1 => vec![ChannelSelection::Mono(0)],
            2 => vec![
                ChannelSelection::IQ { i: 0, q: 1 },
                ChannelSelection::IQ { i: 1, q: 0 },
                ChannelSelection::Mono(1),
            ],
            _ => vec![
                ChannelSelection::IQ { i: 2, q: 0 },
                ChannelSelection::Mono(1),
                ChannelSelection::Mono(2),
            ],
        }
    }

    /// The value `fmt.decode` gives for channel `ch` of frame `f`, scaled.
    fn expected(file: &[u8], layout: &DataLayout, scaling: Scaling, f: usize, ch: u16) -> f32 {
        let b = layout.format.bytes();
        let at = OFFSET + f * layout.frame_bytes() + usize::from(ch) * b;
        let v = layout
            .format
            .decode(&file[at..at + b], layout.endian)
            .unwrap();
        if scaling.is_identity() {
            v
        } else {
            scaling.apply(v)
        }
    }

    fn same(a: f32, b: f32) -> bool {
        a.to_bits() == b.to_bits()
    }

    /// Every format, byte order, channel count and selection, read at the start, one
    /// frame in, the last frame and with a buffer longer than what remains, matches
    /// `SampleFormat::decode` applied to the right bytes. This covers both the fast
    /// paths and the generic fallback.
    #[test]
    fn reads_match_generic_decode_at_every_offset() {
        let dir = tempfile::tempdir().unwrap();
        let frames = 5;
        let scalings = [
            Scaling::default(),
            Scaling {
                gain: 0.5,
                clip: Some(0.25),
            },
        ];
        for fmt in FORMATS {
            for endian in [Endian::Little, Endian::Big] {
                for channels in 1..=3u16 {
                    let frame = fmt.bytes() * usize::from(channels);
                    // A partial trailing frame that must not be counted.
                    let file = bytes(OFFSET + frames * frame + frame / 2);
                    let path = dir
                        .path()
                        .join(format!("{fmt:?}-{endian:?}-{channels}.raw"));
                    std::fs::write(&path, &file).unwrap();
                    let layout = DataLayout {
                        format: fmt,
                        endian,
                        channels,
                        data_offset: OFFSET as u64,
                        data_len: u64::MAX,
                    };
                    for sel in selections(channels) {
                        for scaling in scalings {
                            let map = map_file(&path).unwrap();
                            let src = MappedSource::new(map, layout, sel, scaling, String::new())
                                .unwrap();
                            let ctx =
                                format!("{fmt:?} {endian:?} {channels}ch {sel:?} {scaling:?}");
                            assert_eq!(src.len(), frames, "{ctx}");
                            let (ch_i, ch_q) = match sel {
                                ChannelSelection::Mono(c) => (c, None),
                                ChannelSelection::IQ { i, q } => (i, Some(q)),
                            };
                            for (start, want) in
                                [(0, 2), (1, 3), (frames - 1, 1), (2, 10), (frames, 4)]
                            {
                                let left = frames.saturating_sub(start).min(want);
                                let mut real = vec![f32::NAN; want];
                                assert_eq!(src.read_real(start, &mut real), left, "{ctx} @{start}");
                                let mut cx = vec![Complex32::new(f32::NAN, f32::NAN); want];
                                assert_eq!(
                                    src.read_complex(start, &mut cx),
                                    left,
                                    "{ctx} @{start}"
                                );
                                for k in 0..left {
                                    let i = expected(&file, &layout, scaling, start + k, ch_i);
                                    let q = ch_q.map_or(0.0, |q| {
                                        expected(&file, &layout, scaling, start + k, q)
                                    });
                                    assert!(same(real[k], i), "{ctx} real @{}", start + k);
                                    assert!(same(cx[k].re, i), "{ctx} re @{}", start + k);
                                    assert!(same(cx[k].im, q), "{ctx} im @{}", start + k);
                                }
                                // Elements past the decoded count are left untouched.
                                assert!(real[left..].iter().all(|v| v.is_nan()), "{ctx}");
                            }
                        }
                    }
                }
            }
        }
    }

    /// When the frame count claims more than the map holds, reads stop at the last
    /// sample the map really contains, per channel.
    #[test]
    fn short_map_returns_decoded_count() {
        let dir = tempfile::tempdir().unwrap();
        // I16 stereo: 3 whole frames and the left sample of a fourth.
        let file = bytes(OFFSET + 3 * 4 + 2);
        let path = dir.path().join("short.raw");
        std::fs::write(&path, &file).unwrap();
        let layout = DataLayout {
            format: SampleFormat::I16,
            endian: Endian::Little,
            channels: 2,
            data_offset: OFFSET as u64,
            data_len: 16,
        };
        let src = |selection| MappedSource {
            map: map_file(&path).unwrap(),
            layout,
            selection,
            scaling: Scaling::default(),
            frames: 4, // more than the map holds, bypassing the clamp in `new`
            description: String::new(),
        };
        let mut real = [0.0f32; 4];
        assert_eq!(src(ChannelSelection::Mono(0)).read_real(0, &mut real), 4);
        assert_eq!(src(ChannelSelection::Mono(1)).read_real(0, &mut real), 3);
        assert_eq!(src(ChannelSelection::Mono(1)).read_real(3, &mut real), 0);
        let mut cx = [Complex32::new(0.0, 0.0); 4];
        let iq = src(ChannelSelection::IQ { i: 0, q: 1 });
        assert_eq!(iq.read_complex(0, &mut cx), 3);
        assert_eq!(iq.read_complex(2, &mut cx), 1);
        let s = Scaling::default();
        assert!(same(cx[0].re, expected(&file, &layout, s, 2, 0)));
        assert!(same(cx[0].im, expected(&file, &layout, s, 2, 1)));
    }
}
