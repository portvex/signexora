//! The demodulator's top bar in SA 6.2.3.8's order (`docs/SA_UI_SPEC.md` 8):
//! `RRC` check box and cutoff combo above the alpha field and slider; the Br
//! field with two coarse and five digit spin pairs; the `A` check box and
//! button with `<= Exactly` above `<= Roughly`; Levels above `Raw`.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, StatefulInteractiveElement, Styled, canvas,
};
use sn_core::NodeId;
use sn_ui_core::demod::{BR_COARSE, BR_STEPS, DataAs, EXACT_SPAN};
use sn_ui_core::{format, t};

use super::ctl::{SPIN_W, Step};
use super::{Field, Frame};
use crate::win95::bars::{Tool, tool_button, tooltip};
use crate::win95::button::{Changed, Ctl, check_box};
use crate::win95::colors::{Role, scheme};
use crate::win95::edit::{EditState, spin_edit};
use crate::win95::geom::{Edge, Rect};
use crate::win95::glyph::{self, Glyph, Mode};
use crate::win95::kit::{Lens, action, lens};
use crate::win95::label::{Align, label};
use crate::win95::marks::{self, Dir};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, scroll_bar};
use crate::win95::widget::{Slot, hovered, place, size_of};

/// Top of the first and second row.
const ROW1: i32 = 3;
const ROW2: i32 = 27;
/// Height of a control.
const CTL_H: i32 = 21;
/// Left edges of the groups.
const RRC_X: i32 = 4;
const BR_X: i32 = 150;
const SYNC_X: i32 = 386;
const LEVELS_X: i32 = 650;
/// Width of the Br field.
const BR_W: i32 = 96;
/// Gap between the coarse and the digit spin pairs.
const PAIR_GAP: i32 = 6;
/// Width of the cutoff field (with its drop button).
const CUTOFF_W: i32 = 76;
/// Width of the cutoff list's drop button.
const DROP_W: i32 = 16;
/// Widths of the alpha, `Exactly` and `Roughly` sliders.
const ALPHA_W: i32 = 96;
const EXACT_W: i32 = 130;
const ROUGH_W: i32 = 176;

/// A bold letter glyph (`A`, `D`).
pub(crate) fn letter(s: &str) -> Glyph {
    Glyph::new(16, 16).text(4, 2, s, true, glyph::BLACK)
}

/// Positions of the Br spin pairs, from the left of the Br group.
pub(crate) fn br_pair_x() -> Vec<i32> {
    let mut x = BR_X + BR_W + 4;
    BR_STEPS
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i == BR_COARSE {
                x += PAIR_GAP;
            }
            let at = x;
            x += SPIN_W + 1;
            at
        })
        .collect()
}

/// The controls of the top bar from left to right (for tests): their ids.
#[cfg(test)]
pub(crate) fn order(win: u32) -> Vec<String> {
    let mut v = vec![
        format!("dm{win}.rrc"),
        Field::Cutoff.id(win),
        Field::Alpha.id(win),
        format!("dm{win}.alpha_bar"),
        Field::Br.id(win),
    ];
    v.extend((0..BR_STEPS.len()).map(|i| format!("dm{win}.br{i}")));
    v.extend([
        format!("dm{win}.auto"),
        format!("dm{win}.a"),
        format!("dm{win}.exact"),
        format!("dm{win}.rough"),
        Field::Levels.id(win),
        format!("dm{win}.raw"),
    ]);
    v
}

/// The left edge of each control of [`order`] (for tests).
#[cfg(test)]
pub(crate) fn order_x() -> Vec<i32> {
    let mut v = vec![RRC_X, RRC_X + 48, RRC_X, RRC_X + 44, BR_X];
    v.extend(br_pair_x());
    v.extend([
        SYNC_X,
        SYNC_X + 18,
        SYNC_X + 44,
        SYNC_X,
        LEVELS_X,
        LEVELS_X + 26,
    ]);
    v
}

/// An invisible area at `r` that shows the hint `tip` for control `id`
/// (placed under the control).
pub(crate) fn tip_area(id: &str, r: Rect, tip: &str) -> AnyElement {
    place(r)
        .id(gpui::ElementId::Name(format!("{id}.hint").into()))
        .tooltip(tooltip(tip))
        .into_any_element()
}

/// A slider's scroll state: `value` of `0..=range` with thumb `page`.
fn slider(value: i32, range: i32, page: i32) -> ScrollState {
    ScrollState::new(0, range + page - 1, page, value)
}

impl Frame {
    /// The sliders from the state (alpha, `Exactly`, `Roughly`).
    pub(crate) fn sync_demod_bars(&mut self, node: NodeId) {
        let d = self.demod_win(node);
        let st = &d.st;
        d.alpha_bar = if st.rrc {
            slider(((st.alpha - 0.05) * 100.0).round() as i32, 95, 10)
        } else {
            // Grey while RRC is off.
            slider(0, 0, 1)
        };
        d.exact_bar = slider(((st.exact + 1.0) * 50.0).round() as i32, 100, 10);
        d.rough_bar = slider((st.rough * 100.0).round() as i32, 100, 10);
    }

    /// The top bar of demodulator window `win` in `r`.
    pub(crate) fn demod_top(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::new();
        out.extend(self.dm_rrc_group(win, node, r, cx));
        out.extend(self.dm_br_group(win, node, r, cx));
        out.extend(self.dm_sync_group(win, node, r, cx));
        out.extend(self.dm_levels_group(win, node, r, cx));
        out
    }

    /// `RRC` and the cutoff combo; the alpha field and slider below them.
    fn dm_rrc_group(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let scope = format!("mdi{win}");
        let d = self.demod_win(node);
        let (rrc, cutoff, alpha, alpha_bar) =
            (d.st.rrc, d.cutoff.clone(), d.alpha.clone(), d.alpha_bar);
        let refused = d.st.cutoff_error.clone();
        let rrc_id = format!("dm{win}.rrc");
        let rrc_label = t!("demod-rrc");
        let on: Changed<Self, bool> = Rc::new(move |v: &mut Self, on, _, _| {
            v.demod_win(node).st.rrc = on;
        });
        let check = check_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &rrc_id,
                scope: &scope,
                rect: Rect::new(RRC_X, r.y + ROW1, 44, CTL_H),
                label: &rrc_label,
                enabled: true,
            },
            rrc,
            on,
        );
        let tip = tip_area(
            &rrc_id,
            Rect::new(RRC_X, r.y + ROW1, 44, CTL_H),
            &t!("demod-rrc.hover"),
        );
        let mut out = vec![tip, check];
        let cut_tip = refused.unwrap_or_else(|| t!("demod-cutoff.hover"));
        let cut_rect = Rect::new(RRC_X + 48, r.y + ROW1, CUTOFF_W, CTL_H);
        out.push(self.demod_field(win, node, Field::Cutoff, cut_rect, &cutoff, &cut_tip, cx));
        out.push(self.dm_cutoff_drop(win, node, cut_rect, cx));
        let alpha_rect = Rect::new(RRC_X, r.y + ROW2, 40, CTL_H);
        let mut a = alpha;
        if !rrc {
            a.look = Some(crate::win95::edit::FieldLook {
                bg: scheme(cx).rgb(Role::Face),
                fg: scheme(cx).rgb(Role::Shadow),
                border: true,
            });
        }
        out.push(self.demod_field(
            win,
            node,
            Field::Alpha,
            alpha_rect,
            &a,
            &t!("demod-alpha.hover"),
            cx,
        ));
        let bar_id = format!("dm{win}.alpha_bar");
        out.push(scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &bar_id,
                rect: Rect::new(RRC_X + 44, r.y + ROW2 + 2, ALPHA_W, THICKNESS),
                vertical: false,
            },
            alpha_bar,
            lens(move |v: &mut Self| v.demod.nodes.get_mut(&node).map(|d| &mut d.alpha_bar)),
            Some(action(move |v: &mut Self, _, _| {
                let d = v.demod_win(node);
                d.st.alpha = 0.05 + (f64::from(d.alpha_bar.pos) / 100.0).clamp(0.0, 0.95);
                d.st.alpha_text = format::dec(d.st.alpha, 2);
            })),
        ));
        out
    }

    /// An edit field of the top or bottom bar that applies on Enter and when
    /// it loses the focus.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn demod_field(
        &mut self,
        win: u32,
        node: NodeId,
        f: Field,
        rect: Rect,
        st: &EditState,
        tip: &str,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let id = f.id(win);
        let scope = format!("mdi{win}");
        // The alpha field is grey while RRC is off.
        let enabled = f != Field::Alpha || self.demod_win(node).st.rrc;
        let l: Lens<Self, EditState> =
            lens(move |v: &mut Self| v.demod.nodes.get_mut(&node).map(|d| d.edit_of(f)));
        let on_change = (f == Field::Cutoff).then(|| {
            action(move |v: &mut Self, _, _| {
                let d = v.demod_win(node);
                d.st.cutoff.clone_from(&d.cutoff.text);
            })
        });
        let on_enter = action(move |v: &mut Self, _, _| {
            let e = v.demod_win(node).apply(f, true);
            v.demod_error(e);
        });
        self.dm_enter_field(
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect,
                label: "",
                enabled,
            },
            st,
            l,
            on_change,
            Some(on_enter),
            tip,
        )
    }

    /// The cutoff field's drop button: Br, then SA's presets.
    fn dm_cutoff_drop(
        &self,
        win: u32,
        node: NodeId,
        field: Rect,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let s = scheme(cx);
        let id = format!("dm{win}.cutoff_list");
        let held = self.kit.is_pressed(&id);
        let slot = Slot::default();
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
        let r = Rect::new(field.right() - 2 - DROP_W, field.y + 2, DROP_W, field.h - 4);
        place(r)
            .id(gpui::ElementId::Name(id.clone().into()))
            .tooltip(tooltip(&t!("demod-cutoff.hover")))
            .child(paint)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, _: &MouseDownEvent, _, cx| {
                    v.kit.press(&id, None);
                    if let Some(b) = slot.bounds() {
                        let at = (
                            f32::from(b.origin.x).round() as i32 - (CUTOFF_W - DROP_W - 2),
                            f32::from(b.origin.y + b.size.height).round() as i32 + 2,
                        );
                        v.open_cutoff_list(win, node, at);
                    }
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    /// The Br field (Enter or leaving it applies) and its seven spin pairs.
    fn dm_br_group(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let d = self.demod_win(node);
        let mut br = d.br.clone();
        let refused = d.st.br_refused.clone();
        if refused.is_some() {
            // A refused value tints the field until a usable Br is set.
            br.look = Some(crate::win95::edit::FieldLook {
                bg: 0xFF_C0_C0,
                fg: 0x00_00_00,
                border: true,
            });
        }
        let tip = refused.unwrap_or_else(|| t!("demod-br.hover"));
        let y = r.y + (r.h - CTL_H) / 2;
        let mut out = vec![self.demod_field(
            win,
            node,
            Field::Br,
            Rect::new(BR_X, y, BR_W, CTL_H),
            &br,
            &tip,
            cx,
        )];
        for ((i, step), x) in BR_STEPS.iter().copied().enumerate().zip(br_pair_x()) {
            let hint = t!("demod-br-step.hover", step = format::dec_trim(step, 3));
            let f: Step = Rc::new(move |v: &mut Self, dir, _, _| {
                let st = &mut v.demod_win(node).st;
                let b = st.baud + f64::from(dir) * step;
                if b > 0.0 {
                    st.set_baud(b);
                }
            });
            let id = format!("dm{win}.br{i}");
            out.push(self.dm_spin_pair(cx, &id, (x, r.y + (r.h - 20) / 2), true, &hint, &f));
        }
        out
    }

    /// The `A` check box and button, `<= Exactly` above `<= Roughly`.
    fn dm_sync_group(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let scope = format!("mdi{win}");
        let d = self.demod_win(node);
        let (auto, exact, rough) = (d.st.auto, d.exact_bar, d.rough_bar);
        let auto_id = format!("dm{win}.auto");
        let on: Changed<Self, bool> = Rc::new(move |v: &mut Self, on, _, _| {
            v.demod_win(node).st.auto = on;
        });
        let check = check_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &auto_id,
                scope: &scope,
                rect: Rect::new(SYNC_X, r.y + ROW1, 16, CTL_H),
                label: "",
                enabled: true,
            },
            auto,
            on,
        );
        let check_tip = tip_area(
            &auto_id,
            Rect::new(SYNC_X, r.y + ROW1, 16, CTL_H),
            &t!("demod-auto.hover"),
        );
        let a_id = format!("dm{win}.a");
        let glyph = letter("A");
        let tip = t!("demod-auto-button.hover");
        let a = tool_button(
            &self.kit,
            cx,
            Tool {
                id: &a_id,
                at: (SYNC_X + 18, r.y + ROW1 - 1),
                glyph: &glyph,
                enabled: true,
                checked: false,
                tip: &tip,
            },
            action(move |v: &mut Self, _, _| v.demod_win(node).st.press_auto()),
        );
        let mut out = vec![check_tip, check, a];
        let exact_tip = t!(
            "demod-exactly.hover",
            span = format::dec(EXACT_SPAN * 100.0, 0)
        );
        out.extend(self.dm_sync_slider(
            win,
            node,
            true,
            Rect::new(SYNC_X + 44, r.y + ROW1 + 2, EXACT_W, THICKNESS),
            exact,
            &exact_tip,
            cx,
        ));
        out.extend(self.dm_sync_slider(
            win,
            node,
            false,
            Rect::new(SYNC_X, r.y + ROW2 + 2, ROUGH_W, THICKNESS),
            rough,
            &t!("demod-roughly.hover"),
            cx,
        ));
        out
    }

    /// The `<= Exactly` (`exact`) or `<= Roughly` slider and its label; moving
    /// either turns `A` off.
    #[allow(clippy::too_many_arguments)]
    fn dm_sync_slider(
        &self,
        win: u32,
        node: NodeId,
        exact: bool,
        rect: Rect,
        st: ScrollState,
        tip: &str,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let id = format!("dm{win}.{}", if exact { "exact" } else { "rough" });
        let bar = scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect,
                vertical: false,
            },
            st,
            lens(move |v: &mut Self| {
                v.demod.nodes.get_mut(&node).map(|d| {
                    if exact {
                        &mut d.exact_bar
                    } else {
                        &mut d.rough_bar
                    }
                })
            }),
            Some(action(move |v: &mut Self, _, _| {
                let d = v.demod_win(node);
                if exact {
                    d.st.exact = f64::from(d.exact_bar.pos) / 50.0 - 1.0;
                } else {
                    d.st.rough = f64::from(d.rough_bar.pos) / 100.0;
                }
                d.st.auto = false;
            })),
        );
        let caption = if exact {
            t!("demod-exactly")
        } else {
            t!("demod-roughly")
        };
        let lab = place(Rect::new(rect.right() + 4, rect.y - 2, 70, CTL_H - 2))
            .id(gpui::ElementId::Name(format!("{id}.label").into()))
            .tooltip(tooltip(tip))
            .child(label(
                cx,
                Rect::new(0, 0, 70, CTL_H - 2),
                &caption,
                false,
                true,
                Align::Left,
            ));
        vec![bar, lab.into_any_element()]
    }

    /// The Levels spin edit above `Raw`.
    fn dm_levels_group(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let scope = format!("mdi{win}");
        let id = Field::Levels.id(win);
        let d = self.demod_win(node);
        let (levels, raw, enabled) = (
            d.levels.clone(),
            d.st.data_as == DataAs::Raw,
            !d.st.from_tones,
        );
        let l = lens(move |v: &mut Self| v.demod.nodes.get_mut(&node).map(|d| &mut d.levels));
        let on: Changed<Self, i64> = Rc::new(move |v: &mut Self, n, _, _| {
            v.demod_win(node).st.set_levels(n.max(2) as usize);
        });
        let spin = spin_edit(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: Rect::new(LEVELS_X, r.y + ROW1, 56, CTL_H),
                label: "",
                enabled,
            },
            &levels,
            &l,
            Some(on),
        );
        let tip_el = tip_area(
            &id,
            Rect::new(LEVELS_X, r.y + ROW1, 56, CTL_H),
            &t!("demod-levels.hover"),
        );
        let raw_label = label(
            cx,
            Rect::new(LEVELS_X, r.y + ROW2, 24, CTL_H),
            &t!("demod-raw"),
            false,
            true,
            Align::Left,
        );
        let raw_id = format!("dm{win}.raw");
        let on: Changed<Self, bool> = Rc::new(move |v: &mut Self, on, _, _| {
            let d = v.demod_win(node);
            d.st.data_as = if on { DataAs::Raw } else { DataAs::Default };
            // The new form starts at its top, as a choice from the menu does.
            d.text_bar.pos = 0;
        });
        let raw_box = check_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &raw_id,
                scope: &scope,
                rect: Rect::new(LEVELS_X + 26, r.y + ROW2, 16, CTL_H),
                label: "",
                enabled: true,
            },
            raw,
            on,
        );
        let raw_tip = tip_area(
            &raw_id,
            Rect::new(LEVELS_X, r.y + ROW2, 42, CTL_H),
            &t!("demod-raw.hover"),
        );
        vec![tip_el, spin, raw_label, raw_tip, raw_box]
    }
}
