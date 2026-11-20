//! SA's Correction of BR panel (`docs/SA_UI_SPEC.md` 4.6, M-p11-10, R272/a2):
//! a small panel inside the sonogram at its top left with `Existent Baud Rate`
//! (pre-filled with the measured Br), `Required Baud Rate`, the green bold
//! `Max delta change: +/- 30%`, and `Yes` and `Cancel`. The result opens in a
//! new window. Enter is Yes and Escape is Cancel, as in `sa-app`.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, canvas,
};
use sn_core::MeasurementKind;
use sn_ui_core::edits::brc_edit;
use sn_ui_core::{format, t};

use super::Frame;
use crate::app::dialogs::colored_label;
use crate::win95::bars::tooltip;
use crate::win95::button::{Ctl, push_button};
use crate::win95::colors::{Role, hsla, sa_colors, scheme};
use crate::win95::edit::{EditState, edit_with_keys};
use crate::win95::geom::Rect;
use crate::win95::kit::{KeyHandler, action, lens};
use crate::win95::label::{Align, label};
use crate::win95::paint;
use crate::win95::text::{self, LINE};
use crate::win95::widget::place;

/// Width of the two fields.
const FIELD_W: i32 = 90;
/// Height of a row.
const ROW_H: i32 = 26;

/// The panel's fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Brc {
    pub(crate) existing: EditState,
    pub(crate) required: EditState,
    /// The existing rate was pre-filled from a measured Br.
    pub(crate) measured: bool,
    /// The first field still has to get the focus.
    focus_first: bool,
    /// The field (0 existing, 1 required) a refused Yes was about: it gets the
    /// focus, its text selected, once the error box is closed.
    refocus: Option<usize>,
}

/// The focus id of field `i` of window `win`'s panel.
pub(crate) fn field_id(win: u32, i: usize) -> String {
    format!("sw{win}.brc{i}")
}

impl Frame {
    /// Open the Correction of BR panel of window `win`, pre-filled with the
    /// latest Br (empty when nothing was measured: "0" could never be valid).
    /// The Br is global (not on a node's axis), so the panel needs no node.
    pub(crate) fn open_brc(&mut self, win: u32) {
        let br = self.measured(&MeasurementKind::BaudRate).map(|m| m.value);
        let existing = br.map_or_else(String::new, |b| format::dec_trim(b, 6));
        if let Some(sw) = self.sono.wins.get_mut(&win) {
            sw.brc = Some(Brc {
                existing: EditState::new(&existing),
                required: EditState::default(),
                measured: br.is_some(),
                focus_first: true,
                refocus: None,
            });
        }
    }

    /// Close the panel of window `win`.
    fn close_brc(&mut self, win: u32) {
        if let Some(sw) = self.sono.wins.get_mut(&win) {
            sw.brc = None;
        }
        if self
            .kit
            .focus
            .as_ref()
            .is_some_and(|f| f.starts_with(&format!("sw{win}.brc")))
        {
            self.kit.focus = None;
        }
    }

    /// `Yes`: run the correction, or say what is wrong in SA's error box.
    pub(crate) fn brc_yes(&mut self, win: u32) {
        let Some(brc) = self.sono.wins.get(&win).and_then(|w| w.brc.clone()) else {
            return;
        };
        match brc_edit(&brc.existing.text, &brc.required.text) {
            Ok(e) => {
                self.close_brc(win);
                self.start_edit(win, e);
            }
            Err(e) => {
                if let Some(b) = self.sono.wins.get_mut(&win).and_then(|w| w.brc.as_mut()) {
                    b.refocus = Some(e.field());
                }
                self.alert(&e.message());
            }
        }
    }

    /// Enter in field `i`: from `Existent` with `Required` still empty it moves
    /// on to `Required`; otherwise it is `Yes`.
    fn brc_enter(&mut self, win: u32, i: usize) {
        let empty = self
            .sono
            .wins
            .get(&win)
            .and_then(|w| w.brc.as_ref())
            .is_some_and(|b| b.required.text.trim().is_empty());
        if i == 0 && empty {
            self.kit.focus = Some(field_id(win, 1).into());
        } else {
            self.brc_yes(win);
        }
    }

    /// Give the first field the focus when the panel opens, and a refused
    /// field the focus with its text selected once the error box closed.
    fn brc_focus(&mut self, win: u32) {
        let boxed = self.msgbox.is_some();
        let Some(b) = self.sono.wins.get_mut(&win).and_then(|w| w.brc.as_mut()) else {
            return;
        };
        let target = if b.focus_first {
            b.focus_first = false;
            // Without a measured Br the existing rate is typed first.
            Some(usize::from(b.measured))
        } else if !boxed {
            b.refocus.take()
        } else {
            None
        };
        if let Some(i) = target {
            if let Some(f) = [&mut b.existing, &mut b.required].into_iter().nth(i) {
                f.select_all();
            }
            self.kit.focus = Some(field_id(win, i).into());
        }
    }

    /// The panel of window `win`, if open, inside the plot `plot`.
    pub(crate) fn brc_panel(
        &mut self,
        win: u32,
        plot: Rect,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(brc) = self.sono.wins.get(&win).and_then(|w| w.brc.clone()) else {
            return Vec::new();
        };
        self.brc_focus(win);
        let texts = [t!("sw-brc-existing"), t!("sw-brc-required")];
        let label_w = texts
            .iter()
            .map(|s| text::width(window, s, false))
            .max()
            .unwrap_or(0);
        let green = t!("sw-brc-max-delta");
        let green_w = text::width(window, &green, true);
        let (yes, cancel) = (t!("sw-brc-yes"), t!("sw-popup-cancel"));
        let fx = 6 + label_w + 8;
        let gx = fx + FIELD_W + 10;
        let w = (gx + green_w.max(2 * 75 + 6) + 8).min(plot.w - 16).max(0);
        let r = Rect::new(plot.x + 8, plot.y + 8, w, 2 * ROW_H + 8);
        let s = scheme(cx);
        let bg = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let (w, h) = crate::win95::widget::size_of(b);
                paint::fill(window, b.origin, Rect::new(0, 0, w, h), s.of(Role::Face));
                paint::edge(
                    window,
                    b.origin,
                    Rect::new(0, 0, w, h),
                    crate::win95::geom::Edge::Flat(Role::Shadow),
                    &s,
                );
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let scope = format!("mdi{win}");
        // The panel takes the mouse from the plot under it.
        let mut panel = place(r).occlude().child(bg);
        for (i, caption) in texts.iter().enumerate() {
            let y = 4 + i as i32 * ROW_H;
            panel = panel.child(label(
                cx,
                Rect::new(6, y, label_w, 21),
                caption,
                false,
                true,
                Align::Left,
            ));
            let field = if i == 0 { &brc.existing } else { &brc.required };
            if i == 0 {
                // Where the existing rate came from, on hover.
                let source = if brc.measured {
                    t!("sw-brc-measured")
                } else {
                    t!("sw-brc-not-measured")
                };
                let tip = place(Rect::new(fx, y, FIELD_W, 21))
                    .id(gpui::ElementId::Name(format!("sw{win}.brc.tip").into()))
                    .tooltip(tooltip(&source));
                panel = panel.child(tip);
            }
            panel = panel.child(self.brc_field(
                win,
                i,
                Rect::new(fx, y, FIELD_W, 21),
                field,
                &scope,
                cx,
            ));
        }
        let ink = hsla(sa_colors(cx).green);
        panel = panel.child(colored_label(
            Rect::new(gx, 4 + (21 - LINE) / 2, green_w, LINE),
            &green,
            ink,
            true,
        ));
        let by = 4 + ROW_H - 1;
        for (i, (text, at)) in [(yes, gx), (cancel, gx + 81)].into_iter().enumerate() {
            let id = format!("sw{win}.brc.b{i}");
            let ctl = Ctl {
                id: &id,
                scope: &scope,
                rect: Rect::new(at, by, 75, 23),
                label: &text,
                enabled: true,
            };
            let on = if i == 0 {
                action(move |v: &mut Self, _, _| v.brc_yes(win))
            } else {
                action(move |v: &mut Self, _, _| v.close_brc(win))
            };
            panel = panel.child(push_button(&mut self.kit, cx, ctl, false, on));
        }
        vec![panel.into_any_element()]
    }

    /// Field `i` of the panel: Enter is Yes, Escape is Cancel.
    fn brc_field(
        &mut self,
        win: u32,
        i: usize,
        r: Rect,
        st: &EditState,
        scope: &str,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let id = field_id(win, i);
        let keys: KeyHandler<Self> = Rc::new(move |v: &mut Self, ev, _, _| {
            match ev.keystroke.key.as_str() {
                "enter" => v.brc_enter(win, i),
                "escape" => v.close_brc(win),
                _ => return false,
            }
            true
        });
        edit_with_keys(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope,
                rect: r,
                label: "",
                enabled: true,
            },
            st,
            lens(move |v: &mut Self| {
                let b = v.sono.wins.get_mut(&win)?.brc.as_mut()?;
                Some(if i == 0 {
                    &mut b.existing
                } else {
                    &mut b.required
                })
            }),
            None,
            Some(keys),
        )
    }
}
