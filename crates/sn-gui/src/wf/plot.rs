//! The WF plot (`docs/SA_UI_SPEC.md` 7, 13): the trace image on black without
//! axes, white V marker lines with green triangle handles on the top edge,
//! yellow H marker lines with green triangles on the left edge (red when
//! locked), the Info overlay, the Count freqs boxes, the navigation scroll bar
//! under the plot and the DC offset scroll bar on its right; and the mouse:
//! sweep, drag, lock, collapse, popup menu, boxes, the wheel and middle-drag
//! panning (as `sa-app`), and the plot keys.

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyElement, Context, Corners, Hsla, InteractiveElement, IntoElement, Keystroke, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, ScrollWheelEvent,
    StatefulInteractiveElement, Styled, Window, canvas, px,
};
use sn_core::NodeId;
use sn_ui_core::markers::{MarkerId, Markers};
use sn_ui_core::shortcuts::Command;
use sn_ui_core::sono_tools::{self, ViewAction};
use sn_ui_core::view::View;
use sn_ui_core::wf_plot::{self, SumKey, SumShown, TraceRequest, Vertical};
use sn_ui_core::wf_tools::{self, MAX_SUM_SAMPLES};
use sn_ui_core::{format, t};

use super::bottom::wf_extent;
use super::{Frame, Layout, TraceKey, WfInk, ink, render_image};
use crate::app::model::WinKind;
use crate::win95::colors::scheme;
use crate::win95::geom::Rect;
use crate::win95::glyph::{self, Glyph, Mode};
use crate::win95::kit::{Capture, action, lens};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, place, size_of};

/// Pixels the pointer must move before a press becomes a drag (`SM_CXDRAG`).
pub(crate) const DRAG_START: f64 = 4.0;
/// Pixels per wheel line.
const LINE_PX: f32 = 50.0;
/// Line height of the Info overlay and the boxes.
pub(crate) const ROW: i32 = LINE + 1;

/// What the plot's paint callback draws.
struct Scene {
    image: Option<Arc<gpui::RenderImage>>,
    view: Option<View>,
    vert: Vertical,
    markers: Markers,
    info: Option<Vec<String>>,
    /// Count freqs boxes: lines and place.
    labels: Vec<(Vec<String>, Rect)>,
    /// Text at the top left (Sum messages), and its colour.
    notice: Option<(String, Hsla)>,
    /// Text in the middle (the overview is being built).
    centre: Option<String>,
    ink: WfInk,
}

/// The size of a Count freqs box with `lines`.
pub(crate) fn box_size(window: &Window, lines: &[String]) -> (i32, i32) {
    let w = lines
        .iter()
        .map(|l| text::width(window, l, true))
        .max()
        .unwrap_or(0);
    (w + 12, ROW * lines.len() as i32 + 6)
}

impl Frame {
    /// The vertical mapping of `node`'s WF plot `h` pixels high.
    pub(crate) fn wf_vertical(&self, node: NodeId, h: i32) -> Vertical {
        let (scale, offset) = self
            .ui
            .get(&node)
            .map_or((1.0, 0.0), |u| (u.wave_scale, u.wave_offset));
        Vertical {
            scale,
            offset,
            height: f64::from(h),
        }
    }

    /// Fit the view to the plot, apply a waiting zoom request, place the
    /// markers of a new window, build the Sum histogram and make the trace
    /// image when it changed.
    pub(crate) fn wf_layout(&mut self, win: u32, node: NodeId, layout: Layout, cx: &gpui::App) {
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let overview = self.overviews.get(&node).cloned();
        let vert = self.wf_vertical(node, layout.plot.h);
        let show_q = self.ui.get(&node).is_none_or(|u| u.show_q);
        let hz = self.wf_is_hz(node);
        let colors = self.wf_ink(cx).trace;
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        ww.plot = layout.plot;
        let (w, h) = (layout.plot.w, layout.plot.h);
        if w < 8 || h < 8 {
            self.sync_wf_bars(win, node);
            return;
        }
        let extent = wf_extent(signal.len() as f64);
        let width = f64::from(w);
        let view = ww.view.get_or_insert_with(|| View::fit(extent, width));
        view.resized(extent, width);
        if let Some(a) = ww.action.take() {
            sono_tools::apply_action(a, view, &mut ww.history, &ww.markers, extent, width, false);
        }
        let view = *view;
        let Some(ov) = overview else {
            // The view exists and can change (wheel, keys) before the
            // overview is ready: the bars follow it already.
            self.sync_wf_bars(win, node);
            return;
        };
        if !ww.placed {
            ww.markers =
                wf_tools::initial_markers(signal.len(), vert.offset, f64::from(vert.scale));
            ww.placed = true;
        }
        if ww.amplitude.is_none() {
            let env = ov.span(&signal, 0, signal.len());
            let m = env.im.map_or(env.re, |q| env.re.merge(q));
            if !m.is_empty() {
                ww.amplitude = Some(f64::from(m.min.abs().max(m.max.abs())));
            }
        }
        let sum_key = (ww.sum && ww.ccf.is_none())
            .then(|| wf_sum_key(&view, &vert, layout.plot, signal.len()))
            .flatten();
        if let Some(k) = sum_key {
            ww.sum_cache.sync(&signal, k);
        }
        let key = TraceKey {
            node,
            view,
            size: (w, h),
            scale: vert.scale,
            offset: vert.offset,
            show_q,
            sum: ww.sum.then(|| ww.sum_cache.count()),
            ccf: ww.ccf.as_ref().map(|c| (c.fragment, c.signal.len())),
            colors,
        };
        if ww.shown.as_ref().is_none_or(|(k, _)| *k != key) {
            let hist = sum_key.and_then(|k| match ww.sum_cache.shown(k) {
                SumShown::Ready(h) => Some(h),
                _ => None,
            });
            let req = TraceRequest {
                signal: &signal,
                overview: &ov,
                view,
                width: w as usize,
                vert,
                show_q,
                hz,
                sum: hist,
                dim: ww.sum && ww.ccf.is_none(),
                ccf: ww.ccf.as_ref(),
                colors,
            };
            let image = wf_plot::render(&req);
            if let Some(img) = render_image(&image)
                && let Some((_, old)) = ww.shown.replace((key, img))
            {
                self.wf.stale.push(old);
            }
        }
        self.sync_wf_bars(win, node);
    }

    /// The Sum message for the plot, if any.
    fn wf_sum_notice(&self, win: u32, node: NodeId, ink: &WfInk) -> Option<(String, Hsla)> {
        let ww = self.wf.wins.get(&win)?;
        if !ww.sum || ww.ccf.is_some() {
            return None;
        }
        let color = if self.wf_is_hz(node) {
            ink.trace.sum_hz
        } else {
            ink.trace.sum
        };
        let len = self.graph.get(node)?.signal.len();
        let view = ww.view?;
        let vert = self.wf_vertical(node, ww.plot.h);
        let Some(key) = wf_sum_key(&view, &vert, ww.plot, len) else {
            return Some((t!("wave-sum-zoom-in"), super::ink(color)));
        };
        match ww.sum_cache.shown(key) {
            SumShown::Ready(_) => None,
            SumShown::Building => Some((t!("wave-sum-building"), super::ink(color))),
            SumShown::Failed(e) => Some((
                t!("wave-sum-failed", error = e.to_owned()),
                super::ink(color),
            )),
        }
    }

    /// Everything the plot of window `win` draws.
    fn wf_scene(&mut self, win: u32, node: NodeId, window: &Window, cx: &gpui::App) -> Scene {
        let ink = self.wf_ink(cx);
        let image = self.wf_image(win);
        let notice = self.wf_sum_notice(win, node, &ink);
        let centre = (!self.overviews.contains_key(&node)).then(|| t!("wave-building-overview"));
        let br = self.wf_br();
        let fs = self.graph.get(node).map_or(1.0, |n| n.signal.sample_rate());
        let hz = self.wf_is_hz(node);
        let plot_h = self.wf.wins.get(&win).map_or(0, |w| w.plot.h);
        let vert = self.wf_vertical(node, plot_h);
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return Scene {
                image,
                view: None,
                vert,
                markers: Markers::default(),
                info: None,
                labels: Vec::new(),
                notice,
                centre,
                ink,
            };
        };
        let info = ww
            .info
            .then(|| wf_tools::info_lines(&ww.markers, fs, br, hz.then(|| vert.per_px())));
        let plot = (ww.plot.w, ww.plot.h);
        ww.label_sizes.clear();
        let mut labels = Vec::new();
        for l in &mut ww.labels {
            let size = box_size(window, &l.lines);
            let at = *l
                .offset
                .get_or_insert(((plot.0 - size.0) / 2, (plot.1 - size.1) / 2));
            let at = sono_tools::clamp_label_offset(at, size, plot);
            l.offset = Some(at);
            labels.push((l.lines.clone(), Rect::new(at.0, at.1, size.0, size.1)));
            ww.label_sizes.push(size);
        }
        Scene {
            image,
            view: ww.view,
            vert,
            markers: ww.markers,
            info,
            labels,
            notice,
            centre,
            ink,
        }
    }

    /// The plot element of window `win` at `r` (client coordinates).
    pub(crate) fn wf_plot_element(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let scene = self.wf_scene(win, node, window, cx);
        let slot = Slot::default();
        let paint_slot = slot.clone();
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                paint_slot.set(b);
                paint_scene(window, cx, b, &scene);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let (s1, s2, s3, s4, s5) = (
            slot.clone(),
            slot.clone(),
            slot.clone(),
            slot.clone(),
            slot.clone(),
        );
        place(r)
            .id(gpui::ElementId::Name(format!("wf{win}.plot").into()))
            .child(paint)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.wf_left_down(win, ev, &s1);
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.wf_middle_down(win, ev, &s2);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseUpEvent, _, cx| {
                    v.wf_right_click(win, ev.position, &s3);
                    cx.notify();
                }),
            )
            .on_scroll_wheel(
                cx.listener(move |v: &mut Self, ev: &ScrollWheelEvent, _, cx| {
                    v.wf_wheel(win, ev, &s4);
                    cx.notify();
                }),
            )
            .on_mouse_move(
                cx.listener(move |v: &mut Self, ev: &MouseMoveEvent, _, cx| {
                    v.wf_hover(win, ev.position, &s5);
                    cx.notify();
                }),
            )
            .on_hover(cx.listener(move |v: &mut Self, over: &bool, _, cx| {
                if !*over && let Some(ww) = v.wf.wins.get_mut(&win) {
                    ww.cursor = None;
                    cx.notify();
                }
            }))
            .into_any_element()
    }

    /// The view, vertical mapping, signal length and node of window `win`,
    /// for a gesture.
    fn wf_gesture(&self, win: u32) -> Option<(View, Vertical, f64, NodeId)> {
        let ww = self.wf.wins.get(&win)?;
        let len = self.graph.get(ww.node)?.signal.len() as f64;
        Some((ww.view?, self.wf_vertical(ww.node, ww.plot.h), len, ww.node))
    }

    /// The Count freqs box under `(x, y)` (plot coordinates), topmost first.
    fn wf_label_at(&self, win: u32, x: i32, y: i32) -> Option<usize> {
        let ww = self.wf.wins.get(&win)?;
        ww.labels
            .iter()
            .zip(&ww.label_sizes)
            .enumerate()
            .rev()
            .find(|(_, (l, (w, h)))| {
                l.offset
                    .is_some_and(|(lx, ly)| Rect::new(lx, ly, *w, *h).contains(x, y))
            })
            .map(|(i, _)| i)
    }

    /// Left button down in the plot: move a box, double-click collapse, drag
    /// a marker by its handle or line, or sweep a new V/H selection.
    fn wf_left_down(&mut self, win: u32, ev: &MouseDownEvent, slot: &Slot) {
        let Some((x, y)) = slot.local(ev.position) else {
            return;
        };
        self.kit.focus = None;
        if let Some(i) = self.wf_label_at(win, x, y) {
            self.wf_drag_label(win, i, (x, y), slot);
            return;
        }
        let Some((view, vert, len, _)) = self.wf_gesture(win) else {
            return;
        };
        let p = (f64::from(x), f64::from(y));
        let at = (
            view.x_to_sample(p.0).clamp(0.0, len),
            vert.value_at(p.1.clamp(0.0, vert.height)),
        );
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        if ev.click_count >= 2 {
            ww.undo = Some(ww.markers);
            sono_tools::collapse_markers(&mut ww.markers, at.0, at.1);
            return;
        }
        let grabbed = wf_plot::handle_near(&ww.markers, &view, &vert, p)
            .or_else(|| wf_plot::marker_near(&ww.markers, &view, &vert, p));
        let started = Rc::new(std::cell::Cell::new(false));
        let slot = slot.clone();
        self.kit.capture(Capture {
            on_move: Some(Rc::new(move |v: &mut Self, at, _, _| {
                let Some((x, y)) = slot.local(at) else {
                    return;
                };
                let q = (f64::from(x), f64::from(y));
                if !started.get() {
                    if (q.0 - p.0).hypot(q.1 - p.1) < DRAG_START {
                        return;
                    }
                    started.set(true);
                    v.wf_begin_drag(win, grabbed, p);
                }
                v.wf_drag_to(win, grabbed, q);
            })),
            on_up: None,
        });
    }

    /// A drag started: a sweep saves the markers for undo and starts at `p`.
    fn wf_begin_drag(&mut self, win: u32, grabbed: Option<MarkerId>, p: (f64, f64)) {
        if grabbed.is_some() {
            return;
        }
        let Some((view, vert, len, _)) = self.wf_gesture(win) else {
            return;
        };
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.undo = Some(ww.markers);
            ww.markers.begin_sweep(
                view.x_to_sample(p.0).clamp(0.0, len),
                vert.value_at(p.1.clamp(0.0, vert.height)),
            );
        }
    }

    /// The pointer moved to `q` during a drag.
    fn wf_drag_to(&mut self, win: u32, grabbed: Option<MarkerId>, q: (f64, f64)) {
        let Some((view, vert, len, _)) = self.wf_gesture(win) else {
            return;
        };
        let s = view.x_to_sample(q.0).clamp(0.0, len);
        let v = vert.value_at(q.1.clamp(0.0, vert.height));
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            match grabbed {
                Some(id) if id.is_vertical() => {
                    ww.markers.set(id, s);
                }
                Some(id) => {
                    ww.markers.set(id, v);
                }
                None => ww.markers.sweep_to(s, v),
            }
        }
    }

    /// Move Count freqs box `i` with the mouse.
    fn wf_drag_label(&mut self, win: u32, i: usize, from: (i32, i32), slot: &Slot) {
        let Some(start) = self
            .wf
            .wins
            .get(&win)
            .and_then(|w| w.labels.get(i))
            .and_then(|l| l.offset)
        else {
            return;
        };
        let slot = slot.clone();
        self.kit.capture(Capture {
            on_move: Some(Rc::new(move |v: &mut Self, at, _, _| {
                let Some((x, y)) = slot.local(at) else {
                    return;
                };
                let Some(ww) = v.wf.wins.get_mut(&win) else {
                    return;
                };
                let plot = (ww.plot.w, ww.plot.h);
                let size = ww.label_sizes.get(i).copied().unwrap_or((0, 0));
                if let Some(l) = ww.labels.get_mut(i) {
                    let to = (start.0 + x - from.0, start.1 + y - from.1);
                    l.offset = Some(sono_tools::clamp_label_offset(to, size, plot));
                }
            })),
            on_up: None,
        });
    }

    /// Middle-drag pans the time axis (an invisible SigNexora extra).
    fn wf_middle_down(&mut self, win: u32, ev: &MouseDownEvent, slot: &Slot) {
        let Some((x0, _)) = slot.local(ev.position) else {
            return;
        };
        let last = Rc::new(std::cell::Cell::new(x0));
        let slot = slot.clone();
        self.kit.capture_with(
            MouseButton::Middle,
            Capture {
                on_move: Some(Rc::new(move |v: &mut Self, at, _, _| {
                    let Some((x, _)) = slot.local(at) else {
                        return;
                    };
                    let dx = x - last.replace(x);
                    v.wf_with_view(win, |view, extent, w| {
                        view.pan_time(-f64::from(dx), extent, w);
                    });
                })),
                on_up: None,
            },
        );
    }

    /// Change the view of window `win` (with its extent and width).
    fn wf_with_view(&mut self, win: u32, f: impl FnOnce(&mut View, sn_ui_core::view::Extent, f64)) {
        let len = self
            .node_of(win)
            .and_then(|n| self.graph.get(n))
            .map_or(0.0, |n| n.signal.len() as f64);
        if let Some(ww) = self.wf.wins.get_mut(&win)
            && let Some(view) = ww.view.as_mut()
        {
            f(view, wf_extent(len), f64::from(ww.plot.w));
        }
    }

    /// A right click in the plot: remove a box, lock or unlock a marker by its
    /// handle, or open the popup menu.
    fn wf_right_click(&mut self, win: u32, at: gpui::Point<gpui::Pixels>, slot: &Slot) {
        let Some((x, y)) = slot.local(at) else {
            return;
        };
        if let Some(i) = self.wf_label_at(win, x, y) {
            if let Some(ww) = self.wf.wins.get_mut(&win) {
                ww.labels.remove(i);
            }
            return;
        }
        if let Some((view, vert, _, _)) = self.wf_gesture(win)
            && let Some(ww) = self.wf.wins.get_mut(&win)
            && let Some(id) =
                wf_plot::handle_near(&ww.markers, &view, &vert, (f64::from(x), f64::from(y)))
        {
            ww.markers.toggle_lock(id);
            return;
        }
        let pos = (
            f32::from(at.x).round() as i32,
            f32::from(at.y).round() as i32,
        );
        self.open_wf_popup(win, pos);
    }

    /// The mouse wheel, as `sa-app`: zoom time at the pointer; Shift or a
    /// horizontal scroll pans; Alt changes the vertical scale; Ctrl or Cmd
    /// zooms time faster (egui's pinch zoom).
    fn wf_wheel(&mut self, win: u32, ev: &ScrollWheelEvent, slot: &Slot) {
        let Some((x, _)) = slot.local(ev.position) else {
            return;
        };
        let d = ev.delta.pixel_delta(px(LINE_PX));
        let (dx, dy) = (f64::from(f32::from(d.x)), f64::from(f32::from(d.y)));
        let m = ev.modifiers;
        if m.alt && !(m.control || m.platform) {
            if let Some(node) = self.node_of(win) {
                self.wf_scale(node, (-dy * 0.002).exp() as f32);
            }
            return;
        }
        let ax = f64::from(x);
        self.wf_with_view(win, |view, extent, w| {
            if m.control || m.platform {
                view.zoom_time((-dy / 200.0).exp(), ax, extent, w);
            } else if m.shift || dx.abs() > dy.abs() {
                let delta = if dx.abs() > dy.abs() { dx } else { dy };
                view.pan_time(-delta, extent, w);
            } else {
                view.zoom_time((-dy * 0.002).exp(), ax, extent, w);
            }
        });
    }

    /// The status bar readout of the pointer over the plot: time and value.
    fn wf_hover(&mut self, win: u32, at: gpui::Point<gpui::Pixels>, slot: &Slot) {
        let Some((x, y)) = slot.local(at) else {
            return;
        };
        let Some((view, vert, _, node)) = self.wf_gesture(win) else {
            return;
        };
        let fs = self.graph.get(node).map_or(1.0, |n| n.signal.sample_rate());
        let s = view.x_to_sample(f64::from(x));
        let value = vert.value_at(f64::from(y));
        let text = format!(
            "{}  v = {}",
            sono_tools::cursor_readout(s, None, None, fs, 0.0),
            format::dec(value, 4)
        );
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.cursor = Some(text);
        }
    }

    /// The navigation and DC offset scroll bars from the view and the
    /// vertical settings.
    fn sync_wf_bars(&mut self, win: u32, node: NodeId) {
        let len = self.graph.get(node).map_or(0.0, |n| n.signal.len() as f64);
        let (scale, offset) = self
            .ui
            .get(&node)
            .map_or((1.0, 0.0), |u| (f64::from(u.wave_scale), u.wave_offset));
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        if let Some(view) = ww.view {
            let total = (len / view.samples_per_px.max(f64::MIN_POSITIVE)).ceil() as i64;
            let (nav, unit) =
                crate::app::sono::plot::nav_state(view.first_column(), total, ww.plot.w);
            ww.nav_unit = unit;
            ww.nav = nav;
        }
        ww.dc = dc_state(offset, scale, ww.amplitude);
    }

    /// The navigation scroll bar moved: pan the view to its position.
    fn wf_nav_moved(&mut self, win: u32) {
        let len = self
            .node_of(win)
            .and_then(|n| self.graph.get(n))
            .map_or(0.0, |n| n.signal.len() as f64);
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        let (pos, unit) = (i64::from(ww.nav.pos), ww.nav_unit);
        let w = f64::from(ww.plot.w);
        if let Some(view) = ww.view.as_mut() {
            view.start = (pos * unit) as f64 * view.samples_per_px;
            view.resized(wf_extent(len), w);
        }
    }

    /// The DC offset scroll bar moved: move the trace against zero.
    fn wf_dc_moved(&mut self, win: u32) {
        let Some(node) = self.node_of(win) else {
            return;
        };
        let amplitude = self.wf.wins.get(&win).and_then(|w| w.amplitude);
        let pos = self.wf.wins.get(&win).map_or(0, |w| w.dc.pos);
        if let Some(u) = self.ui.get_mut(&node) {
            u.wave_offset = dc_offset(pos, f64::from(u.wave_scale), amplitude);
        }
    }

    /// SA's navigation scroll bar under the plot of window `win`.
    pub(crate) fn wf_nav_bar(
        &mut self,
        win: u32,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let st = self.wf.wins.get(&win).map(|w| w.nav).unwrap_or_default();
        let id = format!("wf{win}.nav");
        scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect: r,
                vertical: false,
            },
            st,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.nav)),
            Some(action(move |v: &mut Self, _, _| v.wf_nav_moved(win))),
        )
    }

    /// SA's vertical scroll bar on the right of the plot: the DC offset.
    pub(crate) fn wf_dc_bar(
        &mut self,
        win: u32,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let st = self.wf.wins.get(&win).map(|w| w.dc).unwrap_or_default();
        let id = format!("wf{win}.dc");
        scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect: r,
                vertical: true,
            },
            st,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.dc)),
            Some(action(move |v: &mut Self, _, _| v.wf_dc_moved(win))),
        )
    }

    /// The active window if it is a WF window, with its signal.
    fn active_wf(&self) -> Option<(u32, NodeId)> {
        let win = self.mdi.active()?;
        match self.binding(win)? {
            (node, WinKind::Waveform) => Some((win, node)),
            _ => None,
        }
    }

    /// A plot key on the active WF window: Z, Backspace, F and Cmd+Z.
    pub(crate) fn wf_key_command(&mut self, c: Command) {
        let Some((win, _)) = self.active_wf() else {
            return;
        };
        let a = match c {
            Command::ZoomToSelection => ViewAction::ZoomToSelection,
            Command::ZoomBack => ViewAction::Back,
            Command::FullView => ViewAction::Full,
            Command::RestoreMarkers => {
                if let Some(ww) = self.wf.wins.get_mut(&win)
                    && let Some(prev) = ww.undo.take()
                {
                    ww.undo = Some(std::mem::replace(&mut ww.markers, prev));
                }
                return;
            }
            _ => return,
        };
        self.wf_view_action(win, a);
    }

    /// The arrow keys move the V marker moved last (Shift: 10 pixels).
    /// Returns whether the key was used.
    pub(crate) fn wf_arrow_key(&mut self, k: &Keystroke) -> bool {
        let Some((win, _)) = self.active_wf() else {
            return false;
        };
        let m = k.modifiers;
        if m.control || m.platform || m.alt {
            return false;
        }
        let dx = match k.key.as_str() {
            "left" => -1.0,
            "right" => 1.0,
            _ => return false,
        };
        let step = if m.shift { 10.0 } else { 1.0 };
        let Some(node) = self.wf.wins.get(&win).map(|w| w.node) else {
            return false;
        };
        // The mouse keeps the markers on the signal; so do the keys.
        let len = self.graph.get(node).map_or(0.0, |n| n.signal.len() as f64);
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return false;
        };
        let Some(view) = ww.view else {
            return false;
        };
        let Some(id) = ww.markers.active.filter(|id| id.is_vertical()) else {
            return false;
        };
        let Some(pos) = ww.markers.get(id) else {
            return false;
        };
        ww.markers
            .set(id, (pos + dx * step * view.samples_per_px).clamp(0.0, len))
    }
}

/// The Sum histogram of the visible span of a plot `plot` showing `len`
/// samples, or `None` when the span is too long to sum (zoom in). The plot
/// and its notice both use it, so they ask the cache for the same key.
fn wf_sum_key(view: &View, vert: &Vertical, plot: Rect, len: usize) -> Option<SumKey> {
    let (a, b) = wf_tools::visible_span(view, f64::from(plot.w), len);
    (b - a <= MAX_SUM_SAMPLES).then(|| SumKey {
        start: a,
        end: b,
        rows: plot.h.max(2) as usize,
        lo: vert.offset - f64::from(vert.scale),
        hi: vert.offset + f64::from(vert.scale),
    })
}

/// The DC offset scroll range: the signal's amplitude, at least the scale.
fn dc_range(scale: f64, amplitude: Option<f64>) -> f64 {
    amplitude.unwrap_or(1.0).max(scale) * 2.0
}

/// The DC offset scroll range and its step count (one step a tenth of the
/// scale, 1 to 100000 steps), shared by [`dc_state`] and [`dc_offset`] so
/// they stay inverses.
fn dc_steps(scale: f64, amplitude: Option<f64>) -> (f64, i32) {
    let range = dc_range(scale, amplitude);
    let unit = (scale / 10.0).max(f64::MIN_POSITIVE);
    let steps = (2.0 * range / unit).round().clamp(1.0, 100_000.0) as i32;
    (range, steps)
}

/// The DC offset scroll bar for `offset`: one unit is a tenth of the vertical
/// half range (`sa-app`'s arrow step), a page the half range; the top shows the
/// highest offset.
pub(crate) fn dc_state(offset: f64, scale: f64, amplitude: Option<f64>) -> ScrollState {
    let (range, steps) = dc_steps(scale, amplitude);
    let t = wf_tools::offset_to_thumb(offset, range);
    let pos = (t * f64::from(steps)).round() as i32;
    ScrollState::new(0, steps + 10 - 1, 10, pos)
}

/// The DC offset at scroll position `pos` (inverse of [`dc_state`]).
pub(crate) fn dc_offset(pos: i32, scale: f64, amplitude: Option<f64>) -> f64 {
    let (range, steps) = dc_steps(scale, amplitude);
    wf_tools::thumb_to_offset(f64::from(pos) / f64::from(steps), range)
}

/// Paint the plot.
fn paint_scene(window: &mut Window, cx: &mut gpui::App, b: gpui::Bounds<gpui::Pixels>, sc: &Scene) {
    let (w, h) = size_of(b);
    let o = b.origin;
    window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
        paint::fill(window, o, Rect::new(0, 0, w, h), ink(sc.ink.trace.bg));
        if let Some(img) = &sc.image {
            window
                .paint_image(b, Corners::default(), img.clone(), 0, false)
                .ok();
        }
        if let Some(view) = sc.view {
            paint_markers(window, cx, o, (w, h), &view, &sc.vert, &sc.markers, &sc.ink);
        }
        if let Some(t) = &sc.centre {
            let tw = text::width(window, t, false);
            let at = paint::at(o, (w - tw) / 2, (h - LINE) / 2);
            text::draw(window, cx, at, t, false, ink(sc.ink.dim_text));
        }
        if let Some((t, c)) = &sc.notice {
            for (i, l) in (0_i32..).zip(t.lines()) {
                text::draw(window, cx, paint::at(o, 6, 4 + i * ROW), l, false, *c);
            }
        }
        if let Some(lines) = &sc.info {
            paint_info(window, cx, o, w, lines, &sc.ink);
        }
        for (lines, r) in &sc.labels {
            paint_box(window, cx, o, lines, *r, &sc.ink);
        }
    });
}

/// A Count freqs box: green with a yellow border and white bold text.
fn paint_box(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    lines: &[String],
    r: Rect,
    wink: &WfInk,
) {
    paint::fill(window, o, r, ink(wink.grid_bg));
    for e in [
        Rect::new(r.x, r.y, r.w, 2),
        Rect::new(r.x, r.bottom() - 2, r.w, 2),
        Rect::new(r.x, r.y, 2, r.h),
        Rect::new(r.right() - 2, r.y, 2, r.h),
    ] {
        paint::fill(window, o, e, ink(wink.grid_border));
    }
    for (i, l) in (0_i32..).zip(lines) {
        let at = paint::at(o, r.x + 6, r.y + 3 + i * ROW);
        text::draw(window, cx, at, l, true, ink(wink.grid_text));
    }
}

/// The markers: white V lines with green triangles hanging from the top edge,
/// yellow H lines with green triangles on the left edge; a locked handle is
/// red.
fn paint_markers(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    (w, h): (i32, i32),
    view: &View,
    vert: &Vertical,
    m: &Markers,
    wink: &WfInk,
) {
    let s = scheme(cx);
    let hd = wf_plot::HANDLE as f32;
    let color = |id: MarkerId| {
        if m.locked == Some(id) {
            wink.locked
        } else {
            wink.handle
        }
    };
    let mut handles: Vec<(i32, i32, Glyph, u32)> = Vec::new();
    if let Some((a, b)) = m.v {
        for (id, smp) in [(MarkerId::V1, a), (MarkerId::V2, b)] {
            let x = view.sample_to_x(smp).floor() as i32;
            if x < -8 || x > w + 8 {
                continue;
            }
            paint::fill(window, o, Rect::new(x, 0, 1, h), ink(wink.v_line));
            let tri = Glyph::new(13, 10).poly(
                &[(0.0, 0.0), (2.0 * hd + 1.0, 0.0), (hd + 0.5, 1.5 * hd)],
                glyph::BLACK,
            );
            handles.push((x - 6, 0, tri, super::rgb_u32(color(id))));
        }
    }
    if let Some((a, b)) = m.h {
        for (id, v) in [(MarkerId::H1, a), (MarkerId::H2, b)] {
            let y = (vert.y_of(v).floor() as i32).clamp(0, (h - 1).max(0));
            paint::fill(window, o, Rect::new(0, y, w, 1), ink(wink.h_line));
            let tri = Glyph::new(10, 13).poly(
                &[(0.0, 0.0), (1.5 * hd, hd + 0.5), (0.0, 2.0 * hd + 1.0)],
                glyph::BLACK,
            );
            handles.push((0, y - 6, tri, super::rgb_u32(color(id))));
        }
    }
    for (x, y, g, c) in handles {
        glyph::draw(window, cx, o, (x, y), &g, Mode::Mono(c), &s);
    }
}

/// SA's Info overlay: a translucent dark box at the top right of the plot,
/// white text, one value per line.
pub(crate) fn paint_info(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    w: i32,
    lines: &[String],
    wink: &WfInk,
) {
    let tw = lines
        .iter()
        .map(|l| text::width(window, l, false))
        .max()
        .unwrap_or(0);
    let bw = tw + 12;
    let r = Rect::new(w - bw - 4, 4, bw, ROW * lines.len() as i32 + 6);
    paint::fill(window, o, r, ink(wink.info_bg));
    for (i, l) in (0_i32..).zip(lines) {
        let at = paint::at(o, r.x + 6, r.y + 3 + i * ROW);
        text::draw(window, cx, at, l, false, ink(wink.info_text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_bar_steps_a_tenth_of_the_scale() {
        let st = dc_state(0.0, 1.0, Some(1.0));
        // Range 2 (twice the amplitude): 40 steps of 0.1 from +2 down to -2.
        assert_eq!(st.top(), 40);
        assert_eq!(st.pos, 20);
        assert!((dc_offset(19, 1.0, Some(1.0)) - 0.1).abs() < 1e-9);
        assert!((dc_offset(0, 1.0, Some(1.0)) - 2.0).abs() < 1e-9);
        let st = dc_state(0.5, 1.0, Some(1.0));
        assert!((dc_offset(st.pos, 1.0, Some(1.0)) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn dc_bar_round_trips_at_the_step_limit() {
        for scale in [1e-9, 0.0] {
            let (range, steps) = dc_steps(scale, Some(1.0));
            assert_eq!(steps, 100_000, "clamped");
            let resolution = 2.0 * range / f64::from(steps);
            for x in [-1.5, 0.0, 0.25, 1.9] {
                let back = dc_offset(dc_state(x, scale, Some(1.0)).pos, scale, Some(1.0));
                assert!((back - x).abs() <= resolution, "{scale}: {x} -> {back}");
            }
        }
    }
}
