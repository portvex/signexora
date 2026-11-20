//! The phase plane mode's upper half: the ISD corrector (`SA_UI_SPEC.md` 6.1,
//! M-p16 order): the plot with its vertical scale scroll bar (or the angles
//! histogram while `Angles` is on), group `Mode` `AM`/`X^N`, group `N`,
//! `(B)`, `Corr-OFF`/`Corr-ON`, `SaveAs`, group `Blocks`, `Auto`, `Break`,
//! `Reset`, the black text panel and the file navigation scroll bar.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, StatefulInteractiveElement, Styled, Window,
    canvas,
};
use sn_ui_core::phase::{BLOCKS, IsdMode, PhaseState};
use sn_ui_core::phase_plot::{self, FORM, Hover};
use sn_ui_core::t;

use super::{at, scroll_value, tip};
use crate::app::Frame;
use crate::app::dialogs::colored_label;
use crate::app::menus::cmd;
use crate::win95::button::{Changed, Ctl, check_box, push_button, radio_button};
use crate::win95::colors::{hsla, palette, rgb_of, scheme};
use crate::win95::combo::combo_box;
use crate::win95::geom::{Edge, Rect};
use crate::win95::kit::{action, lens};
use crate::win95::label::group_box;
use crate::win95::menu::Item;
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, place, size_of};

/// The corrector plot (and the angles histogram).
pub(crate) const PLOT: Rect = at(8, 4, 512, 196);
/// Its vertical scale scroll bar.
const PLOT_VS: Rect = at(522, 4, 16, 196);
/// The black text panel.
pub(crate) const PANEL: Rect = at(686, 4, 242, 196);
/// The file navigation scroll bar.
const NAV: Rect = at(0, 204, FORM.0, 16);
/// `Auto`, `Break`, `Reset` and `SaveAs`.
pub(crate) const AUTO: Rect = at(616, 120, 62, 22);
pub(crate) const BREAK: Rect = at(616, 146, 62, 22);
pub(crate) const RESET: Rect = at(616, 172, 62, 22);
pub(crate) const SAVE_AS: Rect = at(546, 172, 62, 22);

/// SA's red `Corr-OFF` and green `Corr-ON`.
const CORR_OFF: u32 = 0xFF_00_00;
const CORR_ON: u32 = 0x00_96_00;

impl Frame {
    /// The ISD corrector of window `win`.
    pub(crate) fn corrector_area(
        &mut self,
        win: u32,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(node) = self.phase.wins.get(&win).map(|w| w.node) else {
            return Vec::new();
        };
        let angles = self.phase.states.get(&node).is_some_and(|s| s.form.angles);
        let mut out = vec![if angles {
            self.angles_plot(win, cx)
        } else {
            self.corrector_plot(win, cx)
        }];
        out.extend(self.corrector_scrolls(win, cx));
        out.extend(self.corrector_controls(win, cx));
        out.extend(self.corrector_buttons(win, cx));
        out.push(self.text_panel(win, window, cx));
        out
    }

    /// The vertical scale and the file navigation scroll bars.
    fn corrector_scrolls(&mut self, win: u32, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(pw) = self.phase.wins.get(&win) else {
            return Vec::new();
        };
        let (vs, hs) = (pw.isd_vs, pw.isd_hs);
        let vs_id = format!("pp{win}.isdvs");
        let hs_id = format!("pp{win}.isdhs");
        vec![
            scroll_bar(
                &self.kit,
                cx,
                Bar {
                    id: &vs_id,
                    rect: PLOT_VS,
                    vertical: true,
                },
                vs,
                lens(move |v: &mut Self| v.phase_win(win).map(|w| &mut w.isd_vs)),
                Some(action(move |v: &mut Self, _, _| {
                    let p = v.phase_win(win).map(|w| scroll_value(&w.isd_vs));
                    if let (Some(p), Some(st)) = (p, v.phase_st(win)) {
                        st.form.isd_scale = p;
                    }
                })),
            ),
            scroll_bar(
                &self.kit,
                cx,
                Bar {
                    id: &hs_id,
                    rect: NAV,
                    vertical: false,
                },
                hs,
                lens(move |v: &mut Self| v.phase_win(win).map(|w| &mut w.isd_hs)),
                Some(action(move |v: &mut Self, _, _| {
                    let p = v.phase_win(win).map(|w| scroll_value(&w.isd_hs));
                    if let (Some(p), Some(st)) = (p, v.phase_st(win)) {
                        st.form.isd_pos = p;
                    }
                })),
            ),
        ]
    }

    /// Mode, N, (B), Corr-OFF/ON and `SaveAs`.
    fn corrector_controls(&mut self, win: u32, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(node) = self.phase.wins.get(&win).map(|w| w.node) else {
            return Vec::new();
        };
        let Some(st) = self.phase.states.get(&node) else {
            return Vec::new();
        };
        let (mode, isd_b, corrected) = (st.form.isd_mode, st.form.isd_b, st.corrected.is_some());
        let scope = format!("mdi{win}");
        let mut out = self.mode_group(win, mode, cx);
        out.push(group_box(cx, at(546, 58, 64, 46), "N"));
        let n_id = format!("pp{win}.isdn");
        out.push(tip(
            format!("pp{win}.t.n"),
            at(552, 74, 54, 21),
            &t!("pp-isd-n.hover"),
        ));
        if let Some(st) = self.phase.wins.get(&win).map(|w| w.isd_n.clone()) {
            let on: Changed<Self, usize> = Rc::new(move |v: &mut Self, i, _, _| {
                if let Some(st) = v.phase_st(win) {
                    st.form.isd_n = i as u32 + 1;
                }
            });
            out.push(combo_box(
                &mut self.kit,
                cx,
                Ctl {
                    id: &n_id,
                    scope: &scope,
                    rect: at(552, 74, 54, 21),
                    label: "",
                    enabled: true,
                },
                &st,
                lens(move |v: &mut Self| v.phase_win(win).map(|w| &mut w.isd_n)),
                Some(on),
            ));
        }
        let b_id = format!("pp{win}.isdb");
        out.push(tip(
            format!("pp{win}.t.b"),
            at(548, 110, 60, 18),
            &t!("pp-isd-b.hover"),
        ));
        out.push(check_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &b_id,
                scope: &scope,
                rect: at(548, 110, 60, 18),
                label: "(B)",
                enabled: true,
            },
            isd_b,
            Self::form_flag(win, |f| &mut f.isd_b),
        ));
        let (text, color, hint) = if corrected {
            (t!("pp-corr-on"), CORR_ON, t!("pp-corr-on.hover"))
        } else {
            (t!("pp-corr-off"), CORR_OFF, t!("pp-corr-off.hover"))
        };
        let r = at(550, 135, 60, LINE);
        out.push(tip(format!("pp{win}.t.corr"), r, &hint));
        out.push(colored_label(r, &text, hsla(color), false));
        let save_id = format!("pp{win}.saveas");
        out.push(tip(
            format!("pp{win}.t.save"),
            SAVE_AS,
            &t!("pp-save-as.hover"),
        ));
        out.push(push_button(
            &mut self.kit,
            cx,
            Ctl {
                id: &save_id,
                scope: &scope,
                rect: SAVE_AS,
                label: &t!("pp-save-as"),
                enabled: corrected,
            },
            false,
            action(move |v: &mut Self, _, cx| v.phase_save_corrected(win, cx)),
        ));
        out
    }

    /// The `Mode` group: `AM`, `X^N`.
    fn mode_group(
        &mut self,
        win: u32,
        mode: IsdMode,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let scope = format!("mdi{win}");
        let mut out = vec![
            tip(
                format!("pp{win}.t.mode"),
                at(546, 4, 64, 14),
                &t!("pp-mode.hover"),
            ),
            group_box(cx, at(546, 4, 64, 50), &t!("pp-mode")),
        ];
        for (k, (m, text)) in [(IsdMode::Am, "AM"), (IsdMode::Xn, "X^N")]
            .into_iter()
            .enumerate()
        {
            let id = format!("pp{win}.isd{k}");
            out.push(radio_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: at(552, 18 + 16 * k as i32, 56, 16),
                    label: text,
                    enabled: true,
                },
                mode == m,
                Self::on_st(win, move |st| st.form.isd_mode = m),
            ));
        }
        out
    }

    /// Blocks, Auto, Break and Reset.
    fn corrector_buttons(&mut self, win: u32, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(st) = self
            .phase
            .wins
            .get(&win)
            .and_then(|w| self.phase.states.get(&w.node))
        else {
            return Vec::new();
        };
        let (blocks, isd_running) = (st.form.blocks, st.isd_job.is_some());
        let scope = format!("mdi{win}");
        let mut out = vec![
            tip(
                format!("pp{win}.t.blocks"),
                at(616, 4, 62, 14),
                &t!("pp-blocks.hover"),
            ),
            group_box(cx, at(616, 4, 62, 108), &t!("pp-blocks")),
        ];
        for (k, b) in BLOCKS.into_iter().enumerate() {
            let id = format!("pp{win}.blocks{b}");
            out.push(radio_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: at(622, 18 + 17 * k as i32, 52, 16),
                    label: &b.to_string(),
                    enabled: true,
                },
                blocks == b,
                Self::on_st(win, move |st| st.form.blocks = b),
            ));
        }
        let buttons = [
            (
                AUTO,
                "auto",
                t!("pp-auto"),
                t!("pp-auto.hover"),
                !isd_running,
            ),
            (
                BREAK,
                "break",
                t!("pp-break"),
                t!("pp-isd-break.hover"),
                isd_running,
            ),
            (RESET, "reset", t!("pp-reset"), t!("pp-reset.hover"), true),
        ];
        for (r, name, text, hint, enabled) in buttons {
            let id = format!("pp{win}.{name}");
            out.push(tip(format!("pp{win}.t.{name}"), r, &hint));
            let on = match name {
                "auto" => action(move |v: &mut Self, _, _| v.phase_auto(win)),
                "break" => Self::on_st(win, |st| {
                    st.isd_job = None;
                    st.isd_from = None;
                }),
                _ => action(move |v: &mut Self, _, _| v.phase_reset(win)),
            };
            out.push(push_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: r,
                    label: &text,
                    enabled,
                },
                false,
                on,
            ));
        }
        out
    }

    /// The corrector plot: the yellow trace, cyan lines at the expected
    /// sidebands (X^N) or the clock line (AM), the red line at the PLL
    /// frequency. A left click on a peak sets the PLL frequency (X^N) or the
    /// clock (AM) there.
    fn corrector_plot(&mut self, win: u32, cx: &mut Context<'_, Self>) -> AnyElement {
        let pal = palette(cx);
        let (bg, cyan, red, yellow) = (
            hsla(rgb_of(pal.plot_bg)),
            hsla(rgb_of(pal.trace_i)),
            hsla(rgb_of(pal.cursor)),
            hsla(rgb_of(pal.trace_sum)),
        );
        let plot = self
            .phase
            .wins
            .get(&win)
            .and_then(|w| self.phase.states.get(&w.node))
            .and_then(|st| {
                st.corr.as_ref().map(|c| {
                    phase_plot::corr_plot(c, st.form.isd_scale, PLOT.w as f32, PLOT.h as f32)
                })
            });
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let (w, h) = size_of(b);
                let o = b.origin;
                paint::fill(window, o, Rect::new(0, 0, w, h), bg);
                let Some(p) = &plot else { return };
                let clip = paint::bounds(o, Rect::new(0, 0, w, h));
                window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
                    for x in &p.lines {
                        paint::fill(window, o, Rect::new(*x as i32, 0, 1, h), cyan);
                    }
                    if let Some(x) = p.pll {
                        paint::fill(window, o, Rect::new(x as i32, 0, 1, h), red);
                    }
                    paint_trace(window, o, &p.trace, yellow);
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        Self::isd_plot_el(win, "corr", paint, cx)
    }

    /// The angles histogram (R136): positive angles red upwards, negative blue
    /// downwards, 0 degrees at the right, 180 at the left; the markers as 3 px
    /// lines in their colours. A left click drops a marker, a right click
    /// removes them all.
    fn angles_plot(&mut self, win: u32, cx: &mut Context<'_, Self>) -> AnyElement {
        let pal = palette(cx);
        let (bg, red, blue) = (
            hsla(rgb_of(pal.plot_bg)),
            hsla(rgb_of(pal.cursor)),
            hsla(0x40_80_FF),
        );
        let colors: Vec<gpui::Hsla> = phase_plot::mark_colors(pal)
            .iter()
            .map(|c| hsla(rgb_of(*c)))
            .collect();
        let st = self
            .phase
            .wins
            .get(&win)
            .and_then(|w| self.phase.states.get(&w.node));
        let (w, h) = (PLOT.w as f32, PLOT.h as f32);
        let bars = st
            .map(|s| phase_plot::hist_bars(s, w, h))
            .unwrap_or_default();
        let marks: Vec<f64> = st.map(|s| s.marks.clone()).unwrap_or_default();
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let (pw, ph) = size_of(b);
                let o = b.origin;
                paint::fill(window, o, Rect::new(0, 0, pw, ph), bg);
                let mid = ph / 2;
                for bar in &bars {
                    let (x, y) = (bar.x as i32, bar.y.round() as i32);
                    let (top, len, c) = if bar.up {
                        (y, mid - y, red)
                    } else {
                        (mid, y - mid, blue)
                    };
                    if len > 0 {
                        paint::fill(window, o, Rect::new(x, top, 1, len), c);
                    }
                }
                for (k, a) in marks.iter().enumerate() {
                    let x = phase_plot::hist_x(*a, pw as f32) as i32;
                    let c = colors.get(k).copied().unwrap_or(red);
                    paint::fill(window, o, Rect::new(x - 1, 0, 3, ph), c);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        Self::isd_plot_el(win, "angles", paint, cx)
    }

    /// A plot of the corrector area with its mouse handling.
    fn isd_plot_el(
        win: u32,
        name: &str,
        paint: impl IntoElement,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let slot = Slot::default();
        let s0 = slot.clone();
        let (s1, s2) = (slot.clone(), slot);
        place(PLOT)
            .id(gpui::ElementId::Name(format!("pp{win}.{name}").into()))
            .child(paint)
            .child(
                canvas(move |b, _, _| s0.set(b), |_, (), _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.kit.focus = None;
                    if let Some((x, y)) = s1.local(ev.position) {
                        v.corrector_click(win, x, y);
                    }
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, _: &MouseUpEvent, _, cx| {
                    if let Some(st) = v.phase_st(win)
                        && st.form.angles
                    {
                        st.marks.clear();
                    }
                    cx.notify();
                }),
            )
            .on_mouse_move(
                cx.listener(move |v: &mut Self, ev: &MouseMoveEvent, _, cx| {
                    if let Some((x, _)) = s2.local(ev.position) {
                        v.corrector_hover(win, x);
                    }
                    cx.notify();
                }),
            )
            .on_hover(cx.listener(move |v: &mut Self, over: &bool, _, cx| {
                if !*over && let Some(w) = v.phase_win(win) {
                    w.hover = Hover::default();
                    cx.notify();
                }
            }))
            .into_any_element()
    }

    /// A left click at `(x, y)` on the corrector plot or the histogram.
    pub(crate) fn corrector_click(&mut self, win: u32, x: i32, y: i32) {
        let Some(st) = self.phase_st(win) else {
            return;
        };
        let (w, h) = (PLOT.w as f32, PLOT.h as f32);
        if st.form.angles {
            st.add_mark(phase_plot::hist_mark_at(x as f32, y as f32, w, h));
        } else if let Some(c) = st.corr.clone() {
            phase_plot::corr_click(&mut st.form, &c, x as f32, w);
        }
    }

    /// The pointer at column `x` of the corrector plot or the histogram.
    fn corrector_hover(&mut self, win: u32, x: i32) {
        let hover = self.phase_st(win).map(|st| hover_at(st, x));
        if let (Some(h), Some(w)) = (hover, self.phase_win(win)) {
            w.hover = h;
        }
    }

    /// The black text panel: the angle markers' values or the PSK demodulator
    /// output. A right click offers to copy it (SigNexora).
    fn text_panel(&mut self, win: u32, window: &Window, cx: &mut Context<'_, Self>) -> AnyElement {
        let pal = palette(cx);
        let s = scheme(cx);
        let (bg, ink) = (hsla(rgb_of(pal.plot_bg)), hsla(rgb_of(pal.points)));
        let colors: Vec<gpui::Hsla> = phase_plot::mark_colors(pal)
            .iter()
            .map(|c| hsla(rgb_of(*c)))
            .collect();
        let lines = self
            .phase
            .wins
            .get(&win)
            .and_then(|w| self.phase.states.get(&w.node))
            .map(PhaseState::panel_lines)
            .unwrap_or_default();
        let digit = text::width(window, "0", true).max(1);
        let cols = ((PANEL.w - 8) / digit).max(1) as usize;
        let rows = ((PANEL.h - 6) / LINE).max(1) as usize;
        let shown = panel_rows(&lines, cols, rows);
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let (w, h) = size_of(b);
                let o = b.origin;
                paint::fill(window, o, Rect::new(0, 0, w, h), bg);
                paint::edge(window, o, Rect::new(0, 0, w, h), Edge::Sunken, &s);
                for (i, (mark, l)) in (0_i32..).zip(&shown) {
                    let c = mark.and_then(|k| colors.get(k).copied()).unwrap_or(ink);
                    text::draw(window, cx, paint::at(o, 4, 3 + i * LINE), l, true, c);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        place(PANEL)
            .id(gpui::ElementId::Name(format!("pp{win}.panel").into()))
            .child(paint)
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseUpEvent, _, cx| {
                    let at = (
                        f32::from(ev.position.x).round() as i32,
                        f32::from(ev.position.y).round() as i32,
                    );
                    v.open_panel_popup(win, at);
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    /// The text panel's popup menu (SigNexora): copy the text, or every code
    /// since Start.
    pub(crate) fn open_panel_popup(&mut self, win: u32, at: (i32, i32)) {
        let has_codes = self
            .phase_st(win)
            .is_some_and(|st| st.shown().is_some() && st.play.pos > st.play.dem_from);
        let items = vec![
            Item::command(&t!("pp-copy-panel"), cmd::PP_COPY_TEXT),
            Item::command(&t!("pp-copy-codes"), cmd::PP_COPY_CODES).enabled(has_codes),
        ];
        self.open_popup(items, at);
        self.phase.popup_win = Some(win);
    }
}

/// The status bar readouts for the pointer at column `x`.
fn hover_at(st: &PhaseState, x: i32) -> Hover {
    let w = PLOT.w as f32;
    if st.form.angles {
        Hover {
            f: None,
            angle: Some(phase_plot::hist_angle(x as f32, w)),
        }
    } else {
        Hover {
            f: st
                .corr
                .as_ref()
                .map(|c| phase_plot::corr_hover_hz(c, x as f32, w)),
            angle: None,
        }
    }
}

/// The trace through `pts` as one vertical run per pixel column (what a
/// one-pixel polyline covers).
fn paint_trace(
    window: &mut Window,
    o: gpui::Point<gpui::Pixels>,
    pts: &[(f32, f32)],
    color: gpui::Hsla,
) {
    for pair in pts.windows(2) {
        let [(ax, ay), (bx, by)] = pair else { continue };
        let (x0, x1) = (ax.round() as i32, bx.round() as i32);
        let steps = (x1 - x0).max(1);
        for s in 0..steps {
            let t0 = s as f32 / steps as f32;
            let t1 = (s + 1) as f32 / steps as f32;
            let y0 = (ay + (by - ay) * t0).round() as i32;
            let y1 = (ay + (by - ay) * t1).round() as i32;
            let (top, bottom) = (y0.min(y1), y0.max(y1));
            paint::fill(
                window,
                o,
                Rect::new(x0 + s, top, 1, bottom - top + 1),
                color,
            );
        }
    }
}

/// The panel's rows: lines longer than `cols` characters wrap, and only the
/// last `rows` rows show (the panel follows the newest output).
pub(crate) fn panel_rows(
    lines: &[(Option<usize>, String)],
    cols: usize,
    rows: usize,
) -> Vec<(Option<usize>, String)> {
    let mut out = Vec::new();
    for (mark, line) in lines {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            out.push((*mark, String::new()));
        }
        for chunk in chars.chunks(cols.max(1)) {
            out.push((*mark, chunk.iter().collect()));
        }
    }
    let skip = out.len().saturating_sub(rows);
    out.split_off(skip)
}

/// Copy `text` to the clipboard.
pub(crate) fn copy_text(cx: &mut gpui::App, text: String) {
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_rows_wrap_and_keep_the_newest() {
        let lines = vec![
            (Some(0), "0 45,00".to_owned()),
            (None, "0123456789".to_owned()),
        ];
        let r = panel_rows(&lines, 4, 3);
        assert_eq!(
            r,
            vec![
                (None, "0123".to_owned()),
                (None, "4567".to_owned()),
                (None, "89".to_owned())
            ]
        );
        assert_eq!(panel_rows(&lines, 40, 9).len(), 2);
    }
}
