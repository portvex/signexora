//! The 16x16 glyphs of the WF window's bottom toolbar (`docs/SA_UI_SPEC.md`
//! 7.2): SA's green arrows, text and pages and the black filter and star,
//! redrawn as pixel pictures from the designs of `sa-app`'s `icons.rs`.
//!
//! A disabled glyph is drawn embossed by the kit; its white fills are left
//! out first, and the filled Revert square becomes an outline.

use sn_ui_core::wf_tools::WfButton;

use crate::win95::geom::Rect;
use crate::win95::glyph::{BLACK, Glyph, Ink, Op};

/// SA's green glyph colour.
const GREEN: Ink = Ink::Rgb(0x00_96_00);
const NAVY: Ink = Ink::Rgb(0x00_00_80);
const RED: Ink = Ink::Rgb(0xDC_00_00);
const PAPER: Ink = Ink::Rgb(0xFF_FF_FF);

/// A glyph being drawn; `paper` fills are left out of a disabled glyph.
struct Pen {
    g: Glyph,
    enabled: bool,
}

impl Pen {
    fn new(enabled: bool) -> Self {
        Self {
            g: Glyph::new(16, 16),
            enabled,
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, ink: Ink) {
        self.g.ops.push(Op::Rect(Rect::new(x, y, w, h), ink));
    }

    /// A fill that disappears from the disabled glyph.
    fn paper(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if self.enabled {
            self.rect(x, y, w, h, PAPER);
        }
    }

    /// A one-pixel outline of the rectangle.
    fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, ink: Ink) {
        self.rect(x, y, w, 1, ink);
        self.rect(x, y + h - 1, w, 1, ink);
        self.rect(x, y, 1, h, ink);
        self.rect(x + w - 1, y, 1, h, ink);
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, ink: Ink) {
        self.g.ops.push(Op::Line(x0, y0, x1, y1, ink));
    }

    /// A two-pixel line: the line and a copy one pixel right (steep) or down.
    fn thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, ink: Ink) {
        self.line(x0, y0, x1, y1, ink);
        if (y1 - y0).abs() > (x1 - x0).abs() {
            self.line(x0 + 1, y0, x1 + 1, y1, ink);
        } else {
            self.line(x0, y0 + 1, x1, y1 + 1, ink);
        }
    }

    fn bitmap(&mut self, x: i32, y: i32, rows: &[&str], ink: Ink) {
        let g = std::mem::take(&mut self.g);
        self.g = g.bitmap(x, y, rows, ink);
    }

    /// A triangle pointing left (`dir` -1) or right (1) with its tip at column
    /// `tip`, `w` columns long, centred on rows 7 and 8.
    fn tri_h(&mut self, tip: i32, w: i32, dir: i32, ink: Ink) {
        for i in 0..w {
            self.rect(tip - dir * i, 7 - i, 1, 2 + 2 * i, ink);
        }
    }

    /// A triangle pointing up (`dir` -1) or down (1) with its tip at row `tip`,
    /// `h` rows long, centred on columns 7 and 8.
    fn tri_v(&mut self, tip: i32, h: i32, dir: i32, ink: Ink) {
        for i in 0..h {
            self.rect(7 - i, tip - dir * i, 2 + 2 * i, 1, ink);
        }
    }
}

/// SA's bold `Z`.
const Z: [&str; 8] = [
    "######", "######", "   ###", "  ### ", " ###  ", "###   ", "######", "######",
];

/// The glyph of bottom toolbar button `b`.
pub(crate) fn button_glyph(b: WfButton, enabled: bool) -> Glyph {
    let mut p = Pen::new(enabled);
    match b {
        WfButton::ZoomIn => {
            p.tri_h(1, 5, -1, GREEN);
            p.tri_h(14, 5, 1, GREEN);
            p.rect(6, 7, 4, 2, GREEN);
        }
        WfButton::ZoomOut => {
            p.tri_h(6, 5, 1, GREEN);
            p.tri_h(9, 5, -1, GREEN);
            p.rect(7, 2, 2, 12, GREEN);
        }
        WfButton::ScaleUp => {
            p.tri_v(1, 5, -1, GREEN);
            p.tri_v(14, 5, 1, GREEN);
            p.rect(7, 6, 2, 4, GREEN);
        }
        WfButton::ScaleDown => {
            p.rect(2, 1, 12, 2, GREEN);
            p.tri_v(7, 5, 1, GREEN);
            p.tri_v(8, 5, -1, GREEN);
            p.rect(2, 13, 12, 2, GREEN);
        }
        WfButton::ZoomPlus | WfButton::ZoomMinus => {
            p.bitmap(1, 4, &Z, GREEN);
            if b == WfButton::ZoomPlus {
                p.rect(11, 5, 2, 6, GREEN);
            }
            p.rect(9, 7, 6, 2, GREEN);
        }
        WfButton::Lpf => {
            p.rect(3, 2, 1, 13, BLACK);
            p.rect(3, 14, 12, 1, BLACK);
            p.rect(3, 5, 7, 2, BLACK);
            p.thick(9, 6, 11, 8, BLACK);
            p.thick(11, 8, 12, 14, BLACK);
        }
        WfButton::Revert => revert(&mut p),
        WfButton::Synth => {
            p.rect(7, 2, 2, 12, BLACK);
            p.thick(2, 4, 13, 10, BLACK);
            p.thick(2, 10, 13, 4, BLACK);
        }
        _ => pages(&mut p, b),
    }
    p.g
}

/// Revert to original: a green square with a white return arrow.
fn revert(p: &mut Pen) {
    let arrow = if p.enabled {
        p.rect(1, 1, 14, 14, GREEN);
        PAPER
    } else {
        p.frame(1, 1, 14, 14, GREEN);
        GREEN
    };
    p.rect(10, 4, 2, 7, arrow);
    p.rect(6, 9, 6, 2, arrow);
    for i in 0..3 {
        p.rect(3 + i, 9 - i, 1, 2 + 2 * i, arrow);
    }
}

/// Save, Copy, the diamond, Fit, ACF and CCF.
fn pages(p: &mut Pen, b: WfButton) {
    match b {
        WfButton::Save => {
            p.paper(2, 2, 7, 9);
            p.frame(1, 1, 9, 11, BLACK);
            p.rect(3, 4, 5, 1, BLACK);
            p.rect(3, 6, 5, 1, BLACK);
            p.rect(7, 8, 8, 8, NAVY);
            p.paper(9, 8, 4, 3);
            p.paper(9, 13, 4, 2);
            p.rect(12, 1, 2, 3, RED);
            p.rect(10, 4, 6, 1, RED);
            p.rect(11, 5, 4, 1, RED);
            p.rect(12, 6, 2, 1, RED);
        }
        WfButton::Copy => {
            p.paper(2, 2, 8, 10);
            p.frame(1, 1, 10, 12, GREEN);
            p.paper(7, 6, 7, 9);
            p.frame(6, 5, 9, 11, GREEN);
            for y in [8, 10, 12] {
                p.rect(8, y, 5, 1, GREEN);
            }
        }
        WfButton::Demod => {
            p.thick(7, 1, 13, 7, GREEN);
            p.thick(13, 7, 7, 13, GREEN);
            p.thick(7, 13, 1, 7, GREEN);
            p.thick(1, 7, 7, 1, GREEN);
        }
        WfButton::Fit => {
            p.rect(2, 1, 12, 3, GREEN);
            p.rect(2, 12, 12, 3, GREEN);
            p.rect(3, 5, 10, 2, GREEN);
            p.thick(12, 6, 3, 9, GREEN);
            p.rect(3, 9, 10, 2, GREEN);
        }
        WfButton::Acf => {
            p.rect(1, 13, 14, 2, GREEN);
            for (x, top) in [(1, 3), (4, 8), (7, 2), (10, 8), (13, 3)] {
                p.rect(x, top, 2, 13 - top, GREEN);
            }
        }
        _ => {
            p.thick(3, 3, 11, 11, GREEN);
            p.thick(11, 3, 3, 11, GREEN);
            p.bitmap(0, 0, &["####", "###", "##", "#"], GREEN);
            p.bitmap(11, 0, &[" ####", "  ###", "   ##", "    #"], GREEN);
            p.bitmap(0, 11, &["#", "##", "###", "####"], GREEN);
            p.bitmap(11, 11, &["    #", "   ##", "  ###", " ####"], GREEN);
        }
    }
}

#[cfg(test)]
mod tests {
    use sn_ui_core::wf_tools::WF_BUTTONS;

    use super::*;

    #[test]
    fn every_glyph_draws_inside_its_16_pixels() {
        let inside = |g: &Glyph| {
            g.ops.iter().all(|op| match op {
                Op::Rect(r, _) => r.x >= 0 && r.y >= 0 && r.right() <= 16 && r.bottom() <= 16,
                Op::Line(x0, y0, x1, y1, _) => {
                    [x0, y0, x1, y1].iter().all(|v| (0..16).contains(*v))
                }
                Op::Poly(pts, _) => pts
                    .iter()
                    .all(|(x, y)| (0.0..=16.0).contains(x) && (0.0..=16.0).contains(y)),
                Op::Text(..) | Op::Circle(..) => true,
            })
        };
        for b in WF_BUTTONS {
            for enabled in [true, false] {
                let g = button_glyph(b, enabled);
                assert!(!g.ops.is_empty(), "{b:?} draws nothing");
                assert!(inside(&g), "{b:?} leaves its square");
            }
        }
    }

    #[test]
    fn disabled_glyphs_drop_their_paper() {
        for b in [WfButton::Copy, WfButton::Save, WfButton::Revert] {
            let off = button_glyph(b, false);
            assert!(
                !off.ops
                    .iter()
                    .any(|op| matches!(op, Op::Rect(_, i) if *i == PAPER)),
                "{b:?}"
            );
        }
    }
}
