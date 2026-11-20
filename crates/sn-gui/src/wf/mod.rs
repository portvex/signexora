//! SA's WF (waveform) window and its VMW mode (`docs/SA_UI_SPEC.md` section
//! 7) on the Windows 95 kit, as `sa-app`'s `wf_window.rs`, `waveform.rs`,
//! `vmw_window.rs`, `raster_view.rs` and `app_wf.rs` draw them.
//!
//! The module is a child of `app` (declared there with a path), so it extends
//! [`Frame`] directly. The window keeps its own markers and view; the LPF/RRC
//! filter and the CCF redraw the same window, Copy and ACF open new WF
//! windows (`(wfc0)`, `(wf_acf)`), and `- VMW -` / `WaveForm` switch the same
//! window between the two modes.
//!
//! - `top.rs`, `bottom.rs`: the WF toolbars and their small widgets.
//! - `plot.rs`: the trace image, markers, Info, Count freqs boxes, the
//!   navigation and DC offset scroll bars and the mouse.
//! - `popup.rs`: the popup menu with its `SigNexora tools` submenu, and the VMW
//!   popup.
//! - `actions.rs`: the jobs of the window (filter, Copy, ACF, CCF, Find
//!   tones), Revert and Count freqs.
//! - `vmw.rs`: the VMW mode.
//! - `icons.rs`: the toolbar glyphs.

mod actions;
mod bottom;
mod icons;
mod plot;
pub(crate) mod popup;
#[cfg(test)]
mod tests;
mod top;
mod vmw;

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{AnyElement, Context, IntoElement, ParentElement, RenderImage, Styled, Window, div};
use sn_core::NodeId;
use sn_ui_core::color::Rgba8;
use sn_ui_core::markers::Markers;
use sn_ui_core::sono_cache::SonoImage;
use sn_ui_core::sono_tools::ViewAction;
use sn_ui_core::theme::ThemeChoice;
use sn_ui_core::view::View;
use sn_ui_core::wf_plot::{CcfOverlay, SumCache, WfColors};
use sn_ui_core::wf_tools::{self, GridLabel, WF_LPF_PRESETS};

pub(crate) use actions::WfOutput;
use vmw::VmwWin;

use super::Frame;
use super::model::WinKind;
use crate::win95::colors::{Role, hsla, palette, scheme};
use crate::win95::edit::EditState;
use crate::win95::geom::Rect;
use crate::win95::scrollbar::{self, ScrollState};

/// Height of a toolbar row.
pub(crate) const ROW_H: i32 = 30;
/// Height of the bottom toolbar.
pub(crate) const BOT_H: i32 = 26;
/// Height of the navigation scroll bar, width of the DC offset scroll bar.
pub(crate) const BAR: i32 = scrollbar::THICKNESS;

/// Where the parts of a WF window go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub(crate) top: Rect,
    pub(crate) plot: Rect,
    /// The DC offset scroll bar right of the plot.
    pub(crate) dc: Rect,
    pub(crate) nav: Rect,
    pub(crate) bottom: Rect,
}

impl Layout {
    /// The parts of a client area `cw` x `ch` whose top toolbar has `rows` rows.
    pub(crate) fn new(cw: i32, ch: i32, rows: i32) -> Self {
        let top_h = ROW_H * rows.max(1);
        let plot_h = (ch - top_h - BOT_H - BAR).max(0);
        let plot_w = (cw - BAR).max(0);
        Self {
            top: Rect::new(0, 0, cw, top_h),
            plot: Rect::new(0, top_h, plot_w, plot_h),
            dc: Rect::new(plot_w, top_h, BAR, plot_h),
            nav: Rect::new(0, top_h + plot_h, plot_w, BAR),
            bottom: Rect::new(0, top_h + plot_h + BAR, cw, BOT_H),
        }
    }
}

/// What the trace image was made from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TraceKey {
    node: NodeId,
    view: View,
    size: (i32, i32),
    scale: f32,
    offset: f64,
    show_q: bool,
    /// The Sum histogram's arrival count (Sum on).
    sum: Option<u64>,
    /// The CCF fragment and its signal's length.
    ccf: Option<((usize, usize), usize)>,
    colors: WfColors,
}

/// The state of one WF window that SA keeps in the window's controls.
// Independent check boxes and flags, as SA's controls.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct WfWin {
    pub(crate) node: NodeId,
    pub(crate) view: Option<View>,
    pub(crate) history: Vec<View>,
    /// A view request applied at the next layout.
    pub(crate) action: Option<ViewAction>,
    /// The plot at the last layout (client coordinates).
    pub(crate) plot: Rect,
    /// V markers in samples, H markers in the signal's units.
    pub(crate) markers: Markers,
    /// The markers before the last sweep or collapse.
    pub(crate) undo: Option<Markers>,
    /// The markers were placed for this signal (collapsed V, H at the edges).
    pub(crate) placed: bool,
    /// Largest absolute value of the signal (the DC offset scroll range).
    pub(crate) amplitude: Option<f64>,
    pub(crate) info: bool,
    pub(crate) sum: bool,
    pub(crate) lpf: EditState,
    /// The RRC filter is on (its caption is black, else greyed).
    pub(crate) rrc: bool,
    /// RRC roll-off; 0 until set (SA's `0,50`).
    pub(crate) alpha: f64,
    pub(crate) alpha_field: EditState,
    pub(crate) alpha_bar: ScrollState,
    /// The short-interval frequency field (`0-0` until measured).
    pub(crate) freq: Option<String>,
    /// The signal before the LPF/RRC filter (Revert to original).
    pub(crate) original: Option<NodeId>,
    pub(crate) ccf: Option<CcfOverlay>,
    pub(crate) labels: Vec<GridLabel>,
    /// The boxes' sizes as last drawn, for the mouse.
    pub(crate) label_sizes: Vec<(i32, i32)>,
    /// Count freqs waits for its tones, found between these H marker values
    /// (the grid is fitted to the same bounds, even if the markers move).
    pub(crate) count_pending: Option<(f64, f64)>,
    /// Tones arrived for the waiting Count freqs.
    pub(crate) tones_ready: bool,
    pub(crate) sum_cache: SumCache,
    pub(crate) nav: ScrollState,
    /// Columns per navigation scroll bar unit.
    pub(crate) nav_unit: i64,
    pub(crate) dc: ScrollState,
    /// The pointer readout over the plot.
    pub(crate) cursor: Option<String>,
    shown: Option<(TraceKey, Arc<RenderImage>)>,
    /// The VMW mode of the same window.
    pub(crate) vmw: VmwWin,
    /// Where some controls were last placed (client coordinates).
    pub(crate) hot: HashMap<&'static str, Rect>,
}

impl std::fmt::Debug for WfWin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WfWin")
            .field("node", &self.node)
            .field("markers", &self.markers)
            .field("view", &self.view)
            .finish_non_exhaustive()
    }
}

impl WfWin {
    fn new(node: NodeId, br: Option<f64>, info: bool) -> Self {
        let read_only = |t: &str| EditState {
            read_only: true,
            ..EditState::new(t)
        };
        Self {
            node,
            view: None,
            history: Vec::new(),
            action: None,
            plot: Rect::default(),
            markers: Markers::default(),
            undo: None,
            placed: false,
            amplitude: None,
            info,
            sum: false,
            lpf: EditState::new(&wf_tools::lpf_default(br)),
            rrc: false,
            alpha: 0.0,
            alpha_field: read_only(&sn_ui_core::format::dec(0.5, 2)),
            alpha_bar: ScrollState::default(),
            freq: None,
            original: None,
            ccf: None,
            labels: Vec::new(),
            label_sizes: Vec::new(),
            count_pending: None,
            tones_ready: false,
            sum_cache: SumCache::default(),
            nav: ScrollState::default(),
            nav_unit: 1,
            dc: ScrollState::default(),
            cursor: None,
            shown: None,
            vmw: VmwWin::default(),
            hot: HashMap::new(),
        }
    }

    /// The window now shows `node` (a filter result or the original): the
    /// controls and markers stay, the CCF view and cached images go.
    fn rebind(&mut self, node: NodeId) {
        self.node = node;
        self.ccf = None;
        self.amplitude = None;
        self.cursor = None;
        self.sum_cache.release();
        self.vmw.raster.release();
        self.count_pending = None;
        self.tones_ready = false;
    }
}

/// The WF windows of the frame.
#[derive(Default)]
pub(crate) struct WfHub {
    pub(crate) wins: HashMap<u32, WfWin>,
    /// The window whose popup menu (or LPF preset list) is open.
    pub(crate) popup_win: Option<u32>,
    /// The operations of the open `SigNexora tools` > `Operations` submenu.
    pub(crate) ops: Vec<wf_tools::OpRun>,
    /// The tones of the last Find tones (reverse synthesis).
    pub(crate) last_tones: Vec<f64>,
    /// Images replaced since the last frame, to free from the sprite atlas.
    stale: Vec<Arc<RenderImage>>,
}

impl std::fmt::Debug for WfHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WfHub")
            .field("wins", &self.wins.len())
            .field("popup_win", &self.popup_win)
            .finish_non_exhaustive()
    }
}

/// A GPUI image (BGRA) of an RGBA image.
pub(crate) fn render_image(img: &SonoImage) -> Option<Arc<RenderImage>> {
    let mut bytes = Vec::with_capacity(img.pixels.len() * 4);
    for p in &img.pixels {
        bytes.extend_from_slice(&[p.b, p.g, p.r, p.a]);
    }
    let w = u32::try_from(img.width).ok()?;
    let h = u32::try_from(img.height).ok()?;
    let buf = image::RgbaImage::from_raw(w, h, bytes)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buf)])))
}

/// The WF window's colours: SA's own in the Classic theme (`sa-app`'s
/// `CLASSIC_CHROME`), the theme's in the others.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WfInk {
    pub(crate) trace: WfColors,
    pub(crate) handle: Rgba8,
    pub(crate) locked: Rgba8,
    pub(crate) v_line: Rgba8,
    pub(crate) h_line: Rgba8,
    pub(crate) info_bg: Rgba8,
    pub(crate) info_text: Rgba8,
    pub(crate) dim_text: Rgba8,
    /// Light green faces of the bottom toolbar's buttons.
    pub(crate) button_face: Rgba8,
    /// The light blue `- VMW -` and `WaveForm` buttons and their text.
    pub(crate) mode_face: Rgba8,
    pub(crate) mode_text: Rgba8,
    /// The Count freqs box.
    pub(crate) grid_bg: Rgba8,
    pub(crate) grid_border: Rgba8,
    pub(crate) grid_text: Rgba8,
    /// SA's VMW colours.
    pub(crate) vmw_map: [Rgba8; 4],
}

/// `0xRRGGBB` as a colour.
fn rgb8(c: u32) -> Rgba8 {
    let ch = |s: u32| ((c >> s) & 0xFF) as u8;
    Rgba8::rgb(ch(16), ch(8), ch(0))
}

impl Frame {
    /// The WF window's colours under the current theme.
    pub(crate) fn wf_ink(&self, cx: &gpui::App) -> WfInk {
        let p = palette(cx);
        let trace = WfColors {
            bg: p.plot_bg,
            zero: p.grid,
            trace: p.trace_i,
            trace_hz: p.trace_alt,
            trace_q: p.trace_q,
            sum: p.trace_alt,
            sum_hz: p.trace_sum,
            fragment: p.trace_sum,
        };
        if self.prefs.theme == ThemeChoice::Classic {
            return WfInk {
                trace: WfColors {
                    bg: Rgba8::BLACK,
                    zero: Rgba8::rgb(128, 128, 128),
                    ..trace
                },
                handle: Rgba8::rgb(0, 128, 0),
                locked: Rgba8::rgb(255, 0, 0),
                v_line: Rgba8::WHITE,
                h_line: Rgba8::rgb(255, 255, 0),
                info_bg: Rgba8::black(150),
                info_text: Rgba8::WHITE,
                dim_text: p.plot_text_dim,
                button_face: Rgba8::rgb(200, 240, 200),
                mode_face: Rgba8::rgb(190, 225, 250),
                mode_text: Rgba8::rgb(0, 0, 128),
                grid_bg: Rgba8::rgb(0, 128, 0),
                grid_border: Rgba8::rgb(255, 255, 0),
                grid_text: Rgba8::WHITE,
                vmw_map: sn_ui_core::vmw_tools::SA_MAP,
            };
        }
        let s = scheme(cx);
        WfInk {
            trace,
            handle: p.ok,
            locked: p.marker_locked,
            v_line: p.plot_text,
            h_line: p.trace_sum,
            info_bg: p.overlay_bg,
            info_text: p.plot_text,
            dim_text: p.plot_text_dim,
            button_face: rgb8(s.rgb(Role::Face)),
            mode_face: rgb8(s.rgb(Role::Selection)),
            mode_text: rgb8(s.rgb(Role::SelectionText)),
            grid_bg: rgb8(s.rgb(Role::Selection)),
            grid_border: p.ok,
            grid_text: rgb8(s.rgb(Role::SelectionText)),
            vmw_map: [p.trace_extra, p.plot_bg, p.cursor, p.trace_sum],
        }
    }

    /// The WF window `win`'s state, made on first use (a new signal resets it).
    pub(crate) fn wf_win(&mut self, win: u32, node: NodeId) -> Option<&mut WfWin> {
        self.graph.get(node)?;
        let br = self
            .measured(&sn_core::MeasurementKind::BaudRate)
            .map(|m| m.value);
        let info = self.prefs.info_default;
        let ww = self
            .wf
            .wins
            .entry(win)
            .or_insert_with(|| WfWin::new(node, br, info));
        if ww.node != node {
            *ww = WfWin::new(node, br, info);
        }
        Some(ww)
    }

    /// Forget the state of windows that closed, and free replaced images.
    pub(crate) fn wf_prune(&mut self, window: &mut Window) {
        let open: Vec<u32> = self
            .signal_windows()
            .into_iter()
            .filter(|(_, _, k)| matches!(k, WinKind::Waveform | WinKind::Raster))
            .map(|(w, _, _)| w)
            .collect();
        let gone: Vec<u32> = self
            .wf
            .wins
            .keys()
            .copied()
            .filter(|w| !open.contains(w))
            .collect();
        for w in gone {
            if let Some(mut ww) = self.wf.wins.remove(&w) {
                ww.sum_cache.release();
                ww.vmw.raster.release();
                if let Some((_, img)) = ww.shown.take() {
                    self.wf.stale.push(img);
                }
                if let Some(img) = ww.vmw.take_image() {
                    self.wf.stale.push(img);
                }
            }
        }
        for img in std::mem::take(&mut self.wf.stale) {
            window.drop_image(img).ok();
        }
    }

    /// Work that needs the poll loop: histograms, rasters, a waiting Count
    /// freqs.
    pub(crate) fn wf_busy(&self) -> bool {
        self.wf
            .wins
            .values()
            .any(|w| w.sum_cache.busy() || w.vmw.raster.busy() || w.count_pending.is_some())
    }

    /// Switch window `win` to `kind` (`- VMW -` and `WaveForm`).
    pub(crate) fn wf_set_window_kind(&mut self, win: u32, kind: WinKind) {
        for (id, k) in &mut self.kinds {
            if *id == win
                && let super::ChildKind::Signal(n, _) = *k
            {
                *k = super::ChildKind::Signal(n, kind);
            }
        }
        self.kit.focus = None;
    }

    /// Window `win` now shows `node` in the same view (the LPF filter and
    /// Revert).
    pub(crate) fn wf_rebind_window(&mut self, win: u32, node: NodeId) {
        for (id, k) in &mut self.kinds {
            if *id == win
                && let super::ChildKind::Signal(_, kind) = *k
            {
                *k = super::ChildKind::Signal(node, kind);
            }
        }
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.rebind(node);
        }
    }

    /// The body of WF window `win` showing `node`.
    pub(crate) fn wf_body(
        &mut self,
        win: u32,
        node: NodeId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        if self.wf_win(win, node).is_none() {
            return gpui::Empty.into_any_element();
        }
        let (cw, ch) = self.client_of(win);
        let (top, rows) = self.wf_top_toolbar(win, cw, window, cx);
        let layout = Layout::new(cw, ch, rows);
        self.wf_layout(win, node, layout, cx);
        let mut root = div().relative().size_full();
        root = root.child(Self::wf_bar_face(layout.top, cx));
        for el in top {
            root = root.child(el);
        }
        root = root.child(self.wf_plot_element(win, node, layout.plot, window, cx));
        root = root.child(self.wf_nav_bar(win, layout.nav, cx));
        root = root.child(self.wf_dc_bar(win, layout.dc, cx));
        root = root.child(Self::wf_bar_face(
            Rect::new(layout.dc.x, layout.nav.y, BAR, BAR),
            cx,
        ));
        root = root.child(Self::wf_bar_face(layout.bottom, cx));
        for el in self.wf_bottom_toolbar(win, node, layout.bottom, cx) {
            root = root.child(el);
        }
        root.into_any_element()
    }

    /// A plain face-coloured panel at `r`.
    pub(crate) fn wf_bar_face(r: Rect, cx: &gpui::App) -> AnyElement {
        let face = scheme(cx).of(Role::Face);
        let paint = gpui::canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                window.paint_quad(gpui::fill(b, face));
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        crate::win95::widget::place(r)
            .child(paint)
            .into_any_element()
    }

    /// The signal body of a WF or VMW window (the dispatch of
    /// `Frame::signal_body`).
    pub(crate) fn wf_signal_body(
        &mut self,
        win: u32,
        node: NodeId,
        kind: WinKind,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Option<AnyElement> {
        self.graph.get(node)?;
        match kind {
            WinKind::Waveform => Some(self.wf_body(win, node, window, cx)),
            WinKind::Raster => Some(self.vmw_body(win, node, window, cx)),
            _ => None,
        }
    }

    /// The trace image of window `win`.
    fn wf_image(&self, win: u32) -> Option<Arc<RenderImage>> {
        self.wf
            .wins
            .get(&win)?
            .shown
            .as_ref()
            .map(|(_, i)| i.clone())
    }

    /// The LPF preset texts.
    pub(crate) fn wf_lpf_presets() -> Vec<String> {
        WF_LPF_PRESETS
            .iter()
            .map(|p| sn_ui_core::format::dec_trim(*p, 2))
            .collect()
    }

    /// Remember where control `name` of window `win` was placed.
    pub(crate) fn wf_hot(&mut self, win: u32, name: &'static str, r: Rect) {
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.hot.insert(name, r);
        }
    }

    /// The readout of the pointer over a WF or VMW plot, if any.
    pub(crate) fn wf_readout(&self) -> Option<String> {
        self.wf.wins.values().find_map(|w| w.cursor.clone())
    }
}

/// `0xRRGGBB` of a colour (its alpha dropped).
pub(crate) fn rgb_u32(c: Rgba8) -> u32 {
    (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)
}

/// A colour for GPUI, with its alpha (palette colours are premultiplied).
pub(crate) fn ink(c: Rgba8) -> gpui::Hsla {
    if c.a == 255 {
        return hsla(rgb_u32(c));
    }
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

#[cfg(debug_assertions)]
impl Frame {
    /// Debug builds: the WF scenes of `SA_GUI_SCENE` (`wf`, `wf-popup`,
    /// `vmw`): the generated MFSK-8 recording in a WF window with markers,
    /// Info, Sum and a Count freqs box, or in VMW mode.
    pub(crate) fn wf_debug_scene(&mut self, scene: &str) {
        let Some(signal) = super::sono::test_signal() else {
            return;
        };
        let op = sn_core::Operation::new("load", serde_json::json!({"path": "/demo/mfsk8.wav"}));
        let node = self.on_loaded("mfsk8.wav".into(), op, signal, &[]);
        let sono = self.find_window(node, WinKind::Sonogram);
        let Some(win) = self.open_window(node, WinKind::Waveform, sono) else {
            return;
        };
        if let Some(ww) = self.wf_win(win, node) {
            ww.info = true;
            ww.placed = true;
            ww.markers = Markers {
                v: Some((8000.0, 16_000.0)),
                h: Some((0.7, -0.7)),
                ..Markers::default()
            };
            ww.labels.push(GridLabel {
                lines: wf_tools::grid_lines(8, 125.0, false),
                offset: None,
            });
            ww.vmw.info = true;
            ww.vmw.markers.v = Some((0.2, 0.45));
        }
        match scene {
            "wf-popup" => self.open_wf_popup(win, (420, 260)),
            "vmw" => self.wf_set_window_kind(win, WinKind::Raster),
            _ => {}
        }
    }
}
