//! The demodulator's text panel (`docs/SA_UI_SPEC.md` 8): black, the cyan
//! fixed font, `SA - MFSK dem`, `Levels:`, `Fd:`, `Br:`, `Data as:`, `Data:`
//! and the data (SA's 32-character map or hex bytes), broken anywhere to fit;
//! a scroll bar when it is longer than the panel. Right click: `Copy`,
//! `Save as...`, `Data as`, `Cancel`.

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseUpEvent, ParentElement,
    ScrollWheelEvent, Styled, canvas, px,
};
use sn_core::NodeId;
use sn_ui_core::demod::panel_text;

use super::Frame;
use super::ctl::{CELL_H, CELL_W, draw_mono, wrap_mono};
use crate::win95::geom::Rect;
use crate::win95::kit::lens;
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, scroll_bar};
use crate::win95::widget::{Slot, place};

/// Margin inside the panel.
const PAD: i32 = 3;

/// The panel's lines for a text area `w` pixels wide.
pub(crate) fn panel_lines(text: &str, w: i32) -> Vec<String> {
    let cols = ((w - 2 * PAD) / CELL_W).max(1) as usize;
    wrap_mono(text, cols)
}

impl Frame {
    /// The text panel of demodulator window `win` in `r`.
    pub(crate) fn demod_panel(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let fs = self.graph.get(node).map_or(0.0, |n| n.signal.sample_rate());
        let mc = self.dm_colors(cx);
        let d = self.demod_win(node);
        let text = panel_text(&mut d.st, fs);
        let mut lines = panel_lines(&text, r.w);
        let rows = ((r.h - 2 * PAD) / CELL_H).max(1);
        let long = lines.len() as i32 > rows;
        let text_w = if long { r.w - THICKNESS } else { r.w };
        if long {
            lines = panel_lines(&text, text_w);
        }
        let n = lines.len() as i32;
        let pos = d.text_bar.pos;
        d.text_bar = ScrollState::new(0, n - 1, rows, pos);
        let first = d.text_bar.pos.max(0) as usize;
        let shown: Vec<String> = lines.into_iter().skip(first).take(rows as usize).collect();
        let bar = d.text_bar;
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let (w, h) = crate::win95::widget::size_of(b);
                let o = b.origin;
                paint::fill(window, o, Rect::new(0, 0, w, h), mc.plot_bg);
                window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
                    for (i, l) in (0_i32..).zip(&shown) {
                        draw_mono(window, cx, o, (PAD, PAD + i * CELL_H), l, mc.text);
                    }
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let slot = Slot::default();
        let s2 = slot.clone();
        let measure = canvas(move |b, _, _| s2.set(b), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let el = place(Rect::new(r.x, r.y, text_w, r.h))
            .id(gpui::ElementId::Name(format!("dm{win}.text").into()))
            .child(paint)
            .child(measure)
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseUpEvent, _, cx| {
                    if slot.local(ev.position).is_some() {
                        let at = (
                            f32::from(ev.position.x).round() as i32,
                            f32::from(ev.position.y).round() as i32,
                        );
                        v.open_demod_text_menu(win, node, at);
                    }
                    cx.notify();
                }),
            )
            .on_scroll_wheel(
                cx.listener(move |v: &mut Self, ev: &ScrollWheelEvent, _, cx| {
                    let dy = f32::from(ev.delta.pixel_delta(px(CELL_H as f32)).y);
                    let d = v.demod_win(node);
                    d.text_wheel.scroll(&mut d.text_bar, dy, CELL_H as f32);
                    cx.notify();
                }),
            );
        let mut out = vec![el.into_any_element()];
        if long {
            let id = format!("dm{win}.text_bar");
            out.push(scroll_bar(
                &self.kit,
                cx,
                Bar {
                    id: &id,
                    rect: Rect::new(r.right() - THICKNESS, r.y, THICKNESS, r.h),
                    vertical: true,
                },
                bar,
                lens(move |v: &mut Self| v.demod.nodes.get_mut(&node).map(|d| &mut d.text_bar)),
                None,
            ));
        }
        out
    }
}
