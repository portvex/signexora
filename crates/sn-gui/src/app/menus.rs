//! SA's main menu (`File`, `Windows`, `Options`, `Help`, `docs/SA_UI_SPEC.md`
//! section 2.1) with the SigNexora items of section 17, the MDI child system menu,
//! and the 16x16 item pictures.

use sn_dsp::spectrum::Combine;
use sn_ui_core::colormap::Colormap;
use sn_ui_core::i18n::Lang;
use sn_ui_core::prefs::{Prefs, StartSize};
use sn_ui_core::sono_settings::Settings;
use sn_ui_core::t;
use sn_ui_core::theme::ThemeChoice;
use sn_ui_core::winstyle::{
    ACCENTS, AquaColor, BREEZE_ACCENTS, LINUX_ACCENTS, Look, MAC_ACCENTS, WinStyle, XpColor,
    YARU_ACCENTS,
};

use crate::win95::glyph::{BLACK, Glyph, Ink};
use crate::win95::menu::{Check, Item, Menu};

/// Menu commands.
pub(crate) mod cmd {
    pub(crate) const OPEN_FILE: u32 = 100;
    pub(crate) const DEMO: u32 = 101;
    pub(crate) const OPEN_SESSION: u32 = 102;
    pub(crate) const SAVE_SESSION: u32 = 103;
    pub(crate) const SAVE_SESSION_AS: u32 = 104;
    pub(crate) const REPORT_HTML: u32 = 105;
    pub(crate) const REPORT_MARKDOWN: u32 = 106;
    pub(crate) const RUN_SCRIPT: u32 = 107;
    pub(crate) const GENERATOR: u32 = 108;
    pub(crate) const GALLERY: u32 = 109;
    pub(crate) const EXIT: u32 = 110;
    pub(crate) const CLOSE_ALL: u32 = 200;
    /// `START_SIZE + i`: `StartSize::ALL[i]`.
    pub(crate) const START_SIZE: u32 = 300;
    pub(crate) const INFO_ON: u32 = 320;
    pub(crate) const INFO_OFF: u32 = 321;
    /// `LANGUAGE + i`: 0 follows the system, then `Lang::ALL`.
    pub(crate) const LANGUAGE: u32 = 330;
    /// `THEME + i`: `ThemeChoice::ALL[i]`.
    pub(crate) const THEME: u32 = 350;
    /// `COLOURS + i`: `Colormap::ALL[i]`.
    pub(crate) const COLOURS: u32 = 380;
    pub(crate) const ZOOM_PEAK: u32 = 390;
    pub(crate) const ZOOM_MEAN: u32 = 391;
    pub(crate) const USE_HELP: u32 = 400;
    pub(crate) const ABOUT: u32 = 401;
    pub(crate) const SYS_RESTORE: u32 = 500;
    pub(crate) const SYS_MINIMIZE: u32 = 501;
    pub(crate) const SYS_MAXIMIZE: u32 = 502;
    pub(crate) const SYS_CLOSE: u32 = 503;
    pub(crate) const SYS_NEXT: u32 = 504;
    pub(crate) const SYS_MOVE: u32 = 505;
    pub(crate) const SYS_SIZE: u32 = 506;
    /// The loader's popup menus.
    pub(crate) const LD_UPDATE: u32 = 600;
    pub(crate) const LD_AUTO: u32 = 601;
    pub(crate) const LD_COPY: u32 = 602;
    pub(crate) const LD_PASTE: u32 = 603;
    pub(crate) const LD_CANCEL: u32 = 604;
    /// The sonogram window's popup menu: `SW_SAVE + i` saves in format `i`
    /// of the `Save as` submenu.
    pub(crate) const SW_SAVE: u32 = 700;
    pub(crate) const SW_WAVEFORM: u32 = 710;
    pub(crate) const SW_MEASURE: u32 = 711;
    pub(crate) const SW_ADP: u32 = 712;
    pub(crate) const SW_BRC: u32 = 713;
    pub(crate) const SW_CANCEL: u32 = 714;
    /// `SW_PRESET + i`: preset `i` of the R/S value field.
    pub(crate) const SW_PRESET: u32 = 720;
    pub(crate) const SW_LAST: u32 = 749;
    /// The WF window's popup menu: `WF_SAVE + i` saves in format `i` of the
    /// `Save as` submenu.
    pub(crate) const WF_SAVE: u32 = 1200;
    pub(crate) const WF_COUNT: u32 = 1204;
    pub(crate) const WF_CANCEL: u32 = 1205;
    pub(crate) const WF_PLAY: u32 = 1206;
    pub(crate) const WF_SHOW_Q: u32 = 1207;
    pub(crate) const WF_OPEN_PHASE: u32 = 1208;
    pub(crate) const WF_OPEN_OFDM: u32 = 1209;
    pub(crate) const WF_MEASURE_BR: u32 = 1210;
    pub(crate) const WF_MEASURE_SC: u32 = 1211;
    pub(crate) const WF_ADP: u32 = 1212;
    pub(crate) const WF_MEASURE_DT: u32 = 1213;
    /// `WF_LPF_PRESET + i`: preset `i` of the LPF field.
    pub(crate) const WF_LPF_PRESET: u32 = 1220;
    /// `WF_OP + i`: operation `i` of `SigNexora tools` > `Operations`.
    pub(crate) const WF_OP: u32 = 1240;
    /// The VMW popup.
    pub(crate) const VMW_START_V1: u32 = 1290;
    pub(crate) const VMW_FROM_DT: u32 = 1291;
    pub(crate) const VMW_SA_COLOURS: u32 = 1292;
    /// `VMW_COLOURS + i`: `Colormap::ALL[i]`.
    pub(crate) const VMW_COLOURS: u32 = 1293;
    pub(crate) const WF_LAST: u32 = 1299;
    /// The PhasePlane window's popup menus.
    pub(crate) const PP_COPY_TEXT: u32 = 760;
    pub(crate) const PP_COPY_CODES: u32 = 761;
    pub(crate) const PP_EYE: u32 = 762;
    pub(crate) const PP_LAST: u32 = 769;
    /// Options > Style: `STYLE + i` is `WinStyle::ALL[i]`, `XP_COLOUR + i`
    /// is `XpColor::ALL[i]`, `AQUA + i` is `AquaColor::ALL[i]`, `ACCENT + i`
    /// is the chosen style's accent `i` (`WinStyle::accents`).
    pub(crate) const STYLE: u32 = 900;
    pub(crate) const XP_COLOUR: u32 = 920;
    pub(crate) const AQUA: u32 = 925;
    pub(crate) const AERO_ON: u32 = 930;
    pub(crate) const AERO_OFF: u32 = 931;
    pub(crate) const APP_LIGHT: u32 = 932;
    pub(crate) const APP_DARK: u32 = 933;
    /// The GNOME style in GNOME's own Adwaita colours.
    pub(crate) const GNOME: u32 = 934;
    /// The GNOME style in Ubuntu's Yaru colours.
    pub(crate) const YARU: u32 = 935;
    pub(crate) const ACCENT: u32 = 940;
    pub(crate) const STYLE_LAST: u32 = 969;
}

// The open-ended id ranges (`BASE + i`) must not run into the next command:
// a new theme, language or colour map would otherwise run the wrong one.
const _: () = {
    use sn_ui_core::sono_tools::VALUE_PRESETS;
    use sn_ui_core::wf_tools::WF_LPF_PRESETS;
    const fn len(n: usize) -> u32 {
        n as u32
    }
    assert!(cmd::START_SIZE + len(StartSize::ALL.len()) <= cmd::INFO_ON);
    // 0 follows the system, then every language.
    assert!(cmd::LANGUAGE + 1 + len(Lang::ALL.len()) <= cmd::THEME);
    assert!(cmd::THEME + len(ThemeChoice::ALL.len()) <= cmd::COLOURS);
    assert!(cmd::COLOURS + len(Colormap::ALL.len()) <= cmd::ZOOM_PEAK);
    assert!(cmd::STYLE + len(WinStyle::ALL.len()) <= cmd::XP_COLOUR);
    assert!(cmd::XP_COLOUR + len(XpColor::ALL.len()) <= cmd::AQUA);
    assert!(cmd::AQUA + len(AquaColor::ALL.len()) <= cmd::AERO_ON);
    assert!(cmd::ACCENT + len(ACCENTS.len()) <= cmd::STYLE_LAST);
    assert!(cmd::ACCENT + len(MAC_ACCENTS.len()) <= cmd::STYLE_LAST);
    assert!(cmd::ACCENT + len(LINUX_ACCENTS.len()) <= cmd::STYLE_LAST);
    assert!(cmd::ACCENT + len(YARU_ACCENTS.len()) <= cmd::STYLE_LAST);
    assert!(cmd::ACCENT + len(BREEZE_ACCENTS.len()) <= cmd::STYLE_LAST);
    assert!(cmd::SW_SAVE + len(crate::app::sono::popup::SAVE_FORMATS.len()) <= cmd::SW_WAVEFORM);
    // The rate itself, then the presets.
    assert!(cmd::SW_PRESET + 1 + len(VALUE_PRESETS.len()) <= cmd::SW_LAST);
    assert!(cmd::WF_SAVE + len(crate::app::wf::popup::SAVE_FORMATS.len()) <= cmd::WF_COUNT);
    assert!(cmd::WF_LPF_PRESET + len(WF_LPF_PRESETS.len()) <= cmd::WF_OP);
    assert!(cmd::VMW_COLOURS + len(Colormap::ALL.len()) <= cmd::WF_LAST);
};

/// What the menus show.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct MenuState {
    pub(crate) prefs: Prefs,
    /// The session holds signals (save, report).
    pub(crate) has_signals: bool,
    /// A session save is running.
    pub(crate) saving: bool,
    /// The session's file (Save writes there without asking).
    pub(crate) path: Option<String>,
    /// A sonogram window is active (Colours and Zoomed out change it).
    pub(crate) sono_active: bool,
    /// The sonogram settings Options > Colours and Zoomed out change: the
    /// active sonogram window's, else the defaults for new recordings.
    pub(crate) sono: Settings,
}

fn tick(on: bool) -> Check {
    if on { Check::Tick } else { Check::None }
}

fn dot(on: bool) -> Check {
    if on { Check::Dot } else { Check::None }
}

/// Give every item of `items` (and of its submenus) a mnemonic: the first
/// letter of a word not taken yet in its menu, else any free letter. Labels
/// in scripts without ASCII letters get none.
pub(crate) fn add_mnemonics(items: &mut [Item]) {
    let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
    for (item, label) in items.iter_mut().zip(with_mnemonics(&labels)) {
        if !item.is_separator() {
            item.label = label;
        }
        if let crate::win95::menu::Kind::Submenu(sub) = &mut item.kind {
            add_mnemonics(sub);
        }
    }
}

/// The mnemonic a label marks itself (`&` before a letter or digit).
fn explicit_mnemonic(label: &str) -> Option<char> {
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.next() {
                Some(n) if n.is_alphanumeric() => return n.to_lowercase().next(),
                _ => {}
            }
        }
    }
    None
}

/// `labels` with an `&` before a letter unique among them. A label that marks
/// its own mnemonic (a translation can) keeps it; other `&`s are escaped.
pub(crate) fn with_mnemonics(labels: &[String]) -> Vec<String> {
    let mut used: Vec<char> = labels.iter().filter_map(|l| explicit_mnemonic(l)).collect();
    labels
        .iter()
        .map(|l| {
            if explicit_mnemonic(l).is_some() {
                return l.clone();
            }
            let escaped = l.replace('&', "&&");
            let chars: Vec<(usize, char)> = escaped.char_indices().collect();
            let free =
                |c: char| c.is_ascii_alphanumeric() && !used.contains(&c.to_ascii_lowercase());
            let word_start = chars.iter().enumerate().find(|(k, (_, c))| {
                let prev = k.checked_sub(1).and_then(|p| chars.get(p)).map(|(_, c)| *c);
                free(*c) && prev.is_none_or(|p| !p.is_alphanumeric())
            });
            let pick = word_start
                .map(|(_, ic)| *ic)
                .or_else(|| chars.iter().copied().find(|(_, c)| free(*c)));
            match pick {
                Some((i, c)) => {
                    used.push(c.to_ascii_lowercase());
                    let (a, b) = escaped.split_at(i);
                    format!("{a}&{b}")
                }
                None => escaped,
            }
        })
        .collect()
}

/// The main menu: SA's `File`, `Windows`, `Options`, `Help` (section 2.1)
/// with SigNexora's items.
pub(crate) fn main_menu(st: &MenuState) -> Vec<Menu> {
    let titles = with_mnemonics(&[
        t!("app-menu-file"),
        t!("menu-windows"),
        t!("menu-options"),
        t!("menu-help"),
    ]);
    let bodies = [
        file_menu(st),
        vec![
            Item::command(&t!("menu-close-all"), cmd::CLOSE_ALL)
                .glyph(close_all_glyph())
                .hint(&t!("menu-close-all.hover")),
        ],
        options_menu(st),
        vec![
            Item::command(&t!("menu-use-help"), cmd::USE_HELP)
                .glyph(globe_glyph())
                .shortcut("Ctrl+/")
                .hint(&t!("menu-use-help.hover")),
            Item::separator(),
            Item::command(&t!("menu-about"), cmd::ABOUT),
        ],
    ];
    titles
        .into_iter()
        .zip(bodies)
        .map(|(label, mut items)| {
            add_mnemonics(&mut items);
            Menu { label, items }
        })
        .collect()
}

fn file_menu(st: &MenuState) -> Vec<Item> {
    let can_save = st.has_signals && !st.saving;
    let (save, save_hint) = match &st.path {
        Some(p) => (
            t!("app-menu-save-session"),
            t!("app-menu-save-session-to", path = p.as_str()),
        ),
        None => (
            t!("app-menu-save-session-ask"),
            t!("app-menu-save-session-choose"),
        ),
    };
    let why_not = if st.has_signals {
        t!("app-session-save-running")
    } else {
        t!("app-nothing-to-save")
    };
    let (save_hint, save_as_hint) = if can_save {
        (save_hint, String::new())
    } else {
        (why_not.clone(), why_not)
    };
    let mut items = vec![
        Item::command(&t!("menu-open-file"), cmd::OPEN_FILE)
            .shortcut("Ctrl+O")
            .glyph(folder_glyph()),
        Item::command(&t!("menu-demo"), cmd::DEMO)
            .shortcut("F1")
            .hint(&t!("menu-demo.hover")),
        Item::separator(),
        Item::command(&t!("app-menu-open-session"), cmd::OPEN_SESSION)
            .hint(&t!("app-menu-open-session.hover")),
        Item::command(&save, cmd::SAVE_SESSION)
            .shortcut("Ctrl+S")
            .glyph(floppy_glyph())
            .enabled(can_save)
            .hint(&save_hint),
        Item::command(&t!("app-menu-save-session-as"), cmd::SAVE_SESSION_AS)
            .shortcut("Ctrl+Shift+S")
            .glyph(floppy_glyph())
            .enabled(can_save)
            .hint(&save_as_hint),
        Item::submenu(
            &t!("app-menu-export-report"),
            vec![
                Item::command(&t!("app-menu-report-html"), cmd::REPORT_HTML),
                Item::command(&t!("app-menu-report-markdown"), cmd::REPORT_MARKDOWN),
            ],
        )
        .enabled(st.has_signals),
        Item::command(&t!("app-menu-run-script"), cmd::RUN_SCRIPT)
            .hint(&t!("app-menu-run-script.hover")),
        Item::command(&t!("gen-menu"), cmd::GENERATOR)
            .shortcut("Ctrl+Shift+N")
            .hint(&t!("gen-menu.hover")),
    ];
    if cfg!(debug_assertions) {
        items.push(Item::separator());
        items.push(Item::command("Widget gallery", cmd::GALLERY));
    }
    items.push(Item::separator());
    items.push(Item::command(&t!("menu-exit"), cmd::EXIT).glyph(exit_glyph()));
    items
}

fn options_menu(st: &MenuState) -> Vec<Item> {
    let p = &st.prefs;
    let mut sizes: Vec<Item> = (0_u32..)
        .zip(StartSize::ALL)
        .map(|(i, s)| Item::command(&s.label(), cmd::START_SIZE + i).check(tick(p.start_size == s)))
        .collect();
    sizes.push(Item::separator());
    sizes.push(Item::command(&t!("menu-start-size-note"), 0).enabled(false));
    let system = t!(
        "settings-language-system",
        language = Lang::system().native_name()
    );
    let mut langs = vec![
        Item::command(&system, cmd::LANGUAGE).check(dot(p.language.is_none())),
        Item::separator(),
    ];
    for (i, lang) in (0_u32..).zip(Lang::ALL) {
        langs.push(
            Item::command(lang.native_name(), cmd::LANGUAGE + 1 + i)
                .check(dot(p.language == Some(lang))),
        );
    }
    langs.push(Item::separator());
    langs.push(Item::command(&t!("settings-language-note"), 0).enabled(false));
    let colours: Vec<Item> = (0_u32..)
        .zip(Colormap::ALL)
        .map(|(i, c)| {
            Item::command(&c.label(), cmd::COLOURS + i).check(tick(st.sono.colormap == c))
        })
        .collect();
    let peak = st.sono.combine == Combine::PeakHold;
    let target = t!(
        "app-sono-target",
        active = if st.sono_active { "yes" } else { "no" }
    );
    vec![
        Item::submenu(&t!("menu-start-size"), sizes),
        Item::submenu(
            &t!("menu-info-default"),
            vec![
                Item::command(&t!("menu-on"), cmd::INFO_ON).check(tick(p.info_default)),
                Item::command(&t!("menu-off"), cmd::INFO_OFF).check(tick(!p.info_default)),
            ],
        ),
        Item::separator(),
        Item::submenu(&t!("settings-language"), langs),
        Item::submenu(&t!("winstyle-menu"), style_items(p.look, p.theme))
            .hint(&t!("winstyle-menu.hover")),
        theme_menu(p.theme, p.look),
        Item::separator(),
        Item::submenu(&t!("app-sono-colormap"), colours)
            .hint(&format!("{} {target}", t!("app-sono-colormap.hover"))),
        Item::submenu(
            &t!("app-sono-combine"),
            vec![
                Item::command(&t!("app-sono-peak"), cmd::ZOOM_PEAK)
                    .check(tick(peak))
                    .hint(&t!("app-sono-peak.hover")),
                Item::command(&t!("app-sono-mean"), cmd::ZOOM_MEAN)
                    .check(tick(!peak))
                    .hint(&t!("app-sono-mean.hover")),
            ],
        )
        .hint(&format!("{} {target}", t!("app-sono-combine.hover"))),
    ]
}

/// Options > Style: the Windows, macOS and Linux looks, each family set
/// apart by a separator; then the variants of the chosen one (XP's colour
/// scheme, GNOME's Adwaita or Ubuntu's Yaru colours, Aqua's Blue or
/// Graphite, Aero or Basic, the light or dark mode as radio items, the
/// accent colour as a submenu of swatches) and a note on the colour themes.
/// Under high contrast, which draws the classic look in every style, the
/// variants are greyed and the note says why.
fn style_items(look: Look, theme: ThemeChoice) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::with_capacity(WinStyle::ALL.len() + 12);
    for (i, w) in (0_u32..).zip(WinStyle::ALL) {
        if i > 0 && w.starts_family() {
            // The Windows, macOS and Linux families, apart.
            items.push(Item::separator());
        }
        let item = Item::command(&w.label(), cmd::STYLE + i).check(dot(w == look.style));
        items.push(match w.hint() {
            Some(h) => item.hint(&h),
            None => item,
        });
    }
    let style = look.style;
    let live = !look.overridden_by(theme);
    if style == WinStyle::Xp {
        items.push(Item::separator());
        items.extend((0_u32..).zip(XpColor::ALL).map(|(i, c)| {
            Item::command(&c.label(), cmd::XP_COLOUR + i)
                .check(dot(c == look.xp))
                .enabled(live)
        }));
    }
    if style == WinStyle::LinuxAdwaita {
        items.push(Item::separator());
        items.push(
            Item::command(&t!("winstyle-gnome"), cmd::GNOME)
                .check(dot(!look.yaru))
                .enabled(live)
                .hint(&t!("winstyle-gnome.hover")),
        );
        items.push(
            Item::command(&t!("winstyle-yaru"), cmd::YARU)
                .check(dot(look.yaru))
                .enabled(live)
                .hint(&t!("winstyle-yaru.hover")),
        );
    }
    if style == WinStyle::MacAqua {
        items.push(Item::separator());
        let aqua_hint = t!("winstyle-aqua.hover");
        items.extend((0_u32..).zip(AquaColor::ALL).map(|(i, c)| {
            Item::command(&c.label(), cmd::AQUA + i)
                .check(dot(c == look.aqua))
                .enabled(live)
                .hint(&aqua_hint)
        }));
    }
    if style.has_aero() {
        items.push(Item::separator());
        let glass_hint = t!("winstyle-aero.hover");
        items.push(
            Item::command(&t!("winstyle-aero"), cmd::AERO_ON)
                .check(dot(look.aero))
                .enabled(live)
                .hint(&glass_hint),
        );
        items.push(
            Item::command(&t!("winstyle-basic"), cmd::AERO_OFF)
                .check(dot(!look.aero))
                .enabled(live)
                .hint(&glass_hint),
        );
    }
    if style.has_app_mode() {
        items.push(Item::separator());
        items.push(
            Item::command(&t!("winstyle-light"), cmd::APP_LIGHT)
                .check(dot(!look.dark))
                .enabled(live),
        );
        items.push(
            Item::command(&t!("winstyle-dark"), cmd::APP_DARK)
                .check(dot(look.dark))
                .enabled(live),
        );
    }
    if style.has_accent() {
        if !style.has_app_mode() {
            items.push(Item::separator());
        }
        items.push(Item::submenu(&t!("winstyle-accent"), accent_items(look)).enabled(live));
    }
    items.push(Item::separator());
    let note = if live {
        "winstyle-scheme-note"
    } else {
        "winstyle-hc-active"
    };
    items.push(note_item(note));
    items
}

/// The swatches of the Accent color submenu of `look` ([`Look::accents`]:
/// Yaru and Breeze have their own). Each swatch shows the colour drawn
/// (macOS's brighter variant in the dark mode); the check compares the
/// stored colour.
fn accent_items(look: Look) -> Vec<Item> {
    (0_u32..)
        .zip(look.accents().iter().copied())
        .map(|(i, a)| {
            Item::command(&a.label(), cmd::ACCENT + i)
                .check(dot(a.rgb() == look.accent_rgb()))
                .glyph(swatch_glyph(look.drawn_swatch(a)))
        })
        .collect()
}

/// A greyed one-line note in a menu: the message `key` as its label, and its
/// longer `.hover` attribute as the status-bar hint (menu items never wrap, so
/// the full sentence would widen the popup).
fn note_item(key: &str) -> Item {
    Item::command(&t!(key), 0)
        .enabled(false)
        .hint(&note_hint(key))
}

/// The status-bar text of note `key`: its `.hover` attribute, or the note itself
/// when a locale has no attribute for it.
fn note_hint(key: &str) -> String {
    let hover = format!("{key}.hover");
    let text = t!(&hover);
    if text == hover { t!(key) } else { text }
}

/// The look after the Windows style command `id`, if it is one.
pub(crate) fn look_after(look: Look, id: u32) -> Option<Look> {
    let index = |base: u32| usize::try_from(id.checked_sub(base)?).ok();
    let mut l = look;
    match id {
        i if (cmd::STYLE..cmd::XP_COLOUR).contains(&i) => {
            l.style = *WinStyle::ALL.get(index(cmd::STYLE)?)?;
        }
        i if (cmd::XP_COLOUR..cmd::AQUA).contains(&i) => {
            l.xp = *XpColor::ALL.get(index(cmd::XP_COLOUR)?)?;
        }
        i if (cmd::AQUA..cmd::AERO_ON).contains(&i) => {
            l.aqua = *AquaColor::ALL.get(index(cmd::AQUA)?)?;
        }
        cmd::AERO_ON | cmd::AERO_OFF => l.aero = id == cmd::AERO_ON,
        cmd::APP_LIGHT | cmd::APP_DARK => l.dark = id == cmd::APP_DARK,
        cmd::GNOME | cmd::YARU => l.yaru = id == cmd::YARU,
        i if (cmd::ACCENT..=cmd::STYLE_LAST).contains(&i) => {
            let rgb = l.accents().get(index(cmd::ACCENT)?)?.rgb();
            l.set_accent(rgb);
        }
        _ => return None,
    }
    Some(l)
}

/// Options > Theme. Under the themed Windows styles (XP to 10) the theme
/// colours the plots only, which the submenu says.
fn theme_menu(current: ThemeChoice, look: Look) -> Item {
    let mut items = theme_items(current);
    if look.style.is_classic() {
        Item::submenu(&t!("settings-theme"), items)
    } else {
        items.push(Item::separator());
        items.push(note_item("winstyle-scheme-note"));
        Item::submenu(&t!("settings-theme"), items).hint(&note_hint("winstyle-scheme-note"))
    }
}

/// The appearance schemes: Windows Standard, the two Windows high-contrast
/// schemes, then SigNexora's other themes, grouped as in `sn-ui-core`.
fn theme_items(current: ThemeChoice) -> Vec<Item> {
    let mut items = Vec::new();
    let mut group = None;
    for (i, c) in (0_u32..).zip(ThemeChoice::ALL) {
        if group.is_some_and(|g| g != c.group()) {
            items.push(Item::separator());
        }
        group = Some(c.group());
        items.push(Item::command(&scheme_name(c), cmd::THEME + i).check(dot(c == current)));
    }
    items
}

/// The name of a theme as a Windows appearance scheme.
pub(crate) fn scheme_name(c: ThemeChoice) -> String {
    match c {
        ThemeChoice::Classic => t!("gui-scheme-standard"),
        ThemeChoice::HighContrastDark => t!("gui-scheme-high-contrast-black"),
        ThemeChoice::HighContrastLight => t!("gui-scheme-high-contrast-white"),
        other => t!(other.key()),
    }
}

/// The system menu of an MDI child (its window icon).
pub(crate) fn system_menu(minimized: bool, maximized: bool, resizable: bool) -> Vec<Item> {
    let normal = !minimized && !maximized;
    let mut items = vec![
        Item::command(&t!("gui-sys-restore"), cmd::SYS_RESTORE).enabled(!normal),
        Item::command(&t!("gui-sys-move"), cmd::SYS_MOVE).enabled(false),
        Item::command(&t!("gui-sys-size"), cmd::SYS_SIZE).enabled(false),
        Item::command(&t!("gui-sys-minimize"), cmd::SYS_MINIMIZE).enabled(resizable && !minimized),
        Item::command(&t!("gui-sys-maximize"), cmd::SYS_MAXIMIZE).enabled(resizable && !maximized),
        Item::separator(),
        Item::command(&t!("gui-sys-close"), cmd::SYS_CLOSE).shortcut("Ctrl+F4"),
        Item::separator(),
        Item::command(&t!("gui-sys-next"), cmd::SYS_NEXT).shortcut("Ctrl+F6"),
    ];
    add_mnemonics(&mut items);
    items
}

/// A swatch of colour `rgb` with a text-coloured outline (the accent
/// colours).
pub(crate) fn swatch_glyph(rgb: u32) -> Glyph {
    use crate::win95::colors::Role;
    Glyph::new(16, 16)
        .rect(2, 2, 12, 12, Ink::Role(Role::Text))
        .rect(3, 3, 10, 10, Ink::Rgb(rgb))
}

/// An open yellow folder.
pub(crate) fn folder_glyph() -> Glyph {
    let y = Ink::Rgb(0xFF_FF_00);
    let dy = Ink::Rgb(0x80_80_00);
    Glyph::new(16, 16)
        .bitmap(0, 3, &[" ####", "#    #####"], BLACK)
        .rect(0, 4, 1, 10, BLACK)
        .rect(1, 5, 12, 1, y)
        .rect(1, 6, 1, 7, y)
        .rect(12, 6, 1, 3, dy)
        .rect(12, 5, 1, 4, BLACK)
        .poly(&[(3.0, 8.0), (16.0, 8.0), (13.0, 14.0), (0.0, 14.0)], BLACK)
        .poly(&[(4.0, 9.0), (14.5, 9.0), (12.3, 13.0), (1.7, 13.0)], dy)
}

/// A blue floppy disk.
pub(crate) fn floppy_glyph() -> Glyph {
    let navy = Ink::Rgb(0x00_00_80);
    let white = Ink::Rgb(0xFF_FF_FF);
    let grey = Ink::Rgb(0xC0_C0_C0);
    Glyph::new(16, 16)
        .rect(1, 1, 14, 14, BLACK)
        .rect(2, 2, 12, 12, navy)
        .rect(4, 2, 8, 5, white)
        .rect(4, 9, 8, 5, grey)
        .rect(9, 10, 2, 3, navy)
}

/// The red cross of `CloseAllWindows`.
pub(crate) fn close_all_glyph() -> Glyph {
    let red = Ink::Rgb(0xFF_00_00);
    let mut g = Glyph::new(16, 16);
    for d in 0..2 {
        g = g
            .line(3 + d, 3, 12 + d, 12, red)
            .line(12 + d, 3, 3 + d, 12, red);
    }
    g
}

/// The grey globe of `Use help...`.
pub(crate) fn globe_glyph() -> Glyph {
    let grey = Ink::Rgb(0x80_80_80);
    let light = Ink::Rgb(0xC0_C0_C0);
    Glyph::new(16, 16)
        .circle(8.0, 8.0, 7.0, grey)
        .circle(8.0, 8.0, 6.0, light)
        .rect(2, 7, 12, 1, grey)
        .rect(7, 2, 1, 12, grey)
        .bitmap(
            4,
            3,
            &["  ###", " #   #", "", "", "", "", " #   #", "  ###"],
            grey,
        )
}

/// The exit door (yellow frame, red door).
pub(crate) fn exit_glyph() -> Glyph {
    Glyph::new(16, 16)
        .rect(3, 1, 10, 14, BLACK)
        .rect(4, 2, 8, 12, Ink::Rgb(0xFF_FF_00))
        .poly(
            &[(5.0, 3.0), (11.0, 2.0), (11.0, 14.0), (5.0, 13.0)],
            Ink::Rgb(0xFF_00_00),
        )
        .rect(9, 8, 1, 1, BLACK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win95::menu::{Key, Outcome, Source, Tracker};

    #[test]
    fn main_menu_order_matches_sa() {
        let m = main_menu(&MenuState::default());
        let titles: Vec<&str> = m.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(titles, ["&File", "&Windows", "&Options", "&Help"]);
        let file: Vec<&str> = m[0].items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(file.first(), Some(&"&Open file..."));
        assert_eq!(file.get(1), Some(&"&Demo..."));
        assert_eq!(file.last(), Some(&"E&xit"));
        assert_eq!(m[1].items[0].label, "&CloseAllWindows");
    }

    #[test]
    fn keyboard_reaches_info_by_default() {
        let m = main_menu(&MenuState::default());
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        t.key(src, Key::AltChar('o'));
        t.key(src, Key::Down);
        assert_eq!(t.key(src, Key::Right), Outcome::Handled);
        assert_eq!(t.key(src, Key::Enter), Outcome::Invoke(cmd::INFO_ON));
    }

    #[test]
    fn mnemonics_are_unique_per_menu() {
        let m = main_menu(&MenuState::default());
        let menus = m
            .iter()
            .map(|m| m.items.clone())
            .chain(std::iter::once(system_menu(false, false, true)));
        for items in menus {
            let mut keys: Vec<char> = items
                .iter()
                .filter_map(|i| crate::win95::text::mnemonic(&i.label).key)
                .collect();
            let n = keys.len();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), n, "duplicate mnemonic in {items:?}");
        }
    }

    #[test]
    fn mnemonics_prefer_word_starts_and_escape_ampersands() {
        let l = |v: &[&str]| with_mnemonics(&v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
        assert_eq!(
            l(&["Open file", "Open session"]),
            ["&Open file", "Open &session"]
        );
        assert_eq!(l(&["wav & mp3"]), ["&wav && mp3"]);
        assert_eq!(l(&["\u{65e5}\u{672c}"]), ["\u{65e5}\u{672c}"]);
        assert_eq!(l(&["aa", "ab"]), ["&aa", "a&b"]);
        assert_eq!(l(&["Mi&nimize", "Next"]), ["Mi&nimize", "N&ext"]);
    }

    /// Every command id of `items` and their submenus.
    fn ids(items: &[Item]) -> Vec<u32> {
        let mut out = Vec::new();
        for i in items {
            match &i.kind {
                crate::win95::menu::Kind::Submenu(sub) => out.extend(ids(sub)),
                crate::win95::menu::Kind::Command(id) => out.push(*id),
                crate::win95::menu::Kind::Separator => {}
            }
        }
        out
    }

    #[test]
    fn windows_style_commands_map_to_their_values() {
        let base = Look::default();
        for (i, w) in (0_u32..).zip(WinStyle::ALL) {
            assert_eq!(look_after(base, cmd::STYLE + i).map(|l| l.style), Some(w));
        }
        for (i, c) in (0_u32..).zip(XpColor::ALL) {
            assert_eq!(look_after(base, cmd::XP_COLOUR + i).map(|l| l.xp), Some(c));
        }
        for (i, a) in (0_u32..).zip(ACCENTS) {
            assert_eq!(
                look_after(base, cmd::ACCENT + i).map(|l| l.accent),
                Some(a.rgb())
            );
        }
        let off = look_after(base, cmd::AERO_OFF);
        assert_eq!(off.map(|l| l.aero), Some(false));
        assert_eq!(
            off.and_then(|l| look_after(l, cmd::AERO_ON))
                .map(|l| l.aero),
            Some(true)
        );
        let dark = look_after(base, cmd::APP_DARK);
        assert_eq!(dark.map(|l| l.dark), Some(true));
        assert_eq!(
            dark.and_then(|l| look_after(l, cmd::APP_LIGHT))
                .map(|l| l.dark),
            Some(false)
        );
        // Aqua's appearances, and a macOS accent goes to the macOS field.
        assert_eq!(
            look_after(base, cmd::AQUA + 1).map(|l| l.aqua),
            Some(AquaColor::Graphite)
        );
        let mac = Look {
            style: WinStyle::MacModern,
            ..base
        };
        let pink = look_after(mac, cmd::ACCENT + 2);
        assert_eq!(
            pink.map(|l| l.mac_accent),
            MAC_ACCENTS.get(2).map(|a| a.rgb())
        );
        assert_eq!(pink.map(|l| l.accent), Some(base.accent));
        assert_eq!(
            look_after(mac, cmd::ACCENT + MAC_ACCENTS.len() as u32),
            None
        );
        // Only the chosen field changes.
        assert_eq!(
            look_after(base, cmd::APP_DARK).map(|l| (l.style, l.xp, l.aero)),
            Some((base.style, base.xp, base.aero))
        );
        let gaps = (cmd::STYLE + WinStyle::ALL.len() as u32..cmd::XP_COLOUR)
            .chain(cmd::XP_COLOUR + XpColor::ALL.len() as u32..cmd::AQUA)
            .chain(cmd::AQUA + AquaColor::ALL.len() as u32..cmd::AERO_ON)
            .chain(cmd::YARU + 1..cmd::ACCENT)
            .chain(cmd::ACCENT + ACCENTS.len() as u32..=cmd::STYLE_LAST + 1)
            .chain([cmd::STYLE - 1, 0]);
        for id in gaps {
            assert_eq!(look_after(base, id), None, "{id}");
        }
    }

    #[test]
    fn the_style_menu_shows_the_variants_of_the_chosen_style() {
        for style in WinStyle::ALL {
            let look = Look {
                style,
                ..Look::default()
            };
            let got = ids(&style_items(look, ThemeChoice::Classic));
            let has = |id: u32| got.contains(&id);
            assert_eq!(has(cmd::XP_COLOUR), style == WinStyle::Xp, "{style:?}");
            assert_eq!(has(cmd::AERO_ON), style.has_aero(), "{style:?}");
            assert_eq!(has(cmd::ACCENT), style.has_accent(), "{style:?}");
            assert_eq!(has(cmd::APP_DARK), style.has_app_mode(), "{style:?}");
            for i in 0..WinStyle::ALL.len() as u32 {
                assert!(has(cmd::STYLE + i));
            }
        }
        // The XP schemes and Windows 10's modes are radio items in the menu
        // itself, as Aero and Basic are.
        let xp = style_items(Look::default(), ThemeChoice::Classic);
        // A separator between the last Windows style and the first macOS one.
        let first_mac = WinStyle::ALL.iter().position(|w| w.is_mac());
        let at = first_mac.and_then(|k| {
            xp.iter().position(|i| {
                matches!(i.kind, crate::win95::menu::Kind::Command(c)
                    if Some(c) == u32::try_from(k).ok().map(|k| cmd::STYLE + k))
            })
        });
        assert!(
            at.and_then(|p| p.checked_sub(1))
                .and_then(|p| xp.get(p))
                .is_some_and(Item::is_separator),
            "{at:?}"
        );
        assert!(xp.iter().any(
            |i| matches!(i.kind, crate::win95::menu::Kind::Command(c) if c == cmd::XP_COLOUR)
        ));
    }

    #[test]
    fn accent_swatches_show_the_colour_drawn() {
        let swatches = |look: Look| -> Vec<Option<Glyph>> {
            style_items(look, ThemeChoice::Classic)
                .into_iter()
                .find_map(|i| match i.kind {
                    crate::win95::menu::Kind::Submenu(items) => Some(items),
                    _ => None,
                })
                .unwrap_or_default()
                .into_iter()
                .map(|i| i.glyph)
                .collect()
        };
        for (style, dark, yaru) in [
            (WinStyle::MacModern, false, false),
            (WinStyle::MacModern, true, false),
            (WinStyle::Win10, true, false),
            (WinStyle::LinuxBreeze, false, false),
            (WinStyle::LinuxBreeze, true, false),
            (WinStyle::LinuxAdwaita, false, false),
            (WinStyle::LinuxAdwaita, false, true),
        ] {
            let look = Look {
                style,
                dark,
                yaru,
                ..Look::default()
            };
            let want: Vec<Option<Glyph>> = look
                .accents()
                .iter()
                .map(|a| {
                    let mut l = look;
                    l.set_accent(a.rgb());
                    Some(swatch_glyph(l.drawn_accent()))
                })
                .collect();
            assert!(!want.is_empty(), "{style:?}");
            assert_eq!(swatches(look), want, "{style:?} dark={dark} yaru={yaru}");
        }
    }

    #[test]
    fn linux_accent_swatches_show_yaru_orange_and_breeze_blue() {
        use sn_ui_core::winstyle::{BREEZE_BLUE, DEFAULT_LINUX_ACCENT, YARU_ORANGE};
        let first = |look: Look| {
            style_items(look, ThemeChoice::Classic)
                .into_iter()
                .find_map(|i| match i.kind {
                    crate::win95::menu::Kind::Submenu(items) => items.into_iter().next(),
                    _ => None,
                })
        };
        for (look, rgb) in [
            (
                Look {
                    style: WinStyle::LinuxAdwaita,
                    yaru: true,
                    ..Look::default()
                },
                YARU_ORANGE,
            ),
            (
                Look {
                    style: WinStyle::LinuxBreeze,
                    ..Look::default()
                },
                BREEZE_BLUE,
            ),
            (
                Look {
                    style: WinStyle::LinuxAdwaita,
                    ..Look::default()
                },
                DEFAULT_LINUX_ACCENT,
            ),
        ] {
            // The default swatch is ticked and shows the colour drawn.
            assert_eq!(look.drawn_accent(), rgb, "{:?}", look.style);
            let item = first(look);
            assert_eq!(
                item.as_ref().and_then(|i| i.glyph.clone()),
                Some(swatch_glyph(rgb)),
                "{:?}",
                look.style
            );
            assert_eq!(item.map(|i| i.check), Some(Check::Dot), "{:?}", look.style);
        }
        // Yaru still offers GNOME's blue.
        let yaru = Look {
            style: WinStyle::LinuxAdwaita,
            yaru: true,
            ..Look::default()
        };
        let blue = YARU_ACCENTS
            .iter()
            .position(|a| a.rgb() == DEFAULT_LINUX_ACCENT)
            .and_then(|k| u32::try_from(k).ok())
            .and_then(|k| look_after(yaru, cmd::ACCENT + k));
        assert_eq!(blue.map(Look::drawn_accent), Some(DEFAULT_LINUX_ACCENT));
    }

    #[test]
    fn gnome_commands_and_linux_accents_change_only_their_fields() {
        let adwaita = Look {
            style: WinStyle::LinuxAdwaita,
            ..Look::default()
        };
        let yaru = look_after(adwaita, cmd::YARU);
        assert_eq!(yaru.map(|l| l.yaru), Some(true));
        assert_eq!(
            yaru.and_then(|l| look_after(l, cmd::GNOME)).map(|l| l.yaru),
            Some(false)
        );
        let breeze = Look {
            style: WinStyle::LinuxBreeze,
            ..Look::default()
        };
        let green = look_after(breeze, cmd::ACCENT + 2);
        assert_eq!(
            green.map(|l| l.breeze_accent),
            BREEZE_ACCENTS.get(2).map(|a| a.rgb())
        );
        assert_eq!(
            green.map(|l| (l.accent, l.mac_accent, l.linux_accent, l.yaru_accent)),
            Some((
                breeze.accent,
                breeze.mac_accent,
                breeze.linux_accent,
                breeze.yaru_accent
            ))
        );
        assert_eq!(
            look_after(breeze, cmd::ACCENT + BREEZE_ACCENTS.len() as u32),
            None
        );
        // GNOME's accents go to GNOME's field.
        let teal = look_after(adwaita, cmd::ACCENT + 1);
        assert_eq!(
            teal.map(|l| l.linux_accent),
            LINUX_ACCENTS.get(1).map(|a| a.rgb())
        );
        assert_eq!(teal.map(|l| l.breeze_accent), Some(adwaita.breeze_accent));
        let has_variants = |look: Look| {
            let got = ids(&style_items(look, ThemeChoice::Classic));
            (got.contains(&cmd::GNOME), got.contains(&cmd::YARU))
        };
        assert_eq!(has_variants(adwaita), (true, true));
        // Each variant item describes itself in the status bar.
        let items = style_items(adwaita, ThemeChoice::Classic);
        let hint_of = |id: u32| {
            items.iter().find_map(|i| match i.kind {
                crate::win95::menu::Kind::Command(c) if c == id => Some(i.hint.clone()),
                _ => None,
            })
        };
        for (id, key) in [
            (cmd::GNOME, "winstyle-gnome.hover"),
            (cmd::YARU, "winstyle-yaru.hover"),
        ] {
            let hint = t!(key);
            assert!(!hint.starts_with("winstyle-"), "{key}: {hint}");
            assert_eq!(hint_of(id), Some(hint), "{key}");
        }
        assert_ne!(hint_of(cmd::GNOME), hint_of(cmd::YARU));
        assert_eq!(has_variants(breeze), (false, false));
    }

    #[test]
    fn high_contrast_greys_the_style_variants() {
        let enabled_of = |items: &[Item], id: u32| {
            items.iter().find_map(|i| match i.kind {
                crate::win95::menu::Kind::Command(c) if c == id => Some(i.enabled),
                _ => None,
            })
        };
        let has_label = |items: &[Item], key: &str| {
            let text = t!(key);
            items.iter().any(|i| i.label == text && !i.enabled)
        };
        let w10 = Look {
            style: WinStyle::Win10,
            ..Look::default()
        };
        let normal = style_items(w10, ThemeChoice::Classic);
        assert_eq!(enabled_of(&normal, cmd::AERO_ON), None);
        assert_eq!(enabled_of(&normal, cmd::APP_DARK), Some(true));
        assert!(has_label(&normal, "winstyle-scheme-note"));
        assert!(!has_label(&normal, "winstyle-hc-active"));
        let hc = style_items(w10, ThemeChoice::HighContrastDark);
        assert_eq!(enabled_of(&hc, cmd::APP_DARK), Some(false));
        assert_eq!(enabled_of(&hc, cmd::APP_LIGHT), Some(false));
        assert!(has_label(&hc, "winstyle-hc-active"));
        // The versions themselves stay available.
        assert_eq!(enabled_of(&hc, cmd::STYLE), Some(true));
        let w7 = Look {
            style: WinStyle::Win7,
            ..Look::default()
        };
        let hc7 = style_items(w7, ThemeChoice::HighContrastLight);
        assert_eq!(enabled_of(&hc7, cmd::AERO_ON), Some(false));
        let xp_hc = style_items(Look::default(), ThemeChoice::HighContrastLight);
        assert_eq!(enabled_of(&xp_hc, cmd::XP_COLOUR), Some(false));
        // The classic styles are drawn as chosen, so nothing is greyed.
        let w98 = Look {
            style: WinStyle::Win98,
            ..Look::default()
        };
        let hc98 = style_items(w98, ThemeChoice::HighContrastDark);
        assert!(has_label(&hc98, "winstyle-scheme-note"));
    }

    #[test]
    fn the_glass_hint_is_shown_on_aero_and_basic() {
        let w7 = Look {
            style: WinStyle::Win7,
            ..Look::default()
        };
        let hint = t!("winstyle-aero.hover");
        assert!(!hint.is_empty() && !hint.starts_with("winstyle-"));
        let items = style_items(w7, ThemeChoice::Classic);
        for id in [cmd::AERO_ON, cmd::AERO_OFF] {
            assert!(items.iter().any(|i| {
                matches!(i.kind, crate::win95::menu::Kind::Command(c) if c == id) && i.hint == hint
            }));
        }
    }

    #[test]
    fn the_theme_menu_says_what_it_colours_under_themed_styles() {
        let note = t!("winstyle-scheme-note");
        let hint = note_hint("winstyle-scheme-note");
        assert!(!hint.is_empty() && !hint.starts_with("winstyle-"));
        let has_note = |look: Look| {
            let m = theme_menu(ThemeChoice::Classic, look);
            let crate::win95::menu::Kind::Submenu(items) = &m.kind else {
                return false;
            };
            items
                .iter()
                .any(|i| i.label == note && !i.enabled && i.hint == hint)
                && m.hint == hint
        };
        assert!(has_note(Look::default()), "XP");
        assert!(!has_note(Look::WIN95));
    }

    #[test]
    fn the_theme_menu_starts_with_the_windows_schemes() {
        let items = theme_items(ThemeChoice::Classic);
        assert_eq!(
            items.len(),
            ThemeChoice::ALL.len() + 3,
            "three group separators"
        );
        assert_eq!(items[0].label, scheme_name(ThemeChoice::Classic));
        assert_eq!(items[0].check, Check::Dot);
    }
}
