//! Options > Style: the version of Windows (95, 98, XP, Vista, 7, 8, 10, 11),
//! of macOS (classic Aqua, Yosemite, Big Sur and later) or the Linux desktop
//! (GNOME's Adwaita with Ubuntu's Yaru as a variant, KDE Plasma's Breeze,
//! GNOME 2 with Clearlooks, Motif and CDE) whose look SigNexora draws, and its
//! colour variant.
//!
//! Options > Theme sets the plot colours in every style. The classic styles
//! (95 and 98) also take their window colours from it (the appearance
//! scheme). The themed styles (XP to 11) draw their windows with their own
//! colours: the XP colour scheme, Aero or Basic on Vista and 7, the accent
//! colour on 8, 10 and 11, and the light or dark app mode on 10 and 11. The
//! macOS styles take the Aqua appearance (blue or graphite) on classic Aqua,
//! and the macOS accent colour and the light or dark mode from Yosemite on.
//! Adwaita and Breeze take an accent colour (each look its own palette and
//! default: GNOME blue, Ubuntu orange for Yaru, Breeze blue) and the light or
//! dark mode; Clearlooks and Motif have one look each. A high-contrast theme
//! draws the Windows classic look in every style, the macOS and Linux ones
//! included, as only the classic look takes its colours from the
//! high-contrast scheme.

use crate::color::Rgba8;
use crate::t;
use crate::theme::ThemeChoice;

/// A version of Windows or macOS, or a Linux desktop, whose look SigNexora draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum WinStyle {
    /// Windows 95: grey 3D controls, navy title bars.
    Win95,
    /// Windows 98: Windows 95 with gradient title bars.
    Win98,
    /// Windows XP with the Luna visual style (SA's own screenshots).
    #[default]
    Xp,
    /// Windows Vista.
    Vista,
    /// Windows 7.
    Win7,
    /// Windows 8.
    Win8,
    /// Windows 10.
    Win10,
    /// Windows 11.
    Win11,
    /// Mac OS X 10.0 to 10.9: Aqua (pinstripes, glossy gel controls).
    MacAqua,
    /// OS X 10.10 Yosemite to macOS 10.15 Catalina: the first flat design.
    MacYosemite,
    /// macOS 11 Big Sur and later.
    MacModern,
    /// GNOME's Adwaita (GNOME 42 and later; Ubuntu's Yaru as a variant).
    LinuxAdwaita,
    /// KDE Plasma's Breeze.
    LinuxBreeze,
    /// GNOME 2 with the Clearlooks theme.
    LinuxClearlooks,
    /// Motif and the CDE desktop of 1990s Unix workstations.
    LinuxMotif,
}

impl WinStyle {
    /// Every style, in menu order.
    pub const ALL: [Self; 15] = [
        Self::Win95,
        Self::Win98,
        Self::Xp,
        Self::Vista,
        Self::Win7,
        Self::Win8,
        Self::Win10,
        Self::Win11,
        Self::MacAqua,
        Self::MacYosemite,
        Self::MacModern,
        Self::LinuxAdwaita,
        Self::LinuxBreeze,
        Self::LinuxClearlooks,
        Self::LinuxMotif,
    ];

    /// The menu text.
    pub fn label(self) -> String {
        match self {
            Self::Win95 => t!("winstyle-95"),
            Self::Win98 => t!("winstyle-98"),
            Self::Xp => t!("winstyle-xp"),
            Self::Vista => t!("winstyle-vista"),
            Self::Win7 => t!("winstyle-7"),
            Self::Win8 => t!("winstyle-8"),
            Self::Win10 => t!("winstyle-10"),
            Self::Win11 => t!("winstyle-11"),
            Self::MacAqua => t!("winstyle-mac-aqua"),
            Self::MacYosemite => t!("winstyle-mac-yosemite"),
            Self::MacModern => t!("winstyle-mac-modern"),
            Self::LinuxAdwaita => t!("winstyle-linux-adwaita"),
            Self::LinuxBreeze => t!("winstyle-linux-breeze"),
            Self::LinuxClearlooks => t!("winstyle-linux-clearlooks"),
            Self::LinuxMotif => t!("winstyle-linux-motif"),
        }
    }

    /// The menu hint: the versions a macOS style or the desktops a Linux
    /// style covers (Motif's also says how to close a window, as it has no
    /// close button); `None` for Windows, whose labels name the version.
    pub fn hint(self) -> Option<String> {
        match self {
            Self::MacAqua => Some(t!("winstyle-mac-aqua.hover")),
            Self::MacYosemite => Some(t!("winstyle-mac-yosemite.hover")),
            Self::MacModern => Some(t!("winstyle-mac-modern.hover")),
            Self::LinuxAdwaita => Some(t!("winstyle-linux-adwaita.hover")),
            Self::LinuxBreeze => Some(t!("winstyle-linux-breeze.hover")),
            Self::LinuxClearlooks => Some(t!("winstyle-linux-clearlooks.hover")),
            Self::LinuxMotif => Some(t!("winstyle-linux-motif.hover")),
            Self::Win95
            | Self::Win98
            | Self::Xp
            | Self::Vista
            | Self::Win7
            | Self::Win8
            | Self::Win10
            | Self::Win11 => None,
        }
    }

    /// Whether the style draws classic 3D controls coloured by the appearance
    /// scheme (95, 98).
    pub const fn is_classic(self) -> bool {
        matches!(self, Self::Win95 | Self::Win98)
    }

    /// Whether the style has the Aero/Basic choice (Vista, 7).
    pub const fn has_aero(self) -> bool {
        matches!(self, Self::Vista | Self::Win7)
    }

    /// Whether the style has an accent colour (Windows 8, 10, 11; macOS from
    /// Yosemite on; Adwaita and Breeze).
    pub const fn has_accent(self) -> bool {
        matches!(
            self,
            Self::Win8
                | Self::Win10
                | Self::Win11
                | Self::MacYosemite
                | Self::MacModern
                | Self::LinuxAdwaita
                | Self::LinuxBreeze
        )
    }

    /// Whether the style has a light and a dark app mode (Windows 10, 11;
    /// macOS from Yosemite on, as 10.14 added it; Adwaita and Breeze).
    pub const fn has_app_mode(self) -> bool {
        matches!(
            self,
            Self::Win10
                | Self::Win11
                | Self::MacYosemite
                | Self::MacModern
                | Self::LinuxAdwaita
                | Self::LinuxBreeze
        )
    }

    /// Whether the style is a Linux look.
    pub const fn is_linux(self) -> bool {
        matches!(
            self,
            Self::LinuxAdwaita | Self::LinuxBreeze | Self::LinuxClearlooks | Self::LinuxMotif
        )
    }

    /// Whether the style is a macOS look.
    pub const fn is_mac(self) -> bool {
        matches!(self, Self::MacAqua | Self::MacYosemite | Self::MacModern)
    }

    /// Whether the style is the first of its family (Windows, macOS or Linux) in
    /// [`Self::ALL`], so the menu puts a separator before it (except at the
    /// top).
    pub const fn starts_family(self) -> bool {
        matches!(self, Self::Win95 | Self::MacAqua | Self::LinuxAdwaita)
    }

    /// The accent colours the style offers: macOS's, Breeze's, GNOME's for
    /// the other Linux looks, or Windows'. Adwaita's Yaru variant has its
    /// own; [`Look::accents`] knows it, so the menu takes the swatches from
    /// there.
    pub const fn accents(self) -> &'static [Accent] {
        if self.is_mac() {
            &MAC_ACCENTS
        } else if matches!(self, Self::LinuxBreeze) {
            &BREEZE_ACCENTS
        } else if self.is_linux() {
            &LINUX_ACCENTS
        } else {
            &ACCENTS
        }
    }
}

/// The two appearances of Aqua (Mac OS X 10.0 to 10.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AquaColor {
    /// Blue (the default).
    #[default]
    Blue,
    /// Graphite.
    Graphite,
}

impl AquaColor {
    /// Both, in menu order.
    pub const ALL: [Self; 2] = [Self::Blue, Self::Graphite];

    /// The menu text (Mac OS X's own names).
    pub fn label(self) -> String {
        match self {
            Self::Blue => t!("winstyle-aqua-blue"),
            Self::Graphite => t!("winstyle-aqua-graphite"),
        }
    }
}

/// The colour schemes of Windows XP's Luna style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum XpColor {
    /// Default (blue).
    #[default]
    Blue,
    /// Olive Green.
    Olive,
    /// Silver.
    Silver,
}

impl XpColor {
    /// Every scheme, in menu order.
    pub const ALL: [Self; 3] = [Self::Blue, Self::Olive, Self::Silver];

    /// The menu text (Windows XP's own names).
    pub fn label(self) -> String {
        match self {
            Self::Blue => t!("winstyle-xp-blue"),
            Self::Olive => t!("winstyle-xp-olive"),
            Self::Silver => t!("winstyle-xp-silver"),
        }
    }
}

/// An accent colour: a swatch of the Windows 10 colour picker ([`ACCENTS`]),
/// of macOS's ([`MAC_ACCENTS`]) or of a Linux look's ([`LINUX_ACCENTS`],
/// [`YARU_ACCENTS`], [`BREEZE_ACCENTS`]). Only these swatches exist, so every
/// name is translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accent {
    /// `0xRRGGBB`, the colour the menu stores and marks.
    rgb: u32,
    /// `0xRRGGBB` in dark mode: macOS draws a brighter variant there;
    /// Windows draws the same colour.
    dark: u32,
    /// The Fluent key of its name.
    key: &'static str,
}

impl Accent {
    /// A swatch drawn alike in light and dark mode (Windows').
    const fn same(rgb: u32, key: &'static str) -> Self {
        Self {
            rgb,
            dark: rgb,
            key,
        }
    }

    /// The colour, `0xRRGGBB`.
    pub const fn rgb(self) -> u32 {
        self.rgb
    }

    /// The colour drawn in dark mode, `0xRRGGBB`.
    pub const fn dark_rgb(self) -> u32 {
        self.dark
    }

    /// The menu text.
    pub fn label(self) -> String {
        t!(self.key)
    }
}

/// The default accent colour (Windows 10's blue), the first of [`ACCENTS`].
pub const DEFAULT_ACCENT: u32 = 0x00_78_D7;

/// The accent colours offered: Windows 10's default blue first, then a choice
/// of its colour picker's swatches.
pub const ACCENTS: [Accent; 12] = [
    Accent::same(DEFAULT_ACCENT, "winstyle-accent-blue"),
    Accent::same(0x00_63_B1, "winstyle-accent-navy"),
    Accent::same(0x00_99_BC, "winstyle-accent-teal"),
    Accent::same(0x00_8A_00, "winstyle-accent-green"),
    Accent::same(0x10_7C_10, "winstyle-accent-dark-green"),
    Accent::same(0x88_17_98, "winstyle-accent-purple"),
    Accent::same(0xB1_46_C2, "winstyle-accent-orchid"),
    Accent::same(0xE8_11_23, "winstyle-accent-red"),
    Accent::same(0xC3_00_52, "winstyle-accent-raspberry"),
    Accent::same(0xCA_50_10, "winstyle-accent-orange"),
    Accent::same(0xFF_8C_00, "winstyle-accent-gold"),
    Accent::same(0x69_79_7E, "winstyle-accent-grey"),
];

/// macOS's default accent colour (blue), the first of [`MAC_ACCENTS`].
pub const DEFAULT_MAC_ACCENT: u32 = 0x00_7A_FF;

/// GNOME's default accent colour (blue), the first of [`LINUX_ACCENTS`].
pub const DEFAULT_LINUX_ACCENT: u32 = 0x35_84_E4;

/// The accent colours of GNOME (47 and later), offered for Adwaita: GNOME's
/// blue first.
pub const LINUX_ACCENTS: [Accent; 9] = [
    Accent::same(DEFAULT_LINUX_ACCENT, "winstyle-accent-blue"),
    Accent::same(0x21_90_A4, "winstyle-accent-teal"),
    Accent::same(0x3A_94_4A, "winstyle-accent-green"),
    Accent::same(0xC8_88_00, "winstyle-accent-yellow"),
    Accent::same(0xED_5B_00, "winstyle-accent-orange"),
    Accent::same(0xE6_2D_42, "winstyle-accent-red"),
    Accent::same(0xD5_61_99, "winstyle-accent-pink"),
    Accent::same(0x91_41_AC, "winstyle-accent-purple"),
    Accent::same(0x6F_83_96, "winstyle-linux-accent-slate"),
];

/// Ubuntu's orange, Yaru's default accent and the first of [`YARU_ACCENTS`].
pub const YARU_ORANGE: u32 = 0xE9_54_20;

/// The accent colours of Adwaita's Yaru variant: Ubuntu's orange first, then
/// GNOME's other colours (GNOME's own orange left out, so the menu offers one
/// orange).
pub const YARU_ACCENTS: [Accent; 8] = [
    Accent::same(YARU_ORANGE, "winstyle-accent-orange"),
    Accent::same(DEFAULT_LINUX_ACCENT, "winstyle-accent-blue"),
    Accent::same(0x21_90_A4, "winstyle-accent-teal"),
    Accent::same(0x3A_94_4A, "winstyle-accent-green"),
    Accent::same(0xC8_88_00, "winstyle-accent-yellow"),
    Accent::same(0xE6_2D_42, "winstyle-accent-red"),
    Accent::same(0xD5_61_99, "winstyle-accent-pink"),
    Accent::same(0x91_41_AC, "winstyle-accent-purple"),
];

/// Breeze's default accent ("Breeze blue"), the first of [`BREEZE_ACCENTS`].
pub const BREEZE_BLUE: u32 = 0x3D_AE_E9;

/// The accent colours of Breeze: Breeze blue first, then GNOME's other
/// colours, which the Linux looks share.
pub const BREEZE_ACCENTS: [Accent; 9] = [
    Accent::same(BREEZE_BLUE, "winstyle-accent-blue"),
    Accent::same(0x21_90_A4, "winstyle-accent-teal"),
    Accent::same(0x3A_94_4A, "winstyle-accent-green"),
    Accent::same(0xC8_88_00, "winstyle-accent-yellow"),
    Accent::same(0xED_5B_00, "winstyle-accent-orange"),
    Accent::same(0xE6_2D_42, "winstyle-accent-red"),
    Accent::same(0xD5_61_99, "winstyle-accent-pink"),
    Accent::same(0x91_41_AC, "winstyle-accent-purple"),
    Accent::same(0x6F_83_96, "winstyle-linux-accent-slate"),
];

/// The accent colours of macOS (System Settings > Appearance): the light
/// mode colour, which the menu stores, and the brighter one dark mode draws.
pub const MAC_ACCENTS: [Accent; 8] = [
    mac(DEFAULT_MAC_ACCENT, 0x0A_84_FF, "winstyle-accent-blue"),
    mac(0xA5_50_A7, 0xBF_5A_F2, "winstyle-accent-purple"),
    mac(0xF7_4F_9E, 0xFF_5F_A8, "winstyle-mac-accent-pink"),
    mac(0xFF_52_57, 0x00FF_5F64, "winstyle-accent-red"),
    mac(0xF7_82_1B, 0xFF_9F_0A, "winstyle-accent-orange"),
    mac(0xFF_C6_00, 0xFF_D6_0A, "winstyle-mac-accent-yellow"),
    mac(0x62_BA_46, 0x32_D7_4B, "winstyle-accent-green"),
    mac(0x8C_8C_8C, 0x98_98_9D, "winstyle-mac-accent-graphite"),
];

/// A macOS swatch: its light and dark mode colours.
const fn mac(rgb: u32, dark: u32, key: &'static str) -> Accent {
    Accent { rgb, dark, key }
}

/// The WCAG contrast ratio needed for caption text (WCAG 2 AA, normal text),
/// to one decimal place: Windows 10's default blue gives white text 4.499:1,
/// and keeps it, as on Windows.
pub const MIN_CAPTION_CONTRAST: f64 = 4.45;

/// The WCAG relative luminance of `0xRRGGBB`, 0 (black) to 1 (white). The
/// single implementation is [`Rgba8::luminance`]; this is its `0xRRGGBB` form.
pub fn relative_luminance(rgb: u32) -> f64 {
    Rgba8::from_rgb24(rgb).luminance()
}

/// The WCAG contrast ratio of two `0xRRGGBB` colours, 1 to 21.
pub fn contrast_ratio(a: u32, b: u32) -> f64 {
    Rgba8::from_rgb24(a).contrast_over(Rgba8::from_rgb24(b))
}

/// The caption text colour on a `0xRRGGBB` caption: white, as Windows draws
/// it, unless white is too faint there; then black, as Windows 10 does on
/// light accents.
pub fn caption_text_on(caption: u32) -> u32 {
    const WHITE: u32 = 0xFF_FF_FF;
    if contrast_ratio(caption & 0x00FF_FFFF, WHITE) >= MIN_CAPTION_CONTRAST {
        WHITE
    } else {
        0x00_00_00
    }
}

/// The window look, Windows, macOS or Linux: the style and its variants.
/// Variants of other styles are kept, so switching back restores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Look {
    /// The Windows or macOS version, or the Linux desktop, drawn.
    pub style: WinStyle,
    /// The XP colour scheme.
    pub xp: XpColor,
    /// Aero (true) or Basic (false) on Vista and 7.
    pub aero: bool,
    /// The Windows accent colour (8, 10, 11), `0xRRGGBB`, one of [`ACCENTS`].
    /// The macOS and Linux styles use their own fields instead.
    pub accent: u32,
    /// The dark app mode of Windows 10 and 11, of macOS and of Adwaita and
    /// Breeze.
    pub dark: bool,
    /// The Aqua appearance.
    pub aqua: AquaColor,
    /// The accent colour of the macOS looks (Yosemite on), `0xRRGGBB`: the
    /// light mode colour of one of [`MAC_ACCENTS`].
    pub mac_accent: u32,
    /// Ubuntu's Yaru instead of GNOME's Adwaita.
    pub yaru: bool,
    /// The accent colour of Adwaita (without Yaru), `0xRRGGBB`, one of
    /// [`LINUX_ACCENTS`].
    pub linux_accent: u32,
    /// The accent colour of Adwaita's Yaru variant, `0xRRGGBB`, one of
    /// [`YARU_ACCENTS`].
    pub yaru_accent: u32,
    /// The accent colour of Breeze, `0xRRGGBB`, one of [`BREEZE_ACCENTS`].
    pub breeze_accent: u32,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            style: WinStyle::default(),
            xp: XpColor::default(),
            aero: true,
            accent: DEFAULT_ACCENT,
            dark: false,
            aqua: AquaColor::Blue,
            mac_accent: DEFAULT_MAC_ACCENT,
            yaru: false,
            linux_accent: DEFAULT_LINUX_ACCENT,
            yaru_accent: YARU_ORANGE,
            breeze_accent: BREEZE_BLUE,
        }
    }
}

impl Look {
    /// The look Windows 95 draws (Phase 10's).
    pub const WIN95: Self = Self {
        style: WinStyle::Win95,
        xp: XpColor::Blue,
        aero: true,
        accent: DEFAULT_ACCENT,
        dark: false,
        aqua: AquaColor::Blue,
        mac_accent: DEFAULT_MAC_ACCENT,
        yaru: false,
        linux_accent: DEFAULT_LINUX_ACCENT,
        yaru_accent: YARU_ORANGE,
        breeze_accent: BREEZE_BLUE,
    };

    /// The style actually drawn with `theme`: high contrast draws the classic
    /// look in every style, as Windows turns visual styles off for high
    /// contrast, and only the classic look takes its colours from the
    /// high-contrast scheme. The themed styles (XP to 11), the macOS styles
    /// and the Linux styles are drawn as Windows 95; 95 and 98 are drawn as chosen,
    /// coloured by the high-contrast scheme (with flat captions, as Windows
    /// 98 draws no caption gradient there).
    pub fn drawn_style(self, theme: ThemeChoice) -> WinStyle {
        let high_contrast = matches!(
            theme,
            ThemeChoice::HighContrastDark | ThemeChoice::HighContrastLight
        );
        if high_contrast && !self.style.is_classic() {
            WinStyle::Win95
        } else {
            self.style
        }
    }

    /// Whether `theme` draws another style than the chosen one (high
    /// contrast with a themed style), so the style's variants have no effect.
    pub fn overridden_by(self, theme: ThemeChoice) -> bool {
        self.drawn_style(theme) != self.style
    }

    /// The accent colours the look offers: [`WinStyle::accents`], except
    /// for Adwaita's Yaru variant, which offers [`YARU_ACCENTS`].
    pub const fn accents(self) -> &'static [Accent] {
        if self.is_yaru() {
            &YARU_ACCENTS
        } else {
            self.style.accents()
        }
    }

    /// Whether the look is Adwaita's Yaru variant.
    const fn is_yaru(self) -> bool {
        matches!(self.style, WinStyle::LinuxAdwaita) && self.yaru
    }

    /// The accent colour chosen for the look (Windows', macOS's, Breeze's,
    /// Yaru's or GNOME's for the other Linux looks), kept to 24 bits: the
    /// swatch the Accent color menu marks.
    pub const fn accent_rgb(self) -> u32 {
        let rgb = if self.style.is_mac() {
            self.mac_accent
        } else if matches!(self.style, WinStyle::LinuxBreeze) {
            self.breeze_accent
        } else if self.is_yaru() {
            self.yaru_accent
        } else if self.style.is_linux() {
            self.linux_accent
        } else {
            self.accent
        };
        rgb & 0x00FF_FFFF
    }

    /// Chooses `rgb` (`0xRRGGBB`, a swatch of [`Self::accents`]) as the
    /// accent colour of the look: it goes to the field [`Self::accent_rgb`]
    /// reads, so the other styles keep theirs.
    pub const fn set_accent(&mut self, rgb: u32) {
        if self.style.is_mac() {
            self.mac_accent = rgb;
        } else if matches!(self.style, WinStyle::LinuxBreeze) {
            self.breeze_accent = rgb;
        } else if self.is_yaru() {
            self.yaru_accent = rgb;
        } else if self.style.is_linux() {
            self.linux_accent = rgb;
        } else {
            self.accent = rgb;
        }
    }

    /// The colour the look draws for swatch `a`, `0xRRGGBB`: the swatch's
    /// colour, except in the dark mode of a macOS style, which draws its
    /// brighter dark variant, as macOS does. The Accent color menu paints
    /// each swatch with it, so the swatch shows what the windows draw.
    pub const fn drawn_swatch(self, a: Accent) -> u32 {
        if self.style.is_mac() && self.style.has_app_mode() && self.dark {
            a.dark & 0x00FF_FFFF
        } else {
            a.rgb & 0x00FF_FFFF
        }
    }

    /// The accent colour to draw, `0xRRGGBB`: [`Self::drawn_swatch`] of the
    /// swatch [`Self::accent_rgb`] marks, so the windows draw what that
    /// swatch shows (the stored colour itself if no swatch matches).
    pub fn drawn_accent(self) -> u32 {
        let rgb = self.accent_rgb();
        self.accents()
            .iter()
            .find(|a| a.rgb == rgb)
            .map_or(rgb, |&a| self.drawn_swatch(a))
    }

    /// The look with values the menu could not have produced replaced: an
    /// accent that is not one of [`ACCENTS`] becomes [`DEFAULT_ACCENT`], a
    /// `mac_accent` that is not one of [`MAC_ACCENTS`] becomes
    /// [`DEFAULT_MAC_ACCENT`], a `linux_accent` that is not one of
    /// [`LINUX_ACCENTS`] becomes [`DEFAULT_LINUX_ACCENT`], a `yaru_accent`
    /// that is not one of [`YARU_ACCENTS`] becomes [`YARU_ORANGE`] and a
    /// `breeze_accent` that is not one of [`BREEZE_ACCENTS`] becomes
    /// [`BREEZE_BLUE`], so the Accent color menu always marks the colour in
    /// use.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        if !ACCENTS.iter().any(|a| a.rgb == self.accent) {
            self.accent = DEFAULT_ACCENT;
        }
        if !MAC_ACCENTS.iter().any(|a| a.rgb == self.mac_accent) {
            self.mac_accent = DEFAULT_MAC_ACCENT;
        }
        if !LINUX_ACCENTS.iter().any(|a| a.rgb == self.linux_accent) {
            self.linux_accent = DEFAULT_LINUX_ACCENT;
        }
        if !YARU_ACCENTS.iter().any(|a| a.rgb == self.yaru_accent) {
            self.yaru_accent = YARU_ORANGE;
        }
        if !BREEZE_ACCENTS.iter().any(|a| a.rgb == self.breeze_accent) {
            self.breeze_accent = BREEZE_BLUE;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_look_is_xp_blue() {
        let l = Look::default();
        assert_eq!(
            (l.style, l.xp, l.aero, l.dark),
            (WinStyle::Xp, XpColor::Blue, true, false)
        );
        assert_eq!(l.accent_rgb(), DEFAULT_ACCENT);
    }

    #[test]
    fn high_contrast_draws_the_classic_look() {
        let xp = Look::default();
        assert_eq!(
            xp.drawn_style(ThemeChoice::HighContrastDark),
            WinStyle::Win95
        );
        assert_eq!(xp.drawn_style(ThemeChoice::Classic), WinStyle::Xp);
        let w98 = Look {
            style: WinStyle::Win98,
            ..Look::default()
        };
        assert_eq!(
            w98.drawn_style(ThemeChoice::HighContrastLight),
            WinStyle::Win98
        );
    }

    #[test]
    fn every_label_is_translated() {
        for s in WinStyle::ALL {
            assert!(!s.label().starts_with("winstyle-"), "{s:?}");
        }
        for c in XpColor::ALL {
            assert!(!c.label().starts_with("winstyle-"), "{c:?}");
        }
        for c in AquaColor::ALL {
            assert!(!c.label().starts_with("winstyle-"), "{c:?}");
        }
        // The other languages are held to the English messages by the i18n
        // tests, so a key missing here is caught in every locale.
        for a in all_swatches() {
            assert!(!a.label().starts_with("winstyle-"), "{}", a.key);
        }
        for s in WinStyle::ALL {
            if let Some(h) = s.hint() {
                assert!(!h.starts_with("winstyle-"), "{s:?}");
            }
        }
        for key in [
            "winstyle-yaru.hover",
            "winstyle-hc-active",
            "winstyle-aero.hover",
            "winstyle-scheme-note.hover",
            "winstyle-hc-active.hover",
            "winstyle-scheme-note",
        ] {
            assert!(!t!(key).starts_with("winstyle-"), "{key}");
        }
    }

    /// Every swatch of every list.
    fn all_swatches() -> impl Iterator<Item = &'static Accent> {
        ACCENTS
            .iter()
            .chain(MAC_ACCENTS.iter())
            .chain(LINUX_ACCENTS.iter())
            .chain(YARU_ACCENTS.iter())
            .chain(BREEZE_ACCENTS.iter())
    }

    #[test]
    fn every_style_but_windows_has_a_hint() {
        for s in WinStyle::ALL {
            assert_eq!(s.hint().is_some(), s.is_mac() || s.is_linux(), "{s:?}");
        }
    }

    #[test]
    fn the_default_accent_is_the_first_swatch() {
        assert_eq!(ACCENTS.first().map(|a| a.rgb()), Some(DEFAULT_ACCENT));
        assert_eq!(
            MAC_ACCENTS.first().map(|a| a.rgb()),
            Some(DEFAULT_MAC_ACCENT)
        );
        assert_eq!(
            LINUX_ACCENTS.first().map(|a| a.rgb()),
            Some(DEFAULT_LINUX_ACCENT)
        );
        assert_eq!(YARU_ACCENTS.first().map(|a| a.rgb()), Some(YARU_ORANGE));
        assert_eq!(BREEZE_ACCENTS.first().map(|a| a.rgb()), Some(BREEZE_BLUE));
        // The default look marks the first swatch of every look's list.
        for style in WinStyle::ALL {
            for yaru in [false, true] {
                let l = Look {
                    style,
                    yaru,
                    ..Look::default()
                };
                assert_eq!(
                    l.accents().first().map(|a| a.rgb()),
                    Some(l.accent_rgb()),
                    "{style:?} yaru={yaru}"
                );
            }
        }
    }

    #[test]
    fn each_list_offers_each_colour_once() {
        for list in [
            &ACCENTS[..],
            &MAC_ACCENTS[..],
            &LINUX_ACCENTS[..],
            &YARU_ACCENTS[..],
            &BREEZE_ACCENTS[..],
        ] {
            for (i, a) in list.iter().enumerate() {
                for b in list.iter().skip(i + 1) {
                    assert_ne!(a.rgb(), b.rgb(), "{} {}", a.key, b.key);
                    assert_ne!(a.key, b.key);
                }
            }
        }
    }

    #[test]
    fn accent_swatches_show_the_colour_drawn() {
        // Every swatch of every look, once chosen, is marked and drawn as
        // the menu paints it, so the menu and the windows agree.
        for style in WinStyle::ALL {
            for yaru in [false, true] {
                for dark in [false, true] {
                    let look = Look {
                        style,
                        dark,
                        yaru,
                        ..Look::default()
                    };
                    for &a in look.accents() {
                        let mut l = look;
                        l.set_accent(a.rgb());
                        assert_eq!(l.accent_rgb(), a.rgb(), "{style:?} {}", a.key);
                        assert_eq!(
                            l.drawn_accent(),
                            l.drawn_swatch(a),
                            "{style:?} yaru={yaru} dark={dark} {}",
                            a.key
                        );
                        assert_eq!(l.accents(), look.accents());
                        assert_eq!(l.sanitized(), l, "{style:?} {}", a.key);
                    }
                }
            }
        }
        // macOS's dark mode paints and draws the brighter variant.
        let mac = Look {
            style: WinStyle::MacModern,
            dark: true,
            ..Look::default()
        };
        assert_eq!(
            MAC_ACCENTS.first().map(|&a| mac.drawn_swatch(a)),
            Some(0x0A_84_FF)
        );
    }

    #[test]
    fn windows_swatches_draw_alike_in_dark_mode() {
        for a in ACCENTS {
            assert_eq!(a.dark_rgb(), a.rgb(), "{}", a.key);
        }
    }

    #[test]
    fn the_menu_starts_a_family_at_windows_macos_and_linux() {
        let starts: Vec<WinStyle> = WinStyle::ALL
            .into_iter()
            .filter(|w| w.starts_family())
            .collect();
        assert_eq!(
            starts,
            [WinStyle::Win95, WinStyle::MacAqua, WinStyle::LinuxAdwaita]
        );
        // Each family is one run, so one separator splits each pair.
        let family = |w: WinStyle| (w.is_mac(), w.is_linux());
        let mut changes = 0;
        for pair in WinStyle::ALL.windows(2) {
            if let [a, b] = pair
                && family(*a) != family(*b)
            {
                changes += 1;
                assert!(b.starts_family(), "{b:?}");
            }
        }
        assert_eq!(changes, 2);
    }

    #[test]
    fn the_mac_styles_use_the_mac_accent() {
        let l = Look {
            style: WinStyle::MacModern,
            accent: 0xE8_11_23,
            mac_accent: 0xF7_4F_9E,
            ..Look::default()
        };
        assert_eq!(l.accent_rgb(), 0xF7_4F_9E);
        assert_eq!(l.drawn_accent(), 0xF7_4F_9E);
        let win10 = Look {
            style: WinStyle::Win10,
            ..l
        };
        assert_eq!(win10.accent_rgb(), 0xE8_11_23);
        // The 24-bit mask holds on the macOS path too.
        let high_bits = Look {
            mac_accent: 0xFF00_7AFF,
            ..l
        };
        assert_eq!(high_bits.accent_rgb(), DEFAULT_MAC_ACCENT);
    }

    #[test]
    fn the_linux_styles_use_the_linux_accent() {
        let gnome = Look {
            style: WinStyle::LinuxAdwaita,
            accent: 0xE8_11_23,
            mac_accent: 0xF7_4F_9E,
            linux_accent: 0x91_41_AC,
            ..Look::default()
        };
        assert_eq!(gnome.accents(), &LINUX_ACCENTS[..]);
        assert_eq!(gnome.accent_rgb(), 0x91_41_AC);
        assert_eq!(gnome.drawn_accent(), 0x91_41_AC);
        // The 24-bit mask holds on the Linux path too.
        let high_bits = Look {
            linux_accent: 0xFF35_84E4,
            ..gnome
        };
        assert_eq!(high_bits.accent_rgb(), DEFAULT_LINUX_ACCENT);
        // Without Yaru, the default draws GNOME's blue.
        let plain = Look {
            style: WinStyle::LinuxAdwaita,
            ..Look::default()
        };
        assert_eq!(plain.drawn_accent(), DEFAULT_LINUX_ACCENT);
        // Yaru defaults to Ubuntu's orange, keeps its own choice, and can
        // draw GNOME's blue.
        let yaru = Look {
            yaru: true,
            ..plain
        };
        assert_eq!(yaru.accents(), &YARU_ACCENTS[..]);
        assert_eq!(yaru.accent_rgb(), YARU_ORANGE);
        assert_eq!(yaru.drawn_accent(), YARU_ORANGE);
        let mut purple = yaru;
        purple.set_accent(0x91_41_AC);
        assert_eq!(purple.drawn_accent(), 0x91_41_AC);
        assert_eq!(purple.linux_accent, DEFAULT_LINUX_ACCENT);
        let mut blue = yaru;
        blue.set_accent(DEFAULT_LINUX_ACCENT);
        assert_eq!(blue.drawn_accent(), DEFAULT_LINUX_ACCENT);
        // Turning Yaru off restores GNOME's own choice.
        assert_eq!(
            Look {
                yaru: false,
                ..purple
            }
            .drawn_accent(),
            DEFAULT_LINUX_ACCENT
        );
        // Breeze defaults to Breeze blue, in light and dark mode, and Yaru
        // has no effect there.
        for dark in [false, true] {
            let breeze = Look {
                style: WinStyle::LinuxBreeze,
                yaru: true,
                dark,
                ..Look::default()
            };
            assert_eq!(breeze.accents(), &BREEZE_ACCENTS[..]);
            assert_eq!(breeze.drawn_accent(), BREEZE_BLUE, "dark={dark}");
            let mut red = breeze;
            red.set_accent(0xE6_2D_42);
            assert_eq!(red.drawn_accent(), 0xE6_2D_42, "dark={dark}");
            assert_eq!(red.yaru_accent, YARU_ORANGE);
        }
        // A Linux choice leaves the Windows and macOS accents alone.
        let mut l = gnome;
        l.set_accent(0x21_90_A4);
        assert_eq!((l.accent, l.mac_accent), (0xE8_11_23, 0xF7_4F_9E));
    }

    #[test]
    fn dark_mode_draws_the_brighter_mac_accent() {
        let dark = Look {
            style: WinStyle::MacYosemite,
            dark: true,
            ..Look::default()
        };
        // The menu still marks the stored light-mode swatch.
        assert_eq!(dark.accent_rgb(), DEFAULT_MAC_ACCENT);
        assert_eq!(dark.drawn_accent(), 0x0A_84_FF);
        for a in MAC_ACCENTS {
            let l = Look {
                mac_accent: a.rgb(),
                ..dark
            };
            assert_eq!(l.drawn_accent(), a.dark_rgb(), "{}", a.key);
            assert!(
                relative_luminance(a.dark_rgb()) >= relative_luminance(a.rgb()),
                "{}",
                a.key
            );
        }
        // Light mode, classic Aqua (no dark mode) and Windows draw the stored
        // colour.
        assert_eq!(
            Look {
                dark: false,
                ..dark
            }
            .drawn_accent(),
            DEFAULT_MAC_ACCENT
        );
        let aqua = Look {
            style: WinStyle::MacAqua,
            ..dark
        };
        assert_eq!(aqua.drawn_accent(), DEFAULT_MAC_ACCENT);
        let win11 = Look {
            style: WinStyle::Win11,
            ..dark
        };
        assert_eq!(win11.drawn_accent(), DEFAULT_ACCENT);
    }

    #[test]
    fn caption_text_is_readable_on_every_accent() {
        for &a in all_swatches() {
            for rgb in [a.rgb(), a.dark_rgb()] {
                let text = caption_text_on(rgb);
                let ratio = contrast_ratio(rgb, text);
                // WCAG AA, 4.5:1, to one decimal place.
                assert!(
                    (ratio * 10.0).round() >= 45.0,
                    "{}: {ratio:.2}:1 with {text:06X} on {rgb:06X}",
                    a.key
                );
            }
        }
        // White is unreadable on the light macOS swatches, so they get black.
        assert_eq!(caption_text_on(0xFF_C6_00), 0x00_00_00);
        assert_eq!(caption_text_on(0x8C_8C_8C), 0x00_00_00);
        // Windows 10's default blue keeps white text; gold and teal get black.
        assert_eq!(caption_text_on(DEFAULT_ACCENT), 0xFF_FF_FF);
        assert_eq!(caption_text_on(0xFF_8C_00), 0x00_00_00);
        assert_eq!(caption_text_on(0x00_99_BC), 0x00_00_00);
        // Breeze blue is too light for white; Ubuntu's orange keeps black
        // as well.
        for rgb in [BREEZE_BLUE, YARU_ORANGE, DEFAULT_LINUX_ACCENT] {
            let ratio = contrast_ratio(rgb, caption_text_on(rgb));
            assert!((ratio * 10.0).round() >= 45.0, "{rgb:06X}: {ratio:.2}:1");
        }
        assert_eq!(caption_text_on(BREEZE_BLUE), 0x00_00_00);
        assert!((contrast_ratio(0, 0xFF_FF_FF) - 21.0).abs() < 1e-9);
    }

    #[test]
    fn high_contrast_overrides_only_the_themed_styles() {
        let w7 = Look {
            style: WinStyle::Win7,
            ..Look::default()
        };
        assert!(w7.overridden_by(ThemeChoice::HighContrastDark));
        assert!(w7.overridden_by(ThemeChoice::HighContrastLight));
        assert!(!w7.overridden_by(ThemeChoice::Classic));
        for style in [WinStyle::Win95, WinStyle::Win98] {
            let l = Look {
                style,
                ..Look::default()
            };
            assert!(!l.overridden_by(ThemeChoice::HighContrastDark), "{style:?}");
        }
        // The macOS and Linux styles are drawn as Windows 95 too: only the
        // classic look takes its colours from the high-contrast scheme.
        for style in [
            WinStyle::MacAqua,
            WinStyle::MacYosemite,
            WinStyle::MacModern,
            WinStyle::LinuxAdwaita,
            WinStyle::LinuxBreeze,
            WinStyle::LinuxClearlooks,
            WinStyle::LinuxMotif,
        ] {
            let l = Look {
                style,
                ..Look::default()
            };
            for theme in [
                ThemeChoice::HighContrastDark,
                ThemeChoice::HighContrastLight,
            ] {
                assert!(l.overridden_by(theme), "{style:?}");
                assert_eq!(l.drawn_style(theme), WinStyle::Win95, "{style:?}");
            }
            assert!(!l.overridden_by(ThemeChoice::Classic), "{style:?}");
        }
    }

    #[test]
    fn an_accent_outside_the_swatches_resets_to_the_default() {
        let odd = Look {
            accent: 0x12_34_56,
            ..Look::default()
        };
        assert_eq!(odd.sanitized().accent, DEFAULT_ACCENT);
        let red = Look {
            accent: 0xE8_11_23,
            dark: true,
            ..Look::default()
        };
        assert_eq!(red.sanitized(), red);
        let high_bits = Look {
            accent: 0xFF00_78D7,
            ..Look::default()
        };
        assert_eq!(high_bits.sanitized().accent, DEFAULT_ACCENT);
    }

    #[test]
    fn a_mac_accent_outside_the_swatches_resets_to_the_default() {
        let odd = Look {
            mac_accent: 0x12_34_56,
            ..Look::default()
        };
        assert_eq!(odd.sanitized().mac_accent, DEFAULT_MAC_ACCENT);
        let high_bits = Look {
            mac_accent: 0xFF00_7AFF,
            ..Look::default()
        };
        assert_eq!(high_bits.sanitized().mac_accent, DEFAULT_MAC_ACCENT);
        // Mojave's purple is not one of the swatches offered.
        let mojave = Look {
            mac_accent: 0x95_3D_96,
            ..Look::default()
        };
        assert_eq!(mojave.sanitized().mac_accent, DEFAULT_MAC_ACCENT);
        let pink = Look {
            style: WinStyle::MacModern,
            mac_accent: 0xF7_4F_9E,
            dark: true,
            ..Look::default()
        };
        assert_eq!(pink.sanitized(), pink);
    }

    #[test]
    fn a_linux_accent_outside_the_swatches_resets_to_the_default() {
        let odd = Look {
            linux_accent: 0x12_34_56,
            yaru_accent: 0x12_34_56,
            breeze_accent: 0x12_34_56,
            ..Look::default()
        };
        let s = odd.sanitized();
        assert_eq!(
            (s.linux_accent, s.yaru_accent, s.breeze_accent),
            (DEFAULT_LINUX_ACCENT, YARU_ORANGE, BREEZE_BLUE)
        );
        let high_bits = Look {
            linux_accent: 0xFF35_84E4,
            yaru_accent: 0xFFE9_5420,
            breeze_accent: 0xFF3D_AEE9,
            ..Look::default()
        };
        assert_eq!(high_bits.sanitized(), Look::default());
        // Each look's default belongs to its own list only.
        let swapped = Look {
            linux_accent: BREEZE_BLUE,
            yaru_accent: 0xED_5B_00,
            breeze_accent: YARU_ORANGE,
            ..Look::default()
        };
        assert_eq!(swapped.sanitized(), Look::default());
        let purple = Look {
            style: WinStyle::LinuxBreeze,
            breeze_accent: 0x91_41_AC,
            yaru_accent: 0x91_41_AC,
            linux_accent: 0x6F_83_96,
            dark: true,
            ..Look::default()
        };
        assert_eq!(purple.sanitized(), purple);
    }

    #[test]
    fn the_look_round_trips_and_reads_leniently() {
        let l = Look {
            style: WinStyle::Win10,
            xp: XpColor::Olive,
            aero: false,
            accent: 0xE8_11_23,
            dark: true,
            aqua: AquaColor::Graphite,
            mac_accent: 0xF7_4F_9E,
            yaru: true,
            linux_accent: 0x91_41_AC,
            yaru_accent: 0x21_90_A4,
            breeze_accent: 0x6F_83_96,
        };
        let text = serde_json::to_string(&l).unwrap_or_default();
        assert_eq!(serde_json::from_str::<Look>(&text).ok(), Some(l));
        let partial: Look = serde_json::from_str(r#"{"style":"Win7"}"#).unwrap_or_default();
        assert_eq!(partial.style, WinStyle::Win7);
        assert!(partial.aero);
        // Settings saved before Yaru and Breeze had their own accents get
        // those looks' defaults.
        let old: Look = serde_json::from_str(r#"{"style":"LinuxBreeze","linux_accent":9519532}"#)
            .unwrap_or_default();
        assert_eq!(
            (old.linux_accent, old.yaru_accent, old.breeze_accent),
            (0x91_41_AC, YARU_ORANGE, BREEZE_BLUE)
        );
    }
}
