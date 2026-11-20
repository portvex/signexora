//! A scrolling Windows 95 form for the tool windows (the signal generator,
//! its calculators, the workflow guide and the shortcuts list): rows of
//! labelled controls in etched group boxes, laid out at fixed pixel positions
//! as a Delphi form, with a vertical scroll bar when the window is shorter
//! than the form. Tab moves through the controls in row order and scrolls the
//! focused one into view.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, ScrollWheelEvent,
    StatefulInteractiveElement, Styled, Window, canvas, div, px,
};

use super::{Btn, Pick, TableId, TextField, Toggle, Tool};
use crate::app::Frame;
use crate::win95::bars::tooltip;
use crate::win95::button::{Ctl, check_box, push_button, radio_button};
use crate::win95::colors::{Role, hsla, palette, rgb_of, scheme};
use crate::win95::combo::combo_box;
use crate::win95::edit::edit;
use crate::win95::geom::Rect;
use crate::win95::kit::{action, lens};
use crate::win95::label::{Align, group_box, label};
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, place};

use super::fields::Num;

/// Height of a row of controls (21 px fields, 23 px buttons).
pub(crate) const ROW: i32 = 25;
/// Height of a row of text.
pub(crate) const TEXT_ROW: i32 = LINE + 3;
/// Pixels per scroll unit (an arrow click moves one unit).
pub(crate) const UNIT: i32 = 8;
/// Width of a number field.
pub(crate) const NUM_W: i32 = 90;
/// Gap between the controls of a row.
const GAP: i32 = 6;
/// Space round the form.
const EDGE: i32 = 8;
/// Indent of the rows inside a group box.
const INDENT: i32 = 10;

/// How a text is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Plain,
    Bold,
    /// A problem to fix (red).
    Error,
    /// All is well (green).
    Ok,
    /// Worth a second look (the palette's warning colour).
    Warn,
    /// A note (grey).
    Dim,
}

/// What a place in a row holds.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Cell {
    Text(String, Tone),
    Num(Num),
    Field(TextField),
    Combo(Pick),
    Radio(Pick, usize, String),
    Check(Toggle, String),
    Button(Btn, String),
    Table(TableId),
    Preview,
}

/// A cell at its place in the row.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) x: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) cell: Cell,
    pub(crate) enabled: bool,
    /// The control's tooltip.
    pub(crate) hint: Option<String>,
}

/// A row of the form.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Line {
    pub(crate) h: i32,
    /// The caption in the label column.
    pub(crate) label: Option<String>,
    pub(crate) cells: Vec<Placed>,
    /// The cells start at the label column (no caption of their own).
    pub(crate) aligned: bool,
}

/// A whole form: rows and group boxes (caption, first row, last row).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Form {
    pub(crate) lines: Vec<Line>,
    pub(crate) groups: Vec<(String, usize, usize)>,
    /// Where the controls of labelled rows start.
    pub(crate) label_w: i32,
}

/// Measures text: `(text, bold) -> width`.
pub(crate) type Measure<'a> = &'a dyn Fn(&str, bool) -> i32;

/// Builds a [`Form`] row by row.
pub(crate) struct Builder<'a> {
    measure: Measure<'a>,
    /// Width of the form (wrapped texts break before it).
    width: i32,
    form: Form,
    group: Option<(String, usize)>,
    /// Where the next cell of the row goes.
    x: i32,
    enabled: bool,
    /// The row continues on a new row when a cell does not fit.
    wrap: bool,
}

impl<'a> Builder<'a> {
    pub(crate) fn new(measure: Measure<'a>, width: i32) -> Self {
        Self {
            measure,
            width,
            form: Form::default(),
            group: None,
            x: 0,
            enabled: true,
            wrap: false,
        }
    }

    /// The width of `s` in the form's font.
    pub(crate) fn width_of(&self, s: &str) -> i32 {
        (self.measure)(s, false)
    }

    /// Column captions over rows of number fields: each caption is as wide as
    /// its field `(caption, field width, unit)` with the unit after it.
    pub(crate) fn columns(&mut self, cols: &[(String, i32, &str)]) {
        let widths: Vec<i32> = cols
            .iter()
            .map(|(_, w, unit)| {
                if unit.is_empty() {
                    *w
                } else {
                    *w + GAP + self.width_of(unit)
                }
            })
            .collect();
        let r = self.aligned();
        for ((caption, _, _), w) in cols.iter().zip(widths) {
            r.add(w, LINE, Cell::Text(caption.clone(), Tone::Plain));
        }
    }

    /// Start a group box (ending the open one).
    pub(crate) fn group(&mut self, caption: &str) {
        self.end_group();
        self.group = Some((caption.to_owned(), self.form.lines.len()));
    }

    /// End the open group box.
    pub(crate) fn end_group(&mut self) {
        if let Some((caption, first)) = self.group.take() {
            let n = self.form.lines.len();
            if n > first {
                self.form.groups.push((caption, first, n - 1));
            }
        }
    }

    /// The width a row can use.
    fn inner_w(&self) -> i32 {
        let indent = if self.group.is_some() { INDENT } else { 0 };
        self.width - 2 * (EDGE + indent)
    }

    /// Start a row with a caption in the label column.
    pub(crate) fn row(&mut self, caption: &str) -> &mut Self {
        self.start(Some(caption.to_owned()), false)
    }

    /// Start a row whose cells line up with the controls of labelled rows.
    pub(crate) fn aligned(&mut self) -> &mut Self {
        self.start(None, true)
    }

    /// Start a row whose cells start at the left edge.
    pub(crate) fn line(&mut self) -> &mut Self {
        self.start(None, false)
    }

    /// Start a row that continues on new rows when it is full (a row of
    /// buttons).
    pub(crate) fn flow(&mut self) -> &mut Self {
        self.start(None, false);
        self.wrap = true;
        self
    }

    fn start(&mut self, label: Option<String>, aligned: bool) -> &mut Self {
        self.form.lines.push(Line {
            h: TEXT_ROW,
            label,
            cells: Vec::new(),
            aligned,
        });
        self.x = 0;
        self.enabled = true;
        self.wrap = false;
        self
    }

    /// Text wrapped to the form's width, one row per line.
    pub(crate) fn wrapped(&mut self, s: &str, tone: Tone) {
        let bold = tone == Tone::Bold;
        let measure = self.measure;
        let max = self.inner_w().max(40);
        for l in crate::win95::dialog::wrap(s, max, &|t| measure(t, bold)) {
            self.line().text(&l, tone);
        }
    }

    /// Whether the next cells of this row are enabled.
    pub(crate) fn when(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }

    /// Add a cell `w` x `h` to the row.
    pub(crate) fn add(&mut self, w: i32, h: i32, cell: Cell) -> &mut Self {
        if self.form.lines.is_empty() {
            self.line();
        }
        if self.wrap && self.x > 0 && self.x + w > self.inner_w() {
            let enabled = self.enabled;
            self.flow().enabled = enabled;
        }
        let enabled = self.enabled;
        let x = self.x;
        if let Some(line) = self.form.lines.last_mut() {
            line.h = line.h.max(if h > LINE { h + 4 } else { h + 3 });
            line.cells.push(Placed {
                x,
                w,
                h,
                cell,
                enabled,
                hint: None,
            });
        }
        self.x = x + w + GAP;
        self
    }

    /// Give the last cell a tooltip.
    pub(crate) fn hint(&mut self, s: &str) -> &mut Self {
        if let Some(c) = self.form.lines.last_mut().and_then(|l| l.cells.last_mut()) {
            c.hint = Some(s.to_owned());
        }
        self
    }

    pub(crate) fn text(&mut self, s: &str, tone: Tone) -> &mut Self {
        let w = (self.measure)(s, tone == Tone::Bold);
        self.add(w, LINE, Cell::Text(s.to_owned(), tone))
    }

    /// A number field with its unit after it.
    pub(crate) fn num(&mut self, n: Num) -> &mut Self {
        self.num_w(n, NUM_W)
    }

    pub(crate) fn num_w(&mut self, n: Num, w: i32) -> &mut Self {
        self.add(w, 21, Cell::Num(n));
        let unit = n.unit();
        if !unit.is_empty() {
            self.text(unit, Tone::Plain);
        }
        self
    }

    pub(crate) fn field(&mut self, f: TextField, w: i32) -> &mut Self {
        self.add(w, 21, Cell::Field(f))
    }

    pub(crate) fn combo(&mut self, p: Pick, w: i32) -> &mut Self {
        self.add(w, 21, Cell::Combo(p))
    }

    pub(crate) fn radio(&mut self, p: Pick, i: usize, caption: &str) -> &mut Self {
        let w = (self.measure)(caption, false) + 22;
        self.add(w, 21, Cell::Radio(p, i, caption.to_owned()))
    }

    pub(crate) fn check(&mut self, t: Toggle, caption: &str) -> &mut Self {
        let w = (self.measure)(caption, false) + 22;
        self.add(w, 21, Cell::Check(t, caption.to_owned()))
    }

    pub(crate) fn button(&mut self, b: Btn, caption: &str) -> &mut Self {
        let w = ((self.measure)(&text::mnemonic(caption).text, false) + 16).max(75);
        self.add(w, 23, Cell::Button(b, caption.to_owned()))
    }

    /// A small button sized to its caption.
    pub(crate) fn small_button(&mut self, b: Btn, caption: &str) -> &mut Self {
        let w = (self.measure)(&text::mnemonic(caption).text, false) + 14;
        self.add(w, 23, Cell::Button(b, caption.to_owned()))
    }

    /// A list `rows` lines high across the row.
    pub(crate) fn table(&mut self, id: TableId, rows: i32) -> &mut Self {
        let w = self.inner_w();
        self.add(w, (rows + 1) * LINE + 6, Cell::Table(id))
    }

    /// The preview plot across the row.
    pub(crate) fn preview(&mut self, h: i32) -> &mut Self {
        let w = self.inner_w();
        self.add(w, h, Cell::Preview)
    }

    /// The finished form.
    pub(crate) fn finish(mut self) -> Form {
        self.end_group();
        let widest = self
            .form
            .lines
            .iter()
            .filter_map(|l| l.label.as_deref())
            .map(|l| (self.measure)(&text::mnemonic(l).text, false))
            .max()
            .unwrap_or(0);
        self.form.label_w = (widest + 12).max(90);
        self.form
    }
}

/// Where the rows and group boxes of a form go.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Layout {
    /// Top of each row.
    pub(crate) rows: Vec<i32>,
    pub(crate) groups: Vec<Rect>,
    /// Left edge of each row's cells.
    pub(crate) x0: Vec<i32>,
    pub(crate) height: i32,
}

/// Lay out `form` `width` pixels wide.
pub(crate) fn layout(form: &Form, width: i32) -> Layout {
    let mut out = Layout::default();
    let mut y = EDGE;
    let mut open: Option<i32> = None;
    for (i, line) in form.lines.iter().enumerate() {
        let group = form.groups.iter().find(|(_, a, b)| (*a..=*b).contains(&i));
        if group.is_some_and(|(_, a, _)| *a == i) {
            open = Some(y);
            y += LINE + 4;
        }
        let indent = if group.is_some() { INDENT } else { 0 };
        let x = EDGE
            + indent
            + if line.label.is_some() || line.aligned {
                form.label_w
            } else {
                0
            };
        out.rows.push(y);
        out.x0.push(x);
        y += line.h;
        if let Some((_, _, b)) = group
            && *b == i
            && let Some(top) = open.take()
        {
            y += 6;
            out.groups
                .push(Rect::new(EDGE, top, width - 2 * EDGE, y - top));
            y += 6;
        }
    }
    out.height = y + EDGE;
    out
}

/// The scroll bar of a form `content` pixels high in a view `view` high.
pub(crate) fn scroll_state(content: i32, view: i32, pos: i32) -> ScrollState {
    let units = (content + UNIT - 1) / UNIT;
    ScrollState::new(0, (units - 1).max(0), (view / UNIT).max(1), pos)
}

impl Frame {
    /// The elements of `form` in `view` (client coordinates of window `win`),
    /// scrolled by the tool window's scroll bar.
    pub(crate) fn form_view(
        &mut self,
        tool: Tool,
        win: u32,
        form: &Form,
        view: Rect,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let lay = layout(form, view.w - THICKNESS);
        let scroll = self.tool_scroll(tool, &lay, form, view.h);
        let area = Rect::new(view.x, view.y, view.w - THICKNESS, view.h);
        let clip = self
            .form_layer(win, form, &lay, scroll.pos * UNIT, area, cx)
            .id(gpui::ElementId::Name(format!("form.{tool:?}").into()))
            .on_scroll_wheel(
                cx.listener(move |v: &mut Self, ev: &ScrollWheelEvent, _, cx| {
                    let d = f32::from(ev.delta.pixel_delta(px(LINE as f32)).y);
                    if let Some(w) = v.tools.wins.get_mut(&tool) {
                        w.wheel.scroll(&mut w.scroll, d, UNIT as f32);
                    }
                    cx.notify();
                }),
            );
        let bar = Bar {
            id: &format!("form.{tool:?}.sb"),
            rect: Rect::new(view.right() - THICKNESS, view.y, THICKNESS, view.h),
            vertical: true,
        };
        let sb = scroll_bar(
            &self.kit,
            cx,
            bar,
            scroll,
            lens(move |v: &mut Self| v.tools.wins.get_mut(&tool).map(|w| &mut w.scroll)),
            None,
        );
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(clip)
            .child(sb)
            .into_any_element()
    }

    /// The elements of `form` laid out as `lay` in `area`, without scrolling.
    pub(crate) fn form_static(
        &mut self,
        win: u32,
        form: &Form,
        lay: &Layout,
        area: Rect,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        self.form_layer(win, form, lay, 0, area, cx)
            .into_any_element()
    }

    /// The group boxes, captions and controls of `form` in `area`, moved up
    /// by `dy` and clipped to the area.
    fn form_layer(
        &mut self,
        win: u32,
        form: &Form,
        lay: &Layout,
        dy: i32,
        area: Rect,
        cx: &mut Context<'_, Self>,
    ) -> gpui::Div {
        let scope = format!("mdi{win}");
        let slot = Slot::default();
        let mut inner = div().relative().size_full();
        for (r, (caption, _, _)) in lay.groups.iter().zip(&form.groups) {
            inner = inner.child(group_box(cx, r.offset(0, -dy), caption));
        }
        for (i, line) in form.lines.iter().enumerate() {
            let (y, x0) = (
                lay.rows.get(i).copied().unwrap_or(0) - dy,
                lay.x0.get(i).copied().unwrap_or(0),
            );
            if let Some(l) = &line.label {
                let indent = x0 - form.label_w;
                let r = Rect::new(indent, y, form.label_w - 6, line.h.min(ROW));
                inner = inner.child(label(cx, r, l, false, true, Align::Left));
            }
            for p in &line.cells {
                let r = Rect::new(x0 + p.x, y + (line.h - p.h) / 2, p.w, p.h);
                if let Some(h) = &p.hint {
                    let id = format!("tip.{win}.{i}.{}", p.x);
                    inner = inner.child(
                        place(r)
                            .id(gpui::ElementId::Name(id.into()))
                            .tooltip(tooltip(h)),
                    );
                }
                if let (Cell::Button(b, caption), true) = (&p.cell, p.enabled)
                    && let Some(k) = text::mnemonic(caption).key
                {
                    self.tools.keys.push((win, k.to_ascii_lowercase(), *b));
                }
                let el = self.form_cell(p, r, &scope, &slot, cx);
                inner = inner.child(el);
            }
        }
        let measure = canvas(move |b, _, _| slot.set(b), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        place(area).overflow_hidden().child(measure).child(inner)
    }

    /// The tool window's scroll state for a form laid out as `lay` in a view
    /// `view_h` high; a control that just got the focus is scrolled into view.
    fn tool_scroll(&mut self, tool: Tool, lay: &Layout, form: &Form, view_h: i32) -> ScrollState {
        let focus = self.kit.focus.as_ref().map(ToString::to_string);
        let spot = focus.as_deref().and_then(|f| {
            form.lines.iter().enumerate().find_map(|(i, l)| {
                l.cells
                    .iter()
                    .any(|p| cell_id(&p.cell).as_deref() == Some(f))
                    .then(|| (lay.rows.get(i).copied().unwrap_or(0), l.h))
            })
        });
        let Some(w) = self.tools.wins.get_mut(&tool) else {
            return scroll_state(lay.height, view_h, 0);
        };
        let mut st = scroll_state(lay.height, view_h, w.scroll.pos);
        if w.last_focus != focus {
            w.last_focus.clone_from(&focus);
            if let Some((y, h)) = spot {
                let (top, bottom) = (st.pos * UNIT, st.pos * UNIT + view_h);
                if y < top {
                    st.set(y / UNIT);
                } else if y + h > bottom {
                    st.set((y + h - view_h + UNIT - 1) / UNIT);
                }
            }
        }
        w.scroll = st;
        st
    }

    /// One control of a form.
    fn form_cell(
        &mut self,
        p: &Placed,
        r: Rect,
        scope: &str,
        slot: &Slot,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let id = cell_id(&p.cell).unwrap_or_default();
        let ctl = |label: &'static str| Ctl {
            id: "",
            scope: "",
            rect: r,
            label,
            enabled: p.enabled,
        };
        let mut c = ctl("");
        c.id = &id;
        c.scope = scope;
        match &p.cell {
            Cell::Text(s, tone) => Self::form_text(r, s, *tone, cx),
            Cell::Num(n) => {
                let n = *n;
                let st = self.tools.num_state(n);
                edit(
                    &mut self.kit,
                    cx,
                    c,
                    &st,
                    lens(move |v: &mut Self| v.tools.nums.get_mut(&n)),
                    Some(action(move |v: &mut Self, _, _| v.tool_num_changed(n))),
                )
            }
            Cell::Field(f) => {
                let f = *f;
                let st = self.tools.texts.get(&f).cloned().unwrap_or_default();
                edit(
                    &mut self.kit,
                    cx,
                    c,
                    &st,
                    lens(move |v: &mut Self| v.tools.texts.get_mut(&f)),
                    Some(action(move |v: &mut Self, _, _| v.tool_text_changed(f))),
                )
            }
            Cell::Combo(pk) => {
                let pk = *pk;
                let st = self.tools.combo_state(pk);
                combo_box(
                    &mut self.kit,
                    cx,
                    c,
                    &st,
                    lens(move |v: &mut Self| v.tools.combos.get_mut(&pk)),
                    Some(Rc::new(move |v: &mut Self, i, _, _| v.tool_pick(pk, i))),
                )
            }
            Cell::Radio(pk, i, caption) => {
                let (pk, i) = (*pk, *i);
                c.label = caption;
                let on = self.tools.pick_index(pk) == i;
                radio_button(
                    &mut self.kit,
                    cx,
                    c,
                    on,
                    action(move |v: &mut Self, _, _| v.tool_pick(pk, i)),
                )
            }
            Cell::Check(t, caption) => {
                let t = *t;
                c.label = caption;
                let on = self.tools.toggled(t);
                check_box(
                    &mut self.kit,
                    cx,
                    c,
                    on,
                    Rc::new(move |v: &mut Self, on, _, _| v.tool_toggle(t, on)),
                )
            }
            Cell::Button(b, caption) => {
                let b = *b;
                c.label = caption;
                let slot = slot.clone();
                push_button(
                    &mut self.kit,
                    cx,
                    c,
                    false,
                    action(move |v: &mut Self, _, cx| {
                        let at = slot.bounds().map(|o| {
                            (
                                f32::from(o.origin.x).round() as i32 + r.x,
                                f32::from(o.origin.y).round() as i32 + r.bottom(),
                            )
                        });
                        v.tool_button(b, at, cx);
                    }),
                )
            }
            Cell::Table(t) => self.table_view(*t, c, cx),
            Cell::Preview => self.preview_view(r, cx),
        }
    }

    /// A static text in its tone.
    fn form_text(r: Rect, s: &str, tone: Tone, cx: &mut Context<'_, Self>) -> AnyElement {
        let color = match tone {
            Tone::Plain | Tone::Bold => None,
            Tone::Error => Some(hsla(rgb_of(palette(cx).error))),
            Tone::Ok => Some(hsla(rgb_of(palette(cx).ok))),
            Tone::Warn => Some(hsla(rgb_of(palette(cx).warn))),
            Tone::Dim => Some(scheme(cx).of(Role::Shadow)),
        };
        match color {
            None => label(
                cx,
                r,
                &s.replace('&', "&&"),
                tone == Tone::Bold,
                true,
                Align::Left,
            ),
            Some(c) => crate::app::colored_label(r, s, c, false),
        }
    }
}

/// The control id of a cell that takes the focus.
pub(crate) fn cell_id(c: &Cell) -> Option<String> {
    Some(match c {
        Cell::Num(n) => n.id(),
        Cell::Field(f) => format!("ocg.{f:?}"),
        Cell::Combo(p) => format!("ocg.{p:?}"),
        Cell::Radio(p, i, _) => format!("ocg.{p:?}.{i}"),
        Cell::Check(t, _) => format!("ocg.{t:?}"),
        Cell::Button(b, _) => format!("ocg.{b:?}"),
        Cell::Table(t) => format!("ocg.{t:?}"),
        Cell::Text(..) | Cell::Preview => return None,
    })
}

/// Measure with the window's font.
pub(crate) fn measurer(window: &Window) -> impl Fn(&str, bool) -> i32 + '_ {
    move |s: &str, bold: bool| text::width(window, s, bold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_stack_and_groups_wrap_them() {
        let m = |s: &str, _: bool| s.len() as i32 * 6;
        let mut b = Builder::new(&m, 400);
        b.group("Signal");
        b.row("Baud rate").num(Num::Baud);
        b.row("Carrier").num(Num::Carrier);
        b.end_group();
        b.line().text("Ready", Tone::Ok);
        let f = b.finish();
        assert_eq!(f.groups, vec![("Signal".to_owned(), 0, 1)]);
        assert_eq!(f.label_w, 90, "short labels keep the minimum column");
        let l = layout(&f, 400);
        assert_eq!(l.rows.len(), 3);
        assert!(l.rows[1] - l.rows[0] >= ROW, "{l:?}");
        assert!(l.rows[0] > EDGE, "the caption is above the first row");
        assert_eq!(l.groups.len(), 1);
        assert!(l.groups[0].bottom() <= l.rows[2], "{l:?}");
        assert_eq!(l.x0[0], EDGE + INDENT + 90);
        assert_eq!(l.x0[2], EDGE);
        let st = scroll_state(l.height, 40, 100);
        assert!(st.enabled());
        assert_eq!(st.pos, st.top());
    }
}
