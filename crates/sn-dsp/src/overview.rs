//! Min/max envelope of a signal, for drawing waveforms of any length quickly.
//!
//! [`Overview::build`] scans the signal once (in parallel) and stores the minimum and
//! maximum of every block of [`Overview::block`] samples. Queries over long spans then
//! read only the block table; short spans read the samples directly.

use rayon::prelude::*;
use sn_core::{Complex32, SampleKind, Signal};

use crate::{Error, Result};

/// Minimum and maximum over a span.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinMax {
    /// Smallest value.
    pub min: f32,
    /// Largest value.
    pub max: f32,
}

impl MinMax {
    /// The neutral element: merging it changes nothing.
    pub const EMPTY: MinMax = MinMax {
        min: f32::INFINITY,
        max: f32::NEG_INFINITY,
    };

    /// True when no sample was seen.
    pub fn is_empty(&self) -> bool {
        self.min > self.max
    }

    /// Include a value.
    #[inline]
    pub fn push(&mut self, v: f32) {
        self.min = self.min.min(v);
        self.max = self.max.max(v);
    }

    /// Include another span.
    #[must_use]
    pub fn merge(self, other: MinMax) -> MinMax {
        MinMax {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

/// Envelope of the real part (and the imaginary part for complex signals).
#[derive(Debug, Clone)]
pub struct Overview {
    block: usize,
    len: usize,
    re: Vec<MinMax>,
    im: Option<Vec<MinMax>>,
}

/// Envelope values for one span.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanEnvelope {
    /// Real part (or the only part of a real signal).
    pub re: MinMax,
    /// Imaginary part, for complex signals.
    pub im: Option<MinMax>,
}

impl Overview {
    /// Default block size.
    pub const DEFAULT_BLOCK: usize = 1024;

    /// Scan `signal` in blocks of `block` samples.
    pub fn build(
        signal: &Signal,
        block: usize,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Self> {
        if block == 0 {
            return Err(Error::InvalidParameter {
                name: "block",
                reason: "must be at least 1".into(),
            });
        }
        let len = signal.len();
        let blocks = len.div_ceil(block);
        let complex = signal.kind() == SampleKind::Complex;
        let per_block: Vec<Option<(MinMax, MinMax)>> = (0..blocks)
            .into_par_iter()
            .map_init(
                || vec![Complex32::new(0.0, 0.0); block],
                |buf, b| {
                    if b % 256 == 0 && should_stop() {
                        return None;
                    }
                    let got = signal.read_complex(b.saturating_mul(block), buf);
                    let mut re = MinMax::EMPTY;
                    let mut im = MinMax::EMPTY;
                    for c in buf.iter().take(got) {
                        re.push(c.re);
                        im.push(c.im);
                    }
                    Some((re, im))
                },
            )
            .collect();
        if should_stop() {
            return Err(Error::Cancelled);
        }
        let mut re = Vec::with_capacity(blocks);
        let mut im = Vec::with_capacity(if complex { blocks } else { 0 });
        for entry in per_block {
            let (r, i) = entry.ok_or(Error::Cancelled)?;
            re.push(r);
            if complex {
                im.push(i);
            }
        }
        Ok(Self {
            block,
            len,
            re,
            im: complex.then_some(im),
        })
    }

    /// Block size in samples.
    pub fn block(&self) -> usize {
        self.block
    }

    /// Number of blocks: the signal length divided by the block size, rounded up.
    pub fn blocks(&self) -> usize {
        self.re.len()
    }

    /// Signal length in samples.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True for an empty signal.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Envelope of samples `start..end`. Uses whole blocks where the span covers them
    /// and reads the remaining edge samples from `signal`.
    pub fn span(&self, signal: &Signal, start: usize, end: usize) -> SpanEnvelope {
        let end = end.min(self.len);
        let mut re = MinMax::EMPTY;
        let mut im = MinMax::EMPTY;
        if start >= end {
            return self.envelope(re, im);
        }
        let first_full = start.div_ceil(self.block);
        let last_full = end / self.block; // exclusive
        if first_full < last_full {
            for b in first_full..last_full {
                if let Some(v) = self.re.get(b) {
                    re = re.merge(*v);
                }
                if let Some(v) = self.im.as_ref().and_then(|v| v.get(b)) {
                    im = im.merge(*v);
                }
            }
            Self::read_into(
                signal,
                start,
                first_full.saturating_mul(self.block),
                &mut re,
                &mut im,
            );
            Self::read_into(
                signal,
                last_full.saturating_mul(self.block),
                end,
                &mut re,
                &mut im,
            );
        } else {
            Self::read_into(signal, start, end, &mut re, &mut im);
        }
        self.envelope(re, im)
    }

    fn envelope(&self, re: MinMax, im: MinMax) -> SpanEnvelope {
        SpanEnvelope {
            re,
            im: self.im.is_some().then_some(im),
        }
    }

    fn read_into(signal: &Signal, start: usize, end: usize, re: &mut MinMax, im: &mut MinMax) {
        if start >= end {
            return;
        }
        let mut buf = vec![Complex32::new(0.0, 0.0); end - start];
        let got = signal.read_complex(start, &mut buf);
        for c in buf.iter().take(got) {
            re.push(c.re);
            im.push(c.im);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute(v: &[f32], s: usize, e: usize) -> MinMax {
        let mut m = MinMax::EMPTY;
        for x in &v[s..e] {
            m.push(*x);
        }
        m
    }

    #[test]
    fn spans_match_brute_force() {
        let v: Vec<f32> = (0..10_000)
            .map(|i| ((i * 7919) % 2001) as f32 / 1000.0 - 1.0)
            .collect();
        let s = Signal::real(v.clone(), 1.0).unwrap();
        let o = Overview::build(&s, 64, &|| false).unwrap();
        assert_eq!(o.blocks(), 10_000_usize.div_ceil(64));
        for (a, b) in [
            (0, 10_000),
            (5, 70),
            (63, 129),
            (100, 9_999),
            (640, 704),
            (3, 4),
        ] {
            let got = o.span(&s, a, b);
            assert_eq!(got.re, brute(&v, a, b), "span {a}..{b}");
            assert!(got.im.is_none());
        }
        assert!(o.span(&s, 50, 50).re.is_empty());
        assert_eq!(o.span(&s, 9_990, 20_000).re, brute(&v, 9_990, 10_000));
    }

    #[test]
    fn complex_has_both_parts_and_cancel_works() {
        let iq: Vec<Complex32> = (0..1000)
            .map(|i| Complex32::new(i as f32, -(i as f32)))
            .collect();
        let s = Signal::complex(iq, 1.0).unwrap();
        let o = Overview::build(&s, 100, &|| false).unwrap();
        assert_eq!(o.blocks(), 10);
        let e = o.span(&s, 10, 990);
        assert_eq!(
            e.re,
            MinMax {
                min: 10.0,
                max: 989.0
            }
        );
        assert_eq!(
            e.im,
            Some(MinMax {
                min: -989.0,
                max: -10.0
            })
        );
        assert!(matches!(
            Overview::build(&s, 100, &|| true),
            Err(Error::Cancelled)
        ));
        assert!(Overview::build(&s, 0, &|| false).is_err());
    }
}
