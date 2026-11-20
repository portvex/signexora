//! SA's sonogram work window (`docs/SA_UI_SPEC.md` section 4) on the Windows 95
//! kit: the top toolbar of display settings (4.1), the plot with its markers,
//! Info overlay and floating result boxes (4.2, 4.4), the navigation scroll
//! bar, the bottom toolbar of tools (4.3.2), the right-click popup menu (4.5)
//! and the Correction of BR panel (4.6), as `sa-app`'s `sono_window.rs` and
//! `sonogram.rs` draw them.
//!
//! The module is a child of `app` (declared there with a path), so it extends
//! [`Frame`] directly. Display changes redraw the same window; every tool
//! opens its result in a new window through [`Frame::start_edit`].
//!
//! - `top.rs`, `bottom.rs`: the two toolbars.
//! - `plot.rs`: the image, markers, overlay, labels and their mouse gestures.
//! - `popup.rs`, `brc.rs`: the popup menu and the Correction of BR panel.
//! - `actions.rs`: what the buttons and menu items run (measurements, ADP,
//!   Save as, playback) and the results of those jobs.
//! - `icons.rs`: the toolbar and menu glyphs.

mod actions;
mod bottom;
mod brc;
pub(crate) mod icons;
mod keys;
pub(crate) mod plot;
pub(crate) mod popup;
#[cfg(test)]
mod tests;
mod top;

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{AnyElement, Context, IntoElement, ParentElement, RenderImage, Styled, Window, div};
use sn_core::NodeId;
use sn_ui_core::markers::Markers;
use sn_ui_core::playback::Playback;
use sn_ui_core::sono_cache::{SonoCache, SonoImage};
use sn_ui_core::sono_tools::{self, FloatLabel, ViewAction};
use sn_ui_core::view::{Extent, View};

pub(crate) use actions::SonoOutput;
pub(crate) use brc::Brc;

use super::Frame;
use crate::win95::combo::ComboState;
use crate::win95::edit::EditState;
use crate::win95::geom::Rect;
use crate::win95::mdi;
use crate::win95::scrollbar::{self, ScrollState};

/// Height of the top toolbar.
pub(crate) const TOP_H: i32 = 30;
/// Height of the bottom toolbar.
pub(crate) const BOT_H: i32 = 26;
/// Height of the navigation scroll bar under the plot.
pub(crate) const NAV_H: i32 = scrollbar::THICKNESS;
/// Client size of a new sonogram window: both toolbars fit in one row (as
/// `sa-app`'s 960 x 400 window does).
pub(crate) const CLIENT: (i32, i32) = (860, 330);

/// Where the parts of a window with a client area `cw` x `ch` go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub(crate) top: Rect,
    pub(crate) plot: Rect,
    pub(crate) nav: Rect,
    pub(crate) bottom: Rect,
}

impl Layout {
    pub(crate) fn new(cw: i32, ch: i32) -> Self {
        let plot_h = (ch - TOP_H - BOT_H - NAV_H).max(0);
        Self {
            top: Rect::new(0, 0, cw, TOP_H),
            plot: Rect::new(0, TOP_H, cw, plot_h),
            nav: Rect::new(0, TOP_H + plot_h, cw, NAV_H),
            bottom: Rect::new(0, TOP_H + plot_h + NAV_H, cw, BOT_H),
        }
    }
}

/// The state of one sonogram window that SA keeps in the window's controls.
pub(crate) struct SonoWin {
    pub(crate) node: NodeId,
    /// The zoom and pan; `None` until the plot is first laid out.
    pub(crate) view: Option<View>,
    /// Zoom steps for Zoom back.
    pub(crate) history: Vec<View>,
    /// The markers before the last sweep or collapse.
    pub(crate) undo: Option<Markers>,
    /// A view request applied at the next layout.
    pub(crate) action: Option<ViewAction>,
    pub(crate) cache: SonoCache,
    /// The image shown, with the cache's image count it was made from.
    shown: Option<(u64, Arc<RenderImage>)>,
    /// The spectrum extent at the last layout.
    pub(crate) extent: Option<Extent>,
    /// The plot at the last layout (client coordinates).
    pub(crate) plot: Rect,
    pub(crate) fft: ComboState,
    pub(crate) shear: ComboState,
    pub(crate) image: ComboState,
    pub(crate) winf: ComboState,
    pub(crate) fl: EditState,
    pub(crate) fu: EditState,
    /// The R/S value field.
    pub(crate) value: EditState,
    /// Floating result boxes.
    pub(crate) labels: Vec<FloatLabel>,
    /// Their sizes as last drawn, for the mouse.
    pub(crate) label_sizes: Vec<(i32, i32)>,
    /// The open Correction of BR panel.
    pub(crate) brc: Option<Brc>,
    pub(crate) nav: ScrollState,
    /// Columns per navigation scroll bar unit.
    pub(crate) nav_unit: i64,
    /// The pointer readout over the plot.
    pub(crate) cursor: Option<String>,
}

impl std::fmt::Debug for SonoWin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SonoWin")
            .field("node", &self.node)
            .field("view", &self.view)
            .field("labels", &self.labels)
            .finish_non_exhaustive()
    }
}

impl SonoWin {
    /// The window's state is given up (it closed, or shows another signal):
    /// stop its column job and hand its image over to be freed.
    fn retire(mut self, stale: &mut Vec<Arc<RenderImage>>) {
        self.cache.release();
        if let Some((_, img)) = self.shown.take() {
            stale.push(img);
        }
    }

    fn new(node: NodeId, fs: f64) -> Self {
        let read_only = || EditState {
            read_only: true,
            ..EditState::default()
        };
        Self {
            node,
            view: None,
            history: Vec::new(),
            undo: None,
            action: None,
            cache: SonoCache::default(),
            shown: None,
            extent: None,
            plot: Rect::default(),
            fft: ComboState::default(),
            shear: ComboState::default(),
            image: ComboState::default(),
            winf: ComboState::default(),
            fl: read_only(),
            fu: read_only(),
            value: EditState::new(&sono_tools::value_text(fs)),
            labels: Vec::new(),
            label_sizes: Vec::new(),
            brc: None,
            nav: ScrollState::default(),
            nav_unit: 1,
            cursor: None,
        }
    }
}

/// The sonogram windows of the frame, and the audio they play.
#[derive(Default)]
pub(crate) struct SonoHub {
    pub(crate) wins: HashMap<u32, SonoWin>,
    /// The window whose popup menu (or preset list) is open.
    pub(crate) popup_win: Option<u32>,
    pub(crate) playback: Playback,
    /// Images replaced since the last frame, to free from the sprite atlas.
    stale: Vec<Arc<RenderImage>>,
}

impl std::fmt::Debug for SonoHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SonoHub")
            .field("wins", &self.wins.len())
            .field("popup_win", &self.popup_win)
            .finish_non_exhaustive()
    }
}

/// A GPUI image (BGRA) of a sonogram image.
fn render_image(img: &SonoImage) -> Option<Arc<RenderImage>> {
    let mut bytes = Vec::with_capacity(img.pixels.len() * 4);
    for p in &img.pixels {
        bytes.extend_from_slice(&[p.b, p.g, p.r, p.a]);
    }
    let w = u32::try_from(img.width).ok()?;
    let h = u32::try_from(img.height).ok()?;
    let buf = image::RgbaImage::from_raw(w, h, bytes)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buf)])))
}

impl Frame {
    /// The client size of window `win`.
    pub(crate) fn client_of(&self, win: u32) -> (i32, i32) {
        self.mdi.shown_rect(win).map_or((0, 0), |r| {
            let c = mdi::client_rect(r.w, r.h);
            (c.w, c.h)
        })
    }

    /// The sonogram window `win`'s state, made on first use.
    pub(crate) fn sono_win(&mut self, win: u32, node: NodeId) -> Option<&mut SonoWin> {
        let fs = self.graph.get(node)?.signal.sample_rate();
        let sw = self
            .sono
            .wins
            .entry(win)
            .or_insert_with(|| SonoWin::new(node, fs));
        if sw.node != node {
            // The window shows another signal: its column job stops and its
            // image leaves the sprite atlas.
            let old = std::mem::replace(sw, SonoWin::new(node, fs));
            old.retire(&mut self.sono.stale);
        }
        self.sono.wins.get_mut(&win)
    }

    /// Forget the state of windows that closed, and free replaced images.
    pub(crate) fn sono_prune(&mut self, window: &mut Window) {
        let open: Vec<u32> = self
            .signal_windows()
            .into_iter()
            .map(|(w, _, _)| w)
            .collect();
        let gone: Vec<u32> = self
            .sono
            .wins
            .keys()
            .copied()
            .filter(|w| !open.contains(w))
            .collect();
        for w in gone {
            if let Some(sw) = self.sono.wins.remove(&w) {
                sw.retire(&mut self.sono.stale);
            }
        }
        for img in std::mem::take(&mut self.sono.stale) {
            window.drop_image(img).ok();
        }
    }

    /// Work that needs the poll loop: columns being computed, audio playing.
    pub(crate) fn sono_busy(&self) -> bool {
        self.sono.playback.is_playing() || self.sono.wins.values().any(|w| w.cache.busy())
    }

    /// The body of sonogram window `win` showing `node`.
    pub(crate) fn sono_body(
        &mut self,
        win: u32,
        node: NodeId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let (cw, ch) = self.client_of(win);
        let layout = Layout::new(cw, ch);
        self.sono_layout(win, node, layout, cx);
        let mut root = div().relative().size_full();
        for el in self.top_toolbar(win, node, layout.top, window, cx) {
            root = root.child(el);
        }
        root = root.child(self.plot_element(win, node, layout.plot, window, cx));
        root = root.child(self.nav_bar(win, layout.nav, cx));
        for el in self.bottom_toolbar(win, node, layout.bottom, cx) {
            root = root.child(el);
        }
        for el in self.brc_panel(win, layout.plot, window, cx) {
            root = root.child(el);
        }
        root.into_any_element()
    }

    /// Fit the view to the plot, apply a waiting zoom request, ask for the
    /// visible columns and make the image when it changed.
    fn sono_layout(&mut self, win: u32, node: NodeId, layout: Layout, cx: &Context<'_, Self>) {
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let (settings, markers) = self
            .ui
            .get(&node)
            .map(|u| (u.settings, u.markers))
            .unwrap_or_default();
        let outside = self.plot_ink(cx).outside;
        if self.sono_win(win, node).is_none() {
            return;
        }
        let Some(sw) = self.sono.wins.get_mut(&win) else {
            return;
        };
        sw.plot = layout.plot;
        let extent = sw.cache.extent(&settings, &signal);
        sw.extent = extent;
        let (w, h) = (layout.plot.w, layout.plot.h);
        let Some(extent) = extent.filter(|_| w >= 8 && h >= 8) else {
            return;
        };
        let width = f64::from(w);
        let view = sw.view.get_or_insert_with(|| View::fit(extent, width));
        view.resized(extent, width);
        if let Some(a) = sw.action.take() {
            sono_tools::apply_action(a, view, &mut sw.history, &markers, extent, width, true);
        }
        let view = *view;
        sw.cache.update(
            &settings,
            &signal,
            node,
            &view,
            [w as usize, h as usize],
            outside,
        );
        let count = sw.cache.image_count();
        if sw.shown.as_ref().is_none_or(|(c, _)| *c != count)
            && let Some(img) = sw.cache.image().and_then(|i| render_image(i))
            && let Some((_, old)) = sw.shown.replace((count, img))
        {
            self.sono.stale.push(old);
        }
        self.sync_nav(win);
    }

    /// The image of window `win`.
    fn sono_image(&self, win: u32) -> Option<Arc<RenderImage>> {
        self.sono
            .wins
            .get(&win)?
            .shown
            .as_ref()
            .map(|(_, i)| i.clone())
    }
}

/// A generated test signal for window captures and tests: MFSK-8 at 75 Bd,
/// 125 Hz spacing around 1500 Hz, 8000 Hz, about 6 s, with noise at 15 dB SNR.
#[cfg(any(test, debug_assertions))]
pub(crate) fn test_signal() -> Option<sn_core::Signal> {
    let p = sn_synth::MfskParams {
        order: 8,
        baud: 75.0,
        spacing: 125.0,
        center: 1500.0,
        sample_rate: 8000.0,
        amplitude: 0.5,
    };
    let symbols = sn_synth::random_symbols(450, 8, 7);
    let clean = sn_synth::mfsk(&p, &symbols).ok()?;
    sn_synth::add_awgn_snr(&clean, 15.0, 11).ok()
}

#[cfg(debug_assertions)]
impl Frame {
    /// Debug builds: the sonogram scenes of `SA_GUI_SCENE` (`sonogram`,
    /// `sono-popup`, `sono-brc`, `sono-collapsed`): a generated MFSK-8
    /// recording in its work window with markers, Info and a result box.
    pub(crate) fn sono_scene(&mut self, scene: &str) {
        let Some(signal) = test_signal() else {
            return;
        };
        let op = sn_core::Operation::new("load", serde_json::json!({"path": "/demo/mfsk8.wav"}));
        let node = self.on_loaded("mfsk8.wav".into(), op, signal, &[]);
        let Some(win) = self.find_window(node, crate::app::model::WinKind::Sonogram) else {
            return;
        };
        if let Some(u) = self.ui.get_mut(&node) {
            u.info = true;
            u.markers = Markers {
                v: Some((8000.0, 16_000.0)),
                h: Some((1000.0, 2000.0)),
                ..Markers::default()
            };
            if scene == "sono-collapsed" {
                sono_tools::collapse_markers(&mut u.markers, 12_000.0, 1500.0);
            }
        }
        if let Some(sw) = self.sono_win(win, node) {
            sw.labels.push(FloatLabel::new(
                sono_tools::freq_label(1749.97),
                (16_000.0, 2000.0),
            ));
        }
        match scene {
            "sono-popup" => self.open_sono_popup(win, (420, 200)),
            "sono-brc" => self.open_brc(win),
            _ => {}
        }
    }
}
