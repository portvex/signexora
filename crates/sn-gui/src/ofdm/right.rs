//! The right column of `SA-OFDM` (`docs/SA_UI_SPEC.md` 10): the two
//! constellation panes with `Mode`, `Each` and `Start+` under them (dynamic
//! mode), the results list (light blue, fixed font) or the List of K (yellow
//! on navy), then `Symbols` and `Scale FFT` with their sliders,
//! `Shift Frequency` with the virtual-shift pair and four digit pairs, the Br
//! field and `ADP`, `Show` (`Break`), `Find CT`.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, Hsla, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseUpEvent, ParentElement, ScrollWheelEvent, StatefulInteractiveElement, Styled, Window,
    canvas, px,
};
use sn_core::NodeId;
use sn_ui_core::ofdm::{Line, Model, SHIFT_STEPS, k_hint, k_line, line_text};
use sn_ui_core::{format, t};

use super::{Frame, OField};
use crate::app::demod::colors::ModColors;
use crate::app::demod::ctl::{CELL_H, SPIN_W, Step, draw_mono};
use crate::app::demod::tip_area;
use crate::win95::button::{Changed, Ctl, push_button};
use crate::win95::combo::combo_box;
use crate::win95::edit::{EditState, FieldLook, SpinState, spin_edit};
use crate::win95::geom::{Edge, Rect};
use crate::win95::kit::{Lens, action, lens};
use crate::win95::label::{Align, label};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, tinted_scroll_bar};
use crate::win95::widget::{Slot, place, size_of};

/// Height of a control row.
const ROW_H: i32 = 24;
/// Height of a control.
const CTL_H: i32 = 21;
/// Largest constellation pane.
const PANE: i32 = 150;

/// A list row: its text, whether it is selected, and its hint.
type Row = (String, bool, Option<String>);

/// The rows of the results list or the List of K.
fn list_rows(o: &super::OfdmWin, fs: f64) -> Vec<Row> {
    let st = &o.st;
    if st.k_shown {
        st.k_list()
            .iter()
            .map(|(rank, c)| (k_line(c), st.k_selected == Some(*rank), Some(k_hint(c))))
            .collect()
    } else {
        st.lines
            .iter()
            .enumerate()
            .map(|(i, l)| (line_text(l, fs), st.selected_line == Some(i), None))
            .collect()
    }
}

impl Frame {
    /// The right column of `SA-OFDM` window `win` in `r`.
    pub(crate) fn ofdm_right(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let mc = self.dm_colors(cx);
        let panes = self.ofdm_win(node).st.panes;
        let mut out = Vec::new();
        if panes {
            let size = pane_size(r);
            out.extend(self.of_panes(node, Rect::new(r.x, r.y + 2, r.w, size), size, &mc));
            let y = r.y + 2 + size + 4;
            out.extend(self.of_each_row(win, node, Rect::new(r.x, y, r.w, ROW_H), window, cx));
        }
        let controls_y = r.bottom() - super::CONTROLS_H;
        out.extend(self.ofdm_list(win, node, list_rect(r, panes), &mc, cx));
        out.extend(self.of_controls(
            win,
            node,
            Rect::new(r.x, controls_y, r.w, super::CONTROLS_H),
            window,
            &mc,
            cx,
        ));
        out
    }

    /// The two constellation panes: the green-marked channel, then the red one.
    fn of_panes(&mut self, node: NodeId, r: Rect, size: i32, mc: &ModColors) -> Vec<AnyElement> {
        let st = &self.ofdm_win(node).st;
        let upto = st.show_pos + 1;
        let mut out = Vec::new();
        for k in 0..2_usize {
            let mut pts: Vec<(f32, f32)> = Vec::new();
            let mut over = false;
            let mut tip = None;
            if let Some(data) = &st.show
                && let (Some(vals), Some(scale)) = (data.symbols.values.get(k), data.scales.get(k))
            {
                for v in vals.iter().take(upto) {
                    let (x, y) = (v.re / scale, v.im / scale);
                    over |= x.abs() > 1.0 || y.abs() > 1.0;
                    pts.push((x.clamp(-1.1, 1.1), y.clamp(-1.1, 1.1)));
                }
                let bin = data.symbols.bins.get(k).copied().unwrap_or_default();
                tip = Some(t!(
                    "ofdm-pane.hover",
                    ch = bin.to_string(),
                    count = vals.len().min(upto)
                ));
            }
            let (bg, dot, border) = (mc.plot_bg, mc.mark_selected, mc.overflow);
            let paint = canvas(
                |_, _, _| (),
                move |b, (), window, _| {
                    let (w, h) = size_of(b);
                    let o = b.origin;
                    paint::fill(window, o, Rect::new(0, 0, w, h), bg);
                    let (c, rad) = (w as f32 / 2.0, w as f32 * 0.45);
                    for (x, y) in &pts {
                        let px_ = (c + x * rad).round() as i32;
                        let py = (c - y * rad).round() as i32;
                        paint::fill(window, o, Rect::new(px_ - 1, py - 1, 2, 2), dot);
                    }
                    if over {
                        for e in [
                            Rect::new(1, 1, w - 2, 2),
                            Rect::new(1, h - 3, w - 2, 2),
                            Rect::new(1, 1, 2, h - 2),
                            Rect::new(w - 3, 1, 2, h - 2),
                        ] {
                            paint::fill(window, o, e, border);
                        }
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full();
            let x = r.x + k as i32 * (size + 6);
            let mut el = place(Rect::new(x, r.y, size, size))
                .id(gpui::ElementId::Name(format!("of.pane{k}.{node}").into()))
                .child(paint);
            if let Some(t) = tip {
                el = el.tooltip(crate::win95::bars::tooltip(&t));
            }
            out.push(el.into_any_element());
        }
        out
    }

    /// `Mode`, `Each` and `Start+` under the constellations.
    fn of_each_row(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let scope = format!("mdi{win}");
        let o = self.ofdm_win(node);
        let (model, each, start) = (o.model.clone(), o.each.clone(), o.start.clone());
        let measure = |s: &str| crate::win95::text::width(window, s, false);
        let y = r.y + (r.h - CTL_H) / 2;
        let mut x = r.x;
        let mut out = Vec::new();
        let cap = t!("ofdm-mode");
        let w = measure(&cap);
        out.push(label(
            cx,
            Rect::new(x, y, w, CTL_H),
            &cap,
            false,
            true,
            Align::Left,
        ));
        x += w + 4;
        let id = format!("of{win}.mode");
        out.push(tip_area(
            &id,
            Rect::new(x, y, 44, CTL_H),
            &t!("ofdm-mode.hover"),
        ));
        let on: Changed<Self, usize> = Rc::new(move |v: &mut Self, i, _, _| {
            if let Some(m) = Model::ALL.get(i) {
                v.ofdm_win(node).st.model = *m;
            }
        });
        out.push(combo_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: Rect::new(x, y, 44, CTL_H),
                label: "",
                enabled: true,
            },
            &model,
            lens(move |v: &mut Self| v.ofdm.nodes.get_mut(&node).map(|o| &mut o.model)),
            Some(on),
        ));
        x += 44 + 8;
        for (f, key, hint, st, w) in [
            (OField::Each, "ofdm-each", "ofdm-each.hover", each, 52),
            (
                OField::Start,
                "ofdm-start-plus",
                "ofdm-start-plus.hover",
                start,
                64,
            ),
        ] {
            let cap = t!(key);
            let cw = measure(&cap);
            out.push(label(
                cx,
                Rect::new(x, y, cw, CTL_H),
                &cap,
                false,
                true,
                Align::Left,
            ));
            x += cw + 4;
            out.extend(self.ofdm_spin(win, node, f, Rect::new(x, y, w, CTL_H), &st, &t!(hint), cx));
            x += w + 8;
        }
        out
    }

    /// An `Each` or `Start+` spin edit.
    #[allow(clippy::too_many_arguments)]
    fn ofdm_spin(
        &mut self,
        win: u32,
        node: NodeId,
        f: OField,
        rect: Rect,
        st: &SpinState,
        tip: &str,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let id = f.id(win);
        let scope = format!("mdi{win}");
        let l = lens(move |v: &mut Self| {
            v.ofdm.nodes.get_mut(&node).map(|o| {
                if f == OField::Each {
                    &mut o.each
                } else {
                    &mut o.start
                }
            })
        });
        let on: Changed<Self, i64> = Rc::new(move |v: &mut Self, _, _, _| {
            v.ofdm_win(node).apply(f, false);
        });
        let spin = spin_edit(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect,
                label: "",
                enabled: true,
            },
            st,
            &l,
            Some(on),
        );
        vec![tip_area(&id, rect, tip), spin]
    }

    /// The results list or the List of K, with a scroll bar when needed.
    fn ofdm_list(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        mc: &ModColors,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(fs) = self.graph.get(node).map(|n| n.signal.sample_rate()) else {
            return Vec::new();
        };
        let o = self.ofdm_win(node);
        let k = o.st.k_shown;
        let rows = list_rows(o, fs);
        let visible = ((r.h - 4) / CELL_H).max(1);
        let long = rows.len() as i32 > visible;
        let pos = o.list_bar.pos;
        o.list_bar = ScrollState::new(0, rows.len() as i32 - 1, visible, pos);
        let bar = o.list_bar;
        let first = bar.pos.max(0) as usize;
        let list_w = if long { r.w - THICKNESS } else { r.w };
        let (bg, fg) = if k {
            (mc.k_bg, mc.k_text)
        } else {
            (mc.list_bg, mc.list_text)
        };
        let (sel_bg, sel_fg) = (mc.list_sel_bg, mc.list_sel_text);
        let shown: Vec<(String, bool)> = rows
            .iter()
            .skip(first)
            .take(visible as usize)
            .map(|(t, s, _)| (t.clone(), *s))
            .collect();
        let paint = list_canvas(
            shown,
            [bg, fg, sel_bg, sel_fg],
            crate::win95::colors::scheme(cx),
        );
        let slot = Slot::default();
        let s2 = slot.clone();
        let s3 = slot.clone();
        let measure = canvas(move |b, _, _| s2.set(b), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let mut el = place(Rect::new(r.x, r.y, list_w, r.h))
            .id(gpui::ElementId::Name(format!("of{win}.list").into()))
            .child(paint)
            .child(measure)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    if let Some((_, y)) = slot.local(ev.position) {
                        v.ofdm_list_click(node, row_at(y, first), ev.click_count >= 2);
                    }
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseUpEvent, _, cx| {
                    if let Some((_, y)) = s3.local(ev.position) {
                        let at = (
                            f32::from(ev.position.x).round() as i32,
                            f32::from(ev.position.y).round() as i32,
                        );
                        v.ofdm_list_menu(win, node, row_at(y, first), at);
                    }
                    cx.notify();
                }),
            )
            .on_scroll_wheel(
                cx.listener(move |v: &mut Self, ev: &ScrollWheelEvent, _, cx| {
                    let dy = f32::from(ev.delta.pixel_delta(px(CELL_H as f32)).y);
                    let o = v.ofdm_win(node);
                    o.list_wheel.scroll(&mut o.list_bar, dy, CELL_H as f32);
                    cx.notify();
                }),
            );
        let hint = rows
            .iter()
            .find(|(_, s, _)| *s)
            .and_then(|(_, _, h)| h.clone());
        if let Some(h) = hint {
            el = el.tooltip(crate::win95::bars::tooltip(&h));
        }
        let mut out = vec![el.into_any_element()];
        if long {
            let id = format!("of{win}.list_bar");
            out.push(crate::win95::scrollbar::scroll_bar(
                &self.kit,
                cx,
                Bar {
                    id: &id,
                    rect: Rect::new(r.right() - THICKNESS, r.y, THICKNESS, r.h),
                    vertical: true,
                },
                bar,
                lens(move |v: &mut Self| v.ofdm.nodes.get_mut(&node).map(|o| &mut o.list_bar)),
                None,
            ));
        }
        out
    }

    /// A click on list row `row` (`None`: below the rows).
    pub(crate) fn ofdm_list_click(&mut self, node: NodeId, row: Option<usize>, double: bool) {
        self.kit.focus = None;
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let span = self.ofdm_span(node, signal.len());
        let st = &mut self.ofdm_win(node).st;
        if st.k_shown {
            if let Some((rank, _)) = row.and_then(|i| st.k_list().get(i).cloned()) {
                st.k_selected = Some(rank);
            }
            return;
        }
        let line = row.and_then(|i| st.lines.get(i).cloned());
        let e = match (line, row) {
            (Some(Line::Candidate(_)), _) if double => {
                st.panes = !st.panes;
                None
            }
            (Some(Line::Candidate(_)), Some(i)) => st.pick_line(i).err(),
            (Some(Line::Finish { .. } | Line::Failed { .. }), _) => {
                if !st.is_running() {
                    st.start_ct(&signal, span);
                }
                None
            }
            (None, _) if double => {
                // A running job would put its result back into the cleared
                // state.
                st.stop();
                st.reset();
                None
            }
            _ => None,
        };
        self.ofdm_error(e);
    }

    /// `Symbols`, `Scale FFT`, `Shift Frequency`, then Br, `ADP`, `Show`,
    /// `Find CT`.
    fn of_controls(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        mc: &ModColors,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        self.of_sync_sliders(node);
        let mut out = Vec::new();
        out.extend(self.of_slider_row(
            win,
            node,
            OField::Symbols,
            Rect::new(r.x, r.y, r.w, ROW_H),
            mc,
            cx,
        ));
        out.extend(self.of_slider_row(
            win,
            node,
            OField::Scale,
            Rect::new(r.x, r.y + ROW_H, r.w, ROW_H),
            mc,
            cx,
        ));
        out.extend(self.of_shift_row(
            win,
            node,
            Rect::new(r.x, r.y + 2 * ROW_H, r.w, ROW_H),
            window,
            mc,
            cx,
        ));
        out.extend(self.of_br_row(
            win,
            node,
            Rect::new(r.x, r.y + 3 * ROW_H + 2, r.w, ROW_H),
            window,
            mc,
            cx,
        ));
        out
    }

    /// The Symbols and Scale FFT sliders from the state.
    fn of_sync_sliders(&mut self, node: NodeId) {
        let o = self.ofdm_win(node);
        o.symbols_bar = ScrollState::new(0, 498 + 50 - 1, 50, o.st.symbols as i32 - 2);
        o.scale_bar = ScrollState::new(0, 39 + 4 - 1, 4, o.st.scale_fft as i32 - 1);
    }

    /// A tinted edit field of the controls (applied as typed for numbers).
    #[allow(clippy::too_many_arguments)]
    fn of_tinted_field(
        &mut self,
        win: u32,
        node: NodeId,
        f: OField,
        rect: Rect,
        bg: u32,
        fg: u32,
        tip: &str,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let id = f.id(win);
        let scope = format!("mdi{win}");
        let o = self.ofdm_win(node);
        let mut st: EditState = o.edit_of(f).clone();
        st.look = Some(FieldLook {
            bg,
            fg,
            border: true,
        });
        let l: Lens<Self, EditState> =
            lens(move |v: &mut Self| v.ofdm.nodes.get_mut(&node).map(|o| o.edit_of(f)));
        let typed = matches!(f, OField::Symbols | OField::Scale).then(|| {
            action(move |v: &mut Self, _, _| {
                let o = v.ofdm_win(node);
                if o.edit_of(f).text.trim().parse::<i64>().is_ok() {
                    o.apply(f, false);
                }
            })
        });
        let on_enter = action(move |v: &mut Self, _, _| {
            let e = v.ofdm_win(node).apply(f, true);
            v.ofdm_error(e);
        });
        self.dm_enter_field(
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect,
                label: "",
                enabled: true,
            },
            &st,
            l,
            typed,
            Some(on_enter),
            tip,
        )
    }

    /// `Symbols` (cyan) or `Scale FFT` (lavender): field, caption, slider.
    fn of_slider_row(
        &mut self,
        win: u32,
        node: NodeId,
        f: OField,
        r: Rect,
        mc: &ModColors,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let symbols = f == OField::Symbols;
        let (fill, cap, tip) = if symbols {
            (mc.field_cyan, t!("ofdm-symbols"), t!("ofdm-symbols.hover"))
        } else {
            (
                mc.field_lavender,
                t!("ofdm-scale-fft"),
                t!("ofdm-scale-fft.hover"),
            )
        };
        let y = r.y + (r.h - CTL_H) / 2;
        let field = self.of_tinted_field(
            win,
            node,
            f,
            Rect::new(r.x, y, 56, CTL_H),
            fill,
            mc.field_text,
            &tip,
            cx,
        );
        let caption = label(
            cx,
            Rect::new(r.x + 60, y, 86, CTL_H),
            &cap,
            false,
            true,
            Align::Left,
        );
        let o = self.ofdm_win(node);
        let st = if symbols { o.symbols_bar } else { o.scale_bar };
        let id = format!("{}_bar", f.id(win));
        let bar = tinted_scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect: Rect::new(r.x + 148, r.y + (r.h - THICKNESS) / 2, r.w - 150, THICKNESS),
                vertical: false,
            },
            st,
            lens(move |v: &mut Self| {
                v.ofdm.nodes.get_mut(&node).map(|o| {
                    if symbols {
                        &mut o.symbols_bar
                    } else {
                        &mut o.scale_bar
                    }
                })
            }),
            Some(action(move |v: &mut Self, _, _| {
                let o = v.ofdm_win(node);
                if symbols {
                    o.st.symbols = (o.symbols_bar.pos + 2) as usize;
                } else {
                    o.st.scale_fft = (o.scale_bar.pos + 1) as usize;
                }
            })),
            Some(fill),
        );
        vec![field, caption, bar]
    }

    /// `Shift Frequency`: the field (Enter or leaving it applies), the
    /// virtual-shift pair and four digit pairs.
    fn of_shift_row(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        mc: &ModColors,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let y = r.y + (r.h - CTL_H) / 2;
        let mut out = vec![self.of_tinted_field(
            win,
            node,
            OField::Shift,
            Rect::new(r.x, y, 64, CTL_H),
            mc.field_value,
            mc.field_text,
            &t!("ofdm-shift.hover"),
            cx,
        )];
        let cap = t!("ofdm-shift");
        let cw = crate::win95::text::width(window, &cap, false);
        out.push(label(
            cx,
            Rect::new(r.x + 68, y, cw, CTL_H),
            &cap,
            false,
            true,
            Align::Left,
        ));
        let mut x = r.x + 68 + cw + 6;
        let py = r.y + (r.h - 20) / 2;
        let vs: Step = Rc::new(move |v: &mut Self, d, _, _| {
            v.ofdm_win(node).st.vshift += i64::from(d);
        });
        out.push(self.dm_spin_pair(
            cx,
            &format!("of{win}.vshift"),
            (x, py),
            true,
            &t!("ofdm-vshift.hover"),
            &vs,
        ));
        x += SPIN_W + 6;
        for (i, step) in SHIFT_STEPS.iter().copied().enumerate() {
            let f: Step = Rc::new(move |v: &mut Self, d, _, _| {
                let st = &mut v.ofdm_win(node).st;
                let hz = st.shift + f64::from(d) * step;
                st.set_shift(hz);
            });
            let hint = t!("ofdm-shift-step.hover", step = format::dec_trim(step, 2));
            out.push(self.dm_spin_pair(cx, &format!("of{win}.shift{i}"), (x, py), true, &hint, &f));
            x += SPIN_W + 1;
        }
        out
    }

    /// The Br field, `ADP`, `Show` (`Break` while anything runs or plays) and
    /// `Find CT`.
    #[allow(clippy::too_many_arguments)]
    fn of_br_row(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        mc: &ModColors,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let y = r.y + (r.h - CTL_H) / 2;
        let mut out = vec![self.of_tinted_field(
            win,
            node,
            OField::Br,
            Rect::new(r.x, y, 86, CTL_H),
            mc.field_value,
            mc.field_text,
            &t!("ofdm-br.hover"),
            cx,
        )];
        let st = &self.ofdm_win(node).st;
        let (busy, stoppable, has_ct) = (st.is_running(), st.stoppable(), st.ct.is_some());
        let scope = format!("mdi{win}");
        let measure = |s: &str| crate::win95::text::width(window, s, false);
        let rects = button_rects(r, stoppable, &measure);
        let enabled = [has_ct && !busy, stoppable || has_ct, !busy];
        let tips = [
            if has_ct {
                t!("ofdm-adp.hover")
            } else {
                t!("ofdm-adp.disabled")
            },
            t!("ofdm-show.hover"),
            t!("ofdm-find-ct.hover"),
        ];
        for (((name, cap, rect), enabled), tip) in rects.into_iter().zip(enabled).zip(tips) {
            let id = format!("of{win}.{name}");
            out.push(tip_area(&id, rect, &tip));
            out.push(push_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect,
                    label: &cap,
                    enabled,
                },
                false,
                action(move |v: &mut Self, _, _| v.ofdm_button(node, name)),
            ));
        }
        out
    }

    /// `ADP`, `Show` / `Break` or `Find CT` was pressed.
    pub(crate) fn ofdm_button(&mut self, node: NodeId, name: &str) {
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let span = self.ofdm_span(node, signal.len());
        let st = &mut self.ofdm_win(node).st;
        let e = match name {
            "adp" => st.start_adp().err(),
            "show" if st.stoppable() => {
                st.stop();
                None
            }
            "show" => st.start_show().err(),
            "find" => {
                st.start_ct(&signal, span);
                None
            }
            _ => None,
        };
        self.ofdm_error(e);
    }
}

/// The list's rows painted on its background: `ink` is the background, the
/// text, and the selected row's background and text.
fn list_canvas(
    shown: Vec<(String, bool)>,
    ink: [Hsla; 4],
    s: crate::win95::colors::Scheme,
) -> AnyElement {
    let [bg, fg, sel_bg, sel_fg] = ink;
    canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let o = b.origin;
            let full = Rect::new(0, 0, w, h);
            paint::fill(window, o, full, bg);
            paint::edge(window, o, full, Edge::Sunken, &s);
            let mask = gpui::ContentMask {
                bounds: paint::bounds(o, full.inset(2)),
            };
            window.with_content_mask(Some(mask), |window| {
                for (i, (t, sel)) in (0_i32..).zip(&shown) {
                    let y = 2 + i * CELL_H;
                    let color: Hsla = if *sel {
                        paint::fill(window, o, Rect::new(2, y, w - 4, CELL_H), sel_bg);
                        sel_fg
                    } else {
                        fg
                    };
                    draw_mono(window, cx, o, (4, y), t, color);
                }
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}

/// The side of a constellation pane in a right column `r`.
pub(crate) fn pane_size(r: Rect) -> i32 {
    ((r.w - 6) / 2).min(PANE)
}

/// The results list (or List of K) in a right column `r`.
pub(crate) fn list_rect(r: Rect, panes: bool) -> Rect {
    let y = if panes {
        r.y + 2 + pane_size(r) + 4 + ROW_H + 2
    } else {
        r.y + 2
    };
    let controls_y = r.bottom() - super::CONTROLS_H;
    Rect::new(r.x, y, r.w, (controls_y - 4 - y).max(CELL_H * 2))
}

/// `ADP`, `Show` (or `Break`) and `Find CT` in a right column `r`: name,
/// caption and place.
pub(crate) fn button_rects(
    r: Rect,
    stoppable: bool,
    measure: &dyn Fn(&str) -> i32,
) -> Vec<(&'static str, String, Rect)> {
    let row_y = r.bottom() - super::CONTROLS_H + 3 * ROW_H + 2;
    let y = row_y + (ROW_H - CTL_H) / 2;
    let show = if stoppable {
        t!("ofdm-break")
    } else {
        t!("ofdm-show")
    };
    let mut x = r.x + 90;
    [
        ("adp", t!("ofdm-adp")),
        ("show", show),
        ("find", t!("ofdm-find-ct")),
    ]
    .into_iter()
    .map(|(name, cap)| {
        let w = measure(&cap).max(30) + 16;
        let rect = Rect::new(x, y, w, CTL_H);
        x += w + 4;
        (name, cap, rect)
    })
    .collect()
}

/// The list row at `y` (list coordinates) with `first` the top row.
fn row_at(y: i32, first: usize) -> Option<usize> {
    let r = (y - 2).div_euclid(CELL_H);
    (r >= 0).then(|| first + r as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_from_the_top() {
        assert_eq!(row_at(2, 0), Some(0));
        assert_eq!(row_at(2 + CELL_H, 3), Some(4));
        assert_eq!(row_at(0, 0), None);
    }
}
