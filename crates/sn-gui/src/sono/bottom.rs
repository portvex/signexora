//! The sonogram window's bottom toolbar (`docs/SA_UI_SPEC.md` 4.3.2, SA
//! 6.2.6.7): 27 flat buttons with 16x16 glyphs in SA's order, the progress
//! field and Stop for this window's work, and the R/S value field with its
//! preset list, with SA's slightly wider gaps between the groups.

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, StatefulInteractiveElement, Styled, canvas,
};
use sn_core::{NodeId, SampleKind};
use sn_ui_core::sono_tools::{self, BOTTOM_TOOLBAR, Button, Run, Slot, ToolState};
use sn_ui_core::t;

use super::{Frame, icons};
use crate::win95::bars::{TOOL, Tool, tool_button, tooltip};
use crate::win95::button::Ctl;
use crate::win95::colors::{Role, scheme};
use crate::win95::edit::edit;
use crate::win95::geom::{Edge, Rect};
use crate::win95::glyph::{self, Mode};
use crate::win95::kit::{action, lens};
use crate::win95::marks::{self, Dir};
use crate::win95::paint;
use crate::win95::progress::progress_field;
use crate::win95::widget::{Slot as Bounds, hovered, place, size_of};

/// SA's wider gap between groups.
pub(crate) const GAP: i32 = 6;
/// Width of the progress field.
pub(crate) const PROGRESS_W: i32 = 110;
/// Width of the R/S value field with its drop button.
pub(crate) const VALUE_W: i32 = 100;
/// Width of the value field's drop button.
const DROP_W: i32 = 16;

/// The left edge of every slot of a toolbar starting at `x0`, and its width.
pub(crate) fn slot_places(x0: i32) -> Vec<(Slot, i32, i32)> {
    let mut x = x0;
    BOTTOM_TOOLBAR
        .iter()
        .map(|s| {
            let w = match s {
                Slot::Button(_) => TOOL,
                Slot::Gap => GAP,
                Slot::Progress => PROGRESS_W,
                Slot::Value => VALUE_W,
            };
            let at = (*s, x, w);
            x += w;
            at
        })
        .collect()
}

impl Frame {
    /// What the enable rules of window `win` (signal `node`) look at.
    pub(crate) fn tool_state(&self, win: u32, node: NodeId) -> ToolState {
        let markers = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let history = self
            .sono
            .wins
            .get(&win)
            .is_some_and(|w| !w.history.is_empty());
        ToolState {
            history,
            running: self.node_progress(node).is_running(),
            playing: self.sono.playback.is_playing() || self.preparing_playback(),
            complex: self
                .graph
                .get(node)
                .is_some_and(|n| n.signal.kind() == SampleKind::Complex),
            ..ToolState::default()
        }
        .with_markers(&markers)
    }

    /// The bottom toolbar of window `win` in `r` (client coordinates).
    pub(crate) fn bottom_toolbar(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        if self.is_harmonics(node) {
            return self.harmonics_toolbar(win, node, r, cx);
        }
        let tools = self.tool_state(win, node);
        let y = r.y + (r.h - TOOL) / 2;
        let mut out = Vec::new();
        for (slot, x, w) in slot_places(r.x + 2) {
            match slot {
                Slot::Gap => {}
                Slot::Progress => {
                    let p = self.node_progress(node);
                    out.push(progress_field(cx, Rect::new(x, y + 2, w, TOOL - 4), p));
                }
                Slot::Value => out.extend(self.value_field(win, Rect::new(x, y, w, TOOL), cx)),
                Slot::Button(b) => {
                    let why = sono_tools::missing(b, tools);
                    let enabled = why.is_none();
                    let glyph = icons::button_glyph(b, enabled);
                    let tip = b.tooltip(why.as_deref());
                    let id = format!("sw{win}.b.{b:?}");
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
                        action(move |v: &mut Self, _, _| v.press_button(win, b)),
                    ));
                }
            }
        }
        out
    }

    /// The harmonics module's own bottom toolbar (`(q)` windows: carrier and
    /// power fields, `St^n`, Control), from `src/harmonics/`.
    fn harmonics_toolbar(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        self.harmonics_bar(win, node, r, cx)
    }

    /// The R/S value field: an edit with a drop button for the preset list.
    fn value_field(&mut self, win: u32, r: Rect, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(st) = self.sono.wins.get(&win).map(|w| w.value.clone()) else {
            return Vec::new();
        };
        let id = format!("sw{win}.value");
        let scope = format!("mdi{win}");
        let field = edit(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: r,
                label: "",
                enabled: true,
            },
            &st,
            lens(move |v: &mut Self| v.sono.wins.get_mut(&win).map(|w| &mut w.value)),
            None,
        );
        let s = scheme(cx);
        let drop_id = format!("sw{win}.presets");
        let held = self.kit.is_pressed(&drop_id);
        let slot = Bounds::default();
        let paint_slot = slot.clone();
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                paint_slot.set(b);
                let (w, h) = size_of(b);
                let down = held && hovered(window, b);
                let edge = if down {
                    Edge::Flat(Role::Shadow)
                } else {
                    Edge::Raised
                };
                paint::panel(window, b.origin, Rect::new(0, 0, w, h), edge, &s);
                let m = marks::arrow(Dir::Down, 7);
                let d = i32::from(down);
                let at = ((w - m.w) / 2 + d, (h - m.h) / 2 + d);
                glyph::draw(window, cx, b.origin, at, &m, Mode::Normal, &s);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let drop = Rect::new(r.right() - 2 - DROP_W, r.y + 2, DROP_W, r.h - 4);
        let drop_el = place(drop)
            .id(gpui::ElementId::Name(drop_id.clone().into()))
            .tooltip(tooltip(&t!("sw-value-presets")))
            .child(paint)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, _: &MouseDownEvent, _, cx| {
                    v.kit.press(&drop_id, None);
                    if let Some(b) = slot.bounds() {
                        let at = (
                            f32::from(b.origin.x).round() as i32 - (VALUE_W - DROP_W - 2),
                            f32::from(b.origin.y + b.size.height).round() as i32 + 2,
                        );
                        v.open_presets(win, at);
                    }
                    cx.notify();
                }),
            );
        let tip_el = place(Rect::new(r.x, r.y, r.w - DROP_W - 2, r.h))
            .id(gpui::ElementId::Name(format!("sw{win}.value.tip").into()))
            .tooltip(tooltip(&t!("sw-value.hover")));
        vec![tip_el.into_any_element(), field, drop_el.into_any_element()]
    }

    /// A bottom toolbar button of window `win` was pressed.
    pub(crate) fn press_button(&mut self, win: u32, b: Button) {
        let Some(node) = self.node_of(win) else {
            return;
        };
        let markers = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let value = self
            .sono
            .wins
            .get(&win)
            .map(|w| w.value.text.clone())
            .unwrap_or_default();
        let Some(run) = sono_tools::run_button(b, &markers, &value) else {
            return;
        };
        self.run_tool(win, node, run);
    }

    /// Carry out what a button asked for.
    pub(crate) fn run_tool(&mut self, win: u32, node: NodeId, run: Run) {
        match run {
            Run::View(a) => {
                if let Some(sw) = self.sono.wins.get_mut(&win) {
                    sw.action = Some(a);
                }
            }
            Run::Edit(e) => self.start_edit(win, e),
            // SA's x^n opens the harmonics module (power 2) as a new `(q)` window.
            Run::Power => self.start_edit(win, sn_dsp::ops::Edit::Power { n: 2 }),
            Run::PhasePlane => {
                self.open_window(node, crate::app::model::WinKind::Phase, Some(win));
            }
            Run::Ofdm => {
                self.open_window(node, crate::app::model::WinKind::Ofdm, Some(win));
            }
            Run::Play => {
                if !self.sono.playback.is_playing() && !self.preparing_playback() {
                    self.toggle_play(win, node);
                }
            }
            Run::StopPlay => self.stop_playback(),
            Run::Abort => self.abort_node(node),
            Run::Error(e) => self.alert(&e),
        }
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
        assert!(x + w <= super::super::CLIENT.0, "{}", x + w);
        let buttons = places
            .iter()
            .filter(|(s, _, _)| matches!(s, Slot::Button(_)))
            .count();
        assert_eq!(buttons, 27);
    }
}
