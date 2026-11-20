//! The WF window's bottom toolbar (`docs/SA_UI_SPEC.md` 7.2, SA 6.2.5.4 and
//! 6.2.6.0): fifteen flat buttons with SA's green and black glyphs on light
//! green faces, the progress field after the zoom group, and SA's wider gaps
//! between the groups; and what the buttons do.

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, canvas};
use sn_core::NodeId;
use sn_dsp::ops::Edit;
use sn_ui_core::edits::SaveFormat;
use sn_ui_core::sono_tools::ViewAction;
use sn_ui_core::t;
use sn_ui_core::view::Extent;
use sn_ui_core::wf_tools::{self, WF_TOOLBAR, WfButton, WfSlot, WfTools};

use super::actions::WfDest;
use super::{Frame, icons, ink};
use crate::app::model::WinKind;
use crate::win95::bars::{TOOL, Tool, tool_button};
use crate::win95::geom::Rect;
use crate::win95::kit::action;
use crate::win95::paint;
use crate::win95::progress::progress_field;
use crate::win95::widget::{place, size_of};

/// SA's wider gap between groups.
pub(crate) const GAP: i32 = 6;
/// Width of the progress field.
pub(crate) const PROGRESS_W: i32 = 110;

/// The left edge of every slot of a toolbar starting at `x0`, and its width.
pub(crate) fn slot_places(x0: i32) -> Vec<(WfSlot, i32, i32)> {
    let mut x = x0;
    WF_TOOLBAR
        .iter()
        .map(|s| {
            let w = match s {
                WfSlot::Button(_) => TOOL,
                WfSlot::Gap => GAP,
                WfSlot::Progress => PROGRESS_W,
            };
            let at = (*s, x, w);
            x += w + i32::from(matches!(s, WfSlot::Button(_)));
            at
        })
        .collect()
}

impl Frame {
    /// What the enable rules of window `win` look at.
    pub(crate) fn wf_tools(&self, win: u32, node: NodeId) -> WfTools {
        let ww = self.wf.wins.get(&win);
        WfTools {
            history: ww.is_some_and(|w| !w.history.is_empty()),
            revert: ww.is_some_and(|w| w.original.is_some() || w.ccf.is_some()),
            synth: self.wf_resynth_ready(node),
            derived: self.graph.get(node).is_some_and(|n| !n.inputs.is_empty()),
        }
    }

    /// Tones and a Br are known: reverse synthesis can run.
    pub(crate) fn wf_resynth_ready(&self, node: NodeId) -> bool {
        self.wf_tone_count(node) >= 2 && self.wf_br().is_some()
    }

    /// The bottom toolbar of window `win` in `r` (client coordinates).
    pub(crate) fn wf_bottom_toolbar(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let tools = self.wf_tools(win, node);
        let face = ink(self.wf_ink(cx).button_face);
        let y = r.y + (r.h - TOOL) / 2;
        let mut out = Vec::new();
        for (slot, x, w) in slot_places(r.x + 2) {
            match slot {
                WfSlot::Gap => {}
                WfSlot::Progress => {
                    let p = self.node_progress(node);
                    out.push(progress_field(cx, Rect::new(x, y + 2, w, TOOL - 4), p));
                }
                WfSlot::Button(b) => {
                    let why = wf_tools::missing(b, tools);
                    let enabled = why.is_none();
                    let glyph = icons::button_glyph(b, enabled);
                    let tip = b.tooltip(why.as_deref());
                    let id = format!("wf{win}.b.{b:?}");
                    let bg = canvas(
                        |_, _, _| (),
                        move |b, (), window, _| {
                            let (w, h) = size_of(b);
                            paint::fill(window, b.origin, Rect::new(1, 1, w - 2, h - 2), face);
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full();
                    out.push(
                        place(Rect::new(x, y, TOOL, TOOL))
                            .child(bg)
                            .into_any_element(),
                    );
                    out.push(tool_button(
                        &self.kit,
                        cx,
                        Tool {
                            id: &id,
                            at: (x, y),
                            glyph: &glyph,
                            enabled,
                            checked: false,
                            tip: &tip,
                        },
                        action(move |v: &mut Self, _, cx| v.press_wf_button(win, b, cx)),
                    ));
                }
            }
        }
        out
    }

    /// A bottom toolbar button of window `win` was pressed.
    pub(crate) fn press_wf_button(&mut self, win: u32, b: WfButton, cx: &mut Context<'_, Self>) {
        let Some(node) = self.node_of(win) else {
            return;
        };
        if wf_tools::missing(b, self.wf_tools(win, node)).is_some() {
            return;
        }
        let Some(ww) = self.wf.wins.get(&win) else {
            return;
        };
        let v = ww
            .markers
            .time_range()
            .map(|(a, b)| (a.max(0.0) as usize, b.max(0.0) as usize));
        let check = t!("sw-check-settings");
        match b {
            WfButton::ZoomIn => self.wf_zoom(win, 0.5),
            WfButton::ZoomOut => self.wf_zoom(win, 2.0),
            WfButton::ScaleUp => self.wf_scale(node, 0.5),
            WfButton::ScaleDown => self.wf_scale(node, 2.0),
            WfButton::ZoomPlus => match v {
                Some(_) => self.wf_view_action(win, ViewAction::ZoomToSelection),
                None => self.alert(&check),
            },
            WfButton::ZoomMinus => self.wf_view_action(win, ViewAction::Back),
            WfButton::Lpf => self.wf_filter(win, node),
            WfButton::Revert => self.wf_revert(win, node),
            WfButton::Synth => self.wf_resynth(win, node),
            WfButton::Save => self.choose_save(node, SaveFormat::WavPcm16, cx),
            WfButton::Copy => match v {
                Some((start, end)) => {
                    self.start_wf_job(node, Edit::Copy { start, end }, WfDest::CopyWindow, win);
                }
                None => self.alert(&check),
            },
            WfButton::Demod => match v {
                Some((a, b)) => self.wf_send_to_demod(win, node, (a, b)),
                None => self.alert(&check),
            },
            WfButton::Fit => self.wf_fit(node),
            WfButton::Acf => match v {
                Some((start, end)) => {
                    self.start_wf_job(node, Edit::Acf { start, end }, WfDest::AcfWindow, win);
                }
                None => self.alert(&check),
            },
            WfButton::Ccf => match v {
                Some((start, end)) => {
                    let dest = WfDest::Overlay {
                        win,
                        fragment: (start, end),
                    };
                    self.start_wf_job(node, Edit::Ccf { start, end }, dest, win);
                }
                None => self.alert(&check),
            },
        }
    }

    /// Zoom the time axis of window `win` by `f` around the middle.
    fn wf_zoom(&mut self, win: u32, f: f64) {
        let len = self
            .node_of(win)
            .and_then(|n| self.graph.get(n))
            .map_or(0.0, |n| n.signal.len() as f64);
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            let width = f64::from(ww.plot.w);
            if let Some(view) = ww.view.as_mut() {
                view.zoom_time(f, width / 2.0, wf_extent(len), width);
            }
        }
    }

    /// Multiply the vertical half range of `node` by `f`.
    pub(crate) fn wf_scale(&mut self, node: NodeId, f: f32) {
        if let Some(u) = self.ui.get_mut(&node) {
            let (lo, hi) = wf_tools::SCALE_LIMITS;
            u.wave_scale = (u.wave_scale * f).clamp(lo, hi);
        }
    }

    /// Ask window `win`'s view for `a` at the next layout.
    pub(crate) fn wf_view_action(&mut self, win: u32, a: ViewAction) {
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.action = Some(a);
        }
    }

    /// The LPF button: the low-pass at the LPF value, or the RRC filter while
    /// RRC is on; the result replaces the signal of this window.
    pub(crate) fn wf_filter(&mut self, win: u32, node: NodeId) {
        let Some(fs) = self.graph.get(node).map(|n| n.signal.sample_rate()) else {
            return;
        };
        let Some(ww) = self.wf.wins.get(&win) else {
            return;
        };
        let alpha = wf_tools::rrc_alpha(ww.alpha);
        match wf_tools::filter_edit(&ww.lpf.text, ww.rrc, alpha, fs) {
            Some(e) => self.start_wf_job(node, e, WfDest::Replace { win }, win),
            None => self.alert(&t!("sw-check-settings")),
        }
    }

    /// Reverse FSK/MFSK synthesis from the last tones and Br.
    pub(crate) fn wf_resynth(&mut self, win: u32, node: NodeId) {
        if let Some(baud) = self.wf_br() {
            let tones = self.wf_last_tones_of(node);
            self.start_edit(win, Edit::Resynthesize { tones, baud });
        }
    }

    /// The diamond: the demodulator works on the signal's V markers, so they
    /// take the WF fragment; then its window opens.
    fn wf_send_to_demod(&mut self, win: u32, node: NodeId, (a, b): (usize, usize)) {
        if let Some(u) = self.ui.get_mut(&node) {
            u.markers.v = Some((a as f64, b as f64));
            self.mark_dirty();
        }
        self.open_window(node, WinKind::Demod, Some(win));
    }

    /// Fit the vertical scale of `node` to its signal.
    pub(crate) fn wf_fit(&mut self, node: NodeId) {
        let ov = self.overviews.get(&node).cloned();
        if let (Some(ov), Some(n)) = (ov, self.graph.get(node))
            && let Some((offset, scale)) = sn_ui_core::app_glue::fit_range(&n.signal, &ov)
            && let Some(u) = self.ui.get_mut(&node)
        {
            u.wave_offset = offset;
            u.wave_scale = scale;
        }
    }

    /// Whether `node` is a frequency (Hz) trace, after the phase detector.
    pub(crate) fn wf_is_hz(&self, node: NodeId) -> bool {
        self.graph
            .get(node)
            .is_some_and(|n| n.signal.unit() == sn_core::SampleUnit::Hertz)
    }
}

/// The time extent of a signal of `len` samples (WF has no frequency axis).
pub(crate) fn wf_extent(len: f64) -> Extent {
    Extent {
        len,
        f_min: 0.0,
        f_max: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_toolbar_fits_the_default_window() {
        let places = slot_places(2);
        let Some((_, x, w)) = places.last().copied() else {
            panic!("no slots");
        };
        assert!(x + w <= WinKind::Waveform.client_size().0, "{}", x + w);
    }
}
