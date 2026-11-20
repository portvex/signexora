//! Small Delphi-style controls of SA's module windows (demodulator and
//! `SA-OFDM`), on the Windows 95 kit: spin pairs (an up button over a down
//! button), tinted read-only value fields, edit fields that apply on Enter,
//! fixed-pitch text and the larger texts of the OFDM wait screen.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    PathBuilder, Pixels, Point, SharedString, StatefulInteractiveElement, Styled, TextRun, Window,
    canvas, point, px,
};

use super::Frame;
use crate::win95::bars::tooltip;
use crate::win95::button::Ctl;
use crate::win95::colors::{Rgb, hsla, scheme};
use crate::win95::edit::{EditState, edit_with_keys};
use crate::win95::geom::{Edge, Rect};
use crate::win95::glyph::{self, Mode};
use crate::win95::kit::{Action, KeyHandler, Lens, action, start_repeat};
use crate::win95::marks::{self, Dir};
use crate::win95::paint;
use crate::win95::text::{self, LINE};
use crate::win95::widget::{hovered, place, size_of};

/// Width of a spin pair.
pub(crate) const SPIN_W: i32 = 14;
/// Height of a spin pair (two halves of 10).
pub(crate) const SPIN_H: i32 = 20;
/// Width of a character cell of the fixed-pitch text (Fixedsys: 8).
pub(crate) const CELL_W: i32 = 8;
/// Height of a line of the fixed-pitch text.
pub(crate) const CELL_H: i32 = LINE + 1;

/// A step callback: +1 (up) or -1 (down).
pub(crate) type Step = Rc<dyn Fn(&mut Frame, i32, &mut Window, &mut Context<'_, Frame>)>;

impl Frame {
    /// SA's spin pair at `(x, y)`: an up button over a down button, each
    /// repeating while held. `on_step` gets +1 or -1.
    pub(crate) fn dm_spin_pair(
        &self,
        cx: &mut Context<'_, Self>,
        id: &str,
        (x, y): (i32, i32),
        enabled: bool,
        tip: &str,
        on_step: &Step,
    ) -> AnyElement {
        let half = SPIN_H / 2;
        let mut root = place(Rect::new(x, y, SPIN_W, SPIN_H))
            .id(gpui::ElementId::Name(format!("{id}.tip").into()))
            .tooltip(tooltip(tip));
        for (d, dir, dy) in [(1, Dir::Up, 0), (-1, Dir::Down, half)] {
            let bid = format!("{id}.{}", if d > 0 { "up" } else { "down" });
            let f = on_step.clone();
            let step = action(move |v: &mut Self, w, cx| f(v, d, w, cx));
            root = root.child(self.dm_small_button(
                cx,
                &bid,
                Rect::new(0, dy, SPIN_W, half),
                dir,
                enabled,
                step,
            ));
        }
        root.into_any_element()
    }

    /// One half of a spin pair: a raised button with a small arrow.
    fn dm_small_button(
        &self,
        cx: &mut Context<'_, Self>,
        id: &str,
        r: Rect,
        dir: Dir,
        enabled: bool,
        step: Action<Self>,
    ) -> AnyElement {
        let s = scheme(cx);
        let held = self.kit.is_pressed(id);
        let mark = marks::arrow(dir, 5);
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let (w, h) = size_of(b);
                let down = held && hovered(window, b);
                let e = if down { Edge::ThinSunken } else { Edge::Button };
                paint::panel(window, b.origin, Rect::new(0, 0, w, h), e, &s);
                let d = i32::from(down);
                let mode = if enabled {
                    Mode::Normal
                } else {
                    Mode::Disabled
                };
                let at = ((w - mark.w) / 2 + d, (h - mark.h) / 2 + d);
                glyph::draw(window, cx, b.origin, at, &mark, mode, &s);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let el = place(r)
            .id(gpui::ElementId::Name(id.to_owned().into()))
            .child(paint);
        if !enabled {
            return el.into_any_element();
        }
        let id = id.to_owned();
        el.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |v: &mut Self, _, window, cx| {
                v.kit.press(&id, None);
                start_repeat(v, &id, step.clone(), window, cx);
                cx.notify();
            }),
        )
        .into_any_element()
    }

    /// An edit field that also applies on Enter (`on_enter`) and calls
    /// `on_change` after every edit.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dm_enter_field(
        &mut self,
        cx: &mut Context<'_, Self>,
        c: Ctl<'_>,
        st: &EditState,
        lens: Lens<Self, EditState>,
        on_change: Option<Action<Self>>,
        on_enter: Option<Action<Self>>,
        tip: &str,
    ) -> AnyElement {
        let pre: Option<KeyHandler<Self>> = on_enter.map(|f| {
            let h: KeyHandler<Self> = Rc::new(move |v, ev, w, cx| {
                if ev.keystroke.key == "enter" {
                    f(v, w, cx);
                    return true;
                }
                false
            });
            h
        });
        let r = c.rect;
        let inner = Ctl {
            rect: Rect::new(0, 0, r.w, r.h),
            ..c
        };
        let field = edit_with_keys(&mut self.kit, cx, inner, st, lens, on_change, pre);
        place(r)
            .id(gpui::ElementId::Name(format!("{}.tip", c.id).into()))
            .tooltip(tooltip(tip))
            .child(field)
            .into_any_element()
    }
}

/// A read-only value field on a tinted background, sunken (SA's LS, Total LS,
/// Current LS fields and the OFDM status bar).
pub(crate) fn value_box(cx: &App, r: Rect, text: &str, bg: Rgb, fg: Rgb) -> AnyElement {
    let s = scheme(cx);
    let text = text.to_owned();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let full = Rect::new(0, 0, w, h);
            paint::fill(window, b.origin, full, hsla(bg));
            paint::edge(window, b.origin, full, Edge::ThinSunken, &s);
            let at = paint::at(b.origin, 4, (h - LINE) / 2);
            window.with_content_mask(
                Some(gpui::ContentMask {
                    bounds: paint::bounds(b.origin, full.inset(1)),
                }),
                |window| text::draw(window, cx, at, &text, false, hsla(fg)),
            );
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// Break `text` into lines of at most `cols` characters (anywhere, as SA's
/// fixed-font panels do), keeping its own line breaks.
pub(crate) fn wrap_mono(text: &str, cols: usize) -> Vec<String> {
    let cols = cols.max(1);
    let mut out = Vec::new();
    for line in text.split('\n') {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            out.push(String::new());
        }
        for chunk in chars.chunks(cols) {
            out.push(chunk.iter().collect());
        }
    }
    out
}

/// Draw `s` in fixed-pitch cells of [`CELL_W`] with its top-left corner at
/// `(x, y)` relative to `o`.
pub(crate) fn draw_mono(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    s: &str,
    color: Hsla,
) {
    let mut buf = [0_u8; 4];
    for (i, c) in (0_i32..).zip(s.chars()) {
        if c == ' ' {
            continue;
        }
        let one: &str = c.encode_utf8(&mut buf);
        let w = text::width(window, one, false);
        let at = paint::at(o, x + i * CELL_W + (CELL_W - w) / 2, y);
        text::draw(window, cx, at, one, false, color);
    }
}

/// Draw `s` in the user-interface font at `size` px, centred on `cx_` when
/// `centre`, else from `x`; `y` is the top of the line.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_sized(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    s: &str,
    size: f32,
    centre: bool,
    color: Hsla,
) {
    let text: SharedString = s.to_owned().into();
    let runs = [TextRun {
        len: text.len(),
        font: text::font(false),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    let line = window.text_system().shape_line(text, px(size), &runs, None);
    let w = f32::from(line.width);
    let left = if centre { x as f32 - w / 2.0 } else { x as f32 };
    let at = point(o.x + px(left.round()), o.y + px(y as f32));
    line.paint(at, px(size * 1.1), window, cx).ok();
}

/// A one-pixel polyline through `pts` (relative to `o`).
pub(crate) fn polyline(window: &mut Window, o: Point<Pixels>, pts: &[(f32, f32)], color: Hsla) {
    let Some(&(x0, y0)) = pts.first() else {
        return;
    };
    let p = |x: f32, y: f32| point(o.x + px(x), o.y + px(y));
    let mut b = PathBuilder::stroke(px(1.0));
    b.move_to(p(x0, y0));
    for &(x, y) in pts.iter().skip(1) {
        b.line_to(p(x, y));
    }
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_text_breaks_anywhere_and_keeps_line_breaks() {
        assert_eq!(wrap_mono("abcdef\ngh", 4), vec!["abcd", "ef", "gh"]);
        assert_eq!(wrap_mono("a\n\nb", 4), vec!["a", "", "b"]);
        assert_eq!(wrap_mono("", 0), vec![""]);
    }
}
