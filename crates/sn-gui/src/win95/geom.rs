//! Integer pixel geometry and the Windows 95 3D borders (`DrawEdge`).
//!
//! Windows draws a two-pixel bevel as two nested one-pixel frames. In each
//! frame the top and left lines take the "light" colour and stop one pixel
//! short of the far corner; the bottom and right lines take the "dark" colour
//! and own both far corners. [`bevel`] returns those lines as rectangles, so the
//! geometry is testable without a window.

use super::colors::Role;

/// A rectangle in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Rect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

impl Rect {
    pub(crate) const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// One past the right edge.
    pub(crate) const fn right(&self) -> i32 {
        self.x + self.w
    }

    /// One past the bottom edge.
    pub(crate) const fn bottom(&self) -> i32 {
        self.y + self.h
    }

    /// The rectangle shrunk by `d` on every side (never negative in size).
    pub(crate) fn inset(&self, d: i32) -> Self {
        Self {
            x: self.x + d,
            y: self.y + d,
            w: (self.w - 2 * d).max(0),
            h: (self.h - 2 * d).max(0),
        }
    }

    /// The rectangle moved by `(dx, dy)`.
    pub(crate) const fn offset(&self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            w: self.w,
            h: self.h,
        }
    }

    /// Whether the point lies inside (right and bottom edges excluded).
    pub(crate) const fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.right() && py < self.bottom()
    }

    /// Whether the rectangle has no area.
    pub(crate) const fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// The parts of the rectangle outside `cut` (at most four: above, below,
    /// left and right of it), none of them empty.
    pub(crate) fn minus(&self, cut: Self) -> Vec<Self> {
        let (l, t) = (cut.x.max(self.x), cut.y.max(self.y));
        let (r, b) = (
            cut.right().min(self.right()),
            cut.bottom().min(self.bottom()),
        );
        if cut.is_empty() || l >= r || t >= b {
            return if self.is_empty() { vec![] } else { vec![*self] };
        }
        [
            Self::new(self.x, self.y, self.w, t - self.y),
            Self::new(self.x, b, self.w, self.bottom() - b),
            Self::new(self.x, t, l - self.x, b - t),
            Self::new(r, t, self.right() - r, b - t),
        ]
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect()
    }
}

/// The 3D border styles of Windows 95.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    /// `EDGE_RAISED`: window frames, dialog frames, menus, scroll thumbs.
    Raised,
    /// `EDGE_SUNKEN`: edit fields, list boxes, check boxes.
    Sunken,
    /// Push button (`DFCS_BUTTONPUSH`): white outside, light inside.
    Button,
    /// A push button held down: a black frame with a grey frame inside.
    ButtonPressed,
    /// A caption or toolbar-style button held down (`DFCS_PUSHED`): black and
    /// grey upper left, white and light grey lower right.
    ButtonDown,
    /// The default push button: a black frame around [`Edge::Button`].
    ButtonDefault,
    /// `EDGE_ETCHED`: group boxes and separators.
    Etched,
    /// `BDR_RAISEDINNER`: a hot flat toolbar button.
    ThinRaised,
    /// `BDR_SUNKENOUTER`: status bar panels, a pressed flat toolbar button.
    ThinSunken,
    /// A one-pixel frame in one colour (list popups, tooltips).
    Flat(Role),
}

/// One straight line of a border.
pub(crate) type Line = (Rect, Role);

/// The four lines of a one-pixel frame: `tl` on top and left, `br` on bottom
/// and right (owning the top-right and bottom-left corners).
fn frame(r: Rect, tl: Role, br: Role) -> [Line; 4] {
    [
        (Rect::new(r.x, r.y, r.w - 1, 1), tl),
        (Rect::new(r.x, r.y, 1, r.h - 1), tl),
        (Rect::new(r.x, r.bottom() - 1, r.w, 1), br),
        (Rect::new(r.right() - 1, r.y, 1, r.h), br),
    ]
}

/// The lines of the border `edge` around `r`, outer frame first.
pub(crate) fn bevel(r: Rect, edge: Edge) -> Vec<Line> {
    use Role::{DarkShadow, Highlight, Light, Shadow};
    let layers: &[(Role, Role)] = match edge {
        Edge::Raised => &[(Light, DarkShadow), (Highlight, Shadow)],
        Edge::Sunken => &[(Shadow, Highlight), (DarkShadow, Light)],
        Edge::Button => &[(Highlight, DarkShadow), (Light, Shadow)],
        Edge::ButtonPressed => &[(DarkShadow, DarkShadow), (Shadow, Shadow)],
        Edge::ButtonDown => &[(DarkShadow, Highlight), (Shadow, Light)],
        Edge::ButtonDefault => &[
            (DarkShadow, DarkShadow),
            (Highlight, DarkShadow),
            (Light, Shadow),
        ],
        Edge::Etched => &[(Shadow, Highlight), (Highlight, Shadow)],
        Edge::ThinRaised => &[(Highlight, Shadow)],
        Edge::ThinSunken => &[(Shadow, Highlight)],
        Edge::Flat(role) => return frame_lines(r, &[(role, role)]),
    };
    frame_lines(r, layers)
}

fn frame_lines(r: Rect, layers: &[(Role, Role)]) -> Vec<Line> {
    let mut out = Vec::with_capacity(layers.len() * 4);
    for (i, &(tl, br)) in (0_i32..).zip(layers) {
        let inner = r.inset(i);
        if inner.w < 2 || inner.h < 2 {
            break;
        }
        out.extend(
            frame(inner, tl, br)
                .into_iter()
                .filter(|(l, _)| !l.is_empty()),
        );
    }
    out
}

/// How many pixels the border `edge` takes on each side.
pub(crate) const fn bevel_width(edge: Edge) -> i32 {
    match edge {
        Edge::Raised
        | Edge::Sunken
        | Edge::Button
        | Edge::ButtonPressed
        | Edge::ButtonDown
        | Edge::Etched => 2,
        Edge::ButtonDefault => 3,
        Edge::ThinRaised | Edge::ThinSunken | Edge::Flat(_) => 1,
    }
}

/// The pixels of a one-pixel line from `(x0, y0)` to `(x1, y1)`, both ends
/// included (Bresenham), for glyphs drawn on the pixel grid.
pub(crate) fn pixel_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let (mut x, mut y, mut err) = (x0, y0, dx + dy);
    let mut out = Vec::with_capacity((dx.max(-dy) + 1) as usize);
    loop {
        out.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Paint the lines into a map of pixel to role (later lines win, as on screen).
    fn raster(lines: &[Line]) -> HashMap<(i32, i32), Role> {
        let mut px = HashMap::new();
        for (r, role) in lines {
            for y in r.y..r.bottom() {
                for x in r.x..r.right() {
                    px.insert((x, y), *role);
                }
            }
        }
        px
    }

    #[test]
    fn raised_bevel_corners_follow_draw_edge() {
        let r = Rect::new(10, 20, 6, 5);
        let px = raster(&bevel(r, Edge::Raised));
        // Outer frame: light top-left, black bottom-right owning the far corners.
        assert_eq!(px[&(10, 20)], Role::Light);
        assert_eq!(px[&(14, 20)], Role::Light);
        assert_eq!(px[&(15, 20)], Role::DarkShadow, "top-right corner");
        assert_eq!(px[&(10, 24)], Role::DarkShadow, "bottom-left corner");
        assert_eq!(px[&(15, 24)], Role::DarkShadow);
        // Inner frame: white top-left, grey bottom-right.
        assert_eq!(px[&(11, 21)], Role::Highlight);
        assert_eq!(px[&(14, 21)], Role::Shadow);
        assert_eq!(px[&(11, 23)], Role::Shadow);
        // Interior untouched.
        assert!(!px.contains_key(&(12, 22)));
        // Exactly the border ring: 6x5 minus the 2x1 interior.
        assert_eq!(px.len(), 6 * 5 - 2);
    }

    #[test]
    fn sunken_and_button_layers() {
        let r = Rect::new(0, 0, 8, 8);
        let s = raster(&bevel(r, Edge::Sunken));
        assert_eq!(s[&(0, 0)], Role::Shadow);
        assert_eq!(s[&(7, 7)], Role::Highlight);
        assert_eq!(s[&(1, 1)], Role::DarkShadow);
        assert_eq!(s[&(6, 6)], Role::Light);
        let b = raster(&bevel(r, Edge::Button));
        assert_eq!(b[&(0, 0)], Role::Highlight);
        assert_eq!(b[&(7, 0)], Role::DarkShadow);
        assert_eq!(b[&(1, 1)], Role::Light);
        assert_eq!(b[&(6, 6)], Role::Shadow);
        let d = raster(&bevel(r, Edge::ButtonDefault));
        assert_eq!(d[&(0, 0)], Role::DarkShadow);
        assert_eq!(d[&(1, 1)], Role::Highlight);
        assert_eq!(d[&(2, 2)], Role::Light);
        assert_eq!(d[&(5, 5)], Role::Shadow);
    }

    #[test]
    fn etched_is_grey_then_white() {
        let px = raster(&bevel(Rect::new(0, 0, 10, 10), Edge::Etched));
        assert_eq!(px[&(0, 0)], Role::Shadow);
        assert_eq!(px[&(1, 1)], Role::Highlight);
        assert_eq!(px[&(9, 9)], Role::Highlight);
        assert_eq!(px[&(8, 8)], Role::Shadow);
    }

    #[test]
    fn widths_and_degenerate_rects() {
        assert_eq!(bevel_width(Edge::Raised), 2);
        assert_eq!(bevel_width(Edge::ButtonDefault), 3);
        assert_eq!(bevel_width(Edge::ThinSunken), 1);
        assert_eq!(
            bevel(Rect::new(0, 0, 1, 1), Edge::Raised),
            Vec::<Line>::new()
        );
        // A 3x3 rect has room for the outer frame only.
        assert_eq!(raster(&bevel(Rect::new(0, 0, 3, 3), Edge::Raised)).len(), 8);
    }

    #[test]
    fn rect_helpers() {
        let r = Rect::new(2, 3, 10, 4);
        assert_eq!(r.right(), 12);
        assert_eq!(r.bottom(), 7);
        assert!(r.contains(2, 3));
        assert!(!r.contains(12, 3));
        assert_eq!(r.inset(1), Rect::new(3, 4, 8, 2));
        assert_eq!(r.inset(5).h, 0);
        assert_eq!(r.offset(1, -1), Rect::new(3, 2, 10, 4));
    }

    #[test]
    fn rect_minus_keeps_every_pixel_outside_the_cut() {
        let r = Rect::new(0, 0, 10, 6);
        for cut in [
            Rect::new(3, 2, 4, 2),
            Rect::new(-5, -5, 8, 8),
            Rect::new(8, 1, 10, 10),
            Rect::new(20, 20, 4, 4),
            Rect::new(0, 0, 10, 6),
        ] {
            let parts = r.minus(cut);
            for y in -1..8 {
                for x in -1..12 {
                    let n = parts.iter().filter(|p| p.contains(x, y)).count();
                    let want = usize::from(r.contains(x, y) && !cut.contains(x, y));
                    assert_eq!(n, want, "{cut:?} at ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn pixel_lines() {
        assert_eq!(pixel_line(0, 0, 3, 3), vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
        assert_eq!(pixel_line(3, 0, 0, 0).len(), 4);
        assert_eq!(pixel_line(0, 0, 0, 0), vec![(0, 0)]);
        let l = pixel_line(0, 0, 4, 2);
        assert_eq!(l.first(), Some(&(0, 0)));
        assert_eq!(l.last(), Some(&(4, 2)));
        assert_eq!(l.len(), 5);
    }
}
