//! The 16x16 glyphs of the sonogram window: the 27 bottom toolbar buttons
//! (`docs/SA_UI_SPEC.md` 4.3.2) and the popup menu's pictures (4.5), redrawn
//! as pixel pictures from the designs of `sa-app`'s `icons.rs`.
//!
//! A disabled glyph is drawn embossed by the kit; its white and pale fills are
//! left out first, as Windows drops the light pixels of a disabled bitmap.

use std::f32::consts::PI;

use sn_ui_core::sono_tools::Button;

use crate::win95::geom::Rect;
use crate::win95::glyph::{Glyph, Ink, Op};

/// SA 6.2's navy glyph colour.
pub(crate) const NAVY: Ink = Ink::Rgb(0x00_00_80);
/// The lighter blue of lenses and pages.
const LIGHT: Ink = Ink::Rgb(0x00_40_FF);
const RED: Ink = Ink::Rgb(0xDC_00_00);
const GREEN: Ink = Ink::Rgb(0x00_96_00);
const YELLOW: Ink = Ink::Rgb(0xF0_C8_28);
const BROWN: Ink = Ink::Rgb(0x8C_5A_28);
const PAPER: Ink = Ink::Rgb(0xFF_FF_FF);
const PINK: Ink = Ink::Rgb(0xFF_C8_D2);

/// The popup menu's pictures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuIcon {
    SaveAs,
    WaveForm,
    Measure,
    Adp,
    Calculator,
    Cancel,
}

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

    fn px(&mut self, x: i32, y: i32, ink: Ink) {
        self.g.ops.push(Op::Rect(Rect::new(x, y, 1, 1), ink));
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, ink: Ink) {
        self.g.ops.push(Op::Rect(Rect::new(x, y, w, h), ink));
    }

    /// A fill that disappears from the disabled glyph.
    fn paper(&mut self, x: i32, y: i32, w: i32, h: i32, ink: Ink) {
        if self.enabled {
            self.rect(x, y, w, h, ink);
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

    /// A two-pixel polyline.
    fn polyline(&mut self, pts: &[(i32, i32)], ink: Ink) {
        for w in pts.windows(2) {
            if let [(x0, y0), (x1, y1)] = w {
                self.thick(*x0, *y0, *x1, *y1, ink);
            }
        }
    }

    /// The pixels whose centres lie between radii `r0` and `r1` of `(cx, cy)`
    /// and between angles `a0` and `a1` (degrees; 0 right, 90 down).
    fn arc(&mut self, (cx, cy): (f32, f32), (r0, r1): (f32, f32), (a0, a1): (f32, f32), ink: Ink) {
        for y in 0..16 {
            for x in 0..16 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let d = dx.hypot(dy);
                if d < r0 || d > r1 {
                    continue;
                }
                let a = dy.atan2(dx) * 180.0 / PI;
                let a = if a < a0 { a + 360.0 } else { a };
                if a <= a1 {
                    self.px(x, y, ink);
                }
            }
        }
    }

    fn bitmap(&mut self, x: i32, y: i32, rows: &[&str], ink: Ink) {
        let g = std::mem::take(&mut self.g);
        self.g = g.bitmap(x, y, rows, ink);
    }

    fn poly(&mut self, pts: &[(f32, f32)], ink: Ink) {
        self.g.ops.push(Op::Poly(pts.to_vec(), ink));
    }
}

/// The glyph of bottom toolbar button `b`.
pub(crate) fn button_glyph(b: Button, enabled: bool) -> Glyph {
    let mut p = Pen::new(enabled);
    match b {
        Button::ZoomIn | Button::ZoomBack | Button::ZoomReset => magnifier(&mut p, b),
        Button::Abort => {
            p.arc((8.0, 8.0), (4.6, 6.8), (0.0, 360.0), NAVY);
            p.thick(4, 11, 11, 4, NAVY);
        }
        Button::Play => speaker(&mut p),
        Button::StopPlay => {
            p.frame(2, 2, 12, 12, NAVY);
            p.rect(5, 5, 6, 6, NAVY);
        }
        Button::BandSelect => {
            p.arc((8.0, 8.5), (4.0, 6.0), (180.0, 360.0), NAVY);
            p.rect(2, 8, 2, 7, NAVY);
            p.rect(12, 8, 2, 7, NAVY);
        }
        Button::AreaDelete => {
            p.rect(2, 3, 12, 2, NAVY);
            p.rect(2, 11, 12, 2, NAVY);
            p.rect(2, 3, 2, 10, NAVY);
            p.rect(12, 3, 2, 10, NAVY);
        }
        Button::BandReject => p.polyline(&[(0, 3), (4, 3), (7, 12), (10, 3), (14, 3)], NAVY),
        Button::AmDetector => diode(&mut p, 7),
        Button::ModifiedAm => {
            diode(&mut p, 7);
            p.rect(2, 13, 11, 1, NAVY);
        }
        Button::HalfWave => {
            diode(&mut p, 3);
            diode(&mut p, 11);
        }
        _ => text_glyph(&mut p, b),
    }
    p.g
}

/// The remaining pictures.
fn text_glyph(p: &mut Pen, b: Button) {
    match b {
        Button::Acf => {
            p.rect(1, 13, 14, 2, NAVY);
            for (x, top) in [(1, 3), (4, 8), (7, 2), (10, 8), (13, 3)] {
                p.rect(x, top, 2, 13 - top, NAVY);
            }
        }
        Button::Ccf => crossed_arrows(p),
        Button::Ofdm => ofdm(p),
        Button::PhaseDetector => p.polyline(
            &[
                (0, 12),
                (3, 12),
                (3, 3),
                (7, 3),
                (7, 12),
                (10, 12),
                (10, 3),
                (14, 3),
            ],
            NAVY,
        ),
        Button::Limiter => {
            p.rect(1, 1, 7, 2, NAVY);
            p.rect(4, 6, 1, 9, NAVY);
            p.bitmap(2, 3, &["  #", " ###", "#####"], NAVY);
            p.rect(8, 13, 7, 2, NAVY);
            p.rect(11, 1, 1, 9, NAVY);
            p.bitmap(9, 10, &["#####", " ###", "  #"], NAVY);
        }
        Button::ShiftToBase => meander(p, false),
        Button::ShiftToBaseInverted => meander(p, true),
        Button::Inverse => recycle(p),
        Button::Copy => {
            p.paper(2, 2, 6, 8, PINK);
            p.frame(1, 1, 8, 10, NAVY);
            p.paper(7, 6, 7, 8, PAPER);
            p.frame(6, 5, 9, 10, NAVY);
            for y in [8, 10, 12] {
                p.rect(8, y, 5, 1, LIGHT);
            }
        }
        _ => letter_glyph(p, b),
    }
}

/// SA's bold serif `R` (resample).
const SERIF_R: [&str; 11] = [
    "#########",
    " ###   ###",
    " ###   ###",
    " ###   ###",
    " ###  ###",
    " #######",
    " ###  ###",
    " ###   ###",
    " ###   ###",
    " ###   ###",
    "#####  ####",
];

/// SA's bold serif `S` (shift).
const SERIF_S: [&str; 10] = [
    "  ###### #",
    " ###   ###",
    "###     ##",
    "####",
    " ######",
    "   ######",
    "      ####",
    "##     ###",
    "###   ###",
    "# ######",
];

/// The letter glyphs: `dT`, `x^n`, `Ph`, `IQ`, `R` and `S`.
fn letter_glyph(p: &mut Pen, b: Button) {
    match b {
        Button::Derivative => {
            p.bitmap(
                1,
                4,
                &[
                    "    ##", "    ##", " #####", "##  ##", "##  ##", "##  ##", " #####",
                ],
                NAVY,
            );
            p.bitmap(
                8,
                4,
                &[
                    "#######", "#######", "  ###", "  ###", "  ###", "  ###", "  ###",
                ],
                NAVY,
            );
        }
        Button::Power => {
            p.bitmap(
                1,
                7,
                &["##   ##", " ## ##", "  ###", "  ###", " ## ##", "##   ##"],
                NAVY,
            );
            p.bitmap(
                9,
                1,
                &["## ##", "######", "##  ##", "##  ##", "##  ##"],
                NAVY,
            );
        }
        Button::PhasePlane => {
            p.bitmap(
                0,
                1,
                &[
                    "######## ",
                    " ##   ###",
                    " ##    ##",
                    " ##   ###",
                    " ####### ",
                    " ##",
                    " ##",
                    " ##",
                    "####",
                ],
                NAVY,
            );
            p.bitmap(
                10,
                8,
                &["##", "##", "#####", "##  ##", "##  ##", "##  ##"],
                NAVY,
            );
        }
        Button::IqToReal => {
            p.bitmap(1, 1, &["####", " ##", " ##", " ##", " ##", "####"], NAVY);
            p.bitmap(
                6,
                5,
                &[
                    "  ####", " ##  ##", "##    ##", "##    ##", "##    ##", "##  # ##", " ##  ##",
                    "  ######", "       #",
                ],
                LIGHT,
            );
        }
        Button::Resample => p.bitmap(2, 2, &SERIF_R, NAVY),
        Button::Shift => p.bitmap(3, 2, &SERIF_S, NAVY),
        _ => {}
    }
}

/// A magnifier: brown handle, blue lens, and the red plus, minus or cross.
fn magnifier(p: &mut Pen, b: Button) {
    p.thick(9, 9, 14, 14, BROWN);
    p.line(9, 10, 13, 14, BROWN);
    if p.enabled {
        p.arc((6.5, 6.5), (0.0, 4.2), (0.0, 360.0), PAPER);
    }
    p.arc((6.5, 6.5), (4.2, 5.6), (0.0, 360.0), LIGHT);
    match b {
        Button::ZoomIn => {
            p.rect(3, 6, 7, 1, RED);
            p.rect(6, 3, 1, 7, RED);
        }
        Button::ZoomBack => p.rect(3, 6, 7, 1, RED),
        _ => {
            p.thick(1, 1, 13, 13, RED);
            p.thick(13, 1, 1, 13, RED);
        }
    }
}

/// A speaker with red sound waves.
fn speaker(p: &mut Pen) {
    p.bitmap(
        1,
        2,
        &[
            "       #", "      ##", "     ###", "    ####", "########", "########", "########",
            "########", "    ####", "     ###", "      ##", "       #",
        ],
        NAVY,
    );
    p.arc((8.5, 8.0), (2.4, 3.6), (-50.0, 50.0), RED);
    p.arc((8.5, 8.0), (5.0, 6.2), (-50.0, 50.0), RED);
}

/// A diode on column `cx`: stem, triangle pointing down and a bar.
fn diode(p: &mut Pen, cx: i32) {
    p.rect(cx, 1, 1, 15, NAVY);
    for (i, w) in [7, 5, 3, 1].into_iter().enumerate() {
        let y = 4 + i as i32;
        p.rect(cx - w / 2, y, w, 1, NAVY);
    }
    p.rect(cx - 3, 9, 7, 2, NAVY);
}

/// CCF: a big X with arrow heads.
fn crossed_arrows(p: &mut Pen) {
    p.thick(3, 3, 11, 11, NAVY);
    p.thick(11, 3, 3, 11, NAVY);
    p.bitmap(0, 0, &["####", "###", "##", "#"], NAVY);
    p.bitmap(11, 0, &[" ####", "  ###", "   ##", "    #"], NAVY);
    p.bitmap(0, 11, &["#", "##", "###", "####"], NAVY);
    p.bitmap(11, 11, &["    #", "   ##", "  ###", " ####"], NAVY);
}

/// OFDM: a small window with a 3x3 grid of coloured cells.
fn ofdm(p: &mut Pen) {
    p.rect(1, 1, 14, 3, NAVY);
    p.frame(1, 1, 14, 14, NAVY);
    let colours = [RED, GREEN, LIGHT, YELLOW, RED, GREEN, LIGHT, YELLOW, RED];
    for (i, c) in (0_i32..).zip(colours) {
        let (col, row) = (i % 3, i / 3);
        p.rect(3 + col * 4, 5 + row * 3, 3, 2, c);
    }
}

/// The shift-to-base meander (Greek key), mirrored for the inverted shift.
fn meander(p: &mut Pen, mirrored: bool) {
    let pts = [(1, 14), (1, 2), (13, 2), (13, 10), (6, 10), (6, 6), (10, 6)];
    let pts: Vec<(i32, i32)> = pts
        .iter()
        .map(|&(x, y)| if mirrored { (14 - x, y) } else { (x, y) })
        .collect();
    p.polyline(&pts, NAVY);
}

/// Two circular arrows (in-band inverse).
fn recycle(p: &mut Pen) {
    p.arc((8.0, 8.0), (4.4, 6.0), (200.0, 320.0), NAVY);
    p.poly(&[(11.0, 2.5), (15.5, 5.5), (11.0, 8.0)], NAVY);
    p.arc((8.0, 8.0), (4.4, 6.0), (20.0, 140.0), NAVY);
    p.poly(&[(5.0, 8.0), (0.5, 10.5), (5.0, 13.5)], NAVY);
}

/// A glyph of the popup menu.
pub(crate) fn menu_glyph(icon: MenuIcon) -> Glyph {
    let mut p = Pen::new(true);
    match icon {
        MenuIcon::SaveAs => {
            p.rect(1, 1, 12, 12, NAVY);
            p.rect(3, 1, 8, 4, PAPER);
            p.rect(4, 9, 6, 4, LIGHT);
            p.bitmap(9, 8, &["#", "##", "###", "####", "###", "##", "#"], RED);
        }
        MenuIcon::WaveForm => {
            box_icon(&mut p);
            p.rect(2, 8, 12, 1, LIGHT);
            sine(&mut p);
        }
        MenuIcon::Measure => {
            box_icon(&mut p);
            p.rect(5, 3, 1, 10, NAVY);
            p.rect(10, 3, 1, 10, NAVY);
            p.rect(2, 6, 12, 1, RED);
            p.rect(2, 10, 12, 1, RED);
        }
        MenuIcon::Adp => p.bitmap(
            4,
            2,
            &[
                "   ##", "   ##", "  ####", "  #  #", " ##  ##", " ######", "##    ##", "##    ##",
                "###  ###",
            ],
            NAVY,
        ),
        MenuIcon::Calculator => {
            p.rect(3, 1, 10, 14, BROWN);
            p.rect(4, 2, 8, 3, PAPER);
            for row in 0..3 {
                for col in 0..3 {
                    p.rect(4 + 3 * col, 6 + 3 * row, 2, 2, PAPER);
                }
            }
        }
        MenuIcon::Cancel => {
            p.rect(3, 1, 10, 14, YELLOW);
            p.frame(3, 1, 10, 14, BROWN);
            p.rect(5, 3, 5, 12, NAVY);
        }
    }
    p.g
}

/// A white box with a navy frame.
fn box_icon(p: &mut Pen) {
    p.rect(1, 2, 14, 12, PAPER);
    p.frame(1, 2, 14, 12, NAVY);
}

/// One period of a red sine across the box.
fn sine(p: &mut Pen) {
    let mut last: Option<(i32, i32)> = None;
    for i in 0..=12 {
        let x = 2 + i;
        let y = (8.5 - 4.0 * (2.0 * PI * i as f32 / 12.0).sin()).floor() as i32;
        if let Some((lx, ly)) = last {
            p.line(lx, ly, x, y, RED);
        }
        last = Some((x, y));
    }
}

#[cfg(test)]
mod tests {
    use sn_ui_core::sono_tools::BUTTONS;

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
        for b in BUTTONS {
            for enabled in [true, false] {
                let g = button_glyph(b, enabled);
                assert!(!g.ops.is_empty(), "{b:?} draws nothing");
                assert!(inside(&g), "{b:?} leaves its square");
                assert_eq!((g.w, g.h), (16, 16));
            }
        }
        for m in [
            MenuIcon::SaveAs,
            MenuIcon::WaveForm,
            MenuIcon::Measure,
            MenuIcon::Adp,
            MenuIcon::Calculator,
            MenuIcon::Cancel,
        ] {
            let g = menu_glyph(m);
            assert!(!g.ops.is_empty() && inside(&g), "{m:?}");
        }
    }

    #[test]
    fn disabled_glyphs_drop_their_paper() {
        let on = button_glyph(Button::Copy, true);
        let off = button_glyph(Button::Copy, false);
        assert!(off.ops.len() < on.ops.len());
        assert!(
            !off.ops
                .iter()
                .any(|op| matches!(op, Op::Rect(_, i) if *i == PAPER))
        );
    }
}
