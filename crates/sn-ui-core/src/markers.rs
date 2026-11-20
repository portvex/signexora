//! SA's two marker pairs: V markers select a time span, H markers a frequency band.
//! Almost every tool works on this selection.

/// One of the four marker lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerId {
    /// First vertical (time) marker.
    V1,
    /// Second vertical (time) marker.
    V2,
    /// First horizontal (frequency) marker.
    H1,
    /// Second horizontal (frequency) marker.
    H2,
}

impl MarkerId {
    /// Every marker, in hit-test order (ties go to the earlier one).
    pub const ALL: [Self; 4] = [Self::V1, Self::V2, Self::H1, Self::H2];

    /// The marker is a time (V) marker.
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::V1 | Self::V2)
    }
}

/// Marker positions: times in samples, frequencies in Hz (relative to the signal's
/// 0 Hz). `None` means the pair is off (SA: double-click collapses the markers).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Markers {
    /// V1 and V2 in samples.
    pub v: Option<(f64, f64)>,
    /// H1 and H2 in Hz.
    pub h: Option<(f64, f64)>,
    /// A locked marker cannot be moved (SA: right-click turns it red).
    pub locked: Option<MarkerId>,
    /// The marker moved last; arrow keys nudge it.
    pub active: Option<MarkerId>,
}

/// The marker nearest a point, within `limit` pixels. `distance(id, value)` is the
/// distance in pixels from the point to marker `id` at `value`, or `None` when
/// that marker cannot be grabbed there; each window supplies its own mapping.
/// Ties go to the earlier marker of [`MarkerId::ALL`].
pub fn nearest(
    markers: &Markers,
    limit: f64,
    distance: impl Fn(MarkerId, f64) -> Option<f64>,
) -> Option<MarkerId> {
    MarkerId::ALL
        .into_iter()
        .filter_map(|id| {
            let d = distance(id, markers.get(id)?)?;
            (d <= limit).then_some((id, d))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// The values SA shows for the current selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Readout {
    /// Lower H marker, Hz.
    pub f_lo: Option<f64>,
    /// Upper H marker, Hz.
    pub f_hi: Option<f64>,
    /// Centre between H markers, Hz.
    pub f_c: Option<f64>,
    /// Band between H markers, Hz.
    pub d_f: Option<f64>,
    /// Earlier V marker, seconds.
    pub t1: Option<f64>,
    /// Later V marker, seconds.
    pub t2: Option<f64>,
    /// Span between V markers, seconds.
    pub d_t: Option<f64>,
    /// `1 / dT`: the span as a rate (Bd when it covers one symbol).
    pub s: Option<f64>,
}

impl Markers {
    /// Position of a marker.
    pub fn get(&self, id: MarkerId) -> Option<f64> {
        match id {
            MarkerId::V1 => self.v.map(|v| v.0),
            MarkerId::V2 => self.v.map(|v| v.1),
            MarkerId::H1 => self.h.map(|h| h.0),
            MarkerId::H2 => self.h.map(|h| h.1),
        }
    }

    /// Move a marker. Returns false (and changes nothing) when the marker is locked, its
    /// pair is off, or `value` is not finite.
    pub fn set(&mut self, id: MarkerId, value: f64) -> bool {
        if self.locked == Some(id) || !value.is_finite() {
            return false;
        }
        let slot = match id {
            MarkerId::V1 => self.v.as_mut().map(|v| &mut v.0),
            MarkerId::V2 => self.v.as_mut().map(|v| &mut v.1),
            MarkerId::H1 => self.h.as_mut().map(|h| &mut h.0),
            MarkerId::H2 => self.h.as_mut().map(|h| &mut h.1),
        };
        match slot {
            Some(s) => {
                *s = value;
                self.active = Some(id);
                true
            }
            None => false,
        }
    }

    /// Start a new selection at `(sample, hz)`. A locked marker keeps its position and
    /// the other marker of its pair starts at the click.
    pub fn begin_sweep(&mut self, sample: f64, hz: f64) {
        self.v = Some(match (self.locked, self.v) {
            (Some(MarkerId::V1), Some((a, _))) => (a, sample),
            (Some(MarkerId::V2), Some((_, b))) => (sample, b),
            _ => (sample, sample),
        });
        self.h = Some(match (self.locked, self.h) {
            (Some(MarkerId::H1), Some((a, _))) => (a, hz),
            (Some(MarkerId::H2), Some((_, b))) => (hz, b),
            _ => (hz, hz),
        });
    }

    /// Extend the selection started by [`Self::begin_sweep`] to `(sample, hz)`.
    pub fn sweep_to(&mut self, sample: f64, hz: f64) {
        let v_free = if self.locked == Some(MarkerId::V2) {
            MarkerId::V1
        } else {
            MarkerId::V2
        };
        let h_free = if self.locked == Some(MarkerId::H2) {
            MarkerId::H1
        } else {
            MarkerId::H2
        };
        self.set(v_free, sample);
        self.set(h_free, hz);
    }

    /// Lock or unlock a marker. Only one marker can be locked at a time.
    pub fn toggle_lock(&mut self, id: MarkerId) {
        self.locked = if self.locked == Some(id) {
            None
        } else {
            Some(id)
        };
    }

    /// Readout values for a signal sampled at `sample_rate`.
    pub fn readout(&self, sample_rate: f64) -> Readout {
        let (f_lo, f_hi) = match self.h {
            Some((a, b)) => (Some(a.min(b)), Some(a.max(b))),
            None => (None, None),
        };
        let (t1, t2) = match self.v {
            Some((a, b)) if sample_rate > 0.0 => {
                (Some(a.min(b) / sample_rate), Some(a.max(b) / sample_rate))
            }
            _ => (None, None),
        };
        let d_t = t1.zip(t2).map(|(a, b)| b - a);
        Readout {
            f_lo,
            f_hi,
            f_c: f_lo.zip(f_hi).map(|(a, b)| f64::midpoint(a, b)),
            d_f: f_lo.zip(f_hi).map(|(a, b)| b - a),
            t1,
            t2,
            d_t,
            s: d_t.filter(|d| *d > 0.0).map(|d| 1.0 / d),
        }
    }

    /// Ordered sample range of the V markers, if on and non-empty.
    pub fn time_range(&self) -> Option<(f64, f64)> {
        self.v
            .map(|(a, b)| (a.min(b), a.max(b)))
            .filter(|(a, b)| b > a)
    }

    /// Samples between the V markers of a signal of `len` samples, both ends
    /// clamped to `0..=len` (the sonogram draws past the end of the recording, so
    /// a marker can sit there). The whole signal when the markers are off or the
    /// clamped span is empty.
    pub fn sample_span(&self, len: usize) -> (usize, usize) {
        let clamp = |x: f64| (x.max(0.0) as usize).min(len);
        match self.time_range().map(|(a, b)| (clamp(a), clamp(b))) {
            Some((a, b)) if b > a => (a, b),
            _ => (0, len),
        }
    }

    /// Ordered frequency range of the H markers, if on and non-empty.
    pub fn freq_range(&self) -> Option<(f64, f64)> {
        self.h
            .map(|(a, b)| (a.min(b), a.max(b)))
            .filter(|(a, b)| b > a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_span_clamps_both_ends() {
        let m = |a: f64, b: f64| Markers {
            v: Some((a, b)),
            ..Markers::default()
        };
        assert_eq!(Markers::default().sample_span(100), (0, 100));
        assert_eq!(m(10.0, 20.0).sample_span(100), (10, 20));
        assert_eq!(m(20.0, 10.0).sample_span(100), (10, 20));
        assert_eq!(m(90.0, 150.0).sample_span(100), (90, 100), "partly beyond");
        assert_eq!(m(150.0, 600.0).sample_span(100), (0, 100), "fully beyond");
        assert_eq!(m(-50.0, 10.0).sample_span(100), (0, 10));
    }

    #[test]
    fn readout_matches_sa_definitions() {
        // 2400 Bd: one symbol = 20 samples at 48 kHz.
        let m = Markers {
            v: Some((48_020.0, 48_000.0)),
            h: Some((2100.0, 1500.0)),
            ..Default::default()
        };
        let r = m.readout(48_000.0);
        assert_eq!(r.f_lo, Some(1500.0));
        assert_eq!(r.f_hi, Some(2100.0));
        assert_eq!(r.f_c, Some(1800.0));
        assert_eq!(r.d_f, Some(600.0));
        assert!((r.t1.unwrap() - 1.0).abs() < 1e-12);
        assert!((r.d_t.unwrap() - 20.0 / 48_000.0).abs() < 1e-15);
        assert!((r.s.unwrap() - 2400.0).abs() < 1e-6);
    }

    #[test]
    fn sweep_and_lock() {
        let mut m = Markers::default();
        m.begin_sweep(100.0, 1000.0);
        m.sweep_to(300.0, 1200.0);
        assert_eq!(m.time_range(), Some((100.0, 300.0)));
        assert_eq!(m.freq_range(), Some((1000.0, 1200.0)));

        m.toggle_lock(MarkerId::V1);
        assert!(!m.set(MarkerId::V1, 0.0));
        m.begin_sweep(500.0, 900.0);
        m.sweep_to(700.0, 950.0);
        assert_eq!(m.v, Some((100.0, 700.0)), "locked V1 stays put");
        m.toggle_lock(MarkerId::V1);
        assert_eq!(m.locked, None);
        assert!(m.set(MarkerId::H2, 1234.0));
        assert_eq!(m.active, Some(MarkerId::H2));

        m = Markers::default();
        assert!(!m.set(MarkerId::V1, 1.0), "pair is off");
        let r = m.readout(8000.0);
        assert!(r.s.is_none() && r.f_c.is_none());
    }
}
