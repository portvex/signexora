//! Colour themes as plain data: the theme choices, whether each follows the
//! system's dark or light mode, and the plot palettes.
//!
//! The front ends build their widget looks from these. Plots, traces, markers
//! and notices take their colours from a [`Palette`], never from literal
//! colours, so every theme (including the high-contrast ones) applies
//! everywhere.

use crate::color::Rgba8;

/// The theme the user picked.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum ThemeChoice {
    /// Signals Analyzer's own look: Windows XP Luna beige panels, blue title bars
    /// and black plots. The default for new users.
    #[default]
    Classic,
    /// Dark or Light, following the operating system.
    System,
    /// Dark.
    Dark,
    /// Light.
    Light,
    /// Black background, white text, strong outlines.
    HighContrastDark,
    /// White background, black text, strong outlines.
    HighContrastLight,
    /// Warm dark: brown-grey background, amber accents.
    Dusk,
    /// Cool dark: blue-grey background, teal accents.
    Slate,
    /// Warm light: cream background, ink text.
    Paper,
    /// Deep blue dark.
    Midnight,
    /// Green-grey dark.
    Forest,
    /// Red on black, to keep night vision while operating in the dark.
    NightVision,
    /// Green on black, like an oscilloscope screen.
    Phosphor,
    /// Amber on black, like a terminal.
    Amber,
    /// Dark, with traces in the Okabe-Ito colours that stay distinct for every
    /// common colour vision deficiency.
    ColourSafeDark,
    /// Light, with traces in the Okabe-Ito colours.
    ColourSafeLight,
    /// Warm light: sand background, dark brown text.
    Sand,
}

impl ThemeChoice {
    /// Every theme, in menu order.
    pub const ALL: [Self; 17] = [
        Self::Classic,
        Self::System,
        Self::Dark,
        Self::Light,
        Self::HighContrastDark,
        Self::HighContrastLight,
        Self::ColourSafeDark,
        Self::ColourSafeLight,
        Self::Dusk,
        Self::Slate,
        Self::Midnight,
        Self::Forest,
        Self::NightVision,
        Self::Phosphor,
        Self::Amber,
        Self::Paper,
        Self::Sand,
    ];

    /// Menu group: 0 basic, 1 accessibility, 2 dark palettes, 3 light palettes.
    /// The Settings menu puts a separator between groups.
    pub const fn group(self) -> u8 {
        match self {
            Self::Classic | Self::System | Self::Dark | Self::Light => 0,
            Self::HighContrastDark
            | Self::HighContrastLight
            | Self::ColourSafeDark
            | Self::ColourSafeLight => 1,
            Self::Dusk
            | Self::Slate
            | Self::Midnight
            | Self::Forest
            | Self::NightVision
            | Self::Phosphor
            | Self::Amber => 2,
            Self::Paper | Self::Sand => 3,
        }
    }

    /// Fluent key of the theme's name.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Classic => "theme-classic",
            Self::System => "theme-system",
            Self::Dark => "theme-dark",
            Self::Light => "theme-light",
            Self::HighContrastDark => "theme-high-contrast-dark",
            Self::HighContrastLight => "theme-high-contrast-light",
            Self::Dusk => "theme-dusk",
            Self::Slate => "theme-slate",
            Self::Paper => "theme-paper",
            Self::Midnight => "theme-midnight",
            Self::Forest => "theme-forest",
            Self::NightVision => "theme-night-vision",
            Self::Phosphor => "theme-phosphor",
            Self::Amber => "theme-amber",
            Self::ColourSafeDark => "theme-colour-safe-dark",
            Self::ColourSafeLight => "theme-colour-safe-light",
            Self::Sand => "theme-sand",
        }
    }

    /// A stable small number for the theme, for storing it in an atomic.
    pub const fn index(self) -> u8 {
        match self {
            Self::System => 0,
            Self::Dark => 1,
            Self::Light => 2,
            Self::HighContrastDark => 3,
            Self::HighContrastLight => 4,
            Self::Dusk => 5,
            Self::Slate => 6,
            Self::Paper => 7,
            Self::Midnight => 8,
            Self::Forest => 9,
            Self::NightVision => 10,
            Self::Phosphor => 11,
            Self::Amber => 12,
            Self::ColourSafeDark => 13,
            Self::ColourSafeLight => 14,
            Self::Sand => 15,
            Self::Classic => 16,
        }
    }

    /// The theme of [`Self::index`]; an unknown number gives the default.
    pub fn from_index(i: u8) -> Self {
        Self::ALL
            .into_iter()
            .find(|t| t.index() == i)
            .unwrap_or_default()
    }

    /// The palettes used for the dark and light modes under this choice.
    /// Only `System` uses both; the others pin one mode (see [`Self::mode`]).
    pub const fn kinds(self) -> (PaletteKind, PaletteKind) {
        match self {
            Self::Classic => (PaletteKind::Classic, PaletteKind::Classic),
            Self::System | Self::Dark | Self::Light => (PaletteKind::Dark, PaletteKind::Light),
            Self::HighContrastDark | Self::HighContrastLight => (
                PaletteKind::HighContrastDark,
                PaletteKind::HighContrastLight,
            ),
            Self::Dusk => (PaletteKind::Dusk, PaletteKind::Light),
            Self::Slate => (PaletteKind::Slate, PaletteKind::Light),
            Self::Paper => (PaletteKind::Dark, PaletteKind::Paper),
            Self::Midnight => (PaletteKind::Midnight, PaletteKind::Light),
            Self::Forest => (PaletteKind::Forest, PaletteKind::Light),
            Self::NightVision => (PaletteKind::NightVision, PaletteKind::Light),
            Self::Phosphor => (PaletteKind::Phosphor, PaletteKind::Light),
            Self::Amber => (PaletteKind::Amber, PaletteKind::Light),
            Self::ColourSafeDark | Self::ColourSafeLight => {
                (PaletteKind::ColourSafeDark, PaletteKind::ColourSafeLight)
            }
            Self::Sand => (PaletteKind::Dark, PaletteKind::Sand),
        }
    }

    /// Whether the theme follows the system's dark or light mode or pins one.
    pub const fn mode(self) -> ModePreference {
        match self {
            Self::System => ModePreference::System,
            Self::Dark
            | Self::HighContrastDark
            | Self::Dusk
            | Self::Slate
            | Self::Midnight
            | Self::Forest
            | Self::NightVision
            | Self::Phosphor
            | Self::Amber
            | Self::ColourSafeDark => ModePreference::Dark,
            Self::Classic
            | Self::Light
            | Self::HighContrastLight
            | Self::Paper
            | Self::Sand
            | Self::ColourSafeLight => ModePreference::Light,
        }
    }
}

/// Whether a theme follows the operating system's dark or light mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModePreference {
    /// Dark or light, as the operating system is.
    System,
    /// Always dark.
    Dark,
    /// Always light.
    Light,
}

/// A concrete palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    /// Classic (SA).
    Classic,
    /// Dark.
    Dark,
    /// Light.
    Light,
    /// High contrast, dark.
    HighContrastDark,
    /// High contrast, light.
    HighContrastLight,
    /// Warm dark.
    Dusk,
    /// Cool dark.
    Slate,
    /// Warm light.
    Paper,
    /// Deep blue dark.
    Midnight,
    /// Green-grey dark.
    Forest,
    /// Red on black.
    NightVision,
    /// Green on black.
    Phosphor,
    /// Amber on black.
    Amber,
    /// Dark with the Okabe-Ito trace colours.
    ColourSafeDark,
    /// Light with the Okabe-Ito trace colours.
    ColourSafeLight,
    /// Warm light sand.
    Sand,
}

impl PaletteKind {
    /// Every palette.
    pub const ALL: [Self; 16] = [
        Self::Classic,
        Self::Dark,
        Self::Light,
        Self::HighContrastDark,
        Self::HighContrastLight,
        Self::Dusk,
        Self::Slate,
        Self::Paper,
        Self::Midnight,
        Self::Forest,
        Self::NightVision,
        Self::Phosphor,
        Self::Amber,
        Self::ColourSafeDark,
        Self::ColourSafeLight,
        Self::Sand,
    ];

    /// The colours of this palette.
    pub const fn palette(self) -> &'static Palette {
        match self {
            Self::Classic => &CLASSIC,
            Self::Dark => &DARK,
            Self::Light => &LIGHT,
            Self::HighContrastDark => &HIGH_CONTRAST_DARK,
            Self::HighContrastLight => &HIGH_CONTRAST_LIGHT,
            Self::Dusk => &DUSK,
            Self::Slate => &SLATE,
            Self::Paper => &PAPER,
            Self::Midnight => &MIDNIGHT,
            Self::Forest => &FOREST,
            Self::NightVision => &NIGHT_VISION,
            Self::Phosphor => &PHOSPHOR,
            Self::Amber => &AMBER,
            Self::ColourSafeDark => &COLOUR_SAFE_DARK,
            Self::ColourSafeLight => &COLOUR_SAFE_LIGHT,
            Self::Sand => &SAND,
        }
    }
}

/// Colours for everything drawn by hand: plots, traces, markers and notices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Plot background.
    pub plot_bg: Rgba8,
    /// Background of small inset plots (eye, constellation tiles).
    pub inset_bg: Rgba8,
    /// Grid lines.
    pub grid: Rgba8,
    /// Axes and zero lines.
    pub axis: Rgba8,
    /// Text drawn on a plot.
    pub plot_text: Rgba8,
    /// Secondary text on a plot (hints, readouts).
    pub plot_text_dim: Rgba8,
    /// Background behind text drawn over a plot or image.
    pub overlay_bg: Rgba8,
    /// Fill of a selected span.
    pub selection: Rgba8,
    /// First trace (I, main curve).
    pub trace_i: Rgba8,
    /// Second trace (Q).
    pub trace_q: Rgba8,
    /// Frequency or third trace.
    pub trace_alt: Rgba8,
    /// Sum or highlight trace.
    pub trace_sum: Rgba8,
    /// Extra trace (signed correlation, comparison curve).
    pub trace_extra: Rgba8,
    /// Constellation and scatter points (alpha is applied by the caller).
    pub points: Rgba8,
    /// Faint overlaid traces (eye diagram, persistence).
    pub faint: Rgba8,
    /// Ideal points, reference symbols and the strobe.
    pub reference: Rgba8,
    /// Cursor, peak and found-value lines.
    pub cursor: Rgba8,
    /// Free (movable) markers: near white or near black, a hue no trace uses, so
    /// a marker never blends into the curve it marks.
    pub marker_free: Rgba8,
    /// Locked markers.
    pub marker_locked: Rgba8,
    /// Additional markers (H markers on the sonogram).
    pub marker_extra: Rgba8,
    /// Inactive bars and disabled plot elements.
    pub inactive: Rgba8,
    /// Warnings.
    pub warn: Rgba8,
    /// Errors.
    pub error: Rgba8,
    /// Success and "good" states.
    pub ok: Rgba8,
}

const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8::rgb(r, g, b)
}

const fn white(a: u8) -> Rgba8 {
    Rgba8::white(a)
}

const fn black(a: u8) -> Rgba8 {
    Rgba8::black(a)
}

/// Classic (SA): black plots with white text, cyan traces, pale yellow markers.
const CLASSIC: Palette = Palette {
    plot_bg: Rgba8::BLACK,
    inset_bg: Rgba8::BLACK,
    grid: white(40),
    axis: white(90),
    plot_text: Rgba8::WHITE,
    plot_text_dim: white(190),
    overlay_bg: black(170),
    selection: white(30),
    trace_i: rgb(0, 255, 255),
    trace_q: rgb(255, 128, 255),
    trace_alt: rgb(0, 255, 0),
    trace_sum: rgb(255, 255, 0),
    trace_extra: rgb(100, 149, 237),
    points: rgb(0, 255, 0),
    faint: rgb(0, 200, 0),
    reference: rgb(255, 255, 0),
    cursor: rgb(255, 0, 0),
    marker_free: rgb(255, 255, 170),
    marker_locked: rgb(255, 40, 40),
    marker_extra: rgb(230, 230, 230),
    inactive: rgb(110, 110, 110),
    warn: rgb(200, 110, 0),
    error: rgb(210, 0, 0),
    ok: rgb(0, 160, 0),
};

const DARK: Palette = Palette {
    plot_bg: rgb(10, 12, 18),
    inset_bg: rgb(12, 12, 18),
    grid: white(22),
    axis: white(40),
    plot_text: white(225),
    plot_text_dim: white(150),
    overlay_bg: black(150),
    selection: white(32),
    trace_i: rgb(80, 220, 255),
    trace_q: rgb(255, 170, 60),
    trace_alt: rgb(90, 230, 120),
    trace_sum: rgb(240, 220, 60),
    trace_extra: rgb(90, 140, 230),
    points: rgb(120, 220, 255),
    faint: rgb(90, 230, 120),
    reference: rgb(255, 220, 120),
    cursor: rgb(255, 90, 90),
    marker_free: rgb(235, 235, 235),
    marker_locked: rgb(255, 70, 70),
    marker_extra: rgb(200, 120, 255),
    inactive: rgb(90, 90, 90),
    warn: rgb(240, 190, 80),
    error: rgb(255, 128, 128),
    ok: rgb(90, 230, 120),
};

const LIGHT: Palette = Palette {
    plot_bg: rgb(250, 250, 252),
    inset_bg: rgb(242, 243, 247),
    grid: black(26),
    axis: black(70),
    plot_text: black(200),
    plot_text_dim: black(150),
    overlay_bg: Rgba8::unmultiplied(240, 240, 240, 220),
    selection: black(18),
    trace_i: rgb(0, 110, 180),
    trace_q: rgb(200, 100, 0),
    trace_alt: rgb(20, 140, 60),
    trace_sum: rgb(150, 120, 0),
    trace_extra: rgb(60, 80, 200),
    points: rgb(0, 100, 170),
    faint: rgb(20, 140, 60),
    reference: rgb(170, 100, 0),
    cursor: rgb(210, 30, 30),
    marker_free: rgb(30, 30, 30),
    marker_locked: rgb(200, 0, 0),
    marker_extra: rgb(130, 40, 200),
    inactive: rgb(170, 170, 170),
    warn: rgb(170, 100, 0),
    error: rgb(200, 0, 0),
    ok: rgb(20, 130, 50),
};

const HIGH_CONTRAST_DARK: Palette = Palette {
    plot_bg: Rgba8::BLACK,
    inset_bg: Rgba8::BLACK,
    grid: white(70),
    axis: white(140),
    plot_text: Rgba8::WHITE,
    plot_text_dim: white(220),
    overlay_bg: black(230),
    selection: white(40),
    trace_i: rgb(0, 230, 255),
    trace_q: rgb(255, 170, 0),
    trace_alt: rgb(0, 255, 100),
    trace_sum: rgb(255, 255, 0),
    trace_extra: rgb(130, 170, 255),
    points: rgb(0, 230, 255),
    faint: rgb(0, 255, 100),
    reference: rgb(255, 255, 0),
    cursor: rgb(255, 60, 60),
    marker_free: Rgba8::WHITE,
    marker_locked: rgb(255, 40, 40),
    marker_extra: rgb(255, 80, 255),
    inactive: rgb(150, 150, 150),
    warn: rgb(255, 200, 0),
    error: rgb(255, 90, 90),
    ok: rgb(0, 255, 100),
};

const HIGH_CONTRAST_LIGHT: Palette = Palette {
    plot_bg: Rgba8::WHITE,
    inset_bg: Rgba8::WHITE,
    grid: black(60),
    axis: black(160),
    plot_text: Rgba8::BLACK,
    plot_text_dim: black(220),
    overlay_bg: Rgba8::unmultiplied(255, 255, 255, 235),
    selection: black(40),
    trace_i: rgb(0, 60, 160),
    trace_q: rgb(170, 60, 0),
    trace_alt: rgb(0, 110, 30),
    trace_sum: rgb(110, 80, 0),
    trace_extra: rgb(70, 0, 170),
    points: rgb(0, 60, 160),
    faint: rgb(0, 110, 30),
    reference: rgb(150, 70, 0),
    cursor: rgb(190, 0, 0),
    marker_free: Rgba8::BLACK,
    marker_locked: rgb(190, 0, 0),
    marker_extra: rgb(120, 0, 160),
    inactive: rgb(110, 110, 110),
    warn: rgb(140, 70, 0),
    error: rgb(180, 0, 0),
    ok: rgb(0, 110, 30),
};

const DUSK: Palette = Palette {
    plot_bg: rgb(22, 18, 16),
    inset_bg: rgb(26, 21, 18),
    grid: white(22),
    axis: white(40),
    plot_text: rgb(235, 220, 200),
    plot_text_dim: rgb(190, 175, 155),
    overlay_bg: black(160),
    selection: Rgba8::premultiplied(48, 36, 16, 48),
    trace_i: rgb(255, 190, 90),
    trace_q: rgb(120, 200, 255),
    trace_alt: rgb(170, 220, 110),
    trace_sum: rgb(250, 225, 70),
    trace_extra: rgb(230, 140, 110),
    points: rgb(255, 200, 110),
    faint: rgb(170, 220, 110),
    reference: rgb(140, 220, 255),
    cursor: rgb(255, 100, 80),
    marker_free: rgb(240, 232, 220),
    marker_locked: rgb(255, 90, 70),
    marker_extra: rgb(220, 150, 255),
    inactive: rgb(100, 90, 80),
    warn: rgb(255, 190, 90),
    error: rgb(255, 120, 100),
    ok: rgb(160, 230, 110),
};

const SLATE: Palette = Palette {
    plot_bg: rgb(16, 22, 30),
    inset_bg: rgb(19, 26, 35),
    grid: white(24),
    axis: white(45),
    plot_text: rgb(205, 220, 235),
    plot_text_dim: rgb(160, 178, 196),
    overlay_bg: black(150),
    selection: Rgba8::premultiplied(18, 44, 56, 56),
    trace_i: rgb(80, 225, 210),
    trace_q: rgb(255, 160, 100),
    trace_alt: rgb(140, 230, 140),
    trace_sum: rgb(240, 225, 110),
    trace_extra: rgb(120, 160, 255),
    points: rgb(90, 225, 210),
    faint: rgb(140, 230, 140),
    reference: rgb(255, 210, 130),
    cursor: rgb(255, 100, 110),
    marker_free: rgb(225, 235, 245),
    marker_locked: rgb(255, 80, 90),
    marker_extra: rgb(190, 140, 255),
    inactive: rgb(80, 92, 105),
    warn: rgb(245, 195, 90),
    error: rgb(255, 125, 125),
    ok: rgb(120, 230, 150),
};

const PAPER: Palette = Palette {
    plot_bg: rgb(251, 248, 240),
    inset_bg: rgb(245, 240, 228),
    grid: Rgba8::premultiplied(30, 25, 10, 30),
    axis: Rgba8::premultiplied(40, 30, 15, 80),
    plot_text: rgb(50, 42, 30),
    plot_text_dim: rgb(105, 95, 80),
    overlay_bg: Rgba8::unmultiplied(245, 240, 228, 225),
    selection: Rgba8::premultiplied(25, 20, 5, 25),
    trace_i: rgb(30, 80, 150),
    trace_q: rgb(180, 80, 20),
    trace_alt: rgb(40, 120, 50),
    trace_sum: rgb(140, 100, 0),
    trace_extra: rgb(110, 50, 140),
    points: rgb(30, 80, 150),
    faint: rgb(40, 120, 50),
    reference: rgb(170, 90, 0),
    cursor: rgb(190, 30, 30),
    marker_free: rgb(40, 32, 22),
    marker_locked: rgb(190, 20, 20),
    marker_extra: rgb(120, 40, 170),
    inactive: rgb(175, 165, 150),
    warn: rgb(160, 90, 0),
    error: rgb(180, 20, 20),
    ok: rgb(40, 120, 50),
};

const MIDNIGHT: Palette = Palette {
    plot_bg: rgb(7, 10, 26),
    inset_bg: rgb(10, 14, 34),
    grid: Rgba8::premultiplied(24, 24, 24, 24),
    axis: Rgba8::premultiplied(48, 48, 48, 48),
    plot_text: rgb(212, 222, 248),
    plot_text_dim: rgb(165, 178, 210),
    overlay_bg: Rgba8::premultiplied(0, 0, 0, 165),
    selection: Rgba8::premultiplied(14, 20, 44, 44),
    trace_i: rgb(110, 200, 255),
    trace_q: rgb(255, 175, 90),
    trace_alt: rgb(120, 235, 170),
    trace_sum: rgb(245, 225, 110),
    trace_extra: rgb(175, 150, 255),
    points: rgb(110, 200, 255),
    faint: rgb(120, 235, 170),
    reference: rgb(255, 215, 130),
    cursor: rgb(255, 105, 125),
    marker_free: rgb(240, 246, 255),
    marker_locked: rgb(255, 85, 100),
    marker_extra: rgb(215, 140, 255),
    inactive: rgb(70, 80, 110),
    warn: rgb(250, 200, 90),
    error: rgb(255, 125, 135),
    ok: rgb(120, 235, 170),
};

const FOREST: Palette = Palette {
    plot_bg: rgb(10, 18, 13),
    inset_bg: rgb(13, 22, 16),
    grid: Rgba8::premultiplied(22, 22, 22, 22),
    axis: Rgba8::premultiplied(45, 45, 45, 45),
    plot_text: rgb(215, 230, 210),
    plot_text_dim: rgb(168, 188, 165),
    overlay_bg: Rgba8::premultiplied(0, 0, 0, 160),
    selection: Rgba8::premultiplied(16, 30, 16, 40),
    trace_i: rgb(150, 225, 120),
    trace_q: rgb(255, 180, 100),
    trace_alt: rgb(110, 200, 240),
    trace_sum: rgb(240, 220, 110),
    trace_extra: rgb(200, 160, 240),
    points: rgb(150, 225, 120),
    faint: rgb(110, 200, 240),
    reference: rgb(255, 220, 140),
    cursor: rgb(255, 110, 100),
    marker_free: rgb(250, 250, 235),
    marker_locked: rgb(255, 90, 80),
    marker_extra: rgb(225, 150, 255),
    inactive: rgb(75, 95, 80),
    warn: rgb(245, 195, 90),
    error: rgb(255, 125, 115),
    ok: rgb(150, 225, 120),
};

const NIGHT_VISION: Palette = Palette {
    plot_bg: rgb(0, 0, 0),
    inset_bg: rgb(6, 0, 0),
    grid: Rgba8::premultiplied(40, 6, 5, 40),
    axis: Rgba8::premultiplied(90, 21, 14, 90),
    plot_text: rgb(255, 120, 95),
    plot_text_dim: rgb(225, 85, 65),
    overlay_bg: Rgba8::premultiplied(0, 0, 0, 215),
    selection: Rgba8::premultiplied(70, 10, 6, 70),
    trace_i: rgb(255, 60, 40),
    trace_q: rgb(220, 110, 40),
    trace_alt: rgb(200, 40, 90),
    trace_sum: rgb(255, 140, 60),
    trace_extra: rgb(210, 50, 130),
    points: rgb(255, 60, 40),
    faint: rgb(200, 40, 90),
    reference: rgb(255, 150, 90),
    cursor: rgb(255, 200, 170),
    marker_free: rgb(255, 215, 195),
    marker_locked: rgb(255, 160, 120),
    marker_extra: rgb(255, 110, 170),
    inactive: rgb(110, 25, 20),
    warn: rgb(255, 150, 60),
    error: rgb(255, 110, 90),
    ok: rgb(230, 90, 60),
};

const PHOSPHOR: Palette = Palette {
    plot_bg: rgb(1, 8, 3),
    inset_bg: rgb(2, 12, 5),
    grid: Rgba8::premultiplied(8, 32, 13, 32),
    axis: Rgba8::premultiplied(19, 80, 31, 80),
    plot_text: rgb(150, 255, 170),
    plot_text_dim: rgb(100, 215, 125),
    overlay_bg: Rgba8::premultiplied(0, 0, 0, 210),
    selection: Rgba8::premultiplied(8, 36, 12, 40),
    trace_i: rgb(60, 240, 100),
    trace_q: rgb(170, 255, 90),
    trace_alt: rgb(40, 200, 170),
    trace_sum: rgb(160, 235, 60),
    trace_extra: rgb(90, 220, 210),
    points: rgb(60, 240, 100),
    faint: rgb(40, 200, 170),
    reference: rgb(200, 255, 150),
    cursor: rgb(240, 255, 200),
    marker_free: rgb(235, 255, 235),
    marker_locked: rgb(255, 230, 120),
    marker_extra: rgb(140, 255, 230),
    inactive: rgb(25, 85, 40),
    warn: rgb(220, 255, 90),
    error: rgb(255, 170, 120),
    ok: rgb(90, 255, 130),
};

const AMBER: Palette = Palette {
    plot_bg: rgb(10, 6, 0),
    inset_bg: rgb(14, 9, 2),
    grid: Rgba8::premultiplied(30, 21, 0, 30),
    axis: Rgba8::premultiplied(80, 55, 0, 80),
    plot_text: rgb(255, 200, 110),
    plot_text_dim: rgb(225, 165, 75),
    overlay_bg: Rgba8::premultiplied(0, 0, 0, 210),
    selection: Rgba8::premultiplied(40, 26, 4, 42),
    trace_i: rgb(255, 176, 0),
    trace_q: rgb(255, 120, 40),
    trace_alt: rgb(220, 200, 90),
    trace_sum: rgb(255, 205, 60),
    trace_extra: rgb(235, 140, 90),
    points: rgb(255, 176, 0),
    faint: rgb(220, 200, 90),
    reference: rgb(255, 230, 160),
    cursor: rgb(255, 245, 215),
    marker_free: rgb(255, 250, 230),
    marker_locked: rgb(255, 110, 60),
    marker_extra: rgb(255, 160, 200),
    inactive: rgb(95, 65, 15),
    warn: rgb(255, 215, 90),
    error: rgb(255, 120, 80),
    ok: rgb(220, 210, 90),
};

const COLOUR_SAFE_DARK: Palette = Palette {
    plot_bg: rgb(12, 12, 16),
    inset_bg: rgb(16, 16, 21),
    grid: Rgba8::premultiplied(24, 24, 24, 24),
    axis: Rgba8::premultiplied(48, 48, 48, 48),
    plot_text: rgb(232, 232, 232),
    plot_text_dim: rgb(180, 180, 185),
    overlay_bg: Rgba8::premultiplied(0, 0, 0, 160),
    selection: Rgba8::premultiplied(30, 30, 30, 30),
    trace_i: rgb(86, 180, 233),
    trace_q: rgb(230, 159, 0),
    trace_alt: rgb(0, 158, 115),
    trace_sum: rgb(240, 228, 66),
    trace_extra: rgb(204, 121, 167),
    points: rgb(86, 180, 233),
    faint: rgb(0, 158, 115),
    reference: rgb(240, 228, 66),
    cursor: rgb(213, 94, 0),
    marker_free: rgb(255, 255, 255),
    marker_locked: rgb(213, 94, 0),
    marker_extra: rgb(204, 121, 167),
    inactive: rgb(85, 85, 90),
    warn: rgb(230, 159, 0),
    error: rgb(240, 110, 40),
    ok: rgb(0, 170, 125),
};

const SAND: Palette = Palette {
    plot_bg: rgb(250, 245, 232),
    inset_bg: rgb(243, 236, 218),
    grid: Rgba8::premultiplied(30, 22, 6, 30),
    axis: Rgba8::premultiplied(45, 32, 10, 85),
    plot_text: rgb(58, 44, 26),
    plot_text_dim: rgb(110, 92, 66),
    overlay_bg: Rgba8::premultiplied(220, 215, 200, 228),
    selection: Rgba8::premultiplied(24, 16, 2, 26),
    trace_i: rgb(20, 90, 140),
    trace_q: rgb(185, 75, 20),
    trace_alt: rgb(30, 120, 70),
    trace_sum: rgb(130, 95, 0),
    trace_extra: rgb(120, 50, 140),
    points: rgb(20, 90, 140),
    faint: rgb(30, 120, 70),
    reference: rgb(170, 90, 0),
    cursor: rgb(185, 30, 40),
    marker_free: rgb(40, 30, 18),
    marker_locked: rgb(185, 25, 25),
    marker_extra: rgb(115, 40, 165),
    inactive: rgb(180, 165, 140),
    warn: rgb(160, 90, 0),
    error: rgb(180, 20, 20),
    ok: rgb(30, 120, 70),
};

const COLOUR_SAFE_LIGHT: Palette = Palette {
    plot_bg: rgb(252, 252, 252),
    inset_bg: rgb(244, 244, 246),
    grid: Rgba8::premultiplied(0, 0, 0, 26),
    axis: Rgba8::premultiplied(0, 0, 0, 75),
    plot_text: Rgba8::premultiplied(0, 0, 0, 215),
    plot_text_dim: Rgba8::premultiplied(0, 0, 0, 160),
    overlay_bg: Rgba8::premultiplied(219, 219, 219, 228),
    selection: Rgba8::premultiplied(0, 0, 0, 20),
    trace_i: rgb(0, 114, 178),
    trace_q: rgb(213, 94, 0),
    trace_alt: rgb(0, 140, 100),
    trace_sum: rgb(140, 110, 0),
    trace_extra: rgb(170, 70, 130),
    points: rgb(0, 114, 178),
    faint: rgb(0, 140, 100),
    reference: rgb(170, 100, 0),
    cursor: rgb(170, 40, 0),
    marker_free: rgb(20, 20, 20),
    marker_locked: rgb(213, 94, 0),
    marker_extra: rgb(150, 60, 120),
    inactive: rgb(170, 170, 170),
    warn: rgb(160, 90, 0),
    error: rgb(190, 30, 0),
    ok: rgb(0, 130, 95),
};

/// The palette `choice` uses in dark (`dark_mode`) or light mode.
pub fn palette_for(choice: ThemeChoice, dark_mode: bool) -> &'static Palette {
    let (dark, light) = choice.kinds();
    if dark_mode {
        dark.palette()
    } else {
        light.palette()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Rgba8) -> f64 {
        c.luminance()
    }

    fn over(c: Rgba8, bg: Rgba8) -> Rgba8 {
        c.over(bg)
    }

    fn contrast(a: Rgba8, b: Rgba8) -> f64 {
        a.contrast_over(b)
    }

    const KINDS: [PaletteKind; 16] = PaletteKind::ALL;

    #[test]
    fn every_theme_is_in_a_menu_group_and_pinned_themes_use_their_own_palette() {
        let mut groups: Vec<u8> = ThemeChoice::ALL.iter().map(|t| t.group()).collect();
        let sorted = {
            let mut g = groups.clone();
            g.sort_unstable();
            g
        };
        // Groups are contiguous in menu order.
        assert_eq!(groups, sorted);
        groups.dedup();
        assert_eq!(groups, vec![0, 1, 2, 3]);
        for (choice, dark, palette) in [
            (ThemeChoice::NightVision, true, &NIGHT_VISION),
            (ThemeChoice::Phosphor, true, &PHOSPHOR),
            (ThemeChoice::Amber, true, &AMBER),
            (ThemeChoice::Midnight, true, &MIDNIGHT),
            (ThemeChoice::Forest, true, &FOREST),
            (ThemeChoice::ColourSafeDark, true, &COLOUR_SAFE_DARK),
            (ThemeChoice::ColourSafeLight, false, &COLOUR_SAFE_LIGHT),
            (ThemeChoice::Sand, false, &SAND),
        ] {
            assert_eq!(palette_for(choice, dark), palette, "{choice:?}");
        }
    }

    #[test]
    fn colour_safe_traces_use_the_okabe_ito_colours() {
        // Sky blue, orange, bluish green, yellow and reddish purple.
        let p = &COLOUR_SAFE_DARK;
        assert_eq!(p.trace_i, rgb(86, 180, 233));
        assert_eq!(p.trace_q, rgb(230, 159, 0));
        assert_eq!(p.trace_alt, rgb(0, 158, 115));
        assert_eq!(p.trace_sum, rgb(240, 228, 66));
        assert_eq!(p.trace_extra, rgb(204, 121, 167));
    }

    fn fields(p: &Palette) -> [(&'static str, Rgba8); 24] {
        [
            ("plot_bg", p.plot_bg),
            ("inset_bg", p.inset_bg),
            ("grid", p.grid),
            ("axis", p.axis),
            ("plot_text", p.plot_text),
            ("plot_text_dim", p.plot_text_dim),
            ("overlay_bg", p.overlay_bg),
            ("selection", p.selection),
            ("trace_i", p.trace_i),
            ("trace_q", p.trace_q),
            ("trace_alt", p.trace_alt),
            ("trace_sum", p.trace_sum),
            ("trace_extra", p.trace_extra),
            ("points", p.points),
            ("faint", p.faint),
            ("reference", p.reference),
            ("cursor", p.cursor),
            ("marker_free", p.marker_free),
            ("marker_locked", p.marker_locked),
            ("marker_extra", p.marker_extra),
            ("inactive", p.inactive),
            ("warn", p.warn),
            ("error", p.error),
            ("ok", p.ok),
        ]
    }

    #[test]
    fn every_palette_colour_is_validly_premultiplied() {
        // A colour channel above alpha adds light instead of covering.
        for kind in KINDS {
            for (name, c) in fields(kind.palette()) {
                assert!(
                    c.r <= c.a && c.g <= c.a && c.b <= c.a,
                    "{kind:?}.{name}: {c:?} is not premultiplied"
                );
            }
        }
    }

    #[test]
    fn plot_text_is_readable_on_plots_and_overlays() {
        for kind in KINDS {
            let p = kind.palette();
            let r = contrast(p.plot_text_dim, p.plot_bg);
            assert!(r >= 4.5, "{kind:?}: dim text contrast {r:.2} < 4.5");
            // Text on the backing box of a notice, drawn over the plot.
            let boxed = over(p.overlay_bg, p.plot_bg);
            let r = contrast(p.plot_text, boxed);
            assert!(r >= 4.5, "{kind:?}: overlay text contrast {r:.2} < 4.5");
            // Dim text must look dimmer than text.
            let (t, d) = (
                luminance(over(p.plot_text, p.plot_bg)),
                luminance(over(p.plot_text_dim, p.plot_bg)),
            );
            assert!(
                (t - d).abs() > 0.05
                    || kind == PaletteKind::HighContrastDark
                    || kind == PaletteKind::HighContrastLight,
                "{kind:?}: dim text looks like text"
            );
        }
    }

    #[test]
    fn free_markers_differ_from_every_trace() {
        let distance = |a: Rgba8, b: Rgba8| {
            let d = |x: u8, y: u8| (f64::from(x) - f64::from(y)).powi(2);
            (d(a.r, b.r) + d(a.g, b.g) + d(a.b, b.b)).sqrt()
        };
        for kind in KINDS {
            let p = kind.palette();
            for (name, c) in [
                ("trace_i", p.trace_i),
                ("trace_q", p.trace_q),
                ("trace_alt", p.trace_alt),
                ("trace_sum", p.trace_sum),
                ("trace_extra", p.trace_extra),
                ("faint", p.faint),
            ] {
                // A different hue, or clearly lighter or darker.
                let d = distance(p.marker_free, c);
                let r = contrast(p.marker_free, c);
                assert!(
                    d >= 120.0 || r >= 2.0,
                    "{kind:?}: marker_free is close to {name} (distance {d:.0}, contrast {r:.2})"
                );
            }
        }
    }

    #[test]
    fn image_overlays_stand_out_from_dark_colour_map_pixels() {
        // The low end of every colour map is near black whatever the theme.
        let darkest = [rgb(4, 8, 64), rgb(0, 0, 4), rgb(0, 0, 0), rgb(68, 1, 84)];
        // `image_palette` is the dark palette of the theme in every mode
        // (the global choice is not switched here: tests run in parallel).
        for choice in ThemeChoice::ALL {
            let p = palette_for(choice, true);
            for bg in darkest {
                for (name, c, min) in [
                    ("marker_free", p.marker_free, 4.5),
                    ("marker_locked", p.marker_locked, 3.0),
                    ("marker_extra", p.marker_extra, 3.0),
                    ("axis", p.axis, 1.4),
                    ("selection", p.selection, 1.1),
                ] {
                    let r = contrast(c, bg);
                    assert!(r >= min, "{choice:?}: {name} on {bg:?}: {r:.2} < {min}");
                }
            }
        }
    }

    #[test]
    fn index_round_trips_and_pinned_themes_pick_their_mode() {
        for t in ThemeChoice::ALL {
            assert_eq!(ThemeChoice::from_index(t.index()), t);
        }
        assert_eq!(ThemeChoice::from_index(200), ThemeChoice::Classic);
        assert_eq!(
            palette_for(ThemeChoice::HighContrastDark, true),
            &HIGH_CONTRAST_DARK
        );
        assert_eq!(palette_for(ThemeChoice::Paper, false), &PAPER);
        assert_eq!(palette_for(ThemeChoice::System, false), &LIGHT);
    }

    #[test]
    fn traces_and_markers_stand_out_from_the_plot_background() {
        for kind in KINDS {
            let p = kind.palette();
            let min = if matches!(
                kind,
                PaletteKind::HighContrastDark | PaletteKind::HighContrastLight
            ) {
                4.5
            } else {
                3.0
            };
            for (name, c) in [
                ("trace_i", p.trace_i),
                ("trace_q", p.trace_q),
                ("trace_alt", p.trace_alt),
                ("trace_sum", p.trace_sum),
                ("points", p.points),
                ("reference", p.reference),
                ("cursor", p.cursor),
                ("marker_free", p.marker_free),
                ("marker_locked", p.marker_locked),
                ("plot_text", p.plot_text),
                ("warn", p.warn),
                ("error", p.error),
            ] {
                let r = contrast(c, p.plot_bg);
                assert!(r >= min, "{kind:?}.{name}: contrast {r:.2} < {min}");
            }
        }
    }
}
