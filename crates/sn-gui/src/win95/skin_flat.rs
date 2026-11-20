//! Windows 8 and 10: flat, square controls and accent-coloured frames, drawn
//! on the pixel grid. Windows 8 centres the caption title and keeps its close
//! button red; Windows 10 left-aligns the title, shows the close button red
//! only under the mouse, and has a dark app mode.

use gpui::{App, Hsla, Pixels, Point, Window};

use super::colors::{Rgb, hsla, mix};
use super::geom::Rect;
use super::marks::Dir;
use super::paint;
use super::skin::{CapButton, State, outline};
use super::text;

/// The parameters of the flat styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Flat {
    /// Windows 10 (else 8).
    pub(crate) win10: bool,
    /// Windows 10's dark app mode.
    pub(crate) dark: bool,
    /// The accent colour.
    pub(crate) accent: Rgb,
}

/// Black or white on `bg`, as Windows 10 picks the caption text on an accent:
/// white unless it would be too faint (below WCAG AA), then black.
fn on(bg: Rgb) -> Rgb {
    sn_ui_core::winstyle::caption_text_on(bg)
}

/// The frame colour of a window: the accent when active, a light (or, in
/// dark mode, dark) grey when inactive.
fn frame_color(f: Flat, active: bool) -> Rgb {
    match (active, f.dark) {
        (true, _) => f.accent,
        (false, false) => 0xEB_EB_EB,
        (false, true) => 0x2B_2B_2B,
    }
}

/// The flat frame and caption of a window `w` x `h`.
pub(crate) fn frame(
    window: &mut Window,
    o: Point<Pixels>,
    (w, h): (i32, i32),
    frame: i32,
    caption: i32,
    active: bool,
    f: Flat,
) {
    let c = hsla(frame_color(f, active));
    let top = frame + caption;
    paint::fill(window, o, Rect::new(0, 0, w, top), c);
    paint::fill(window, o, Rect::new(0, top, frame, h - top), c);
    paint::fill(window, o, Rect::new(w - frame, top, frame, h - top), c);
    paint::fill(
        window,
        o,
        Rect::new(frame, h - frame, w - 2 * frame, frame),
        c,
    );
    // A one-pixel darker outline, as the frame's edge.
    let edge = hsla(mix(frame_color(f, active), 0x00_00_00, 0.25));
    outline(window, o, Rect::new(0, 0, w, h), edge);
}

/// The caption title: centred in Windows 8, left in Windows 10; black or
/// white for the frame colour.
#[allow(clippy::too_many_arguments)]
pub(crate) fn title(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    title_w: i32,
    title: &str,
    active: bool,
    f: Flat,
) {
    let bg = frame_color(f, active);
    let fg = if active { on(bg) } else { mix(on(bg), bg, 0.5) };
    let x = if f.win10 {
        x
    } else {
        let tw = text::width_in(text::Face::Caption, window, title, false);
        x + ((title_w - tw) / 2).max(0)
    };
    let at = paint::at(o, x, y);
    text::draw_in(text::Face::Caption, window, cx, at, title, false, hsla(fg));
}

/// A flat caption button: the glyph only, a lighter or darker square under
/// the mouse, and the red close button (always in Windows 8 on an active
/// window, under the mouse in Windows 10).
pub(crate) fn caption_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    kind: CapButton,
    st: State,
    active: bool,
    f: Flat,
) {
    let bg = frame_color(f, active);
    let close = kind == CapButton::Close;
    let red = close && (st.hot || st.down || (!f.win10 && active));
    let fill: Option<Rgb> = if red {
        Some(match (st.down, st.hot, f.win10) {
            (true, ..) => 0x8B_0A_14,
            (false, true, true) => 0xE8_11_23,
            (false, true, false) => 0xE0_43_43,
            _ => 0xC7_50_50,
        })
    } else if st.down {
        Some(mix(bg, on(bg), 0.3))
    } else if st.hot {
        Some(mix(bg, on(bg), 0.15))
    } else {
        None
    };
    if let Some(c) = fill {
        paint::fill(window, o, r, hsla(c));
    }
    let ink_rgb = if red {
        0xFF_FF_FF
    } else if active {
        on(bg)
    } else {
        mix(on(bg), bg, 0.5)
    };
    let ink = hsla(ink_rgb);
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    match kind {
        CapButton::Minimize => paint::fill(window, o, Rect::new(cx - 5, cy, 10, 1), ink),
        CapButton::Maximize => outline(window, o, Rect::new(cx - 5, cy - 5, 10, 10), ink),
        CapButton::Restore => {
            outline(window, o, Rect::new(cx - 5, cy - 3, 8, 8), ink);
            paint::fill(window, o, Rect::new(cx - 3, cy - 5, 8, 1), ink);
            paint::fill(window, o, Rect::new(cx + 4, cy - 5, 1, 8), ink);
        }
        CapButton::Close => {
            for k in -5..=5 {
                paint::fill(window, o, Rect::new(cx + k, cy + k, 1, 1), ink);
                paint::fill(window, o, Rect::new(cx - k, cy + k, 1, 1), ink);
            }
        }
    }
}

/// The colours of a flat push button: (border, fill, text).
fn button_colors(f: Flat, st: State) -> (Rgb, Rgb, Rgb) {
    match (f.dark, st.enabled, st.down, st.hot, st.focus) {
        (false, false, ..) => (0xBF_BF_BF, 0xCC_CC_CC, 0x83_83_83),
        (false, true, true, ..) => (0x00_54_99, 0xCC_E4_F7, 0x00_00_00),
        (false, true, false, true, _) => (0x00_78_D7, 0xE5_F1_FB, 0x00_00_00),
        (false, true, false, false, true) => (0x00_78_D7, 0xE1_E1_E1, 0x00_00_00),
        (false, true, false, false, false) => (0xAD_AD_AD, 0xE1_E1_E1, 0x00_00_00),
        (true, false, ..) => (0x4A_4A_4A, 0x2B_2B_2B, 0x7A_7A_7A),
        (true, true, true, ..) => (0xFF_FF_FF, 0x66_66_66, 0xFF_FF_FF),
        (true, true, false, true, _) => (0xFF_FF_FF, 0x45_45_45, 0xFF_FF_FF),
        (true, true, false, false, true) => (f.accent, 0x33_33_33, 0xFF_FF_FF),
        (true, true, false, false, false) => (0x9B_9B_9B, 0x33_33_33, 0xFF_FF_FF),
    }
}

/// A flat push button: a square one-pixel border, a flat fill, blue when hot
/// or pressed, a blue border when defaulted.
pub(crate) fn push_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State, f: Flat) {
    let (border, fill, _) = button_colors(f, st);
    paint::fill(window, o, r, hsla(fill));
    outline(window, o, r, hsla(border));
    if st.focus && st.enabled && !st.down {
        // The defaulted button's border is two pixels wide.
        outline(window, o, r.inset(1), hsla(border));
    }
}

/// The text colour of a flat control.
pub(crate) fn text_color(f: Flat, enabled: bool) -> Rgb {
    button_colors(
        f,
        State {
            enabled,
            ..State::default()
        },
    )
    .2
}

/// The box colours of a check box or radio button: (border, fill, mark).
fn box_colors(f: Flat, st: State) -> (Rgb, Rgb, Rgb) {
    match (f.dark, st.enabled, st.down, st.hot) {
        (false, false, ..) => (0xCC_CC_CC, 0xFF_FF_FF, 0xCC_CC_CC),
        (false, true, true, _) => (0x00_54_99, 0xCC_E4_F7, 0x00_54_99),
        (false, true, false, true) => (0x00_78_D7, 0xFF_FF_FF, 0x00_78_D7),
        (false, true, false, false) => (0x33_33_33, 0xFF_FF_FF, 0x33_33_33),
        (true, false, ..) => (0x4A_4A_4A, 0x19_19_19, 0x4A_4A_4A),
        (true, true, true, _) => (0xFF_FF_FF, 0x66_66_66, 0xFF_FF_FF),
        (true, true, false, _) => (0xFF_FF_FF, 0x19_19_19, 0xFF_FF_FF),
    }
}

/// A flat check box (13x13): a square border and a thin tick.
pub(crate) fn check(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    checked: bool,
    st: State,
    f: Flat,
) {
    let r = Rect::new(x, y, 13, 13);
    let (border, fill, mark) = box_colors(f, st);
    paint::fill(window, o, r, hsla(fill));
    outline(window, o, r, hsla(border));
    if checked {
        let ink = hsla(mark);
        let map = [
            "........#",
            ".......#.",
            "#.....#..",
            ".#...#...",
            "..#.#....",
            "...#.....",
        ];
        for (dy, line) in (0_i32..).zip(map) {
            for (dx, ch) in (0_i32..).zip(line.chars()) {
                if ch == '#' {
                    paint::fill(window, o, Rect::new(x + 2 + dx, y + 3 + dy, 1, 1), ink);
                }
            }
        }
    }
}

/// A flat radio button (13x13): a thin ring and a round dot.
pub(crate) fn radio(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    selected: bool,
    st: State,
    f: Flat,
) {
    let (border, fill, mark) = box_colors(f, st);
    let ring = [
        "....BBBBB....",
        "..BBFFFFFBB..",
        ".BFFFFFFFFFB.",
        ".BFFFFFFFFFB.",
        "BFFFFFFFFFFFB",
        "BFFFFFFFFFFFB",
        "BFFFFFFFFFFFB",
        "BFFFFFFFFFFFB",
        "BFFFFFFFFFFFB",
        ".BFFFFFFFFFB.",
        ".BFFFFFFFFFB.",
        "..BBFFFFFBB..",
        "....BBBBB....",
    ];
    for (dy, line) in (0_i32..).zip(ring) {
        for (dx, ch) in (0_i32..).zip(line.chars()) {
            let c = match ch {
                'B' => border,
                'F' => fill,
                _ => continue,
            };
            paint::fill(window, o, Rect::new(x + dx, y + dy, 1, 1), hsla(c));
        }
    }
    if selected {
        let dot = [".###.", "#####", "#####", "#####", ".###."];
        for (dy, line) in (0_i32..).zip(dot) {
            for (dx, ch) in (0_i32..).zip(line.chars()) {
                if ch == '#' {
                    paint::fill(
                        window,
                        o,
                        Rect::new(x + 4 + dx, y + 4 + dy, 1, 1),
                        hsla(mark),
                    );
                }
            }
        }
    }
}

/// The one-pixel border of fields.
pub(crate) fn field_border(f: Flat) -> Rgb {
    match (f.win10, f.dark) {
        (_, true) => 0x9B_9B_9B,
        (true, false) => 0x7A_7A_7A,
        (false, false) => 0xAB_AD_B3,
    }
}

/// A thin chevron (the arrows of flat scroll bars and combo boxes).
pub(crate) fn chevron(window: &mut Window, o: Point<Pixels>, r: Rect, dir: Dir, ink: Hsla) {
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    for k in 0..4 {
        let pts = match dir {
            Dir::Down => [(cx - 3 + k, cy - 1 + k), (cx + 3 - k, cy - 1 + k)],
            Dir::Up => [(cx - 3 + k, cy + 2 - k), (cx + 3 - k, cy + 2 - k)],
            Dir::Right => [(cx - 1 + k, cy - 3 + k), (cx - 1 + k, cy + 3 - k)],
            Dir::Left => [(cx + 2 - k, cy - 3 + k), (cx + 2 - k, cy + 3 - k)],
        };
        for (px, py) in pts {
            paint::fill(window, o, Rect::new(px, py, 1, 1), ink);
        }
    }
}

/// A flat scroll arrow (glyph only, a grey square under the mouse) or thumb
/// (a grey bar, darker under the mouse or held).
pub(crate) fn scroll_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    arrow: Option<Dir>,
    st: State,
    vertical: bool,
    f: Flat,
) {
    let Some(dir) = arrow else {
        let c = match (f.dark, st.down, st.hot) {
            (false, true, _) => 0x60_60_60,
            (false, false, true) => 0xA6_A6_A6,
            (false, false, false) => 0xCD_CD_CD,
            (true, true, _) => 0xC8_C8_C8,
            (true, false, true) => 0x9E_9E_9E,
            (true, false, false) => 0x4D_4D_4D,
        };
        let bar = if vertical {
            Rect::new(r.x + 1, r.y, r.w - 2, r.h)
        } else {
            Rect::new(r.x, r.y + 1, r.w, r.h - 2)
        };
        paint::fill(window, o, bar, hsla(c));
        return;
    };
    if st.down || st.hot {
        let c = match (f.dark, st.down) {
            (false, true) => 0x60_60_60,
            (false, false) => 0xDA_DA_DA,
            (true, true) => 0xC8_C8_C8,
            (true, false) => 0x37_37_37,
        };
        paint::fill(window, o, r, hsla(c));
    }
    let ink = match (f.dark, st.enabled, st.down) {
        (_, false, _) => 0xBF_BF_BF,
        (false, true, true) => 0xFF_FF_FF,
        (false, true, false) => 0x60_60_60,
        (true, true, true) => 0x00_00_00,
        (true, true, false) => 0xC8_C8_C8,
    };
    chevron(window, o, r, dir, hsla(ink));
}

/// The track of a flat scroll bar.
pub(crate) fn scroll_track(window: &mut Window, o: Point<Pixels>, r: Rect, f: Flat) {
    paint::fill(
        window,
        o,
        r,
        hsla(if f.dark { 0x17_17_17 } else { 0xF0_F0_F0 }),
    );
}

/// A flat combo drop-down button: a chevron, a light square under the mouse.
pub(crate) fn drop_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State, f: Flat) {
    if st.hot || st.down {
        let c = match (f.dark, st.down) {
            (false, true) => 0xCC_E4_F7,
            (false, false) => 0xE5_F1_FB,
            (true, true) => 0x66_66_66,
            (true, false) => 0x45_45_45,
        };
        paint::fill(window, o, r, hsla(c));
    }
    let ink = if !st.enabled {
        0xBF_BF_BF
    } else if f.dark {
        0xFF_FF_FF
    } else {
        0x60_60_60
    };
    chevron(window, o, r, Dir::Down, hsla(ink));
}

/// A flat progress bar: a grey trough and a flat green fill.
pub(crate) fn progress(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    span: Option<(i32, i32)>,
    f: Flat,
) {
    paint::fill(
        window,
        o,
        r,
        hsla(if f.dark { 0x33_33_33 } else { 0xE6_E6_E6 }),
    );
    outline(
        window,
        o,
        r,
        hsla(if f.dark { 0x4A_4A_4A } else { 0xBC_BC_BC }),
    );
    let inner = r.inset(1);
    let Some((x, w)) = span else {
        return;
    };
    let x = x.clamp(0, inner.w);
    let w = w.min(inner.w - x);
    if w > 0 {
        paint::fill(
            window,
            o,
            Rect::new(inner.x + x, inner.y, w, inner.h),
            hsla(0x06_B0_25),
        );
    }
}

/// A flat track bar: a thin grey channel and a thumb in the accent colour.
pub(crate) fn trackbar(
    window: &mut Window,
    o: Point<Pixels>,
    channel: Rect,
    thumb: Rect,
    st: State,
    f: Flat,
) {
    let ch = Rect::new(channel.x, channel.y + 1, channel.w, 2);
    paint::fill(
        window,
        o,
        ch,
        hsla(if f.dark { 0x6B_6B_6B } else { 0xC4_C4_C4 }),
    );
    let c = match (st.enabled, st.hot, f.dark) {
        (false, ..) => 0xCC_CC_CC,
        (true, true, false) => 0x17_17_17,
        (true, true, true) => 0xF2_F2_F2,
        (true, false, _) => f.accent,
    };
    paint::fill(
        window,
        o,
        Rect::new(thumb.x + 2, thumb.y, thumb.w - 4, thumb.h),
        hsla(c),
    );
}

/// A flat group box frame (square).
pub(crate) fn group(window: &mut Window, o: Point<Pixels>, r: Rect, f: Flat) {
    outline(
        window,
        o,
        r,
        hsla(if f.dark { 0x4A_4A_4A } else { 0xDC_DC_DC }),
    );
}

/// A hot, pressed or checked flat toolbar button.
pub(crate) fn tool_button(window: &mut Window, o: Point<Pixels>, r: Rect, down: bool, f: Flat) {
    let (border, fill) = match (f.dark, down) {
        (false, true) => (0x99_D1_FF, 0xCC_E8_FF),
        (false, false) => (0xCC_E8_FF, 0xE5_F3_FF),
        (true, true) => (0x99_99_99, 0x66_66_66),
        (true, false) => (0x66_66_66, 0x45_45_45),
    };
    paint::fill(window, o, r, hsla(fill));
    outline(window, o, r, hsla(border));
}

/// The colours of flat menus: (popup background, border, hot item, separator).
pub(crate) fn menu_colors(f: Flat) -> (Rgb, Rgb, Rgb, Rgb) {
    if f.dark {
        (0x2B_2B_2B, 0x55_55_55, 0x41_41_41, 0x55_55_55)
    } else if f.win10 {
        (0xF2_F2_F2, 0xCC_CC_CC, 0x91_C9_F7, 0xD7_D7_D7)
    } else {
        (0xF0_F0_F0, 0x97_97_97, 0xD5_E7_F8, 0xD7_D7_D7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win95::colors::contrast;
    use sn_ui_core::winstyle::{ACCENTS, MIN_CAPTION_CONTRAST};

    #[test]
    fn active_titles_read_on_every_accent() {
        for a in ACCENTS {
            for (win10, dark) in [(false, false), (true, false), (true, true)] {
                let f = Flat {
                    win10,
                    dark,
                    accent: a.rgb(),
                };
                let bg = frame_color(f, true);
                assert!(
                    contrast(on(bg), bg) >= MIN_CAPTION_CONTRAST,
                    "{a:?} {f:?}: {:06X} on {bg:06X}",
                    on(bg)
                );
            }
        }
        for bg in [frame_color_inactive(false), frame_color_inactive(true)] {
            assert!(contrast(on(bg), bg) >= 7.0, "{bg:06X}");
        }
    }

    fn frame_color_inactive(dark: bool) -> Rgb {
        frame_color(
            Flat {
                win10: true,
                dark,
                accent: 0,
            },
            false,
        )
    }
}
