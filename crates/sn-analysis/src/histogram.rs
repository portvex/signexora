//! Value histogram of a span (SA: the waveform viewer's "Sum").
//!
//! On phase-detector output (values in Hz) each FSK/MFSK tone shows up as a peak, so
//! [`Histogram::peaks`] gives the tone frequencies; on ordinary signals it shows the
//! amplitude distribution (clipping, 8-bit steps, offsets).

use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use sn_core::Signal;

use crate::{Error, Result};

/// Counts of sample values in equal-width bins over `lo..hi`.
#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    /// Lower edge of the first bin.
    pub lo: f64,
    /// Upper edge of the last bin.
    pub hi: f64,
    /// Counts per bin.
    pub counts: Vec<u32>,
}

/// Largest span histogrammed, in samples.
pub const MAX_SAMPLES: usize = 1 << 27;

/// Samples read per chunk (the span is streamed, never held in full).
const CHUNK: usize = 1 << 16;

/// Most values kept for the automatic (percentile) range.
const RANGE_SAMPLES: usize = 1 << 20;

/// Smoothing kernels wider than this (in bins) use three box filters instead of a
/// direct Gaussian, so the cost does not grow with the width.
const DIRECT_SIGMA: f64 = 16.0;

impl Histogram {
    /// Histogram of the real part of samples `start..end` in `bins` bins. With no
    /// `range`, the range covers the 0.1-99.9 percentile of the finite values (robust
    /// to spikes). An explicit `range` must be two finite values with low < high.
    ///
    /// Same as [`Histogram::build_with_stop`] without cancellation.
    pub fn build(
        signal: &Signal,
        start: usize,
        end: usize,
        bins: usize,
        range: Option<(f64, f64)>,
    ) -> Result<Self> {
        Self::build_with_stop(signal, start, end, bins, range, &|| false)
    }

    /// [`Histogram::build`] that checks `should_stop` between chunks of samples and
    /// returns [`Error::Cancelled`] when it fires.
    pub fn build_with_stop(
        signal: &Signal,
        start: usize,
        end: usize,
        bins: usize,
        range: Option<(f64, f64)>,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Self> {
        let end = end.min(signal.len());
        let n = end.saturating_sub(start);
        if n < 2 {
            return Err(Error::Invalid(
                "the span holds fewer than two samples".into(),
            ));
        }
        if n > MAX_SAMPLES {
            return Err(Error::Invalid(format!(
                "select at most {MAX_SAMPLES} samples"
            )));
        }
        if !(2..=1 << 16).contains(&bins) {
            return Err(Error::Invalid("use between 2 and 65536 bins".into()));
        }
        let (lo, hi) = match range {
            Some((a, b)) => {
                if !(a.is_finite() && b.is_finite() && b > a) {
                    return Err(Error::Invalid(
                        "the histogram range must be two finite values with low < high".into(),
                    ));
                }
                (a, b)
            }
            None => robust_range(signal, start, end, should_stop)?,
        };
        let scale = bins as f64 / (hi - lo);
        // Chunks are counted in parallel and the integer counts added, which gives
        // exactly the counts of a serial pass. A failed read does not short-circuit
        // at once: every chunk before it is still read, and the error of the
        // lowest-numbered failing chunk is returned, so the result does not depend
        // on thread scheduling.
        let chunks = n.div_ceil(CHUNK);
        let first = FirstError::new(chunks);
        let (counts, err) = (0..chunks)
            .into_par_iter()
            .fold(
                || (vec![0_u32; bins], vec![0.0_f32; CHUNK.min(n)], None),
                |(mut counts, mut buf, mut err): (Vec<u32>, Vec<f32>, Option<(usize, Error)>),
                 c| {
                    if first.skips(c) {
                        return (counts, buf, err);
                    }
                    let pos = start + c * CHUNK;
                    let len = CHUNK.min(end - pos);
                    match read_chunk(
                        signal,
                        pos,
                        buf.get_mut(..len).unwrap_or_default(),
                        should_stop,
                    ) {
                        Ok(chunk) => {
                            for x in chunk {
                                let pos = (f64::from(*x) - lo) * scale;
                                if pos >= 0.0
                                    && pos < bins as f64
                                    && let Some(c) = counts.get_mut(pos as usize)
                                {
                                    *c = c.saturating_add(1);
                                }
                            }
                        }
                        Err(e) => {
                            first.record(c);
                            err = Some(earlier(err, (c, e)));
                        }
                    }
                    (counts, buf, err)
                },
            )
            .map(|(counts, _, err)| (counts, err))
            .reduce(
                || (vec![0_u32; bins], None),
                |(mut a, ea), (b, eb)| {
                    for (x, y) in a.iter_mut().zip(b) {
                        *x = x.saturating_add(y);
                    }
                    let err = match eb {
                        Some(e) => Some(earlier(ea, e)),
                        None => ea,
                    };
                    (a, err)
                },
            );
        if let Some((_, e)) = err {
            return Err(e);
        }
        Ok(Self { lo, hi, counts })
    }

    /// Width of one bin in value units (Hz for phase-detector output).
    pub fn bin_width(&self) -> f64 {
        (self.hi - self.lo) / self.counts.len().max(1) as f64
    }

    /// Centre value of bin `k`.
    pub fn value_of(&self, k: f64) -> f64 {
        self.lo + (k + 0.5) * self.bin_width()
    }

    /// Peak positions (value units), strongest first.
    ///
    /// The histogram is smoothed with a Gaussian of `smooth` (value units, e.g. a few
    /// Hz; at least half a bin, at most the whole histogram), then a peak is kept when
    /// its *prominence* (height above the deepest valley separating it from any higher
    /// peak) reaches `min_rel` of the highest peak. Transition humps between FSK tones
    /// have low prominence; real tones do not. Positions are refined by the centroid
    /// of the raw counts around the peak.
    ///
    /// A `smooth` that is negative or not finite is not a usable width: the result is
    /// then empty.
    pub fn peaks(&self, min_rel: f64, smooth: f64) -> Vec<f64> {
        let n = self.counts.len();
        if !(smooth.is_finite() && smooth >= 0.0) || n == 0 {
            return Vec::new();
        }
        let sigma = (smooth / self.bin_width()).clamp(0.5, n as f64);
        let s = self.smoothed(sigma);
        let top = s.iter().copied().fold(0.0_f64, f64::max);
        if top <= 0.0 {
            return Vec::new();
        }
        let at = |k: usize| s.get(k).copied().unwrap_or(0.0);
        let maxima: Vec<usize> = (1..n.saturating_sub(1))
            .filter(|&k| at(k) >= at(k - 1) && at(k) > at(k + 1))
            .collect();
        let mut peaks: Vec<(usize, f64)> = maxima
            .iter()
            .filter_map(|&k| {
                let h = at(k);
                // Walk left and right until a higher point; track the lowest valley.
                let side = |range: &mut dyn Iterator<Item = usize>| -> f64 {
                    let mut low = h;
                    for j in range {
                        let v = at(j);
                        if v > h {
                            return low;
                        }
                        low = low.min(v);
                    }
                    0.0 // reached the edge: the base is zero
                };
                let left = side(&mut (0..k).rev());
                let right = side(&mut (k + 1..n));
                let prominence = h - left.max(right);
                (prominence >= min_rel * top).then_some((k, prominence))
            })
            .collect();
        peaks.sort_by(|a, b| b.1.total_cmp(&a.1));
        // sigma <= n, so the reach stays within a few histogram lengths.
        let reach = ((3.0 * sigma).ceil() as usize).min(n);
        peaks
            .into_iter()
            .map(|(k, _)| {
                let (mut num, mut den) = (0.0, 0.0);
                for j in k.saturating_sub(reach)..=(k + reach).min(n - 1) {
                    let c = f64::from(self.counts.get(j).copied().unwrap_or(0));
                    num += c * j as f64;
                    den += c;
                }
                self.value_of(if den > 0.0 { num / den } else { k as f64 })
            })
            .collect()
    }

    /// The counts smoothed by a Gaussian of `sigma` bins (zero outside the histogram).
    fn smoothed(&self, sigma: f64) -> Vec<f64> {
        let counts: Vec<f64> = self.counts.iter().map(|c| f64::from(*c)).collect();
        if sigma <= DIRECT_SIGMA {
            gaussian(&counts, sigma)
        } else {
            three_box(&counts, sigma)
        }
    }
}

/// Direct Gaussian convolution; `sigma` is small (see `DIRECT_SIGMA`).
fn gaussian(counts: &[f64], sigma: f64) -> Vec<f64> {
    let radius = (3.0 * sigma).ceil() as i64;
    let kernel: Vec<f64> = (-radius..=radius)
        .map(|i| (-0.5 * (i as f64 / sigma).powi(2)).exp())
        .collect();
    let norm: f64 = kernel.iter().sum();
    let n = counts.len() as i64;
    (0..n)
        .map(|k| {
            kernel
                .iter()
                .zip(-radius..=radius)
                .map(|(w, d)| {
                    usize::try_from(k + d)
                        .ok()
                        .and_then(|j| counts.get(j))
                        .map_or(0.0, |c| w * c)
                })
                .sum::<f64>()
                / norm
        })
        .collect()
}

/// Gaussian approximated by three passes of a box filter (cost independent of the
/// width). The data is zero-padded so nothing wraps around.
fn three_box(counts: &[f64], sigma: f64) -> Vec<f64> {
    // Three boxes of odd width w have variance 3 (w^2 - 1) / 12 = sigma^2.
    let w = ((4.0 * sigma * sigma + 1.0).sqrt().round() as usize) | 1;
    let r = w / 2;
    let pad = 3 * r + 1;
    let mut a = vec![0.0_f64; counts.len() + 2 * pad];
    if let Some(dst) = a.get_mut(pad..pad + counts.len()) {
        dst.copy_from_slice(counts);
    }
    for _ in 0..3 {
        a = box_filter(&a, r);
    }
    a.get(pad..pad + counts.len())
        .map_or_else(|| vec![0.0; counts.len()], <[f64]>::to_vec)
}

/// Centred moving average of width `2r + 1`, zero outside `a`.
fn box_filter(a: &[f64], r: usize) -> Vec<f64> {
    let mut prefix = Vec::with_capacity(a.len() + 1);
    prefix.push(0.0_f64);
    let mut acc = 0.0;
    for v in a {
        acc += v;
        prefix.push(acc);
    }
    let w = (2 * r + 1) as f64;
    let p = |i: usize| prefix.get(i.min(a.len())).copied().unwrap_or(0.0);
    (0..a.len())
        .map(|k| (p(k + r + 1) - p(k.saturating_sub(r))) / w)
        .collect()
}

/// Lowest-numbered chunk whose read failed so far, shared by parallel workers.
/// Chunks after it need not be read, but every chunk before it must be, so the
/// error finally returned is that of the first failing chunk whatever the thread
/// timing.
struct FirstError(AtomicUsize);

impl FirstError {
    /// No failure yet among `chunks` chunks.
    fn new(chunks: usize) -> Self {
        Self(AtomicUsize::new(chunks))
    }

    /// True when chunk `c` comes after a chunk that already failed.
    fn skips(&self, c: usize) -> bool {
        c > self.0.load(Ordering::Relaxed)
    }

    /// Notes that chunk `c` failed.
    fn record(&self, c: usize) {
        self.0.fetch_min(c, Ordering::Relaxed);
    }
}

/// Whichever of `a` and `b` belongs to the lower-numbered chunk.
fn earlier(a: Option<(usize, Error)>, b: (usize, Error)) -> (usize, Error) {
    match a {
        Some(a) if a.0 <= b.0 => a,
        _ => b,
    }
}

/// Extremes and picked values of one chunk, from [`robust_range`].
type RangePart = (f32, f32, Vec<f32>);

/// Fill `chunk` with the real samples from `pos` on, after checking `should_stop`.
///
/// A source that delivers fewer samples than requested (a truncated or changed file)
/// is reported as an error (from [`sn_core::Error::ShortRead`]) rather than giving a
/// result over fewer samples.
fn read_chunk<'a>(
    signal: &Signal,
    pos: usize,
    chunk: &'a mut [f32],
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<&'a [f32]> {
    if should_stop() {
        return Err(Error::Cancelled);
    }
    let got = signal.read_real(pos, chunk);
    if got < chunk.len() {
        return Err(sn_core::Error::ShortRead {
            start: pos,
            expected: chunk.len(),
            got,
        }
        .into());
    }
    Ok(chunk)
}

/// Robust automatic range of samples `start..end`: the 0.1 and 99.9 percentile of
/// the finite values, padded by 5%. When those coincide (more than 99.8% of the
/// values equal) the full min..max of the finite values is used instead.
///
/// Long spans are subsampled (at most `RANGE_SAMPLES` values, spread evenly).
fn robust_range(
    signal: &Signal,
    start: usize,
    end: usize,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<(f64, f64)> {
    let n = end.saturating_sub(start);
    let stride = n.div_ceil(RANGE_SAMPLES).max(1);
    // Chunks are scanned in parallel; the extremes and the picked values do not
    // depend on the order in which chunks are combined. Results are kept in chunk
    // order and the first failing chunk's error is returned, so the error does not
    // depend on thread scheduling either.
    let chunks = n.div_ceil(CHUNK);
    let first = FirstError::new(chunks);
    let results: Vec<Option<Result<RangePart>>> = (0..chunks)
        .into_par_iter()
        .map_init(
            || vec![0.0_f32; CHUNK.min(n)],
            |buf, c| {
                if first.skips(c) {
                    return None;
                }
                let offset = c * CHUNK;
                let len = CHUNK.min(n - offset);
                let chunk = match read_chunk(
                    signal,
                    start + offset,
                    buf.get_mut(..len).unwrap_or_default(),
                    should_stop,
                ) {
                    Ok(chunk) => chunk,
                    Err(e) => {
                        first.record(c);
                        return Some(Err(e));
                    }
                };
                let (mut min, mut max) = (f32::INFINITY, f32::NEG_INFINITY);
                let mut kept = Vec::new();
                for (i, x) in chunk.iter().enumerate() {
                    if !x.is_finite() {
                        continue;
                    }
                    min = min.min(*x);
                    max = max.max(*x);
                    let k = offset + i;
                    // Pick one value per stride block, at a position that varies from
                    // block to block so a periodic signal does not alias with the
                    // stride.
                    let block = k / stride;
                    let pick = (block.wrapping_mul(2_654_435_761) >> 7) % stride;
                    if k % stride == pick {
                        kept.push(*x);
                    }
                }
                Some(Ok((min, max, kept)))
            },
        )
        .collect();
    // A chunk is skipped only after a lower-numbered chunk failed, so walking in
    // order meets that error before any skipped chunk.
    let mut parts = Vec::with_capacity(results.len());
    for r in results {
        match r {
            Some(Ok(part)) => parts.push(part),
            Some(Err(e)) => return Err(e),
            None => {}
        }
    }
    let mut kept: Vec<f32> = Vec::with_capacity(n.min(RANGE_SAMPLES));
    let (mut min, mut max) = (f32::INFINITY, f32::NEG_INFINITY);
    for (lo, hi, values) in parts {
        min = min.min(lo);
        max = max.max(hi);
        kept.extend(values);
    }
    if !(min.is_finite() && max.is_finite()) {
        return Err(Error::NotFound("the span holds no finite values".into()));
    }
    if min >= max {
        return Err(Error::NotFound("the values are constant".into()));
    }
    let (mut lo, mut hi) = (f64::from(min), f64::from(max));
    if !kept.is_empty() {
        let last = kept.len() - 1;
        let rank = |q: f64| ((last as f64 * q).round() as usize).min(last);
        let (ilo, ihi) = (rank(0.001), rank(0.999));
        let plo = f64::from(*kept.select_nth_unstable_by(ilo, f32::total_cmp).1);
        let phi = f64::from(*kept.select_nth_unstable_by(ihi, f32::total_cmp).1);
        if phi > plo {
            (lo, hi) = (plo, phi);
        }
    }
    let pad = (hi - lo) * 0.05;
    Ok((lo - pad, hi + pad))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clusters() -> Signal {
        // Three clusters of values at 100, 200 and 350.
        let mut v = Vec::new();
        for (c, n) in [(100.0_f32, 3000), (200.0, 2000), (350.0, 1000)] {
            for i in 0..n {
                v.push(c + ((i % 7) as f32 - 3.0) * 0.5);
            }
        }
        Signal::real(v, 1000.0).unwrap()
    }

    #[test]
    fn counts_and_finds_peaks() {
        let s = clusters();
        let h = Histogram::build(&s, 0, s.len(), 500, None).unwrap();
        assert_eq!(h.counts.iter().map(|c| *c as usize).sum::<usize>(), 6000);
        let p = h.peaks(0.1, 2.0);
        assert_eq!(p.len(), 3, "{p:?}");
        for (got, want) in p.iter().zip([100.0, 200.0, 350.0]) {
            assert!((got - want).abs() < h.bin_width(), "{got} vs {want}");
        }
        assert!(Histogram::build(&s, 0, 1, 10, None).is_err());
        let flat = Signal::real(vec![1.0_f32; 100], 1.0).unwrap();
        assert!(Histogram::build(&flat, 0, 100, 10, None).is_err());
    }

    /// A lazy source that claims more samples than it delivers.
    #[derive(Debug)]
    struct Truncated;

    impl sn_core::SampleSource for Truncated {
        fn len(&self) -> usize {
            3 * CHUNK
        }
        fn kind(&self) -> sn_core::SampleKind {
            sn_core::SampleKind::Real
        }
        fn read_complex(&self, _start: usize, _out: &mut [sn_core::Complex32]) -> usize {
            0
        }
        fn read_real(&self, start: usize, out: &mut [f32]) -> usize {
            // Only the first chunk and a half are really there.
            let have = (CHUNK + CHUNK / 2).saturating_sub(start).min(out.len());
            for (i, v) in out.iter_mut().take(have).enumerate() {
                *v = ((start + i) % 13) as f32;
            }
            have
        }
    }

    #[test]
    fn short_reads_are_errors() {
        let s = Signal::lazy(std::sync::Arc::new(Truncated), 1000.0).unwrap();
        // Chunks 1 and 2 both fail; the error must always be chunk 1's, whatever
        // the thread timing, so repeat to catch a scheduling-dependent answer.
        let ranges = [None, Some((0.0, 13.0))];
        for range in ranges.into_iter().cycle().take(100) {
            let r = Histogram::build(&s, 0, s.len(), 13, range);
            assert!(
                matches!(&r, Err(Error::Invalid(m)) if m.contains(&format!(
                    "returned only {} of {CHUNK} samples starting at sample {CHUNK}",
                    CHUNK / 2
                ))),
                "{r:?}"
            );
        }
    }

    #[test]
    fn extreme_smoothing_neither_panics_nor_hangs() {
        let s = clusters();
        let h = Histogram::build(&s, 0, s.len(), 4, None).unwrap();
        assert_eq!(h.peaks(0.1, f64::INFINITY), Vec::<f64>::new());
        assert_eq!(h.peaks(0.1, f64::NAN), Vec::<f64>::new());
        assert_eq!(h.peaks(0.1, -1.0), Vec::<f64>::new());
        let t = std::time::Instant::now();
        let p = h.peaks(0.1, 1e12);
        assert!(p.len() <= 1, "{p:?}");
        let big = Histogram::build(&s, 0, s.len(), 1 << 16, None).unwrap();
        let p = big.peaks(0.1, 1e12);
        assert!(p.len() <= 1, "{p:?}");
        assert!(
            t.elapsed().as_secs() < 5,
            "smoothing took {:?}",
            t.elapsed()
        );
        // Wide but finite smoothing merges the clusters into fewer peaks.
        let wide = Histogram::build(&s, 0, s.len(), 2000, None).unwrap();
        assert!(wide.peaks(0.1, 60.0).len() < 3);
    }

    #[test]
    fn bad_ranges_and_degenerate_data() {
        let s = clusters();
        for r in [
            (5.0, 5.0),
            (6.0, 5.0),
            (f64::NAN, 1.0),
            (f64::NEG_INFINITY, 1.0),
            (0.0, f64::INFINITY),
        ] {
            assert!(
                matches!(
                    Histogram::build(&s, 0, s.len(), 10, Some(r)),
                    Err(Error::Invalid(_))
                ),
                "{r:?}"
            );
        }
        let nan = Signal::real(vec![f32::NAN; 100], 1.0).unwrap();
        let e = Histogram::build(&nan, 0, 100, 10, None).unwrap_err();
        assert!(e.to_string().contains("no finite"), "{e}");
        // Mostly silent with a few events: not constant.
        let mut v = vec![0.0_f32; 10_000];
        v[100] = 1.0;
        v[5000] = -1.0;
        let sparse = Signal::real(v, 1.0).unwrap();
        let h = Histogram::build(&sparse, 0, 10_000, 10, None).unwrap();
        assert!(h.lo < -1.0 && h.hi > 1.0, "{h:?}");
        assert_eq!(h.counts.iter().sum::<u32>(), 10_000);
    }

    #[test]
    fn long_spans_are_subsampled_and_cancellable() {
        let n = 3 * RANGE_SAMPLES;
        let v: Vec<f32> = (0..n).map(|i| (i % 1000) as f32).collect();
        let s = Signal::real(v, 1.0).unwrap();
        let h = Histogram::build(&s, 0, n, 100, None).unwrap();
        assert!(h.lo < 2.0 && h.hi > 997.0, "{h:?}");
        assert_eq!(h.counts.iter().map(|c| *c as usize).sum::<usize>(), n);
        assert!(matches!(
            Histogram::build_with_stop(&s, 0, n, 100, None, &|| true),
            Err(Error::Cancelled)
        ));
    }
}
