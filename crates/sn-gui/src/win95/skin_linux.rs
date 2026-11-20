//! The Linux looks, drawn on the pixel grid: GNOME's Adwaita (with Ubuntu's
//! Yaru as a variant), KDE Plasma's Breeze, GNOME 2 with Clearlooks, and
//! Motif with the CDE desktop. Adwaita and Breeze have light and dark modes
//! and accent colours; Clearlooks and Motif have one look each.
//!
//! Motif windows have no close button: the window-menu button at the left of
//! the title bar opens the window menu (Close is there; a double click on it
//! closes), as in mwm. In a dialog, whose window menu would hold Close
//! alone, a click on that button closes.

use gpui::{App, Hsla, Pixels, Point, Window};
use sn_ui_core::winstyle::{Look, WinStyle};

use super::colors::{Rgb, Scheme, hsla, mix};
use super::geom::Rect;
use super::marks::Dir;
use super::paint;
use super::skin::{CapButton, State, outline, ramp, round_border, round_rect};
use super::skin_mac::disc;
use super::style::shade;
use super::text;

/// Which desktop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Desk {
    Adwaita,
    Breeze,
    Clearlooks,
    Motif,
}

/// The parameters of a Linux look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Lin {
    pub(crate) desk: Desk,
    /// Dark mode (Adwaita, Breeze).
    pub(crate) dark: bool,
    /// The accent colour (Adwaita, Breeze; Yaru's orange by default).
    pub(crate) accent: Rgb,
    /// Ubuntu's Yaru variant of Adwaita.
    pub(crate) yaru: bool,
}

impl Lin {
    /// The Linux look of `look`, or `None` for another style.
    pub(crate) fn of(look: Look) -> Option<Self> {
        let desk = match look.style {
            WinStyle::LinuxAdwaita => Desk::Adwaita,
            WinStyle::LinuxBreeze => Desk::Breeze,
            WinStyle::LinuxClearlooks => Desk::Clearlooks,
            WinStyle::LinuxMotif => Desk::Motif,
            _ => return None,
        };
        let modern = matches!(desk, Desk::Adwaita | Desk::Breeze);
        Some(Self {
            desk,
            dark: look.dark && modern,
            accent: match desk {
                Desk::Clearlooks => 0x86_AB_D9,
                Desk::Motif => 0x8C_9C_C6,
                _ => look.drawn_accent(),
            },
            yaru: look.yaru && desk == Desk::Adwaita,
        })
    }

    /// The window colours.
    pub(crate) fn scheme(self) -> Scheme {
        let sel_text = sn_ui_core::winstyle::caption_text_on(self.accent);
        match (self.desk, self.dark) {
            (Desk::Adwaita | Desk::Breeze, true) => {
                let (face, view, title) = if self.desk == Desk::Adwaita {
                    (0x24_24_24, 0x1E_1E_1E, 0x30_30_30)
                } else {
                    (0x31_36_3B, 0x1B_1E_20, 0x002A_2E32)
                };
                Scheme {
                    face,
                    highlight: shade(face, 0.2),
                    light: shade(face, 0.1),
                    shadow: shade(face, -0.3),
                    dark_shadow: 0x00_00_00,
                    window: view,
                    text: 0xFC_FC_FC,
                    active_caption: title,
                    active_caption_text: 0xFC_FC_FC,
                    inactive_caption: face,
                    inactive_caption_text: 0x9A_9A_9A,
                    selection: self.accent,
                    selection_text: sel_text,
                    info: title,
                    info_text: 0xFC_FC_FC,
                    app_workspace: shade(face, -0.25),
                }
            }
            (Desk::Adwaita, false) => Scheme {
                face: 0xFA_FA_FA,
                highlight: 0xFF_FF_FF,
                light: 0xF0_F0_F0,
                shadow: 0xC0_BF_BC,
                dark_shadow: 0x77_76_7B,
                window: 0xFF_FF_FF,
                text: 0x2E_34_36,
                active_caption: 0xEB_EB_EB,
                active_caption_text: 0x2E_34_36,
                inactive_caption: 0xFA_FA_FA,
                inactive_caption_text: 0x6E_6E_6E,
                selection: self.accent,
                selection_text: sel_text,
                info: 0x35_35_35,
                info_text: 0xFF_FF_FF,
                app_workspace: 0xDE_DD_DA,
            },
            (Desk::Breeze, false) => Scheme {
                face: 0xEF_F0_F1,
                highlight: 0xFF_FF_FF,
                light: 0xF7_F7_F7,
                shadow: 0xBC_BE_BF,
                dark_shadow: 0x7F_8C_8D,
                window: 0xFF_FF_FF,
                text: 0x23_26_27,
                active_caption: 0xDE_E0_E2,
                active_caption_text: 0x23_26_27,
                inactive_caption: 0xEF_F0_F1,
                inactive_caption_text: 0x70_7D_8A,
                selection: self.accent,
                selection_text: sel_text,
                info: 0xF7_F7_F7,
                info_text: 0x23_26_27,
                app_workspace: 0xBD_C3_C7,
            },
            (Desk::Clearlooks, _) => Scheme {
                face: 0xED_EC_EB,
                highlight: 0xFF_FF_FF,
                light: 0xF5_F4_F3,
                shadow: 0xA8_A4_9D,
                dark_shadow: 0x5C_59_55,
                window: 0xFF_FF_FF,
                text: 0x00_00_00,
                active_caption: 0x4F_7C_B8,
                active_caption_text: 0xFF_FF_FF,
                inactive_caption: 0xD6_D3_CE,
                inactive_caption_text: 0x5A_5A_5A,
                selection: 0x86_AB_D9,
                selection_text: 0x00_00_00,
                info: 0xF5_F5_B5,
                info_text: 0x00_00_00,
                app_workspace: 0x9D_A7_B3,
            },
            (Desk::Motif, _) => Scheme {
                face: 0xAE_B2_C3,
                highlight: 0xE3_E5_EB,
                light: 0xC9_CC_D8,
                shadow: 0x5C_5F_6B,
                dark_shadow: 0x2E_30_36,
                window: 0xC4_C8_D4,
                text: 0x00_00_00,
                active_caption: 0x9B_A4_C2,
                active_caption_text: 0x00_00_00,
                inactive_caption: 0xAE_B2_C3,
                inactive_caption_text: 0x44_46_50,
                selection: 0x8C_9C_C6,
                selection_text: 0x00_00_00,
                info: 0xFF_FF_E0,
                info_text: 0x00_00_00,
                app_workspace: 0x6F_7D_A0,
            },
        }
    }

    fn text(self) -> Rgb {
        self.scheme().text
    }

    /// Greyed text, 3:1 or better on the face and the menus: greyed menu
    /// items carry notes worth reading.
    fn text_disabled(self) -> Rgb {
        match (self.desk, self.dark) {
            (Desk::Motif, _) => 0x58_5C_6A,
            (_, true) => 0x8C_8C_8C,
            (_, false) => 0x85_85_85,
        }
    }

    /// A translucent tint of the text colour (Adwaita's button fills).
    fn tint(self, a: f32) -> Hsla {
        hsla(if self.dark { 0xFF_FF_FF } else { 0x00_00_00 }).opacity(a)
    }
}

fn row(rows: &[Rgb], y: i32) -> Hsla {
    hsla(
        rows.get(usize::try_from(y).unwrap_or(0))
            .copied()
            .unwrap_or(0),
    )
}

/// A Motif bevel `w` pixels wide: light top and left, dark bottom and right
/// (`raised`), or the other way round.
fn bevel(window: &mut Window, o: Point<Pixels>, r: Rect, w: i32, raised: bool, l: Lin) {
    bevel_in(window, o, r, w, raised, true, l);
}

/// The Motif colours of a bevel's lit and shaded edges: half way to the face
/// colour on a disabled control, so it reads as greyed.
fn bevel_colors(l: Lin, enabled: bool) -> (Rgb, Rgb) {
    let s = l.scheme();
    if enabled {
        (s.highlight, s.shadow)
    } else {
        (mix(s.highlight, s.face, 0.5), mix(s.shadow, s.face, 0.5))
    }
}

/// [`bevel`], greyed when not `enabled`.
fn bevel_in(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    w: i32,
    raised: bool,
    enabled: bool,
    l: Lin,
) {
    let (lit, dim) = bevel_colors(l, enabled);
    let (tl, br) = if raised { (lit, dim) } else { (dim, lit) };
    for i in 0..w {
        let q = r.inset(i);
        if q.w < 2 || q.h < 2 {
            break;
        }
        paint::fill(window, o, Rect::new(q.x, q.y, q.w - 1, 1), hsla(tl));
        paint::fill(window, o, Rect::new(q.x, q.y, 1, q.h - 1), hsla(tl));
        paint::fill(window, o, Rect::new(q.x, q.bottom() - 1, q.w, 1), hsla(br));
        paint::fill(window, o, Rect::new(q.right() - 1, q.y, 1, q.h), hsla(br));
    }
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

/// The frame and title bar of a window `w` x `h`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn frame(
    window: &mut Window,
    o: Point<Pixels>,
    (w, h): (i32, i32),
    frame: i32,
    caption: i32,
    active: bool,
    l: Lin,
) {
    let s = l.scheme();
    let top = frame + caption;
    let title = if active {
        s.active_caption
    } else {
        s.inactive_caption
    };
    match l.desk {
        Desk::Adwaita | Desk::Breeze => {
            let radius = if l.desk == Desk::Adwaita { 8 } else { 3 };
            let edge = hsla(if l.dark { 0x0E_0E_0E } else { 0xB4_B4_B4 });
            round_rect(
                window,
                o,
                Rect::new(0, 0, w, top),
                radius,
                (true, false),
                Some(edge),
                &|_| hsla(title),
            );
            paint::fill(window, o, Rect::new(0, top, w, h - top), hsla(s.face));
            paint::fill(window, o, Rect::new(0, top, 1, h - top), edge);
            paint::fill(window, o, Rect::new(w - 1, top, 1, h - top), edge);
            paint::fill(window, o, Rect::new(0, h - 1, w, 1), edge);
            let line = hsla(if l.dark {
                0x1A_1A_1A
            } else {
                shade(title, -0.12)
            });
            paint::fill(window, o, Rect::new(1, top - 1, w - 2, 1), line);
        }
        Desk::Clearlooks => {
            let rows = if active {
                ramp(
                    &[(0.0, 0x7C_A5_D8), (0.5, 0x4F_7C_B8), (1.0, 0x41_6C_A8)],
                    top,
                )
            } else {
                ramp(&[(0.0, 0xE6_E4_E0), (1.0, 0xD0_CD_C8)], top)
            };
            let edge = hsla(if active { 0x3C_5F_8C } else { 0x8A_88_84 });
            round_rect(
                window,
                o,
                Rect::new(0, 0, w, top),
                5,
                (true, false),
                Some(edge),
                &|y| row(&rows, y),
            );
            paint::fill(window, o, Rect::new(0, top, w, h - top), hsla(s.face));
            outline(window, o, Rect::new(0, top - 1, w, h - top + 1), edge);
        }
        Desk::Motif => {
            // mwm: a raised frame in the window colour with corner handles,
            // the title bar a raised panel inside it.
            let frame_c = hsla(title);
            paint::fill(window, o, Rect::new(0, 0, w, h), frame_c);
            bevel(window, o, Rect::new(0, 0, w, h), 1, true, l);
            bevel(
                window,
                o,
                Rect::new(frame - 1, frame - 1, w - 2 * frame + 2, h - 2 * frame + 2),
                1,
                false,
                l,
            );
            // Corner handle grooves.
            let groove = |window: &mut Window, r: Rect| {
                paint::fill(window, o, r, hsla(s.shadow));
                let lit = if r.w == 1 {
                    Rect::new(r.x + 1, r.y, 1, r.h)
                } else {
                    Rect::new(r.x, r.y + 1, r.w, 1)
                };
                paint::fill(window, o, lit, hsla(s.highlight));
            };
            let k = caption + frame;
            for x in [k, w - k - 1] {
                groove(window, Rect::new(x, 0, 1, frame));
                groove(window, Rect::new(x, h - frame, 1, frame));
            }
            for y in [k, h - k - 1] {
                groove(window, Rect::new(0, y, frame, 1));
                groove(window, Rect::new(w - frame, y, frame, 1));
            }
            let bar = Rect::new(frame, frame, w - 2 * frame, caption);
            paint::fill(window, o, bar, frame_c);
            bevel(window, o, bar, 1, true, l);
            paint::fill(
                window,
                o,
                Rect::new(frame, top, w - 2 * frame, h - top - frame),
                hsla(s.face),
            );
        }
    }
}

/// The width of the window icon place at the left of a title bar `cap_h`
/// high, which opens the window menu: none in Adwaita, Motif's square
/// window-menu button, the 18 pixels of the app icon otherwise.
pub(crate) fn icon_width(l: Lin, cap_h: i32) -> i32 {
    match l.desk {
        Desk::Adwaita => 0,
        Desk::Motif => cap_h,
        Desk::Breeze | Desk::Clearlooks => 18,
    }
}

/// The window icon place at the left of the title bar: nothing in Adwaita,
/// Motif's window-menu button (a raised square with a bar). Returns whether
/// the look draws its own, so the caller draws the app icon otherwise.
pub(crate) fn caption_icon(window: &mut Window, o: Point<Pixels>, cap: Rect, l: Lin) -> bool {
    match l.desk {
        Desk::Adwaita => true,
        Desk::Motif => {
            let b = Rect::new(cap.x, cap.y, icon_width(l, cap.h), cap.h);
            bevel(window, o, b, 1, true, l);
            bevel(window, o, menu_button_bar(b), 1, true, l);
            true
        }
        Desk::Breeze | Desk::Clearlooks => false,
    }
}

/// The bar inside Motif's window-menu button `b`.
fn menu_button_bar(b: Rect) -> Rect {
    Rect::new(b.x + 5, b.y + b.h / 2 - 1, b.w - 10, 3)
}

/// The title: bold and centred in Adwaita, centred in Breeze and Motif, left
/// and bold in Clearlooks; inside `room`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn title(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    room: Rect,
    title: &str,
    active: bool,
    l: Lin,
) {
    let s = l.scheme();
    let fg = if active {
        s.active_caption_text
    } else {
        s.inactive_caption_text
    };
    let bold = matches!(l.desk, Desk::Adwaita | Desk::Clearlooks);
    let tw = text::width_in(text::Face::Caption, window, title, bold);
    let x = if l.desk == Desk::Clearlooks {
        room.x
    } else {
        room.x + ((room.w - tw) / 2).max(0)
    };
    let y = room.y + (room.h - text::LINE) / 2;
    text::draw_in(
        text::Face::Caption,
        window,
        cx,
        paint::at(o, x, y),
        title,
        bold,
        hsla(fg),
    );
}

/// A title bar button.
pub(crate) fn caption_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    kind: CapButton,
    st: State,
    active: bool,
    l: Lin,
) {
    if r.is_empty() {
        // Motif has no close button in title bars and the menu bar.
        return;
    }
    let s = l.scheme();
    let ink_rgb = if active {
        s.active_caption_text
    } else {
        s.inactive_caption_text
    };
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    let glyph = |window: &mut Window, ink: Hsla| match kind {
        CapButton::Close => {
            for k in -3..=3 {
                paint::fill(window, o, Rect::new(cx + k, cy + k, 1, 1), ink);
                paint::fill(window, o, Rect::new(cx - k, cy + k, 1, 1), ink);
            }
        }
        CapButton::Minimize => paint::fill(window, o, Rect::new(cx - 4, cy + 2, 8, 1), ink),
        CapButton::Maximize => outline(window, o, Rect::new(cx - 4, cy - 4, 8, 8), ink),
        CapButton::Restore => {
            outline(window, o, Rect::new(cx - 4, cy - 2, 6, 6), ink);
            paint::fill(window, o, Rect::new(cx - 2, cy - 4, 6, 1), ink);
            paint::fill(window, o, Rect::new(cx + 3, cy - 4, 1, 6), ink);
        }
    };
    match l.desk {
        Desk::Adwaita => {
            let a = match (st.down, st.hot) {
                (true, _) => 0.24,
                (false, true) => 0.16,
                (false, false) => 0.08,
            };
            let tint = l.tint(a);
            disc(window, o, (r.x, r.y), r.w.min(r.h), None, &|_| tint);
            glyph(window, hsla(ink_rgb));
        }
        Desk::Breeze => {
            let close = kind == CapButton::Close;
            if close && (st.hot || st.down) {
                let red = hsla(if st.down { 0xB0_2B_3B } else { 0xDA_44_53 });
                disc(window, o, (r.x, r.y), r.w.min(r.h), None, &|_| red);
                glyph(window, hsla(0xFF_FF_FF));
            } else {
                if st.hot || st.down {
                    let tint = l.tint(if st.down { 0.2 } else { 0.1 });
                    disc(window, o, (r.x, r.y), r.w.min(r.h), None, &|_| tint);
                }
                glyph(window, hsla(ink_rgb));
            }
        }
        Desk::Clearlooks => {
            let base = if active { 0x7F_A6_D8 } else { 0xDE_DB_D6 };
            let base = if st.down {
                shade(base, -0.15)
            } else if st.hot {
                shade(base, 0.12)
            } else {
                base
            };
            let rows = ramp(&[(0.0, shade(base, 0.25)), (1.0, base)], r.h);
            let edge = hsla(if active { 0x3C_5F_8C } else { 0x9A_98_94 });
            round_rect(window, o, r, 3, (true, true), Some(edge), &|y| {
                row(&rows, y)
            });
            glyph(window, hsla(if active { 0xFF_FF_FF } else { 0x6A_6A_6A }));
        }
        Desk::Motif => {
            bevel_in(window, o, r, 1, !st.down, st.enabled, l);
            // Minimize: a small raised square; maximize: a large one.
            let q = match kind {
                CapButton::Minimize => Rect::new(cx - 2, cy - 2, 5, 5),
                CapButton::Maximize | CapButton::Restore => {
                    Rect::new(r.x + 3, r.y + 3, r.w - 6, r.h - 6)
                }
                // A dialog's close is its window-menu button, drawn by
                // `caption_icon`: the bevel above shows it pressed or, with
                // its bar redrawn here, greyed when the dialog cannot close.
                CapButton::Close => {
                    if !st.enabled {
                        bevel_in(window, o, menu_button_bar(r), 1, true, false, l);
                    }
                    return;
                }
            };
            bevel_in(window, o, q, 1, !st.down, st.enabled, l);
        }
    }
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

/// A push button.
pub(crate) fn push_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State, l: Lin) {
    let s = l.scheme();
    match l.desk {
        Desk::Adwaita => {
            let suggested = st.focus && st.enabled;
            if suggested {
                let c = match (st.down, st.hot) {
                    (true, _) => shade(l.accent, -0.15),
                    (false, true) => shade(l.accent, 0.08),
                    (false, false) => l.accent,
                };
                round_rect(window, o, r, 6, (true, true), None, &|_| hsla(c));
            } else {
                let a = match (st.enabled, st.down, st.hot) {
                    (false, ..) => 0.05,
                    (true, true, _) => 0.24,
                    (true, false, true) => 0.14,
                    (true, false, false) => 0.09,
                };
                let tint = l.tint(a);
                round_rect(window, o, r, 6, (true, true), None, &|_| tint);
            }
        }
        Desk::Breeze => {
            let fill = match (l.dark, st.down) {
                (false, false) => 0xFC_FC_FC,
                (false, true) => 0xD3_DA_E3,
                (true, false) => 0x31_36_3B,
                (true, true) => 0x23_26_29,
            };
            let border = if st.enabled && (st.hot || st.focus || st.down) {
                l.accent
            } else if l.dark {
                0x61_65_69
            } else {
                0xBC_BE_BF
            };
            let fill = if st.enabled && st.focus && !st.down {
                mix(fill, l.accent, 0.12)
            } else {
                fill
            };
            round_rect(window, o, r, 3, (true, true), Some(hsla(border)), &|_| {
                hsla(fill)
            });
        }
        Desk::Clearlooks => {
            let (top, bottom) = match (st.down, st.hot) {
                (true, _) => (0xD4_D0_C8, 0xE4_E1_DB),
                (false, true) => (0xFF_FF_FF, 0xF0_EE_EA),
                (false, false) => (0xFB_FB_FA, 0xE3_E0_DB),
            };
            let rows = ramp(&[(0.0, top), (1.0, bottom)], r.h);
            let border = hsla(if st.enabled { 0x8D_8A_85 } else { 0xC0_BD_B8 });
            round_rect(window, o, r, 3, (true, true), Some(border), &|y| {
                row(&rows, y)
            });
            if st.focus && st.enabled {
                round_border(window, o, r.inset(1), 2, (true, true), hsla(s.selection));
            }
        }
        Desk::Motif => {
            paint::fill(window, o, r, hsla(s.face));
            if st.focus {
                // The default button's extra sunken ring.
                bevel(window, o, r, 1, false, l);
                bevel(window, o, r.inset(2), 2, !st.down, l);
            } else {
                bevel(window, o, r, 2, !st.down, l);
            }
        }
    }
}

/// The caption colour of a push button (white on Adwaita's suggested one).
pub(crate) fn button_text(l: Lin, st: State) -> Rgb {
    match (st.enabled, l.desk == Desk::Adwaita && st.focus) {
        (false, _) => l.text_disabled(),
        (true, true) => sn_ui_core::winstyle::caption_text_on(l.accent),
        (true, false) => l.text(),
    }
}

/// The text colour of a control.
pub(crate) fn text_color(l: Lin, enabled: bool) -> Rgb {
    if enabled { l.text() } else { l.text_disabled() }
}

/// A check box (13x13).
pub(crate) fn check(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    checked: bool,
    st: State,
    l: Lin,
) {
    let r = Rect::new(x, y, 13, 13);
    let s = l.scheme();
    let tick = |window: &mut Window, ink: Hsla| {
        let map = [
            ".......#", "......##", "#....##.", "##..##..", ".####...", "..##....",
        ];
        for (dy, line) in (0_i32..).zip(map) {
            for (dx, ch) in (0_i32..).zip(line.chars()) {
                if ch == '#' {
                    paint::fill(window, o, Rect::new(x + 2 + dx, y + 3 + dy, 1, 1), ink);
                }
            }
        }
    };
    match l.desk {
        Desk::Adwaita | Desk::Breeze => {
            let radius = if l.desk == Desk::Adwaita { 4 } else { 3 };
            if checked && st.enabled {
                round_rect(window, o, r, radius, (true, true), None, &|_| {
                    hsla(l.accent)
                });
                tick(
                    window,
                    hsla(sn_ui_core::winstyle::caption_text_on(l.accent)),
                );
            } else {
                let border = if st.hot && st.enabled {
                    l.accent
                } else if l.dark {
                    0x6E_6E_6E
                } else {
                    0xA0_A0_A0
                };
                round_rect(
                    window,
                    o,
                    r,
                    radius,
                    (true, true),
                    Some(hsla(border)),
                    &|_| hsla(s.window),
                );
                if checked {
                    tick(window, hsla(l.text_disabled()));
                }
            }
        }
        Desk::Clearlooks => {
            let border = if st.hot && st.enabled {
                0x5C_7F_AC
            } else {
                0x8D_8A_85
            };
            paint::fill(
                window,
                o,
                r,
                hsla(if st.down { 0xD4_D0_C8 } else { 0xFF_FF_FF }),
            );
            outline(window, o, r, hsla(border));
            if checked {
                tick(
                    window,
                    hsla(if st.enabled { 0x00_00_00 } else { 0xA8_A4_9D }),
                );
            }
        }
        Desk::Motif => {
            // A toggle indicator: raised when off, sunken and filled when on;
            // greyed, a flatter bevel and a face-coloured fill.
            if checked {
                let fill = if st.enabled {
                    s.selection
                } else {
                    shade(s.face, -0.05)
                };
                paint::fill(window, o, r, hsla(fill));
            }
            bevel_in(window, o, r, 2, !checked, st.enabled, l);
        }
    }
}

/// A radio button (13x13): round, Motif's a diamond.
pub(crate) fn radio(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    selected: bool,
    st: State,
    l: Lin,
) {
    let s = l.scheme();
    match l.desk {
        Desk::Adwaita | Desk::Breeze => {
            if selected && st.enabled {
                disc(window, o, (x, y), 13, None, &|_| hsla(l.accent));
                let dot = hsla(sn_ui_core::winstyle::caption_text_on(l.accent));
                disc(window, o, (x + 4, y + 4), 5, None, &|_| dot);
            } else {
                let border = hsla(if st.hot && st.enabled {
                    l.accent
                } else if l.dark {
                    0x6E_6E_6E
                } else {
                    0xA0_A0_A0
                });
                disc(window, o, (x, y), 13, Some(border), &|_| hsla(s.window));
                if selected {
                    let dot = hsla(l.text_disabled());
                    disc(window, o, (x + 4, y + 4), 5, None, &|_| dot);
                }
            }
        }
        Desk::Clearlooks => {
            let border = hsla(if st.hot && st.enabled {
                0x5C_7F_AC
            } else {
                0x8D_8A_85
            });
            disc(window, o, (x, y), 13, Some(border), &|_| hsla(0xFF_FF_FF));
            if selected {
                let dot = hsla(if st.enabled { 0x00_00_00 } else { 0xA8_A4_9D });
                disc(window, o, (x + 4, y + 4), 5, None, &|_| dot);
            }
        }
        Desk::Motif => {
            // A diamond, raised when off, sunken and filled when on; greyed
            // like the check box.
            let (lit, dim) = bevel_colors(l, st.enabled);
            let (tl, br) = if selected { (dim, lit) } else { (lit, dim) };
            let fill = if st.enabled {
                s.selection
            } else {
                shade(s.face, -0.05)
            };
            let c = 6;
            for i in 0..=c {
                let half = i;
                let ytop = y + i;
                let ybot = y + 2 * c - i;
                if selected {
                    paint::fill(
                        window,
                        o,
                        Rect::new(x + c - half, ytop, 2 * half + 1, 1),
                        hsla(fill),
                    );
                    paint::fill(
                        window,
                        o,
                        Rect::new(x + c - half, ybot, 2 * half + 1, 1),
                        hsla(fill),
                    );
                }
                paint::fill(window, o, Rect::new(x + c - half, ytop, 1, 1), hsla(tl));
                paint::fill(window, o, Rect::new(x + c + half, ytop, 1, 1), hsla(tl));
                paint::fill(window, o, Rect::new(x + c - half, ybot, 1, 1), hsla(br));
                paint::fill(window, o, Rect::new(x + c + half, ybot, 1, 1), hsla(br));
            }
        }
    }
}

/// A field border; `false` for Motif, which keeps the classic sunken bevel.
pub(crate) fn field(window: &mut Window, o: Point<Pixels>, r: Rect, l: Lin) -> bool {
    match l.desk {
        Desk::Adwaita => round_border(window, o, r, 5, (true, true), l.tint(0.14)),
        Desk::Breeze => {
            let c = hsla(if l.dark { 0x61_65_69 } else { 0xBC_BE_BF });
            round_border(window, o, r, 3, (true, true), c);
        }
        Desk::Clearlooks => {
            outline(window, o, r, hsla(0x8D_8A_85));
            paint::fill(
                window,
                o,
                Rect::new(r.x + 1, r.y + 1, r.w - 2, 1),
                hsla(0xD8_D6_D2),
            );
        }
        Desk::Motif => return false,
    }
    true
}

/// A small solid triangle.
fn triangle(window: &mut Window, o: Point<Pixels>, r: Rect, dir: Dir, ink: Hsla) {
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

/// A scroll bar arrow or thumb. Adwaita and Breeze: faint arrows on the
/// track (the step places GTK and Qt hide), a rounded slider; Clearlooks: bevelled buttons and a glossy thumb; Motif: 3D arrows
/// and a raised slider.
pub(crate) fn scroll_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    arrow: Option<Dir>,
    st: State,
    vertical: bool,
    l: Lin,
) {
    let s = l.scheme();
    match (l.desk, arrow) {
        (Desk::Adwaita | Desk::Breeze, Some(dir)) => {
            // The arrow places still step a line: a faint arrow shows them.
            scroll_track(window, o, r, vertical, l);
            triangle(window, o, r, dir, l.tint(scroll_arrow_alpha(st)));
        }
        (Desk::Adwaita | Desk::Breeze, None) => {
            let thick = if st.hot || st.down { 8 } else { 6 };
            let pill = if vertical {
                Rect::new(r.x + (r.w - thick) / 2, r.y + 2, thick, r.h - 4)
            } else {
                Rect::new(r.x + 2, r.y + (r.h - thick) / 2, r.w - 4, thick)
            };
            let c = if l.desk == Desk::Breeze && (st.hot || st.down) {
                hsla(l.accent)
            } else {
                l.tint(if st.down { 0.6 } else { 0.4 })
            };
            round_rect(window, o, pill, thick / 2, (true, true), None, &|_| c);
        }
        (Desk::Clearlooks, arrow) => {
            let rows = ramp(
                &[
                    (0.0, 0xFB_FB_FA),
                    (1.0, if st.down { 0xCF_CB_C4 } else { 0xE3_E0_DB }),
                ],
                if vertical { r.w } else { r.h },
            );
            let border = hsla(0x8D_8A_85);
            if vertical {
                // The gradient runs across the bar: left to right, one
                // column at a time inside the border.
                for x in 1..r.w - 1 {
                    let col = Rect::new(r.x + x, r.y + 1, 1, r.h - 2);
                    if !col.is_empty() {
                        paint::fill(window, o, col, row(&rows, x));
                    }
                }
                round_border(window, o, r, 2, (true, true), border);
            } else {
                round_rect(window, o, r, 2, (true, true), Some(border), &|y| {
                    row(&rows, y)
                });
            }
            if let Some(dir) = arrow {
                triangle(
                    window,
                    o,
                    r,
                    dir,
                    hsla(if st.enabled { 0x2E_34_36 } else { 0xA8_A4_9D }),
                );
            } else {
                // The grip lines of a Clearlooks thumb.
                let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
                for i in -1..=1 {
                    let q = if vertical {
                        Rect::new(cx - 3, cy + 2 * i, 7, 1)
                    } else {
                        Rect::new(cx + 2 * i, cy - 3, 1, 7)
                    };
                    paint::fill(window, o, q, hsla(0xA8_A4_9D));
                }
            }
        }
        (Desk::Motif, Some(dir)) => {
            paint::fill(window, o, r, hsla(s.face));
            motif_arrow(window, o, r.inset(2), dir, !st.down, l);
        }
        (Desk::Motif, None) => {
            paint::fill(window, o, r, hsla(s.face));
            bevel(window, o, r, 2, true, l);
        }
    }
}

/// A Motif arrow: a triangle with a light and a dark edge.
fn motif_arrow(window: &mut Window, o: Point<Pixels>, r: Rect, dir: Dir, raised: bool, l: Lin) {
    let s = l.scheme();
    triangle(window, o, r, dir, hsla(s.face));
    let (lit, dim) = if raised {
        (s.highlight, s.shadow)
    } else {
        (s.shadow, s.highlight)
    };
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    for k in 0..4 {
        let (a, b) = match dir {
            Dir::Up => ((cx - 3 + k, cy + 1 - k), (cx + 3 - k, cy + 1 - k)),
            Dir::Down => ((cx - 3 + k, cy - 1 + k), (cx + 3 - k, cy - 1 + k)),
            Dir::Left => ((cx + 1 - k, cy - 3 + k), (cx + 1 - k, cy + 3 - k)),
            Dir::Right => ((cx - 1 + k, cy - 3 + k), (cx - 1 + k, cy + 3 - k)),
        };
        paint::fill(window, o, Rect::new(a.0, a.1, 1, 1), hsla(lit));
        paint::fill(window, o, Rect::new(b.0, b.1, 1, 1), hsla(dim));
    }
}

/// The track of a scroll bar.
/// The opacity of an Adwaita or Breeze scroll arrow's tint: at least 3:1
/// on the window colour when enabled, darker when hot or pressed.
fn scroll_arrow_alpha(st: State) -> f32 {
    match (st.enabled, st.hot || st.down) {
        (false, _) => 0.2,
        (true, true) => 0.7,
        (true, false) => 0.5,
    }
}

pub(crate) fn scroll_track(window: &mut Window, o: Point<Pixels>, r: Rect, vertical: bool, l: Lin) {
    let s = l.scheme();
    match l.desk {
        Desk::Adwaita => paint::fill(window, o, r, hsla(s.window)),
        Desk::Breeze => {
            paint::fill(window, o, r, hsla(s.window));
            let groove = if vertical {
                Rect::new(r.x + (r.w - 6) / 2, r.y + 2, 6, r.h - 4)
            } else {
                Rect::new(r.x + 2, r.y + (r.h - 6) / 2, r.w - 4, 6)
            };
            let c = l.tint(0.1);
            round_rect(window, o, groove, 3, (true, true), None, &|_| c);
        }
        Desk::Clearlooks => paint::fill(window, o, r, hsla(0xD6_D3_CE)),
        Desk::Motif => {
            // A sunken trough in a darker shade.
            paint::fill(window, o, r, hsla(shade(s.face, -0.12)));
        }
    }
}

/// The drop-down part of a combo box.
pub(crate) fn drop_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State, l: Lin) {
    let s = l.scheme();
    match l.desk {
        Desk::Motif => {
            // Motif's option menu indicator: a small raised bar.
            let bar = Rect::new(r.x + (r.w - 10) / 2, r.y + (r.h - 6) / 2, 10, 6);
            bevel(window, o, bar, 1, !st.down, l);
        }
        Desk::Clearlooks => {
            let rows = ramp(&[(0.0, 0xFB_FB_FA), (1.0, 0xE3_E0_DB)], r.h);
            round_rect(
                window,
                o,
                r.inset(1),
                2,
                (true, true),
                Some(hsla(0x8D_8A_85)),
                &|y| row(&rows, y),
            );
            triangle(
                window,
                o,
                r,
                Dir::Down,
                hsla(if st.enabled { 0x2E_34_36 } else { 0xA8_A4_9D }),
            );
        }
        Desk::Adwaita | Desk::Breeze => {
            if st.hot || st.down {
                let c = l.tint(if st.down { 0.16 } else { 0.08 });
                round_rect(window, o, r.inset(1), 4, (true, true), None, &|_| c);
            }
            let ink = hsla(if st.enabled {
                s.text
            } else {
                l.text_disabled()
            });
            // A thin chevron.
            let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
            for k in 0..4 {
                paint::fill(window, o, Rect::new(cx - 3 + k, cy - 1 + k, 1, 1), ink);
                paint::fill(window, o, Rect::new(cx + 3 - k, cy - 1 + k, 1, 1), ink);
            }
        }
    }
}

/// The room a progress bar `r` fills: inside Motif's 2 pixel bevel, inside
/// the 1 pixel border or track inset of the others.
pub(crate) fn progress_inner_w(r: Rect, l: Lin) -> i32 {
    match l.desk {
        Desk::Motif => r.w - 4,
        Desk::Adwaita | Desk::Breeze | Desk::Clearlooks => r.w - 2,
    }
}

/// A progress bar; `span` is the filled part, from 0 to
/// [`progress_inner_w`].
pub(crate) fn progress(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    span: Option<(i32, i32)>,
    l: Lin,
) {
    let s = l.scheme();
    match l.desk {
        Desk::Adwaita | Desk::Breeze => {
            let t = 6;
            let track = Rect::new(r.x + 1, r.y + (r.h - t) / 2, progress_inner_w(r, l), t);
            let tc = l.tint(0.15);
            round_rect(window, o, track, t / 2, (true, true), None, &|_| tc);
            if let Some((x, w)) = span.filter(|&(_, w)| w > 0) {
                let bar = Rect::new(track.x + x, track.y, w.min(track.w - x), t);
                round_rect(window, o, bar, t / 2, (true, true), None, &|_| {
                    hsla(l.accent)
                });
            }
        }
        Desk::Clearlooks => {
            paint::fill(window, o, r, hsla(0xD6_D3_CE));
            outline(window, o, r, hsla(0x8D_8A_85));
            if let Some((x, w)) = span.filter(|&(_, w)| w > 0) {
                let bar = Rect::new(
                    r.x + 1 + x,
                    r.y + 1,
                    w.min(progress_inner_w(r, l) - x),
                    r.h - 2,
                );
                let rows = ramp(&[(0.0, 0xA8_C6_EA), (1.0, 0x6E_9A_CF)], bar.h);
                for (yy, c) in (0_i32..).zip(&rows) {
                    paint::fill(window, o, Rect::new(bar.x, bar.y + yy, bar.w, 1), hsla(*c));
                }
                // Clearlooks' diagonal stripes.
                let light = hsla(0xFF_FF_FF).opacity(0.25);
                for yy in 0..bar.h {
                    let mut sx = -(yy % 10);
                    while sx < bar.w {
                        let x0 = sx.max(0);
                        let wq = (5 + sx.min(0)).min(bar.w - x0);
                        if wq > 0 {
                            paint::fill(window, o, Rect::new(bar.x + x0, bar.y + yy, wq, 1), light);
                        }
                        sx += 10;
                    }
                }
            }
        }
        Desk::Motif => {
            paint::fill(window, o, r, hsla(shade(s.face, -0.12)));
            bevel(window, o, r, 2, false, l);
            if let Some((x, w)) = span.filter(|&(_, w)| w > 0) {
                let bar = Rect::new(
                    r.x + 2 + x,
                    r.y + 2,
                    w.min(progress_inner_w(r, l) - x),
                    r.h - 4,
                );
                paint::fill(window, o, bar, hsla(s.selection));
            }
        }
    }
}

/// A slider.
pub(crate) fn trackbar(
    window: &mut Window,
    o: Point<Pixels>,
    channel: Rect,
    thumb: Rect,
    st: State,
    l: Lin,
) {
    let s = l.scheme();
    let mid = channel.y + channel.h / 2;
    let cxp = thumb.x + thumb.w / 2;
    match l.desk {
        Desk::Adwaita | Desk::Breeze => {
            let t = if l.desk == Desk::Adwaita { 4 } else { 6 };
            let track = Rect::new(channel.x, mid - t / 2, channel.w, t);
            let tc = l.tint(0.15);
            round_rect(window, o, track, t / 2, (true, true), None, &|_| tc);
            if st.enabled {
                let fill = Rect::new(channel.x, mid - t / 2, (cxp - channel.x).max(0), t);
                round_rect(window, o, fill, t / 2, (true, true), None, &|_| {
                    hsla(l.accent)
                });
            }
            let d = 17;
            let knob = hsla(if l.dark { 0xD0_D0_D0 } else { 0xFF_FF_FF });
            let rim = hsla(if st.hot && l.desk == Desk::Breeze {
                l.accent
            } else if l.dark {
                0x10_10_10
            } else {
                0xB8_B8_B8
            });
            disc(window, o, (cxp - d / 2, mid - d / 2), d, Some(rim), &|_| {
                knob
            });
        }
        Desk::Clearlooks => {
            let track = Rect::new(channel.x, mid - 2, channel.w, 4);
            paint::fill(window, o, track, hsla(0xD6_D3_CE));
            outline(window, o, track, hsla(0x8D_8A_85));
            let rows = ramp(&[(0.0, 0xFF_FF_FF), (1.0, 0xE3_E0_DB)], thumb.h);
            round_rect(
                window,
                o,
                thumb,
                2,
                (true, true),
                Some(hsla(0x8D_8A_85)),
                &|y| row(&rows, y),
            );
        }
        Desk::Motif => {
            let track = Rect::new(channel.x, mid - 3, channel.w, 6);
            paint::fill(window, o, track, hsla(shade(s.face, -0.12)));
            bevel(window, o, track, 1, false, l);
            paint::fill(window, o, thumb, hsla(s.face));
            bevel(window, o, thumb, 2, true, l);
            // The slider's centre groove.
            paint::fill(
                window,
                o,
                Rect::new(cxp - 1, thumb.y + 3, 1, thumb.h - 6),
                hsla(s.shadow),
            );
            paint::fill(
                window,
                o,
                Rect::new(cxp, thumb.y + 3, 1, thumb.h - 6),
                hsla(s.highlight),
            );
        }
    }
}

/// A group box frame.
pub(crate) fn group(window: &mut Window, o: Point<Pixels>, r: Rect, l: Lin) {
    let s = l.scheme();
    match l.desk {
        Desk::Adwaita => round_border(window, o, r, 8, (true, true), l.tint(0.12)),
        Desk::Breeze => round_border(window, o, r, 3, (true, true), l.tint(0.18)),
        Desk::Clearlooks | Desk::Motif => {
            // An etched line.
            outline(window, o, r, hsla(s.shadow));
            outline(
                window,
                o,
                Rect::new(r.x + 1, r.y + 1, r.w - 1, r.h - 1),
                hsla(s.highlight),
            );
            outline(
                window,
                o,
                Rect::new(r.x, r.y, r.w - 1, r.h - 1),
                hsla(s.shadow),
            );
        }
    }
}

/// A hot, pressed or checked toolbar button.
pub(crate) fn tool_button(window: &mut Window, o: Point<Pixels>, r: Rect, down: bool, l: Lin) {
    match l.desk {
        Desk::Adwaita => {
            let c = l.tint(if down { 0.2 } else { 0.1 });
            round_rect(window, o, r, 6, (true, true), None, &|_| c);
        }
        Desk::Breeze => {
            let fill = hsla(l.accent).opacity(if down { 0.35 } else { 0.18 });
            round_rect(window, o, r, 3, (true, true), Some(hsla(l.accent)), &|_| {
                fill
            });
        }
        Desk::Clearlooks => push_button(
            window,
            o,
            r,
            State {
                hot: !down,
                down,
                focus: false,
                enabled: true,
            },
            l,
        ),
        Desk::Motif => bevel(window, o, r, 2, !down, l),
    }
}

// ---------------------------------------------------------------------------
// Menus
// ---------------------------------------------------------------------------

/// The background of a popup menu.
pub(crate) fn menu_bg(l: Lin) -> Rgb {
    match (l.desk, l.dark) {
        (Desk::Adwaita, false) => 0xFF_FF_FF,
        (Desk::Adwaita, true) => 0x38_38_38,
        (Desk::Breeze, false) => 0xFC_FC_FC,
        (Desk::Breeze, true) => 0x29_2C_30,
        (Desk::Clearlooks, _) => 0xF5_F4_F3,
        (Desk::Motif, _) => 0xAE_B2_C3,
    }
}

/// A popup menu.
pub(crate) fn menu_popup(window: &mut Window, o: Point<Pixels>, r: Rect, l: Lin) {
    let bg = hsla(menu_bg(l));
    match l.desk {
        Desk::Adwaita => round_rect(window, o, r, 9, (true, true), Some(l.tint(0.14)), &|_| bg),
        Desk::Breeze => round_rect(window, o, r, 4, (true, true), Some(l.tint(0.2)), &|_| bg),
        Desk::Clearlooks => {
            paint::fill(window, o, r, bg);
            outline(window, o, r, hsla(0x8D_8A_85));
        }
        Desk::Motif => {
            paint::fill(window, o, r, bg);
            bevel(window, o, r, 2, true, l);
        }
    }
}

/// The highlight of a hot menu item or menu bar title.
pub(crate) fn menu_highlight(window: &mut Window, o: Point<Pixels>, r: Rect, l: Lin) {
    match l.desk {
        Desk::Adwaita => {
            let q = Rect::new(r.x + 4, r.y + 1, r.w - 8, r.h - 2);
            let c = l.tint(0.1);
            round_rect(window, o, q, 6, (true, true), None, &|_| c);
        }
        Desk::Breeze => {
            let q = Rect::new(r.x + 2, r.y, r.w - 4, r.h);
            let fill = hsla(l.accent).opacity(0.22);
            round_rect(window, o, q, 3, (true, true), Some(hsla(l.accent)), &|_| {
                fill
            });
        }
        Desk::Clearlooks => {
            let rows = ramp(&[(0.0, 0x9C_BC_E3), (1.0, 0x6E_9A_CF)], r.h);
            for (y, c) in (0_i32..).zip(&rows) {
                paint::fill(window, o, Rect::new(r.x, r.y + y, r.w, 1), hsla(*c));
            }
        }
        // Motif raises the item; its text keeps its colour.
        Desk::Motif => bevel(window, o, r, 2, true, l),
    }
}

/// Whether hot menu items keep the text colour (all but Clearlooks, whose
/// blue highlight takes the selection text colour: black, which reads on
/// the light end of the gradient where GNOME 2's white would not).
pub(crate) fn menu_keeps_text(l: Lin) -> bool {
    l.desk != Desk::Clearlooks
}

/// Whether menus always draw mnemonic underlines (Motif and Clearlooks);
/// GTK and Qt show them only while Alt is held, which the menus add.
pub(crate) fn menu_underlines(l: Lin) -> bool {
    matches!(l.desk, Desk::Motif | Desk::Clearlooks)
}

/// A separator of a popup menu.
pub(crate) fn menu_separator(window: &mut Window, o: Point<Pixels>, row: Rect, y: i32, l: Lin) {
    let s = l.scheme();
    match l.desk {
        Desk::Motif | Desk::Clearlooks => {
            paint::fill(
                window,
                o,
                Rect::new(row.x + 2, y, row.w - 4, 1),
                hsla(s.shadow),
            );
            paint::fill(
                window,
                o,
                Rect::new(row.x + 2, y + 1, row.w - 4, 1),
                hsla(s.highlight),
            );
        }
        Desk::Adwaita | Desk::Breeze => paint::fill(
            window,
            o,
            Rect::new(row.x + 8, y, row.w - 16, 1),
            l.tint(0.12),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(style: WinStyle) -> Look {
        Look {
            style,
            ..Look::default()
        }
    }

    #[test]
    fn only_the_linux_styles_have_a_linux_look() {
        assert!(Lin::of(look(WinStyle::Xp)).is_none());
        assert_eq!(
            Lin::of(look(WinStyle::LinuxMotif)).map(|l| l.desk),
            Some(Desk::Motif)
        );
    }

    #[test]
    fn yaru_is_orange_and_breeze_blue_by_default() {
        let yaru = Look {
            yaru: true,
            ..look(WinStyle::LinuxAdwaita)
        };
        assert_eq!(
            Lin::of(yaru).map(|l| l.accent),
            Some(sn_ui_core::winstyle::YARU_ORANGE)
        );
        assert_eq!(
            Lin::of(look(WinStyle::LinuxBreeze)).map(|l| l.accent),
            Some(sn_ui_core::winstyle::BREEZE_BLUE)
        );
    }

    #[test]
    fn greyed_motif_bevels_are_flatter() {
        use sn_ui_core::winstyle::contrast_ratio;
        let Some(l) = Lin::of(look(WinStyle::LinuxMotif)) else {
            panic!("a Linux look");
        };
        let (lit, dim) = bevel_colors(l, true);
        let (glit, gdim) = bevel_colors(l, false);
        assert!(contrast_ratio(glit, gdim) < contrast_ratio(lit, dim));
    }

    #[test]
    fn motif_progress_fills_inside_its_bevel() {
        let r = Rect::new(0, 0, 200, 16);
        let inner = |style| Lin::of(look(style)).map(|l| progress_inner_w(r, l));
        assert_eq!(inner(WinStyle::LinuxMotif), Some(196));
        assert_eq!(inner(WinStyle::LinuxAdwaita), Some(198));
        assert_eq!(inner(WinStyle::LinuxClearlooks), Some(198));
    }

    #[test]
    fn only_adwaita_and_breeze_go_dark() {
        for (style, dark) in [
            (WinStyle::LinuxAdwaita, true),
            (WinStyle::LinuxBreeze, true),
            (WinStyle::LinuxClearlooks, false),
            (WinStyle::LinuxMotif, false),
        ] {
            let l = Lin::of(Look {
                dark: true,
                ..look(style)
            });
            assert_eq!(l.map(|l| l.dark), Some(dark), "{style:?}");
        }
    }

    #[test]
    fn scroll_arrows_read_on_the_window_colour() {
        use sn_ui_core::winstyle::contrast_ratio;
        for style in [WinStyle::LinuxAdwaita, WinStyle::LinuxBreeze] {
            for dark in [false, true] {
                let Some(l) = Lin::of(Look {
                    dark,
                    ..look(style)
                }) else {
                    panic!("a Linux look");
                };
                let bg = l.scheme().window;
                let ink = if dark { 0xFF_FF_FF } else { 0 };
                for hot in [false, true] {
                    let st = State {
                        hot,
                        down: false,
                        focus: false,
                        enabled: true,
                    };
                    let arrow = mix(bg, ink, f64::from(scroll_arrow_alpha(st)));
                    assert!(
                        contrast_ratio(arrow, bg) >= 3.0,
                        "{style:?} dark {dark} hot {hot}: {arrow:06X} on {bg:06X}"
                    );
                }
            }
        }
    }

    #[test]
    fn text_reads_on_every_scheme() {
        use sn_ui_core::winstyle::contrast_ratio;
        for style in [
            WinStyle::LinuxAdwaita,
            WinStyle::LinuxBreeze,
            WinStyle::LinuxClearlooks,
            WinStyle::LinuxMotif,
        ] {
            for dark in [false, true] {
                let Some(l) = Lin::of(Look {
                    dark,
                    ..look(style)
                }) else {
                    panic!("a Linux look");
                };
                let s = l.scheme();
                assert!(contrast_ratio(s.text, s.face) >= 4.5, "{style:?} {dark}");
                let grey = l.text_disabled();
                for bg in [s.face, menu_bg(l)] {
                    assert!(
                        contrast_ratio(grey, bg) >= 3.0,
                        "{style:?} {dark}: {grey:06X} on {bg:06X}"
                    );
                }
                assert!(
                    contrast_ratio(s.active_caption_text, s.active_caption) >= 3.0,
                    "{style:?} {dark}"
                );
            }
        }
    }
}
