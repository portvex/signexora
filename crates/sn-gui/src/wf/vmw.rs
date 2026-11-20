//! SA's VMW mode of the WF window (`docs/SA_UI_SPEC.md` 7.3, M-p21, R261/a4):
//! the signal folded at a line period, one raster line per period (every
//! strobe-th), in SA's blue, black, red and orange colours. Top bar: strobe
//! spin edit (1 to 1000), `Y scale - N`, `As time`, `Bright` with its slider,
//! `WaveForm` (back to WF), the large `TS` line period (a left click on a
//! digit adds 1, a right click subtracts 1) and `Info`. Under the raster the
//! navigation scroll bar, on its right the offset scroll bar, then the line
//! period slider and the status `dT = 0:000.000.000 | Speed = 0`. Left drag
//! places the V markers in the line, a double click collapses them, a right
//! drag along a slanted feature straightens it; a right click opens the VMW
//! popup. As `sa-app`'s `vmw_window.rs` and `raster_view.rs`.

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyElement, Context, Corners, Hsla, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, TextRun, Window, canvas,
    div, px,
};
use sn_core::{MeasurementKind, NodeId};
use sn_ui_core::colormap::Colormap;
use sn_ui_core::markers::{MarkerId, Markers};
use sn_ui_core::t;
use sn_ui_core::vmw_tools::{
    self, RasterCache, RasterLook, STROBE_MAX, STROBE_MIN, Y_SCALE_MAX, ts_click, ts_text,
};

use super::plot::{DRAG_START, paint_info};
use super::top::{
    BarItem, check_group, check_group_w, ctl_rect, flow, grey_look, mode_button, slider,
};
use super::{BAR, Frame, WfInk, ink, render_image};
use crate::win95::bars::tooltip;
use crate::win95::button::{Changed, Ctl, check_box};
use crate::win95::colors::{Role, scheme};
use crate::win95::combo::{ComboState, combo_box};
use crate::win95::edit::{EditState, SpinState, edit, spin_edit};
use crate::win95::geom::Rect;
use crate::win95::glyph::{self, Glyph, Mode};
use crate::win95::kit::{Capture, action, lens};
use crate::win95::label::{Align, label};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, scroll_bar};
use crate::win95::text::{self, FONT_SIZE, LINE};
use crate::win95::widget::{Slot, place, size_of};

/// Height of the part under the raster: the line period slider and the status.
const UNDER_H: i32 = 2 + THICKNESS + 2 + 21 + 3;
/// Widths of the top bar's controls.
const STROBE_W: i32 = 56;
const Y_SCALE_W: i32 = 96;
const AS_TIME_W: i32 = 70;
const BRIGHT_W: i32 = 110;
const WAVEFORM_W: i32 = 80;
/// Width of the status field.
const STATUS_W: i32 = 260;
/// Positions of the line period slider.
const PERIOD_STEPS: i32 = 1000;
/// Positions of the navigation scroll bar.
const NAV_STEPS: i32 = 1000;
/// Positions of the offset scroll bar (a step of 0.05, as `sa-app`).
const OFFSET_STEPS: i32 = 40;

/// What the raster image was made from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ImageKey {
    count: u64,
    look: RasterLook,
}

/// The VMW controls SA keeps in the window.
pub(crate) struct VmwWin {
    /// Line period in samples (fractional).
    pub(crate) period: f64,
    /// First sample of line 0.
    pub(crate) start: f64,
    pub(crate) strobe: SpinState,
    pub(crate) y_scale: ComboState,
    /// `Bright`, 0 to 1.
    pub(crate) bright: f64,
    pub(crate) bright_bar: ScrollState,
    /// The offset scroll bar: the signal against zero, -1 to 1.
    pub(crate) offset: f64,
    pub(crate) offset_bar: ScrollState,
    /// `As time`: the line period follows the WF dT.
    pub(crate) as_time: bool,
    pub(crate) info: bool,
    /// V markers as fractions of a line (0 to 1).
    pub(crate) markers: Markers,
    /// SigNexora extra: another colour map instead of SA's.
    pub(crate) own_map: Option<Colormap>,
    pub(crate) raster: RasterCache,
    shown: Option<(ImageKey, Arc<gpui::RenderImage>)>,
    pub(crate) period_bar: ScrollState,
    pub(crate) nav: ScrollState,
    /// The samples the navigation bar spans.
    nav_range: f64,
    /// The status field.
    pub(crate) status: EditState,
    /// The raster at the last layout (client coordinates).
    pub(crate) plot: Rect,
    /// A right drag along a feature: from, to (plot coordinates).
    align: Option<((f64, f64), (f64, f64))>,
    initialised: bool,
}

impl Default for VmwWin {
    fn default() -> Self {
        let y_items: Vec<String> = (1..=Y_SCALE_MAX).map(vmw_tools::y_scale_text).collect();
        let refs: Vec<&str> = y_items.iter().map(String::as_str).collect();
        Self {
            period: vmw_tools::DEFAULT_PERIOD,
            start: 0.0,
            strobe: SpinState::new(1, STROBE_MIN as i64, STROBE_MAX as i64, 1),
            y_scale: ComboState::new(&refs, Some(1)),
            bright: 0.5,
            bright_bar: ScrollState::default(),
            offset: 0.0,
            offset_bar: ScrollState::default(),
            as_time: false,
            info: false,
            markers: Markers::default(),
            own_map: None,
            raster: RasterCache::default(),
            shown: None,
            period_bar: ScrollState::default(),
            nav: ScrollState::default(),
            nav_range: 1.0,
            status: EditState {
                read_only: true,
                ..EditState::default()
            },
            plot: Rect::default(),
            align: None,
            initialised: false,
        }
    }
}

impl VmwWin {
    /// The image on screen, given up (the window closed).
    pub(crate) fn take_image(&mut self) -> Option<Arc<gpui::RenderImage>> {
        self.shown.take().map(|(_, i)| i)
    }

    /// The strobe: the typed value while it is valid, else the spin's.
    pub(crate) fn strobe(&self) -> usize {
        vmw_tools::parse_strobe(&self.strobe.edit.text)
            .unwrap_or_else(|| usize::try_from(self.strobe.value).unwrap_or(1))
    }

    /// Pixels per raster line.
    pub(crate) fn y_scale(&self) -> usize {
        self.y_scale.selected.map_or(2, |i| i + 1)
    }
}

/// The `TS` field's text drawn twice the normal size.
pub(crate) fn big_line(window: &Window, s: &str, color: Hsla) -> gpui::ShapedLine {
    let runs = [TextRun {
        len: s.len(),
        font: text::font(true),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    window
        .text_system()
        .shape_line(s.to_owned().into(), px(FONT_SIZE * 2.0), &runs, None)
}

impl Frame {
    /// The WF V span of window `win` in seconds (`As time` is greyed without
    /// it).
    fn wf_dt(&self, win: u32, fs: f64) -> Option<f64> {
        self.wf
            .wins
            .get(&win)?
            .markers
            .time_range()
            .map(|(a, b)| (b - a) / fs)
    }

    /// The first time VMW opens, start at the WF dT or the measured period;
    /// `As time` follows the WF dT.
    fn vmw_prepare(&mut self, win: u32, fs: f64) {
        let wf_dt = self.wf_dt(win, fs);
        let measured = self
            .measures
            .latest(&MeasurementKind::Period)
            .map(|m| m.value);
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        let v = &mut ww.vmw;
        if !v.initialised {
            v.initialised = true;
            if let Some(p) = wf_dt
                .or(measured)
                .map(|d| d * fs)
                .filter(|p| p.is_finite() && *p >= 1.0)
            {
                v.period = p;
            }
        }
        if wf_dt.is_none() {
            v.as_time = false;
        }
        if v.as_time
            && let Some(dt) = wf_dt
        {
            v.period = (dt * fs).max(1.0);
        }
    }

    /// The body of a WF window in VMW mode.
    pub(crate) fn vmw_body(
        &mut self,
        win: u32,
        node: NodeId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        if self.wf_win(win, node).is_none() {
            return gpui::Empty.into_any_element();
        }
        let fs = self.graph.get(node).map_or(1.0, |n| n.signal.sample_rate());
        self.vmw_prepare(win, fs);
        let (cw, ch) = self.client_of(win);
        let (top, rows) = self.vmw_top_bar(win, fs, cw, window, cx);
        let top_h = super::ROW_H * rows;
        let plot_h = (ch - top_h - BAR - UNDER_H).max(0);
        let plot = Rect::new(0, top_h, (cw - BAR).max(0), plot_h);
        self.vmw_layout(win, node, plot, cx);
        let mut root = div().relative().size_full();
        root = root.child(Self::wf_bar_face(Rect::new(0, 0, cw, top_h), cx));
        for el in top {
            root = root.child(el);
        }
        root = root.child(self.vmw_plot(win, plot, cx));
        let nav = Rect::new(0, plot.bottom(), plot.w, BAR);
        root = root.child(self.vmw_nav_bar(win, nav, cx));
        root = root.child(self.vmw_offset_bar(win, Rect::new(plot.w, top_h, BAR, plot_h), cx));
        root = root.child(Self::wf_bar_face(
            Rect::new(plot.w, plot.bottom(), BAR, BAR),
            cx,
        ));
        let under = Rect::new(0, nav.bottom(), cw, UNDER_H);
        root = root.child(Self::wf_bar_face(under, cx));
        for el in self.vmw_under(win, under, fs, cx) {
            root = root.child(el);
        }
        root.into_any_element()
    }

    /// Ask for the raster of the plot, make its image when it changed, and
    /// bring the scroll bars up to date.
    fn vmw_layout(&mut self, win: u32, node: NodeId, plot: Rect, cx: &gpui::App) {
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let map = self.wf_ink(cx).vmw_map;
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        let v = &mut ww.vmw;
        v.plot = plot;
        let len = signal.len().max(1) as f64;
        let limits = vmw_tools::period_limits(signal.len());
        let pos = (vmw_tools::period_to_slider(v.period, limits) * f64::from(PERIOD_STEPS)).round();
        v.period_bar = ScrollState::new(0, PERIOD_STEPS + 50 - 1, 50, pos as i32);
        v.bright_bar = ScrollState::new(0, 100 + 10 - 1, 10, (v.bright * 100.0).round() as i32);
        let off = ((1.0 - v.offset) / 2.0 * f64::from(OFFSET_STEPS)).round() as i32;
        v.offset_bar = ScrollState::new(0, OFFSET_STEPS + 4 - 1, 4, off);
        if plot.w < 8 || plot.h < 8 {
            return;
        }
        let req = vmw_tools::request(v.start, v.period, v.strobe(), v.y_scale(), (plot.w, plot.h));
        v.raster.sync(&signal, req);
        let shown = v
            .raster
            .shown_request()
            .map_or(0.0, |r| r.rows as f64 * r.period * r.strobe as f64);
        let (t, range) = vmw_tools::nav_position(v.start, shown, len);
        v.nav_range = range;
        let frac = (shown / len).clamp(0.02, 1.0);
        v.nav = if frac >= 1.0 {
            ScrollState::new(0, 0, 1, 0)
        } else {
            let page = (frac * f64::from(NAV_STEPS) / (1.0 - frac))
                .round()
                .clamp(1.0, 1e6) as i32;
            ScrollState::new(
                0,
                NAV_STEPS + page - 1,
                page,
                (t * f64::from(NAV_STEPS)).round() as i32,
            )
        };
        let look = RasterLook {
            bright: v.bright,
            offset: v.offset,
            map,
            own: v.own_map,
        };
        let key = ImageKey {
            count: v.raster.count(),
            look,
        };
        if v.shown.as_ref().is_none_or(|(k, _)| *k != key)
            && let Some(img) = v.raster.image(&look).and_then(|i| render_image(&i))
            && let Some((_, old)) = v.shown.replace((key, img))
        {
            self.wf.stale.push(old);
        }
    }

    /// The top bar of VMW window `win`. Returns its elements and rows.
    fn vmw_top_bar(
        &mut self,
        win: u32,
        fs: f64,
        cw: i32,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> (Vec<AnyElement>, i32) {
        let bright = "Bright".to_owned();
        let bright_w = text::width(window, &bright, true) + 4 + BRIGHT_W;
        let ts_cap_w = text::width(window, &t!("vmw-ts"), true);
        let period = self.wf.wins.get(&win).map_or(0.0, |w| w.vmw.period);
        let ts = ts_text(period / fs);
        let ts_w =
            ts_cap_w + 6 + f32::from(big_line(window, &ts, gpui::black()).width).ceil() as i32 + 8;
        let info_cap = t!("vmw-info");
        let can_time = self.wf_dt(win, fs).is_some();
        let items = vec![
            BarItem {
                w: STROBE_W,
                before: 0,
                build: Box::new(move |f, r, cx| f.vmw_strobe_spin(win, r, cx)),
            },
            BarItem {
                w: Y_SCALE_W,
                before: 0,
                build: Box::new(move |f, r, cx| f.vmw_y_scale_list(win, r, cx)),
            },
            BarItem {
                w: AS_TIME_W,
                before: 0,
                build: Box::new(move |f, r, cx| f.vmw_as_time_box(win, r, can_time, cx)),
            },
            BarItem {
                w: bright_w,
                before: 0,
                build: Box::new(move |f, r, cx| f.vmw_bright_slider(win, r, &bright, cx)),
            },
            BarItem {
                w: WAVEFORM_W,
                before: 0,
                build: Box::new(move |f, r, cx| {
                    let id = format!("wf{win}.waveform");
                    let back = action(move |v: &mut Frame, _, _| {
                        v.wf_set_window_kind(win, crate::app::model::WinKind::Waveform);
                    });
                    f.wf_hot(win, "waveform", ctl_rect(r, r.x, r.w));
                    mode_button(
                        f,
                        cx,
                        &id,
                        ctl_rect(r, r.x, r.w),
                        &t!("vmw-waveform"),
                        &t!("vmw-waveform.hover"),
                        back,
                    )
                }),
            },
            BarItem {
                w: ts_w,
                before: 8,
                build: Box::new(move |f, r, cx| f.vmw_ts_field(win, r, ts_cap_w, fs, cx)),
            },
            BarItem {
                w: check_group_w(window, &info_cap),
                before: 0,
                build: Box::new(move |f, r, cx| {
                    let on = f.wf.wins.get(&win).is_some_and(|w| w.vmw.info);
                    let set: Changed<Frame, bool> = Rc::new(move |v: &mut Frame, on, _, _| {
                        if let Some(w) = v.wf.wins.get_mut(&win) {
                            w.vmw.info = on;
                        }
                    });
                    check_group(
                        f,
                        cx,
                        win,
                        "vinfo",
                        &info_cap,
                        &t!("vmw-info.hover"),
                        r,
                        on,
                        set,
                    )
                }),
            },
        ];
        flow(self, items, cw, cx)
    }

    /// SA's strobe spin edit (1 to 1000).
    fn vmw_strobe_spin(&mut self, win: u32, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        let Some(st) = self.wf.wins.get(&win).map(|w| w.vmw.strobe.clone()) else {
            return gpui::Empty.into_any_element();
        };
        let id = format!("wf{win}.strobe");
        let scope = format!("mdi{win}");
        let l = lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.vmw.strobe));
        let spin = spin_edit(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: ctl_rect(r, r.x, r.w),
                label: "",
                enabled: true,
            },
            &st,
            &l,
            None,
        );
        let tip = place(ctl_rect(r, r.x, r.w - THICKNESS))
            .id(gpui::ElementId::Name(format!("{id}.tip").into()))
            .tooltip(tooltip(&t!("vmw-strobe.hover")));
        div()
            .absolute()
            .size_full()
            .child(tip)
            .child(spin)
            .into_any_element()
    }

    /// The `Y scale - N` list.
    fn vmw_y_scale_list(&mut self, win: u32, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        let Some(st) = self.wf.wins.get(&win).map(|w| w.vmw.y_scale.clone()) else {
            return gpui::Empty.into_any_element();
        };
        let id = format!("wf{win}.yscale");
        let scope = format!("mdi{win}");
        let l = lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.vmw.y_scale));
        let el = combo_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: ctl_rect(r, r.x, r.w),
                label: "",
                enabled: true,
            },
            &st,
            l,
            None,
        );
        let tip = place(ctl_rect(r, r.x, r.w - THICKNESS))
            .id(gpui::ElementId::Name(format!("{id}.tip").into()))
            .tooltip(tooltip(&t!("vmw-y-scale.hover")));
        div()
            .absolute()
            .size_full()
            .child(tip)
            .child(el)
            .into_any_element()
    }

    /// `As time` (greyed until the WF V markers select a span).
    fn vmw_as_time_box(
        &mut self,
        win: u32,
        r: Rect,
        can_time: bool,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let on = self.wf.wins.get(&win).is_some_and(|w| w.vmw.as_time);
        let id = format!("wf{win}.astime");
        let scope = format!("mdi{win}");
        let set: Changed<Frame, bool> = Rc::new(move |v: &mut Frame, on, _, _| {
            if let Some(w) = v.wf.wins.get_mut(&win) {
                w.vmw.as_time = on;
            }
        });
        let caption = "As time".to_owned();
        let el = check_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: ctl_rect(r, r.x, r.w),
                label: &caption,
                enabled: can_time,
            },
            on,
            set,
        );
        let tip = if can_time {
            t!("vmw-as-time.hover")
        } else {
            t!("vmw-as-time.disabled")
        };
        let tip_el = place(ctl_rect(r, r.x, r.w))
            .id(gpui::ElementId::Name(format!("{id}.tip").into()))
            .tooltip(tooltip(&tip));
        div()
            .absolute()
            .size_full()
            .child(tip_el)
            .child(el)
            .into_any_element()
    }

    /// Bold `Bright` and its slider.
    fn vmw_bright_slider(
        &mut self,
        win: u32,
        r: Rect,
        caption: &str,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let cap_w = r.w - 4 - BRIGHT_W;
        let st = self
            .wf
            .wins
            .get(&win)
            .map(|w| w.vmw.bright_bar)
            .unwrap_or_default();
        let bar_r = Rect::new(
            r.x + cap_w + 4,
            r.y + (super::ROW_H - THICKNESS) / 2,
            BRIGHT_W,
            THICKNESS,
        );
        let bar = slider(
            self,
            cx,
            &format!("wf{win}.bright"),
            bar_r,
            st,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.vmw.bright_bar)),
            action(move |v: &mut Self, _, _| {
                if let Some(w) = v.wf.wins.get_mut(&win) {
                    w.vmw.bright = f64::from(w.vmw.bright_bar.pos) / 100.0;
                }
            }),
        );
        div()
            .absolute()
            .size_full()
            .child(label(
                cx,
                ctl_rect(r, r.x, cap_w),
                caption,
                true,
                true,
                Align::Left,
            ))
            .child(bar)
            .into_any_element()
    }

    /// Bold `TS` and the large line period: a left click on a digit adds 1,
    /// a right click subtracts 1.
    fn vmw_ts_field(
        &mut self,
        win: u32,
        r: Rect,
        cap_w: i32,
        fs: f64,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let s = scheme(cx);
        let text_color = s.of(Role::Text);
        let period = self.wf.wins.get(&win).map_or(0.0, |w| w.vmw.period);
        let value = ts_text(period / fs);
        let slot = Slot::default();
        let paint_slot = slot.clone();
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                paint_slot.set(b);
                let (_, h) = size_of(b);
                let line = big_line(window, &value, text_color);
                let at = paint::at(b.origin, 0, (h - 2 * LINE) / 2);
                line.paint(at, px(2.0 * LINE as f32), window, cx).ok();
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let value_r = Rect::new(r.x + cap_w + 6, r.y, (r.w - cap_w - 6).max(0), r.h);
        self.wf_hot(win, "ts", value_r);
        let id = format!("wf{win}.ts");
        let click = |up: bool, slot: Slot| {
            move |v: &mut Frame,
                  ev: &MouseDownEvent,
                  window: &mut Window,
                  cx: &mut Context<'_, Frame>| {
                v.vmw_ts_clicked(win, ev, &slot, up, fs, window);
                cx.notify();
            }
        };
        let field = place(value_r)
            .id(gpui::ElementId::Name(id.into()))
            .tooltip(tooltip(&t!("vmw-ts.hover")))
            .child(paint)
            .on_mouse_down(MouseButton::Left, cx.listener(click(true, slot.clone())))
            .on_mouse_down(MouseButton::Right, cx.listener(click(false, slot)));
        div()
            .absolute()
            .size_full()
            .child(label(
                cx,
                ctl_rect(r, r.x, cap_w),
                &t!("vmw-ts"),
                true,
                true,
                Align::Left,
            ))
            .child(field)
            .into_any_element()
    }

    /// A click on digit under `ev` of the `TS` field.
    fn vmw_ts_clicked(
        &mut self,
        win: u32,
        ev: &MouseDownEvent,
        slot: &Slot,
        up: bool,
        fs: f64,
        window: &Window,
    ) {
        let Some((x, _)) = slot.local(ev.position) else {
            return;
        };
        self.kit.focus = None;
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        let text = ts_text(ww.vmw.period / fs);
        let line = big_line(window, &text, gpui::black());
        let Some(index) = line.index_for_x(px(x as f32)) else {
            return;
        };
        let seconds = ts_click(ww.vmw.period / fs, index, up, 1.0 / fs);
        ww.vmw.period = (seconds * fs).max(1.0);
        ww.vmw.as_time = false;
    }

    /// The raster element of window `win` at `r`.
    fn vmw_plot(&mut self, win: u32, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        let wink = self.wf_ink(cx);
        let fs = self
            .node_of(win)
            .and_then(|n| self.graph.get(n))
            .map_or(1.0, |n| n.signal.sample_rate());
        let Some(ww) = self.wf.wins.get(&win) else {
            return gpui::Empty.into_any_element();
        };
        let v = &ww.vmw;
        let req = vmw_tools::request(v.start, v.period, v.strobe(), v.y_scale(), (r.w, r.h));
        let shown = v.raster.shown_request();
        let scene = RasterScene {
            image: v.shown.as_ref().map(|(_, i)| i.clone()),
            current: shown.is_some_and(|s| vmw_tools::same_settings(s, req)),
            markers: v.markers,
            info: v.info.then(|| {
                // The overlay describes the image on screen, built with its own period.
                let period = shown.map_or(v.period, |s| s.period);
                let span = v
                    .markers
                    .time_range()
                    .map_or(0.0, |(a, b)| (b - a) * period / fs);
                vmw_tools::info_lines(span, period / fs)
            }),
            error: v.raster.error().map(str::to_owned),
            align: v.align,
            ink: wink,
        };
        let slot = Slot::default();
        let paint_slot = slot.clone();
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                paint_slot.set(b);
                paint_raster(window, cx, b, &scene);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let (s1, s2) = (slot.clone(), slot);
        place(r)
            .id(gpui::ElementId::Name(format!("wf{win}.raster").into()))
            .child(paint)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.vmw_left_down(win, ev, &s1);
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.vmw_right_down(win, ev, &s2);
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    /// Left button in the raster: double click collapses the V markers, a
    /// drag moves one or places both in the line.
    fn vmw_left_down(&mut self, win: u32, ev: &MouseDownEvent, slot: &Slot) {
        let Some((x, _)) = slot.local(ev.position) else {
            return;
        };
        self.kit.focus = None;
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        let w = f64::from(ww.vmw.plot.w.max(1));
        let frac = |x: f64| (x / w).clamp(0.0, 1.0);
        let m = &mut ww.vmw.markers;
        if ev.click_count >= 2 {
            let f = frac(f64::from(x));
            m.v = Some((f, f));
            m.locked = None;
            return;
        }
        let near = [MarkerId::V1, MarkerId::V2].into_iter().find(|id| {
            m.get(*id)
                .is_some_and(|v| (v * w - f64::from(x)).abs() <= 6.0)
        });
        let p0 = f64::from(x);
        let started = Rc::new(std::cell::Cell::new(false));
        let slot = slot.clone();
        self.kit.capture(Capture {
            on_move: Some(Rc::new(move |v: &mut Self, at, _, _| {
                let Some((x, _)) = slot.local(at) else {
                    return;
                };
                let q = f64::from(x);
                let Some(ww) = v.wf.wins.get_mut(&win) else {
                    return;
                };
                let w = f64::from(ww.vmw.plot.w.max(1));
                let m = &mut ww.vmw.markers;
                if !started.get() {
                    if (q - p0).abs() < DRAG_START {
                        return;
                    }
                    started.set(true);
                    if near.is_none() {
                        m.begin_sweep((p0 / w).clamp(0.0, 1.0), 0.0);
                        m.h = None;
                    }
                }
                m.set(near.unwrap_or(MarkerId::V2), (q / w).clamp(0.0, 1.0));
            })),
            on_up: None,
        });
    }

    /// Right button in the raster: a drag along a slanted feature corrects the
    /// period; a click opens the VMW popup.
    fn vmw_right_down(&mut self, win: u32, ev: &MouseDownEvent, slot: &Slot) {
        let Some((x, y)) = slot.local(ev.position) else {
            return;
        };
        let p0 = (f64::from(x), f64::from(y));
        let slot2 = slot.clone();
        let slot = slot.clone();
        self.kit.capture(Capture {
            on_move: Some(Rc::new(move |v: &mut Self, at, _, _| {
                if let (Some((x, y)), Some(ww)) = (slot.local(at), v.wf.wins.get_mut(&win)) {
                    ww.vmw.align = Some((p0, (f64::from(x), f64::from(y))));
                }
            })),
            on_up: Some(Rc::new(move |v: &mut Self, at, _, _| {
                let p1 = slot2
                    .local(at)
                    .map_or(p0, |(x, y)| (f64::from(x), f64::from(y)));
                if let Some(ww) = v.wf.wins.get_mut(&win) {
                    ww.vmw.align = None;
                }
                if (p1.0 - p0.0).hypot(p1.1 - p0.1) < DRAG_START {
                    let pos = (
                        f32::from(at.x).round() as i32,
                        f32::from(at.y).round() as i32,
                    );
                    v.open_vmw_popup(win, pos);
                } else {
                    v.vmw_straighten(win, (p1.0 - p0.0, p1.1 - p0.1));
                }
            })),
        });
    }

    /// A right drag of `(dx, dy)` pixels: correct the period so the feature
    /// runs straight down (measured on the raster on screen).
    pub(crate) fn vmw_straighten(&mut self, win: u32, (dx, dy): (f64, f64)) {
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        let v = &mut ww.vmw;
        let Some(shown) = v.raster.shown_request() else {
            return;
        };
        if dy.abs() <= 5.0 {
            return;
        }
        if let Some(drift) = vmw_tools::drag_drift(
            (dx, dy),
            (f64::from(v.plot.w), f64::from(v.plot.h)),
            (shown.cols, shown.rows),
            shown.period,
        ) {
            v.period = (shown.period + drift / shown.strobe.max(1) as f64).max(1.0);
            v.as_time = false;
        }
    }

    /// The navigation scroll bar under the raster (position in the file).
    fn vmw_nav_bar(&mut self, win: u32, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        let st = self
            .wf
            .wins
            .get(&win)
            .map(|w| w.vmw.nav)
            .unwrap_or_default();
        let id = format!("wf{win}.vnav");
        scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect: r,
                vertical: false,
            },
            st,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.vmw.nav)),
            Some(action(move |v: &mut Self, _, _| v.vmw_nav_moved(win))),
        )
    }

    pub(crate) fn vmw_nav_moved(&mut self, win: u32) {
        let len = self
            .node_of(win)
            .and_then(|n| self.graph.get(n))
            .map_or(1.0, |n| n.signal.len() as f64);
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            let v = &mut ww.vmw;
            let t = f64::from(v.nav.pos) / f64::from(NAV_STEPS);
            v.start = (t * v.nav_range).clamp(0.0, (len - v.period).max(0.0));
        }
    }

    /// The offset scroll bar right of the raster (the signal against zero).
    fn vmw_offset_bar(&mut self, win: u32, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        let st = self
            .wf
            .wins
            .get(&win)
            .map(|w| w.vmw.offset_bar)
            .unwrap_or_default();
        let id = format!("wf{win}.voff");
        scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect: r,
                vertical: true,
            },
            st,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.vmw.offset_bar)),
            Some(action(move |v: &mut Self, _, _| {
                if let Some(w) = v.wf.wins.get_mut(&win) {
                    let t = f64::from(w.vmw.offset_bar.pos) / f64::from(OFFSET_STEPS);
                    w.vmw.offset = (1.0 - 2.0 * t).clamp(-1.0, 1.0);
                }
            })),
        )
    }

    /// Under the raster: the line period slider and the status field.
    fn vmw_under(
        &mut self,
        win: u32,
        r: Rect,
        fs: f64,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let len = self
            .node_of(win)
            .and_then(|n| self.graph.get(n))
            .map_or(0, |n| n.signal.len());
        let look = grey_look(cx);
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return Vec::new();
        };
        let v = &mut ww.vmw;
        let span = v
            .markers
            .time_range()
            .map_or(0.0, |(a, b)| (b - a) * v.period / fs);
        let status = vmw_tools::status_text(span);
        if v.status.text != status {
            v.status.set_text(&status);
        }
        v.status.read_only = true;
        v.status.look = Some(look);
        let (bar_st, field_st) = (v.period_bar, v.status.clone());
        let bar_r = Rect::new(2, r.y + 2, (r.w - 4).max(0), THICKNESS);
        let limits = vmw_tools::period_limits(len);
        let bar = slider(
            self,
            cx,
            &format!("wf{win}.period"),
            bar_r,
            bar_st,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.vmw.period_bar)),
            action(move |v: &mut Self, _, _| {
                if let Some(w) = v.wf.wins.get_mut(&win) {
                    let t = f64::from(w.vmw.period_bar.pos) / f64::from(PERIOD_STEPS);
                    w.vmw.period = vmw_tools::slider_to_period(t, limits).max(1.0);
                    w.vmw.as_time = false;
                }
            }),
        );
        let tip = place(bar_r)
            .id(gpui::ElementId::Name(format!("wf{win}.period.tip").into()))
            .tooltip(tooltip(&t!("vmw-period-slider.hover")));
        let id = format!("wf{win}.status");
        let scope = format!("mdi{win}");
        let field_r = Rect::new(2, bar_r.bottom() + 2, STATUS_W, 21);
        let field = edit(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: field_r,
                label: "",
                enabled: true,
            },
            &field_st,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.vmw.status)),
            None,
        );
        let field_tip = place(field_r)
            .id(gpui::ElementId::Name(format!("{id}.tip").into()))
            .tooltip(tooltip(&t!("vmw-status.hover")));
        vec![
            tip.into_any_element(),
            bar,
            field_tip.into_any_element(),
            field,
        ]
    }
}

/// What the raster's paint callback draws.
struct RasterScene {
    image: Option<Arc<gpui::RenderImage>>,
    /// The image was built for the current settings (else it is dimmed).
    current: bool,
    markers: Markers,
    info: Option<Vec<String>>,
    error: Option<String>,
    align: Option<((f64, f64), (f64, f64))>,
    ink: WfInk,
}

/// Paint the raster: the image (dimmed while a new one is computed), the V
/// markers, the right-drag line, the Info overlay and an error.
fn paint_raster(
    window: &mut Window,
    cx: &mut gpui::App,
    b: gpui::Bounds<gpui::Pixels>,
    sc: &RasterScene,
) {
    let (w, h) = size_of(b);
    let o = b.origin;
    let s = scheme(cx);
    window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
        paint::fill(
            window,
            o,
            Rect::new(0, 0, w, h),
            ink(sn_ui_core::color::Rgba8::BLACK),
        );
        if let Some(img) = &sc.image {
            window
                .paint_image(b, Corners::default(), img.clone(), 0, false)
                .ok();
            if !sc.current {
                let dim = ink(sn_ui_core::color::Rgba8::black(145));
                paint::fill(window, o, Rect::new(0, 0, w, h), dim);
            }
        }
        if let Some((a, c)) = sc.markers.v {
            for (id, f) in [(MarkerId::V1, a), (MarkerId::V2, c)] {
                // A marker at the right edge (1.0) stays on the raster.
                let x = ((f * f64::from(w)).floor() as i32).clamp(0, (w - 1).max(0));
                paint::fill(window, o, Rect::new(x, 0, 1, h), ink(sc.ink.v_line));
                let color = if sc.markers.locked == Some(id) {
                    sc.ink.locked
                } else {
                    sc.ink.handle
                };
                let tri =
                    Glyph::new(13, 10).poly(&[(0.0, 0.0), (13.0, 0.0), (6.5, 9.0)], glyph::BLACK);
                glyph::draw(
                    window,
                    cx,
                    o,
                    (x - 6, 0),
                    &tri,
                    Mode::Mono(super::rgb_u32(color)),
                    &s,
                );
            }
        }
        if let Some((p0, p1)) = sc.align {
            for (x, y) in crate::win95::geom::pixel_line(
                p0.0.round() as i32,
                p0.1.round() as i32,
                p1.0.round() as i32,
                p1.1.round() as i32,
            ) {
                paint::fill(window, o, Rect::new(x, y, 2, 2), ink(sc.ink.h_line));
            }
        }
        if let Some(lines) = &sc.info {
            paint_info(window, cx, o, w, lines, &sc.ink);
        }
        if let Some(e) = &sc.error {
            let tw = text::width(window, e, false).min(w - 16);
            paint::fill(
                window,
                o,
                Rect::new(5, 3, tw + 6, LINE + 6),
                ink(sc.ink.info_bg),
            );
            text::draw(
                window,
                cx,
                paint::at(o, 8, 6),
                e,
                false,
                ink(sc.ink.info_text),
            );
        }
    });
}
