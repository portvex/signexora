//! The active Windows style (Options > Windows style): which version of
//! Windows the kit draws, its system colours and its metrics.
//!
//! The style is set once per change with [`apply`], which also sets the
//! colour scheme. Widgets read it with [`active`]; geometry code that has no
//! GPUI context (the MDI manager, menu layout) reads the metrics with
//! [`metrics`]. Both come from a thread-local copy, since GPUI runs every view
//! on the main thread (and each test on its own thread).

use std::cell::Cell;

use sn_ui_core::theme::ThemeChoice;
use sn_ui_core::winstyle::{Look, WinStyle, XpColor, caption_text_on};

use super::colors::{Rgb, Scheme, contrast, mix};
use super::skin_mac::Mac;

/// The style being drawn: the chosen look and the style actually drawn (a
/// high-contrast theme draws the classic look).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Active {
    /// The chosen look (with the variants of every style).
    pub(crate) look: Look,
    /// The style drawn.
    pub(crate) style: WinStyle,
    /// The menu is in the macOS menu bar (a macOS look on a Mac), so the
    /// window draws no menu bar of its own.
    pub(crate) native_menu: bool,
}

impl Active {
    /// Windows 95, the look of Phase 10 (and of the tests).
    pub(crate) const WIN95: Self = Self {
        look: Look::WIN95,
        style: WinStyle::Win95,
        native_menu: false,
    };

    /// Whether classic 3D controls are drawn (95, 98 or high contrast).
    pub(crate) const fn classic(self) -> bool {
        self.style.is_classic()
    }
}

thread_local! {
    static ACTIVE: Cell<Active> = const { Cell::new(Active::WIN95) };
}

/// The active style.
pub(crate) fn active() -> Active {
    ACTIVE.with(Cell::get)
}

/// Make `look` (with the theme `theme`) the active style.
pub(crate) fn set(look: Look, theme: ThemeChoice) {
    let style = look.drawn_style(theme);
    let a = Active {
        look,
        style,
        // Tests run without the system menu bar, in every look.
        native_menu: cfg!(all(target_os = "macos", not(test))) && style.is_mac(),
    };
    ACTIVE.with(|c| c.set(a));
}

/// The metrics that differ between the styles, in logical pixels at 96 dpi.
/// Window contents keep their layouts in every style; only the frames,
/// captions and menus change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Metrics {
    /// Width of the sizing frame of an MDI child (`SM_CXFRAME`).
    pub(crate) frame: i32,
    /// Height of a caption (`SM_CYCAPTION`).
    pub(crate) caption: i32,
    /// Size of the minimize and maximize caption buttons.
    pub(crate) btn: (i32, i32),
    /// Width of the close button (wider from Vista on).
    pub(crate) close_w: i32,
    /// Gaps between close and maximize, and between maximize and minimize.
    pub(crate) btn_gaps: (i32, i32),
    /// Space between the close button and the frame.
    pub(crate) btn_margin: i32,
    /// Top of the caption buttons from the window's top edge; `None` centres
    /// them in the caption.
    pub(crate) btn_top: Option<i32>,
    /// Width of the frame of a dialog (not sizable).
    pub(crate) dlg_frame: i32,
    /// Height of the menu bar.
    pub(crate) bar_h: i32,
    /// Height of a menu item.
    pub(crate) item_h: i32,
    /// The caption buttons sit at the left in the order close, minimize,
    /// zoom (macOS), not at the right.
    pub(crate) btn_left: bool,
}

impl Metrics {
    /// Windows 95 and 98.
    pub(crate) const CLASSIC: Self = Self {
        frame: 4,
        caption: 18,
        btn: (16, 14),
        close_w: 16,
        btn_gaps: (2, 0),
        btn_margin: 2,
        btn_top: None,
        dlg_frame: 3,
        bar_h: 18,
        item_h: 17,
        btn_left: false,
    };

    /// Windows XP (Luna).
    pub(crate) const XP: Self = Self {
        frame: 4,
        caption: 25,
        btn: (21, 21),
        close_w: 21,
        btn_gaps: (2, 2),
        btn_margin: 2,
        btn_top: None,
        dlg_frame: 3,
        bar_h: 20,
        item_h: 18,
        btn_left: false,
    };

    /// Windows Vista and 7 (an MDI child has the Basic frame).
    pub(crate) const AERO: Self = Self {
        frame: 8,
        caption: 22,
        btn: (25, 17),
        close_w: 43,
        btn_gaps: (0, 0),
        btn_margin: 0,
        btn_top: Some(1),
        dlg_frame: 8,
        bar_h: 20,
        item_h: 22,
        btn_left: false,
    };

    /// Windows 8 and 10.
    pub(crate) const FLAT: Self = Self {
        frame: 6,
        caption: 24,
        btn: (29, 22),
        close_w: 29,
        btn_gaps: (0, 0),
        btn_margin: 0,
        btn_top: Some(1),
        dlg_frame: 6,
        bar_h: 20,
        item_h: 22,
        btn_left: false,
    };

    /// Windows 11.
    pub(crate) const FLUENT: Self = Self {
        frame: 4,
        caption: 28,
        btn: (36, 26),
        close_w: 36,
        btn_gaps: (0, 0),
        btn_margin: 0,
        btn_top: Some(1),
        dlg_frame: 4,
        bar_h: 24,
        item_h: 26,
        btn_left: false,
    };

    /// The macOS looks: one-pixel window edges (with a few pixels to grab for
    /// sizing), the traffic lights at the left.
    pub(crate) const MAC: Self = Self {
        frame: 3,
        caption: 22,
        btn: (14, 14),
        close_w: 14,
        btn_gaps: (7, 7),
        btn_margin: 6,
        btn_top: None,
        dlg_frame: 3,
        bar_h: 22,
        item_h: 20,
        btn_left: true,
    };

    /// macOS Big Sur and later: a taller title bar, 12 px traffic lights.
    pub(crate) const MAC_MODERN: Self = Self {
        caption: 28,
        btn: (12, 12),
        close_w: 12,
        btn_gaps: (8, 8),
        btn_margin: 8,
        item_h: 22,
        ..Self::MAC
    };

    /// GNOME's Adwaita: a tall header bar with round title buttons.
    pub(crate) const ADWAITA: Self = Self {
        frame: 3,
        caption: 34,
        btn: (24, 24),
        close_w: 24,
        btn_gaps: (6, 6),
        btn_margin: 6,
        btn_top: None,
        dlg_frame: 3,
        bar_h: 26,
        item_h: 26,
        btn_left: false,
    };

    /// KDE Plasma's Breeze.
    pub(crate) const BREEZE: Self = Self {
        frame: 3,
        caption: 28,
        btn: (18, 18),
        close_w: 18,
        btn_gaps: (6, 6),
        btn_margin: 6,
        btn_top: None,
        dlg_frame: 3,
        bar_h: 24,
        item_h: 24,
        btn_left: false,
    };

    /// GNOME 2 with Metacity's Clearlooks frames.
    pub(crate) const CLEARLOOKS: Self = Self {
        frame: 4,
        caption: 22,
        btn: (18, 18),
        close_w: 18,
        btn_gaps: (2, 2),
        btn_margin: 3,
        btn_top: None,
        dlg_frame: 4,
        bar_h: 22,
        item_h: 22,
        btn_left: false,
    };

    /// Motif's mwm: thick frames with corner handles, minimize and maximize
    /// at the right, no close button (the window menu at the left closes).
    pub(crate) const MOTIF: Self = Self {
        frame: 6,
        caption: 22,
        btn: (22, 22),
        close_w: 0,
        btn_gaps: (0, 0),
        btn_margin: 0,
        btn_top: None,
        dlg_frame: 6,
        bar_h: 26,
        item_h: 22,
        btn_left: false,
    };

    /// The metrics of `style`.
    pub(crate) const fn of(style: WinStyle) -> Self {
        match style {
            WinStyle::Win95 | WinStyle::Win98 => Self::CLASSIC,
            WinStyle::Xp => Self::XP,
            WinStyle::Vista | WinStyle::Win7 => Self::AERO,
            WinStyle::Win8 | WinStyle::Win10 => Self::FLAT,
            WinStyle::Win11 => Self::FLUENT,
            WinStyle::MacAqua | WinStyle::MacYosemite => Self::MAC,
            WinStyle::MacModern => Self::MAC_MODERN,
            WinStyle::LinuxAdwaita => Self::ADWAITA,
            WinStyle::LinuxBreeze => Self::BREEZE,
            WinStyle::LinuxClearlooks => Self::CLEARLOOKS,
            WinStyle::LinuxMotif => Self::MOTIF,
        }
    }
}

/// The metrics of the active style. With the menu in the macOS menu bar the
/// window has no menu bar of its own.
pub(crate) fn metrics() -> Metrics {
    let a = active();
    let m = Metrics::of(a.style);
    if a.native_menu {
        Metrics { bar_h: 0, ..m }
    } else {
        m
    }
}

/// Whether the menu is in the macOS menu bar.
pub(crate) fn native_menu() -> bool {
    active().native_menu
}

/// The system colours of a themed style (XP to 10). The classic styles take
/// theirs from the appearance scheme.
pub(crate) fn themed_scheme(look: Look) -> Scheme {
    match look.style {
        WinStyle::Win95 | WinStyle::Win98 => Scheme::STANDARD,
        WinStyle::Xp => xp_scheme(look.xp),
        WinStyle::MacAqua | WinStyle::MacYosemite | WinStyle::MacModern => mac_scheme(look),
        WinStyle::LinuxAdwaita
        | WinStyle::LinuxBreeze
        | WinStyle::LinuxClearlooks
        | WinStyle::LinuxMotif => super::skin_linux::Lin::of(look)
            .map_or(Scheme::STANDARD, super::skin_linux::Lin::scheme),
        WinStyle::Vista | WinStyle::Win7 => Scheme {
            face: 0xF0_F0_F0,
            highlight: 0xFF_FF_FF,
            light: 0xE3_E3_E3,
            shadow: 0xA0_A0_A0,
            dark_shadow: 0x69_69_69,
            window: 0xFF_FF_FF,
            text: 0x00_00_00,
            active_caption: 0x99_B4_D1,
            active_caption_text: 0x00_00_00,
            inactive_caption: 0xBF_CD_DB,
            inactive_caption_text: 0x43_4E_54,
            selection: 0x33_99_FF,
            selection_text: 0xFF_FF_FF,
            info: 0xFF_FF_E1,
            info_text: 0x00_00_00,
            app_workspace: 0xAB_AB_AB,
        },
        WinStyle::Win8 | WinStyle::Win10 if look.dark && look.style == WinStyle::Win10 => {
            let accent = look.drawn_accent();
            Scheme {
                face: 0x2B_2B_2B,
                highlight: 0x5A_5A_5A,
                light: 0x40_40_40,
                shadow: 0x1A_1A_1A,
                dark_shadow: 0x00_00_00,
                window: 0x19_19_19,
                text: 0xFF_FF_FF,
                active_caption: 0x20_20_20,
                active_caption_text: 0xFF_FF_FF,
                inactive_caption: 0x2B_2B_2B,
                inactive_caption_text: 0xA0_A0_A0,
                selection: accent,
                // White, or black on a light accent (Gold, Teal).
                selection_text: caption_text_on(accent),
                info: 0x2B_2B_2B,
                info_text: 0xFF_FF_FF,
                app_workspace: 0x1F_1F_1F,
            }
        }
        WinStyle::Win11 => win11_scheme(look),
        WinStyle::Win8 | WinStyle::Win10 => Scheme {
            face: 0xF0_F0_F0,
            highlight: 0xFF_FF_FF,
            light: 0xE3_E3_E3,
            shadow: 0xA0_A0_A0,
            dark_shadow: 0x69_69_69,
            window: 0xFF_FF_FF,
            text: 0x00_00_00,
            active_caption: look.drawn_accent(),
            active_caption_text: caption_text_on(look.drawn_accent()),
            inactive_caption: 0xFF_FF_FF,
            inactive_caption_text: 0x99_99_99,
            selection: 0x00_78_D7,
            selection_text: 0xFF_FF_FF,
            info: 0xFF_FF_FF,
            info_text: 0x57_57_57,
            app_workspace: 0xAB_AB_AB,
        },
    }
}

/// The system colours of Windows 11, light or dark.
fn win11_scheme(look: Look) -> Scheme {
    if look.dark {
        Scheme {
            face: 0x20_20_20,
            highlight: 0x45_45_45,
            light: 0x33_33_33,
            shadow: 0x1A_1A_1A,
            dark_shadow: 0x00_00_00,
            window: 0x2B_2B_2B,
            text: 0xFF_FF_FF,
            active_caption: 0x20_20_20,
            active_caption_text: 0xFF_FF_FF,
            inactive_caption: 0x2B_2B_2B,
            inactive_caption_text: 0x78_78_78,
            selection: shade(look.drawn_accent(), 0.45),
            selection_text: 0x00_00_00,
            info: 0x2B_2B_2B,
            info_text: 0xFF_FF_FF,
            app_workspace: 0x1C_1C_1C,
        }
    } else {
        Scheme {
            face: 0xF3_F3_F3,
            highlight: 0xFF_FF_FF,
            light: 0xE5_E5_E5,
            shadow: 0xA0_A0_A0,
            dark_shadow: 0x69_69_69,
            window: 0xFF_FF_FF,
            text: 0x1A_1A_1A,
            active_caption: 0xEE_F4_F9,
            active_caption_text: 0x1A_1A_1A,
            inactive_caption: 0xF3_F3_F3,
            inactive_caption_text: 0xA0_A0_A0,
            selection: shade(look.drawn_accent(), -0.2),
            selection_text: 0xFF_FF_FF,
            info: 0xF9_F9_F9,
            info_text: 0x1A_1A_1A,
            app_workspace: 0xAB_AB_AB,
        }
    }
}

/// The colours of the macOS looks (window background, text, selection).
/// The dark appearance and the selection come from the skin's
/// [`Mac`](super::skin_mac::Mac), so selections match the controls.
fn mac_scheme(look: Look) -> Scheme {
    let Some(mac) = Mac::of(look) else {
        return Scheme::STANDARD;
    };
    let (selection, selection_text) = mac.selection();
    if mac.dark {
        return Scheme {
            face: 0x0032_3232,
            highlight: 0x50_50_50,
            light: 0x40_40_40,
            shadow: 0x24_24_24,
            dark_shadow: 0x00_00_00,
            window: 0x1E_1E_1E,
            text: 0xDF_DF_DF,
            active_caption: 0x38_38_38,
            active_caption_text: 0xDF_DF_DF,
            inactive_caption: Mac::INACTIVE_TITLE_DARK,
            // At least 4.5:1 on the inactive title bar.
            inactive_caption_text: 0x9A_9A_9A,
            selection,
            selection_text,
            info: 0x0032_3232,
            info_text: 0xDF_DF_DF,
            app_workspace: 0x1C_1C_1C,
        };
    }
    let aqua = look.style == WinStyle::MacAqua;
    Scheme {
        face: if aqua { 0xE8_E8_E8 } else { 0xEC_EC_EC },
        highlight: 0xFF_FF_FF,
        light: 0xF4_F4_F4,
        shadow: 0xB4_B4_B4,
        dark_shadow: 0x80_80_80,
        window: 0xFF_FF_FF,
        text: 0x00_00_00,
        active_caption: 0xE0_E0_E0,
        active_caption_text: 0x00_00_00,
        // The colour the skin paints behind an inactive title (Aqua's
        // pinstriped ramp at its middle).
        inactive_caption: if aqua {
            Mac::AQUA_TITLE_MID
        } else {
            Mac::INACTIVE_TITLE_LIGHT
        },
        // At least 4.5:1 on every row of the inactive title bar: Aqua's
        // ramp reaches 0xC0 on its darkest pinstripe.
        inactive_caption_text: if aqua { 0x4A_4A_4A } else { 0x6E_6E_6E },
        selection,
        selection_text,
        info: 0xFF_FF_C6,
        info_text: 0x00_00_00,
        app_workspace: 0xC8_C8_C8,
    }
}

/// The system colours of Luna (XP's `COLOR_*` values for each scheme).
fn xp_scheme(c: XpColor) -> Scheme {
    let base = Scheme {
        face: 0xEC_E9_D8,
        highlight: 0xFF_FF_FF,
        light: 0xF1_EF_E2,
        shadow: 0xAC_A8_99,
        dark_shadow: 0x0071_6F64,
        window: 0xFF_FF_FF,
        text: 0x00_00_00,
        active_caption: 0x00_54_E3,
        active_caption_text: 0xFF_FF_FF,
        inactive_caption: 0x7A_96_DF,
        inactive_caption_text: 0xD8_E4_F8,
        selection: 0x31_6A_C5,
        selection_text: 0xFF_FF_FF,
        info: 0xFF_FF_E1,
        info_text: 0x00_00_00,
        app_workspace: 0x80_80_80,
    };
    match c {
        XpColor::Blue => base,
        XpColor::Olive => Scheme {
            active_caption: 0x8B_A1_69,
            inactive_caption: 0xBA_C8_A0,
            inactive_caption_text: 0xFF_FF_FF,
            selection: 0x93_A0_70,
            ..base
        },
        XpColor::Silver => Scheme {
            face: 0xE0_DF_E3,
            light: 0xEC_EB_EF,
            shadow: 0x9D_9D_A1,
            dark_shadow: 0x5C_5C_5F,
            active_caption: 0xC0_C0_D8,
            active_caption_text: 0x00_00_00,
            inactive_caption: 0xE3_E2_EA,
            inactive_caption_text: 0x80_80_80,
            selection: 0xB2_B4_BF,
            selection_text: 0x00_00_00,
            ..base
        },
    }
}

/// The right-hand colour of a Windows 98 caption gradient
/// (`COLOR_GRADIENTACTIVECAPTION`, `COLOR_GRADIENTINACTIVECAPTION`): Windows
/// 98's own for the Windows Standard scheme; the caption colour itself for
/// the high-contrast schemes (as Windows sets them, so there is no
/// gradient); for the others a lighter shade of the caption colour, no
/// lighter than keeps the caption text readable.
pub(crate) fn gradient_end(s: &Scheme, active: bool) -> Rgb {
    let (bg, fg) = if active {
        (s.active_caption, s.active_caption_text)
    } else {
        (s.inactive_caption, s.inactive_caption_text)
    };
    if *s == Scheme::STANDARD {
        return if active { 0x10_84_D0 } else { 0xC0_C0_C0 };
    }
    if *s == Scheme::HIGH_CONTRAST_BLACK || *s == Scheme::HIGH_CONTRAST_WHITE {
        return bg;
    }
    // The caption's own contrast, if that is already below the aim, is not
    // made worse.
    let aim = contrast(fg, bg).min(4.5);
    (0..=9)
        .rev()
        .map(|k| shade(bg, 0.05 * f64::from(k)))
        .find(|&c| contrast(fg, c) >= aim)
        .unwrap_or(bg)
}

/// A lighter (positive `t`) or darker (negative `t`) shade of `c`.
pub(crate) fn shade(c: Rgb, t: f64) -> Rgb {
    if t >= 0.0 {
        mix(c, 0xFF_FF_FF, t)
    } else {
        mix(c, 0x00_00_00, -t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_95_keeps_the_phase_10_metrics() {
        let m = Metrics::of(WinStyle::Win95);
        assert_eq!(
            (m.frame, m.caption, m.btn, m.bar_h, m.item_h, m.dlg_frame),
            (4, 18, (16, 14), 18, 17, 3)
        );
    }

    #[test]
    fn high_contrast_draws_classic_and_the_thread_keeps_the_style() {
        set(Look::default(), ThemeChoice::HighContrastDark);
        assert!(active().classic());
        set(Look::default(), ThemeChoice::Classic);
        assert_eq!(active().style, WinStyle::Xp);
        assert_eq!(metrics(), Metrics::XP);
        set(Look::WIN95, ThemeChoice::Classic);
    }

    #[test]
    fn windows_98_gradients_keep_caption_text_readable() {
        for hc in [Scheme::HIGH_CONTRAST_BLACK, Scheme::HIGH_CONTRAST_WHITE] {
            assert_eq!(gradient_end(&hc, true), hc.active_caption, "no gradient");
            assert_eq!(gradient_end(&hc, false), hc.inactive_caption);
        }
        assert_eq!(gradient_end(&Scheme::STANDARD, true), 0x10_84_D0);
        // Schemes other than Windows Standard and high contrast still get a
        // real gradient, not a flat caption.
        let mut gradients = 0;
        for choice in ThemeChoice::ALL {
            for dark in [false, true] {
                let s = Scheme::for_theme(choice, dark);
                if s == Scheme::STANDARD {
                    // Windows 98's own colours, kept as they are.
                    continue;
                }
                for (active, bg, fg) in [
                    (true, s.active_caption, s.active_caption_text),
                    (false, s.inactive_caption, s.inactive_caption_text),
                ] {
                    let end = gradient_end(&s, active);
                    if end != bg {
                        gradients += 1;
                    }
                    assert!(
                        contrast(fg, end) >= contrast(fg, bg).min(4.5),
                        "{choice:?} dark={dark} active={active}"
                    );
                }
            }
        }
        assert!(gradients > 0, "no scheme besides Standard has a gradient");
    }

    #[test]
    fn accent_captions_and_selections_read() {
        use sn_ui_core::winstyle::{ACCENTS, MIN_CAPTION_CONTRAST};
        for a in ACCENTS {
            for (style, dark) in [
                (WinStyle::Win8, false),
                (WinStyle::Win10, false),
                (WinStyle::Win10, true),
            ] {
                let s = themed_scheme(Look {
                    style,
                    dark,
                    accent: a.rgb(),
                    ..Look::default()
                });
                let name = format!("{a:?} {style:?} dark={dark}");
                assert!(
                    contrast(s.selection_text, s.selection) >= MIN_CAPTION_CONTRAST,
                    "selection, {name}"
                );
                assert!(
                    contrast(s.active_caption_text, s.active_caption) >= MIN_CAPTION_CONTRAST,
                    "caption, {name}"
                );
            }
        }
    }

    #[test]
    fn mac_selections_and_inactive_titles_read() {
        use sn_ui_core::winstyle::{AquaColor, MAC_ACCENTS, MIN_CAPTION_CONTRAST};
        for a in MAC_ACCENTS {
            for (style, dark) in [
                (WinStyle::MacAqua, false),
                (WinStyle::MacYosemite, false),
                (WinStyle::MacYosemite, true),
                (WinStyle::MacModern, false),
                (WinStyle::MacModern, true),
            ] {
                let look = Look {
                    style,
                    dark,
                    mac_accent: a.rgb(),
                    ..Look::default()
                };
                let s = themed_scheme(look);
                let name = format!("{a:?} {style:?} dark={dark}");
                assert!(
                    contrast(s.selection_text, s.selection) >= MIN_CAPTION_CONTRAST,
                    "selection, {name}"
                );
                // The skin's controls use the same colours.
                let Some(m) = Mac::of(look) else {
                    panic!("{name} has a Mac skin");
                };
                assert_eq!(m.selection(), (s.selection, s.selection_text), "{name}");
                // The titles read on every row the skin paints behind them.
                for top in 16..=40 {
                    for (active, fg) in [
                        (true, s.active_caption_text),
                        (false, s.inactive_caption_text),
                    ] {
                        for (y, bg) in m.title_rows(top, active).into_iter().enumerate() {
                            assert!(
                                contrast(fg, bg) >= 4.5,
                                "{name} active={active} top={top} row {y}: {bg:06X}"
                            );
                        }
                    }
                    let inactive = m.title_rows(top, false);
                    if style == WinStyle::MacAqua {
                        let (lo, hi) = inactive.iter().fold((0xFF, 0), |(lo, hi), c| {
                            ((c & 0xFF).min(lo), (c & 0xFF).max(hi))
                        });
                        let mid = s.inactive_caption & 0xFF;
                        assert!(
                            lo <= mid && mid <= hi,
                            "{name}: inactive caption in the ramp"
                        );
                    } else {
                        assert!(
                            inactive.iter().all(|&c| c == s.inactive_caption),
                            "{name}: the scheme's inactive caption is what is painted"
                        );
                    }
                }
            }
        }
        for aqua in AquaColor::ALL {
            let s = themed_scheme(Look {
                style: WinStyle::MacAqua,
                aqua,
                dark: true,
                ..Look::default()
            });
            assert_eq!(s.window, 0xFF_FF_FF, "Aqua has no dark appearance");
            assert!(contrast(s.selection_text, s.selection) >= MIN_CAPTION_CONTRAST);
        }
    }

    #[test]
    fn the_xp_schemes_differ_in_their_captions() {
        let caps: Vec<Rgb> = XpColor::ALL
            .iter()
            .map(|c| xp_scheme(*c).active_caption)
            .collect();
        assert_eq!(caps.len(), 3);
        assert!(caps[0] != caps[1] && caps[1] != caps[2]);
    }
}
