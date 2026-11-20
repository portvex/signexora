//! Windows 11: the Fluent look of its common controls, drawn on the pixel
//! grid. Rounded controls (4 px) and menus (8 px), light grey Mica title bars
//! (accent title bars are off by default), accent-filled check boxes, radio
//! buttons and default buttons, thin pill scroll thumbs, a thin progress
//! line, round slider thumbs, and a light and a dark app mode.
//!
//! MDI children keep square corners (Windows rounds only top-level windows);
//! SA's dialogs and message boxes are top-level and get rounded corners.

use gpui::{App, Hsla, Pixels, Point, Window};

use super::colors::{Rgb, hsla, mix};
use super::geom::Rect;
use super::marks::Dir;
use super::paint;
use super::skin::{CapButton, State, outline, round_rect};
use super::style::shade;
use super::text;

/// The parameters of the Windows 11 style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fluent {
    /// Dark app mode.
    pub(crate) dark: bool,
    /// The accent colour.
    pub(crate) accent: Rgb,
}

impl Fluent {
    /// The accent as controls use it: one shade darker in light mode
    /// (`AccentDark1`), lighter in dark mode (`AccentLight2`).
    fn control_accent(self) -> Rgb {
        if self.dark {
            shade(self.accent, 0.45)
        } else {
            shade(self.accent, -0.2)
        }
    }

    /// Text on the accent.
    fn on_accent(self) -> Rgb {
        if self.dark { 0x00_00_00 } else { 0xFF_FF_FF }
    }

    fn text(self) -> Rgb {
        if self.dark { 0xFF_FF_FF } else { 0x1A_1A_1A }
    }

    fn text_disabled(self) -> Rgb {
        if self.dark { 0x78_78_78 } else { 0xA0_A0_A0 }
    }

    /// The Mica title bar and window background.
    fn mica(self, active: bool) -> Rgb {
        match (self.dark, active) {
            (false, true) => 0xEE_F4_F9,
            (false, false) => 0xF3_F3_F3,
            (true, true) => 0x20_20_20,
            (true, false) => 0x2B_2B_2B,
        }
    }

    fn stroke(self) -> Rgb {
        if self.dark { 0x40_40_40 } else { 0xE5_E5_E5 }
    }
}

/// The frame and title bar of a window: square for MDI children, rounded
/// (8 px) for top-level windows; a one-pixel grey border.
#[allow(clippy::too_many_arguments)]
pub(crate) fn frame(
    window: &mut Window,
    o: Point<Pixels>,
    (w, h): (i32, i32),
    frame: i32,
    caption: i32,
    active: bool,
    top_level: bool,
    f: Fluent,
) {
    let bg = hsla(f.mica(active));
    let border = hsla(if f.dark { 0x5A_5A_5A } else { 0xBD_BD_BD });
    let all = Rect::new(0, 0, w, h);
    if top_level {
        round_rect(window, o, all, 8, (true, true), Some(border), &|_| bg);
        // The client area below keeps the window colour.
        let top = frame + caption;
        paint::fill(window, o, Rect::new(1, top, w - 2, h - top - 8), bg);
    } else {
        let top = frame + caption;
        paint::fill(window, o, Rect::new(0, 0, w, top), bg);
        paint::fill(window, o, Rect::new(0, top, frame, h - top), bg);
        paint::fill(window, o, Rect::new(w - frame, top, frame, h - top), bg);
        paint::fill(
            window,
            o,
            Rect::new(frame, h - frame, w - 2 * frame, frame),
            bg,
        );
        outline(window, o, all, border);
    }
}

/// The title: Segoe UI Variable, black (white in dark mode), grey when the
/// window is inactive.
#[allow(clippy::too_many_arguments)]
pub(crate) fn title(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    title: &str,
    active: bool,
    f: Fluent,
) {
    let fg = if active { f.text() } else { f.text_disabled() };
    let at = paint::at(o, x + 2, y);
    text::draw_in(text::Face::Caption, window, cx, at, title, false, hsla(fg));
}

/// A Windows 11 caption button: the thin glyph alone, a light square under
/// the mouse, red under the mouse for close.
pub(crate) fn caption_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    kind: CapButton,
    st: State,
    active: bool,
    f: Fluent,
) {
    let close = kind == CapButton::Close;
    let fill = match (close, st.down, st.hot, f.dark) {
        (true, true, ..) => Some(0xC8_3C_31),
        (true, false, true, _) => Some(0xC4_2B_1C),
        (false, true, _, false) => Some(0xDF_DF_DF),
        (false, false, true, false) => Some(0xE9_E9_E9),
        (false, true, _, true) => Some(0x33_33_33),
        (false, false, true, true) => Some(0x2D_2D_2D),
        _ => None,
    };
    if let Some(c) = fill {
        paint::fill(window, o, r, hsla(c));
    }
    let ink = hsla(if close && (st.hot || st.down) {
        0xFF_FF_FF
    } else if active && st.enabled {
        f.text()
    } else {
        f.text_disabled()
    });
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    match kind {
        CapButton::Minimize => paint::fill(window, o, Rect::new(cx - 5, cy, 10, 1), ink),
        CapButton::Maximize => {
            round_rect(
                window,
                o,
                Rect::new(cx - 5, cy - 5, 10, 10),
                2,
                (true, true),
                Some(ink),
                &|_| hsla(0).opacity(0.0),
            );
        }
        CapButton::Restore => {
            round_rect(
                window,
                o,
                Rect::new(cx - 5, cy - 3, 8, 8),
                2,
                (true, true),
                Some(ink),
                &|_| hsla(0).opacity(0.0),
            );
            paint::fill(window, o, Rect::new(cx - 2, cy - 5, 6, 1), ink);
            paint::fill(window, o, Rect::new(cx + 4, cy - 2, 1, 6), ink);
        }
        CapButton::Close => {
            for k in -5..=5 {
                paint::fill(window, o, Rect::new(cx + k, cy + k, 1, 1), ink);
                paint::fill(window, o, Rect::new(cx - k, cy + k, 1, 1), ink);
            }
        }
    }
}

/// A Windows 11 push button: rounded (4 px), a light fill with a slightly
/// darker bottom edge; the default button filled with the accent.
pub(crate) fn push_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State, f: Fluent) {
    let accent = st.focus && st.enabled;
    let (fill, border, bottom) = match (accent, st.enabled, st.down, st.hot, f.dark) {
        (true, _, true, ..) => (
            mix(f.control_accent(), f.mica(true), 0.2),
            f.control_accent(),
            f.control_accent(),
        ),
        (true, _, false, true, _) => (
            mix(f.control_accent(), 0xFF_FF_FF, 0.1),
            f.control_accent(),
            shade(f.control_accent(), -0.25),
        ),
        (true, ..) => (
            f.control_accent(),
            f.control_accent(),
            shade(f.control_accent(), -0.25),
        ),
        // Disabled and pressed share the flat, borderless look.
        (false, false, .., false) | (false, true, true, _, false) => {
            (0xF5_F5_F5, 0xE5_E5_E5, 0xE5_E5_E5)
        }
        (false, false, .., true) => (0x2A_2A_2A, 0x36_36_36, 0x36_36_36),
        (false, true, false, true, false) => (0xF6_F6_F6, 0xE5_E5_E5, 0xCC_CC_CC),
        (false, true, false, false, false) => (0xFD_FD_FD, 0xE5_E5_E5, 0xCC_CC_CC),
        (false, true, true, _, true) => (0x27_27_27, 0x36_36_36, 0x36_36_36),
        (false, true, false, true, true) => (0x0032_3232, 0x3A_3A_3A, 0x45_45_45),
        (false, true, false, false, true) => (0x2D_2D_2D, 0x36_36_36, 0x45_45_45),
    };
    round_rect(window, o, r, 4, (true, true), Some(hsla(border)), &|_| {
        hsla(fill)
    });
    paint::fill(
        window,
        o,
        Rect::new(r.x + 3, r.bottom() - 1, r.w - 6, 1),
        hsla(bottom),
    );
}

/// The caption colour of a push button (white on the accent-filled default
/// button).
pub(crate) fn button_text(f: Fluent, st: State) -> Rgb {
    match (st.enabled, st.focus) {
        (false, _) => f.text_disabled(),
        (true, true) => f.on_accent(),
        (true, false) => f.text(),
    }
}

/// The text colour of a control.
pub(crate) fn text_color(f: Fluent, enabled: bool) -> Rgb {
    if enabled { f.text() } else { f.text_disabled() }
}

/// A Windows 11 check box (13x13): rounded (3 px); unchecked a light square
/// with a grey border, checked filled with the accent and a white tick.
pub(crate) fn check(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    checked: bool,
    st: State,
    f: Fluent,
) {
    let r = Rect::new(x, y, 13, 13);
    if checked {
        let c = match (st.enabled, st.down, st.hot) {
            (false, ..) => f.text_disabled(),
            (true, true, _) => mix(f.control_accent(), f.mica(true), 0.3),
            (true, false, true) => mix(f.control_accent(), 0xFF_FF_FF, 0.1),
            (true, false, false) => f.control_accent(),
        };
        round_rect(window, o, r, 3, (true, true), Some(hsla(c)), &|_| hsla(c));
        let ink = hsla(f.on_accent());
        let map = [".......#", "......#.", "#....#..", ".#..#...", "..##...."];
        for (dy, line) in (0_i32..).zip(map) {
            for (dx, ch) in (0_i32..).zip(line.chars()) {
                if ch == '#' {
                    paint::fill(window, o, Rect::new(x + 2 + dx, y + 4 + dy, 1, 1), ink);
                }
            }
        }
    } else {
        let (border, fill) = match (st.enabled, st.down, st.hot, f.dark) {
            (false, .., false) => (0xC6_C6_C6, 0xF5_F5_F5),
            (false, .., true) => (0x4A_4A_4A, 0x26_26_26),
            (true, true, _, false) => (0x8A_8A_8A, 0xE0_E0_E0),
            (true, false, true, false) => (0x8A_8A_8A, 0xEB_EB_EB),
            (true, false, false, false) => (0x8A_8A_8A, 0xF5_F5_F5),
            (true, _, _, true) => (0x9A_9A_9A, if st.hot { 0x33_33_33 } else { 0x26_26_26 }),
        };
        round_rect(window, o, r, 3, (true, true), Some(hsla(border)), &|_| {
            hsla(fill)
        });
    }
}

/// A Windows 11 radio button (13x13): a grey ring, or when selected a thick
/// accent ring around a white dot.
pub(crate) fn radio(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    selected: bool,
    st: State,
    f: Fluent,
) {
    let disc = |window: &mut Window, d: i32, c: Hsla| {
        // A filled circle `d` pixels across, centred in the 13x13 box.
        let rr = f64::from(d) / 2.0;
        let c0 = 6.5;
        for py in 0..13 {
            for px_ in 0..13 {
                let dx = f64::from(px_) + 0.5 - c0;
                let dy = f64::from(py) + 0.5 - c0;
                if dx * dx + dy * dy <= rr * rr {
                    paint::fill(window, o, Rect::new(x + px_, y + py, 1, 1), c);
                }
            }
        }
    };
    if selected {
        let ring = match (st.enabled, st.hot) {
            (false, _) => f.text_disabled(),
            (true, true) => mix(f.control_accent(), 0xFF_FF_FF, 0.1),
            (true, false) => f.control_accent(),
        };
        disc(window, 13, hsla(ring));
        let dot = if st.down {
            5
        } else if st.hot {
            8
        } else {
            6
        };
        disc(window, dot, hsla(f.on_accent()));
    } else {
        let (border, fill) = if f.dark {
            (0x9A_9A_9A, 0x26_26_26)
        } else {
            (0x8A_8A_8A, 0xF5_F5_F5)
        };
        let border = if st.enabled { border } else { 0xC6_C6_C6 };
        disc(window, 13, hsla(border));
        disc(
            window,
            11,
            hsla(if st.hot { shade(fill, -0.05) } else { fill }),
        );
        if st.down {
            disc(window, 6, hsla(border));
        }
    }
}

/// A Windows 11 field: rounded (4 px), a light border and a darker bottom
/// line (edit fields, lists, combo boxes).
pub(crate) fn field(window: &mut Window, o: Point<Pixels>, r: Rect, f: Fluent) {
    let stroke = hsla(f.stroke());
    round_rect(window, o, r, 4, (true, true), Some(stroke), &|_| {
        hsla(0).opacity(0.0)
    });
    let bottom = if f.dark { 0x9A_9A_9A } else { 0x86_86_86 };
    paint::fill(
        window,
        o,
        Rect::new(r.x + 3, r.bottom() - 1, r.w - 6, 1),
        hsla(bottom),
    );
}

/// A thin chevron.
fn chevron(window: &mut Window, o: Point<Pixels>, r: Rect, dir: Dir, ink: Hsla) {
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    for k in 0..4 {
        let pts = match dir {
            Dir::Down => [(cx - 3 + k, cy - 1 + k), (cx + 3 - k, cy - 1 + k)],
            Dir::Up => [(cx - 3 + k, cy + 2 - k), (cx + 3 - k, cy + 2 - k)],
            Dir::Right => [(cx - 1 + k, cy - 3 + k), (cx - 1 + k, cy + 3 - k)],
            Dir::Left => [(cx + 2 - k, cy - 3 + k), (cx + 2 - k, cy + 3 - k)],
        };
        for (px_, py) in pts {
            paint::fill(window, o, Rect::new(px_, py, 1, 1), ink);
        }
    }
}

/// A small solid triangle (Windows 11 scroll arrows).
fn triangle(window: &mut Window, o: Point<Pixels>, r: Rect, dir: Dir, ink: Hsla) {
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    for k in 0..3 {
        let q = match dir {
            Dir::Down => Rect::new(cx - 2 + k, cy - 1 + k, 5 - 2 * k, 1),
            Dir::Up => Rect::new(cx - 2 + k, cy + 1 - k, 5 - 2 * k, 1),
            Dir::Right => Rect::new(cx - 1 + k, cy - 2 + k, 1, 5 - 2 * k),
            Dir::Left => Rect::new(cx + 1 - k, cy - 2 + k, 1, 5 - 2 * k),
        };
        paint::fill(window, o, q, ink);
    }
}

/// A Windows 11 scroll arrow (a small triangle) or thumb (a thin grey pill,
/// wider under the mouse).
pub(crate) fn scroll_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    arrow: Option<Dir>,
    st: State,
    vertical: bool,
    f: Fluent,
) {
    let grey = if f.dark { 0x9F_9F_9F } else { 0x8A_8A_8A };
    let Some(dir) = arrow else {
        let thick = if st.hot || st.down { 6 } else { 4 };
        let pill = if vertical {
            Rect::new(r.x + (r.w - thick) / 2, r.y + 1, thick, r.h - 2)
        } else {
            Rect::new(r.x + 1, r.y + (r.h - thick) / 2, r.w - 2, thick)
        };
        let c = hsla(if st.down { shade(grey, -0.2) } else { grey });
        round_rect(window, o, pill, thick / 2, (true, true), None, &|_| c);
        return;
    };
    let ink = hsla(if st.enabled { grey } else { f.text_disabled() });
    triangle(window, o, r, dir, ink);
}

/// The track of a Windows 11 scroll bar.
pub(crate) fn scroll_track(window: &mut Window, o: Point<Pixels>, r: Rect, f: Fluent) {
    paint::fill(
        window,
        o,
        r,
        hsla(if f.dark { 0x2B_2B_2B } else { 0xF9_F9_F9 }),
    );
}

/// The chevron of a Windows 11 combo box.
pub(crate) fn drop_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State, f: Fluent) {
    if st.hot || st.down {
        let c = if f.dark { 0x0032_3232 } else { 0xF0_F0_F0 };
        round_rect(window, o, r.inset(1), 3, (true, true), None, &|_| hsla(c));
    }
    let ink = hsla(if st.enabled {
        f.text()
    } else {
        f.text_disabled()
    });
    chevron(window, o, r, Dir::Down, ink);
}

/// A Windows 11 progress bar: a thin grey line with a thicker rounded accent
/// bar over the filled `span` (start, width) of the inner width.
pub(crate) fn progress(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    span: Option<(i32, i32)>,
    f: Fluent,
) {
    paint::fill(window, o, r, hsla(f.mica(true)));
    let mid = r.y + r.h / 2;
    let track = Rect::new(r.x + 1, mid, r.w - 2, 1);
    paint::fill(
        window,
        o,
        track,
        hsla(if f.dark { 0x9A_9A_9A } else { 0x86_86_86 }),
    );
    if let Some((x, w)) = span.filter(|&(_, w)| w > 0) {
        let bar = Rect::new(r.x + 1 + x, mid - 1, w.min(r.w - 2 - x), 3);
        let c = hsla(f.control_accent());
        round_rect(window, o, bar, 1, (true, true), None, &|_| c);
    }
}

/// The background of a Windows 11 popup menu.
pub(crate) fn menu_bg(f: Fluent) -> Rgb {
    if f.dark { 0x2C_2C_2C } else { 0xF9_F9_F9 }
}

/// A Windows 11 slider: a thin rounded track, accent up to the thumb, and a
/// round thumb with an accent centre.
pub(crate) fn trackbar(
    window: &mut Window,
    o: Point<Pixels>,
    channel: Rect,
    thumb: Rect,
    st: State,
    f: Fluent,
) {
    let mid = channel.y + channel.h / 2;
    let track = Rect::new(channel.x, mid - 2, channel.w, 4);
    let grey = hsla(if f.dark { 0x9A_9A_9A } else { 0x86_86_86 });
    round_rect(window, o, track, 2, (true, true), None, &|_| grey);
    let cxp = thumb.x + thumb.w / 2;
    let acc = hsla(if st.enabled {
        f.control_accent()
    } else {
        f.text_disabled()
    });
    let filled = Rect::new(channel.x, mid - 2, (cxp - channel.x).max(0), 4);
    round_rect(window, o, filled, 2, (true, true), None, &|_| acc);
    let d = 17;
    let disc = Rect::new(cxp - d / 2, mid - d / 2, d, d);
    let ring = hsla(if f.dark { 0x45_45_45 } else { 0xFF_FF_FF });
    round_rect(
        window,
        o,
        disc,
        8,
        (true, true),
        Some(hsla(f.stroke())),
        &|_| ring,
    );
    let inner = if st.hot { 11 } else { 9 };
    let ic = Rect::new(cxp - inner / 2, mid - inner / 2, inner, inner);
    round_rect(window, o, ic, inner / 2, (true, true), None, &|_| acc);
}

/// A Windows 11 group box frame (rounded, light).
pub(crate) fn group(window: &mut Window, o: Point<Pixels>, r: Rect, f: Fluent) {
    round_rect(
        window,
        o,
        r,
        4,
        (true, true),
        Some(hsla(f.stroke())),
        &|_| hsla(0).opacity(0.0),
    );
}

/// A hot, pressed or checked Windows 11 toolbar button: a rounded subtle fill.
pub(crate) fn tool_button(window: &mut Window, o: Point<Pixels>, r: Rect, down: bool, f: Fluent) {
    let c = match (f.dark, down) {
        (false, true) => 0xE5_E5_E5,
        (false, false) => 0xF0_F0_F0,
        (true, true) => 0x3A_3A_3A,
        (true, false) => 0x0032_3232,
    };
    round_rect(window, o, r, 4, (true, true), None, &|_| hsla(c));
}

/// A Windows 11 popup menu: rounded (8 px), light Mica, a light border.
pub(crate) fn menu_popup(window: &mut Window, o: Point<Pixels>, r: Rect, f: Fluent) {
    let bg = hsla(menu_bg(f));
    round_rect(
        window,
        o,
        r,
        8,
        (true, true),
        Some(hsla(if f.dark { 0x45_45_45 } else { 0xD6_D6_D6 })),
        &|_| bg,
    );
}

/// The rounded, inset highlight of a hot menu item or menu bar title.
pub(crate) fn menu_highlight(window: &mut Window, o: Point<Pixels>, r: Rect, f: Fluent) {
    let c = hsla(if f.dark { 0x3A_3A_3A } else { 0xEA_EA_EA });
    let q = Rect::new(r.x + 3, r.y + 1, r.w - 6, r.h - 2);
    round_rect(window, o, q, 4, (true, true), None, &|_| c);
}

/// A separator line of a Windows 11 popup menu.
pub(crate) fn menu_separator(window: &mut Window, o: Point<Pixels>, row: Rect, y: i32, f: Fluent) {
    paint::fill(
        window,
        o,
        Rect::new(row.x + 1, y, row.w - 2, 1),
        hsla(f.stroke()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_accent_is_darker_in_light_mode() {
        let light = Fluent {
            dark: false,
            accent: 0x00_78_D7,
        };
        let dark = Fluent {
            dark: true,
            ..light
        };
        assert_ne!(light.control_accent(), 0x00_78_D7);
        assert_ne!(light.control_accent(), dark.control_accent());
        assert_eq!(light.on_accent(), 0xFF_FF_FF);
    }
}
