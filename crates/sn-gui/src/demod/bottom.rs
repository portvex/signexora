//! The demodulator's bottom bar (`docs/SA_UI_SPEC.md` 8): back 10 pages,
//! back, the page field (Enter), forward, forward 10; vertical scale up and
//! down; the progress field (right click: Stop) and Stop; filter (applies
//! RRC) and revert (back to the unfiltered track); `D` (the decided symbols
//! into the text panel).

use gpui::{AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseUpEvent};
use sn_core::NodeId;
use sn_ui_core::demod::ZOOM;
use sn_ui_core::t;

use super::top::{letter, tip_area};
use super::{Field, Frame};
use crate::win95::bars::{TOOL, Tool, tool_button};
use crate::win95::colors::Role;
use crate::win95::geom::Rect;
use crate::win95::glyph::{BLACK, Glyph, Ink};
use crate::win95::kit::{Action, action};
use crate::win95::progress::{Progress, abort_glyph, progress_field};
use crate::win95::widget::place;

/// SA's slightly wider gap between groups.
const GAP: i32 = 6;
/// Width of the page field.
const PAGE_W: i32 = 40;
/// Width of the progress field.
const PROGRESS_W: i32 = 80;

/// A bottom bar button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Btn {
    BackFast,
    Back,
    Forward,
    ForwardFast,
    ScaleUp,
    ScaleDown,
    Stop,
    Filter,
    Revert,
    D,
}

/// A slot of the bottom bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    Button(Btn),
    Page,
    Progress,
    Gap,
}

/// The bottom bar from left to right.
pub(crate) const BAR: [Slot; 16] = [
    Slot::Button(Btn::BackFast),
    Slot::Button(Btn::Back),
    Slot::Page,
    Slot::Button(Btn::Forward),
    Slot::Button(Btn::ForwardFast),
    Slot::Gap,
    Slot::Button(Btn::ScaleUp),
    Slot::Button(Btn::ScaleDown),
    Slot::Gap,
    Slot::Progress,
    Slot::Button(Btn::Stop),
    Slot::Gap,
    Slot::Button(Btn::Filter),
    Slot::Button(Btn::Revert),
    Slot::Gap,
    Slot::Button(Btn::D),
];

/// Each slot with its left edge and width, from `x0`.
pub(crate) fn places(x0: i32) -> Vec<(Slot, i32, i32)> {
    let mut x = x0;
    BAR.iter()
        .map(|s| {
            let w = match s {
                Slot::Button(_) => TOOL,
                Slot::Page => PAGE_W + 2,
                Slot::Progress => PROGRESS_W + 2,
                Slot::Gap => GAP,
            };
            let at = (*s, x, w);
            x += w;
            at
        })
        .collect()
}

/// A triangle pointing left or right in a 16x16 glyph, `dx` from the left.
fn triangle(g: Glyph, dx: f32, back: bool) -> Glyph {
    let pts: [(f32, f32); 3] = if back {
        [(dx + 5.0, 3.0), (dx + 5.0, 13.0), (dx, 8.0)]
    } else {
        [(dx, 3.0), (dx, 13.0), (dx + 5.0, 8.0)]
    };
    g.poly(&pts, BLACK)
}

/// The glyph of a button.
pub(crate) fn glyph_of(b: Btn) -> Glyph {
    let g = Glyph::new(16, 16);
    match b {
        Btn::BackFast => triangle(triangle(g, 3.0, true), 8.0, true),
        Btn::Back => triangle(g, 5.5, true),
        Btn::Forward => triangle(g, 5.5, false),
        Btn::ForwardFast => triangle(triangle(g, 3.0, false), 8.0, false),
        // Arrows apart (scale up) or towards the centre (scale down).
        Btn::ScaleUp => g
            .poly(&[(4.0, 6.0), (12.0, 6.0), (8.0, 1.0)], BLACK)
            .poly(&[(4.0, 10.0), (12.0, 10.0), (8.0, 15.0)], BLACK),
        Btn::ScaleDown => g
            .poly(&[(4.0, 1.0), (12.0, 1.0), (8.0, 6.0)], BLACK)
            .poly(&[(4.0, 15.0), (12.0, 15.0), (8.0, 10.0)], BLACK),
        Btn::Stop => abort_glyph(),
        // Low-pass filter: a step from high to low over a base line.
        Btn::Filter => g
            .line(1, 4, 7, 4, BLACK)
            .line(7, 4, 10, 11, BLACK)
            .line(10, 11, 14, 11, BLACK)
            .line(1, 14, 14, 14, BLACK),
        // Revert: a white return arrow on a blue square.
        Btn::Revert => {
            let paper = Ink::Role(Role::Highlight);
            g.rect(1, 1, 14, 14, Ink::Rgb(0x00_00_C0))
                .rect(11, 4, 2, 7, paper)
                .rect(6, 9, 6, 2, paper)
                .poly(&[(3.0, 10.0), (7.0, 6.0), (7.0, 14.0)], paper)
        }
        Btn::D => letter("D"),
    }
}

impl Btn {
    /// The button's hint.
    fn tip(self) -> String {
        match self {
            Self::BackFast => t!("demod-back-fast.hover"),
            Self::Back => t!("demod-back.hover"),
            Self::Forward => t!("demod-forward.hover"),
            Self::ForwardFast => t!("demod-forward-fast.hover"),
            Self::ScaleUp => t!("demod-scale-up.hover"),
            Self::ScaleDown => t!("demod-scale-down.hover"),
            Self::Stop => t!("sw-hint-abort"),
            Self::Filter => t!("demod-filter.hover"),
            Self::Revert => t!("demod-revert.hover"),
            Self::D => t!("demod-d.hover"),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::BackFast => "back10",
            Self::Back => "back",
            Self::Forward => "fwd",
            Self::ForwardFast => "fwd10",
            Self::ScaleUp => "scale_up",
            Self::ScaleDown => "scale_down",
            Self::Stop => "stop",
            Self::Filter => "filter",
            Self::Revert => "revert",
            Self::D => "d",
        }
    }
}

/// The control id of button `b` of window `win`.
pub(crate) fn button_id(win: u32, b: Btn) -> String {
    format!("dm{win}.{}", b.name())
}

impl Frame {
    /// The bottom bar of demodulator window `win` in `r`.
    pub(crate) fn demod_bottom(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let y = r.y + (r.h - TOOL) / 2;
        let d = self.demod_win(node);
        let page = d.page.clone();
        let progress = match d.st.progress() {
            None => Progress::Idle,
            Some(Some(f)) if f > 0.0 && f < 1.0 => Progress::Done(f),
            Some(_) => Progress::Busy(self.tick),
        };
        let has = self
            .demod
            .nodes
            .get(&node)
            .is_some_and(|d| d.st.result().is_some());
        let mut out = Vec::new();
        for (slot, x, w) in places(r.x + 2) {
            match slot {
                Slot::Gap => {}
                Slot::Page => {
                    let total = self.demod_win(node).st.page_count();
                    out.push(self.demod_field(
                        win,
                        node,
                        Field::Page,
                        Rect::new(x + 1, y, PAGE_W, TOOL),
                        &page,
                        &t!("demod-page.hover", count = total),
                        cx,
                    ));
                }
                Slot::Progress => {
                    let pr = Rect::new(x + 1, y + 3, w - 2, TOOL - 6);
                    out.push(tip_area(
                        &format!("dm{win}.progress"),
                        pr,
                        &t!("demod-progress.hover"),
                    ));
                    out.push(progress_field(cx, pr, progress));
                    out.push(Self::dm_progress_menu(win, node, pr, cx));
                }
                Slot::Button(b) => {
                    let enabled = match b {
                        Btn::Stop => progress.is_running(),
                        Btn::D => has,
                        _ => true,
                    };
                    let glyph = glyph_of(b);
                    let tip = b.tip();
                    let id = button_id(win, b);
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
                        Self::demod_button_action(node, b),
                    ));
                }
            }
        }
        out
    }

    /// The right-click area of the progress field (its popup: `Stop`).
    fn dm_progress_menu(win: u32, node: NodeId, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        place(r)
            .id(gpui::ElementId::Name(
                format!("dm{win}.progress.menu").into(),
            ))
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseUpEvent, _, cx| {
                    let at = (
                        f32::from(ev.position.x).round() as i32,
                        f32::from(ev.position.y).round() as i32,
                    );
                    v.open_demod_stop_menu(win, node, at);
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    /// What button `b` does.
    fn demod_button_action(node: NodeId, b: Btn) -> Action<Self> {
        action(move |v: &mut Self, _, _| {
            let d = v.demod_win(node);
            let st = &mut d.st;
            let page = st.page as i64;
            match b {
                Btn::BackFast => st.go_to_page(page - 10),
                Btn::Back => st.go_to_page(page - 1),
                Btn::Forward => st.go_to_page(page + 1),
                Btn::ForwardFast => st.go_to_page(page + 10),
                Btn::ScaleUp => st.zoom(1.0 / ZOOM),
                Btn::ScaleDown => st.zoom(ZOOM),
                Btn::Stop => st.stop(),
                Btn::Filter => st.rrc = true,
                Btn::Revert => {
                    st.rrc = false;
                    st.average = false;
                }
                Btn::D => {
                    let e = st.output().err();
                    d.text_bar.pos = 0;
                    v.demod_error(e);
                }
            }
        })
    }
}
