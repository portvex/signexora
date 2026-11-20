use std::fmt;
use std::sync::Arc;

use num_complex::Complex32;
use rayon::prelude::*;

use crate::{Error, Result};

/// Whether a signal holds real or complex (I/Q) samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SampleKind {
    /// Real-valued samples, e.g. receiver audio or IF.
    Real,
    /// Complex samples, e.g. I/Q from an SDR.
    Complex,
}

/// What the sample values mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SampleUnit {
    /// Amplitude relative to full scale (recordings and most derived signals).
    #[default]
    FullScale,
    /// Instantaneous frequency in Hz (phase-detector output; SA drew these in green).
    Hertz,
}

/// Samples that are produced on demand instead of held in memory, such as a
/// memory-mapped recording that is decoded as it is read.
///
/// Implementations must be cheap to call repeatedly for small, possibly overlapping
/// ranges, and safe to call from several threads at once.
pub trait SampleSource: Send + Sync + fmt::Debug {
    /// Number of samples (complex samples count once).
    fn len(&self) -> usize;

    /// True when the source has no samples.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Real or complex.
    fn kind(&self) -> SampleKind;

    /// Fill `out` with complex samples starting at `start`. Real sources set the
    /// imaginary part to zero. Returns how many samples were written, which is less
    /// than `out.len()` only at the end of the data.
    fn read_complex(&self, start: usize, out: &mut [Complex32]) -> usize;

    /// Fill `out` with real samples starting at `start`. Complex sources return the
    /// real (I) part. Returns how many samples were written.
    fn read_real(&self, start: usize, out: &mut [f32]) -> usize;

    /// Short description of where the samples come from, for the inspector.
    fn describe(&self) -> String {
        "external source".to_owned()
    }
}

/// Sample storage. In-memory buffers are reference-counted so derived views and the UI
/// can share them without copying; large files stay on disk behind [`SampleSource`].
#[derive(Debug, Clone)]
pub enum Samples {
    /// Real-valued samples in memory.
    Real(Arc<[f32]>),
    /// Complex (I/Q) samples in memory.
    Complex(Arc<[Complex32]>),
    /// Samples read on demand (for example a memory-mapped file).
    Lazy(Arc<dyn SampleSource>),
}

impl Samples {
    /// Number of samples (complex samples count once).
    pub fn len(&self) -> usize {
        match self {
            Samples::Real(s) => s.len(),
            Samples::Complex(s) => s.len(),
            Samples::Lazy(s) => s.len(),
        }
    }

    /// True when there are no samples.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Real or complex.
    pub fn kind(&self) -> SampleKind {
        match self {
            Samples::Real(_) => SampleKind::Real,
            Samples::Complex(_) => SampleKind::Complex,
            Samples::Lazy(s) => s.kind(),
        }
    }
}

/// An immutable block of samples with its timing metadata.
///
/// The sample rate is an `f64` so fractional rates (needed for OFDM native-rate work)
/// are represented exactly enough.
#[derive(Debug, Clone)]
pub struct Signal {
    samples: Samples,
    sample_rate: f64,
    center_frequency: f64,
    unit: SampleUnit,
}

impl Signal {
    /// Create a real-valued signal.
    pub fn real(samples: impl Into<Arc<[f32]>>, sample_rate: f64) -> Result<Self> {
        Self::new(Samples::Real(samples.into()), sample_rate)
    }

    /// Create a complex (I/Q) signal.
    pub fn complex(samples: impl Into<Arc<[Complex32]>>, sample_rate: f64) -> Result<Self> {
        Self::new(Samples::Complex(samples.into()), sample_rate)
    }

    /// Create a signal whose samples are read on demand.
    pub fn lazy(source: Arc<dyn SampleSource>, sample_rate: f64) -> Result<Self> {
        Self::new(Samples::Lazy(source), sample_rate)
    }

    fn new(samples: Samples, sample_rate: f64) -> Result<Self> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(Error::InvalidSampleRate(sample_rate));
        }
        Ok(Self {
            samples,
            sample_rate,
            center_frequency: 0.0,
            unit: SampleUnit::FullScale,
        })
    }

    /// An empty signal with this one's kind, sample rate, centre frequency and unit,
    /// holding no samples (and no reference to this one's buffer or source).
    #[must_use]
    pub fn empty_like(&self) -> Self {
        let samples = match self.kind() {
            SampleKind::Real => Samples::Real(Arc::from(Vec::<f32>::new())),
            SampleKind::Complex => Samples::Complex(Arc::from(Vec::<Complex32>::new())),
        };
        Self {
            samples,
            sample_rate: self.sample_rate,
            center_frequency: self.center_frequency,
            unit: self.unit,
        }
    }

    /// Set the RF frequency that sample frequency 0 Hz corresponds to (for I/Q captures).
    #[must_use]
    pub fn with_center_frequency(mut self, hz: f64) -> Self {
        self.center_frequency = hz;
        self
    }

    /// Declare what the sample values mean.
    #[must_use]
    pub fn with_unit(mut self, unit: SampleUnit) -> Self {
        self.unit = unit;
        self
    }

    /// What the sample values mean.
    pub fn unit(&self) -> SampleUnit {
        self.unit
    }

    /// The samples.
    pub fn samples(&self) -> &Samples {
        &self.samples
    }

    /// Sample rate in Hz.
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// RF centre frequency in Hz (0 when unknown).
    pub fn center_frequency(&self) -> f64 {
        self.center_frequency
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// True when the signal has no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Real or complex.
    pub fn kind(&self) -> SampleKind {
        self.samples.kind()
    }

    /// True when samples are read from disk on demand.
    pub fn is_lazy(&self) -> bool {
        matches!(self.samples, Samples::Lazy(_))
    }

    /// Duration in seconds.
    pub fn duration(&self) -> f64 {
        self.len() as f64 / self.sample_rate
    }

    /// Fill `out` with complex samples from `start` (real signals get a zero imaginary
    /// part). Returns the number written; the rest of `out` is zeroed. Reading past the
    /// end is allowed and simply returns fewer samples.
    pub fn read_complex(&self, start: usize, out: &mut [Complex32]) -> usize {
        let n = match &self.samples {
            Samples::Real(s) => copy_from(s, start, out, |v| Complex32::new(*v, 0.0)),
            Samples::Complex(s) => copy_from(s, start, out, |v| *v),
            Samples::Lazy(s) => s.read_complex(start, out),
        };
        for o in out.iter_mut().skip(n) {
            *o = Complex32::new(0.0, 0.0);
        }
        n
    }

    /// Fill `out` with real samples from `start` (complex signals give their I part).
    /// Returns the number written; the rest of `out` is zeroed.
    pub fn read_real(&self, start: usize, out: &mut [f32]) -> usize {
        let n = match &self.samples {
            Samples::Real(s) => copy_from(s, start, out, |v| *v),
            Samples::Complex(s) => copy_from(s, start, out, |v| v.re),
            Samples::Lazy(s) => s.read_real(start, out),
        };
        for o in out.iter_mut().skip(n) {
            *o = 0.0;
        }
        n
    }

    /// A copy of this signal held in memory. In-memory signals are returned as a cheap
    /// clone; lazy signals are read in full, so call this only on data that fits in RAM.
    ///
    /// Fails when the signal is too large to allocate or the lazy source delivers fewer
    /// samples than it claims to hold.
    pub fn materialize(&self) -> Result<Self> {
        match &self.samples {
            Samples::Lazy(src) => Ok(Self {
                samples: read_range(src.as_ref(), 0, src.len())?,
                ..self.clone()
            }),
            _ => Ok(self.clone()),
        }
    }

    /// Copy of the samples in `start..end` as a new in-memory signal with the same rate.
    pub fn slice(&self, start: usize, end: usize) -> Result<Self> {
        if start > end {
            return Err(Error::InvalidRange { start, end });
        }
        let len = self.len();
        if end > len {
            return Err(Error::RangeOutOfBounds { start, end, len });
        }
        let out_of_bounds = || Error::RangeOutOfBounds { start, end, len };
        let samples = match &self.samples {
            Samples::Real(s) => Samples::Real(s.get(start..end).ok_or_else(out_of_bounds)?.into()),
            Samples::Complex(s) => {
                Samples::Complex(s.get(start..end).ok_or_else(out_of_bounds)?.into())
            }
            Samples::Lazy(src) => read_range(src.as_ref(), start, end)?,
        };
        Ok(Self {
            samples,
            ..self.clone()
        })
    }
}

/// Copy `src[start..]` into `out` through `map`, returning how many were copied.
fn copy_from<T, U>(src: &[T], start: usize, out: &mut [U], map: impl Fn(&T) -> U) -> usize {
    let avail = src.get(start..).unwrap_or_default();
    let mut n = 0;
    for (o, v) in out.iter_mut().zip(avail) {
        *o = map(v);
        n += 1;
    }
    n
}

/// A zero-filled buffer of `count` elements, or [`Error::TooLarge`] when the memory
/// cannot be reserved (instead of aborting the process).
fn zeroed<T: Clone>(count: usize, zero: T) -> Result<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(count)
        .map_err(|_| Error::TooLarge { samples: count })?;
    v.resize(count, zero);
    Ok(v)
}

/// Read `start..end` of a lazy source into memory. The source must deliver every
/// requested sample; a short read means the data changed under us and is an error.
///
/// Blocks are read in parallel (sources are safe to call from several threads).
fn read_range(src: &dyn SampleSource, start: usize, end: usize) -> Result<Samples> {
    let count = end.saturating_sub(start);
    let short = |got: usize| Error::ShortRead {
        start,
        expected: count,
        got,
    };
    match src.kind() {
        SampleKind::Real => {
            let mut v = zeroed(count, 0.0_f32)?;
            if let Some(got) = read_blocks(&mut v, start, |at, out| src.read_real(at, out)) {
                return Err(short(got));
            }
            Ok(Samples::Real(v.into()))
        }
        SampleKind::Complex => {
            let mut v = zeroed(count, Complex32::new(0.0, 0.0))?;
            if let Some(got) = read_blocks(&mut v, start, |at, out| src.read_complex(at, out)) {
                return Err(short(got));
            }
            Ok(Samples::Complex(v.into()))
        }
    }
}

/// Samples per parallel block of [`read_range`].
const READ_BLOCK: usize = 1 << 18;

/// Fill `v` from source index `start` on with `read`, a block at a time in parallel.
/// Returns the number of samples before the first one that could not be read, or
/// `None` when every sample was read.
fn read_blocks<T: Send>(
    v: &mut [T],
    start: usize,
    read: impl Fn(usize, &mut [T]) -> usize + Sync,
) -> Option<usize> {
    v.par_chunks_mut(READ_BLOCK)
        .enumerate()
        .filter_map(|(b, out)| {
            let at = b * READ_BLOCK;
            let got = read(start + at, out);
            (got < out.len()).then_some(at + got)
        })
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lazy source that computes `sample[i] = i` on demand.
    #[derive(Debug)]
    struct Ramp {
        len: usize,
        kind: SampleKind,
    }

    impl SampleSource for Ramp {
        fn len(&self) -> usize {
            self.len
        }
        fn kind(&self) -> SampleKind {
            self.kind
        }
        fn read_complex(&self, start: usize, out: &mut [Complex32]) -> usize {
            let n = self.len.saturating_sub(start).min(out.len());
            for (k, o) in out[..n].iter_mut().enumerate() {
                let v = (start + k) as f32;
                *o = Complex32::new(v, -v);
            }
            n
        }
        fn read_real(&self, start: usize, out: &mut [f32]) -> usize {
            let n = self.len.saturating_sub(start).min(out.len());
            for (k, o) in out[..n].iter_mut().enumerate() {
                *o = (start + k) as f32;
            }
            n
        }
    }

    #[test]
    fn rejects_bad_sample_rates() {
        for sr in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                Signal::real(vec![0.0_f32; 4], sr).is_err(),
                "rate {sr} accepted"
            );
        }
    }

    #[test]
    fn duration_and_slice() {
        let s = Signal::real((0..8000).map(|i| i as f32).collect::<Vec<_>>(), 8000.0).unwrap();
        assert!((s.duration() - 1.0).abs() < 1e-12);
        let part = s.slice(100, 200).unwrap();
        assert_eq!(part.len(), 100);
        let Samples::Real(v) = part.samples() else {
            panic!("expected real samples")
        };
        assert!((v[0] - 100.0).abs() < f32::EPSILON);
        assert!(s.slice(10, 9000).is_err());
    }

    #[test]
    fn fractional_sample_rate_is_kept() {
        let s = Signal::complex(vec![Complex32::new(0.0, 0.0); 2], 6999.87).unwrap();
        assert!((s.sample_rate() - 6999.87).abs() < 1e-9);
        assert_eq!(s.kind(), SampleKind::Complex);
    }

    #[test]
    fn reads_are_uniform_across_storage_kinds() {
        let mem = Signal::real((0..10).map(|i| i as f32).collect::<Vec<_>>(), 1.0).unwrap();
        let lazy = Signal::lazy(
            Arc::new(Ramp {
                len: 10,
                kind: SampleKind::Real,
            }),
            1.0,
        )
        .unwrap();
        assert!(lazy.is_lazy());
        for s in [&mem, &lazy] {
            let mut buf = [9.0_f32; 4];
            assert_eq!(s.read_real(8, &mut buf), 2);
            assert_eq!(buf, [8.0, 9.0, 0.0, 0.0]);
            let mut c = [Complex32::new(1.0, 1.0); 2];
            assert_eq!(s.read_complex(20, &mut c), 0);
            assert_eq!(c, [Complex32::new(0.0, 0.0); 2]);
        }
    }

    #[test]
    fn lazy_slice_and_materialize() {
        let lazy = Signal::lazy(
            Arc::new(Ramp {
                len: 100,
                kind: SampleKind::Complex,
            }),
            50.0,
        )
        .unwrap();
        let part = lazy.slice(10, 13).unwrap();
        assert!(!part.is_lazy());
        let Samples::Complex(v) = part.samples() else {
            panic!("expected complex")
        };
        assert_eq!(v[2], Complex32::new(12.0, -12.0));
        let all = lazy.materialize().unwrap();
        assert_eq!(all.len(), 100);
        assert!(!all.is_lazy());
        assert!((all.sample_rate() - 50.0).abs() < f64::EPSILON);
    }

    /// A lazy source that claims more samples than it delivers.
    #[derive(Debug)]
    struct Liar;

    impl SampleSource for Liar {
        fn len(&self) -> usize {
            100
        }
        fn kind(&self) -> SampleKind {
            SampleKind::Real
        }
        fn read_complex(&self, _start: usize, _out: &mut [Complex32]) -> usize {
            0
        }
        fn read_real(&self, _start: usize, out: &mut [f32]) -> usize {
            out.len() / 2
        }
    }

    fn ramp(len: usize, kind: SampleKind) -> Signal {
        Signal::lazy(Arc::new(Ramp { len, kind }), 50.0).unwrap()
    }

    #[test]
    fn slice_edge_cases() {
        let s = Signal::real(vec![0.0_f32; 10], 100.0).unwrap();
        assert!(matches!(
            s.slice(5, 3),
            Err(Error::InvalidRange { start: 5, end: 3 })
        ));
        let empty = s.slice(10, 10).unwrap();
        assert!(empty.is_empty());
        assert!((empty.sample_rate() - 100.0).abs() < f64::EPSILON);
        assert!(s.slice(0, 0).unwrap().is_empty());

        let lazy = ramp(100, SampleKind::Real);
        assert!(matches!(
            lazy.slice(90, 101),
            Err(Error::RangeOutOfBounds {
                start: 90,
                end: 101,
                len: 100
            })
        ));
        assert!(matches!(lazy.slice(7, 2), Err(Error::InvalidRange { .. })));
        let lazy_empty = lazy.slice(100, 100).unwrap();
        assert!(lazy_empty.is_empty() && !lazy_empty.is_lazy());
        assert!((lazy_empty.sample_rate() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn short_reads_are_errors() {
        let liar = Signal::lazy(Arc::new(Liar), 1.0).unwrap();
        assert!(matches!(
            liar.slice(0, 10),
            Err(Error::ShortRead {
                start: 0,
                expected: 10,
                got: 5
            })
        ));
        assert!(matches!(liar.materialize(), Err(Error::ShortRead { .. })));
    }

    /// A ramp whose length spans several [`READ_BLOCK`]s, with a short final block.
    const MULTI_LEN: usize = 3 * READ_BLOCK + 17;

    #[test]
    fn multi_block_reads_keep_every_sample_in_place() {
        // Each read starting at `start` lands in blocks at source offsets
        // `start + b * READ_BLOCK`; check both sides of every block boundary.
        for start in [0, 5] {
            let count = MULTI_LEN - start;
            let edges = [
                0,
                READ_BLOCK - 1,
                READ_BLOCK,
                2 * READ_BLOCK - 1,
                2 * READ_BLOCK,
                3 * READ_BLOCK - 1,
                3 * READ_BLOCK,
                count - 1,
            ];

            let real = ramp(MULTI_LEN, SampleKind::Real)
                .slice(start, MULTI_LEN)
                .unwrap();
            assert_eq!(real.len(), count);
            let Samples::Real(v) = real.samples() else {
                panic!("expected real samples")
            };
            for k in edges {
                assert_eq!(v[k], (start + k) as f32, "real, start {start}, index {k}");
            }

            let cplx = ramp(MULTI_LEN, SampleKind::Complex)
                .slice(start, MULTI_LEN)
                .unwrap();
            assert_eq!(cplx.len(), count);
            let Samples::Complex(c) = cplx.samples() else {
                panic!("expected complex samples")
            };
            for k in edges {
                let want = (start + k) as f32;
                assert_eq!(
                    c[k],
                    Complex32::new(want, -want),
                    "complex, start {start}, index {k}"
                );
            }
        }
    }

    /// A ramp that delivers only half of any read whose start lies in one of the
    /// half-open ranges `bad`.
    #[derive(Debug)]
    struct Patchy {
        len: usize,
        bad: Vec<(usize, usize)>,
    }

    impl Patchy {
        fn deliver(&self, start: usize, want: usize) -> usize {
            let n = self.len.saturating_sub(start).min(want);
            if self.bad.iter().any(|&(lo, hi)| (lo..hi).contains(&start)) {
                n / 2
            } else {
                n
            }
        }
    }

    impl SampleSource for Patchy {
        fn len(&self) -> usize {
            self.len
        }
        fn kind(&self) -> SampleKind {
            SampleKind::Real
        }
        fn read_complex(&self, _start: usize, _out: &mut [Complex32]) -> usize {
            0
        }
        fn read_real(&self, start: usize, out: &mut [f32]) -> usize {
            let n = self.deliver(start, out.len());
            for (k, o) in out[..n].iter_mut().enumerate() {
                *o = (start + k) as f32;
            }
            n
        }
    }

    /// The `got` of the [`Error::ShortRead`] from slicing `start..MULTI_LEN` of a
    /// [`Patchy`] source failing on `bad`.
    fn short_got(start: usize, bad: Vec<(usize, usize)>) -> usize {
        let s = Signal::lazy(
            Arc::new(Patchy {
                len: MULTI_LEN,
                bad,
            }),
            1.0,
        )
        .unwrap();
        match s.slice(start, MULTI_LEN) {
            Err(Error::ShortRead {
                start: s0,
                expected,
                got,
            }) => {
                assert_eq!(s0, start);
                assert_eq!(expected, MULTI_LEN - start);
                got
            }
            other => panic!("expected a short read, got {other:?}"),
        }
    }

    #[test]
    fn multi_block_short_read_reports_the_exact_prefix() {
        let start = 5;
        let half = READ_BLOCK / 2;
        // Blocks start reading the source at `start + b * READ_BLOCK`; the final
        // block holds 17 - 5 = 12 samples.
        let block = |b: usize| start + b * READ_BLOCK;

        // Only block 2 fails: everything before it, plus half of it.
        assert_eq!(
            short_got(start, vec![(block(2), block(2) + 1)]),
            2 * READ_BLOCK + half
        );
        // Only the short final block fails.
        assert_eq!(
            short_got(start, vec![(block(3), block(3) + 1)]),
            3 * READ_BLOCK + 6
        );
        // Blocks 1 and 3 fail: the earlier failure is the one reported.
        assert_eq!(
            short_got(
                start,
                vec![(block(1), block(1) + 1), (block(3), block(3) + 1)]
            ),
            READ_BLOCK + half
        );
        // Every block from 2 on fails.
        assert_eq!(
            short_got(start, vec![(block(2), MULTI_LEN)]),
            2 * READ_BLOCK + half
        );
    }

    #[test]
    fn huge_lazy_signal_fails_instead_of_aborting() {
        let huge = ramp(usize::MAX / 2, SampleKind::Complex);
        assert!(matches!(huge.materialize(), Err(Error::TooLarge { .. })));
        let msg = huge.materialize().unwrap_err().to_string();
        assert!(msg.contains("shorter range"), "{msg}");
    }
}
