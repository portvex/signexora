//! The Windows 95 progress bar (`msctls_progress32`): a thin sunken field
//! filled from the left with blocks in the selection colour, and SA's Stop
//! glyph that goes beside it.

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, canvas};

use super::colors::{Role, scheme};
use super::geom::{Edge, Rect};
use super::glyph::{Glyph, Ink};
use super::paint;
use super::widget::{place, size_of};

/// Width of one block.
const BLOCK: i32 = 8;
/// Gap between blocks.
const GAP: i32 = 2;

/// What a progress field shows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Progress {
    /// Nothing running: an empty field.
    #[default]
    Idle,
    /// Running without a known fraction: a short run of blocks that moves
    /// (`phase` counts the steps).
    Busy(u32),
    /// Running, this fraction done (0 to 1).
    Done(f32),
}

impl Progress {
    /// Something is running.
    pub(crate) fn is_running(self) -> bool {
        self != Self::Idle
    }
}

/// The blocks, as (left edge, width), inside a field whose inner area is `w`
/// pixels wide.
pub(crate) fn blocks(p: Progress, w: i32) -> Vec<(i32, i32)> {
    let step = BLOCK + GAP;
    let count = ((w + GAP) / step).max(0);
    let (first, n) = match p {
        Progress::Idle => (0, 0),
        Progress::Done(f) => {
            let f = if f.is_finite() {
                f.clamp(0.0, 1.0)
            } else {
                0.0
            };
            (0, (f * count as f32).round() as i32)
        }
        Progress::Busy(phase) => {
            let run = 4.min(count);
            let span = (count + run).max(1);
            // The modulo in u32 first: `phase as i32` would turn negative
            // after 2^31 steps (`span` is positive).
            ((phase % span as u32) as i32 - run, run)
        }
    };
    (first..first + n)
        .filter(|i| (0..count).contains(i))
        .map(|i| {
            let x = i * step;
            (x, BLOCK.min(w - x))
        })
        .collect()
}

/// The continuous fill of the styles from Vista on, as (left edge, width)
/// inside an inner area `w` pixels wide: the fraction done rounded to whole
/// pixels (nothing at 0, the whole width at 1), or for a busy field a
/// segment that moves as the classic run of blocks does. `None` when nothing
/// is filled.
pub(crate) fn fill_span(p: Progress, w: i32) -> Option<(i32, i32)> {
    let span = match p {
        Progress::Idle => return None,
        Progress::Done(f) => {
            let f = if f.is_finite() {
                f.clamp(0.0, 1.0)
            } else {
                0.0
            };
            (0, (f * w.max(0) as f32).round() as i32)
        }
        Progress::Busy(_) => {
            let run = blocks(p, w);
            let (first, last) = (run.first()?, run.last()?);
            (first.0, last.0 + last.1 - first.0)
        }
    };
    (span.1 > 0).then_some(span)
}

/// A progress field at `r`.
pub(crate) fn progress_field(cx: &App, r: Rect, p: Progress) -> AnyElement {
    let s = scheme(cx);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, _| {
            let (w, h) = size_of(b);
            let o = b.origin;
            let outer = Rect::new(0, 0, w, h);
            if super::skin::skin() != super::skin::Skin::Classic {
                super::skin::progress(window, o, outer, p);
                return;
            }
            paint::fill(window, o, outer, s.of(Role::Face));
            paint::edge(window, o, outer, Edge::ThinSunken, &s);
            let inner = outer.inset(2);
            for (x, bw) in blocks(p, inner.w) {
                let block = Rect::new(inner.x + x, inner.y, bw, inner.h);
                paint::fill(window, o, block, s.of(Role::Selection));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// SA's Stop (abort) glyph: a red circle with a bar across.
pub(crate) fn abort_glyph() -> Glyph {
    let red = Ink::Rgb(0xFF_00_00);
    let white = Ink::Role(Role::Face);
    let mut g = Glyph::new(16, 16)
        .circle(8.0, 8.0, 7.0, red)
        .circle(8.0, 8.0, 4.6, white);
    for d in 0..3 {
        g = g.line(4 + d, 11, 11, 4 + d, red);
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_fill_from_the_left() {
        assert_eq!(blocks(Progress::Idle, 100), [] as [(i32, i32); 0]);
        let half = blocks(Progress::Done(0.5), 100);
        assert_eq!(half.len(), 5);
        assert_eq!(half.first(), Some(&(0, 8)));
        assert_eq!(half.get(1), Some(&(10, 8)));
        assert_eq!(blocks(Progress::Done(1.0), 100).len(), 10);
        assert_eq!(blocks(Progress::Done(f32::NAN), 100).len(), 0);
    }

    #[test]
    fn busy_blocks_move_and_stay_inside() {
        let a = blocks(Progress::Busy(4), 100);
        let b = blocks(Progress::Busy(5), 100);
        assert_ne!(a, b);
        for phase in 0..40 {
            for (x, w) in blocks(Progress::Busy(phase), 100) {
                assert!(x >= 0 && x + w <= 100);
            }
        }
        for phase in [u32::MAX, 1 << 31, (1 << 31) + 3] {
            for (x, w) in blocks(Progress::Busy(phase), 100) {
                assert!(x >= 0 && x + w <= 100, "{phase}");
            }
        }
        assert!(Progress::Busy(0).is_running());
        assert!(!Progress::Idle.is_running());
    }

    #[test]
    fn continuous_fill_is_empty_at_zero_and_full_at_one() {
        assert_eq!(fill_span(Progress::Idle, 98), None);
        assert_eq!(fill_span(Progress::Done(0.0), 98), None);
        assert_eq!(fill_span(Progress::Done(f32::NAN), 98), None);
        assert_eq!(fill_span(Progress::Done(1.0), 98), Some((0, 98)));
        assert_eq!(fill_span(Progress::Done(0.5), 98), Some((0, 49)));
        assert_eq!(fill_span(Progress::Done(0.5), 0), None);
        // Busy: a moving segment, not a fill from the left edge.
        let starts: Vec<i32> = (0..14)
            .filter_map(|ph| fill_span(Progress::Busy(ph), 98))
            .map(|(x, w)| {
                assert!(x >= 0 && x + w <= 98 && w <= 38, "({x}, {w})");
                x
            })
            .collect();
        assert!(starts.iter().any(|&x| x > 0), "{starts:?}");
        assert_eq!(
            fill_span(Progress::Busy(5), 98),
            Some((10, 38)),
            "four blocks from the second"
        );
    }
}
