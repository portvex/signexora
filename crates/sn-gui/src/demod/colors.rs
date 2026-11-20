//! The colours SA's module windows (demodulator, `SA-OFDM`) set themselves:
//! black plots with a cyan fixed font, coloured level numbers, the OFDM
//! results list and List of K, and the tinted fields. SA's own values under
//! the Classic theme (estimated from the screenshots, as `sa-app`'s
//! `ModuleColors`); built from the theme's plot palette and scheme otherwise.

use gpui::{App, Hsla};
use sn_ui_core::theme::ThemeChoice;

use super::Frame;
use crate::win95::colors::{Rgb, Role, hsla, palette, scheme};

/// The module colours.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ModColors {
    /// Plot and text panel background.
    pub(crate) plot_bg: Hsla,
    /// Fixed-font text on the plots and text panels.
    pub(crate) text: Hsla,
    /// Demodulator level lines and their handles.
    pub(crate) level_line: Hsla,
    pub(crate) level_handle: Rgb,
    /// Level numbers: the highest level and the lowest (blended between).
    pub(crate) level_top: Rgb,
    pub(crate) level_bottom: Rgb,
    /// Demodulator points: near the level centre, between, far from it.
    pub(crate) point_core: Hsla,
    pub(crate) point_mid: Hsla,
    pub(crate) point_edge: Hsla,
    /// OFDM correlation triangle and its LU marker.
    pub(crate) curve: Hsla,
    pub(crate) lu_marker: Hsla,
    /// LU spectrum bars of even and odd channels.
    pub(crate) even: Hsla,
    pub(crate) odd: Hsla,
    /// Channel markers: the red one, the green (selected) one, both (purple).
    pub(crate) mark_other: Hsla,
    pub(crate) mark_selected: Hsla,
    pub(crate) mark_both: Hsla,
    /// The Survey spectrum.
    pub(crate) survey: Hsla,
    /// Line between the triangle and the spectrum.
    pub(crate) divider: Hsla,
    /// OFDM results list.
    pub(crate) list_bg: Hsla,
    pub(crate) list_text: Hsla,
    pub(crate) list_sel_bg: Hsla,
    pub(crate) list_sel_text: Hsla,
    /// OFDM List of K.
    pub(crate) k_bg: Hsla,
    pub(crate) k_text: Hsla,
    /// Tinted fields and scroll bars: Symbols (cyan), Scale FFT (lavender),
    /// read-only values.
    pub(crate) field_cyan: Rgb,
    pub(crate) field_lavender: Rgb,
    pub(crate) field_value: Rgb,
    /// Text in the tinted fields.
    pub(crate) field_text: Rgb,
    /// Constellation border when points fall outside it.
    pub(crate) overflow: Hsla,
    /// `Don't panic! Please wait...`, its note and its blocks.
    pub(crate) wait_text: Hsla,
    pub(crate) wait_note: Hsla,
    pub(crate) wait_bar: Hsla,
    /// Errors drawn over a plot.
    pub(crate) error: Hsla,
}

/// SA's module colours.
fn classic(cx: &App) -> ModColors {
    ModColors {
        plot_bg: hsla(0x00_00_00),
        text: hsla(0x00_FF_FF),
        level_line: hsla(0xFF_FF_FF),
        level_handle: 0x00_50_FF,
        level_top: 0xE6_1E_1E,
        level_bottom: 0x28_6E_FF,
        point_core: hsla(0xFF_FF_FF),
        point_mid: hsla(0xFF_FF_3C),
        point_edge: hsla(0x28_C8_28),
        curve: hsla(0x3C_DC_3C),
        lu_marker: hsla(0xE6_D2_28),
        even: hsla(0x32_D2_D2),
        odd: hsla(0x64_64_EB),
        mark_other: hsla(0xEB_14_14),
        mark_selected: hsla(0x14_EB_14),
        mark_both: hsla(0xAA_28_C8),
        survey: hsla(0x14_EB_14),
        divider: hsla(0x00_3C_FF),
        list_bg: hsla(0xB8_CC_F4),
        list_text: hsla(0x00_00_60),
        list_sel_bg: hsla(0x00_00_80),
        list_sel_text: hsla(0xFF_FF_FF),
        k_bg: hsla(0x00_00_80),
        k_text: hsla(0xFF_FF_00),
        field_cyan: 0xB0_F0_FF,
        field_lavender: 0xCC_CC_FF,
        field_value: 0xD6_E2_F4,
        field_text: 0x00_00_00,
        overflow: hsla(0xE6_14_14),
        wait_text: hsla(0x00_E6_E6),
        wait_note: hsla(0xFF_FF_FF),
        wait_bar: hsla(0x14_C8_14),
        error: hsla8(palette(cx).error),
    }
}

/// A palette colour (premultiplied) as a GPUI colour.
pub(crate) fn hsla8(c: sn_ui_core::color::Rgba8) -> Hsla {
    let a = f32::from(c.a) / 255.0;
    let un = |v: u8| {
        if c.a == 0 {
            0.0
        } else {
            (f32::from(v) / 255.0 / a).min(1.0)
        }
    };
    gpui::Rgba {
        r: un(c.r),
        g: un(c.g),
        b: un(c.b),
        a,
    }
    .into()
}

/// `0xRRGGBB` of a palette colour.
fn rgb8(c: sn_ui_core::color::Rgba8) -> Rgb {
    crate::win95::colors::rgb_of(c)
}

impl Frame {
    /// The module colours under the current theme.
    pub(crate) fn dm_colors(&self, cx: &App) -> ModColors {
        if self.prefs.theme == ThemeChoice::Classic {
            return classic(cx);
        }
        let p = palette(cx);
        let s = scheme(cx);
        ModColors {
            plot_bg: hsla8(p.plot_bg),
            text: hsla8(p.plot_text),
            level_line: hsla8(p.axis),
            level_handle: rgb8(p.marker_extra),
            level_top: rgb8(p.error),
            level_bottom: rgb8(p.trace_i),
            point_core: hsla8(p.plot_text),
            point_mid: hsla8(p.trace_sum),
            point_edge: hsla8(p.points),
            curve: hsla8(p.trace_alt),
            lu_marker: hsla8(p.cursor),
            even: hsla8(p.trace_i),
            odd: hsla8(p.trace_q),
            mark_other: hsla8(p.error),
            mark_selected: hsla8(p.ok),
            mark_both: hsla8(p.marker_extra),
            survey: hsla8(p.points),
            divider: hsla8(p.axis),
            list_bg: s.of(Role::Window),
            list_text: s.of(Role::Text),
            list_sel_bg: s.of(Role::Selection),
            list_sel_text: s.of(Role::SelectionText),
            k_bg: hsla8(p.plot_bg),
            k_text: hsla8(p.trace_sum),
            field_cyan: s.rgb(Role::Window),
            field_lavender: s.rgb(Role::Window),
            field_value: s.rgb(Role::Window),
            field_text: s.rgb(Role::Text),
            overflow: hsla8(p.error),
            wait_text: hsla8(p.trace_extra),
            wait_note: hsla8(p.plot_text),
            wait_bar: hsla8(p.ok),
            error: hsla8(p.error),
        }
    }
}

/// The colour a fraction `f` (0..1) of the way from `a` to `b`.
pub(crate) fn blend(a: Rgb, b: Rgb, f: f64) -> Rgb {
    crate::win95::colors::mix(a, b, f.clamp(0.0, 1.0))
}
