//! The user-interface fonts and text drawing.
//!
//! Windows 95 and 98 drew their user interface in MS Sans Serif 8 pt, a
//! proprietary bitmap font that SigNexora must not ship. SigNexora embeds "W95FA
//! Grid", a pixel-grid copy of the OFL-licensed W95FA lookalike (see
//! `assets/fonts/OFL.txt`). At 12.5 px every font unit grid step is one
//! logical pixel, so the letters land on whole pixels and stay crisp at 1x
//! and 2x. From XP on the styles use Tahoma, Trebuchet MS and Segoe UI when
//! installed, else the bundled Selawik (`assets/fonts/selawik/OFL.txt`); the
//! macOS looks use Lucida Grande, Helvetica Neue or the system font on a Mac;
//! the Linux looks Cantarell, Noto Sans or `DejaVu Sans`, bundled as subsets
//! (`assets/fonts/linux/README.txt`).
//! Japanese and Korean text falls back to the bundled Noto subsets.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;

use gpui::{
    App, Font, FontFallbacks, FontWeight, Hsla, Pixels, Point, ShapedLine, SharedString, TextRun,
    Window, point, px,
};

/// The family name of the embedded user-interface font.
pub(crate) const FAMILY: &str = "W95FA Grid";
/// Font size: one pixel of the font's grid per logical pixel.
pub(crate) const FONT_SIZE: f32 = 12.5;
/// Height of a line of text (MS Sans Serif 8 pt: 13 px).
pub(crate) const LINE: i32 = 13;
/// Baseline offset from the top of a line.
pub(crate) const ASCENT: i32 = 11;

const REGULAR: &[u8] = include_bytes!("../../assets/fonts/W95FAGrid-Regular.otf");
const BOLD: &[u8] = include_bytes!("../../assets/fonts/W95FAGrid-Bold.otf");
const SELAWIK: &[u8] = include_bytes!("../../assets/fonts/selawik/Selawik-Regular.ttf");
const SELAWIK_BOLD: &[u8] = include_bytes!("../../assets/fonts/selawik/Selawik-Bold.ttf");
/// Microsoft's open stand-in for Segoe UI (OFL), used for the style fonts that
/// are not installed (Segoe UI outside Windows, Tahoma and Trebuchet MS where
/// they are missing).
const FALLBACK: &str = "Selawik";
const CANTARELL: &[u8] = include_bytes!("../../assets/fonts/linux/Cantarell-Regular-subset.ttf");
const CANTARELL_BOLD: &[u8] = include_bytes!("../../assets/fonts/linux/Cantarell-Bold-subset.ttf");
const NOTO_SANS: &[u8] = include_bytes!("../../assets/fonts/linux/NotoSans-Regular-subset.ttf");
const NOTO_SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/linux/NotoSans-Bold-subset.ttf");
const DEJAVU: &[u8] = include_bytes!("../../assets/fonts/linux/DejaVuSans-subset.ttf");
const DEJAVU_BOLD: &[u8] = include_bytes!("../../assets/fonts/linux/DejaVuSans-Bold-subset.ttf");
const NOTO_JP: &[u8] = include_bytes!("../../assets/fonts/noto/NotoSansJP-Regular-subset.ttf");
const NOTO_KR: &[u8] = include_bytes!("../../assets/fonts/noto/NotoSansKR-Regular-subset.ttf");

/// Register the embedded fonts with GPUI.
pub(crate) fn load_fonts(cx: &App) -> Result<(), String> {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(REGULAR),
            Cow::Borrowed(BOLD),
            Cow::Borrowed(SELAWIK),
            Cow::Borrowed(SELAWIK_BOLD),
            Cow::Borrowed(CANTARELL),
            Cow::Borrowed(CANTARELL_BOLD),
            Cow::Borrowed(NOTO_SANS),
            Cow::Borrowed(NOTO_SANS_BOLD),
            Cow::Borrowed(DEJAVU),
            Cow::Borrowed(DEJAVU_BOLD),
            Cow::Borrowed(NOTO_JP),
            Cow::Borrowed(NOTO_KR),
        ])
        .map_err(|e| e.to_string())?;
    let names: HashSet<String> = cx.text_system().all_font_names().into_iter().collect();
    INSTALLED.with(|i| *i.borrow_mut() = Some(names));
    Ok(())
}

thread_local! {
    /// The font families the system has, read once by [`load_fonts`]; `None`
    /// until then (tests), when every family counts as installed.
    static INSTALLED: RefCell<Option<HashSet<String>>> = const { RefCell::new(None) };
}

/// Windows 11's user-interface font.
const VARIABLE: &str = "Segoe UI Variable Text";

/// The Mac's system font (San Francisco) as GPUI names it on macOS.
const SYSTEM_UI: &str = ".SystemUIFont";

/// Whether `family` is installed (always, before the fonts are read). The
/// Mac's system font is always there on macOS and nowhere else.
fn is_installed(family: &str) -> bool {
    if family == SYSTEM_UI {
        return cfg!(target_os = "macos");
    }
    INSTALLED.with(|i| i.borrow().as_ref().is_none_or(|set| set.contains(family)))
}

/// `family` if it is installed, else the bundled stand-in.
fn installed_or_fallback(family: &'static str) -> &'static str {
    if is_installed(family) {
        family
    } else {
        FALLBACK
    }
}

/// What a text is: window contents (SA's own form font), a caption or a menu.
/// The contents keep SA's form font in the Windows styles, and the macOS
/// looks use the Mac system font for contents too; captions and menus take
/// the style's system font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Face {
    /// Window contents: labels, fields, lists.
    Content,
    /// Window and dialog captions.
    Caption,
    /// The menu bar and popup menus.
    Menu,
}

/// The family and pixel size of `face` in the active style.
fn spec(face: Face) -> (&'static str, f32) {
    use sn_ui_core::winstyle::WinStyle as W;
    let style = super::style::active().style;
    let (family, size) = match (face, style) {
        (_, W::Win95 | W::Win98) => return (FAMILY, FONT_SIZE),
        // The Linux looks: the desktop's own font (bundled, so always there):
        // Cantarell for GNOME (Ubuntu's own for Yaru, when installed), Noto
        // Sans for KDE, DejaVu Sans for GNOME 2 and Motif.
        (_, W::LinuxAdwaita | W::LinuxBreeze | W::LinuxClearlooks | W::LinuxMotif) => {
            let a = super::style::active();
            let family = match style {
                W::LinuxAdwaita if a.look.yaru && is_installed("Ubuntu") => "Ubuntu",
                W::LinuxAdwaita => "Cantarell",
                W::LinuxBreeze => "Noto Sans",
                _ => "DejaVu Sans",
            };
            let size = if face == Face::Content { 11.0 } else { 13.0 };
            return (installed_or_fallback(family), size);
        }
        // The macOS looks use the Mac's system font everywhere (a Mac app
        // has no form font of its own): small (11 px) in windows, 13 px in
        // title bars and menus.
        (_, W::MacAqua | W::MacYosemite | W::MacModern) => {
            let family = match style {
                W::MacAqua => "Lucida Grande",
                W::MacYosemite => "Helvetica Neue",
                _ => SYSTEM_UI,
            };
            let size = if face == Face::Content { 11.0 } else { 13.0 };
            return (installed_or_fallback(family), size);
        }
        // Tahoma 8 pt: SA's form font on XP and later.
        (Face::Content, _) | (Face::Menu, W::Xp) => ("Tahoma", 11.0),
        // Trebuchet MS bold 10 pt: Luna's caption font.
        (Face::Caption, W::Xp) => ("Trebuchet MS", 13.0),
        // Segoe UI Variable on Windows 11, Segoe UI where it is missing.
        (Face::Caption | Face::Menu, W::Win11) if is_installed(VARIABLE) => (VARIABLE, 12.0),
        // Segoe UI 9 pt from Vista on.
        (Face::Caption | Face::Menu, _) => ("Segoe UI", 12.0),
    };
    (installed_or_fallback(family), size)
}

/// The pixel size of the window-contents font in the active style.
pub(crate) fn content_size() -> f32 {
    spec(Face::Content).1
}

/// The user-interface font, regular or bold.
pub(crate) fn font(bold: bool) -> Font {
    font_of(Face::Content, bold)
}

/// The font of `face` in the active style, regular or bold.
pub(crate) fn font_of(face: Face, bold: bool) -> Font {
    let mut f = gpui::font(spec(face).0);
    f.weight = if bold {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    };
    f.fallbacks = Some(FontFallbacks::from_fonts(vec![
        "Noto Sans JP".to_owned(),
        "Noto Sans KR".to_owned(),
    ]));
    f
}

fn run(face: Face, len: usize, bold: bool, color: Hsla) -> TextRun {
    TextRun {
        len,
        font: font_of(face, bold),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

/// Shape one line of text (newlines are shown as spaces).
pub(crate) fn shape(window: &Window, text: &str, bold: bool, color: Hsla) -> ShapedLine {
    shape_in(Face::Content, window, text, bold, color)
}

/// Shape one line of text in `face` (newlines are shown as spaces).
pub(crate) fn shape_in(
    face: Face,
    window: &Window,
    text: &str,
    bold: bool,
    color: Hsla,
) -> ShapedLine {
    let text: SharedString = if text.contains('\n') {
        text.replace('\n', " ").into()
    } else {
        SharedString::from(text.to_owned())
    };
    let runs = [run(face, text.len(), bold, color)];
    window
        .text_system()
        .shape_line(text, px(spec(face).1), &runs, None)
}

/// Width of a line of text in `face`, in whole pixels.
pub(crate) fn width_in(face: Face, window: &Window, text: &str, bold: bool) -> i32 {
    f32::from(shape_in(face, window, text, bold, gpui::black()).width).ceil() as i32
}

/// Width of a line of text in whole pixels.
pub(crate) fn width(window: &Window, text: &str, bold: bool) -> i32 {
    f32::from(shape(window, text, bold, gpui::black()).width).ceil() as i32
}

/// A label with an `&` mnemonic, as in Windows resource strings: `&File`
/// underlines F and is chosen with Alt+F; `&&` is a literal ampersand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Mnemonic {
    /// The text to show, without the markers.
    pub(crate) text: String,
    /// Byte index in `text` of the underlined character.
    pub(crate) index: Option<usize>,
    /// The key that chooses the label (lower case).
    pub(crate) key: Option<char>,
}

/// Parse a label with `&` mnemonic markers.
pub(crate) fn mnemonic(label: &str) -> Mnemonic {
    let mut m = Mnemonic::default();
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            m.text.push(c);
            continue;
        }
        match chars.next() {
            Some('&') => m.text.push('&'),
            Some(next) => {
                if m.index.is_none() {
                    m.index = Some(m.text.len());
                    m.key = next.to_lowercase().next();
                }
                m.text.push(next);
            }
            None => {}
        }
    }
    m
}

/// How text is inked.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Ink {
    /// Plain text in one colour.
    Color(Hsla),
    /// Disabled text: embossed, a white copy one pixel lower right under a grey one.
    Embossed { light: Hsla, dark: Hsla },
}

/// Draw one line of text with its top-left corner at `at` (the top of a
/// [`LINE`]-high line).
pub(crate) fn draw(
    window: &mut Window,
    cx: &mut App,
    at: Point<Pixels>,
    text: &str,
    bold: bool,
    color: Hsla,
) {
    draw_in(Face::Content, window, cx, at, text, bold, color);
}

/// Draw one line of text in `face` (see [`draw`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_in(
    face: Face,
    window: &mut Window,
    cx: &mut App,
    at: Point<Pixels>,
    text: &str,
    bold: bool,
    color: Hsla,
) {
    shape_in(face, window, text, bold, color)
        .paint(at, px(LINE as f32), window, cx)
        .ok();
}

/// Draw a label with an optional mnemonic underline. Returns its width.
pub(crate) fn draw_label(
    window: &mut Window,
    cx: &mut App,
    at: Point<Pixels>,
    label: &Mnemonic,
    bold: bool,
    ink: Ink,
    underline: bool,
) -> i32 {
    draw_label_in(Face::Content, window, cx, at, label, bold, ink, underline)
}

/// Draw a label in `face` (see [`draw_label`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_label_in(
    face: Face,
    window: &mut Window,
    cx: &mut App,
    at: Point<Pixels>,
    label: &Mnemonic,
    bold: bool,
    ink: Ink,
    underline: bool,
) -> i32 {
    let passes: &[(Hsla, f32)] = match ink {
        Ink::Color(c) => &[(c, 0.0)],
        Ink::Embossed { light, dark } => &[(light, 1.0), (dark, 0.0)],
    };
    let mut w = 0;
    for &(color, d) in passes {
        let line = shape_in(face, window, &label.text, bold, color);
        w = f32::from(line.width).ceil() as i32;
        let origin = point(at.x + px(d), at.y + px(d));
        line.paint(origin, px(LINE as f32), window, cx).ok();
        if let (true, Some(i)) = (underline, label.index) {
            let x0 = f32::from(line.x_for_index(i));
            let end = label
                .text
                .get(i..)
                .and_then(|s| s.chars().next())
                .map_or(i, |c| i + c.len_utf8());
            let x1 = f32::from(line.x_for_index(end)) - 1.0;
            if x1 > x0 {
                let y = origin.y + px((ASCENT + 1) as f32);
                window.paint_quad(gpui::fill(
                    gpui::Bounds::new(
                        point(origin.x + px(x0), y),
                        gpui::size(px(x1 - x0), px(1.0)),
                    ),
                    color,
                ));
            }
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Name `id` of a TrueType font, from its Windows (UTF-16) record.
    fn font_name(font: &[u8], id: u16) -> Option<String> {
        let u16_at = |at: usize| Some(u16::from_be_bytes([*font.get(at)?, *font.get(at + 1)?]));
        let u32_at = |at: usize| {
            Some(u32::from_be_bytes([
                *font.get(at)?,
                *font.get(at + 1)?,
                *font.get(at + 2)?,
                *font.get(at + 3)?,
            ]))
        };
        let tables = usize::from(u16_at(4)?);
        let name = (0..tables).find_map(|i| {
            let rec = 12 + 16 * i;
            (font.get(rec..rec + 4)? == b"name").then(|| u32_at(rec + 8))?
        })? as usize;
        let count = usize::from(u16_at(name + 2)?);
        let strings = name + usize::from(u16_at(name + 4)?);
        (0..count).find_map(|i| {
            let r = name + 6 + 12 * i;
            if u16_at(r)? != 3 || u16_at(r + 6)? != id {
                return None;
            }
            let (len, off) = (usize::from(u16_at(r + 8)?), usize::from(u16_at(r + 10)?));
            let units: Vec<u16> = font
                .get(strings + off..strings + off + len)?
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&c| u16::from_be_bytes(c))
                .collect();
            String::from_utf16(&units).ok()
        })
    }

    #[test]
    fn the_bundled_linux_fonts_have_their_own_postscript_names() {
        // macOS keys loaded fonts by PostScript name: two faces with one name
        // would replace each other, and bold text draw in the regular face.
        let fonts = [
            (CANTARELL, "Cantarell-Regular", "Regular"),
            (CANTARELL_BOLD, "Cantarell-Bold", "Bold"),
            (NOTO_SANS, "NotoSans-Regular", "Regular"),
            (NOTO_SANS_BOLD, "NotoSans-Bold", "Bold"),
            (DEJAVU, "DejaVuSans", "Book"),
            (DEJAVU_BOLD, "DejaVuSans-Bold", "Bold"),
        ];
        for (font, ps, sub) in fonts {
            assert_eq!(font_name(font, 6).as_deref(), Some(ps));
            assert_eq!(font_name(font, 2).as_deref(), Some(sub), "{ps}");
        }
    }

    #[test]
    fn mnemonic_parsing() {
        let m = mnemonic("&File");
        assert_eq!(m.text, "File");
        assert_eq!(m.index, Some(0));
        assert_eq!(m.key, Some('f'));
        let m = mnemonic("Save &As...");
        assert_eq!(m.text, "Save As...");
        assert_eq!(m.index, Some(5));
        assert_eq!(m.key, Some('a'));
        let m = mnemonic("R&&D &Tools");
        assert_eq!(m.text, "R&D Tools");
        assert_eq!(m.key, Some('t'));
        let m = mnemonic("Plain");
        assert_eq!(m.index, None);
        assert_eq!(m.key, None);
        assert_eq!(mnemonic("trailing&").text, "trailing");
        // Non-ASCII mnemonic letters are found by byte index.
        let m = mnemonic("\u{e9}t&\u{e9}");
        assert_eq!(m.index, Some(3));
        assert_eq!(m.key, Some('\u{e9}'));
    }
}
