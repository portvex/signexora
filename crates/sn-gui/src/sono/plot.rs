//! The sonogram plot (`docs/SA_UI_SPEC.md` 4.2, 4.4): the image without axes,
//! SA 6.2's markers (thin pale yellow lines, solid blue triangle handles on
//! the top and left edges, red when locked), the Info overlay, the floating
//! result boxes, and the mouse: sweep, drag, lock, collapse, popup menu, the
//! wheel (as `sa-app`), middle-drag panning and the navigation scroll bar.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, Corners, Hsla, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, ScrollWheelEvent,
    StatefulInteractiveElement, Styled, Window, canvas, px,
};
use sn_core::NodeId;
use sn_ui_core::color::Rgba8;
use sn_ui_core::colormap::OUTSIDE;
use sn_ui_core::markers::{MarkerId, Markers};
use sn_ui_core::sono_tools::{self, HANDLE};
use sn_ui_core::theme::ThemeChoice;
use sn_ui_core::view::View;
use sn_ui_core::{lineage, t};

use super::Frame;
use crate::win95::colors::{Role, hsla, palette, scheme};
use crate::win95::geom::Rect;
use crate::win95::glyph::{self, Glyph, Mode};
use crate::win95::kit::{Capture, action, lens};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, place};

/// Pixels the pointer must move before a press becomes a drag (`SM_CXDRAG`).
const DRAG_START: f64 = 4.0;
/// Pixels per wheel line (as egui counts them).
const LINE_PX: f32 = 50.0;
/// Line height of the Info overlay and the result boxes.
const ROW: i32 = LINE + 1;

/// The colours of the plot: SA's own in the Classic theme, the theme's plot
/// palette in the others (as `sa-app`'s `Chrome`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlotInk {
    /// Columns before the start and after the end of the recording.
    pub(crate) outside: Rgba8,
    pub(crate) bg: Hsla,
    pub(crate) line: Hsla,
    pub(crate) handle: Hsla,
    pub(crate) locked: Hsla,
    pub(crate) info_bg: Hsla,
    pub(crate) info_text: Hsla,
    pub(crate) label_bg: Hsla,
    pub(crate) label_border: Hsla,
    pub(crate) label_text: Hsla,
    pub(crate) notice: Hsla,
    /// The harmonics module's Control line.
    pub(crate) control: Hsla,
}

/// A colour with its alpha (palette colours are premultiplied).
pub(crate) fn hsla8(c: Rgba8) -> Hsla {
    let a = f32::from(c.a) / 255.0;
    let un = |v: u8| {
        if c.a == 0 {
            0.0
        } else {
            (f32::from(v) / 255.0 / a).min(1.0)
        }
    };
    gpui::Rgba {
        r: un(c.r),
        g: un(c.g),
        b: un(c.b),
        a,
    }
    .into()
}

/// What the plot's paint callback draws.
struct Scene {
    image: Option<std::sync::Arc<gpui::RenderImage>>,
    view: Option<View>,
    markers: Markers,
    info: Option<Vec<String>>,
    /// Result boxes: text, place (plot coordinates), size.
    labels: Vec<(String, Rect)>,
    notice: Option<String>,
    control: Option<f64>,
    ink: PlotInk,
}

impl Frame {
    /// The plot colours under the current theme.
    pub(crate) fn plot_ink(&self, cx: &gpui::App) -> PlotInk {
        let s = scheme(cx);
        if self.prefs.theme == ThemeChoice::Classic {
            return PlotInk {
                outside: Rgba8::rgb(0, 0, 64),
                bg: hsla(0x00_00_00),
                line: hsla(0xFF_FF_AA),
                handle: hsla(0x00_50_FF),
                locked: hsla(0xFF_00_00),
                info_bg: hsla8(Rgba8::premultiplied(0, 0, 0, 150)),
                info_text: hsla(0xFF_FF_FF),
                label_bg: hsla(0x00_00_80),
                label_border: hsla(0x50_8C_FF),
                label_text: hsla(0xFF_FF_FF),
                notice: hsla8(palette(cx).error),
                control: hsla8(palette(cx).cursor),
            };
        }
        let p = palette(cx);
        PlotInk {
            outside: OUTSIDE,
            bg: hsla8(p.plot_bg),
            line: hsla8(p.marker_free),
            handle: hsla8(p.marker_extra),
            locked: hsla8(p.marker_locked),
            info_bg: hsla8(p.overlay_bg),
            info_text: hsla8(p.plot_text),
            label_bg: s.of(Role::Selection),
            label_border: s.of(Role::SelectionText),
            label_text: s.of(Role::SelectionText),
            notice: hsla8(p.error),
            control: hsla8(p.cursor),
        }
    }

    /// The latest Br, and the latest Sc mapped onto `node`'s axis, for the
    /// Info overlay.
    pub(crate) fn info_values(&self, node: NodeId) -> (Option<f64>, Option<f64>) {
        let br = self
            .measured(&sn_core::MeasurementKind::BaudRate)
            .map(|m| m.value);
        // An Sc that cannot be mapped onto this signal's axis would read as a
        // valid absolute carrier here; leave it out of the overlay.
        let sc = self.sc_for(node).filter(|s| !s.other_axis).map(|s| s.value);
        (br, sc)
    }

    /// The Info overlay's lines for `node`, when its Info box is checked.
    pub(crate) fn info_lines(&self, node: NodeId) -> Option<Vec<String>> {
        let u = self.ui.get(&node).filter(|u| u.info)?;
        let n = self.graph.get(node)?;
        let (br, sc) = self.info_values(node);
        Some(sono_tools::info_lines(
            &u.markers,
            n.signal.sample_rate(),
            n.signal.center_frequency(),
            br,
            sc,
        ))
    }

    /// Everything the plot of window `win` draws.
    fn scene(
        &mut self,
        win: u32,
        node: NodeId,
        plot: Rect,
        window: &Window,
        cx: &gpui::App,
    ) -> Scene {
        let ink = self.plot_ink(cx);
        let markers = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let info = self.info_lines(node);
        let control = self.control_line(node);
        let image = self.sono_image(win);
        let Some(sw) = self.sono.wins.get_mut(&win) else {
            return Scene {
                image,
                view: None,
                markers,
                info,
                labels: Vec::new(),
                notice: None,
                control,
                ink,
            };
        };
        let view = sw.view;
        let mut labels = Vec::new();
        sw.label_sizes.clear();
        if let Some(v) = view {
            for l in &mut sw.labels {
                let size = (text::width(window, &l.text, true) + 12, ROW + 5);
                let (x, y) = l.place(&v, size, (plot.w, plot.h));
                labels.push((l.text.clone(), Rect::new(x, y, size.0, size.1)));
                sw.label_sizes.push(size);
            }
        }
        let notice = match (sw.cache.error(), sw.cache.job_error()) {
            (Some(e), _) => Some(e.to_owned()),
            (None, Some(e)) => Some(t!("sono-failed", error = e)),
            (None, None) if sw.extent.is_none() => Some(t!("sono-no-spectrum")),
            _ => None,
        };
        Scene {
            image,
            view,
            markers,
            info,
            labels,
            notice,
            control,
            ink,
        }
    }

    /// The harmonics module's Control line (Hz on this window's axis), while
    /// its `Control` box is checked (`src/harmonics/`).
    fn control_line(&self, node: NodeId) -> Option<f64> {
        self.harmonics_line(node)
    }

    /// The plot element of window `win` at `r` (client coordinates).
    pub(crate) fn plot_element(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let scene = self.scene(win, node, r, window, cx);
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
            .id(gpui::ElementId::Name(format!("sw{win}.plot").into()))
            .child(paint)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.plot_left_down(win, ev, &s1);
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.plot_middle_down(win, ev, &s2);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseUpEvent, _, cx| {
                    v.plot_right_click(win, ev.position, &s3);
                    cx.notify();
                }),
            )
            .on_scroll_wheel(
                cx.listener(move |v: &mut Self, ev: &ScrollWheelEvent, _, cx| {
                    v.plot_wheel(win, ev, &s4);
                    cx.notify();
                }),
            )
            .on_mouse_move(
                cx.listener(move |v: &mut Self, ev: &MouseMoveEvent, _, cx| {
                    v.plot_hover(win, ev.position, &s5);
                    cx.notify();
                }),
            )
            .on_hover(cx.listener(move |v: &mut Self, over: &bool, _, cx| {
                if !*over && let Some(sw) = v.sono.wins.get_mut(&win) {
                    sw.cursor = None;
                    cx.notify();
                }
            }))
            .into_any_element()
    }

    /// The view, extent, plot height and markers of window `win`, for a gesture.
    fn gesture_ctx(&self, win: u32) -> Option<(View, sn_ui_core::view::Extent, f64, NodeId)> {
        let sw = self.sono.wins.get(&win)?;
        Some((sw.view?, sw.extent?, f64::from(sw.plot.h), sw.node))
    }

    /// The result box under `(x, y)` (plot coordinates), topmost first.
    fn label_at(&self, win: u32, x: i32, y: i32) -> Option<usize> {
        let sw = self.sono.wins.get(&win)?;
        sw.labels
            .iter()
            .zip(&sw.label_sizes)
            .enumerate()
            .rev()
            .find(|(_, (l, (w, h)))| {
                l.offset
                    .is_some_and(|(lx, ly)| Rect::new(lx, ly, *w, *h).contains(x, y))
            })
            .map(|(i, _)| i)
    }

    /// Left button down in the plot: move a result box, double-click collapse,
    /// drag a marker by its handle or line, or sweep a new V/H selection.
    fn plot_left_down(&mut self, win: u32, ev: &MouseDownEvent, slot: &Slot) {
        let Some((x, y)) = slot.local(ev.position) else {
            return;
        };
        // The plot takes the keyboard back from a text field (the plot keys).
        self.kit.focus = None;
        if let Some(i) = self.label_at(win, x, y) {
            self.drag_label(win, i, (x, y), slot);
            return;
        }
        let Some((view, extent, h, node)) = self.gesture_ctx(win) else {
            return;
        };
        let p = (f64::from(x), f64::from(y));
        let (s, f) = (view.x_to_sample(p.0), view.y_to_freq(p.1, h));
        let undo = self.sono.wins.get(&win).and_then(|w| w.undo);
        let Some(nui) = self.ui.get_mut(&node) else {
            return;
        };
        if ev.click_count >= 2 {
            let mut undo = undo;
            if nui.markers != Markers::default() {
                undo = Some(nui.markers);
            }
            sono_tools::collapse_markers(
                &mut nui.markers,
                s.clamp(0.0, extent.len),
                f.clamp(extent.f_min, extent.f_max),
            );
            if let Some(sw) = self.sono.wins.get_mut(&win) {
                sw.undo = undo;
            }
            self.mark_dirty();
            return;
        }
        let grabbed = sono_tools::handle_near(&nui.markers, &view, h, p)
            .or_else(|| sono_tools::marker_near(&nui.markers, &view, h, p));
        self.start_marker_drag(win, grabbed, p, slot.clone());
    }

    /// Follow the mouse after a press at `p`: move marker `grabbed`, or sweep.
    fn start_marker_drag(
        &mut self,
        win: u32,
        grabbed: Option<MarkerId>,
        p: (f64, f64),
        slot: Slot,
    ) {
        let started = Rc::new(std::cell::Cell::new(false));
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
                    v.begin_drag(win, grabbed, p);
                }
                v.drag_to(win, grabbed, q);
            })),
            on_up: None,
        });
    }

    /// A drag started: a sweep saves the markers for undo and starts at `p`.
    fn begin_drag(&mut self, win: u32, grabbed: Option<MarkerId>, p: (f64, f64)) {
        let Some((view, _, h, node)) = self.gesture_ctx(win) else {
            return;
        };
        if grabbed.is_some() {
            return;
        }
        let Some(nui) = self.ui.get_mut(&node) else {
            return;
        };
        let before = nui.markers;
        nui.markers
            .begin_sweep(view.x_to_sample(p.0), view.y_to_freq(p.1, h));
        if (before.v.is_some() || before.h.is_some())
            && let Some(sw) = self.sono.wins.get_mut(&win)
        {
            sw.undo = Some(before);
        }
        self.mark_dirty();
    }

    /// The pointer moved to `q` during a drag.
    fn drag_to(&mut self, win: u32, grabbed: Option<MarkerId>, q: (f64, f64)) {
        let Some((view, extent, h, node)) = self.gesture_ctx(win) else {
            return;
        };
        let s = view.x_to_sample(q.0).clamp(0.0, extent.len);
        let f = view.y_to_freq(q.1, h).clamp(extent.f_min, extent.f_max);
        let Some(nui) = self.ui.get_mut(&node) else {
            return;
        };
        let before = nui.markers;
        match grabbed {
            Some(id) if id.is_vertical() => {
                nui.markers.set(id, s);
            }
            Some(id) => {
                nui.markers.set(id, f);
            }
            None => nui.markers.sweep_to(s, f),
        }
        if nui.markers != before {
            self.mark_dirty();
        }
    }

    /// Move result box `i` with the mouse.
    fn drag_label(&mut self, win: u32, i: usize, from: (i32, i32), slot: &Slot) {
        let Some(start) = self
            .sono
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
                let Some(sw) = v.sono.wins.get_mut(&win) else {
                    return;
                };
                let plot = (sw.plot.w, sw.plot.h);
                let size = sw.label_sizes.get(i).copied().unwrap_or((0, 0));
                if let Some(l) = sw.labels.get_mut(i) {
                    let to = (start.0 + x - from.0, start.1 + y - from.1);
                    l.offset = Some(sono_tools::clamp_label_offset(to, size, plot));
                }
            })),
            on_up: None,
        });
    }

    /// Middle-drag pans the time axis (an invisible SigNexora extra).
    fn plot_middle_down(&mut self, win: u32, ev: &MouseDownEvent, slot: &Slot) {
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
                    v.with_view(win, |view, extent, w, _| {
                        view.pan_time(-f64::from(dx), extent, w);
                    });
                })),
                on_up: None,
            },
        );
    }

    /// Change the view of window `win` (with its extent, width and height).
    pub(crate) fn with_view(
        &mut self,
        win: u32,
        f: impl FnOnce(&mut View, sn_ui_core::view::Extent, f64, f64),
    ) {
        if let Some(sw) = self.sono.wins.get_mut(&win)
            && let (Some(view), Some(extent)) = (sw.view.as_mut(), sw.extent)
        {
            f(view, extent, f64::from(sw.plot.w), f64::from(sw.plot.h));
        }
    }

    /// A right click in the plot: remove a result box, lock or unlock a marker
    /// by its handle, or open the popup menu.
    fn plot_right_click(&mut self, win: u32, at: gpui::Point<gpui::Pixels>, slot: &Slot) {
        let Some((x, y)) = slot.local(at) else {
            return;
        };
        if let Some(i) = self.label_at(win, x, y) {
            if let Some(sw) = self.sono.wins.get_mut(&win) {
                sw.labels.remove(i);
            }
            return;
        }
        if let Some((view, _, h, node)) = self.gesture_ctx(win)
            && let Some(nui) = self.ui.get_mut(&node)
            && let Some(id) =
                sono_tools::handle_near(&nui.markers, &view, h, (f64::from(x), f64::from(y)))
        {
            nui.markers.toggle_lock(id);
            self.mark_dirty();
            return;
        }
        let pos = (
            f32::from(at.x).round() as i32,
            f32::from(at.y).round() as i32,
        );
        self.open_sono_popup(win, pos);
    }

    /// The mouse wheel, as `sa-app`: zoom time at the pointer; Shift or a
    /// horizontal scroll pans; Alt zooms frequency; Ctrl or Cmd zooms time
    /// faster (egui's pinch zoom).
    fn plot_wheel(&mut self, win: u32, ev: &ScrollWheelEvent, slot: &Slot) {
        let Some((x, y)) = slot.local(ev.position) else {
            return;
        };
        let d = ev.delta.pixel_delta(px(LINE_PX));
        let (dx, dy) = (f64::from(f32::from(d.x)), f64::from(f32::from(d.y)));
        let m = ev.modifiers;
        let (ax, ay) = (f64::from(x), f64::from(y));
        self.with_view(win, |view, extent, w, h| {
            if m.control || m.platform {
                let f = (-dy / 200.0).exp();
                if m.alt {
                    view.zoom_freq(f, ay, h, extent);
                } else {
                    view.zoom_time(f, ax, extent, w);
                }
            } else if m.shift || dx.abs() > dy.abs() {
                let delta = if dx.abs() > dy.abs() { dx } else { dy };
                view.pan_time(-delta, extent, w);
            } else if m.alt {
                view.zoom_freq((-dy * 0.002).exp(), ay, h, extent);
            } else {
                view.zoom_time((-dy * 0.002).exp(), ax, extent, w);
            }
        });
    }

    /// The status bar readout of the pointer over the plot.
    fn plot_hover(&mut self, win: u32, at: gpui::Point<gpui::Pixels>, slot: &Slot) {
        let Some((x, y)) = slot.local(at) else {
            return;
        };
        let Some((view, _, h, node)) = self.gesture_ctx(win) else {
            return;
        };
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let (s, f) = (
            view.x_to_sample(f64::from(x)),
            view.y_to_freq(f64::from(y), h),
        );
        let Some(sw) = self.sono.wins.get_mut(&win) else {
            return;
        };
        let level = sw.cache.level_at(&signal, &view, f64::from(x), f);
        sw.cursor = Some(sono_tools::cursor_readout(
            s,
            Some(f),
            level,
            signal.sample_rate(),
            signal.center_frequency(),
        ));
    }

    /// The readout of the pointer over a sonogram plot, if any.
    pub(crate) fn sono_readout(&self) -> Option<String> {
        self.sono.wins.values().find_map(|w| w.cursor.clone())
    }

    /// SA's navigation scroll bar under the plot of window `win`.
    pub(crate) fn nav_bar(&mut self, win: u32, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        let st = self.sono.wins.get(&win).map(|w| w.nav).unwrap_or_default();
        let id = format!("sw{win}.nav");
        scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect: r,
                vertical: false,
            },
            st,
            lens(move |v: &mut Self| v.sono.wins.get_mut(&win).map(|w| &mut w.nav)),
            Some(action(move |v: &mut Self, _, _| v.nav_moved(win))),
        )
    }

    /// The scroll bar from the view: one unit is a twentieth of the plot
    /// width (an arrow click steps that far, as `sa-app`'s arrows), a page is
    /// the plot.
    pub(crate) fn sync_nav(&mut self, win: u32) {
        let Some(sw) = self.sono.wins.get_mut(&win) else {
            return;
        };
        let (Some(view), Some(extent)) = (sw.view, sw.extent) else {
            return;
        };
        let total = (extent.len / view.samples_per_px.max(f64::MIN_POSITIVE)).ceil() as i64;
        let (nav, unit) = nav_state(view.first_column(), total, sw.plot.w);
        sw.nav_unit = unit;
        sw.nav = nav;
    }

    /// The scroll bar moved: pan the view to its position.
    fn nav_moved(&mut self, win: u32) {
        let Some(sw) = self.sono.wins.get_mut(&win) else {
            return;
        };
        let (pos, unit) = (i64::from(sw.nav.pos), sw.nav_unit);
        let w = f64::from(sw.plot.w);
        if let (Some(view), Some(extent)) = (sw.view.as_mut(), sw.extent) {
            view.start = (pos * unit) as f64 * view.samples_per_px;
            view.resized(extent, w);
        }
    }

    /// Whether `node` is a power-N result, shown as SA's harmonics module.
    pub(crate) fn is_harmonics(&self, node: NodeId) -> bool {
        self.graph
            .get(node)
            .and_then(lineage::edit_of)
            .is_some_and(|e| matches!(e, sn_dsp::ops::Edit::Power { .. }))
    }
}

/// A navigation scroll bar for a view whose first column is `first` of
/// `total` columns in a plot `w` pixels wide, and the columns per unit (a
/// twentieth of the plot: an arrow click steps that far, as `sa-app`'s
/// arrows; a page is the plot). A view at the end puts the thumb at the end
/// (`first / unit` would floor below it, and an arrow press would then not
/// move the view).
pub(crate) fn nav_state(first: i64, total: i64, w: i32) -> (ScrollState, i64) {
    let w = i64::from(w.max(1));
    let unit = (w / 20).max(1);
    let units = |c: i64| i32::try_from(c.div_euclid(unit)).unwrap_or(i32::MAX);
    let max = units(total.max(1) + unit - 1) - 1;
    let mut st = ScrollState::new(0, max.max(0), units(w), units(first));
    if first + w >= total {
        st.set(st.top());
    }
    (st, unit)
}

/// Paint the plot.
fn paint_scene(window: &mut Window, cx: &mut gpui::App, b: gpui::Bounds<gpui::Pixels>, sc: &Scene) {
    let (w, h) = crate::win95::widget::size_of(b);
    let o = b.origin;
    let full = Rect::new(0, 0, w, h);
    window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
        // SA: black where there is no data.
        paint::fill(window, o, full, sc.ink.bg);
        if let Some(img) = &sc.image {
            window
                .paint_image(b, Corners::default(), img.clone(), 0, false)
                .ok();
        }
        if let (Some(view), Some(hz)) = (sc.view, sc.control) {
            let y = view.freq_to_y(hz, f64::from(h)).round() as i32;
            paint::fill(window, o, Rect::new(0, y, w, 1), sc.ink.control);
        }
        if let Some(view) = sc.view {
            paint_markers(window, cx, o, (w, h), &view, &sc.markers, &sc.ink);
        }
        if let Some(lines) = &sc.info {
            paint_info(window, cx, o, w, lines, &sc.ink);
        }
        for (t, r) in &sc.labels {
            paint::fill(window, o, *r, sc.ink.label_bg);
            for e in [
                Rect::new(r.x, r.y, r.w, 2),
                Rect::new(r.x, r.bottom() - 2, r.w, 2),
                Rect::new(r.x, r.y, 2, r.h),
                Rect::new(r.right() - 2, r.y, 2, r.h),
            ] {
                paint::fill(window, o, e, sc.ink.label_border);
            }
            let at = paint::at(o, r.x + 6, r.y + 3);
            text::draw(window, cx, at, t, true, sc.ink.label_text);
        }
        if let Some(n) = &sc.notice {
            let tw = text::width(window, n, false).min(w - 12);
            paint::fill(window, o, Rect::new(4, 2, tw + 4, LINE + 4), sc.ink.info_bg);
            text::draw(window, cx, paint::at(o, 6, 4), n, false, sc.ink.notice);
        }
    });
}

/// SA 6.2 markers: thin lines, triangle handles (V on the top edge pointing
/// down, H on the left edge pointing right), red when locked.
fn paint_markers(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    (w, h): (i32, i32),
    view: &View,
    m: &Markers,
    ink: &PlotInk,
) {
    let s = scheme(cx);
    let hd = HANDLE as f32;
    let color = |id: MarkerId| {
        if m.locked == Some(id) {
            ink.locked
        } else {
            ink.handle
        }
    };
    let mut handles: Vec<(i32, i32, Glyph, Hsla)> = Vec::new();
    if let Some((a, b)) = m.v {
        for (id, smp) in [(MarkerId::V1, a), (MarkerId::V2, b)] {
            let x = view.sample_to_x(smp).floor() as i32;
            if x < -8 || x > w + 8 {
                continue;
            }
            paint::fill(window, o, Rect::new(x, 0, 1, h), ink.line);
            let tri = Glyph::new(15, 11).poly(
                &[(0.0, 0.0), (2.0 * hd + 1.0, 0.0), (hd + 0.5, 1.5 * hd)],
                glyph::BLACK,
            );
            handles.push((x - 7, 0, tri, color(id)));
        }
    }
    if let Some((a, b)) = m.h {
        for (id, f) in [(MarkerId::H1, a), (MarkerId::H2, b)] {
            let y = view.freq_to_y(f, f64::from(h)).floor() as i32;
            if y < -8 || y > h + 8 {
                continue;
            }
            paint::fill(window, o, Rect::new(0, y, w, 1), ink.line);
            let tri = Glyph::new(11, 15).poly(
                &[(0.0, 0.0), (1.5 * hd, hd + 0.5), (0.0, 2.0 * hd + 1.0)],
                glyph::BLACK,
            );
            handles.push((0, y - 7, tri, color(id)));
        }
    }
    for (x, y, g, c) in handles {
        let rgb = hsla_to_rgb(c);
        glyph::draw(window, cx, o, (x, y), &g, Mode::Mono(rgb), &s);
    }
}

/// `0xRRGGBB` of an opaque colour.
fn hsla_to_rgb(c: Hsla) -> u32 {
    let r = gpui::Rgba::from(c);
    let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    (ch(r.r) << 16) | (ch(r.g) << 8) | ch(r.b)
}

/// SA's Info overlay: a translucent dark box at the top right of the plot,
/// white text, one value per line.
fn paint_info(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    w: i32,
    lines: &[String],
    ink: &PlotInk,
) {
    let tw = lines
        .iter()
        .map(|l| text::width(window, l, false))
        .max()
        .unwrap_or(0);
    let bw = tw + 12;
    let r = Rect::new(w - bw - 4, 4, bw, ROW * lines.len() as i32 + 6);
    paint::fill(window, o, r, ink.info_bg);
    for (i, l) in (0_i32..).zip(lines) {
        let at = paint::at(o, r.x + 6, r.y + 3 + i * ROW);
        text::draw(window, cx, at, l, false, ink.info_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nav_bar_follows_the_view() {
        // 2000 columns in a 400 px plot: units of 20 columns, a page of 20.
        let (st, unit) = nav_state(0, 2000, 400);
        assert_eq!((unit, st.pos, st.page), (20, 0, 20));
        assert_eq!(st.top(), 2000 / 20 - 20);
        let (st, _) = nav_state(200, 2000, 400);
        assert_eq!(st.pos, 10, "one unit is 20 columns");
        // Panned (by the wheel) to the clamped end, between two units.
        let (st, _) = nav_state(1590, 2000, 400);
        assert_ne!(st.pos, st.top());
        let (st, _) = nav_state(1601, 2001, 400);
        assert_eq!(st.pos, st.top(), "the end shows the thumb at the end");
        // A view wider than the signal cannot scroll.
        let (st, _) = nav_state(0, 100, 400);
        assert!(!st.enabled());
    }
}
