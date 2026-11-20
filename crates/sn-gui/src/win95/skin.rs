//! The visual styles of the themed Windows versions, drawn on the pixel grid:
//! Windows XP's Luna (Blue, Olive Green, Silver) here, Vista and 7 (Aero and
//! Basic) in [`super::skin_aero`], 8 and 10 in [`super::skin_flat`]. Each
//! widget asks [`skin`] which style to draw and calls the matching dispatcher
//! here; the classic styles (95, 98 and high contrast) keep the 3D drawing of
//! the kit, and the dispatchers are called only when the skin is not
//! [`Skin::Classic`].
//!
//! The colours are those of the visual style's bitmaps, read as rows (top
//! first): a gradient is a list of stops interpolated over the height.

use gpui::{App, Hsla, Pixels, Point, Window};
use sn_ui_core::winstyle::{WinStyle, XpColor};

use super::colors::{Rgb, hsla, mix};
use super::geom::{Rect, pixel_line};
use super::marks::Dir;
use super::paint;
use super::style;
use super::text;

/// The style a widget is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Skin {
    /// Classic 3D controls (95, 98 and high contrast).
    Classic,
    /// Windows XP Luna with its colour scheme.
    Xp(XpColor),
    /// Windows Vista or 7; `glass` with Aero (top-level windows only).
    Aero { vista: bool, glass: bool },
    /// Windows 8 or 10.
    Flat(flat::Flat),
    /// Windows 11.
    Fluent(fluent::Fluent),
    /// A macOS look.
    Mac(mac::Mac),
    /// A Linux look.
    Linux(linux::Lin),
}

/// The skin of the active style.
pub(crate) fn skin() -> Skin {
    let a = style::active();
    match a.style {
        WinStyle::Xp => Skin::Xp(a.look.xp),
        WinStyle::Vista | WinStyle::Win7 => Skin::Aero {
            vista: a.style == WinStyle::Vista,
            glass: a.look.aero,
        },
        WinStyle::Win8 | WinStyle::Win10 => Skin::Flat(flat::Flat {
            win10: a.style == WinStyle::Win10,
            dark: a.style == WinStyle::Win10 && a.look.dark,
            accent: a.look.drawn_accent(),
        }),
        WinStyle::Win11 => Skin::Fluent(fluent::Fluent {
            dark: a.look.dark,
            accent: a.look.drawn_accent(),
        }),
        WinStyle::MacAqua | WinStyle::MacYosemite | WinStyle::MacModern => {
            mac::Mac::of(a.look).map_or(Skin::Classic, Skin::Mac)
        }
        WinStyle::LinuxAdwaita
        | WinStyle::LinuxBreeze
        | WinStyle::LinuxClearlooks
        | WinStyle::LinuxMotif => linux::Lin::of(a.look).map_or(Skin::Classic, Skin::Linux),
        WinStyle::Win95 | WinStyle::Win98 => Skin::Classic,
    }
}

/// The one-pixel border of fields (edit fields, list boxes, combo boxes) in
/// the themed styles; `None` in the classic ones (a sunken 3D edge).
pub(crate) fn field_border() -> Option<Rgb> {
    match skin() {
        Skin::Xp(_) => Some(XP_FIELD),
        Skin::Aero { .. } => Some(0xAB_AD_B3),
        Skin::Flat(f) => Some(flat::field_border(f)),
        Skin::Fluent(_) | Skin::Mac(_) | Skin::Linux(_) | Skin::Classic => None,
    }
}

/// The state of a control being drawn.
// The four independent states a visual style draws a control in (hot,
// pressed, defaulted, disabled), as Windows' part states.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct State {
    /// Under the mouse.
    pub(crate) hot: bool,
    /// Held down.
    pub(crate) down: bool,
    /// Focused, or the default button.
    pub(crate) focus: bool,
    pub(crate) enabled: bool,
}

/// A one-pixel rectangle outline.
pub(crate) fn outline(window: &mut Window, o: Point<Pixels>, r: Rect, c: Hsla) {
    paint::fill(window, o, Rect::new(r.x, r.y, r.w, 1), c);
    paint::fill(window, o, Rect::new(r.x, r.bottom() - 1, r.w, 1), c);
    paint::fill(window, o, Rect::new(r.x, r.y, 1, r.h), c);
    paint::fill(window, o, Rect::new(r.right() - 1, r.y, 1, r.h), c);
}

/// One pixel.
fn dot(window: &mut Window, o: Point<Pixels>, x: i32, y: i32, c: Hsla) {
    paint::fill(window, o, Rect::new(x, y, 1, 1), c);
}

/// The colours of `n` rows from gradient stops `(position 0..=1, colour)`.
pub(crate) fn ramp(stops: &[(f64, Rgb)], n: i32) -> Vec<Rgb> {
    let n = n.max(1);
    (0..n)
        .map(|i| {
            let t = if n == 1 {
                0.0
            } else {
                f64::from(i) / f64::from(n - 1)
            };
            let mut prev = stops.first().copied().unwrap_or((0.0, 0));
            for &(p, c) in stops {
                if t <= p {
                    let span = p - prev.0;
                    let u = if span > 0.0 { (t - prev.0) / span } else { 1.0 };
                    return mix(prev.1, c, u.clamp(0.0, 1.0));
                }
                prev = (p, c);
            }
            prev.1
        })
        .collect()
}

/// How far each of the first rows of a rounded corner of radius `r` is cut
/// in from the side.
fn corner(r: i32) -> Vec<i32> {
    let rf = f64::from(r);
    (0..r)
        .map(|i| {
            let d = rf - f64::from(i);
            (rf - (rf * rf - d * d).max(0.0).sqrt()).round() as i32
        })
        .collect()
}

/// How far row `y` of a rectangle `h` high is cut in from each side, given
/// the cuts of a corner and which ends are rounded.
fn row_cut(cut: &[i32], h: i32, (top, bottom): (bool, bool), y: i32) -> i32 {
    let from_top = usize::try_from(y).ok();
    let from_bottom = usize::try_from(h - 1 - y).ok();
    let t = from_top.filter(|_| top).and_then(|i| cut.get(i)).copied();
    let b = from_bottom
        .filter(|_| bottom)
        .and_then(|i| cut.get(i))
        .copied();
    t.or(b).unwrap_or(0)
}

/// The border pixels of row `y` of a rounded rectangle, and the inner span
/// of the row (the part inside the border), as rectangles.
fn border_row(cut: &[i32], r: Rect, ends: (bool, bool), y: i32) -> (Vec<Rect>, Rect) {
    let off = |y: i32| row_cut(cut, r.h, ends, y);
    let k = off(y);
    if y == 0 || y == r.h - 1 {
        let row = Rect::new(r.x + k, r.y + y, r.w - 2 * k, 1);
        return (vec![row], Rect::new(r.x, r.y + y, 0, 0));
    }
    // The border runs from this row's cut to the next row's (toward the
    // nearer edge), so a corner is a closed staircase.
    let toward = if y < r.h / 2 { off(y - 1) } else { off(y + 1) };
    let span = (toward - k).max(1);
    (
        vec![
            Rect::new(r.x + k, r.y + y, span, 1),
            Rect::new(r.right() - k - span, r.y + y, span, 1),
        ],
        Rect::new(r.x + k + span, r.y + y, r.w - 2 * (k + span), 1),
    )
}

/// A rectangle with rounded corners (top and/or bottom), an optional
/// one-pixel border and rows filled by `fill(row)`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn round_rect(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    radius: i32,
    ends: (bool, bool),
    border: Option<Hsla>,
    fill: &dyn Fn(i32) -> Hsla,
) {
    if r.is_empty() {
        return;
    }
    let cut = corner(radius.min(r.h / 2).min(r.w / 2).max(0));
    for y in 0..r.h {
        let Some(bc) = border else {
            let k = row_cut(&cut, r.h, ends, y);
            paint::fill(
                window,
                o,
                Rect::new(r.x + k, r.y + y, r.w - 2 * k, 1),
                fill(y),
            );
            continue;
        };
        let (edges, inner) = border_row(&cut, r, ends, y);
        for e in edges {
            paint::fill(window, o, e, bc);
        }
        if !inner.is_empty() {
            paint::fill(window, o, inner, fill(y));
        }
    }
}

/// The rectangles of the one-pixel border of a rounded rectangle alone: the
/// rows of the rounded ends one by one, each straight side as one rectangle.
pub(crate) fn round_border_rects(r: Rect, radius: i32, ends: (bool, bool)) -> Vec<Rect> {
    if r.is_empty() {
        return Vec::new();
    }
    let cut = corner(radius.min(r.h / 2).min(r.w / 2).max(0));
    let n = i32::try_from(cut.len()).unwrap_or(0);
    // A row next to a rounded end still steps in toward it.
    let a = (if ends.0 { n + 1 } else { 1 }).min(r.h);
    let b = (r.h - if ends.1 { n + 1 } else { 1 }).max(a);
    let mut out = Vec::new();
    for y in (0..a).chain(b..r.h) {
        out.extend(border_row(&cut, r, ends, y).0);
    }
    if b > a {
        out.push(Rect::new(r.x, r.y + a, 1, b - a));
        out.push(Rect::new(r.right() - 1, r.y + a, 1, b - a));
    }
    out.retain(|p| !p.is_empty());
    out
}

/// The one-pixel border of a rounded rectangle, leaving its inside as it is.
pub(crate) fn round_border(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    radius: i32,
    ends: (bool, bool),
    c: Hsla,
) {
    for p in round_border_rects(r, radius, ends) {
        paint::fill(window, o, p, c);
    }
}

// ---------------------------------------------------------------------------
// Windows XP Luna
// ---------------------------------------------------------------------------

/// The colours of a Luna colour scheme.
struct Luna {
    /// Caption rows of an active window (top frame edge included), stops.
    caption: &'static [(f64, Rgb)],
    /// Caption rows of an inactive window.
    caption_inactive: &'static [(f64, Rgb)],
    /// Frame columns from the outside in, active and inactive.
    frame: [Rgb; 4],
    frame_inactive: [Rgb; 4],
    /// Caption text and its shadow (none on Silver).
    title: Rgb,
    title_inactive: Rgb,
    title_shadow: Option<Rgb>,
    /// Minimize/maximize button: normal, hot, pressed, inactive stops.
    button: [&'static [(f64, Rgb)]; 4],
    /// The glyph colour on minimize/maximize.
    button_glyph: Rgb,
    /// Scroll bar and drop-down buttons: fill stops, border, arrow, hot fill,
    /// pressed fill.
    scroll: &'static [(f64, Rgb)],
    scroll_border: Rgb,
    scroll_arrow: Rgb,
    scroll_hot: &'static [(f64, Rgb)],
    scroll_down: &'static [(f64, Rgb)],
    /// Scroll bar track.
    track: Rgb,
}

const BLUE: Luna = Luna {
    caption: &[
        (0.0, 0x00_58_EE),
        (0.04, 0x3A_93_FF),
        (0.12, 0x28_8E_FF),
        (0.2, 0x12_7D_FF),
        (0.32, 0x03_69_FC),
        (0.45, 0x00_62_F8),
        (0.7, 0x00_5B_F0),
        (0.86, 0x00_55_E9),
        (0.95, 0x00_48_DB),
        (1.0, 0x00_43_CF),
    ],
    caption_inactive: &[
        (0.0, 0x7B_9A_E1),
        (0.04, 0xA7_C0_F4),
        (0.2, 0x9B_B5_F0),
        (0.45, 0x91_AC_EE),
        (0.86, 0x8C_A7_EC),
        (1.0, 0x7F_99_E1),
    ],
    frame: [0x08_31_D9, 0x16_6A_EE, 0x08_55_DD, 0x08_41_D3],
    frame_inactive: [0x65_82_D0, 0x99_B1_EA, 0x8B_A4_E6, 0x7A_96_DF],
    title: 0xFF_FF_FF,
    title_inactive: 0xD8_E4_F8,
    title_shadow: Some(0x0A_18_83),
    button: [
        &[(0.0, 0x5A_9C_FF), (0.5, 0x2A_6D_F0), (1.0, 0x1D_5A_E0)],
        &[(0.0, 0x7A_B4_FF), (0.5, 0x4A_8B_FF), (1.0, 0x35_70_F0)],
        &[(0.0, 0x1F_57_D4), (0.5, 0x16_48_C4), (1.0, 0x10_3E_B8)],
        &[(0.0, 0xA8_BE_F2), (1.0, 0x8D_A4_E6)],
    ],
    button_glyph: 0xFF_FF_FF,
    scroll: &[(0.0, 0xD4_E1_FC), (0.5, 0xC1_D3_FB), (1.0, 0xB4_C8_F6)],
    scroll_border: 0x9D_B5_EE,
    scroll_arrow: 0x4D_61_85,
    scroll_hot: &[(0.0, 0xE4_EC_FE), (0.5, 0xD2_DE_FD), (1.0, 0xC5_D5_FC)],
    scroll_down: &[(0.0, 0x8E_A9_F2), (1.0, 0x6E_8E_F1)],
    track: 0xF4_F3_EE,
};

const OLIVE: Luna = Luna {
    caption: &[
        (0.0, 0x7D_8F_55),
        (0.04, 0xC4_D2_A3),
        (0.15, 0xB3_C5_8C),
        (0.4, 0xA4_B8_7C),
        (0.75, 0x97_AC_6E),
        (1.0, 0x7F_94_59),
    ],
    caption_inactive: &[
        (0.0, 0xA8_B4_8C),
        (0.04, 0xDD_E5_CB),
        (0.4, 0xCF_D9_B8),
        (1.0, 0xBA_C8_A0),
    ],
    frame: [0x6E_7E_46, 0xA4_B8_7C, 0x93_A8_6A, 0x7D_91_52],
    frame_inactive: [0xA0_AC_86, 0xD3_DC_BE, 0xC6_D1_AE, 0xB6_C3_9C],
    title: 0xFF_FF_FF,
    title_inactive: 0xFF_FF_FF,
    title_shadow: Some(0x5A_6B_30),
    button: [
        &[(0.0, 0xC9_D8_A4), (0.5, 0x9F_B4_75), (1.0, 0x86_9B_5B)],
        &[(0.0, 0xDC_E8_BB), (0.5, 0xB4_C8_8A), (1.0, 0x9C_B0_70)],
        &[(0.0, 0x7D_91_52), (1.0, 0x6A_7D_42)],
        &[(0.0, 0xD6_DF_C2), (1.0, 0xC2_CE_A8)],
    ],
    button_glyph: 0xFF_FF_FF,
    scroll: &[(0.0, 0xEC_F1_DE), (0.5, 0xDB_E5_C0), (1.0, 0xC8_D6_A6)],
    scroll_border: 0xB3_C3_8D,
    scroll_arrow: 0x5F_7C_2F,
    scroll_hot: &[(0.0, 0xF4_F8_EA), (1.0, 0xDC_E7_C3)],
    scroll_down: &[(0.0, 0xB7_C9_8E), (1.0, 0xA2_B8_74)],
    track: 0xF4_F3_EE,
};

const SILVER: Luna = Luna {
    caption: &[
        (0.0, 0xA5_A5_BA),
        (0.04, 0xFD_FD_FE),
        (0.15, 0xED_ED_F2),
        (0.45, 0xDC_DC_E6),
        (0.8, 0xC7_C8_D8),
        (1.0, 0xA9_AA_C0),
    ],
    caption_inactive: &[
        (0.0, 0xC4_C4_D2),
        (0.04, 0xFF_FF_FF),
        (0.45, 0xF2_F2_F6),
        (1.0, 0xE3_E2_EA),
    ],
    frame: [0x8D_8E_A6, 0xF2_F2_F6, 0xD7_D8_E2, 0xB2_B3_C6],
    frame_inactive: [0xB9_BA_C9, 0xFA_FA_FC, 0xEC_EC_F1, 0xD9_D9_E3],
    title: 0x00_00_00,
    title_inactive: 0x80_80_80,
    title_shadow: None,
    button: [
        &[(0.0, 0xFF_FF_FF), (0.5, 0xE6_E6_EE), (1.0, 0xC8_C9_D6)],
        &[(0.0, 0xFF_FF_FF), (0.5, 0xF2_F2_F7), (1.0, 0xDA_DB_E6)],
        &[(0.0, 0xB9_BA_C9), (1.0, 0xA3_A4_B9)],
        &[(0.0, 0xFA_FA_FC), (1.0, 0xE8_E8_EF)],
    ],
    button_glyph: 0x3C_3C_5A,
    scroll: &[(0.0, 0xFC_FC_FD), (0.5, 0xE9_E9_F0), (1.0, 0xD7_D8_E2)],
    scroll_border: 0xB7_B8_C8,
    scroll_arrow: 0x5A_5A_75,
    scroll_hot: &[(0.0, 0xFF_FF_FF), (1.0, 0xE9_E9_F0)],
    scroll_down: &[(0.0, 0xC8_C9_D6), (1.0, 0xB4_B5_C6)],
    track: 0xF3_F3_F5,
};

fn luna(c: XpColor) -> &'static Luna {
    match c {
        XpColor::Blue => &BLUE,
        XpColor::Olive => &OLIVE,
        XpColor::Silver => &SILVER,
    }
}

/// The red close button: normal, hot, pressed (the same in every scheme).
const CLOSE: [&[(f64, Rgb)]; 3] = [
    &[(0.0, 0xEC_A8_92), (0.5, 0xD7_6F_4F), (1.0, 0xC2_49_1F)],
    &[(0.0, 0xF5_C3_B3), (0.5, 0xE8_88_6B), (1.0, 0xD5_5E_36)],
    &[(0.0, 0xB6_40_1C), (1.0, 0xA2_36_0F)],
];

/// Push buttons, check boxes and radio buttons.
const XP_BORDER: Rgb = 0x00_3C_74;
const XP_BORDER_DISABLED: Rgb = 0xC9_C7_BA;
const XP_CHECK_BORDER: Rgb = 0x1C_51_80;
const XP_GREEN: Rgb = 0x21_A1_21;
const XP_GREEN_LIGHT: Rgb = 0x55_D5_51;
/// XP's grey for disabled text.
pub(crate) const XP_DISABLED_TEXT: Rgb = 0xA1_9F_8B;
/// The 1-pixel border of edit fields, list boxes and combo boxes.
pub(crate) const XP_FIELD: Rgb = 0x7F_9D_B9;

/// The text colour of a themed control (XP greys disabled text flat).
pub(crate) fn text_color(enabled: bool) -> Rgb {
    match (enabled, skin()) {
        (_, Skin::Flat(f)) => flat::text_color(f, enabled),
        (_, Skin::Fluent(f)) => fluent::text_color(f, enabled),
        (_, Skin::Mac(m)) => mac::text_color(m, enabled),
        (_, Skin::Linux(l)) => linux::text_color(l, enabled),
        (true, _) => 0x00_00_00,
        (false, Skin::Aero { .. }) => aero::DISABLED_TEXT,
        (false, _) => XP_DISABLED_TEXT,
    }
}

/// The frame and caption background of a window `w` x `h` with frames
/// `frame` wide and a caption `caption` high: the rounded caption gradient
/// with its top frame, and the side and bottom frames.
#[allow(clippy::too_many_arguments)]
pub(crate) fn xp_frame(
    window: &mut Window,
    o: Point<Pixels>,
    c: XpColor,
    w: i32,
    h: i32,
    frame: i32,
    caption: i32,
    active: bool,
) {
    let l = luna(c);
    let top = frame + caption;
    let rows = ramp(
        if active {
            l.caption
        } else {
            l.caption_inactive
        },
        top,
    );
    round_rect(
        window,
        o,
        Rect::new(0, 0, w, top),
        7,
        (true, false),
        None,
        &|y| {
            hsla(
                rows.get(usize::try_from(y).unwrap_or(0))
                    .copied()
                    .unwrap_or(0),
            )
        },
    );
    let cols = if active { l.frame } else { l.frame_inactive };
    for (i, col) in (0_i32..).zip(cols) {
        if i >= frame {
            break;
        }
        let ink = hsla(col);
        paint::fill(window, o, Rect::new(i, top, 1, h - top - i), ink);
        paint::fill(window, o, Rect::new(w - 1 - i, top, 1, h - top - i), ink);
        paint::fill(window, o, Rect::new(i, h - 1 - i, w - 2 * i, 1), ink);
    }
}

/// The caption title with its shadow, in the caption font.
#[allow(clippy::too_many_arguments)]
pub(crate) fn xp_title(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    c: XpColor,
    (x, y): (i32, i32),
    title: &str,
    active: bool,
) {
    let l = luna(c);
    if let Some(sh) = l.title_shadow.filter(|_| active) {
        let at = paint::at(o, x + 1, y + 1);
        text::draw_in(text::Face::Caption, window, cx, at, title, true, hsla(sh));
    }
    let fg = if active { l.title } else { l.title_inactive };
    let at = paint::at(o, x, y);
    text::draw_in(text::Face::Caption, window, cx, at, title, true, hsla(fg));
}

/// What a caption button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CapButton {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// A Luna caption button (21x21): rounded, a white edge, its gradient and a
/// white glyph.
pub(crate) fn xp_caption_button(
    window: &mut Window,
    o: Point<Pixels>,
    c: XpColor,
    r: Rect,
    kind: CapButton,
    st: State,
    active: bool,
) {
    let l = luna(c);
    let stops = match (kind, active, st.down, st.hot) {
        (_, false, _, false) => l.button[3],
        (CapButton::Close, _, true, _) => CLOSE[2],
        (CapButton::Close, _, _, true) => CLOSE[1],
        (CapButton::Close, _, _, _) => CLOSE[0],
        (_, _, true, _) => l.button[2],
        (_, _, _, true) => l.button[1],
        _ => l.button[0],
    };
    let rows = ramp(stops, r.h);
    let edge = if active {
        0xFF_FF_FF
    } else {
        mix(0xFF_FF_FF, l.frame_inactive[3], 0.4)
    };
    round_rect(window, o, r, 3, (true, true), Some(hsla(edge)), &|y| {
        hsla(
            rows.get(usize::try_from(y).unwrap_or(0))
                .copied()
                .unwrap_or(0),
        )
    });
    let glyph = match (kind, active) {
        (_, false) => mix(0xFF_FF_FF, l.frame_inactive[3], 0.3),
        (CapButton::Close, true) => 0xFF_FF_FF,
        _ => l.button_glyph,
    };
    let g = hsla(glyph);
    let (x, y) = (r.x, r.y);
    let d = i32::from(st.down);
    match kind {
        CapButton::Minimize => paint::fill(window, o, Rect::new(x + 5 + d, y + 13 + d, 6, 3), g),
        CapButton::Maximize => {
            let b = Rect::new(x + 5 + d, y + 5 + d, 11, 11);
            paint::fill(window, o, Rect::new(b.x, b.y, b.w, 3), g);
            paint::fill(window, o, Rect::new(b.x, b.y, 1, b.h), g);
            paint::fill(window, o, Rect::new(b.right() - 1, b.y, 1, b.h), g);
            paint::fill(window, o, Rect::new(b.x, b.bottom() - 1, b.w, 1), g);
        }
        CapButton::Restore => {
            for (bx, by) in [(8, 4), (5, 8)] {
                let b = Rect::new(x + bx + d, y + by + d, 8, 8);
                paint::fill(window, o, Rect::new(b.x, b.y, b.w, 2), g);
                paint::fill(window, o, Rect::new(b.x, b.y, 1, b.h), g);
                paint::fill(window, o, Rect::new(b.right() - 1, b.y, 1, b.h), g);
                paint::fill(window, o, Rect::new(b.x, b.bottom() - 1, b.w, 1), g);
            }
        }
        CapButton::Close => {
            for k in 0..2 {
                for (px_, py) in pixel_line(x + 6 + k + d, y + 6 + d, x + 13 + k + d, y + 13 + d) {
                    dot(window, o, px_, py, g);
                }
                for (px_, py) in pixel_line(x + 13 + k + d, y + 6 + d, x + 6 + k + d, y + 13 + d) {
                    dot(window, o, px_, py, g);
                }
            }
        }
    }
}

/// A Luna push button: rounded dark-blue border, a light gradient, an orange
/// inner glow when hot, a blue one when focused or default.
pub(crate) fn xp_push_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State) {
    let border = if st.enabled {
        XP_BORDER
    } else {
        XP_BORDER_DISABLED
    };
    let face: &[(f64, Rgb)] = match (st.enabled, st.down) {
        (false, _) => &[(0.0, 0xF5_F4_EA), (1.0, 0xF5_F4_EA)],
        (true, true) => &[(0.0, 0xD9_D8_D1), (0.3, 0xE2_E1_DA), (1.0, 0xEF_EE_E8)],
        (true, false) => &[
            (0.0, 0xFF_FF_FF),
            (0.6, 0xF6_F6_F2),
            (0.85, 0xEC_EB_E6),
            (1.0, 0xD6_D0_C5),
        ],
    };
    let rows = ramp(face, r.h);
    round_rect(window, o, r, 3, (true, true), Some(hsla(border)), &|y| {
        hsla(
            rows.get(usize::try_from(y).unwrap_or(0))
                .copied()
                .unwrap_or(0),
        )
    });
    let glow: Option<(Rgb, Rgb, Rgb, Rgb)> = match (st.enabled, st.down, st.hot, st.focus) {
        (true, false, true, _) => Some((0xFF_F0_CF, 0xFD_D8_89, 0xF8_B3_30, 0xE5_97_00)),
        (true, false, false, true) => Some((0xCE_E7_FF, 0xBC_D4_F6, 0x89_AD_E4, 0x69_82_EE)),
        _ => None,
    };
    if let Some((t1, t2, b1, b2)) = glow {
        let inner = r.inset(1);
        let side = ramp(&[(0.0, t2), (1.0, b1)], inner.h);
        paint::fill(
            window,
            o,
            Rect::new(inner.x + 1, inner.y, inner.w - 2, 1),
            hsla(t1),
        );
        paint::fill(
            window,
            o,
            Rect::new(inner.x + 1, inner.bottom() - 1, inner.w - 2, 1),
            hsla(b2),
        );
        for (y, c) in (0_i32..).zip(&side) {
            if y == 0 || y == inner.h - 1 {
                continue;
            }
            for x in [inner.x, inner.x + 1, inner.right() - 2, inner.right() - 1] {
                dot(window, o, x, inner.y + y, hsla(*c));
            }
        }
    }
}

/// A Luna check box (13x13) at `(x, y)`.
pub(crate) fn xp_check(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    checked: bool,
    st: State,
) {
    let r = Rect::new(x, y, 13, 13);
    let border = if st.enabled {
        XP_CHECK_BORDER
    } else {
        XP_BORDER_DISABLED
    };
    outline(window, o, r, hsla(border));
    let inner = r.inset(1);
    let stops: &[(f64, Rgb)] = match (st.enabled, st.down, st.hot) {
        (false, _, _) => &[(0.0, 0xFF_FF_FF), (1.0, 0xFF_FF_FF)],
        (true, true, _) => &[(0.0, 0xB0_B0_A7), (1.0, 0xE3_E3_DB)],
        (true, false, true) => &[
            (0.0, 0xFF_F0_CF),
            (0.25, 0xF8_D2_7C),
            (0.75, 0xF8_D2_7C),
            (1.0, 0xF8_B3_30),
        ],
        (true, false, false) => &[(0.0, 0xDC_DC_D7), (1.0, 0xFF_FF_FF)],
    };
    let rows = ramp(stops, inner.h);
    for (i, c) in (0_i32..).zip(&rows) {
        paint::fill(
            window,
            o,
            Rect::new(inner.x, inner.y + i, inner.w, 1),
            hsla(*c),
        );
    }
    if st.enabled && st.hot && !st.down {
        // The hot frame keeps a white-grey middle.
        let mid = ramp(&[(0.0, 0xDC_DC_D7), (1.0, 0xFF_FF_FF)], inner.h - 4);
        for (i, c) in (0_i32..).zip(&mid) {
            paint::fill(
                window,
                o,
                Rect::new(inner.x + 2, inner.y + 2 + i, inner.w - 4, 1),
                hsla(*c),
            );
        }
    }
    if checked {
        let ink = hsla(if st.enabled {
            XP_GREEN
        } else {
            XP_BORDER_DISABLED
        });
        let rows = [
            "......#", ".....##", "#...###", "##.###.", "#####..", ".###...", "..#....",
        ];
        for (dy, row) in (0_i32..).zip(rows) {
            for (dx, ch) in (0_i32..).zip(row.chars()) {
                if ch == '#' {
                    dot(window, o, x + 3 + dx, y + 3 + dy, ink);
                }
            }
        }
    }
}

/// A Luna radio button (13x13) at `(x, y)`.
pub(crate) fn xp_radio(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    selected: bool,
    st: State,
) {
    let border = if st.enabled {
        XP_CHECK_BORDER
    } else {
        XP_BORDER_DISABLED
    };
    let stops: &[(f64, Rgb)] = match (st.enabled, st.down, st.hot) {
        (false, _, _) => &[(0.0, 0xFF_FF_FF), (1.0, 0xFF_FF_FF)],
        (true, true, _) => &[(0.0, 0xB0_B0_A7), (1.0, 0xE3_E3_DB)],
        (true, false, true) => &[(0.0, 0xFF_F0_CF), (1.0, 0xF8_B3_30)],
        (true, false, false) => &[(0.0, 0xDC_DC_D7), (1.0, 0xFF_FF_FF)],
    };
    let rows = ramp(stops, 13);
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
    let b = hsla(border);
    for (dy, row) in (0_i32..).zip(ring) {
        let fill_c = rows
            .get(usize::try_from(dy).unwrap_or(0))
            .copied()
            .unwrap_or(0xFF_FF_FF);
        for (dx, ch) in (0_i32..).zip(row.chars()) {
            let c = match ch {
                'B' => b,
                'a' => hsla(mix(border, fill_c, 0.5)),
                'F' => hsla(fill_c),
                _ => continue,
            };
            dot(window, o, x + dx, y + dy, c);
        }
    }
    if selected {
        let (light, dark) = if st.enabled {
            (XP_GREEN_LIGHT, XP_GREEN)
        } else {
            (XP_BORDER_DISABLED, XP_BORDER_DISABLED)
        };
        let dotmap = [".aDa.", "aLLLa", "DLLLD", "aLLLa", ".aDa."];
        for (dy, row) in (0_i32..).zip(dotmap) {
            for (dx, ch) in (0_i32..).zip(row.chars()) {
                let c = match ch {
                    'D' => dark,
                    'L' => mix(
                        light,
                        dark,
                        f64::from((dx - 2).abs() + (dy - 2).abs()) / 3.0,
                    ),
                    'a' => mix(dark, 0xFF_FF_FF, 0.4),
                    _ => continue,
                };
                dot(window, o, x + 4 + dx, y + 4 + dy, hsla(c));
            }
        }
    }
}

/// A Luna scroll bar arrow or drop-down button (also the thumb without an
/// arrow): a rounded blue button with a bold chevron.
pub(crate) fn xp_scroll_button(
    window: &mut Window,
    o: Point<Pixels>,
    c: XpColor,
    r: Rect,
    arrow: Option<Dir>,
    st: State,
) {
    let l = luna(c);
    let stops = if !st.enabled {
        &[(0.0, 0xF4_F3_EE), (1.0, 0xEE_ED_E5)][..]
    } else if st.down {
        l.scroll_down
    } else if st.hot {
        l.scroll_hot
    } else {
        l.scroll
    };
    let rows = ramp(stops, r.h);
    let border = if st.enabled {
        l.scroll_border
    } else {
        0xE8_E6_DD
    };
    round_rect(window, o, r, 2, (true, true), Some(hsla(border)), &|y| {
        hsla(
            rows.get(usize::try_from(y).unwrap_or(0))
                .copied()
                .unwrap_or(0),
        )
    });
    let Some(dir) = arrow else {
        return;
    };
    let ink = hsla(match (st.enabled, st.down) {
        (false, _) => 0xC9_C7_BA,
        (true, true) => 0xFF_FF_FF,
        (true, false) => l.scroll_arrow,
    });
    // A chevron 7 wide and 4 high, two pixels thick.
    let (cx_, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    for k in 0..4 {
        let (a, b) = match dir {
            Dir::Down => ((cx_ - 3 + k, cy - 2 + k), (cx_ + 3 - k, cy - 2 + k)),
            Dir::Up => ((cx_ - 3 + k, cy + 1 - k), (cx_ + 3 - k, cy + 1 - k)),
            Dir::Right => ((cx_ - 2 + k, cy - 3 + k), (cx_ - 2 + k, cy + 3 - k)),
            Dir::Left => ((cx_ + 1 - k, cy - 3 + k), (cx_ + 1 - k, cy + 3 - k)),
        };
        // Only the two ends of each step: a hollow chevron 2 px thick.
        for (px_, py) in [a, b] {
            dot(window, o, px_, py, ink);
        }
        let (a2, b2) = match dir {
            Dir::Down | Dir::Up => ((a.0 + 1, a.1), (b.0 - 1, b.1)),
            Dir::Left | Dir::Right => ((a.0, a.1 + 1), (b.0, b.1 - 1)),
        };
        if k < 3 {
            dot(window, o, a2.0, a2.1, ink);
            dot(window, o, b2.0, b2.1, ink);
        }
    }
}

/// The grip lines in the middle of a Luna scroll thumb.
pub(crate) fn xp_thumb_grip(
    window: &mut Window,
    o: Point<Pixels>,
    c: XpColor,
    r: Rect,
    vertical: bool,
) {
    let long = if vertical { r.h } else { r.w };
    if long < 16 {
        return;
    }
    let l = luna(c);
    let (light, dark) = (
        hsla(mix(
            l.scroll.first().map_or(0xFF_FF_FF, |c| c.1),
            0xFF_FF_FF,
            0.6,
        )),
        hsla(l.scroll_border),
    );
    let (cx_, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    for i in 0..4 {
        let line = |len: i32, k: i32| {
            if vertical {
                Rect::new(cx_ - len / 2, cy - 4 + 2 * i + k, len, 1)
            } else {
                Rect::new(cx_ - 4 + 2 * i + k, cy - len / 2, 1, len)
            }
        };
        paint::fill(window, o, line(7, 0), light);
        paint::fill(window, o, line(7, 1), dark);
    }
}

/// The track of a Luna scroll bar.
pub(crate) fn xp_scroll_track(
    window: &mut Window,
    o: Point<Pixels>,
    c: XpColor,
    r: Rect,
    vertical: bool,
) {
    let base = luna(c).track;
    paint::fill(window, o, r, hsla(base));
    let edge = hsla(mix(base, 0xFF_FF_FF, 0.7));
    if vertical {
        paint::fill(window, o, Rect::new(r.x, r.y, 1, r.h), edge);
        paint::fill(window, o, Rect::new(r.right() - 1, r.y, 1, r.h), edge);
    } else {
        paint::fill(window, o, Rect::new(r.x, r.y, r.w, 1), edge);
        paint::fill(window, o, Rect::new(r.x, r.bottom() - 1, r.w, 1), edge);
    }
}

/// A Luna progress bar: a rounded grey border, white inside, green chunks.
pub(crate) fn xp_progress(window: &mut Window, o: Point<Pixels>, r: Rect, chunks: &[(i32, i32)]) {
    round_rect(
        window,
        o,
        r,
        2,
        (true, true),
        Some(hsla(0x68_68_68)),
        &|_| hsla(0xFF_FF_FF),
    );
    let inner = r.inset(2);
    let rows = ramp(
        &[
            (0.0, 0xA6_E9_A0),
            (0.3, 0x45_D8_4D),
            (0.5, 0x2F_D2_3F),
            (0.7, 0x45_D8_4D),
            (1.0, 0xA6_E9_A0),
        ],
        inner.h,
    );
    for &(x, w) in chunks {
        for (y, c) in (0_i32..).zip(&rows) {
            paint::fill(
                window,
                o,
                Rect::new(inner.x + x, inner.y + y, w, 1),
                hsla(*c),
            );
        }
    }
}

/// A Luna track bar: the channel and the thumb (green-edged, orange when hot).
pub(crate) fn xp_trackbar(
    window: &mut Window,
    o: Point<Pixels>,
    channel: Rect,
    thumb: Rect,
    st: State,
) {
    round_rect(
        window,
        o,
        channel,
        1,
        (true, true),
        Some(hsla(0x9D_9C_99)),
        &|_| hsla(0xF4_F3_EE),
    );
    let border = if st.enabled {
        0x76_97_B8
    } else {
        XP_BORDER_DISABLED
    };
    let rows = ramp(&[(0.0, 0xFF_FF_FF), (1.0, 0xE4_E4_DD)], thumb.h);
    round_rect(
        window,
        o,
        thumb,
        2,
        (true, true),
        Some(hsla(border)),
        &|y| {
            hsla(
                rows.get(usize::try_from(y).unwrap_or(0))
                    .copied()
                    .unwrap_or(0),
            )
        },
    );
    let accent = match (st.enabled, st.hot || st.down) {
        (false, _) => XP_BORDER_DISABLED,
        (true, true) => 0xF8_B3_30,
        (true, false) => XP_GREEN,
    };
    let a = hsla(accent);
    paint::fill(
        window,
        o,
        Rect::new(thumb.x + 2, thumb.y + 1, thumb.w - 4, 2),
        a,
    );
    paint::fill(
        window,
        o,
        Rect::new(thumb.x + 2, thumb.bottom() - 3, thumb.w - 4, 2),
        a,
    );
}

/// A Luna group box frame: a rounded light line. Its title is blue.
pub(crate) fn xp_group(window: &mut Window, o: Point<Pixels>, r: Rect) {
    round_border(window, o, r, 3, (true, true), hsla(0xD0_D0_BF));
}

/// The title colour of a Luna group box.
pub(crate) const XP_GROUP_TITLE: Rgb = 0x00_46_D5;

/// A Luna toolbar button that is hot, pressed or checked.
pub(crate) fn xp_tool_button(window: &mut Window, o: Point<Pixels>, r: Rect, down: bool) {
    let (border, stops): (Rgb, &[(f64, Rgb)]) = if down {
        (0x9D_9D_92, &[(0.0, 0xD8_D7_CF), (1.0, 0xEB_EA_E3)])
    } else {
        (0xCE_CE_C3, &[(0.0, 0xFF_FF_FF), (1.0, 0xEC_EB_E3)])
    };
    let rows = ramp(stops, r.h);
    round_rect(window, o, r, 2, (true, true), Some(hsla(border)), &|y| {
        hsla(
            rows.get(usize::try_from(y).unwrap_or(0))
                .copied()
                .unwrap_or(0),
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramps_hit_their_stops() {
        let r = ramp(&[(0.0, 0x00_00_00), (1.0, 0xFF_FF_FF)], 3);
        assert_eq!(r.first(), Some(&0x00_00_00));
        assert_eq!(r.last(), Some(&0xFF_FF_FF));
        assert_eq!(ramp(&[(0.0, 0x12_34_56)], 4), vec![0x12_34_56; 4]);
    }

    /// The pixels of `rects`, counting overlaps.
    fn pixels(rects: &[Rect]) -> std::collections::HashMap<(i32, i32), u32> {
        let mut px = std::collections::HashMap::new();
        for r in rects {
            for y in r.y..r.bottom() {
                for x in r.x..r.right() {
                    *px.entry((x, y)).or_insert(0) += 1;
                }
            }
        }
        px
    }

    #[test]
    fn the_border_alone_matches_the_border_of_a_filled_rectangle() {
        for (r, radius, ends) in [
            (Rect::new(0, 7, 191, 120), 3, (true, true)),
            (Rect::new(2, 3, 40, 30), 6, (true, false)),
            (Rect::new(0, 0, 9, 5), 3, (true, true)),
            (Rect::new(0, 0, 12, 2), 3, (false, true)),
        ] {
            let cut = corner(radius.min(r.h / 2).min(r.w / 2).max(0));
            let rows: Vec<Rect> = (0..r.h)
                .flat_map(|y| border_row(&cut, r, ends, y).0)
                .collect();
            let fast = round_border_rects(r, radius, ends);
            assert_eq!(pixels(&fast), pixels(&rows), "{r:?}");
            assert!(fast.len() < 4 * radius as usize + 8, "{}", fast.len());
        }
    }

    #[test]
    fn corners_cut_less_toward_the_side() {
        let c = corner(7);
        assert_eq!(c.first(), Some(&7));
        assert!(c.windows(2).all(|w| w[0] >= w[1]), "{c:?}");
        assert_eq!(corner(0), Vec::<i32>::new());
    }
}

// ---------------------------------------------------------------------------
// Dispatch: the painter of the active skin. The widgets call these only when
// `skin()` is not `Skin::Classic`.
// ---------------------------------------------------------------------------

use super::progress::{Progress, blocks, fill_span};
use super::skin_aero as aero;
use super::skin_flat as flat;
use super::skin_fluent as fluent;
use super::skin_linux as linux;
use super::skin_mac as mac;

/// The frame and caption background of a window; `top_level` windows (SA's
/// dialogs) get glass under Aero.
#[allow(clippy::too_many_arguments)]
pub(crate) fn frame(
    window: &mut Window,
    o: Point<Pixels>,
    (w, h): (i32, i32),
    frame: i32,
    caption: i32,
    active: bool,
    top_level: bool,
) {
    match skin() {
        Skin::Xp(c) => xp_frame(window, o, c, w, h, frame, caption, active),
        Skin::Aero { vista, glass } => {
            aero::frame(
                window,
                o,
                (w, h),
                frame,
                caption,
                active,
                vista,
                glass && top_level,
            );
        }
        Skin::Flat(f) => flat::frame(window, o, (w, h), frame, caption, active, f),
        Skin::Fluent(f) => {
            fluent::frame(window, o, (w, h), frame, caption, active, top_level, f);
        }
        Skin::Mac(m) => mac::frame(window, o, (w, h), frame, caption, active, m),
        Skin::Linux(l) => linux::frame(window, o, (w, h), frame, caption, active, l),
        Skin::Classic => {}
    }
}

/// The caption title.
#[allow(clippy::too_many_arguments)]
pub(crate) fn title(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    (at, title_w): ((i32, i32), i32),
    title: &str,
    active: bool,
    top_level: bool,
) {
    match skin() {
        Skin::Xp(c) => xp_title(window, cx, o, c, at, title, active),
        Skin::Aero { vista, glass } => {
            aero::title(
                window,
                cx,
                o,
                at,
                title,
                active,
                (vista, glass && top_level),
            );
        }
        Skin::Flat(f) => flat::title(window, cx, o, at, title_w, title, active, f),
        Skin::Fluent(f) => fluent::title(window, cx, o, at, title, active, f),
        // The macOS title is drawn centred by the caption painter.
        Skin::Mac(_) | Skin::Linux(_) | Skin::Classic => {}
    }
}

/// A caption button.
#[allow(clippy::too_many_arguments)]
pub(crate) fn caption_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    kind: CapButton,
    st: State,
    active: bool,
    top_level: bool,
) {
    match skin() {
        Skin::Xp(c) => xp_caption_button(window, o, c, r, kind, st, active),
        Skin::Aero { glass, .. } => {
            aero::caption_button(window, o, r, kind, st, active, glass && top_level);
        }
        Skin::Flat(f) => flat::caption_button(window, o, r, kind, st, active, f),
        Skin::Fluent(f) => fluent::caption_button(window, o, r, kind, st, active, f),
        Skin::Mac(m) => mac::caption_button(window, o, r, kind, st, active, m),
        Skin::Linux(l) => linux::caption_button(window, o, r, kind, st, active, l),
        Skin::Classic => {}
    }
}

/// A push button.
pub(crate) fn push_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State) {
    match skin() {
        Skin::Aero { .. } => aero::push_button(window, o, r, st),
        Skin::Flat(f) => flat::push_button(window, o, r, st, f),
        Skin::Fluent(f) => fluent::push_button(window, o, r, st, f),
        Skin::Mac(m) => mac::push_button(window, o, r, st, m),
        Skin::Linux(l) => linux::push_button(window, o, r, st, l),
        _ => xp_push_button(window, o, r, st),
    }
}

/// A check box mark at `at`.
pub(crate) fn check(
    window: &mut Window,
    o: Point<Pixels>,
    at: (i32, i32),
    checked: bool,
    st: State,
) {
    match skin() {
        Skin::Aero { .. } => aero::check(window, o, at, checked, st),
        Skin::Flat(f) => flat::check(window, o, at, checked, st, f),
        Skin::Fluent(f) => fluent::check(window, o, at, checked, st, f),
        Skin::Mac(m) => mac::check(window, o, at, checked, st, m),
        Skin::Linux(l) => linux::check(window, o, at, checked, st, l),
        _ => xp_check(window, o, at, checked, st),
    }
}

/// A radio button mark at `at`.
pub(crate) fn radio(
    window: &mut Window,
    o: Point<Pixels>,
    at: (i32, i32),
    selected: bool,
    st: State,
) {
    match skin() {
        Skin::Aero { .. } => aero::radio(window, o, at, selected, st),
        Skin::Flat(f) => flat::radio(window, o, at, selected, st, f),
        Skin::Fluent(f) => fluent::radio(window, o, at, selected, st, f),
        Skin::Mac(m) => mac::radio(window, o, at, selected, st, m),
        Skin::Linux(l) => linux::radio(window, o, at, selected, st, l),
        _ => xp_radio(window, o, at, selected, st),
    }
}

/// A scroll bar arrow, a scroll thumb (`arrow` none) or a drop-down button.
pub(crate) fn scroll_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    arrow: Option<Dir>,
    st: State,
    vertical: bool,
) {
    match skin() {
        Skin::Xp(c) => xp_scroll_button(window, o, c, r, arrow, st),
        Skin::Aero { .. } => aero::scroll_button(window, o, r, arrow, st, vertical),
        Skin::Flat(f) => flat::scroll_button(window, o, r, arrow, st, vertical, f),
        Skin::Fluent(f) => fluent::scroll_button(window, o, r, arrow, st, vertical, f),
        Skin::Mac(m) => mac::scroll_button(window, o, r, arrow, st, vertical, m),
        Skin::Linux(l) => linux::scroll_button(window, o, r, arrow, st, vertical, l),
        Skin::Classic => {}
    }
}

/// The drop-down button of a combo box.
pub(crate) fn drop_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State) {
    match skin() {
        Skin::Aero { .. } => {
            if st.hot || st.down {
                aero::scroll_button(window, o, r, None, st, false);
            }
            let ink = hsla(if st.enabled { 0x00_00_00 } else { 0xBF_BF_BF });
            aero::triangle(window, o, r, Dir::Down, ink);
        }
        Skin::Flat(f) => flat::drop_button(window, o, r, st, f),
        Skin::Fluent(f) => fluent::drop_button(window, o, r, st, f),
        Skin::Mac(m) => mac::drop_button(window, o, r, st, m),
        Skin::Linux(l) => linux::drop_button(window, o, r, st, l),
        _ => scroll_button(window, o, r, Some(Dir::Down), st, true),
    }
}

/// The grip lines of a scroll thumb.
pub(crate) fn thumb_grip(window: &mut Window, o: Point<Pixels>, r: Rect, vertical: bool) {
    match skin() {
        Skin::Xp(c) => xp_thumb_grip(window, o, c, r, vertical),
        Skin::Aero { .. } => aero::thumb_grip(window, o, r, vertical),
        Skin::Flat(_) | Skin::Fluent(_) | Skin::Mac(_) | Skin::Linux(_) | Skin::Classic => {}
    }
}

/// A scroll bar track.
pub(crate) fn scroll_track(window: &mut Window, o: Point<Pixels>, r: Rect, vertical: bool) {
    match skin() {
        Skin::Xp(c) => xp_scroll_track(window, o, c, r, vertical),
        Skin::Aero { .. } => aero::scroll_track(window, o, r, vertical),
        Skin::Flat(f) => flat::scroll_track(window, o, r, f),
        Skin::Fluent(f) => fluent::scroll_track(window, o, r, f),
        Skin::Mac(m) => mac::scroll_track(window, o, r, vertical, m),
        Skin::Linux(l) => linux::scroll_track(window, o, r, vertical, l),
        Skin::Classic => {}
    }
}

/// A progress bar showing `p` (XP draws blocks; later styles fill
/// continuously, see [`super::progress::fill_span`]).
pub(crate) fn progress(window: &mut Window, o: Point<Pixels>, r: Rect, p: Progress) {
    match skin() {
        Skin::Aero { .. } => {
            aero::progress(window, o, r, fill_span(p, r.inset(1).w));
        }
        Skin::Flat(f) => flat::progress(window, o, r, fill_span(p, r.inset(1).w), f),
        Skin::Linux(l) => {
            linux::progress(window, o, r, fill_span(p, linux::progress_inner_w(r, l)), l);
        }
        Skin::Mac(m) => mac::progress(window, o, r, fill_span(p, r.w - 2), m),
        Skin::Fluent(f) => fluent::progress(window, o, r, fill_span(p, r.w - 2), f),
        _ => xp_progress(window, o, r, &blocks(p, r.inset(2).w)),
    }
}

/// A track bar.
pub(crate) fn trackbar(
    window: &mut Window,
    o: Point<Pixels>,
    channel: Rect,
    thumb: Rect,
    st: State,
) {
    match skin() {
        Skin::Aero { .. } => aero::trackbar(window, o, channel, thumb, st),
        Skin::Flat(f) => flat::trackbar(window, o, channel, thumb, st, f),
        Skin::Fluent(f) => fluent::trackbar(window, o, channel, thumb, st, f),
        Skin::Mac(m) => mac::trackbar(window, o, channel, thumb, st, m),
        Skin::Linux(l) => linux::trackbar(window, o, channel, thumb, st, l),
        _ => xp_trackbar(window, o, channel, thumb, st),
    }
}

/// A group box frame (the border only; the inside is left as it is).
pub(crate) fn group(window: &mut Window, o: Point<Pixels>, r: Rect) {
    match skin() {
        Skin::Aero { .. } => aero::group(window, o, r),
        Skin::Flat(f) => flat::group(window, o, r, f),
        Skin::Fluent(f) => fluent::group(window, o, r, f),
        Skin::Mac(m) => mac::group(window, o, r, m),
        Skin::Linux(l) => linux::group(window, o, r, l),
        _ => xp_group(window, o, r),
    }
}

/// The title colour of a group box.
pub(crate) fn group_title() -> Rgb {
    match skin() {
        Skin::Aero { .. } => aero::GROUP_TITLE,
        Skin::Flat(f) => flat::text_color(f, true),
        Skin::Fluent(f) => fluent::text_color(f, true),
        Skin::Mac(m) => mac::text_color(m, true),
        Skin::Linux(l) => linux::text_color(l, true),
        _ => XP_GROUP_TITLE,
    }
}

/// A hot, pressed or checked toolbar button.
pub(crate) fn tool_button(window: &mut Window, o: Point<Pixels>, r: Rect, down: bool) {
    match skin() {
        Skin::Aero { .. } => aero::tool_button(window, o, r, down),
        Skin::Flat(f) => flat::tool_button(window, o, r, down, f),
        Skin::Fluent(f) => fluent::tool_button(window, o, r, down, f),
        Skin::Mac(m) => mac::tool_button(window, o, r, down, m),
        Skin::Linux(l) => linux::tool_button(window, o, r, down, l),
        _ => xp_tool_button(window, o, r, down),
    }
}

/// How menus are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuLook {
    /// 3D popups, the selection colour (95, 98).
    Classic,
    /// Flat popups with a one-pixel border, the selection colour (XP).
    Flat,
    /// Light popups with an icon gutter and a rounded light-blue highlight;
    /// the text stays black (Vista, 7).
    Aero,
    /// Light (or dark) popups with a square highlight (8, 10).
    Modern(flat::Flat),
    /// Rounded popups with a rounded, inset highlight (11).
    Fluent(fluent::Fluent),
    /// macOS menus: no mnemonic underlines, the accent highlight with white
    /// text.
    Mac(mac::Mac),
    /// The Linux desktops' menus.
    Linux(linux::Lin),
}

/// How menus are drawn in the active style.
pub(crate) fn menu_look() -> MenuLook {
    match skin() {
        Skin::Classic => MenuLook::Classic,
        Skin::Xp(_) => MenuLook::Flat,
        Skin::Linux(l) => MenuLook::Linux(l),
        Skin::Mac(m) => MenuLook::Mac(m),
        Skin::Fluent(f) => MenuLook::Fluent(f),
        Skin::Flat(f) => MenuLook::Modern(f),
        Skin::Aero { .. } => MenuLook::Aero,
    }
}

/// Draw the field border `r` in the active style if it is one with its own
/// field shape (Windows 11's rounded fields); returns whether it did.
pub(crate) fn field(window: &mut Window, o: Point<Pixels>, r: Rect) -> bool {
    match skin() {
        Skin::Fluent(f) => fluent::field(window, o, r, f),
        Skin::Mac(m) => mac::field(window, o, r, m),
        Skin::Linux(l) => return linux::field(window, o, r, l),
        _ => return false,
    }
    true
}

/// The Linux look being drawn, if one is.
pub(crate) fn linux_look() -> Option<linux::Lin> {
    match skin() {
        Skin::Linux(l) => Some(l),
        _ => None,
    }
}

/// The macOS look being drawn, if one is.
pub(crate) fn mac_look() -> Option<mac::Mac> {
    match skin() {
        Skin::Mac(m) => Some(m),
        _ => None,
    }
}

/// The caption colour of a push button, if the style has its own (Windows
/// 11's default button is accent-filled with white text).
pub(crate) fn button_text(st: State) -> Option<Rgb> {
    match skin() {
        Skin::Fluent(f) => Some(fluent::button_text(f, st)),
        Skin::Mac(m) => Some(mac::button_text(m, st)),
        Skin::Linux(l) => Some(linux::button_text(l, st)),
        _ => None,
    }
}
