//! Small pictures drawn from rectangles, pixel lines, filled polygons and text:
//! toolbar and menu glyphs (16x16), caption-button and scroll-arrow marks, check
//! marks and message-box icons. No picture uses a font symbol, so every mark
//! looks the same on every platform.
//!
//! Glyph coordinates are pixels from the glyph's top-left corner. A disabled
//! glyph is drawn embossed like Windows 95's `DSS_DISABLED`: the dark parts in
//! white one pixel lower right, then in grey. Light parts (white, light grey,
//! yellow) are left out, as Windows 95 makes the mono mask from dark pixels
//! only, so a floppy keeps its label hole instead of becoming a grey square.

use gpui::{App, Hsla, PathBuilder, Pixels, Point, Window, point, px};

use super::colors::{Rgb, Role, Scheme, hsla};
use super::geom::{Rect, pixel_line};
use super::paint;
use super::text;

/// The colour of a glyph part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ink {
    /// A fixed colour, `0xRRGGBB`.
    Rgb(Rgb),
    /// A system colour of the active scheme.
    Role(Role),
}

impl Ink {
    fn resolve(self, s: &Scheme) -> Hsla {
        match self {
            Self::Rgb(c) => hsla(c),
            Self::Role(r) => s.of(r),
        }
    }
}

/// Black, the ink of most marks.
pub(crate) const BLACK: Ink = Ink::Role(Role::Text);

/// One drawing operation of a glyph.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Op {
    /// A filled rectangle.
    Rect(Rect, Ink),
    /// A one-pixel line, both ends included.
    Line(i32, i32, i32, i32, Ink),
    /// A filled polygon (corner points in pixels; use `.0` and `.5` for crisp edges).
    Poly(Vec<(f32, f32)>, Ink),
    /// Text in the UI font, its line top at `(x, y)`.
    Text(i32, i32, String, bool, Ink),
    /// A filled circle: centre x, centre y, radius.
    Circle(f32, f32, f32, Ink),
}

/// A small picture.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Glyph {
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) ops: Vec<Op>,
}

impl Glyph {
    /// An empty glyph of the given size.
    pub(crate) fn new(w: i32, h: i32) -> Self {
        Self {
            w,
            h,
            ops: Vec::new(),
        }
    }

    /// Add a filled rectangle.
    #[must_use]
    pub(crate) fn rect(mut self, x: i32, y: i32, w: i32, h: i32, ink: Ink) -> Self {
        self.ops.push(Op::Rect(Rect::new(x, y, w, h), ink));
        self
    }

    /// Add a one-pixel line.
    #[must_use]
    pub(crate) fn line(mut self, x0: i32, y0: i32, x1: i32, y1: i32, ink: Ink) -> Self {
        self.ops.push(Op::Line(x0, y0, x1, y1, ink));
        self
    }

    /// Add rows of a pixel picture: each `#` in `rows` is a pixel in `ink`,
    /// the first row at `(x, y)`.
    #[must_use]
    pub(crate) fn bitmap(mut self, x: i32, y: i32, rows: &[&str], ink: Ink) -> Self {
        for (dy, row) in (0_i32..).zip(rows) {
            let mut run: Option<i32> = None;
            for (dx, c) in (0_i32..).zip(row.chars().chain(std::iter::once(' '))) {
                match (c == '#', run) {
                    (true, None) => run = Some(dx),
                    (false, Some(start)) => {
                        self.ops
                            .push(Op::Rect(Rect::new(x + start, y + dy, dx - start, 1), ink));
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        self
    }

    /// Add a pixel picture with several colours: each character of `rows` that
    /// appears in `palette` is a pixel in its ink; other characters are clear.
    #[must_use]
    pub(crate) fn charmap(
        mut self,
        x: i32,
        y: i32,
        rows: &[&str],
        palette: &[(char, Ink)],
    ) -> Self {
        for (dy, row) in (0_i32..).zip(rows) {
            for (dx, c) in (0_i32..).zip(row.chars()) {
                if let Some(&(_, ink)) = palette.iter().find(|(k, _)| *k == c) {
                    self.ops
                        .push(Op::Rect(Rect::new(x + dx, y + dy, 1, 1), ink));
                }
            }
        }
        self
    }

    /// Add a filled polygon.
    #[must_use]
    pub(crate) fn poly(mut self, pts: &[(f32, f32)], ink: Ink) -> Self {
        self.ops.push(Op::Poly(pts.to_vec(), ink));
        self
    }

    /// Add a filled circle.
    #[must_use]
    pub(crate) fn circle(mut self, cx: f32, cy: f32, r: f32, ink: Ink) -> Self {
        self.ops.push(Op::Circle(cx, cy, r, ink));
        self
    }

    /// Add rows of a pixel picture at `scale` pixels per `#`.
    #[must_use]
    pub(crate) fn bitmap_scaled(
        mut self,
        x: i32,
        y: i32,
        scale: i32,
        rows: &[&str],
        ink: Ink,
    ) -> Self {
        for (dy, row) in (0_i32..).zip(rows) {
            for (dx, c) in (0_i32..).zip(row.chars()) {
                if c == '#' {
                    let r = Rect::new(x + dx * scale, y + dy * scale, scale, scale);
                    self.ops.push(Op::Rect(r, ink));
                }
            }
        }
        self
    }

    /// Add text in the UI font.
    #[must_use]
    pub(crate) fn text(mut self, x: i32, y: i32, s: &str, bold: bool, ink: Ink) -> Self {
        self.ops.push(Op::Text(x, y, s.to_owned(), bold, ink));
        self
    }
}

/// How a glyph is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// In its own colours.
    Normal,
    /// Embossed grey (disabled).
    Disabled,
    /// Every part in one colour (marks on a selected menu item, for example).
    Mono(Rgb),
}

/// Draw `g` with its top-left corner at `(x, y)` relative to `origin`.
pub(crate) fn draw(
    window: &mut Window,
    cx: &mut App,
    origin: Point<Pixels>,
    (x, y): (i32, i32),
    g: &Glyph,
    mode: Mode,
    s: &Scheme,
) {
    let o = paint::at(origin, x, y);
    match mode {
        Mode::Normal => ops(window, cx, o, g, None, s),
        Mode::Mono(c) => ops(window, cx, o, g, Some(hsla(c)), s),
        Mode::Disabled => {
            let lo = paint::at(o, 1, 1);
            embossed(window, cx, lo, g, s.of(Role::Highlight), s);
            embossed(window, cx, o, g, s.of(Role::Shadow), s);
        }
    }
}

/// Whether `c` is light enough to be left out of a disabled glyph's mask
/// (Rec. 601 luma of at least 0.7: white, light grey, yellow).
fn is_light(c: Hsla) -> bool {
    let r = c.to_rgb();
    0.299 * r.r + 0.587 * r.g + 0.114 * r.b >= 0.7
}

/// One pass of a disabled glyph: dark parts in `ink`, light parts in the face
/// colour (a light part drawn over a dark one opens a hole in the mask).
fn embossed(window: &mut Window, cx: &mut App, o: Point<Pixels>, g: &Glyph, ink: Hsla, s: &Scheme) {
    let face = s.of(Role::Face);
    let pick = |i: Ink| if is_light(i.resolve(s)) { face } else { ink };
    for op in &g.ops {
        match op {
            Op::Rect(r, i) => paint::fill(window, o, *r, pick(*i)),
            Op::Line(x0, y0, x1, y1, i) => {
                let c = pick(*i);
                for (px_, py) in pixel_line(*x0, *y0, *x1, *y1) {
                    paint::fill(window, o, Rect::new(px_, py, 1, 1), c);
                }
            }
            Op::Poly(pts, i) => polygon(window, o, pts, pick(*i)),
            Op::Text(x, y, t, bold, i) => {
                text::draw(window, cx, paint::at(o, *x, *y), t, *bold, pick(*i));
            }
            Op::Circle(x, y, r, i) => circle(window, o, (*x, *y), *r, pick(*i)),
        }
    }
}

fn ops(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    g: &Glyph,
    mono: Option<Hsla>,
    s: &Scheme,
) {
    for op in &g.ops {
        let color = |ink: Ink| mono.unwrap_or_else(|| ink.resolve(s));
        match op {
            Op::Rect(r, ink) => paint::fill(window, o, *r, color(*ink)),
            Op::Line(x0, y0, x1, y1, ink) => {
                let c = color(*ink);
                for (px_, py) in pixel_line(*x0, *y0, *x1, *y1) {
                    paint::fill(window, o, Rect::new(px_, py, 1, 1), c);
                }
            }
            Op::Poly(pts, ink) => polygon(window, o, pts, color(*ink)),
            Op::Text(x, y, t, bold, ink) => {
                text::draw(window, cx, paint::at(o, *x, *y), t, *bold, color(*ink));
            }
            Op::Circle(x, y, r, ink) => circle(window, o, (*x, *y), *r, color(*ink)),
        }
    }
}

fn polygon(window: &mut Window, o: Point<Pixels>, pts: &[(f32, f32)], color: Hsla) {
    let Some(&(x0, y0)) = pts.first() else {
        return;
    };
    let p = |x: f32, y: f32| point(o.x + px(x), o.y + px(y));
    let mut b = PathBuilder::fill();
    b.move_to(p(x0, y0));
    for &(x, y) in pts.iter().skip(1) {
        b.line_to(p(x, y));
    }
    b.close();
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

/// Draw a filled circle (message-box icons) centred at `(cx_, cy)` with radius `r`.
pub(crate) fn circle(
    window: &mut Window,
    origin: Point<Pixels>,
    (cx_, cy): (f32, f32),
    r: f32,
    color: Hsla,
) {
    let mut b = PathBuilder::fill();
    let p = |x: f32, y: f32| point(origin.x + px(x), origin.y + px(y));
    b.move_to(p(cx_ + r, cy));
    b.arc_to(point(px(r), px(r)), px(0.0), false, false, p(cx_ - r, cy));
    b.arc_to(point(px(r), px(r)), px(0.0), false, false, p(cx_ + r, cy));
    b.close();
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_rows_become_runs() {
        let g = Glyph::new(7, 2).bitmap(1, 2, &["##.#", " ###"], BLACK);
        assert_eq!(
            g.ops,
            vec![
                Op::Rect(Rect::new(1, 2, 2, 1), BLACK),
                Op::Rect(Rect::new(4, 2, 1, 1), BLACK),
                Op::Rect(Rect::new(2, 3, 3, 1), BLACK),
            ]
        );
    }

    #[test]
    fn builder_collects_ops() {
        let g = Glyph::new(16, 16)
            .rect(0, 0, 2, 2, BLACK)
            .line(0, 0, 3, 3, Ink::Rgb(0xFF_00_00))
            .rect(5, 5, 1, 1, BLACK)
            .poly(&[(0.0, 0.0), (4.0, 0.0), (2.0, 2.0)], BLACK)
            .text(0, 0, "Ph", true, BLACK);
        assert_eq!(g.ops.len(), 5);
        assert_eq!((g.w, g.h), (16, 16));
    }
}
