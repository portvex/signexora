//! Windows Vista and 7 (Aero and Basic), drawn on the pixel grid.
//!
//! MDI children never get glass on Windows: their frames are the visual
//! style's opaque Basic frames, with or without Aero. Top-level windows (SA's
//! dialogs and message boxes) get the translucent glass frame under Aero and
//! the Basic frame otherwise. Vista's colours are a little darker and cooler
//! than 7's.

use gpui::{App, Hsla, Pixels, Point, Window};

use super::colors::{Rgb, hsla, mix};
use super::geom::Rect;
use super::marks::Dir;
use super::paint;
use super::skin::{CapButton, State, outline, ramp, round_border, round_rect};
use super::text;

/// A colour with transparency.
fn alpha(c: Rgb, a: f32) -> Hsla {
    let mut h = hsla(c);
    h.a = a;
    h
}

fn row(rows: &[Rgb], y: i32) -> Hsla {
    hsla(
        rows.get(usize::try_from(y).unwrap_or(0))
            .copied()
            .unwrap_or(0),
    )
}

/// The Basic frame colours: caption stops, the outer border and the side
/// colour, active and inactive.
struct Basic {
    caption: &'static [(f64, Rgb)],
    caption_inactive: &'static [(f64, Rgb)],
    border: Rgb,
    border_inactive: Rgb,
    side: Rgb,
    side_inactive: Rgb,
}

const BASIC_7: Basic = Basic {
    caption: &[
        (0.0, 0xDB_E8_F6),
        (0.35, 0xCB_DD_F2),
        (0.36, 0xBE_D3_EB),
        (1.0, 0xB9_D1_EA),
    ],
    caption_inactive: &[(0.0, 0xEB_F1_F9), (0.35, 0xE1_EA_F5), (1.0, 0xD7_E4_F2)],
    border: 0x5E_7D_A3,
    border_inactive: 0x96_AC_C6,
    side: 0xB9_D1_EA,
    side_inactive: 0xD7_E4_F2,
};

const BASIC_VISTA: Basic = Basic {
    caption: &[
        (0.0, 0xD0_DC_EA),
        (0.35, 0xBF_CD_DE),
        (0.36, 0xB0_C2_D8),
        (1.0, 0xA9_BC_D2),
    ],
    caption_inactive: &[(0.0, 0xE6_EC_F3), (1.0, 0xD3_DD_E8)],
    border: 0x4A_5F_7A,
    border_inactive: 0x8E_9E_B2,
    side: 0xA9_BC_D2,
    side_inactive: 0xD3_DD_E8,
};

fn basic(vista: bool) -> &'static Basic {
    if vista { &BASIC_VISTA } else { &BASIC_7 }
}

/// The glass tint of a top-level window (7's "Sky", Vista's darker default).
fn glass_tint(vista: bool, active: bool) -> (Rgb, f32) {
    match (vista, active) {
        (false, true) => (0x6F_9B_CF, 0.62),
        (false, false) => (0xA7_C0_DC, 0.45),
        (true, true) => (0x34_4C_6A, 0.72),
        (true, false) => (0x7A_8A_A0, 0.5),
    }
}

/// The frame and caption of a window `w` x `h`: glass for a top-level window
/// under Aero, the opaque Basic frame otherwise.
#[allow(clippy::too_many_arguments)]
pub(crate) fn frame(
    window: &mut Window,
    o: Point<Pixels>,
    (w, h): (i32, i32),
    frame: i32,
    caption: i32,
    active: bool,
    vista: bool,
    glass: bool,
) {
    let top = frame + caption;
    if glass {
        let (tint, a) = glass_tint(vista, active);
        let rows = ramp(
            &[(0.0, mix(tint, 0xFF_FF_FF, 0.35)), (0.5, tint), (1.0, tint)],
            top,
        );
        round_rect(
            window,
            o,
            Rect::new(0, 0, w, top),
            6,
            (true, false),
            None,
            &|y| {
                let mut c = row(&rows, y);
                c.a = a;
                c
            },
        );
        let side = alpha(tint, a);
        paint::fill(window, o, Rect::new(0, top, frame, h - top), side);
        paint::fill(window, o, Rect::new(w - frame, top, frame, h - top), side);
        paint::fill(
            window,
            o,
            Rect::new(frame, h - frame, w - 2 * frame, frame),
            side,
        );
        // The dark outer line and the white inner line of the glass edge.
        let edge = alpha(0x00_00_00, 0.55);
        round_border(window, o, Rect::new(0, 0, w, h), 6, (true, false), edge);
        outline(
            window,
            o,
            Rect::new(frame - 1, top - 1, w - 2 * frame + 2, h - top - frame + 2),
            alpha(0xFF_FF_FF, 0.5),
        );
        return;
    }
    let b = basic(vista);
    let rows = ramp(
        if active {
            b.caption
        } else {
            b.caption_inactive
        },
        top,
    );
    let border = hsla(if active { b.border } else { b.border_inactive });
    round_rect(
        window,
        o,
        Rect::new(0, 0, w, top),
        6,
        (true, false),
        Some(border),
        &|y| row(&rows, y),
    );
    let side = hsla(if active { b.side } else { b.side_inactive });
    paint::fill(window, o, Rect::new(1, top, frame - 1, h - top - 1), side);
    paint::fill(
        window,
        o,
        Rect::new(w - frame, top, frame - 1, h - top - 1),
        side,
    );
    paint::fill(window, o, Rect::new(1, h - frame, w - 2, frame - 1), side);
    paint::fill(window, o, Rect::new(0, top, 1, h - top), border);
    paint::fill(window, o, Rect::new(w - 1, top, 1, h - top), border);
    paint::fill(window, o, Rect::new(0, h - 1, w, 1), border);
    // The client edge: a thin dark line around the client area.
    let inner = Rect::new(frame - 1, top - 1, w - 2 * frame + 2, h - top - frame + 2);
    outline(window, o, inner, hsla(mix(b.border, 0xFF_FF_FF, 0.35)));
}

/// The caption title: black Segoe UI; on glass with a soft white glow.
#[allow(clippy::too_many_arguments)]
pub(crate) fn title(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    title: &str,
    active: bool,
    (vista, glass): (bool, bool),
) {
    if glass {
        for (dx, dy, a) in glow_offsets(vista) {
            let at = paint::at(o, x + dx, y + dy);
            let glow = alpha(0xFF_FF_FF, a);
            text::draw_in(text::Face::Caption, window, cx, at, title, false, glow);
        }
    }
    let fg = if active { 0x00_00_00 } else { 0x43_4E_54 };
    let at = paint::at(o, x, y);
    text::draw_in(text::Face::Caption, window, cx, at, title, false, hsla(fg));
}

/// Where the white glow behind a glass title is drawn, and how opaque: one
/// pixel around it under Windows 7's light glass, a strong halo two pixels
/// wide under Vista's dark glass, so the black title reads on it.
fn glow_offsets(vista: bool) -> Vec<(i32, i32, f32)> {
    let (reach, a): (i32, f32) = if vista { (2, 0.8) } else { (1, 0.35) };
    let mut out = Vec::new();
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            // The corners of the square would square the halo off.
            if (dx, dy) != (0, 0) && (dx.abs() + dy.abs() < 2 * reach || reach == 1) {
                out.push((dx, dy, a));
            }
        }
    }
    out
}

/// A caption button hanging from the window's top edge: blue-grey glossy
/// minimize/maximize, red close (wide), each with rounded bottom corners.
#[allow(clippy::too_many_arguments)]
pub(crate) fn caption_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    kind: CapButton,
    st: State,
    active: bool,
    glass: bool,
) {
    let close = kind == CapButton::Close;
    let stops: &[(f64, Rgb)] = match (close, st.down, st.hot, active) {
        (true, true, _, _) => &[(0.0, 0xB0_3A_25), (0.5, 0x90_1F_10), (1.0, 0xC0_4A_2E)],
        (true, false, true, _) => &[
            (0.0, 0xF6_B4_A6),
            (0.5, 0xE8_6A_52),
            (0.51, 0xD8_3A_1E),
            (1.0, 0xF0_9C_7C),
        ],
        (true, false, false, true) => &[
            (0.0, 0xE9_A9_9C),
            (0.5, 0xD3_6E_5A),
            (0.51, 0xC0_41_2D),
            (1.0, 0xD8_86_6E),
        ],
        (false, true, _, _) => &[(0.0, 0x8C_A8_CC), (0.5, 0x5B_7F_AE), (1.0, 0x7E_A3_CF)],
        (false, false, true, _) => &[
            (0.0, 0xEA_F6_FD),
            (0.5, 0xBE_E3_FA),
            (0.51, 0x8C_CE_F5),
            (1.0, 0xB6_E3_FB),
        ],
        _ => &[
            (0.0, 0xF2_F6_FA),
            (0.5, 0xDD_E7_F2),
            (0.51, 0xC9_D7_E8),
            (1.0, 0xD6_E2_EF),
        ],
    };
    let rows = ramp(stops, r.h);
    let a = if glass && !st.hot && !st.down && !close {
        0.55
    } else {
        1.0
    };
    let border = hsla(if active { 0x4A_5E_7E } else { 0x8A_9A_B2 });
    round_rect(window, o, r, 3, (false, true), Some(border), &|y| {
        let mut c = row(&rows, y);
        c.a = a;
        c
    });
    // White glyph with a dark outline, as the Aero glyphs.
    let g = hsla(0xFF_FF_FF);
    let dark = hsla(if close { 0x5A_14_08 } else { 0x23_35_52 });
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    let glyph = |window: &mut Window, rects: &[Rect]| {
        for q in rects {
            paint::fill(window, o, q.inset(-1), dark);
        }
        for q in rects {
            paint::fill(window, o, *q, g);
        }
    };
    match kind {
        CapButton::Minimize => glyph(window, &[Rect::new(cx - 4, cy + 2, 9, 2)]),
        CapButton::Maximize => glyph(
            window,
            &[
                Rect::new(cx - 5, cy - 4, 10, 2),
                Rect::new(cx - 5, cy - 4, 1, 8),
                Rect::new(cx + 4, cy - 4, 1, 8),
                Rect::new(cx - 5, cy + 3, 10, 1),
            ],
        ),
        CapButton::Restore => glyph(
            window,
            &[
                Rect::new(cx - 2, cy - 5, 7, 2),
                Rect::new(cx + 4, cy - 5, 1, 6),
                Rect::new(cx - 5, cy - 2, 7, 2),
                Rect::new(cx - 5, cy - 2, 1, 6),
                Rect::new(cx + 1, cy - 2, 1, 6),
                Rect::new(cx - 5, cy + 3, 7, 1),
            ],
        ),
        CapButton::Close => {
            let mut rs = Vec::new();
            for k in -3..=3 {
                rs.push(Rect::new(cx + k - 1, cy + k, 3, 1));
                rs.push(Rect::new(cx - k - 1, cy + k, 3, 1));
            }
            glyph(window, &rs);
        }
    }
}

/// An Aero push button: rounded grey border, a two-part grey gradient; light
/// blue when hot, deeper blue when pressed, a blue border when defaulted.
pub(crate) fn push_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State) {
    let (border, stops): (Rgb, &[(f64, Rgb)]) = match (st.enabled, st.down, st.hot, st.focus) {
        (false, ..) => (0xAD_B2_B5, &[(0.0, 0xF4_F4_F4), (1.0, 0xF4_F4_F4)]),
        (true, true, ..) => (
            0x2C_62_8B,
            &[
                (0.0, 0xE5_F4_FC),
                (0.5, 0xC4_E5_F6),
                (0.51, 0x98_D1_EF),
                (1.0, 0x68_B3_DB),
            ],
        ),
        (true, false, true, _) => (
            0x3C_7F_B1,
            &[
                (0.0, 0xEA_F6_FD),
                (0.5, 0xD9_F0_FC),
                (0.51, 0xBE_E6_FD),
                (1.0, 0xA7_D9_F5),
            ],
        ),
        (true, false, false, true) => (
            0x3C_7F_B1,
            &[
                (0.0, 0xF2_F2_F2),
                (0.5, 0xEB_EB_EB),
                (0.51, 0xDD_DD_DD),
                (1.0, 0xCF_CF_CF),
            ],
        ),
        (true, false, false, false) => (
            0x70_70_70,
            &[
                (0.0, 0xF2_F2_F2),
                (0.5, 0xEB_EB_EB),
                (0.51, 0xDD_DD_DD),
                (1.0, 0xCF_CF_CF),
            ],
        ),
    };
    let rows = ramp(stops, r.h - 2);
    round_rect(window, o, r, 3, (true, true), Some(hsla(border)), &|y| {
        row(&rows, y - 1)
    });
    if st.enabled && !st.down {
        // The white inner highlight.
        let inner = r.inset(1);
        outline(
            window,
            o,
            Rect::new(inner.x + 1, inner.y, inner.w - 2, inner.h),
            alpha(0xFF_FF_FF, 0.6),
        );
    }
}

/// An Aero check box (13x13): grey border, a soft inner square, a dark tick.
pub(crate) fn check(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    checked: bool,
    st: State,
) {
    let r = Rect::new(x, y, 13, 13);
    let (border, inner): (Rgb, &[(f64, Rgb)]) = match (st.enabled, st.down, st.hot) {
        (false, ..) => (0xBC_BC_BC, &[(0.0, 0xF4_F4_F4), (1.0, 0xF4_F4_F4)]),
        (true, true, _) => (0x2C_62_8B, &[(0.0, 0xB9_DF_F2), (1.0, 0xDB_F0_FA)]),
        (true, false, true) => (0x3C_7F_B1, &[(0.0, 0xDE_F9_FA), (1.0, 0xC2_E4_F6)]),
        (true, false, false) => (0x8E_8F_8F, &[(0.0, 0xCB_CF_D5), (1.0, 0xF6_F6_F6)]),
    };
    paint::fill(window, o, r, hsla(0xFF_FF_FF));
    outline(window, o, r, hsla(border));
    let rows = ramp(inner, 9);
    for (i, c) in (0_i32..).zip(&rows) {
        paint::fill(window, o, Rect::new(x + 2, y + 2 + i, 9, 1), hsla(*c));
    }
    if checked {
        let ink = hsla(if st.enabled { 0x1E_39_5B } else { 0xA0_A0_A0 });
        let map = [
            "......#", ".....##", "#...##.", "##.##..", ".###...", "..#....",
        ];
        for (dy, line) in (0_i32..).zip(map) {
            for (dx, ch) in (0_i32..).zip(line.chars()) {
                if ch == '#' {
                    paint::fill(window, o, Rect::new(x + 3 + dx, y + 3 + dy, 1, 1), ink);
                }
            }
        }
    }
}

/// An Aero radio button (13x13): a grey ring and a glossy blue dot.
pub(crate) fn radio(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    selected: bool,
    st: State,
) {
    let (border, fill_top, fill_bottom) = match (st.enabled, st.down, st.hot) {
        (false, ..) => (0xBC_BC_BC, 0xF4_F4_F4, 0xF4_F4_F4),
        (true, true, _) => (0x2C_62_8B, 0xB9_DF_F2, 0xDB_F0_FA),
        (true, false, true) => (0x3C_7F_B1, 0xDE_F9_FA, 0xC2_E4_F6),
        (true, false, false) => (0x8E_8F_8F, 0xD5_D8_DC, 0xFA_FA_FA),
    };
    let rows = ramp(&[(0.0, fill_top), (1.0, fill_bottom)], 13);
    let ring = [
        "....BBBBB....",
        "..BBaFFFaBB..",
        ".BaFFFFFFFaB.",
        ".BFFFFFFFFFB.",
        "BaFFFFFFFFFaB",
        "BFFFFFFFFFFFB",
        "BFFFFFFFFFFFB",
        "BFFFFFFFFFFFB",
        "BaFFFFFFFFFaB",
        ".BFFFFFFFFFB.",
        ".BaFFFFFFFaB.",
        "..BBaFFFaBB..",
        "....BBBBB....",
    ];
    for (dy, line) in (0_i32..).zip(ring) {
        let f = rows
            .get(usize::try_from(dy).unwrap_or(0))
            .copied()
            .unwrap_or(0xFF_FF_FF);
        for (dx, ch) in (0_i32..).zip(line.chars()) {
            let c = match ch {
                'B' => border,
                'a' => mix(border, f, 0.5),
                'F' => f,
                _ => continue,
            };
            paint::fill(window, o, Rect::new(x + dx, y + dy, 1, 1), hsla(c));
        }
    }
    if selected {
        let (light, dark) = if st.enabled {
            (0x8F_D3_F6, 0x1A_4C_8E)
        } else {
            (0xD0_D0_D0, 0xA0_A0_A0)
        };
        let dot = [".aDa.", "aLDDa", "DDDDD", "aDDDa", ".aDa."];
        for (dy, line) in (0_i32..).zip(dot) {
            for (dx, ch) in (0_i32..).zip(line.chars()) {
                let c = match ch {
                    'D' => dark,
                    'L' => light,
                    'a' => mix(dark, 0xFF_FF_FF, 0.45),
                    _ => continue,
                };
                paint::fill(window, o, Rect::new(x + 4 + dx, y + 4 + dy, 1, 1), hsla(c));
            }
        }
    }
}

/// A small solid triangle (the arrows of Aero scroll bars and combo boxes).
pub(crate) fn triangle(window: &mut Window, o: Point<Pixels>, r: Rect, dir: Dir, ink: Hsla) {
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    for k in 0..4 {
        let q = match dir {
            Dir::Down => Rect::new(cx - 3 + k, cy - 1 + k, 7 - 2 * k, 1),
            Dir::Up => Rect::new(cx - 3 + k, cy + 1 - k, 7 - 2 * k, 1),
            Dir::Right => Rect::new(cx - 1 + k, cy - 3 + k, 1, 7 - 2 * k),
            Dir::Left => Rect::new(cx + 1 - k, cy - 3 + k, 1, 7 - 2 * k),
        };
        if !q.is_empty() {
            paint::fill(window, o, q, ink);
        }
    }
}

/// An Aero scroll bar arrow (flat until hot) or thumb (always drawn).
pub(crate) fn scroll_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    arrow: Option<Dir>,
    st: State,
    vertical: bool,
) {
    let thumb = arrow.is_none();
    if thumb || st.hot || st.down {
        let (border, a, b) = match (st.down, st.hot || st.down) {
            (true, _) => (0x2C_62_8B, 0xB0_DE_F5, 0x7E_C5_EB),
            (false, true) => (0x3C_7F_B1, 0xE3_F4_FC, 0xB4_DF_F7),
            (false, false) => (0x97_97_97, 0xF3_F3_F3, 0xE1_E1_E1),
        };
        // The thumb's gradient runs across it.
        let n = if vertical { r.w } else { r.h };
        let cols = ramp(&[(0.0, a), (1.0, b)], n);
        if vertical {
            round_rect(window, o, r, 2, (true, true), Some(hsla(border)), &|_| {
                hsla(a)
            });
            for (i, c) in (0_i32..).zip(&cols) {
                if i == 0 || i == n - 1 {
                    continue;
                }
                paint::fill(window, o, Rect::new(r.x + i, r.y + 2, 1, r.h - 4), hsla(*c));
            }
        } else {
            round_rect(window, o, r, 2, (true, true), Some(hsla(border)), &|y| {
                row(&cols, y)
            });
        }
    }
    if let Some(dir) = arrow {
        let ink = hsla(match (st.enabled, st.hot || st.down) {
            (false, _) => 0xBF_BF_BF,
            (true, true) => 0x1C_4B_7C,
            (true, false) => 0x60_60_60,
        });
        triangle(window, o, r, dir, ink);
    }
}

/// The grip of an Aero thumb: three short lines.
pub(crate) fn thumb_grip(window: &mut Window, o: Point<Pixels>, r: Rect, vertical: bool) {
    let long = if vertical { r.h } else { r.w };
    if long < 16 {
        return;
    }
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    for i in 0..4 {
        let (a, b) = if vertical {
            (
                Rect::new(cx - 3, cy - 4 + 2 * i, 6, 1),
                Rect::new(cx - 3, cy - 3 + 2 * i, 6, 1),
            )
        } else {
            (
                Rect::new(cx - 4 + 2 * i, cy - 3, 1, 6),
                Rect::new(cx - 3 + 2 * i, cy - 3, 1, 6),
            )
        };
        paint::fill(window, o, a, hsla(0x9D_9D_9D));
        paint::fill(window, o, b, hsla(0xFF_FF_FF));
    }
}

/// The track of an Aero scroll bar: light grey with darker edges.
pub(crate) fn scroll_track(window: &mut Window, o: Point<Pixels>, r: Rect, vertical: bool) {
    let n = if vertical { r.w } else { r.h };
    let cols = ramp(
        &[
            (0.0, 0xE3_E3_E3),
            (0.3, 0xEE_EE_EE),
            (0.7, 0xEE_EE_EE),
            (1.0, 0xE3_E3_E3),
        ],
        n,
    );
    for (i, c) in (0_i32..).zip(&cols) {
        let q = if vertical {
            Rect::new(r.x + i, r.y, 1, r.h)
        } else {
            Rect::new(r.x, r.y + i, r.w, 1)
        };
        paint::fill(window, o, q, hsla(*c));
    }
}

/// An Aero progress bar: continuous glossy green in a rounded grey trough.
pub(crate) fn progress(window: &mut Window, o: Point<Pixels>, r: Rect, span: Option<(i32, i32)>) {
    let trough = ramp(&[(0.0, 0xF3_F3_F3), (1.0, 0xE0_E0_E0)], r.h);
    round_rect(
        window,
        o,
        r,
        2,
        (true, true),
        Some(hsla(0xB2_B2_B2)),
        &|y| row(&trough, y),
    );
    let inner = r.inset(1);
    let Some((x, w)) = span else {
        return;
    };
    let x = x.clamp(0, inner.w);
    let w = w.min(inner.w - x);
    if w <= 0 {
        return;
    }
    let bar = ramp(
        &[
            (0.0, 0x7E_E6_8B),
            (0.45, 0x37_D2_4D),
            (0.46, 0x06_B0_25),
            (1.0, 0x0C_C2_30),
        ],
        inner.h,
    );
    for (y, c) in (0_i32..).zip(&bar) {
        paint::fill(
            window,
            o,
            Rect::new(inner.x + x, inner.y + y, w, 1),
            hsla(*c),
        );
    }
}

/// An Aero track bar: a thin grey channel and a glossy rectangular thumb.
pub(crate) fn trackbar(
    window: &mut Window,
    o: Point<Pixels>,
    channel: Rect,
    thumb: Rect,
    st: State,
) {
    let ch = Rect::new(channel.x, channel.y + 1, channel.w, 3);
    paint::fill(window, o, ch, hsla(0xE7_EA_EA));
    outline(window, o, ch, hsla(0xD6_D6_D6));
    let (border, a, b) = match (st.enabled, st.hot) {
        (false, _) => (0xBC_BC_BC, 0xF4_F4_F4, 0xF4_F4_F4),
        (true, true) => (0x3C_7F_B1, 0xEA_F6_FD, 0xB2_DE_F7),
        (true, false) => (0x70_70_70, 0xF3_F3_F3, 0xD8_D8_D8),
    };
    let rows = ramp(&[(0.0, a), (1.0, b)], thumb.h);
    round_rect(
        window,
        o,
        thumb,
        2,
        (true, true),
        Some(hsla(border)),
        &|y| row(&rows, y),
    );
}

/// An Aero group box frame.
pub(crate) fn group(window: &mut Window, o: Point<Pixels>, r: Rect) {
    round_border(window, o, r, 3, (true, true), hsla(0xD5_DF_E5));
}

/// The title colour of an Aero group box.
pub(crate) const GROUP_TITLE: Rgb = 0x1E_39_5B;

/// A hot, pressed or checked Aero toolbar button.
pub(crate) fn tool_button(window: &mut Window, o: Point<Pixels>, r: Rect, down: bool) {
    let (border, a, b) = if down {
        (0x6A_8B_B8, 0xC4_E0_F7, 0xA6_CF_F1)
    } else {
        (0x7D_A2_CE, 0xF2_F8_FD, 0xD8_EA_FB)
    };
    let rows = ramp(&[(0.0, a), (1.0, b)], r.h);
    round_rect(window, o, r, 2, (true, true), Some(hsla(border)), &|y| {
        row(&rows, y)
    });
}

/// The rounded light-blue highlight of a hot menu item or menu bar title.
pub(crate) fn menu_highlight(window: &mut Window, o: Point<Pixels>, r: Rect) {
    let rows = ramp(&[(0.0, 0xF2_F8_FD), (1.0, 0xDC_EB_FC)], r.h);
    round_rect(
        window,
        o,
        r,
        2,
        (true, true),
        Some(hsla(0xAE_CF_F7)),
        &|y| row(&rows, y),
    );
}

/// The background of an Aero popup menu.
pub(crate) const MENU_BG: Rgb = 0xF0_F0_F0;

/// Aero's grey for disabled text.
pub(crate) const DISABLED_TEXT: Rgb = 0x83_83_83;

/// The background of an Aero popup menu: light grey, the icon gutter and its
/// two-line edge at `gutter`, the grey border.
pub(crate) fn menu_popup(window: &mut Window, o: Point<Pixels>, r: Rect, gutter: i32) {
    paint::fill(window, o, r, hsla(MENU_BG));
    paint::fill(
        window,
        o,
        Rect::new(r.x + 1, r.y + 1, gutter, r.h - 2),
        hsla(0xF1_F1_F1),
    );
    paint::fill(
        window,
        o,
        Rect::new(r.x + gutter, r.y + 1, 1, r.h - 2),
        hsla(0xE2_E3_E3),
    );
    paint::fill(
        window,
        o,
        Rect::new(r.x + gutter + 1, r.y + 1, 1, r.h - 2),
        hsla(0xFF_FF_FF),
    );
    outline(window, o, r, hsla(0x97_97_97));
}

/// A separator line of an Aero popup menu, right of the gutter.
pub(crate) fn menu_separator(
    window: &mut Window,
    o: Point<Pixels>,
    row: Rect,
    gutter: i32,
    y: i32,
) {
    let x = row.x + gutter + 2;
    paint::fill(
        window,
        o,
        Rect::new(x, y, row.w - gutter - 3, 1),
        hsla(0xE0_E0_E0),
    );
    paint::fill(
        window,
        o,
        Rect::new(x, y + 1, row.w - gutter - 3, 1),
        hsla(0xFF_FF_FF),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win95::colors::contrast;

    #[test]
    fn glass_titles_read_on_their_glow() {
        for vista in [true, false] {
            let (tint, a) = glass_tint(vista, true);
            // The glass over the grey MDI workspace, then the glow over it.
            let glass = mix(0xAB_AB_AB, tint, f64::from(a));
            let offsets = glow_offsets(vista);
            let glow = offsets.first().map_or(0.0, |o| o.2);
            let behind = mix(glass, 0xFF_FF_FF, f64::from(glow));
            assert!(
                contrast(0x00_00_00, behind) >= 4.5,
                "vista={vista}: {behind:06X}"
            );
            assert!(!offsets.iter().any(|&(x, y, _)| (x, y) == (0, 0)));
        }
        assert_eq!(glow_offsets(false).len(), 8);
        assert_eq!(glow_offsets(true).len(), 20);
    }
}
