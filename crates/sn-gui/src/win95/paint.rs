//! Low-level painting on the pixel grid: rectangles, bevels, the dotted focus
//! rectangle and the dither pattern of scroll bar tracks.

use gpui::{Bounds, Hsla, Pixels, Point, Window, point, px, size};

use super::colors::{Role, Scheme};
use super::geom::{Edge, Line, Rect, bevel};

/// Window bounds of `r`, which is relative to `origin`.
pub(crate) fn bounds(origin: Point<Pixels>, r: Rect) -> Bounds<Pixels> {
    Bounds::new(
        point(origin.x + px(r.x as f32), origin.y + px(r.y as f32)),
        size(px(r.w as f32), px(r.h as f32)),
    )
}

/// The point `(x, y)` relative to `origin`.
pub(crate) fn at(origin: Point<Pixels>, x: i32, y: i32) -> Point<Pixels> {
    point(origin.x + px(x as f32), origin.y + px(y as f32))
}

/// Fill `r` with one colour.
pub(crate) fn fill(window: &mut Window, origin: Point<Pixels>, r: Rect, color: Hsla) {
    if !r.is_empty() {
        window.paint_quad(gpui::fill(bounds(origin, r), color));
    }
}

/// Fill `r` with a horizontal gradient from `left` to `right` (Windows 98
/// captions).
pub(crate) fn gradient_h(
    window: &mut Window,
    origin: Point<Pixels>,
    r: Rect,
    left: Hsla,
    right: Hsla,
) {
    if !r.is_empty() {
        let g = gpui::linear_gradient(
            90.0,
            gpui::linear_color_stop(left, 0.0),
            gpui::linear_color_stop(right, 1.0),
        );
        window.paint_quad(gpui::fill(bounds(origin, r), g));
    }
}

/// Paint border lines.
pub(crate) fn lines(window: &mut Window, origin: Point<Pixels>, lines: &[Line], s: &Scheme) {
    for (r, role) in lines {
        fill(window, origin, *r, s.of(*role));
    }
}

/// Paint the 3D border `edge` around `r`. In the themed styles a sunken
/// field border (edit fields, list boxes, combo boxes) is the style's
/// one-pixel line; the second pixel keeps the field's own background.
pub(crate) fn edge(window: &mut Window, origin: Point<Pixels>, r: Rect, e: Edge, s: &Scheme) {
    if e == Edge::Sunken && r.w >= 8 && r.h >= 8 && super::skin::field(window, origin, r) {
        return;
    }
    if e == Edge::Sunken
        && let Some(c) = super::skin::field_border()
        && r.w >= 2
        && r.h >= 2
    {
        let ink = super::colors::hsla(c);
        fill(window, origin, Rect::new(r.x, r.y, r.w, 1), ink);
        fill(window, origin, Rect::new(r.x, r.bottom() - 1, r.w, 1), ink);
        fill(window, origin, Rect::new(r.x, r.y, 1, r.h), ink);
        fill(window, origin, Rect::new(r.right() - 1, r.y, 1, r.h), ink);
        return;
    }
    lines(window, origin, &bevel(r, e), s);
}

/// Fill `r` with the face colour and paint the border `e` around it.
pub(crate) fn panel(window: &mut Window, origin: Point<Pixels>, r: Rect, e: Edge, s: &Scheme) {
    fill(window, origin, r, s.of(Role::Face));
    edge(window, origin, r, e, s);
}

/// The dotted focus rectangle (`DrawFocusRect`): every other pixel black.
pub(crate) fn focus_rect(window: &mut Window, origin: Point<Pixels>, r: Rect, color: Hsla) {
    for (x, y) in focus_dots(r) {
        fill(window, origin, Rect::new(x, y, 1, 1), color);
    }
}

/// The pixels of a dotted focus rectangle: the border pixels on the even squares
/// of a checkerboard anchored at the rectangle's corner.
pub(crate) fn focus_dots(r: Rect) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    if r.w < 1 || r.h < 1 {
        return out;
    }
    let on = |x: i32, y: i32| (x - r.x + y - r.y) % 2 == 0;
    // The rows and columns are disjoint by construction (no search for
    // duplicates, which made this quadratic in the perimeter).
    let mut rows = vec![r.y];
    if r.h > 1 {
        rows.push(r.bottom() - 1);
    }
    let mut cols = vec![r.x];
    if r.w > 1 {
        cols.push(r.right() - 1);
    }
    for &y in &rows {
        out.extend((r.x..r.right()).filter(|&x| on(x, y)).map(|x| (x, y)));
    }
    for y in r.y + 1..r.bottom() - 1 {
        out.extend(cols.iter().filter(|&&x| on(x, y)).map(|&x| (x, y)));
    }
    out
}

/// The dotted (dithered) frame of a window being moved or sized: `r` with a
/// border `thickness` pixels wide, as Windows 95 draws it.
pub(crate) fn drag_frame(window: &mut Window, origin: Point<Pixels>, r: Rect, thickness: i32) {
    let t = thickness;
    for side in [
        Rect::new(r.x, r.y, r.w, t),
        Rect::new(r.x, r.bottom() - t, r.w, t),
        Rect::new(r.x, r.y + t, t, r.h - 2 * t),
        Rect::new(r.right() - t, r.y + t, t, r.h - 2 * t),
    ] {
        dither(window, origin, side, gpui::black(), gpui::white());
    }
}

/// The 50% dither of Windows 95 (scroll bar tracks, checked toolbar buttons):
/// `a` with every other pixel in `b`, in a checkerboard anchored at `(0, 0)`.
pub(crate) fn dither(window: &mut Window, origin: Point<Pixels>, r: Rect, a: Hsla, b: Hsla) {
    fill(window, origin, r, a);
    for y in r.y..r.bottom() {
        let start = r.x + (r.x + y).rem_euclid(2);
        for x in (start..r.right()).step_by(2) {
            fill(window, origin, Rect::new(x, y, 1, 1), b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_dots_alternate() {
        let dots = focus_dots(Rect::new(0, 0, 5, 3));
        assert!(dots.contains(&(0, 0)));
        assert!(!dots.contains(&(1, 0)));
        assert!(dots.contains(&(2, 0)));
        assert!(dots.contains(&(4, 2)));
        assert!(!dots.contains(&(0, 1)));
        // Only border pixels.
        assert!(
            dots.iter()
                .all(|&(x, y)| x == 0 || y == 0 || x == 4 || y == 2)
        );
        assert_eq!(focus_dots(Rect::new(0, 0, 0, 3)), Vec::<(i32, i32)>::new());
    }

    #[test]
    fn thin_focus_rects_have_each_dot_once() {
        for r in [
            Rect::new(3, 4, 1, 7),
            Rect::new(3, 4, 7, 1),
            Rect::new(0, 0, 1, 1),
            Rect::new(0, 0, 800, 400),
        ] {
            let dots = focus_dots(r);
            let mut unique = dots.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), dots.len(), "{r:?}");
            assert!(dots.iter().all(|&(x, y)| r.contains(x, y)));
        }
        assert_eq!(focus_dots(Rect::new(0, 0, 1, 1)), vec![(0, 0)]);
    }
}
