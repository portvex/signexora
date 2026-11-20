//! Static labels and group boxes.

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, canvas};

use super::button::caption_ink;
use super::colors::{Role, scheme};
use super::geom::{Edge, Rect};
use super::paint;
use super::text::{self, LINE, mnemonic};
use super::widget::{place, size_of};

/// Horizontal alignment of a label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// A static label (one line per `\n`), vertically centred when it is one line
/// shorter than its rectangle. `bold` gives SA's bold toolbar captions.
pub(crate) fn label(
    cx: &App,
    r: Rect,
    caption: &str,
    bold: bool,
    enabled: bool,
    align: Align,
) -> AnyElement {
    let s = scheme(cx);
    let lines: Vec<_> = caption.split('\n').map(mnemonic).collect();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let n = lines.len() as i32;
            let mut y = if n == 1 { (h - LINE) / 2 } else { 0 };
            for l in &lines {
                let tw = text::width(window, &l.text, bold);
                let x = match align {
                    Align::Left => 0,
                    Align::Center => (w - tw) / 2,
                    Align::Right => w - tw,
                };
                text::draw_label(
                    window,
                    cx,
                    paint::at(b.origin, x, y),
                    l,
                    bold,
                    caption_ink(&s, enabled),
                    true,
                );
                y += LINE;
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// A group box: an etched frame with its caption in the top line. Place the
/// group's controls separately, on top of it.
pub(crate) fn group_box(cx: &App, r: Rect, caption: &str) -> AnyElement {
    let s = scheme(cx);
    let cap = mnemonic(caption);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let top = LINE / 2;
            let themed = super::skin::skin() != super::skin::Skin::Classic;
            if themed {
                super::skin::group(window, b.origin, Rect::new(0, top, w, h - top));
            } else {
                paint::edge(
                    window,
                    b.origin,
                    Rect::new(0, top, w, h - top),
                    Edge::Etched,
                    &s,
                );
            }
            if !cap.text.is_empty() {
                let tw = text::width(window, &cap.text, false);
                paint::fill(
                    window,
                    b.origin,
                    Rect::new(6, 0, tw + 4, LINE),
                    s.of(Role::Face),
                );
                let ink = if themed {
                    text::Ink::Color(super::colors::hsla(super::skin::group_title()))
                } else {
                    caption_ink(&s, true)
                };
                text::draw_label(
                    window,
                    cx,
                    paint::at(b.origin, 8, 0),
                    &cap,
                    false,
                    ink,
                    true,
                );
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}
