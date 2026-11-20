//! The generator's preview plot: the mean spectrum (80 dB below its peak at
//! the bottom) or the first samples of I and Q, drawn one pixel column at a
//! time in the plot colours, with a caption at the top left; or a note when
//! there is no preview (`sa-app`'s `draw_spectrum` and `draw_wave`).

use std::sync::Arc;

use gpui::{AnyElement, Context, Hsla, IntoElement, ParentElement, Styled, Window, canvas};
use sn_ui_core::generator::{NoPreview, Preview, trim};
use sn_ui_core::t;

use crate::app::Frame;
use crate::win95::colors::{hsla, palette, rgb_of, scheme};
use crate::win95::geom::{Edge, Rect};
use crate::win95::paint;
use crate::win95::text::{self, LINE};
use crate::win95::widget::{place, size_of};

/// What the plot shows.
enum Shown {
    Note(String, bool),
    Spectrum(Arc<Preview>),
    Wave(Arc<Preview>),
}

/// The plot's colours.
#[derive(Clone, Copy)]
struct Inks {
    bg: Hsla,
    grid: Hsla,
    axis: Hsla,
    dim: Hsla,
    error: Hsla,
    i: Hsla,
    q: Hsla,
}

impl Frame {
    /// The preview plot at `r`.
    pub(crate) fn preview_view(&mut self, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        let valid = self.tools.problem().is_none();
        let g = &self.tools.ocg;
        let shown = match &g.preview {
            _ if !valid => Shown::Note(t!("gen-preview-invalid"), false),
            None => Shown::Note(t!("gen-preview-building"), false),
            Some((_, Err(NoPreview::Failed(e)))) => {
                Shown::Note(t!("gen-preview-failed", error = e.clone()), true)
            }
            Some((_, Err(NoPreview::TooLong(note)))) => Shown::Note(note.clone(), false),
            Some((_, Ok(p))) if g.show_wave => Shown::Wave(p.clone()),
            Some((_, Ok(p))) => Shown::Spectrum(p.clone()),
        };
        let pal = palette(cx);
        let inks = Inks {
            bg: hsla(rgb_of(pal.plot_bg)),
            grid: hsla(rgb_of(pal.grid)),
            axis: hsla(rgb_of(pal.axis)),
            dim: hsla(rgb_of(pal.plot_text_dim)),
            error: hsla(rgb_of(pal.error)),
            i: hsla(rgb_of(pal.trace_i)),
            q: hsla(rgb_of(pal.trace_q)),
        };
        let s = scheme(cx);
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let (w, h) = size_of(b);
                let all = Rect::new(0, 0, w, h);
                paint::fill(window, b.origin, all, inks.bg);
                paint::edge(window, b.origin, all, Edge::Sunken, &s);
                let plot = all.inset(2);
                let clip = paint::bounds(b.origin, plot);
                window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
                    draw(window, cx, b.origin, plot, &shown, inks);
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        place(r).child(paint).into_any_element()
    }
}

fn draw(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    r: Rect,
    shown: &Shown,
    inks: Inks,
) {
    match shown {
        Shown::Note(note, error) => {
            let tw = text::width(window, note, false);
            let at = paint::at(o, r.x + (r.w - tw).max(0) / 2, r.y + (r.h - LINE) / 2);
            let ink = if *error { inks.error } else { inks.dim };
            text::draw(window, cx, at, note, false, ink);
        }
        Shown::Spectrum(p) => {
            let caption = if p.spectrum.len() < 2 {
                t!("gen-preview-short")
            } else {
                spectrum(window, o, r, p, inks);
                t!(
                    "gen-preview-axis",
                    lo = trim(p.lo_hz, 1),
                    hi = trim(p.hi_hz, 1)
                )
            };
            text::draw(
                window,
                cx,
                paint::at(o, r.x + 4, r.y + 2),
                &caption,
                false,
                inks.dim,
            );
        }
        Shown::Wave(p) => {
            let peak = wave(window, o, r, p, inks);
            let caption = t!(
                "gen-preview-wave-axis",
                ms = trim(p.wave_seconds * 1e3, 2),
                peak = trim(f64::from(peak), 4)
            );
            text::draw(
                window,
                cx,
                paint::at(o, r.x + 4, r.y + 2),
                &caption,
                false,
                inks.dim,
            );
        }
    }
}

/// The y of each column of the spectrum (`top` dB at the top, 80 dB below
/// at the bottom).
fn spectrum_ys(spec: &[f32], top: f32, r: Rect) -> Vec<i32> {
    const RANGE_DB: f32 = 80.0;
    let n = spec.len();
    let cols = r.w.max(1) as usize;
    (0..cols)
        .map(|c| {
            let a = c * n / cols;
            let b = ((c + 1) * n / cols).max(a + 1).min(n);
            let v = spec
                .get(a..b)
                .unwrap_or(&[])
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .fold(f32::NEG_INFINITY, f32::max);
            let rel = ((top - v) / RANGE_DB).clamp(0.0, 1.0);
            let rel = if rel.is_finite() { rel } else { 1.0 };
            r.y + 2 + (rel * (r.h - 4) as f32).round() as i32
        })
        .collect()
}

fn spectrum(window: &mut Window, o: gpui::Point<gpui::Pixels>, r: Rect, p: &Preview, inks: Inks) {
    for k in 1..4 {
        let y = r.y + r.h * k / 4;
        paint::fill(window, o, Rect::new(r.x, y, r.w, 1), inks.grid);
    }
    let top = p
        .spectrum
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);
    let top = if top.is_finite() { top } else { 0.0 };
    let ys = spectrum_ys(&p.spectrum, top, r);
    let mut prev: Option<i32> = None;
    for (c, y) in (0_i32..).zip(ys) {
        let (a, b) = match prev {
            Some(p) => (p.min(y), p.max(y)),
            None => (y, y),
        };
        paint::fill(window, o, Rect::new(r.x + c, a, 1, b - a + 1), inks.i);
        prev = Some(y);
    }
}

/// Draw the waveform; returns the peak it is scaled to.
fn wave(
    window: &mut Window,
    o: gpui::Point<gpui::Pixels>,
    r: Rect,
    p: &Preview,
    inks: Inks,
) -> f32 {
    let peak = p
        .wave_i
        .iter()
        .chain(p.wave_q.iter().flatten())
        .fold(
            0.0_f32,
            |a, v| if v.is_finite() { a.max(v.abs()) } else { a },
        )
        .max(1e-9);
    let mid = r.y + r.h / 2;
    let half = (r.h / 2 - 2) as f32;
    paint::fill(window, o, Rect::new(r.x, mid, r.w, 1), inks.axis);
    let y_of = |s: f32| mid - (s / peak * half).round() as i32;
    let mut draw = |v: &[f32], ink: Hsla| {
        let n = v.len();
        if n == 0 {
            return;
        }
        let cols = r.w.max(1) as usize;
        let mut prev: Option<(i32, i32)> = None;
        for c in 0..cols {
            let a = c * n / cols;
            let b = ((c + 1) * n / cols).max(a + 1).min(n);
            let (lo, hi) = v
                .get(a..b)
                .unwrap_or(&[])
                .iter()
                .filter(|s| s.is_finite())
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(l, h), s| {
                    (l.min(*s), h.max(*s))
                });
            if !(lo.is_finite() && hi.is_finite()) {
                continue;
            }
            let (mut y1, mut y2) = (y_of(hi), y_of(lo));
            if let Some((p1, p2)) = prev {
                // Join the columns where the trace steps.
                y1 = y1.min(p2);
                y2 = y2.max(p1);
            }
            paint::fill(
                window,
                o,
                Rect::new(r.x + c as i32, y1, 1, y2 - y1 + 1),
                ink,
            );
            prev = Some((y_of(hi), y_of(lo)));
        }
    };
    if let Some(q) = &p.wave_q {
        draw(q, inks.q);
    }
    draw(&p.wave_i, inks.i);
    peak
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_peak_is_at_the_top_and_minus_80_db_at_the_bottom() {
        let r = Rect::new(0, 0, 4, 84);
        let ys = spectrum_ys(&[0.0, -40.0, -80.0, f32::NAN], 0.0, r);
        assert_eq!(ys, vec![2, 42, 82, 82]);
    }
}
