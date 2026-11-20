//! The calculators' tables as Windows 95 list boxes: a header, rows in
//! columns, one selected row, a scroll bar. A click selects a row, a double
//! click or Enter uses it (`Use in OFDM`); the arrow keys, Page Up/Down, Home
//! and End move the selection.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, ScrollWheelEvent, Styled, canvas, px,
};

use super::{Btn, TableId};
use crate::app::Frame;
use crate::win95::button::Ctl;
use crate::win95::colors::{Role, scheme};
use crate::win95::geom::{Edge, Rect, bevel_width};
use crate::win95::kit::{KeyHandler, lens};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, Wheel, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, place, size_of};

/// The selected row and the scroll position of a table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TableState {
    pub(crate) selected: Option<usize>,
    pub(crate) scroll: ScrollState,
    /// The wheel's part row over the list.
    pub(crate) wheel: Wheel,
    /// A digest of the rows shown last: when the list is refilled (new
    /// calculator inputs) the selection and scroll position start over, as
    /// in a Windows list box.
    rows: u64,
}

impl TableState {
    /// Move the selection by `d` rows among `n` and keep it in view.
    fn step(&mut self, d: i64, n: usize) {
        if n == 0 {
            self.selected = None;
            return;
        }
        let cur = self.selected.map_or(-1, |s| s as i64);
        let next = (cur + d).clamp(0, n as i64 - 1) as usize;
        self.selected = Some(next);
        self.show(next);
    }

    /// Scroll so row `i` is visible.
    fn show(&mut self, i: usize) {
        let i = i as i32;
        let page = self.scroll.page.max(1);
        if i < self.scroll.pos {
            self.scroll.set(i);
        } else if i >= self.scroll.pos + page {
            self.scroll.set(i - page + 1);
        }
    }
}

impl Frame {
    /// The selected row of table `t`.
    pub(crate) fn table_selected(&self, t: TableId) -> Option<usize> {
        self.tools.tables.get(&t).and_then(|s| s.selected)
    }

    /// The `Use in OFDM` button of table `t`.
    fn table_use(t: TableId) -> Btn {
        match t {
            TableId::Candidates => Btn::UseCandidate,
            TableId::KList => Btn::UseK,
        }
    }

    /// Table `t` as a list box at `c.rect`.
    pub(crate) fn table_view(
        &mut self,
        t: TableId,
        c: Ctl<'_>,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let (head, rows) = self.tools.calc.table_rows(t);
        let n = rows.len();
        let visible = ((c.rect.h - 4 - LINE - 2) / LINE).max(1);
        let digest = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            rows.hash(&mut h);
            h.finish()
        };
        let st = self.tools.tables.entry(t).or_default();
        if st.rows != digest {
            st.rows = digest;
            st.selected = None;
            st.scroll.pos = 0;
        }
        if st.selected.is_some_and(|s| s >= n) {
            st.selected = None;
        }
        st.scroll = ScrollState::new(0, (n as i32 - 1).max(0), visible, st.scroll.pos);
        let st = *st;
        let focused = self.kit.has_focus(c.id);
        self.kit
            .focusable(c.id, c.scope, Some(table_keys(t, n, visible)));
        let s = scheme(cx);
        let top = st.scroll.pos.max(0) as usize;
        let shown: Vec<String> = rows
            .iter()
            .skip(top)
            .take(visible as usize)
            .cloned()
            .collect();
        let selected = st.selected;
        let paint = table_canvas(head, shown, top, selected, focused, s);
        let edge = bevel_width(Edge::Sunken);
        let bar = Bar {
            id: &format!("{}.sb", c.id),
            rect: Rect::new(
                c.rect.w - edge - THICKNESS,
                edge,
                THICKNESS,
                c.rect.h - 2 * edge,
            ),
            vertical: true,
        };
        let sb = scroll_bar(
            &self.kit,
            cx,
            bar,
            st.scroll,
            lens(move |v: &mut Self| v.tools.tables.get_mut(&t).map(|s| &mut s.scroll)),
            None,
        );
        let slot = Slot::default();
        let s2 = slot.clone();
        let measure = canvas(move |b, _, _| s2.set(b), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let id = c.id.to_owned();
        // The rows end where the scroll bar starts: a click on the bar only
        // scrolls (its own handler), it does not pick the row beside it.
        let rows_w = bar.rect.x;
        place(c.rect)
            .id(gpui::ElementId::Name(format!("{}.box", c.id).into()))
            .child(measure)
            .child(paint)
            .child(sb)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.kit.focus = Some(id.clone().into());
                    if let Some((x, y)) = slot.local(ev.position)
                        && x < rows_w
                    {
                        v.table_click(t, x, y, top, n, ev.click_count >= 2, cx);
                    }
                    cx.notify();
                }),
            )
            .on_scroll_wheel(
                cx.listener(move |v: &mut Self, ev: &ScrollWheelEvent, _, cx| {
                    // The list under the pointer scrolls, not the form around it.
                    let dy = f32::from(ev.delta.pixel_delta(px(LINE as f32)).y);
                    if let Some(st) = v.tools.tables.get_mut(&t) {
                        st.wheel.scroll(&mut st.scroll, dy, LINE as f32);
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    /// A click at `(x, y)` in table `t` whose first shown row is `top`.
    #[allow(clippy::too_many_arguments)]
    fn table_click(
        &mut self,
        t: TableId,
        x: i32,
        y: i32,
        top: usize,
        n: usize,
        double: bool,
        cx: &mut Context<'_, Self>,
    ) {
        let first = 4 + LINE;
        if x < 2 || y < first {
            return;
        }
        let i = top + ((y - first) / LINE) as usize;
        if i >= n {
            return;
        }
        if let Some(st) = self.tools.tables.get_mut(&t) {
            st.selected = Some(i);
        }
        if double {
            self.tool_button(Self::table_use(t), None, cx);
        }
    }
}

/// The list box's face: the header, the shown rows from row `top`, the
/// selection and the focus rectangle.
fn table_canvas(
    head: String,
    shown: Vec<String>,
    top: usize,
    selected: Option<usize>,
    focused: bool,
    s: crate::win95::colors::Scheme,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let all = Rect::new(0, 0, w, h);
            paint::fill(window, b.origin, all, s.of(Role::Window));
            paint::edge(window, b.origin, all, Edge::Sunken, &s);
            let inner_w = w - 4 - THICKNESS;
            let head_r = Rect::new(2, 2, inner_w, LINE + 2);
            paint::panel(window, b.origin, head_r, Edge::Raised, &s);
            let cols: Vec<&str> = head.split('\t').collect();
            let col_w = inner_w / cols.len().max(1) as i32;
            for (k, c) in (0_i32..).zip(&cols) {
                let at = paint::at(b.origin, 6 + k * col_w, 3);
                text::draw(window, cx, at, c, false, s.of(Role::Text));
            }
            for (i, row) in (0_i32..).zip(&shown) {
                let y = 4 + LINE + i * LINE;
                let is_sel = selected == Some(top + i as usize);
                let ink = if is_sel {
                    let r = Rect::new(2, y, inner_w, LINE);
                    paint::fill(window, b.origin, r, s.of(Role::Selection));
                    if focused {
                        paint::focus_rect(window, b.origin, r, s.of(Role::SelectionText));
                    }
                    s.of(Role::SelectionText)
                } else {
                    s.of(Role::Text)
                };
                for (k, cell) in (0_i32..).zip(row.split('\t')) {
                    let at = paint::at(b.origin, 6 + k * col_w, y);
                    text::draw(window, cx, at, cell, false, ink);
                }
            }
            if focused && selected.is_none() {
                let r = Rect::new(2, 4 + LINE, inner_w, LINE);
                paint::focus_rect(window, b.origin, r, s.of(Role::Text));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The keys of a table with `n` rows, `page` of them shown.
fn table_keys(t: TableId, n: usize, page: i32) -> KeyHandler<Frame> {
    Rc::new(move |v: &mut Frame, ev, _, cx| {
        let page = i64::from(page.max(1));
        let d = match ev.keystroke.key.as_str() {
            "up" => -1,
            "down" => 1,
            "pageup" => -page,
            "pagedown" => page,
            "home" => i64::MIN / 2,
            "end" => i64::MAX / 2,
            "enter" => {
                if v.table_selected(t).is_some() {
                    v.tool_button(Frame::table_use(t), None, cx);
                }
                return true;
            }
            _ => return false,
        };
        if let Some(st) = v.tools.tables.get_mut(&t) {
            st.step(d, n);
        }
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selection_moves_and_stays_in_view() {
        let mut st = TableState {
            selected: None,
            scroll: ScrollState::new(0, 19, 5, 0),
            ..TableState::default()
        };
        st.step(1, 20);
        assert_eq!(st.selected, Some(0));
        st.step(7, 20);
        assert_eq!(st.selected, Some(7));
        assert_eq!(st.scroll.pos, 3);
        st.step(i64::MAX / 2, 20);
        assert_eq!(st.selected, Some(19));
        st.step(i64::MIN / 2, 20);
        assert_eq!((st.selected, st.scroll.pos), (Some(0), 0));
        st.step(1, 0);
        assert_eq!(st.selected, None);
    }
}
