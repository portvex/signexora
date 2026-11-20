//! The macOS looks, drawn on the pixel grid without any Apple artwork:
//! classic Aqua (Mac OS X 10.0 to 10.9: pinstripes, glossy "gel" controls in
//! Blue or Graphite), Yosemite to Catalina (10.10 to 10.15: the first flat
//! design) and the modern design (Big Sur, macOS 11, and later: rounded
//! controls in the accent colour). Yosemite and later have a light and a
//! dark appearance.
//!
//! Title bars carry the close, minimize and zoom buttons ("traffic lights")
//! at the left and the title centred; their glyphs show under the mouse.

use gpui::{App, Hsla, Pixels, Point, Window};

use sn_ui_core::winstyle::{AquaColor, Look, MIN_CAPTION_CONTRAST, WinStyle, caption_text_on};

use super::colors::{Rgb, contrast, hsla, mix};
use super::geom::Rect;
use super::marks::Dir;
use super::paint;
use super::skin::{CapButton, State, outline, ramp, round_border, round_rect};
use super::style::shade;
use super::text;

/// Which macOS design.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Era {
    /// Mac OS X 10.0 to 10.9.
    Aqua,
    /// OS X 10.10 to macOS 10.15.
    Yosemite,
    /// macOS 11 and later.
    Modern,
}

/// The parameters of a macOS look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Mac {
    pub(crate) era: Era,
    /// Dark appearance (Yosemite and later).
    pub(crate) dark: bool,
    /// The accent colour (Yosemite and later).
    pub(crate) accent: Rgb,
    /// Aqua's Graphite appearance.
    pub(crate) graphite: bool,
}

impl Mac {
    /// The macOS look of `look`, or `None` for a Windows style. The one place
    /// that decides the era, the dark appearance (not in Aqua) and Aqua's
    /// Blue or Graphite, for the skin and the colour scheme alike.
    pub(crate) fn of(look: Look) -> Option<Self> {
        let era = match look.style {
            WinStyle::MacAqua => Era::Aqua,
            WinStyle::MacYosemite => Era::Yosemite,
            WinStyle::MacModern => Era::Modern,
            _ => return None,
        };
        Some(Self {
            era,
            dark: look.dark && era != Era::Aqua,
            // The brighter dark variant in the dark mode, as macOS draws it.
            accent: look.drawn_accent(),
            graphite: look.aqua == AquaColor::Graphite,
        })
    }

    /// The control accent: Aqua's gel blue (or graphite), else the accent.
    pub(crate) fn tint(self) -> Rgb {
        match (self.era, self.graphite) {
            (Era::Aqua, false) => 0x38_75_D7,
            (Era::Aqua, true) => 0x6E_7B_8E,
            _ => self.accent,
        }
    }

    /// The accent as a fill under text (selections, menu highlights, the
    /// default button, a checked box, a radio, a pop-up cap) and the text
    /// colour on it: white on the accent, darkened in 1% steps (up to 25%)
    /// only as far as white needs to reach [`MIN_CAPTION_CONTRAST`], or
    /// black on an accent that would need more (yellow, green, orange).
    ///
    /// The readable text wins over macOS's own colours here: in the dark
    /// mode macOS draws white on the brighter accent (0x0A84FF for blue,
    /// about 3.6:1), which this darkens to the same readable level as the
    /// light mode. The brighter accent shows in the dark mode where no
    /// text sits on it ([`Self::tint`]: focus rings, sliders, progress
    /// bars).
    pub(crate) fn selection(self) -> (Rgb, Rgb) {
        const WHITE: Rgb = 0xFF_FF_FF;
        let t = self.tint();
        (0..=25)
            .map(|k| shade(t, -0.01 * f64::from(k)))
            .find(|&c| contrast(WHITE, c) >= MIN_CAPTION_CONTRAST)
            .map_or((t, caption_text_on(t)), |c| (c, WHITE))
    }

    /// The rows of the title bar and top frame, `top` pixels high, of an
    /// active or inactive window: what [`frame`] paints and what the title
    /// text must read on.
    pub(crate) fn title_rows(self, top: i32, active: bool) -> Vec<Rgb> {
        let n = usize::try_from(top).unwrap_or(0);
        match (self.era, self.dark, active) {
            (Era::Aqua, ..) => {
                // Pinstripes over a grey metal gradient.
                let base = ramp(&[(0.0, 0xEE_EE_EE), (1.0, 0xC8_C8_C8)], top);
                (0_i32..)
                    .zip(base)
                    .map(|(y, c)| if y % 2 == 0 { c } else { shade(c, -0.04) })
                    .collect()
            }
            (_, true, true) => ramp(&[(0.0, 0x3D_3D_3D), (1.0, 0x33_33_33)], top),
            (_, true, false) => vec![Self::INACTIVE_TITLE_DARK; n],
            (Era::Yosemite, false, true) => ramp(&[(0.0, 0xEB_EB_EB), (1.0, 0xD5_D5_D5)], top),
            (Era::Modern, false, true) => vec![0xE8_E8_E8; n],
            (_, false, false) => vec![Self::INACTIVE_TITLE_LIGHT; n],
        }
    }

    /// The flat inactive title bar of Yosemite and later, light mode.
    pub(crate) const INACTIVE_TITLE_LIGHT: Rgb = 0xF6_F6_F6;
    /// The flat inactive title bar of Yosemite and later, dark mode.
    pub(crate) const INACTIVE_TITLE_DARK: Rgb = 0x2D_2D_2D;
    /// Aqua's pinstriped title bar at its middle row (the colour a scheme
    /// reports as the inactive caption).
    pub(crate) const AQUA_TITLE_MID: Rgb = 0xDB_DB_DB;

    fn text(self) -> Rgb {
        if self.dark { 0xDF_DF_DF } else { 0x00_00_00 }
    }

    fn text_disabled(self) -> Rgb {
        if self.dark { 0x6E_6E_6E } else { 0xA6_A6_A6 }
    }

    fn window_bg(self) -> Rgb {
        match (self.era, self.dark) {
            (_, true) => 0x0032_3232,
            (Era::Aqua, false) => 0xE8_E8_E8,
            (_, false) => 0xEC_EC_EC,
        }
    }

    fn control_bg(self) -> Rgb {
        if self.dark { 0x65_65_65 } else { 0xFF_FF_FF }
    }

    fn border(self) -> Rgb {
        match (self.era, self.dark) {
            (_, true) => 0x50_50_50,
            (Era::Aqua, false) => 0x8E_8E_8E,
            (Era::Yosemite, false) => 0xC0_C0_C0,
            (Era::Modern, false) => 0xCC_CC_CC,
        }
    }
}

fn row(rows: &[Rgb], y: i32) -> Hsla {
    hsla(
        rows.get(usize::try_from(y).unwrap_or(0))
            .copied()
            .unwrap_or(0),
    )
}

/// A filled circle `d` pixels across with its top left at `(x, y)`, each
/// row coloured by `fill(row)`, with an optional rim. Each row is painted as
/// one span (plus a pixel run of rim at each end).
pub(crate) fn disc(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    d: i32,
    rim: Option<Hsla>,
    fill: &dyn Fn(i32) -> Hsla,
) {
    for py in 0..d {
        let (outer, inner) = disc_row(d, py);
        let Some((a, b)) = outer else {
            continue;
        };
        let Some(rc) = rim else {
            paint::fill(window, o, Rect::new(x + a, y + py, b - a + 1, 1), fill(py));
            continue;
        };
        match inner {
            Some((ia, ib)) => {
                paint::fill(window, o, Rect::new(x + a, y + py, ia - a, 1), rc);
                paint::fill(
                    window,
                    o,
                    Rect::new(x + ia, y + py, ib - ia + 1, 1),
                    fill(py),
                );
                paint::fill(window, o, Rect::new(x + ib + 1, y + py, b - ib, 1), rc);
            }
            None => paint::fill(window, o, Rect::new(x + a, y + py, b - a + 1, 1), rc),
        }
    }
}

/// The first and last pixel of a run in a row.
type Run = (i32, i32);

/// Row `py` of a disc `d` pixels across: the first and last pixel inside
/// the disc, and the first and last pixel inside its one-pixel rim. The disc
/// is convex, so both are single runs.
fn disc_row(d: i32, py: i32) -> (Option<Run>, Option<Run>) {
    let r = f64::from(d) / 2.0;
    let dy = f64::from(py) + 0.5 - r;
    let dist = |px: i32| {
        let dx = f64::from(px) + 0.5 - r;
        (dx * dx + dy * dy).sqrt()
    };
    let run = |limit: f64| {
        let a = (0..d).find(|&px| dist(px) <= limit)?;
        let b = (0..d).rev().find(|&px| dist(px) <= limit)?;
        Some((a, b))
    };
    (run(r), run(r - 1.0))
}

/// The glossy "gel" rows of Aqua: a bright top half and a glowing bottom.
fn gel(base: Rgb, h: i32) -> Vec<Rgb> {
    ramp(
        &[
            (0.0, mix(base, 0xFF_FF_FF, 0.75)),
            (0.45, mix(base, 0xFF_FF_FF, 0.35)),
            (0.5, base),
            (0.8, mix(base, 0xFF_FF_FF, 0.2)),
            (1.0, mix(base, 0xFF_FF_FF, 0.6)),
        ],
        h,
    )
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

/// The frame and title bar of a window `w` x `h`: rounded top corners, the
/// era's title bar, the window background below and a one-pixel edge.
#[allow(clippy::too_many_arguments)]
pub(crate) fn frame(
    window: &mut Window,
    o: Point<Pixels>,
    (w, h): (i32, i32),
    frame: i32,
    caption: i32,
    active: bool,
    m: Mac,
) {
    let top = frame + caption;
    let rows = m.title_rows(top, active);
    let edge = hsla(if m.dark { 0x1A_1A_1A } else { 0x9A_9A_9A });
    let radius = if m.era == Era::Modern { 9 } else { 5 };
    round_rect(
        window,
        o,
        Rect::new(0, 0, w, top),
        radius,
        (true, false),
        Some(edge),
        &|y| row(&rows, y),
    );
    // The line under the title bar, then the window body.
    let line = hsla(if m.dark { 0x1A_1A_1A } else { 0xB4_B4_B4 });
    paint::fill(window, o, Rect::new(1, top - 1, w - 2, 1), line);
    let body = hsla(m.window_bg());
    paint::fill(window, o, Rect::new(0, top, w, h - top), body);
    paint::fill(window, o, Rect::new(0, top, 1, h - top), edge);
    paint::fill(window, o, Rect::new(w - 1, top, 1, h - top), edge);
    paint::fill(window, o, Rect::new(0, h - 1, w, 1), edge);
}

/// The title, centred over the whole title bar `cap` when it fits in `room`
/// (the part of the caption clear of the traffic lights); else it starts at
/// the left of `room` and ends with "..." so its start stays readable. `fg`
/// is the scheme's caption text colour.
#[allow(clippy::too_many_arguments)]
pub(crate) fn title(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    (cap, room): (Rect, Rect),
    title: &str,
    active: bool,
    fg: Rgb,
    m: Mac,
) {
    let bold = m.era == Era::Modern;
    let width = |t: &str| text::width_in(text::Face::Caption, window, t, bold);
    let tw = width(title);
    let (shown, x) = if tw <= room.w {
        let x = cap.x + (cap.w - tw) / 2;
        (
            title.to_owned(),
            x.clamp(room.x, (room.right() - tw).max(room.x)),
        )
    } else {
        (fit_title(title, room.w, &width), room.x)
    };
    let y = cap.y + (cap.h - text::LINE) / 2;
    if m.era == Era::Aqua && active {
        // Aqua's embossed title: a white line under the black text.
        let at = paint::at(o, x, y + 1);
        text::draw_in(
            text::Face::Caption,
            window,
            cx,
            at,
            &shown,
            bold,
            hsla(0xFF_FF_FF).opacity(0.7),
        );
    }
    let at = paint::at(o, x, y);
    text::draw_in(text::Face::Caption, window, cx, at, &shown, bold, hsla(fg));
}

/// The longest start of `title` that, followed by "...", is at most `w`
/// wide as measured by `width` (just "..." if nothing fits).
fn fit_title(title: &str, w: i32, width: &dyn Fn(&str) -> i32) -> String {
    let ends: Vec<usize> = title
        .char_indices()
        .map(|(i, _)| i)
        .skip(1)
        .chain(std::iter::once(title.len()))
        .collect();
    let with_dots = |n: usize| {
        let end = n
            .checked_sub(1)
            .and_then(|k| ends.get(k))
            .copied()
            .unwrap_or(0);
        format!("{}...", title.get(..end).unwrap_or_default().trim_end())
    };
    // Binary search for the most characters that fit.
    let (mut lo, mut hi) = (0, ends.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if width(&with_dots(mid)) <= w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    with_dots(lo)
}

/// A traffic light: red close, yellow minimize, green zoom; glossy in Aqua,
/// flat later; grey when the window is inactive, unless the mouse is over
/// the lights. `st.hot` is set on all the lights while the mouse is over any
/// of them, and then each shows its glyph, as macOS does.
pub(crate) fn caption_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    kind: CapButton,
    st: State,
    active: bool,
    m: Mac,
) {
    let lit = (active || st.hot) && st.enabled;
    let (fill, rim): (Rgb, Rgb) = if !lit {
        if m.dark {
            (0x5A_5A_5A, 0x4A_4A_4A)
        } else {
            (0xDC_DC_DC, 0xC4_C4_C4)
        }
    } else if m.era == Era::Aqua && m.graphite {
        (0xA8_AE_B8, 0x6A_70_7A)
    } else {
        match (kind, m.era) {
            (CapButton::Close, Era::Modern) => (0xFF_5F_57, 0xE0_44_3E),
            (CapButton::Close, _) => (0xFC_60_5C, 0xDE_3F_3A),
            (CapButton::Minimize, Era::Modern) => (0xFE_BC_2E, 0xDE_A1_23),
            (CapButton::Minimize, _) => (0xFD_BC_40, 0xDE_9F_34),
            (CapButton::Maximize | CapButton::Restore, Era::Modern) => (0x28_C8_40, 0x1A_AB_29),
            (CapButton::Maximize | CapButton::Restore, _) => (0x34_C8_4A, 0x27_AA_35),
        }
    };
    let fill = if st.down { shade(fill, -0.2) } else { fill };
    let d = r.w.min(r.h);
    let at = (r.x + (r.w - d) / 2, r.y + (r.h - d) / 2);
    if m.era == Era::Aqua {
        let rows = gel(fill, d);
        disc(window, o, at, d, Some(hsla(shade(rim, -0.3))), &|y| {
            row(&rows, y)
        });
    } else {
        disc(window, o, at, d, Some(hsla(rim)), &|_| hsla(fill));
    }
    if st.hot && st.enabled {
        let ink = hsla(shade(rim, -0.55));
        let (cx, cy) = (at.0 + d / 2, at.1 + d / 2);
        match kind {
            CapButton::Close => {
                for k in -2..=2 {
                    paint::fill(window, o, Rect::new(cx + k, cy + k, 1, 1), ink);
                    paint::fill(window, o, Rect::new(cx - k, cy + k, 1, 1), ink);
                }
            }
            CapButton::Minimize => paint::fill(window, o, Rect::new(cx - 3, cy, 6, 1), ink),
            CapButton::Maximize | CapButton::Restore => {
                paint::fill(window, o, Rect::new(cx - 3, cy, 7, 1), ink);
                paint::fill(window, o, Rect::new(cx, cy - 3, 1, 7), ink);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

/// A push button: Aqua's white gel pill (the default one blue gel), later a
/// white rounded button (the default one in the accent colour).
pub(crate) fn push_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State, m: Mac) {
    let blue = (st.focus || st.down) && st.enabled;
    match m.era {
        Era::Aqua => {
            let base = if blue { m.tint() } else { 0xE6_E6_E6 };
            let base = if st.down { shade(base, -0.15) } else { base };
            let rows = gel(base, r.h);
            let border = if st.enabled {
                if blue {
                    shade(m.tint(), -0.45)
                } else {
                    0x8E_8E_8E
                }
            } else {
                0xC0_C0_C0
            };
            round_rect(
                window,
                o,
                r,
                r.h / 2,
                (true, true),
                Some(hsla(border)),
                &|y| {
                    let c = row(&rows, y);
                    if st.enabled { c } else { c.opacity(0.5) }
                },
            );
        }
        Era::Yosemite | Era::Modern => {
            let radius = if m.era == Era::Modern { 5 } else { 4 };
            let accent = m.selection().0;
            let fill: Rgb = match (blue, st.enabled, st.down) {
                (true, _, true) => shade(accent, -0.15),
                (true, ..) => accent,
                (false, true, true) => shade(m.control_bg(), -0.12),
                (false, ..) => m.control_bg(),
            };
            let border = if blue {
                shade(accent, -0.2)
            } else {
                m.border()
            };
            let rows: Vec<Rgb> = if blue && m.era == Era::Yosemite {
                ramp(&[(0.0, mix(fill, 0xFF_FF_FF, 0.35)), (1.0, fill)], r.h)
            } else {
                vec![fill; usize::try_from(r.h).unwrap_or(0)]
            };
            round_rect(
                window,
                o,
                r,
                radius,
                (true, true),
                Some(hsla(border)),
                &|y| {
                    let c = row(&rows, y);
                    if st.enabled { c } else { c.opacity(0.6) }
                },
            );
            if !blue && !m.dark {
                // The soft shadow line under a button.
                paint::fill(
                    window,
                    o,
                    Rect::new(r.x + radius, r.bottom(), r.w - 2 * radius, 1),
                    hsla(0x00_00_00).opacity(0.08),
                );
            }
        }
    }
}

/// The caption colour of a push button: on the accent default button
/// (Yosemite and later) white, or black on a light accent; black on Aqua's
/// gel.
pub(crate) fn button_text(m: Mac, st: State) -> Rgb {
    match (st.enabled, (st.focus || st.down) && m.era != Era::Aqua) {
        (false, _) => m.text_disabled(),
        (true, true) => m.selection().1,
        (true, false) => m.text(),
    }
}

/// The text colour of a control.
pub(crate) fn text_color(m: Mac, enabled: bool) -> Rgb {
    if enabled { m.text() } else { m.text_disabled() }
}

/// The accent of a checked box, a radio button or a pop-up button cap, and
/// the ink on it: Aqua's gel tint (its inks are dark), later the accent
/// with readable ink.
fn accent_of(m: Mac) -> (Rgb, Rgb) {
    if m.era == Era::Aqua {
        (m.tint(), 0x00_00_00)
    } else {
        m.selection()
    }
}

/// A check box (13x13): Aqua's glossy box (blue gel when checked, with a
/// dark tick), later a rounded box filled with the accent and a white tick
/// (black on a light accent).
pub(crate) fn check(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    checked: bool,
    st: State,
    m: Mac,
) {
    let r = Rect::new(x, y, 13, 13);
    let on = checked && st.enabled;
    let (accent, on_accent) = accent_of(m);
    let base = if on { accent } else { m.control_bg() };
    let base = if st.down { shade(base, -0.15) } else { base };
    let border = if on { shade(accent, -0.3) } else { m.border() };
    if m.era == Era::Aqua {
        let rows = gel(if on { base } else { 0xEE_EE_EE }, 13);
        round_rect(window, o, r, 2, (true, true), Some(hsla(border)), &|yy| {
            row(&rows, yy)
        });
    } else {
        round_rect(window, o, r, 3, (true, true), Some(hsla(border)), &|_| {
            hsla(base)
        });
    }
    if checked {
        let ink = hsla(match (m.era, st.enabled) {
            (_, false) => m.text_disabled(),
            (Era::Aqua, true) => 0x00_00_00,
            (_, true) => on_accent,
        });
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
    }
}

/// A radio button (13x13): glossy in Aqua (blue gel with a dark dot when
/// selected), later the accent with a white dot (black on a light accent).
pub(crate) fn radio(
    window: &mut Window,
    o: Point<Pixels>,
    (x, y): (i32, i32),
    selected: bool,
    st: State,
    m: Mac,
) {
    let on = selected && st.enabled;
    let (accent, on_accent) = accent_of(m);
    let base = if on { accent } else { m.control_bg() };
    let base = if st.down { shade(base, -0.15) } else { base };
    let rim = hsla(if on { shade(accent, -0.3) } else { m.border() });
    if m.era == Era::Aqua {
        let rows = gel(if on { base } else { 0xEE_EE_EE }, 13);
        disc(window, o, (x, y), 13, Some(rim), &|yy| row(&rows, yy));
    } else {
        disc(window, o, (x, y), 13, Some(rim), &|_| hsla(base));
    }
    if selected {
        let ink = hsla(match (m.era, st.enabled) {
            (_, false) => m.text_disabled(),
            (Era::Aqua, true) => 0x10_10_10,
            (_, true) => on_accent,
        });
        disc(window, o, (x + 4, y + 4), 5, None, &|_| ink);
    }
}

/// A field border (edit fields, lists, combo boxes): Aqua's inset line with
/// its darker top, later a light line (rounded in the modern design).
pub(crate) fn field(window: &mut Window, o: Point<Pixels>, r: Rect, m: Mac) {
    match m.era {
        Era::Aqua => {
            outline(window, o, r, hsla(0x8E_8E_8E));
            paint::fill(window, o, Rect::new(r.x, r.y, r.w, 1), hsla(0x6E_6E_6E));
        }
        Era::Yosemite => outline(window, o, r, hsla(m.border())),
        Era::Modern => round_border(window, o, r, 3, (true, true), hsla(m.border())),
    }
}

/// A scroll bar arrow or thumb. Aqua: glossy white arrows and a blue gel
/// thumb; later: no arrows (overlay scroll bars; the arrow squares are
/// painted as track) and a thin grey pill.
pub(crate) fn scroll_button(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    arrow: Option<Dir>,
    st: State,
    vertical: bool,
    m: Mac,
) {
    match (m.era, arrow) {
        (Era::Aqua, Some(dir)) => {
            let rows = gel(if st.down { 0xB0_B0_B0 } else { 0xE8_E8_E8 }, r.h);
            for (y, c) in (0_i32..).zip(&rows) {
                paint::fill(window, o, Rect::new(r.x, r.y + y, r.w, 1), hsla(*c));
            }
            outline(window, o, r, hsla(0xA0_A0_A0));
            let ink = hsla(if st.enabled { 0x20_20_20 } else { 0xA6_A6_A6 });
            triangle(window, o, r, dir, ink);
        }
        (Era::Aqua, None) => {
            let thumb = r.inset(1);
            let n = if vertical { thumb.w } else { thumb.h };
            let cols = gel(m.tint(), n);
            let radius = n / 2;
            round_rect(
                window,
                o,
                thumb,
                radius,
                (true, true),
                Some(hsla(shade(m.tint(), -0.45))),
                &|y| {
                    if vertical {
                        hsla(m.tint())
                    } else {
                        row(&cols, y)
                    }
                },
            );
            if vertical {
                for (i, c) in (0_i32..).zip(&cols) {
                    if i == 0 || i == n - 1 {
                        continue;
                    }
                    paint::fill(
                        window,
                        o,
                        Rect::new(thumb.x + i, thumb.y + radius, 1, thumb.h - 2 * radius),
                        hsla(*c),
                    );
                }
            }
        }
        // Overlay scroll bars have no arrows: the arrow squares (still
        // line steps) continue the track.
        (_, Some(_)) => scroll_track(window, o, r, vertical, m),
        (_, None) => {
            let thick = if st.hot || st.down { 9 } else { 7 };
            let pill = if vertical {
                Rect::new(r.x + (r.w - thick) / 2, r.y + 2, thick, r.h - 4)
            } else {
                Rect::new(r.x + 2, r.y + (r.h - thick) / 2, r.w - 4, thick)
            };
            let c = if m.dark {
                hsla(0xFF_FF_FF).opacity(0.45)
            } else {
                hsla(0x00_00_00).opacity(0.42)
            };
            round_rect(window, o, pill, thick / 2, (true, true), None, &|_| c);
        }
    }
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

/// The track of a scroll bar: Aqua's grooved light track, later the window
/// background (overlay scroll bars).
pub(crate) fn scroll_track(window: &mut Window, o: Point<Pixels>, r: Rect, vertical: bool, m: Mac) {
    if m.era == Era::Aqua {
        let n = if vertical { r.w } else { r.h };
        let cols = ramp(
            &[(0.0, 0xC4_C4_C4), (0.3, 0xE8_E8_E8), (1.0, 0xF6_F6_F6)],
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
    } else {
        let bg = if m.dark { 0x2B_2B_2B } else { 0xFA_FA_FA };
        paint::fill(window, o, r, hsla(bg));
    }
}

/// The arrow part of a combo box: Aqua's blue gel end cap, later the
/// accent-coloured (light: blue) double-arrow cap of a pop-up button.
pub(crate) fn drop_button(window: &mut Window, o: Point<Pixels>, r: Rect, st: State, m: Mac) {
    let (accent, on_accent) = accent_of(m);
    let base = if st.enabled { accent } else { 0xC8_C8_C8 };
    let base = if st.down { shade(base, -0.15) } else { base };
    let cap = r.inset(1);
    if m.era == Era::Aqua {
        let rows = gel(base, cap.h);
        round_rect(
            window,
            o,
            cap,
            3,
            (true, true),
            Some(hsla(shade(base, -0.45))),
            &|y| row(&rows, y),
        );
    } else {
        round_rect(window, o, cap, 3, (true, true), None, &|_| hsla(base));
    }
    let ink = hsla(match (m.era, st.enabled) {
        (Era::Aqua, _) => 0x10_10_10,
        (_, true) => on_accent,
        (_, false) => 0xFF_FF_FF,
    });
    // The up-and-down chevrons of a macOS pop-up button.
    let (cx, cy) = (cap.x + cap.w / 2, cap.y + cap.h / 2);
    for k in 0..3 {
        paint::fill(
            window,
            o,
            Rect::new(cx - 2 + k, cy - 2 - k, 5 - 2 * k, 1),
            ink,
        );
        paint::fill(
            window,
            o,
            Rect::new(cx - 2 + k, cy + 2 + k, 5 - 2 * k, 1),
            ink,
        );
    }
}

/// A progress bar: Aqua's striped blue gel bar, later a thin rounded bar in
/// the accent colour.
pub(crate) fn progress(
    window: &mut Window,
    o: Point<Pixels>,
    r: Rect,
    span: Option<(i32, i32)>,
    m: Mac,
) {
    if m.era == Era::Aqua {
        round_rect(
            window,
            o,
            r,
            r.h / 2,
            (true, true),
            Some(hsla(0x8E_8E_8E)),
            &|_| hsla(0xF4_F4_F4),
        );
        if let Some((x, w)) = span.filter(|&(_, w)| w > 0) {
            let bar = Rect::new(r.x + 1 + x, r.y + 1, w.min(r.w - 2 - x), r.h - 2);
            let rows = gel(m.tint(), bar.h);
            round_rect(window, o, bar, bar.h / 2, (true, true), None, &|y| {
                row(&rows, y)
            });
            // The diagonal stripes of the "barber pole".
            let light = hsla(0xFF_FF_FF).opacity(0.25);
            for y in 0..bar.h {
                let mut sx = -(y % 12);
                while sx < bar.w {
                    let q = Rect::new(
                        bar.x + sx.max(0),
                        bar.y + y,
                        (6 + sx.min(0)).min(bar.w - sx.max(0)),
                        1,
                    );
                    if q.w > 0 {
                        paint::fill(window, o, q, light);
                    }
                    sx += 12;
                }
            }
        }
        return;
    }
    let t = 6;
    let track = Rect::new(r.x + 1, r.y + (r.h - t) / 2, r.w - 2, t);
    let tc = hsla(if m.dark { 0x50_50_50 } else { 0xDD_DD_DD });
    round_rect(window, o, track, t / 2, (true, true), None, &|_| tc);
    if let Some((x, w)) = span.filter(|&(_, w)| w > 0) {
        let bar = Rect::new(track.x + x, track.y, w.min(track.w - x), t);
        let c = hsla(m.tint());
        round_rect(window, o, bar, t / 2, (true, true), None, &|_| c);
    }
}

/// A slider: a thin rounded track (in the accent up to the knob from Big
/// Sur on) and a round knob (blue gel in Aqua, white later).
pub(crate) fn trackbar(
    window: &mut Window,
    o: Point<Pixels>,
    channel: Rect,
    thumb: Rect,
    st: State,
    m: Mac,
) {
    let mid = channel.y + channel.h / 2;
    let track = Rect::new(channel.x, mid - 2, channel.w, 4);
    let tc = hsla(if m.dark { 0x5A_5A_5A } else { 0xB5_B5_B5 });
    round_rect(window, o, track, 2, (true, true), None, &|_| tc);
    let cxp = thumb.x + thumb.w / 2;
    if m.era == Era::Modern && st.enabled {
        let fill = Rect::new(channel.x, mid - 2, (cxp - channel.x).max(0), 4);
        let ac = hsla(m.tint());
        round_rect(window, o, fill, 2, (true, true), None, &|_| ac);
    }
    let d = 15;
    let at = (cxp - d / 2, mid - d / 2);
    if m.era == Era::Aqua {
        let base = if st.enabled { m.tint() } else { 0xC8_C8_C8 };
        let rows = gel(base, d);
        disc(window, o, at, d, Some(hsla(shade(base, -0.45))), &|y| {
            row(&rows, y)
        });
    } else {
        let knob = if m.dark { 0xCC_CC_CC } else { 0xFF_FF_FF };
        let knob = if st.down { shade(knob, -0.1) } else { knob };
        disc(window, o, at, d, Some(hsla(m.border())), &|_| hsla(knob));
    }
}

/// A group box: a rounded box, slightly darker inside (no line in Aqua's
/// later releases; here a light line, as 10.0 to 10.9 drew it).
pub(crate) fn group(window: &mut Window, o: Point<Pixels>, r: Rect, m: Mac) {
    let radius = if m.era == Era::Modern { 6 } else { 4 };
    let tint = if m.dark {
        hsla(0xFF_FF_FF).opacity(0.04)
    } else {
        hsla(0x00_00_00).opacity(0.04)
    };
    let border = hsla(if m.dark { 0x48_48_48 } else { 0xC8_C8_C8 });
    round_rect(window, o, r, radius, (true, true), Some(border), &|_| tint);
}

/// A hot, pressed or checked toolbar button.
pub(crate) fn tool_button(window: &mut Window, o: Point<Pixels>, r: Rect, down: bool, m: Mac) {
    let c = match (m.dark, down) {
        (false, true) => hsla(0x00_00_00).opacity(0.15),
        (false, false) => hsla(0x00_00_00).opacity(0.07),
        (true, true) => hsla(0xFF_FF_FF).opacity(0.2),
        (true, false) => hsla(0xFF_FF_FF).opacity(0.1),
    };
    round_rect(window, o, r, 5, (true, true), None, &|_| c);
}

// ---------------------------------------------------------------------------
// Menus
// ---------------------------------------------------------------------------

/// The background of a popup menu.
pub(crate) fn menu_bg(m: Mac) -> Rgb {
    match (m.era, m.dark) {
        (_, true) => 0x2A_2A_2A,
        (Era::Aqua, false) => 0xF8_F8_F8,
        (_, false) => 0xF0_F0_F0,
    }
}

/// A popup menu: rounded (bottom corners in Aqua, all from Big Sur on).
pub(crate) fn menu_popup(window: &mut Window, o: Point<Pixels>, r: Rect, m: Mac) {
    let bg = hsla(menu_bg(m));
    let border = hsla(if m.dark { 0x50_50_50 } else { 0xC0_C0_C0 });
    match m.era {
        Era::Aqua => round_rect(window, o, r, 4, (false, true), Some(border), &|_| bg),
        Era::Yosemite => round_rect(window, o, r, 5, (true, true), Some(border), &|_| bg),
        Era::Modern => round_rect(window, o, r, 7, (true, true), Some(border), &|_| bg),
    }
}

/// The highlight of a hot menu item or menu bar title: Aqua's blue gradient
/// bar, Yosemite's accent bar, Big Sur's rounded inset accent.
pub(crate) fn menu_highlight(window: &mut Window, o: Point<Pixels>, r: Rect, m: Mac) {
    // The scheme's selection colour, which the item text is chosen against.
    let sel = m.selection().0;
    match m.era {
        Era::Aqua => {
            for (y, c) in (0_i32..).zip(&aqua_highlight_rows(sel, r.h)) {
                paint::fill(window, o, Rect::new(r.x, r.y + y, r.w, 1), hsla(*c));
            }
        }
        Era::Yosemite => paint::fill(window, o, r, hsla(sel)),
        Era::Modern => {
            let q = Rect::new(r.x + 4, r.y + 1, r.w - 8, r.h - 2);
            let c = hsla(sel);
            round_rect(window, o, q, 4, (true, true), None, &|_| c);
        }
    }
}

/// The rows of Aqua's menu highlight `h` high on the selection colour
/// `sel`: a gradient from `sel` down to a darker shade, never lighter than
/// `sel`, so the item text keeps its contrast on every row.
fn aqua_highlight_rows(sel: Rgb, h: i32) -> Vec<Rgb> {
    ramp(&[(0.0, sel), (1.0, shade(sel, -0.2))], h)
}

/// A separator of a popup menu.
pub(crate) fn menu_separator(window: &mut Window, o: Point<Pixels>, row: Rect, y: i32, m: Mac) {
    let c = hsla(if m.dark { 0x48_48_48 } else { 0xD6_D6_D6 });
    paint::fill(window, o, Rect::new(row.x + 10, y, row.w - 20, 1), c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_mode_tints_with_the_brighter_accent() {
        let light = Look {
            style: WinStyle::MacModern,
            ..Look::default()
        };
        let dark = Look {
            dark: true,
            ..light
        };
        let (Some(l), Some(d)) = (Mac::of(light), Mac::of(dark)) else {
            panic!("a macOS style has a Mac skin");
        };
        assert_eq!(l.tint(), light.accent_rgb());
        assert_eq!(d.tint(), dark.drawn_accent());
        assert_ne!(d.tint(), dark.accent_rgb());
        // White on the selection still reads in the dark mode.
        let (sel, text) = d.selection();
        assert!(contrast(text, sel) >= MIN_CAPTION_CONTRAST);
    }

    #[test]
    fn selections_darken_the_drawn_accent_only_as_far_as_white_needs() {
        use sn_ui_core::winstyle::MAC_ACCENTS;
        for a in MAC_ACCENTS {
            for dark in [false, true] {
                let look = Look {
                    style: WinStyle::MacModern,
                    mac_accent: a.rgb(),
                    dark,
                    ..Look::default()
                };
                let Some(m) = Mac::of(look) else {
                    panic!("a macOS style has a Mac skin");
                };
                let t = m.tint();
                assert_eq!(t, if dark { a.dark_rgb() } else { a.rgb() });
                let (fill, ink) = m.selection();
                assert!(contrast(ink, fill) >= MIN_CAPTION_CONTRAST, "{a:?} {dark}");
                if ink != 0xFF_FF_FF {
                    assert_eq!(fill, t, "{a:?} {dark}: black text on the accent itself");
                    continue;
                }
                // The lightest shade of the drawn accent in 1% steps that
                // white reads on: one step lighter would not do.
                let k = (0..=25)
                    .find(|&k| shade(t, -0.01 * f64::from(k)) == fill)
                    .unwrap_or_else(|| panic!("{a:?} {dark}: fill is a shade of the tint"));
                if k > 0 {
                    let lighter = shade(t, -0.01 * f64::from(k - 1));
                    assert!(contrast(0xFF_FF_FF, lighter) < MIN_CAPTION_CONTRAST);
                }
            }
        }
    }

    #[test]
    fn white_reads_on_every_row_of_the_aqua_menu_highlight() {
        for graphite in [false, true] {
            let m = Mac {
                era: Era::Aqua,
                dark: false,
                accent: 0,
                graphite,
            };
            let (sel, ink) = m.selection();
            for h in 12..=28 {
                for c in aqua_highlight_rows(sel, h) {
                    assert!(contrast(ink, c) >= MIN_CAPTION_CONTRAST, "{c:06X} h={h}");
                }
            }
        }
    }

    #[test]
    fn aqua_ignores_the_accent_and_graphite_greys_it() {
        let blue = Mac {
            era: Era::Aqua,
            dark: false,
            accent: 0xFF_52_57,
            graphite: false,
        };
        assert_eq!(blue.tint(), 0x38_75_D7);
        let graphite = Mac {
            graphite: true,
            ..blue
        };
        assert_ne!(graphite.tint(), blue.tint());
        let modern = Mac {
            era: Era::Modern,
            ..blue
        };
        assert_eq!(modern.tint(), 0xFF_52_57);
    }

    #[test]
    fn a_long_title_keeps_its_start_and_ends_with_dots() {
        let width = |t: &str| 5 * i32::try_from(t.chars().count()).unwrap_or(0);
        assert_eq!(fit_title("Spectrum of a.wav", 50, &width), "Spectru...");
        assert!(width(&fit_title("Spectrum of a.wav", 50, &width)) <= 50);
        assert_eq!(fit_title("abc", 5, &width), "...");
        assert_eq!(
            fit_title("\u{e9}t\u{e9} long", 30, &width),
            "\u{e9}t\u{e9}..."
        );
    }

    #[test]
    fn disc_rows_are_single_runs_inside_the_rim() {
        for d in [5, 12, 13, 14, 15] {
            for py in 0..d {
                let (outer, inner) = disc_row(d, py);
                let (a, b) = outer.unwrap_or((0, -1));
                assert!(a <= b, "d={d} row {py} is empty");
                assert_eq!(a, d - 1 - b, "d={d} row {py} is symmetric");
                if let Some((ia, ib)) = inner {
                    assert!(a < ia && ib < b, "d={d} row {py}: rim at both ends");
                }
            }
            assert_eq!(disc_row(d, d / 2).1.is_some(), d > 2);
        }
    }

    #[test]
    fn accent_text_reads_on_every_mac_accent() {
        use sn_ui_core::winstyle::MAC_ACCENTS;
        for a in MAC_ACCENTS {
            for era in [Era::Yosemite, Era::Modern] {
                let m = Mac {
                    era,
                    dark: false,
                    accent: a.rgb(),
                    graphite: false,
                };
                let (fill, ink) = m.selection();
                assert!(contrast(ink, fill) >= MIN_CAPTION_CONTRAST, "{a:?}");
                let st = State {
                    hot: false,
                    down: false,
                    focus: true,
                    enabled: true,
                };
                assert_eq!(button_text(m, st), ink, "{a:?}");
            }
        }
    }

    #[test]
    fn gel_rows_run_light_to_base_to_glow() {
        let rows = gel(0x38_75_D7, 20);
        assert_eq!(rows.len(), 20);
        assert_ne!(rows.first(), rows.get(10));
    }

    #[test]
    fn the_default_button_text_is_white_after_aqua() {
        let st = State {
            hot: false,
            down: false,
            focus: true,
            enabled: true,
        };
        let aqua = Mac {
            era: Era::Aqua,
            dark: false,
            accent: 0x00_7A_FF,
            graphite: false,
        };
        assert_eq!(button_text(aqua, st), 0x00_00_00);
        assert_eq!(
            button_text(
                Mac {
                    era: Era::Modern,
                    ..aqua
                },
                st
            ),
            0xFF_FF_FF
        );
    }
}
