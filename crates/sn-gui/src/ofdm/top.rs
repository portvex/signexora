//! The `SA-OFDM` top bar (`docs/SA_UI_SPEC.md` 10): `Min Br-LS` spin edit and
//! its LS field, `Max Br-LS` and its LS field, `Total LS`, `Current LS`,
//! `Survey` and `List of K`; and the status bar inside the form.

use std::rc::Rc;

use gpui::{AnyElement, Context, Window};
use sn_core::NodeId;
use sn_ui_core::ofdm::ls_of;
use sn_ui_core::{t, titles};

use super::{Frame, OField};
use crate::app::demod::ctl::value_box;
use crate::app::demod::tip_area;
use crate::win95::button::{Changed, Ctl, check_box, push_button};
use crate::win95::colors::{Role, scheme};
use crate::win95::edit::{SpinState, spin_edit};
use crate::win95::geom::Rect;
use crate::win95::kit::{action, lens};
use crate::win95::label::{Align, label};
use crate::win95::text;

/// Height of a control.
const CTL_H: i32 = 21;
/// Gap between a caption and its control, and between controls.
const SP: i32 = 4;
/// Extra gap between groups.
const GROUP: i32 = 8;

/// One item of the top bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    Caption(&'static str),
    Spin(OField),
    MinLs,
    MaxLs,
    Total,
    Current,
    Survey,
    ListK,
    Gap,
}

/// The top bar from left to right.
pub(crate) const BAR: [Item; 17] = [
    Item::Caption("ofdm-min-br"),
    Item::Spin(OField::Min),
    Item::MinLs,
    Item::Gap,
    Item::Caption("ofdm-max-br"),
    Item::Spin(OField::Max),
    Item::MaxLs,
    Item::Gap,
    Item::Caption("ofdm-total-ls"),
    Item::Total,
    Item::Caption("ofdm-current-ls"),
    Item::Current,
    Item::Gap,
    Item::Caption("ofdm-survey"),
    Item::Survey,
    Item::Gap,
    Item::ListK,
];

/// The width of an item (captions measured with `measure`).
fn width(it: Item, measure: &dyn Fn(&str) -> i32) -> i32 {
    match it {
        Item::Caption(key) => measure(&t!(key)),
        Item::Spin(_) => 64,
        Item::MinLs | Item::MaxLs | Item::Total => 44,
        Item::Current => 60,
        Item::Survey => 16,
        Item::ListK => measure(&t!("ofdm-list-k")) + 20,
        Item::Gap => GROUP - SP,
    }
}

/// Each item with its left edge and width, from `x0`.
pub(crate) fn places(x0: i32, measure: &dyn Fn(&str) -> i32) -> Vec<(Item, i32, i32)> {
    let mut x = x0;
    BAR.iter()
        .map(|it| {
            let w = width(*it, measure);
            let at = (*it, x, w);
            x += w + SP;
            at
        })
        .collect()
}

impl Frame {
    /// The top bar of `SA-OFDM` window `win` in `r`.
    pub(crate) fn ofdm_top(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some((fs, n)) = self
            .graph
            .get(node)
            .map(|n| (n.signal.sample_rate(), n.signal.len()))
        else {
            return Vec::new();
        };
        let span = self.ofdm_span(node, n);
        let len = span.1.saturating_sub(span.0);
        let mc = self.dm_colors(cx);
        let o = self.ofdm_win(node);
        let st = &o.st;
        let busy = st.is_running();
        let (min_br, max_br) = (st.min_br, st.max_br);
        let total = len / ls_of(fs, min_br).max(1);
        let current = st.current_ls();
        let (survey, has_br) = (st.survey, st.br.is_some());
        let (min, max) = (o.min.clone(), o.max.clone());
        let y = r.y + (r.h - CTL_H) / 2;
        let measure = |s: &str| text::width(window, s, false);
        let mut out = Vec::new();
        let value = |cx: &Context<'_, Self>, x: i32, w: i32, s: String| {
            value_box(
                cx,
                Rect::new(x, y, w, CTL_H),
                &s,
                mc.field_value,
                mc.field_text,
            )
        };
        for (it, x, w) in places(r.x + 4, &measure) {
            match it {
                Item::Caption(key) => out.push(label(
                    cx,
                    Rect::new(x, y, w, CTL_H),
                    &t!(key),
                    false,
                    true,
                    Align::Left,
                )),
                Item::Spin(f) => {
                    let st = if f == OField::Min { &min } else { &max };
                    out.extend(self.of_bound_spin(
                        win,
                        node,
                        f,
                        Rect::new(x, y, w, CTL_H),
                        st,
                        !busy,
                        cx,
                    ));
                }
                Item::MinLs => out.push(value(cx, x, w, ls_of(fs, min_br).to_string())),
                Item::MaxLs => out.push(value(cx, x, w, ls_of(fs, max_br).to_string())),
                Item::Total => out.push(value(cx, x, w, total.to_string())),
                Item::Current => out.push(value(cx, x, w, current.to_string())),
                Item::Survey => {
                    let id = format!("of{win}.survey");
                    let scope = format!("mdi{win}");
                    let on: Changed<Self, bool> = Rc::new(move |v: &mut Self, on, _, _| {
                        v.ofdm_win(node).st.survey = on;
                    });
                    out.push(tip_area(
                        &id,
                        Rect::new(x - 50, y, w + 50, CTL_H),
                        &t!("ofdm-survey.hover"),
                    ));
                    out.push(check_box(
                        &mut self.kit,
                        cx,
                        Ctl {
                            id: &id,
                            scope: &scope,
                            rect: Rect::new(x, y, w, CTL_H),
                            label: "",
                            enabled: !busy,
                        },
                        survey,
                        on,
                    ));
                }
                Item::ListK => out.extend(self.of_list_k_button(
                    win,
                    node,
                    Rect::new(x, y, w, CTL_H),
                    has_br && !busy,
                    cx,
                )),
                Item::Gap => {}
            }
        }
        out
    }

    /// A `Min Br-LS` or `Max Br-LS` spin edit.
    #[allow(clippy::too_many_arguments)]
    fn of_bound_spin(
        &mut self,
        win: u32,
        node: NodeId,
        f: OField,
        rect: Rect,
        st: &SpinState,
        enabled: bool,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let id = f.id(win);
        let scope = format!("mdi{win}");
        let l = lens(move |v: &mut Self| {
            v.ofdm.nodes.get_mut(&node).map(|o| {
                if f == OField::Min {
                    &mut o.min
                } else {
                    &mut o.max
                }
            })
        });
        let on: Changed<Self, i64> = Rc::new(move |v: &mut Self, _, _, _| {
            v.ofdm_win(node).apply(f, false);
        });
        let tip = if f == OField::Min {
            t!("ofdm-min-br.hover")
        } else {
            t!("ofdm-max-br.hover")
        };
        let spin = spin_edit(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect,
                label: "",
                enabled,
            },
            st,
            &l,
            Some(on),
        );
        vec![tip_area(&id, rect, &tip), spin]
    }

    /// `List of K` (enabled after ADP).
    fn of_list_k_button(
        &mut self,
        win: u32,
        node: NodeId,
        rect: Rect,
        enabled: bool,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let id = format!("of{win}.listk");
        let scope = format!("mdi{win}");
        let tip = if enabled {
            t!("ofdm-list-k.hover")
        } else {
            t!("ofdm-list-k.disabled")
        };
        let tip_el = tip_area(&id, rect, &tip);
        let b = push_button(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect,
                label: &t!("ofdm-list-k"),
                enabled,
            },
            false,
            action(move |v: &mut Self, _, _| {
                let e = v.ofdm_win(node).st.start_k().err();
                v.ofdm_error(e);
            }),
        );
        vec![tip_el, b]
    }

    /// The status bar inside the form: the signal's window name.
    pub(crate) fn ofdm_status(
        &self,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let s = scheme(cx);
        let name = titles::window_title(&self.name_of(node));
        value_box(cx, r, &name, s.rgb(Role::Face), s.rgb(Role::Text))
    }
}
