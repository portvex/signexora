//! The left column of `SA-OFDM` (`docs/SA_UI_SPEC.md` 10): in Survey the
//! 1024-point spectrum (green) of the window at the cyan scroll bar's
//! position; after Find CT the correlation triangle (green) with the yellow
//! LU marker on the good slope and the cyan info block, a blue line, then the
//! LU spectrum with even channels cyan and odd ones blue-purple on a small
//! pedestal and the green (selected) and red channel markers, purple on one
//! channel. The vertical scroll bar sets the spectrum gain, the cyan one the
//! Survey position or the symbol shown, the lavender one the spectrum pan.
//! Long work shows `Don't panic! Please wait...` over the plots.

use gpui::{
    AnyElement, Context, Hsla, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Styled, Window, canvas,
};
use sn_core::NodeId;
use sn_ui_core::ofdm::{
    OfdmState, SURVEY_FFT, Work, default_pan, pitch, survey_db, wait_note, wait_progress,
};
use sn_ui_core::t;

use super::{Frame, Layout};
use crate::app::demod::colors::ModColors;
use crate::app::demod::ctl::{CELL_H, draw_mono, draw_sized, polyline};
use crate::win95::geom::Rect;
use crate::win95::kit::{action, lens};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, scroll_bar, tinted_scroll_bar};
use crate::win95::widget::{Slot, place, size_of};

/// What the plots draw.
#[derive(Clone, Debug)]
enum Scene {
    /// Survey: bar heights (0..1 of the plot), left to right.
    Survey(Vec<f32>),
    /// After Find CT.
    Grid(Box<GridScene>),
}

/// Spectrum bars: left x, height 0..1, colour, marker colour above.
type Bars = Vec<(f32, f32, Hsla, Option<Hsla>)>;

/// The triangle and spectrum views (plot coordinates of their own rects).
#[derive(Clone, Debug)]
struct GridScene {
    /// The triangle (0..1 across, 0..1 up), or why it is missing.
    curve: Result<Vec<(f32, f32)>, String>,
    /// The LU marker (0..1 across).
    lu: Option<f32>,
    info: Vec<String>,
    /// Spectrum bars, or why they are missing.
    bars: Result<Bars, String>,
    bar_w: f32,
}

/// The wait screen: the note, the remaining time and the fraction done.
#[derive(Clone, Debug)]
struct Wait {
    note: String,
    left: Option<String>,
    frac: f32,
}

/// The Survey bars of `st` on `signal` from sample `start`.
fn survey_scene(signal: &sn_core::Signal, start: usize) -> Vec<f32> {
    let db = survey_db(signal, start);
    let top = db
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(-200.0_f32, f32::max);
    db.iter()
        .map(|v| ((v - top + 80.0) / 80.0).clamp(0.0, 1.0) * 0.6)
        .collect()
}

/// The spectrum's pan in pixels: the stored one (or the default for `bins`
/// bins of `p` pixels), never past the last full view of a spectrum `width`
/// pixels wide. Drawing, clicks and the scroll bar all use it, so they agree
/// after Scale FFT is lowered or the window shrinks.
pub(crate) fn effective_pan(
    pan: Option<f64>,
    signal: &sn_core::Signal,
    bins: usize,
    p: f64,
    width: f64,
) -> f64 {
    let max = (bins as f64 * p - width).max(0.0);
    pan.unwrap_or_else(|| default_pan(signal, bins, p))
        .clamp(0.0, max)
}

/// The triangle and spectrum of `st` for a spectrum `width` pixels wide.
fn grid_scene(
    st: &mut OfdmState,
    signal: &sn_core::Signal,
    width: f32,
    mc: &ModColors,
) -> GridScene {
    let mut g = GridScene {
        curve: Ok(Vec::new()),
        lu: None,
        info: st.info_lines(signal.sample_rate()),
        bars: Ok(Vec::new()),
        bar_w: 1.0,
    };
    let back = st.lu_back;
    g.curve = match st.triangle() {
        Some(Ok(t)) => {
            let n = t.signed.len().max(2);
            let (lo, hi) = t
                .signed
                .iter()
                .fold((0.0_f32, 0.0_f32), |(a, b), v| (a.min(*v), b.max(*v)));
            let range = (hi - lo).max(1e-9);
            let pts = t
                .signed
                .iter()
                .enumerate()
                .map(|(i, v)| (i as f32 / (n - 1) as f32, (v - lo) / range))
                .collect();
            let b = back.unwrap_or(t.lg / 2).min(t.lg);
            let m = (t.peak + t.ls - b) % t.ls.max(1);
            g.lu = Some(m as f32 / (n - 1) as f32);
            Ok(pts)
        }
        Some(Err(e)) => Err(e.to_owned()),
        None => Err(t!("ofdm-plot-empty")),
    };
    let (scale, gain, sel, other, pan) = (st.scale_fft, st.gain, st.sel, st.other, st.pan);
    let p = pitch(scale);
    g.bar_w = (p as f32 - 1.0).max(1.0);
    g.bars = match st.spectrum() {
        Some(Ok(spec)) => {
            let pan = effective_pan(pan, signal, spec.bins.len(), p, f64::from(width));
            let peak = spec
                .values
                .iter()
                .map(|v| v.norm())
                .fold(0.0_f32, f32::max)
                .max(1e-12);
            let mut bars = Vec::new();
            for (i, (bin, v)) in spec.bins.iter().zip(&spec.values).enumerate() {
                let x = (i as f64 * p - pan) as f32;
                if x + g.bar_w < 0.0 || x > width {
                    continue;
                }
                let h = (f64::from(v.norm() / peak) * gain).min(1.0) as f32;
                let color = if bin.rem_euclid(2) == 0 {
                    mc.even
                } else {
                    mc.odd
                };
                let mark = match (*bin == sel, *bin == other) {
                    (true, true) => Some(mc.mark_both),
                    (true, false) => Some(mc.mark_selected),
                    (false, true) => Some(mc.mark_other),
                    (false, false) => None,
                };
                bars.push((x, h, color, mark));
            }
            Ok(bars)
        }
        Some(Err(e)) => Err(e.to_owned()),
        None => Err(t!("ofdm-plot-empty")),
    };
    g
}

/// Paint a message in the middle of an empty plot.
fn message(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    (w, h): (i32, i32),
    s: &str,
    c: Hsla,
) {
    draw_sized(window, cx, o, (w / 2, h / 2 - 7), s, 12.5, true, c);
}

/// Paint the triangle view in `r`.
fn paint_triangle(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    r: Rect,
    g: &GridScene,
    mc: &ModColors,
) {
    paint::fill(window, o, r, mc.plot_bg);
    let o2 = paint::at(o, r.x, r.y);
    match &g.curve {
        Ok(pts) => {
            let (w, h) = (r.w as f32, r.h as f32);
            let line: Vec<(f32, f32)> = pts
                .iter()
                .map(|(x, y)| (x * w, h - 4.0 - y * (h - 12.0)))
                .collect();
            polyline(window, o2, &line, mc.curve);
            if let Some(m) = g.lu {
                let x = (m * w).round() as i32;
                let top = 2;
                let bottom = (h * 0.6) as i32;
                paint::fill(
                    window,
                    o2,
                    Rect::new(x - 1, top, 3, bottom - top),
                    mc.lu_marker,
                );
            }
        }
        Err(e) => message(window, cx, o2, (r.w, r.h), e, mc.error),
    }
    for (i, l) in (0_i32..).zip(&g.info) {
        draw_mono(window, cx, o2, (4, 2 + i * CELL_H), l, mc.text);
    }
}

/// Paint the spectrum view in `r`.
fn paint_spectrum(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    r: Rect,
    g: &GridScene,
    mc: &ModColors,
) {
    paint::fill(window, o, r, mc.plot_bg);
    let o2 = paint::at(o, r.x, r.y);
    match &g.bars {
        Ok(bars) => {
            let usable = (r.h - 14) as f32;
            let bw = g.bar_w.round().max(1.0) as i32;
            for (x, hf, c, mark) in bars {
                let h = 3 + (hf * usable) as i32;
                let x = x.round() as i32;
                paint::fill(window, o2, Rect::new(x, r.h - h, bw, h), *c);
                if let Some(m) = mark {
                    paint::fill(window, o2, Rect::new(x, 0, bw, r.h - h), *m);
                }
            }
        }
        Err(e) => message(window, cx, o2, (r.w, r.h), e, mc.error),
    }
}

/// Paint the wait screen over `r`.
fn paint_wait(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    r: Rect,
    w: &Wait,
    mc: &ModColors,
) {
    paint::fill(window, o, r, mc.plot_bg);
    let c = r.x + r.w / 2;
    let y = |f: f32| r.y + (r.h as f32 * f) as i32;
    draw_sized(
        window,
        cx,
        o,
        (c, y(0.2) - 14),
        &t!("ofdm-wait"),
        26.0,
        true,
        mc.wait_text,
    );
    draw_sized(
        window,
        cx,
        o,
        (c, y(0.5) - 8),
        &w.note,
        15.0,
        true,
        mc.wait_note,
    );
    if let Some(left) = &w.left {
        draw_sized(
            window,
            cx,
            o,
            (c, y(0.72) - 10),
            left,
            18.0,
            true,
            mc.wait_text,
        );
    }
    // SA's progress: a row of green blocks.
    let bar_w = r.w - 8;
    let blocks = (bar_w / 8).max(0);
    let n = (blocks as f32 * w.frac) as i32;
    for i in 0..n {
        let b = Rect::new(r.x + 4 + i * 8, r.bottom() - 14, 6, 10);
        paint::fill(window, o, b, mc.wait_bar);
    }
}

impl Frame {
    /// The plots and their three scroll bars.
    pub(crate) fn ofdm_plots(
        &mut self,
        win: u32,
        node: NodeId,
        l: Layout,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return Vec::new();
        };
        let span = self.ofdm_span(node, signal.len());
        let mc = self.dm_colors(cx);
        let (tri_r, spec_r) = l.split();
        let o = self.ofdm_win(node);
        let scene = if o.st.surveying() {
            Scene::Survey(survey_scene(
                &signal,
                span.0 + o.st.survey_pos.max(0.0) as usize,
            ))
        } else {
            Scene::Grid(Box::new(grid_scene(
                &mut o.st,
                &signal,
                spec_r.w as f32,
                &mc,
            )))
        };
        let wait =
            o.st.running()
                .filter(|(w, ..)| *w != Work::Show)
                .map(|(w, secs, p)| {
                    let (left, frac) = wait_progress(secs, p);
                    Wait {
                        note: wait_note(w),
                        left,
                        frac,
                    }
                });
        self.sync_ofdm_bars(node, span.1.saturating_sub(span.0), &signal, l);
        let p = l.plots;
        let (tri, spec) = (
            Rect::new(0, 0, tri_r.w, tri_r.h),
            Rect::new(0, tri_r.h, spec_r.w, spec_r.h),
        );
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let o = b.origin;
                let (w, h) = size_of(b);
                window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
                    match &scene {
                        Scene::Survey(bars) => {
                            paint::fill(window, o, Rect::new(0, 0, w, h), mc.plot_bg);
                            let n = bars.len().max(2);
                            for (i, f) in bars.iter().enumerate() {
                                let x = (i as f32 / (n - 1) as f32 * w as f32) as i32;
                                let bh = (f * h as f32) as i32;
                                paint::fill(window, o, Rect::new(x, h - bh, 1, bh), mc.survey);
                            }
                        }
                        Scene::Grid(g) => {
                            paint_triangle(window, cx, o, tri, g, &mc);
                            paint_spectrum(window, cx, o, spec, g, &mc);
                            paint::fill(window, o, Rect::new(0, tri.h - 1, w, 2), mc.divider);
                        }
                    }
                    if let Some(wt) = &wait {
                        paint_wait(window, cx, o, Rect::new(0, 0, w, h), wt, &mc);
                    }
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let slot = Slot::default();
        let s2 = slot.clone();
        let measure = canvas(move |b, _, _| s2.set(b), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let plots = place(p)
            .id(gpui::ElementId::Name(format!("of{win}.plots").into()))
            .child(paint)
            .child(measure)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.ofdm_plot_click(node, ev, &slot, l);
                    cx.notify();
                }),
            );
        let mut out = vec![plots.into_any_element()];
        out.extend(self.ofdm_bars(win, node, l, &mc, cx));
        out
    }

    /// A click in the plots: on the triangle it places the LU marker; on the
    /// spectrum it moves the nearer channel marker (a double click both).
    fn ofdm_plot_click(&mut self, node: NodeId, ev: &MouseDownEvent, slot: &Slot, l: Layout) {
        self.kit.focus = None;
        let Some((x, y)) = slot.local(ev.position) else {
            return;
        };
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let (tri, _) = l.split();
        let st = &mut self.ofdm_win(node).st;
        if st.surveying() || st.is_running() {
            return;
        }
        if y < tri.h {
            let n = match st.triangle() {
                Some(Ok(t)) => t.signed.len().max(2),
                _ => return,
            };
            let i = (x as f32 / tri.w.max(1) as f32 * (n - 1) as f32)
                .round()
                .max(0.0) as usize;
            st.place_lu(i);
            return;
        }
        let p = pitch(st.scale_fft);
        let pan_px = st.pan;
        let bins = match st.spectrum() {
            Some(Ok(s)) => s.bins.clone(),
            _ => return,
        };
        let (_, spec_r) = l.split();
        let pan = effective_pan(pan_px, &signal, bins.len(), p, f64::from(spec_r.w));
        let i = ((f64::from(x) + pan) / p).floor();
        if i < 0.0 {
            return;
        }
        if let Some(b) = bins.get(i as usize).copied() {
            st.click_channel(b, ev.click_count >= 2);
        }
    }

    /// The scroll bars from the state.
    fn sync_ofdm_bars(&mut self, node: NodeId, len: usize, signal: &sn_core::Signal, l: Layout) {
        let o = self.ofdm_win(node);
        let st = &o.st;
        let g = ((50.0 - 12.5 * st.gain.log2()).clamp(0.0, 100.0) / 2.0).round() as i32;
        o.gain_bar = ScrollState::new(0, 50 + 6 - 1, 6, g);
        if st.surveying() {
            let unit = (SURVEY_FFT / 4) as f64;
            let max = len.saturating_sub(SURVEY_FFT) as f64;
            let units = (max / unit).ceil() as i32;
            o.pos_unit = unit;
            o.survey_max = max;
            o.pos_bar =
                ScrollState::new(0, units + 32 - 1, 32, (st.survey_pos / unit).round() as i32);
        } else {
            let max = st.symbol_count().saturating_sub(1) as i32;
            o.pos_unit = 1.0;
            o.pos_bar = ScrollState::new(0, max + 10 - 1, 10, st.symbol as i32);
        }
        let width = f64::from(l.plots.w.max(1));
        match st.grid().filter(|_| !st.survey) {
            Some(g) => {
                let p = pitch(st.scale_fft);
                let unit = p * 4.0;
                let total = g.bins().len() as f64 * p;
                let max = (total - width).max(0.0);
                let page = (width / unit).floor().max(1.0) as i32;
                let pan = effective_pan(st.pan, signal, g.bins().len(), p, width);
                o.pan_unit = unit;
                o.pan_max = max;
                o.pan_bar = ScrollState::new(
                    0,
                    (max / unit).ceil() as i32 + page - 1,
                    page,
                    (pan / unit).round() as i32,
                );
            }
            None => o.pan_bar = ScrollState::new(0, 0, 1, 0),
        }
    }

    /// The gain, position (cyan) and pan (lavender) scroll bars.
    fn ofdm_bars(
        &mut self,
        win: u32,
        node: NodeId,
        l: Layout,
        mc: &ModColors,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(o) = self.ofdm.nodes.get(&node) else {
            return Vec::new();
        };
        let (gain, pos, pan) = (o.gain_bar, o.pos_bar, o.pan_bar);
        let gid = format!("of{win}.gain");
        let pid = format!("of{win}.pos");
        let nid = format!("of{win}.pan");
        let gain_el = scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &gid,
                rect: l.gain,
                vertical: true,
            },
            gain,
            lens(move |v: &mut Self| v.ofdm.nodes.get_mut(&node).map(|o| &mut o.gain_bar)),
            Some(action(move |v: &mut Self, _, _| {
                let o = v.ofdm_win(node);
                o.st.gain = 2f64.powf((50.0 - 2.0 * f64::from(o.gain_bar.pos)) / 12.5);
            })),
        );
        let pos_el = tinted_scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &pid,
                rect: l.pos,
                vertical: false,
            },
            pos,
            lens(move |v: &mut Self| v.ofdm.nodes.get_mut(&node).map(|o| &mut o.pos_bar)),
            Some(action(move |v: &mut Self, _, _| {
                let o = v.ofdm_win(node);
                let p = f64::from(o.pos_bar.pos);
                if o.st.surveying() {
                    // Never past the last full Survey FFT of the selection.
                    o.st.survey_pos = (p * o.pos_unit).min(o.survey_max);
                } else {
                    o.st.symbol = p as usize;
                }
            })),
            Some(mc.field_cyan),
        );
        let pan_el = tinted_scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &nid,
                rect: l.pan,
                vertical: false,
            },
            pan,
            lens(move |v: &mut Self| v.ofdm.nodes.get_mut(&node).map(|o| &mut o.pan_bar)),
            Some(action(move |v: &mut Self, _, _| {
                let o = v.ofdm_win(node);
                // The thumb's top position may stand for up to a unit past
                // the end: keep the stored pan on the last full view.
                let max = f64::from(o.pan_bar.max - o.pan_bar.page + 1) * o.pan_unit;
                let end = o.pan_max.min(max);
                o.st.pan = Some((f64::from(o.pan_bar.pos) * o.pan_unit).min(end));
            })),
            Some(mc.field_lavender),
        );
        vec![gain_el, pos_el, pan_el]
    }
}
