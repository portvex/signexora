//! Windows 95 system colours and appearance schemes.
//!
//! Every widget takes its colours from a [`Scheme`]. SigNexora's colour themes map
//! onto Windows-style appearance schemes (Options > Theme): "Windows Standard"
//! (the default), "High Contrast Black" and "High Contrast White" as in
//! Windows 95, and one more scheme per other SigNexora theme. The active scheme,
//! SA's own panel colours and the plot palette are GPUI globals, read with
//! [`scheme`], [`sa_colors`] and [`palette`].

use gpui::{App, Global, Hsla};
use sn_ui_core::theme::{Palette, ThemeChoice, palette_for};

/// A colour as `0xRRGGBB`.
pub(crate) type Rgb = u32;

/// Convert `0xRRGGBB` to a GPUI colour.
pub(crate) fn hsla(c: Rgb) -> Hsla {
    gpui::rgb(c).into()
}

/// The system colours of one appearance scheme (Windows `COLOR_*` names).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Scheme {
    /// `COLOR_3DFACE`: dialogs, buttons, toolbars, menus.
    pub(crate) face: Rgb,
    /// `COLOR_3DHILIGHT`: the bright edge of a 3D border.
    pub(crate) highlight: Rgb,
    /// `COLOR_3DLIGHT`: the second, softer bright edge.
    pub(crate) light: Rgb,
    /// `COLOR_3DSHADOW`: the grey edge of a 3D border; also grey text.
    pub(crate) shadow: Rgb,
    /// `COLOR_3DDKSHADOW`: the black outer edge.
    pub(crate) dark_shadow: Rgb,
    /// `COLOR_WINDOW`: edit fields and list boxes.
    pub(crate) window: Rgb,
    /// `COLOR_WINDOWTEXT` and `COLOR_BTNTEXT`.
    pub(crate) text: Rgb,
    /// `COLOR_ACTIVECAPTION`.
    pub(crate) active_caption: Rgb,
    /// `COLOR_CAPTIONTEXT`.
    pub(crate) active_caption_text: Rgb,
    /// `COLOR_INACTIVECAPTION`.
    pub(crate) inactive_caption: Rgb,
    /// `COLOR_INACTIVECAPTIONTEXT`.
    pub(crate) inactive_caption_text: Rgb,
    /// `COLOR_HIGHLIGHT`: selected items.
    pub(crate) selection: Rgb,
    /// `COLOR_HIGHLIGHTTEXT`.
    pub(crate) selection_text: Rgb,
    /// `COLOR_INFOBK`: tooltips.
    pub(crate) info: Rgb,
    /// `COLOR_INFOTEXT`.
    pub(crate) info_text: Rgb,
    /// `COLOR_APPWORKSPACE`: the MDI client area.
    pub(crate) app_workspace: Rgb,
}

impl Scheme {
    /// Windows 95 "Windows Standard".
    pub(crate) const STANDARD: Self = Self {
        face: 0xC0_C0_C0,
        highlight: 0xFF_FF_FF,
        light: 0xDF_DF_DF,
        shadow: 0x80_80_80,
        dark_shadow: 0x00_00_00,
        window: 0xFF_FF_FF,
        text: 0x00_00_00,
        active_caption: 0x00_00_80,
        active_caption_text: 0xFF_FF_FF,
        inactive_caption: 0x80_80_80,
        inactive_caption_text: 0xC0_C0_C0,
        selection: 0x00_00_80,
        selection_text: 0xFF_FF_FF,
        info: 0xFF_FF_E1,
        info_text: 0x00_00_00,
        app_workspace: 0x80_80_80,
    };

    /// Windows 95 "High Contrast Black".
    pub(crate) const HIGH_CONTRAST_BLACK: Self = Self {
        face: 0x00_00_00,
        highlight: 0xFF_FF_FF,
        light: 0xC0_C0_C0,
        shadow: 0x80_80_80,
        dark_shadow: 0xFF_FF_FF,
        window: 0x00_00_00,
        text: 0xFF_FF_FF,
        active_caption: 0x80_00_80,
        active_caption_text: 0xFF_FF_FF,
        inactive_caption: 0x00_80_00,
        inactive_caption_text: 0xFF_FF_FF,
        selection: 0x80_00_80,
        selection_text: 0xFF_FF_FF,
        info: 0x00_00_00,
        info_text: 0xFF_FF_FF,
        app_workspace: 0x00_00_00,
    };

    /// Windows 95 "High Contrast White".
    pub(crate) const HIGH_CONTRAST_WHITE: Self = Self {
        face: 0xFF_FF_FF,
        highlight: 0xC0_C0_C0,
        light: 0xE0_E0_E0,
        shadow: 0x80_80_80,
        dark_shadow: 0x00_00_00,
        window: 0xFF_FF_FF,
        text: 0x00_00_00,
        active_caption: 0x00_00_00,
        active_caption_text: 0xFF_FF_FF,
        inactive_caption: 0xFF_FF_FF,
        inactive_caption_text: 0x00_00_00,
        selection: 0x00_00_00,
        selection_text: 0xFF_FF_FF,
        info: 0xFF_FF_FF,
        info_text: 0x00_00_00,
        app_workspace: 0xFF_FF_FF,
    };

    /// A scheme built from a face colour, an accent (title bars, selection),
    /// the colour of edit fields and the text colour, in the way Windows 95
    /// schemes relate their colours.
    pub(crate) fn derived(face: Rgb, accent: Rgb, window: Rgb, text: Rgb) -> Self {
        let dark = luminance(face) < 0.2;
        let on_accent = readable_on(accent);
        Self {
            face,
            highlight: if dark {
                mix(face, 0xFF_FF_FF, 0.45)
            } else {
                0xFF_FF_FF
            },
            light: mix(face, 0xFF_FF_FF, if dark { 0.2 } else { 0.5 }),
            shadow: mix(face, 0x00_00_00, if dark { 0.5 } else { 0.4 }),
            dark_shadow: 0x00_00_00,
            window,
            text,
            active_caption: accent,
            active_caption_text: on_accent,
            inactive_caption: mix(face, 0x80_80_80, 0.5),
            inactive_caption_text: mix(face, text, 0.6),
            selection: accent,
            selection_text: on_accent,
            info: window,
            info_text: text,
            app_workspace: mix(face, 0x00_00_00, if dark { 0.35 } else { 0.4 }),
        }
    }

    /// The appearance scheme of an SigNexora theme; `dark_mode` is the system's
    /// mode, used by the theme that follows it.
    pub(crate) fn for_theme(choice: ThemeChoice, dark_mode: bool) -> Self {
        use ThemeChoice as T;
        match choice {
            T::Classic => Self::STANDARD,
            T::HighContrastDark => Self::HIGH_CONTRAST_BLACK,
            T::HighContrastLight => Self::HIGH_CONTRAST_WHITE,
            T::System if !dark_mode => Self::derived(0xD4_D0_C8, 0x0A_24_6A, 0xFF_FF_FF, 0),
            T::Light => Self::derived(0xD4_D0_C8, 0x0A_24_6A, 0xFF_FF_FF, 0),
            T::System | T::Dark => Self::derived(0x3C_3C_3C, 0x2A_5A_A8, 0x1E_1E_1E, 0xF0_F0_F0),
            T::ColourSafeDark => Self::derived(0x33_33_33, 0x00_72_B2, 0x11_11_11, 0xFF_FF_FF),
            T::ColourSafeLight => Self::derived(0xD8_D8_D8, 0x00_72_B2, 0xFF_FF_FF, 0),
            T::Dusk => Self::derived(0x4A_40_3A, 0xA0_5A_10, 0x2A_24_20, 0xF2_E6_D8),
            T::Slate => Self::derived(0x3E_4A_54, 0x1E_70_70, 0x23_2B_31, 0xE6_EE_F2),
            T::Midnight => Self::derived(0x1E_2A_4A, 0x3A_5F_CD, 0x0E_16_30, 0xE0_E8_FF),
            T::Forest => Self::derived(0x34_44_3A, 0x2E_7A_40, 0x1C_26_20, 0xE4_F0_E6),
            T::NightVision => Self::derived(0x20_00_00, 0x80_00_00, 0x00_00_00, 0xFF_40_40),
            T::Phosphor => Self::derived(0x0A_20_0A, 0x00_60_00, 0x00_00_00, 0x33_FF_66),
            T::Amber => Self::derived(0x26_1A_00, 0x80_50_00, 0x00_00_00, 0xFF_B0_00),
            T::Paper => Self::derived(0xE8_DF_C8, 0x7A_4A_1A, 0xFF_F8_E8, 0x2A_20_18),
            T::Sand => Self::derived(0xD8_C8_A0, 0x8A_5A_2A, 0xF4_EC_D8, 0x3A_2A_18),
        }
    }

    /// The colour of a [`Role`].
    pub(crate) fn rgb(&self, role: Role) -> Rgb {
        match role {
            Role::Face => self.face,
            Role::Highlight => self.highlight,
            Role::Light => self.light,
            Role::Shadow => self.shadow,
            Role::DarkShadow => self.dark_shadow,
            Role::Window => self.window,
            Role::Text => self.text,
            Role::Selection => self.selection,
            Role::SelectionText => self.selection_text,
            Role::Info => self.info,
            Role::InfoText => self.info_text,
        }
    }

    /// The colour of a [`Role`] as a GPUI colour.
    pub(crate) fn of(&self, role: Role) -> Hsla {
        hsla(self.rgb(role))
    }
}

/// A system colour by meaning, resolved through the active [`Scheme`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Face,
    Highlight,
    Light,
    Shadow,
    DarkShadow,
    Window,
    Text,
    Selection,
    SelectionText,
    Info,
    InfoText,
}

/// The active scheme, as a GPUI global.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ActiveScheme(pub(crate) Scheme);

impl Global for ActiveScheme {}

/// The active scheme ("Windows Standard" until one is set).
pub(crate) fn scheme(cx: &App) -> Scheme {
    cx.try_global::<ActiveScheme>()
        .map_or(Scheme::STANDARD, |s| s.0)
}

/// Signals Analyzer's own colours, which its forms set themselves rather than
/// taking them from the Windows scheme: the navy panels of `SA Open file`, the
/// green text of Correction of BR. They stay SA's under every scheme except the
/// high-contrast ones, which use the system colours instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SaColors {
    /// Navy panel background.
    pub(crate) navy_bg: Rgb,
    /// Text on navy (16-bit mono PCM info, file names).
    pub(crate) navy_text: Rgb,
    /// Text flagging something unusual (yellow on navy).
    pub(crate) navy_warn: Rgb,
    /// The selected file's name.
    pub(crate) navy_select: Rgb,
    /// The selected file's band.
    pub(crate) navy_select_bg: Rgb,
    /// The full-scale lines of the preview.
    pub(crate) preview_limit: Rgb,
    /// Green text (`Max delta change: +/- 30%`), green icons.
    pub(crate) green: Rgb,
    /// Plot background (black in SA).
    pub(crate) plot_bg: Rgb,
    /// Text on plots.
    pub(crate) plot_text: Rgb,
}

impl SaColors {
    /// SA's colours (from the screenshots).
    pub(crate) const SA: Self = Self {
        navy_bg: 0x00_00_A8,
        navy_text: 0xFF_FF_FF,
        navy_warn: 0xFF_FF_00,
        navy_select: 0x00_FF_FF,
        navy_select_bg: 0x00_00_68,
        preview_limit: 0xFF_00_00,
        green: 0x00_96_00,
        plot_bg: 0x00_00_00,
        plot_text: 0xFF_FF_FF,
    };

    /// The colours under scheme `s`: SA's, or the system colours of a
    /// high-contrast scheme.
    pub(crate) fn for_scheme(s: &Scheme) -> Self {
        if *s != Scheme::HIGH_CONTRAST_BLACK && *s != Scheme::HIGH_CONTRAST_WHITE {
            return Self::SA;
        }
        Self {
            navy_bg: s.window,
            navy_text: s.text,
            navy_warn: s.text,
            navy_select: s.selection_text,
            navy_select_bg: s.selection,
            preview_limit: s.text,
            green: s.text,
            plot_bg: s.window,
            plot_text: s.text,
        }
    }
}

/// SA's colours under the active scheme.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ActiveSa(pub(crate) SaColors);

impl Global for ActiveSa {}

/// SA's colours under the active scheme.
pub(crate) fn sa_colors(cx: &App) -> SaColors {
    cx.try_global::<ActiveSa>().map_or(SaColors::SA, |s| s.0)
}

/// The plot palette of the active theme.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ActivePalette(pub(crate) &'static Palette);

impl Global for ActivePalette {}

/// The plot palette (traces, markers, notices) of the active theme.
pub(crate) fn palette(cx: &App) -> &'static Palette {
    cx.try_global::<ActivePalette>()
        .map_or_else(|| palette_for(ThemeChoice::Classic, false), |p| p.0)
}

/// Make `choice` the active theme: its scheme, SA's colours and its palette.
pub(crate) fn apply_theme(
    cx: &mut App,
    choice: ThemeChoice,
    look: sn_ui_core::winstyle::Look,
    dark_mode: bool,
) {
    super::style::set(look, choice);
    // The classic styles (and high contrast) colour their controls with the
    // appearance scheme; the themed styles with their own colours.
    let classic = super::style::active().classic();
    let s = if classic {
        Scheme::for_theme(choice, dark_mode)
    } else {
        super::style::themed_scheme(look)
    };
    cx.set_global(ActiveScheme(s));
    cx.set_global(ActiveSa(SaColors::for_scheme(&s)));
    cx.set_global(ActivePalette(palette_for(
        choice,
        palette_dark(classic, dark_mode, &s),
    )));
}

/// Whether the palette takes its dark variant: with the system's mode under
/// the classic styles (whose scheme follows the theme), with the drawn
/// scheme's face under the themed ones (Windows 10's dark app mode is dark
/// whatever the system's mode, XP to 8 are light), so the notice colours
/// of the forms read on the face they are drawn on.
fn palette_dark(classic: bool, dark_mode: bool, drawn: &Scheme) -> bool {
    if classic {
        dark_mode
    } else {
        luminance(drawn.face) < 0.2
    }
}

/// A colour of a palette as `0xRRGGBB` (palette colours are opaque or
/// premultiplied; the alpha is dropped).
pub(crate) fn rgb_of(c: sn_ui_core::color::Rgba8) -> Rgb {
    (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)
}

fn channels(c: Rgb) -> [f64; 3] {
    [(c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF].map(|v| f64::from(v) / 255.0)
}

/// Relative luminance (WCAG); the single formula lives in sn-ui-core.
pub(crate) fn luminance(c: Rgb) -> f64 {
    sn_ui_core::winstyle::relative_luminance(c)
}

/// Contrast ratio of two colours (WCAG), 1 to 21.
pub(crate) fn contrast(a: Rgb, b: Rgb) -> f64 {
    sn_ui_core::winstyle::contrast_ratio(a, b)
}

/// White or black, whichever reads better on `bg`.
fn readable_on(bg: Rgb) -> Rgb {
    if contrast(0xFF_FF_FF, bg) >= contrast(0, bg) {
        0xFF_FF_FF
    } else {
        0
    }
}

/// `a` moved towards `b` by `t` (0 to 1).
pub(crate) fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let [ar, ag, ab] = channels(a);
    let [br, bg, bb] = channels(b);
    let ch = |x: f64, y: f64| ((x + (y - x) * t) * 255.0).round().clamp(0.0, 255.0) as u32;
    (ch(ar, br) << 16) | (ch(ag, bg) << 8) | ch(ab, bb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scheme_reads_well() {
        for choice in ThemeChoice::ALL {
            for dark in [false, true] {
                let s = Scheme::for_theme(choice, dark);
                let name = format!("{choice:?} dark={dark}");
                assert!(contrast(s.text, s.face) >= 4.5, "text on face, {name}");
                assert!(contrast(s.text, s.window) >= 4.5, "text in fields, {name}");
                assert!(
                    contrast(s.active_caption_text, s.active_caption) >= 4.5,
                    "active caption, {name}"
                );
                assert!(
                    contrast(s.selection_text, s.selection) >= 4.5,
                    "selection, {name}"
                );
                assert!(
                    contrast(s.highlight, s.face).max(contrast(s.dark_shadow, s.face)) >= 3.0,
                    "3D edges, {name}"
                );
            }
        }
    }

    #[test]
    fn themed_styles_pick_the_palette_by_their_face() {
        use sn_ui_core::winstyle::{Look, WinStyle};
        let dark10 = super::super::style::themed_scheme(Look {
            style: WinStyle::Win10,
            dark: true,
            ..Look::default()
        });
        let xp = super::super::style::themed_scheme(Look::default());
        assert!(
            palette_dark(false, false, &dark10),
            "dark app mode, light OS"
        );
        assert!(!palette_dark(false, true, &xp), "XP under a dark OS");
        assert!(
            palette_dark(true, true, &Scheme::STANDARD),
            "classic follows the OS"
        );
        assert!(!palette_dark(true, false, &Scheme::STANDARD));
    }

    #[test]
    fn standard_is_classic_and_high_contrast_maps_to_windows_schemes() {
        assert_eq!(
            Scheme::for_theme(ThemeChoice::Classic, true),
            Scheme::STANDARD
        );
        assert_eq!(
            Scheme::for_theme(ThemeChoice::HighContrastDark, false),
            Scheme::HIGH_CONTRAST_BLACK
        );
        assert_eq!(
            Scheme::for_theme(ThemeChoice::HighContrastLight, false),
            Scheme::HIGH_CONTRAST_WHITE
        );
        assert_eq!(SaColors::for_scheme(&Scheme::STANDARD), SaColors::SA);
        let hc = SaColors::for_scheme(&Scheme::HIGH_CONTRAST_BLACK);
        assert_eq!((hc.navy_bg, hc.navy_text), (0, 0xFF_FF_FF));
        assert_eq!(mix(0, 0xFF_FF_FF, 0.5), 0x80_80_80);
        assert_eq!(rgb_of(sn_ui_core::color::Rgba8::rgb(1, 2, 3)), 0x01_02_03);
    }
}
