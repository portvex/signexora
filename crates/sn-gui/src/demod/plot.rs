//! The demodulator's plot (`docs/SA_UI_SPEC.md` 8): black, one page of 100
//! symbols folded one symbol across, the points coloured by their distance
//! from the level centre (white, yellow, green), white level lines, blue
//! triangle handles on the main markers (dragged up and down), level numbers
//! from blue (lowest) to red (highest), and the vertical scroll bar that moves
//! the shown range while zoomed. Right click: the popup menu.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, Hsla, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseUpEvent, ParentElement, Styled, Window, canvas,
};
use sn_core::NodeId;
use sn_ui_core::demod::{DemodState, PAGE, boundaries};
use sn_ui_core::{format, t};

use super::colors::{ModColors, blend};
use super::ctl::draw_sized;
use super::{Frame, Layout};
use crate::win95::colors::{Rgb, hsla, scheme};
use crate::win95::geom::Rect;
use crate::win95::glyph::{self, Glyph, Mode};
use crate::win95::kit::{Capture, action, lens};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, place};

/// Pixels from a main marker that grab it.
const GRAB: f32 = 8.0;
/// Thinnest zone that still gets its level number.
const MIN_BAND: f32 = 12.0;

/// What the plot's paint callback draws (plot coordinates).
#[derive(Clone, Debug, Default)]
pub(crate) struct Scene {
    pub(crate) points: Vec<(f32, f32, Hsla)>,
    /// Level boundary lines, bottom to top.
    pub(crate) lines: Vec<f32>,
    /// Level numbers: centre y, text, colour.
    pub(crate) numbers: Vec<(f32, String, Rgb)>,
    pub(crate) info: Option<String>,
    /// Why nothing is recomputed (an invalid cutoff).
    pub(crate) waiting: Option<String>,
}

/// The y of value `v` in a plot `h` high showing `(lo, hi)`.
fn y_of(v: f64, (lo, hi): (f64, f64), h: f32) -> f32 {
    h - ((v - lo) / (hi - lo).max(1e-12)) as f32 * h
}

/// The value at height `y` of a plot `h` high showing `(lo, hi)`.
pub(crate) fn value_at(y: f32, (lo, hi): (f64, f64), h: f32) -> f64 {
    lo + f64::from((h - y) / h.max(1.0)) * (hi - lo)
}

/// Build the scene of a plot `(w, h)` for `st`.
pub(crate) fn scene(st: &DemodState, (w, h): (f32, f32), mc: &ModColors) -> Scene {
    let range = st.shown_range();
    let mut sc = Scene::default();
    if let (Some(d), Some(track)) = (st.result(), st.track()) {
        let period = track.fs / d.measured_baud.max(1e-9);
        let stride = ((period / 48.0).ceil() as usize).max(1);
        for inst in d.instants.iter().skip(st.page * PAGE).take(PAGE) {
            let a = (inst - period / 2.0).ceil().max(0.0) as usize;
            let b = (inst + period / 2.0).floor().max(0.0) as usize;
            for i in (a..=b).step_by(stride) {
                let Some(v) = track.values.get(i).map(|v| f64::from(*v)) else {
                    break;
                };
                let x = (((i as f64 - inst) / period + 0.5) as f32) * w;
                let (_, m) = d.layout.margin(v);
                let color = match m.abs() {
                    m if m < 0.3 => mc.point_core,
                    m if m < 0.6 => mc.point_mid,
                    _ => mc.point_edge,
                };
                sc.points.push((x, y_of(v, range, h), color));
            }
        }
        let lines = boundaries(d.layout.centres());
        sc.lines = lines.iter().map(|v| y_of(*v, range, h)).collect();
        let band = sc
            .lines
            .windows(2)
            .map(|p| match p {
                [a, b] => (a - b).abs(),
                _ => 0.0,
            })
            .fold(f32::INFINITY, f32::min);
        let n = d.layout.order();
        if band >= MIN_BAND {
            for (k, c) in d.layout.centres().iter().enumerate() {
                let f = if n > 1 {
                    k as f64 / (n - 1) as f64
                } else {
                    0.0
                };
                let color = blend(mc.level_bottom, mc.level_top, f);
                sc.numbers.push((y_of(*c, range, h), k.to_string(), color));
            }
        }
        if st.info {
            sc.info = Some(t!(
                "demod-info",
                baud = format::dec(d.measured_baud, 3),
                clock = format::dec(d.clock_error * 100.0, 4),
                quality = format::dec(d.quality, 2),
                eye = format::dec(d.eye_opening, 2),
                count = d.symbols.len()
            ));
        }
    }
    sc.waiting.clone_from(&st.cutoff_error);
    sc
}

/// A main marker handle: a solid triangle on the left edge pointing right.
fn handle() -> Glyph {
    Glyph::new(8, 11).poly(&[(0.0, 0.0), (7.0, 5.5), (0.0, 11.0)], glyph::BLACK)
}

/// Paint the plot.
fn paint_plot(
    window: &mut Window,
    cx: &mut gpui::App,
    b: gpui::Bounds<gpui::Pixels>,
    sc: &Scene,
    mc: &ModColors,
) {
    let (w, h) = crate::win95::widget::size_of(b);
    let o = b.origin;
    let s = scheme(cx);
    window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
        paint::fill(window, o, Rect::new(0, 0, w, h), mc.plot_bg);
        for (x, y, c) in &sc.points {
            let r = Rect::new(*x as i32 - 1, *y as i32 - 1, 2, 2);
            paint::fill(window, o, r, *c);
        }
        for y in &sc.lines {
            paint::fill(window, o, Rect::new(0, *y as i32, w, 1), mc.level_line);
        }
        let hd = handle();
        for y in [sc.lines.first(), sc.lines.last()].into_iter().flatten() {
            let at = (0, *y as i32 - 5);
            glyph::draw(window, cx, o, at, &hd, Mode::Mono(mc.level_handle), &s);
        }
        for (y, n, c) in &sc.numbers {
            let tw = text::width(window, n, true);
            let at = paint::at(o, w - 30 - tw / 2, *y as i32 - LINE / 2);
            text::draw(window, cx, at, n, true, hsla(*c));
        }
        if let Some(info) = &sc.info {
            text::draw(
                window,
                cx,
                paint::at(o, 10, h - LINE - 4),
                info,
                false,
                mc.text,
            );
        }
        if let Some(e) = &sc.waiting {
            // The result shown was made with the old settings: dimmed, with
            // the reason written over it.
            let mut dim = mc.plot_bg;
            dim.a = 0.75;
            paint::fill(window, o, Rect::new(0, 0, w, h), dim);
            let (cx_, cy) = (w / 2, h / 2);
            let note = t!("demod-waiting-cutoff");
            draw_sized(window, cx, o, (cx_, cy - 16), &note, 12.5, true, mc.text);
            draw_sized(window, cx, o, (cx_, cy + 2), e, 12.5, true, mc.error);
        }
    });
}

impl Frame {
    /// The plot and its vertical scroll bar of demodulator window `win`.
    pub(crate) fn demod_plot(
        &mut self,
        win: u32,
        node: NodeId,
        l: Layout,
        _window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let mc = self.dm_colors(cx);
        let d = self.demod_win(node);
        let sc = scene(&d.st, (l.plot.w as f32, l.plot.h as f32), &mc);
        d.vbar = vbar_state(&d.st);
        let vbar = d.vbar;
        let slot = Slot::default();
        let paint_slot = slot.clone();
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                paint_slot.set(b);
                paint_plot(window, cx, b, &sc, &mc);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let s2 = slot.clone();
        let plot = place(l.plot)
            .id(gpui::ElementId::Name(format!("dm{win}.plot").into()))
            .child(paint)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.demod_plot_down(node, ev, &slot);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseUpEvent, _, cx| {
                    if s2.local(ev.position).is_some() {
                        let at = (
                            f32::from(ev.position.x).round() as i32,
                            f32::from(ev.position.y).round() as i32,
                        );
                        v.open_demod_plot_menu(win, node, at);
                    }
                    cx.notify();
                }),
            );
        let id = format!("dm{win}.vbar");
        let bar = scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect: l.vbar,
                vertical: true,
            },
            vbar,
            lens(move |v: &mut Self| v.demod.nodes.get_mut(&node).map(|d| &mut d.vbar)),
            Some(action(move |v: &mut Self, _, _| {
                let d = v.demod_win(node);
                let pos = d.vbar.pos;
                vbar_moved(&mut d.st, pos);
            })),
        );
        vec![plot.into_any_element(), bar]
    }

    /// Left button down in the plot: grab the nearer main marker within
    /// [`GRAB`] pixels and drag it.
    fn demod_plot_down(&mut self, node: NodeId, ev: &MouseDownEvent, slot: &Slot) {
        self.kit.focus = None;
        let Some((_, y)) = slot.local(ev.position) else {
            return;
        };
        let d = self.demod_win(node);
        if d.st.from_tones {
            return;
        }
        let Some((b, t)) = d.st.markers else {
            return;
        };
        let h = d.plot.h as f32;
        let range = d.st.shown_range();
        let y = y as f32;
        let (db, dt) = ((y - y_of(b, range, h)).abs(), (y - y_of(t, range, h)).abs());
        if db.min(dt) > GRAB {
            return;
        }
        let top = dt < db;
        let slot = slot.clone();
        // The range follows the markers, so it is frozen for the drag: else
        // every move would rescale the plot under the pointer and the marker
        // would run away. It follows them again once the button is released.
        let was = d.st.view;
        d.st.view = Some(range);
        self.kit.capture(Capture {
            on_move: Some(Rc::new(move |v: &mut Self, at, _, _| {
                let Some((_, y)) = slot.local(at) else {
                    return;
                };
                let d = v.demod_win(node);
                let val = value_at(y as f32, range, d.plot.h as f32);
                d.st.drag_marker(top, val, range);
            })),
            on_up: Some(Rc::new(move |v: &mut Self, _, _, _| {
                v.demod_win(node).st.view = was;
            })),
        });
    }
}

/// The vertical scroll bar for the shown range: units of a tenth of the
/// shown height, the page is the plot.
pub(crate) fn vbar_state(st: &DemodState) -> ScrollState {
    let (b0, b1) = st.base_range();
    let (lo, hi) = st.shown_range();
    let unit = ((hi - lo) / 10.0).max(1e-12);
    let total = ((b1 - b0) / unit).round().max(10.0) as i32;
    let pos = ((b1 - hi).max(0.0) / unit).round() as i32;
    ScrollState::new(0, total - 1, 10, pos)
}

/// The vertical scroll bar moved to `pos`: show the range there.
pub(crate) fn vbar_moved(st: &mut DemodState, pos: i32) {
    let (_, b1) = st.base_range();
    let (lo, hi) = st.shown_range();
    let unit = (hi - lo) / 10.0;
    let top = b1 - f64::from(pos) * unit;
    st.view = Some((top - (hi - lo), top));
}

#[cfg(test)]
mod tests {
    use super::*;
    use sn_ui_core::demod::ZOOM;

    fn state() -> DemodState {
        let mut st = DemodState::default();
        st.markers = Some((-1.0, 3.0));
        st
    }

    #[test]
    fn the_full_range_cannot_scroll() {
        let st = state();
        assert!(!vbar_state(&st).enabled());
    }

    #[test]
    fn the_scroll_bar_round_trips_and_reaches_both_ends() {
        let mut st = state();
        st.zoom(1.0 / ZOOM);
        st.zoom(1.0 / ZOOM);
        let (lo, hi) = st.shown_range();
        let unit = (hi - lo) / 10.0;
        let bar = vbar_state(&st);
        assert!(bar.enabled());
        vbar_moved(&mut st, bar.pos);
        let (lo2, hi2) = st.shown_range();
        assert!((lo2 - lo).abs() <= unit / 2.0 + 1e-9, "{lo} -> {lo2}");
        assert!(((hi2 - lo2) - (hi - lo)).abs() < 1e-9, "the height stays");
        // The bar's top shows the top of the base range, its end the bottom.
        let (b0, b1) = st.base_range();
        vbar_moved(&mut st, 0);
        assert!((st.shown_range().1 - b1).abs() < 1e-9);
        let bar = vbar_state(&st);
        vbar_moved(&mut st, bar.top());
        assert!(
            (st.shown_range().0 - b0).abs() <= unit,
            "{:?}",
            st.shown_range()
        );
    }
}
