//! The SA MDI frame: main menu, MDI client area with the signal windows,
//! status bar, SA's loader, dialogs and message boxes, and the session behind
//! them (`model.rs`, `windows.rs`, `sessions.rs`), as in `sa-app`.

pub(crate) mod dialogs;
#[cfg(debug_assertions)]
mod gallery;
pub(crate) mod keys;
pub(crate) mod loader;
mod mac_window;
pub(crate) mod menus;
pub(crate) mod model;
mod native_menu;
mod sessions;
// The sonogram work window lives in `src/sono/`; as a child of `app` it
// extends the frame directly.
#[path = "../sono/mod.rs"]
pub(crate) mod sono;
// The PhasePlane and harmonics (`(q)`) windows live in `src/phase/` and
// `src/harmonics/`.
#[path = "../harmonics/mod.rs"]
pub(crate) mod harmonics;
#[path = "../phase/mod.rs"]
pub(crate) mod phase;
// The demodulator and SA-OFDM windows live in `src/demod/` and `src/ofdm/`.
#[path = "../demod/mod.rs"]
pub(crate) mod demod;
#[path = "../ofdm/mod.rs"]
pub(crate) mod ofdm;
mod status;
#[cfg(test)]
mod tests;
// The WF window and its VMW mode live in `src/wf/`.
mod texts;
#[path = "../wf/mod.rs"]
pub(crate) mod wf;
// The signal generator, its calculators, the workflow guide and the
// shortcuts list live in `src/tools/`.
#[path = "../tools/mod.rs"]
pub(crate) mod tools;
mod windows;

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    AnyElement, Context, ExternalPaths, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent,
    ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Render,
    Styled, Window, WindowAppearance, canvas, div, px,
};
use sn_core::{MeasurementContext, NodeId, SignalGraph};
use sn_dsp::overview::Overview;
use sn_ui_core::app_glue::Confirm;
use sn_ui_core::i18n::{self, Lang};
use sn_ui_core::prefs::{Prefs, StartSize};
use sn_ui_core::t;
use sn_ui_core::theme::ThemeChoice;
use sn_ui_core::titles::Counters;

pub(crate) use dialogs::{MsgAction, colored_label};
use dialogs::{MsgQueue, ParamDialog};
use loader::{Loader, Memory};
use menus::{MenuState, cmd};
use model::{AppJob, NodeUi, WinKind};
use sessions::Chooser;
use texts::TextWin;

use crate::store::Stored;
use crate::win95::colors::{Role, apply_theme, scheme};
use crate::win95::dialog::{MsgBox, message_box};
use crate::win95::geom::Rect;
use crate::win95::kit::{Host, Kit, capture_layer, lens};
use crate::win95::mdi::{self, Hit, Mdi, State};
use crate::win95::mdi_view::{MdiEvent, MdiHooks, caption_button_of, caption_tips, mdi_client};
use crate::win95::menu::{Item, Menu, Tracker};
use crate::win95::menu_view::{MenuHooks, Menus, bar_h, menu_bar, menu_popups};
use crate::win95::skin::CapButton;
use crate::win95::text::LINE;
use crate::win95::widget::{place, size_of};

/// How often running jobs are checked (progress fields, results).
const POLL: Duration = Duration::from_millis(50);

/// The frame's title: SA's `Signals Analyzer [v 6.2.6.7]` format with our name.
pub(crate) fn frame_title() -> String {
    format!("SigNexora [v {}]", env!("CARGO_PKG_VERSION"))
}

/// What an MDI child shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChildKind {
    /// A signal in one of SA's views.
    Signal(NodeId, WinKind),
    /// A read-only text (script output, session problems).
    Text,
    /// A tool window (signal generator, calculators, guide, shortcuts).
    Tool(tools::Tool),
    /// The widget gallery (debug builds).
    #[cfg(debug_assertions)]
    Gallery,
}

/// The open popup menu: a child's system menu or one of the loader's.
#[derive(Clone, Debug)]
struct Popup {
    items: Rc<Vec<Item>>,
    at: (i32, i32),
    /// The child the system menu belongs to.
    child: Option<u32>,
    /// The window icon (or Motif window-menu button) the system menu was
    /// opened from: a second click on it closes the child.
    icon: Rect,
}

/// How the frame starts: the settings, where they are stored, and a file to
/// open (`sn-gui <file>`).
#[derive(Clone, Debug, Default)]
pub(crate) struct Setup {
    pub(crate) stored: Stored,
    /// The settings file; `None` keeps the settings for this run only.
    pub(crate) store: Option<PathBuf>,
    pub(crate) file: Option<PathBuf>,
}

/// The frame window's view.
// The flags are independent: menus, polling, unsaved work.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct Frame {
    kit: Kit<Self>,
    focus: FocusHandle,
    mdi: Mdi,
    outline: Option<Rect>,
    kinds: Vec<(u32, ChildKind)>,
    menus: Rc<Vec<Menu>>,
    track: Tracker,
    popup: Option<Popup>,
    popup_track: Tracker,
    msgbox: Option<MsgBox>,
    /// The message box needs centring before it is first drawn.
    msgbox_new: bool,
    /// What answering the open box does.
    msg_action: MsgAction,
    msg_queue: MsgQueue,
    /// Alt went down with no other key since (an Alt tap selects the menu bar).
    alt_alone: bool,
    /// The menu last put in the macOS menu bar (its signature), `None` while
    /// the window draws its own menu.
    native_menus: Option<String>,
    /// The appearance last given to the main window's native title bar.
    window_appearance: Option<mac_window::TitleAppearance>,
    // The session.
    pub(crate) graph: SignalGraph,
    pub(crate) measures: MeasurementContext,
    pub(crate) ui: HashMap<NodeId, NodeUi>,
    pub(crate) overviews: HashMap<NodeId, Arc<Overview>>,
    titles: Counters,
    /// Results whose windows were all closed: no longer part of the session.
    discarded: BTreeSet<NodeId>,
    jobs: Vec<AppJob>,
    /// Bumped whenever the session is replaced.
    generation: u64,
    /// Work since the last save that would be lost.
    dirty: bool,
    /// Bumped with every change that sets `dirty` (see `Frame::mark_dirty`).
    changes: u64,
    session_path: Option<PathBuf>,
    /// Carried out once the running session save succeeds.
    after_save: Option<Confirm>,
    /// Confirmed actions waiting for the next frame (they need the context).
    pending: Vec<Confirm>,
    /// Replay or script results waiting for the user's answer before they
    /// replace work done while they ran, oldest first: one question each, and
    /// the questions are answered in the same order.
    held: VecDeque<model::Output>,
    /// The last message (status bar).
    status: String,
    /// The status bar's rate and path, kept when the last window closes.
    last_source: Option<(f64, String)>,
    /// The pointer (window coordinates), for the status bar.
    pointer: Option<(i32, i32)>,
    // Settings.
    prefs: Prefs,
    /// A look drawn instead of the stored one (debug builds: `SA_GUI_STYLE`
    /// for window captures). It is never saved; choosing a Windows style
    /// clears it.
    look_override: Option<sn_ui_core::winstyle::Look>,
    stored: Stored,
    store: Option<PathBuf>,
    /// The frame's size, remembered for `Size of start SA`.
    last_size: Option<(f32, f32)>,
    // Dialogs and windows.
    loader: Option<Loader>,
    loader_memory: Memory,
    params: Option<ParamDialog>,
    texts: HashMap<u32, TextWin>,
    /// The sonogram work windows.
    sono: sono::SonoHub,
    /// The PhasePlane windows and the harmonics module.
    phase: phase::PhaseHub,
    harmonics: harmonics::HarmonicsHub,
    /// The demodulator and SA-OFDM states, per signal.
    demod: demod::DemodHub,
    ofdm: ofdm::OfdmHub,
    /// The WF windows and their VMW mode.
    wf: wf::WfHub,
    /// The tool windows: signal generator, calculators, guide, shortcuts.
    pub(crate) tools: tools::ToolsHub,
    /// A poll task is running.
    polling: bool,
    /// Steps of the busy progress fields.
    tick: u32,
    #[cfg(debug_assertions)]
    gallery: gallery::Gallery,
}

impl Host for Frame {
    fn kit(&mut self) -> &mut Kit<Self> {
        &mut self.kit
    }
}

/// Whether the window shows the system's dark appearance.
fn dark_mode(window: &Window) -> bool {
    matches!(
        window.appearance(),
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

impl Frame {
    /// A frame with default settings that are not stored (tests).
    #[cfg(test)]
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        Self::with_setup(Setup::default(), window, cx)
    }

    pub(crate) fn with_setup(
        setup: Setup,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        // Captions turn inactive when another application is active.
        cx.observe_window_activation(window, |v: &mut Self, window, cx| {
            v.window_activation(window.is_window_active());
            cx.notify();
        })
        .detach();
        cx.observe_window_appearance(window, |v: &mut Self, window, cx| {
            apply_theme(cx, v.prefs.theme, v.look(), dark_mode(window));
            cx.notify();
        })
        .detach();
        let prefs = setup.stored.prefs;
        #[cfg(debug_assertions)]
        let look_override = std::env::var("SA_GUI_STYLE").ok().and_then(|v| {
            let look = debug_look(&v, prefs.look);
            if look.is_none() {
                eprintln!("sn-gui: SA_GUI_STYLE={v:?} is not a look; the stored one is drawn");
            }
            look
        });
        #[cfg(not(debug_assertions))]
        let look_override = None;
        apply_theme(
            cx,
            prefs.theme,
            look_override.unwrap_or(prefs.look),
            dark_mode(window),
        );
        let spec = sn_ui_core::prefs::generator_from_json(
            setup
                .stored
                .generator
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
        );
        let mut this = Self {
            kit: Kit::default(),
            focus,
            mdi: Mdi::new(800, 500),
            outline: None,
            kinds: Vec::new(),
            menus: Rc::new(Vec::new()),
            track: Tracker::default(),
            popup: None,
            popup_track: Tracker::default(),
            msgbox: None,
            msgbox_new: false,
            msg_action: MsgAction::None,
            msg_queue: MsgQueue::new(),
            alt_alone: false,
            native_menus: None,
            // A new window follows the system's appearance.
            window_appearance: Some(mac_window::TitleAppearance::System),
            graph: SignalGraph::default(),
            measures: MeasurementContext::default(),
            ui: HashMap::new(),
            overviews: HashMap::new(),
            titles: Counters::default(),
            discarded: BTreeSet::new(),
            jobs: Vec::new(),
            generation: 0,
            dirty: false,
            changes: 0,
            session_path: None,
            after_save: None,
            pending: Vec::new(),
            held: VecDeque::new(),
            status: String::new(),
            last_source: None,
            pointer: None,
            prefs,
            look_override,
            last_size: setup.stored.frame,
            stored: setup.stored,
            store: setup.store,
            loader: None,
            loader_memory: Memory::default(),
            params: None,
            texts: HashMap::new(),
            sono: sono::SonoHub::default(),
            phase: phase::PhaseHub::default(),
            harmonics: harmonics::HarmonicsHub::default(),
            demod: demod::DemodHub::default(),
            ofdm: ofdm::OfdmHub::default(),
            wf: wf::WfHub::default(),
            tools: tools::ToolsHub::new(spec),
            polling: false,
            tick: 0,
            #[cfg(debug_assertions)]
            gallery: gallery::Gallery::default(),
        };
        if let Some(f) = setup.file {
            this.open_loader(Some(f));
        }
        #[cfg(debug_assertions)]
        this.apply_scene();
        this
    }

    /// Open an MDI child with a client area of `cw` x `ch`, 24 px lower right
    /// from window `near`.
    fn open_child(
        &mut self,
        title: &str,
        cw: i32,
        ch: i32,
        resizable: bool,
        near: Option<u32>,
        kind: ChildKind,
    ) -> u32 {
        let (w, h) = (
            cw + 2 * mdi::frame_w(),
            ch + 2 * mdi::frame_w() + mdi::caption_h(),
        );
        let id = self.mdi.add(title, w, h, resizable, near);
        self.kinds.push((id, kind));
        self.kit.focus = None;
        id
    }

    /// Show a message box (centred on its first frame).
    pub(crate) fn show_message(&mut self, mb: MsgBox) {
        self.msgbox = Some(mb);
        self.msgbox_new = true;
        self.track.close();
        self.popup = None;
        self.kit.focus = None;
        self.kit.popup = None;
    }

    /// The look drawn: the stored one, or the debug override.
    fn look(&self) -> sn_ui_core::winstyle::Look {
        self.look_override.unwrap_or(self.prefs.look)
    }

    /// The menu state.
    fn menu_state(&self) -> MenuState {
        let sono = match (self.active_kind(), self.active_node()) {
            (Some(WinKind::Sonogram), Some(n)) => self.ui.get(&n).map(|u| u.settings),
            _ => None,
        };
        MenuState {
            prefs: Prefs {
                look: self.look(),
                ..self.prefs
            },
            has_signals: self.has_signals(),
            saving: self.saving_session(),
            path: self.session_path.as_ref().map(|p| p.display().to_string()),
            sono_active: sono.is_some(),
            sono: sono.unwrap_or(self.prefs.sonogram),
        }
    }

    /// Remember the settings in the settings file (if this frame has one).
    /// A failure is shown in the status bar and returned as the message.
    pub(crate) fn save_prefs(&mut self) -> Result<(), String> {
        self.remember_generator();
        self.stored.prefs = self.current_prefs();
        self.stored.frame = self.last_size;
        let Some(path) = self.store.clone() else {
            return Ok(());
        };
        self.stored.save(&path).map_err(|e| {
            let path = path.display().to_string();
            let msg = match e {
                crate::store::SaveError::Encode => t!("gui-prefs-not-encoded", path = path),
                crate::store::SaveError::Io(e) => {
                    t!("gui-prefs-not-saved", path = path, error = e.to_string())
                }
            };
            self.status.clone_from(&msg);
            msg
        })
    }

    /// Run a menu command.
    fn command(&mut self, id: u32, window: &mut Window, cx: &mut Context<'_, Self>) {
        match id {
            cmd::OPEN_FILE => self.open_loader(None),
            cmd::OPEN_SESSION => self.request(Confirm::OpenSession, cx),
            cmd::SAVE_SESSION => self.save_session(cx),
            cmd::SAVE_SESSION_AS => self.choose(Chooser::SaveSessionAs, cx),
            cmd::REPORT_HTML | cmd::REPORT_MARKDOWN => {
                self.choose(Chooser::Report(id == cmd::REPORT_HTML), cx);
            }
            cmd::RUN_SCRIPT => self.request(Confirm::RunScript, cx),
            cmd::EXIT => self.request(Confirm::Quit, cx),
            cmd::CLOSE_ALL => self.request(Confirm::CloseAll, cx),
            #[cfg(debug_assertions)]
            cmd::GALLERY => self.open_gallery(),
            cmd::ABOUT => self.about(),
            cmd::SYS_RESTORE..=cmd::SYS_SIZE => self.system_command(id),
            cmd::LD_UPDATE..=cmd::LD_CANCEL => self.loader_command(id, cx),
            cmd::SW_SAVE..=cmd::SW_LAST => self.sono_command(id, cx),
            cmd::WF_SAVE..=cmd::WF_LAST => self.wf_command(id, cx),
            cmd::PP_COPY_TEXT..=cmd::PP_LAST => self.phase_command(id, cx),
            demod::DM_FIRST..=demod::DM_LAST => self.demod_command(id, cx),
            ofdm::OF_FIRST..=ofdm::OF_LAST => self.ofdm_command(id, cx),
            cmd::DEMO => {
                self.open_tool(tools::Tool::Guide);
            }
            cmd::USE_HELP => {
                self.open_tool(tools::Tool::Help);
            }
            cmd::GENERATOR => {
                self.open_tool(tools::Tool::Generator);
            }
            tools::OC_FIRST..=tools::OC_LAST => self.tool_command(id),
            other => self.option_command(other, window, cx),
        }
        cx.notify();
    }

    /// Options menu commands; they are remembered at once.
    fn option_command(&mut self, id: u32, window: &mut Window, cx: &mut Context<'_, Self>) {
        let index = |base: u32| (id - base) as usize;
        match id {
            i if (cmd::START_SIZE..cmd::START_SIZE + StartSize::ALL.len() as u32).contains(&i) => {
                if let Some(s) = StartSize::ALL.get(index(cmd::START_SIZE)) {
                    self.prefs.start_size = *s;
                }
            }
            cmd::INFO_ON | cmd::INFO_OFF => self.prefs.info_default = id == cmd::INFO_ON,
            i if (cmd::LANGUAGE..=cmd::LANGUAGE + Lang::ALL.len() as u32).contains(&i) => {
                self.prefs.language = index(cmd::LANGUAGE)
                    .checked_sub(1)
                    .and_then(|l| Lang::ALL.get(l).copied());
                i18n::set_language(self.prefs.effective_language());
            }
            i if (cmd::STYLE..=cmd::STYLE_LAST).contains(&i) => {
                if let Some(look) = menus::look_after(self.look(), i) {
                    let before = crate::win95::style::metrics();
                    self.look_override = None;
                    self.prefs.look = look;
                    apply_theme(cx, self.prefs.theme, look, dark_mode(window));
                    self.mdi.restyle(before, crate::win95::style::metrics());
                }
            }
            i if (cmd::THEME..cmd::THEME + ThemeChoice::ALL.len() as u32).contains(&i) => {
                if let Some(c) = ThemeChoice::ALL.get(index(cmd::THEME)) {
                    // High contrast switches between the classic and the
                    // themed frames.
                    let before = crate::win95::style::metrics();
                    self.prefs.theme = *c;
                    apply_theme(cx, *c, self.look(), dark_mode(window));
                    self.mdi.restyle(before, crate::win95::style::metrics());
                }
            }
            i if (cmd::COLOURS
                ..cmd::COLOURS + sn_ui_core::colormap::Colormap::ALL.len() as u32)
                .contains(&i) =>
            {
                if let Some(c) = sn_ui_core::colormap::Colormap::ALL.get(index(cmd::COLOURS)) {
                    self.edit_sono_settings(|s| s.colormap = *c);
                }
            }
            cmd::ZOOM_PEAK | cmd::ZOOM_MEAN => {
                let c = if id == cmd::ZOOM_PEAK {
                    sn_dsp::spectrum::Combine::PeakHold
                } else {
                    sn_dsp::spectrum::Combine::Mean
                };
                self.edit_sono_settings(|s| s.combine = c);
            }
            _ => return,
        }
        // A failure is in the status bar; the change itself took effect.
        let _ = self.save_prefs();
        window.refresh();
    }

    /// Change the active sonogram window's settings, or the defaults for new
    /// recordings when no sonogram window is active (as `sa-app` does).
    fn edit_sono_settings(&mut self, f: impl Fn(&mut sn_ui_core::sono_settings::Settings)) {
        let active = match (self.active_kind(), self.active_node()) {
            (Some(WinKind::Sonogram), Some(n)) => self.ui.get_mut(&n),
            _ => None,
        };
        match active {
            Some(nui) => f(&mut nui.settings),
            None => f(&mut self.prefs.sonogram),
        }
    }

    /// Debug builds: put the frame in the state named by `SA_GUI_SCENE`, for
    /// window screenshots without any input (see `shot.rs`).
    #[cfg(debug_assertions)]
    fn apply_scene(&mut self) {
        use crate::win95::menu::Key;
        let scene = std::env::var("SA_GUI_SCENE").unwrap_or_default();
        let menus = menus::main_menu(&self.menu_state());
        let src = crate::win95::menu::Source::Bar(&menus);
        let keys: &[Key] = match scene.as_str() {
            "file" => &[Key::AltChar('f')],
            "options" => &[Key::AltChar('o'), Key::Down, Key::Right],
            "language" => &[Key::AltChar('o'), Key::Down, Key::Down, Key::Right],
            "style" => &[
                Key::AltChar('o'),
                Key::Down,
                Key::Down,
                Key::Down,
                Key::Right,
            ],
            "theme" => &[
                Key::AltChar('o'),
                Key::Down,
                Key::Down,
                Key::Down,
                Key::Down,
                Key::Right,
            ],
            "bar" => &[Key::Alt],
            _ => &[],
        };
        for k in keys {
            self.track.key(src, *k);
        }
        match scene.as_str() {
            "gallery" | "combo" | "focus" => {
                self.open_gallery();
                if scene == "combo" {
                    self.kit.popup = Some("g.winf".into());
                    self.kit.focus = Some("g.winf".into());
                    self.gallery.win_f.open();
                } else if scene == "focus" {
                    self.kit.focus = Some("g.name".into());
                    self.gallery.name.select_all();
                }
            }
            "error" => self.alert("Cannot open the file:\nthe format is not supported."),
            "about" => self.about(),
            "sonogram" | "sono-popup" | "sono-brc" | "sono-collapsed" => self.sono_scene(&scene),
            "phase" | "phase-plane" | "harmonics" => self.phase_scene(&scene),
            "demod" => self.demod_scene(),
            "ofdm" | "ofdm-survey" | "ofdm-show" | "ofdm-k" => self.ofdm_scene(&scene),
            "wf" | "wf-popup" | "vmw" => self.wf_debug_scene(&scene),
            "loader" => self.open_loader(None),
            "generator" | "calculators" | "guide" | "help" => {
                let tool = match scene.as_str() {
                    "generator" => tools::Tool::Generator,
                    "calculators" => tools::Tool::Calculators,
                    "guide" => tools::Tool::Guide,
                    _ => tools::Tool::Help,
                };
                self.open_tool(tool);
            }
            "max" => {
                if let Some(id) = self.mdi.active() {
                    self.mdi.command(id, Hit::Maximize);
                }
            }
            _ => {}
        }
    }

    #[cfg(debug_assertions)]
    fn open_gallery(&mut self) {
        let existing = self
            .kinds
            .iter()
            .find(|(_, k)| *k == ChildKind::Gallery)
            .map(|(id, _)| *id);
        if let Some(id) = existing {
            self.mdi.activate(id);
        } else {
            let (w, h) = gallery::SIZE;
            self.open_child(
                "Widget gallery",
                w,
                h,
                true,
                self.mdi.active(),
                ChildKind::Gallery,
            );
        }
    }

    /// A command of a child's system menu.
    fn system_command(&mut self, id: u32) {
        let Some(child) = self
            .popup
            .as_ref()
            .and_then(|p| p.child)
            .or(self.mdi.active())
        else {
            return;
        };
        let state = self.mdi.get(child).map(|c| c.state);
        match id {
            cmd::SYS_CLOSE => self.close_child(child),
            cmd::SYS_NEXT => self.mdi.next(false),
            cmd::SYS_MINIMIZE => self.mdi.command(child, Hit::Minimize),
            cmd::SYS_MAXIMIZE => self.mdi.command(child, Hit::Maximize),
            cmd::SYS_RESTORE => {
                let hit = if state == Some(State::Minimized) {
                    Hit::Minimize
                } else {
                    Hit::Maximize
                };
                self.mdi.command(child, hit);
            }
            _ => {}
        }
        self.popup = None;
    }

    /// Close a child: a signal window is SA's undo; other windows just close.
    fn close_child(&mut self, id: u32) {
        if self.binding(id).is_some() {
            self.close_window(id);
        } else {
            self.mdi.close(id);
            self.kinds.retain(|(k, _)| *k != id);
            self.texts.remove(&id);
            self.tool_closed(id);
            self.kit.focus = None;
        }
    }

    fn mdi_hooks() -> MdiHooks<Self> {
        MdiHooks {
            lens: lens(|v: &mut Self| Some(&mut v.mdi)),
            outline: lens(|v: &mut Self| Some(&mut v.outline)),
            on_event: Rc::new(|v: &mut Self, ev, _, cx| {
                match ev {
                    MdiEvent::Close(id) => v.close_child(id),
                    MdiEvent::SysMenu(id, at) => {
                        // The menu opens under the icon, which is at the
                        // left of the caption just above `at`.
                        let h = mdi::caption_h();
                        let icon = Rect::new(at.0, at.1 - h, mdi::icon_zone_w(h), h);
                        v.open_system_menu(id, at, icon);
                    }
                    MdiEvent::Activated(_) => v.kit.focus = None,
                }
                cx.notify();
            }),
        }
    }

    fn open_system_menu(&mut self, id: u32, at: (i32, i32), icon: Rect) {
        let Some(c) = self.mdi.get(id) else { return };
        let items = menus::system_menu(
            c.state == State::Minimized,
            c.state == State::Maximized,
            c.resizable,
        );
        self.open_popup(items, at);
        if let Some(p) = self.popup.as_mut() {
            p.child = Some(id);
            p.icon = icon;
        }
    }

    /// Open a popup menu at window position `at`.
    pub(crate) fn open_popup(&mut self, items: Vec<Item>, at: (i32, i32)) {
        self.popup = Some(Popup {
            items: Rc::new(items),
            at,
            child: None,
            icon: Rect::default(),
        });
        self.popup_track.open_popup();
        self.track.close();
    }

    fn menu_hooks(popup: bool) -> MenuHooks<Self> {
        MenuHooks {
            tracker: if popup {
                lens(|v: &mut Self| Some(&mut v.popup_track))
            } else {
                lens(|v: &mut Self| Some(&mut v.track))
            },
            on_invoke: Rc::new(|v: &mut Self, id, window, cx| v.command(id, window, cx)),
            on_outside: popup.then(|| {
                let f: crate::win95::menu_view::OutsideClick<Self> =
                    Rc::new(|v: &mut Self, (at, clicks), _, cx| v.popup_outside(at, clicks, cx));
                f
            }),
        }
    }

    /// A click outside the open popup menu: the second click of a double
    /// click on a window's icon (the first opened its system menu) closes
    /// the window, as in Windows.
    fn popup_outside(&mut self, (x, y): (i32, i32), clicks: usize, cx: &mut Context<'_, Self>) {
        let Some(p) = self.popup.take() else {
            return;
        };
        let Some(id) = p.child else {
            return;
        };
        if clicks >= 2 && p.icon.contains(x, y) {
            self.close_child(id);
            cx.notify();
        }
    }

    /// The client-area element of each child.
    fn child_contents(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<(u32, AnyElement)> {
        let kinds = self.kinds.clone();
        let mut out = Vec::with_capacity(kinds.len());
        for (id, kind) in kinds {
            let el = match kind {
                ChildKind::Signal(node, k) => self.signal_body(id, node, k, window, cx),
                ChildKind::Text => self.text_body(id, cx),
                ChildKind::Tool(t) => self.tool_body(id, t, window, cx),
                #[cfg(debug_assertions)]
                ChildKind::Gallery => gallery::view(self, window, cx, &format!("mdi{id}")),
            };
            out.push((id, el));
        }
        out
    }

    /// Icon and caption buttons of a maximized child in the menu bar.
    fn maximized_controls(&self, cx: &mut Context<'_, Self>, vw: i32) -> Option<AnyElement> {
        // With the menu in the macOS menu bar a maximized child keeps its
        // own title bar and buttons.
        if crate::win95::style::native_menu() {
            return None;
        }
        let c = self.mdi.maximized()?;
        let (id, s) = (c.id, scheme(cx));
        let held = self
            .kit
            .pressed
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        let paint = canvas(
            crate::win95::widget::hover_hitbox,
            move |b, hitbox, window, cx| {
                let (w, _) = size_of(b);
                let mac = mdi_buttons_left();
                let motif = crate::win95::skin::linux_look()
                    .filter(|l| l.desk == crate::win95::skin_linux::Desk::Motif);
                if let Some(l) = motif {
                    // Motif's window-menu button, as in its title bar.
                    let icon = maximized_icon();
                    let side = icon.w - 2;
                    let at = Rect::new(1, (icon.h - side) / 2, side, side);
                    crate::win95::skin_linux::caption_icon(window, b.origin, at, l);
                } else if !mac {
                    // macOS windows have no icon.
                    crate::brand::paint(window, b.origin, 0, 1, 16);
                }
                let btns = buttons_at(w);
                let (min, max, close) = btns;
                // macOS: all the lights show their glyphs while the mouse is
                // over any of them.
                let cluster_hot = mac
                    && crate::win95::widget::hot(
                        window,
                        &hitbox,
                        b.origin,
                        mdi::light_cluster(btns, true),
                    );
                for (r, kind, n) in [
                    (min, CapButton::Minimize, "min"),
                    (max, CapButton::Restore, "max"),
                    (close, CapButton::Close, "close"),
                ] {
                    let st = crate::win95::skin::State {
                        hot: cluster_hot || crate::win95::widget::hot(window, &hitbox, b.origin, r),
                        down: held == format!("maxbar.{n}"),
                        focus: false,
                        enabled: true,
                    };
                    caption_button_of(window, cx, b.origin, r, kind, st, true, false, &s);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let slot = crate::win95::widget::Slot::default();
        let s2 = slot.clone();
        let measure = canvas(move |b, _, _| s2.set(b), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let (min, max, close) = buttons_at(vw);
        let tips = caption_tips(
            "maxbar",
            &[
                (min, CapButton::Minimize, "min"),
                (max, CapButton::Restore, "max"),
                (close, CapButton::Close, "close"),
            ],
        );
        let el = place(Rect::new(0, 0, vw, bar_h()))
            .child(paint)
            .child(measure)
            .children(tips)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.maximized_bar_click(id, &slot, ev);
                    cx.notify();
                }),
            );
        Some(el.into_any_element())
    }

    fn maximized_bar_click(
        &mut self,
        id: u32,
        slot: &crate::win95::widget::Slot,
        ev: &MouseDownEvent,
    ) {
        let (Some((x, y)), Some(b)) = (slot.local(ev.position), slot.bounds()) else {
            return;
        };
        let (w, _) = size_of(b);
        let icon = maximized_icon();
        if icon.contains(x, y) && !mdi_buttons_left() {
            // A double click closes, as on the icon in the title bar (the
            // second click usually lands on the system menu's click catcher,
            // which closes the same way).
            if ev.click_count >= 2 {
                self.popup = None;
                self.popup_track.close();
                self.close_child(id);
            } else {
                self.open_system_menu(id, (0, bar_h()), icon);
            }
            return;
        }
        let (min, max, close) = buttons_at(w);
        for (r, hit, n) in [
            (min, Hit::Minimize, "min"),
            (max, Hit::Maximize, "max"),
            (close, Hit::Close, "close"),
        ] {
            if r.contains(x, y) {
                let slot = slot.clone();
                self.kit.press(
                    &format!("maxbar.{n}"),
                    Some(crate::win95::kit::Capture {
                        on_move: None,
                        on_up: Some(Rc::new(move |v: &mut Self, p, _, _| {
                            if slot.local(p).is_some_and(|(x, y)| r.contains(x, y)) {
                                if hit == Hit::Close {
                                    v.close_child(id);
                                } else {
                                    v.mdi.command(id, hit);
                                }
                            }
                        })),
                    }),
                );
            }
        }
    }

    /// Something needs the poll loop: jobs, a preview being built, waiting
    /// actions.
    fn needs_poll(&self) -> bool {
        !self.jobs.is_empty()
            || !self.pending.is_empty()
            || self.sono_busy()
            || self.wf_busy()
            || self.phase_busy()
            || self.demod_busy()
            || self.ofdm_busy()
            || self.tools_busy()
            || self.loader.as_ref().is_some_and(Loader::preview_pending)
    }

    /// Start checking the jobs every [`POLL`] while there is work.
    fn ensure_poll(&mut self, cx: &mut Context<'_, Self>) {
        if self.polling || !self.needs_poll() {
            return;
        }
        self.polling = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                let go = this.update(cx, |v, cx| {
                    v.poll(cx);
                    cx.notify();
                    let go = v.needs_poll();
                    v.polling = go;
                    go
                });
                if !matches!(go, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    /// One poll step: finished jobs, the loader's preview, waiting actions.
    pub(crate) fn poll(&mut self, cx: &mut Context<'_, Self>) {
        self.tick = self.tick.wrapping_add(1);
        self.collect_finished_jobs();
        self.wf_finish_counts();
        self.recheck_confirm(cx);
        if let Some(l) = self.loader.as_mut() {
            l.poll_preview();
        }
        for c in std::mem::take(&mut self.pending) {
            self.do_confirmed(c, cx);
        }
        self.tools_poll(cx);
    }

    /// The frame was asked to close (its close button): ask first with unsaved
    /// work, as File > Exit does. Returns whether it may close now.
    pub(crate) fn should_close(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.must_ask(Confirm::Quit) {
            self.confirm(Confirm::Quit);
            cx.notify();
            return false;
        }
        self.quit_now(cx)
    }
}

#[cfg(debug_assertions)]
impl Frame {
    /// Debug builds with `SA_GUI_LOG_MOUSE`: one line about a mouse press, for
    /// finding what takes the clicks on a real window.
    fn mouse_log_line(
        &self,
        phase: gpui::DispatchPhase,
        ev: &MouseDownEvent,
        window: &Window,
        cx: &gpui::App,
    ) -> String {
        let (x, y) = (f32::from(ev.position.x), f32::from(ev.position.y));
        let vp = window.viewport_size();
        format!(
            "down {:?} {phase:?} at ({x:.0}, {y:.0}) count {} first_mouse {} | viewport {:.0}x{:.0} scale {} active {} | loader {:?} prompt {} msgbox {} menu {} popup {} capturing {} drag {} focus {:?}",
            ev.button,
            ev.click_count,
            ev.first_mouse,
            f32::from(vp.width),
            f32::from(vp.height),
            window.scale_factor(),
            window.is_window_active(),
            self.loader.as_ref().map(|l| l.rect),
            self.loader.as_ref().is_some_and(|l| l.prompt.is_some()),
            self.msgbox.is_some(),
            self.track.is_active(),
            self.popup.is_some(),
            self.kit.is_capturing(),
            cx.has_active_drag(),
            self.kit.focus,
        )
    }
}

/// Debug builds: the look named by `SA_GUI_STYLE` for window captures, as
/// `<style>[-<variant>]...`: style `95`, `98`, `xp`, `vista`, `7`, `8`, `10`;
/// variants `olive`, `silver`, `basic`, `dark` and an accent `#RRGGBB`.
/// `None` when any part is not one of these.
#[cfg(debug_assertions)]
fn debug_look(v: &str, base: sn_ui_core::winstyle::Look) -> Option<sn_ui_core::winstyle::Look> {
    use sn_ui_core::winstyle::{WinStyle, XpColor};
    let mut parts = v.split('-');
    let mut look = base;
    look.style = match parts.next()? {
        "95" => WinStyle::Win95,
        "98" => WinStyle::Win98,
        "xp" => WinStyle::Xp,
        "vista" => WinStyle::Vista,
        "7" => WinStyle::Win7,
        "8" => WinStyle::Win8,
        "10" => WinStyle::Win10,
        "11" => WinStyle::Win11,
        "aqua" => WinStyle::MacAqua,
        "yosemite" => WinStyle::MacYosemite,
        "mac" => WinStyle::MacModern,
        "gnome" => WinStyle::LinuxAdwaita,
        "kde" => WinStyle::LinuxBreeze,
        "gnome2" => WinStyle::LinuxClearlooks,
        "motif" => WinStyle::LinuxMotif,
        _ => return None,
    };
    // The accent goes where the finished look reads it (`Look::set_accent`
    // depends on `yaru`), so it is applied after every other part.
    let mut accent = None;
    for p in parts {
        match p {
            "olive" => look.xp = XpColor::Olive,
            "silver" => look.xp = XpColor::Silver,
            "basic" => look.aero = false,
            "dark" => look.dark = true,
            "graphite" => look.aqua = sn_ui_core::winstyle::AquaColor::Graphite,
            "yaru" => look.yaru = true,
            hex => {
                let rgb = hex
                    .strip_prefix('#')
                    .filter(|h| h.len() == 6)
                    .and_then(|h| u32::from_str_radix(h, 16).ok())?;
                accent = Some(rgb);
            }
        }
    }
    if let Some(rgb) = accent {
        look.set_accent(rgb);
    }
    Some(look)
}

/// An invisible layer that follows the pointer over the whole frame (also over
/// the child windows, which take the mouse events themselves), for the status
/// bar.
fn pointer_layer(cx: &mut Context<'_, Frame>) -> impl IntoElement {
    let view = cx.entity().downgrade();
    canvas(
        |_, _, _| (),
        move |_, (), window, _| {
            let v = view.clone();
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != gpui::DispatchPhase::Capture {
                    return;
                }
                let at = (
                    f32::from(ev.position.x) as i32,
                    f32::from(ev.position.y) as i32,
                );
                v.update(cx, |this, cx| {
                    if this.pointer != Some(at) {
                        this.pointer = Some(at);
                        cx.notify();
                    }
                })
                .ok();
            });
            #[cfg(debug_assertions)]
            if std::env::var_os("SA_GUI_LOG_MOUSE").is_some() {
                let v = view.clone();
                window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                    v.update(cx, |this, cx| {
                        let line = this.mouse_log_line(phase, ev, window, cx);
                        eprintln!("sn-gui mouse: {line}");
                    })
                    .ok();
                });
                let v = view.clone();
                window.on_mouse_event(move |ev: &gpui::MouseUpEvent, phase, _, cx| {
                    if phase != gpui::DispatchPhase::Bubble {
                        return;
                    }
                    let (x, y) = (f32::from(ev.position.x), f32::from(ev.position.y));
                    v.update(cx, |this, _| {
                        eprintln!(
                            "sn-gui mouse: up {:?} at ({x:.0}, {y:.0}) capturing {}",
                            ev.button,
                            this.kit.is_capturing()
                        );
                    })
                    .ok();
                });
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// Whether the caption buttons sit at the left (the macOS looks).
fn mdi_buttons_left() -> bool {
    crate::win95::style::metrics().btn_left
}

/// Where the menu titles start while a child is maximized: after the
/// window icon, or (macOS) after the traffic lights.
fn maximized_bar_x0() -> i32 {
    if mdi_buttons_left() {
        buttons_at(0).1.right() + crate::win95::style::metrics().btn_margin
    } else {
        maximized_icon().right()
    }
}

/// The window icon (Motif: the window-menu button) of a maximized child at
/// the left of the menu bar.
fn maximized_icon() -> Rect {
    Rect::new(0, 0, 20, bar_h())
}

/// Caption buttons of a maximized child in a menu bar `w` wide: the
/// style's minimize and maximize size (the close button as wide as they
/// are), no taller than fits in the bar. At the right in Windows' order; in
/// the macOS looks at the left, close, minimize and zoom, spaced as in a
/// title bar. In Motif, which has no close button, `close` is empty.
fn buttons_at(w: i32) -> (Rect, Rect, Rect) {
    let m = crate::win95::style::metrics();
    let (bw, bh) = (mdi::btn_w(), mdi::btn_h().min(bar_h() - 2));
    let y = (bar_h() - bh) / 2;
    if m.btn_left {
        let close = Rect::new(m.btn_margin, y, bw, bh);
        let min = Rect::new(close.right() + m.btn_gaps.1, y, bw, bh);
        let max = Rect::new(min.right() + m.btn_gaps.0, y, bw, bh);
        return (min, max, close);
    }
    // Motif has no close button (its window menu closes), here as in the
    // title bar.
    let close = if m.close_w == 0 {
        Rect::new(w - 2, y, 0, bh)
    } else {
        Rect::new(w - 2 - bw, y, bw, bh)
    };
    let gap = if close.is_empty() { 0 } else { 2 };
    let max = Rect::new(close.x - gap - bw, y, bw, bh);
    let min = Rect::new(max.x - bw, y, bw, bh);
    (min, max, close)
}

impl Frame {
    /// A command chosen in the macOS menu bar.
    fn native_command(
        &mut self,
        a: &native_menu::MenuCommand,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.command(a.0, window, cx);
        cx.notify();
    }

    /// Keep the macOS menu bar in step: SA's menus there while a macOS look is
    /// drawn on a Mac (rebuilt only when an item changes), the application
    /// menu alone otherwise.
    fn sync_native_menus(&mut self, cx: &mut Context<'_, Self>) {
        if crate::win95::style::native_menu() {
            let sig = native_menu::signature(&self.menus);
            if self.native_menus.as_deref() != Some(sig.as_str()) {
                native_menu::install(cx, &self.menus);
                self.native_menus = Some(sig);
            }
            self.track.close();
        } else if self.native_menus.take().is_some() {
            native_menu::uninstall(cx);
        }
    }

    /// Before drawing a frame: the window title, the menus (also in the macOS
    /// menu bar) and the views of closed windows.
    fn prepare_frame(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let title = match self.mdi.maximized() {
            Some(c) => format!("{} - [{}]", frame_title(), c.title),
            None => frame_title(),
        };
        window.set_window_title(&title);
        self.menus = Rc::new(menus::main_menu(&self.menu_state()));
        self.sync_native_menus(cx);
        // A macOS look gives the native title bar its light or dark look.
        let a = crate::win95::style::active();
        let wanted = match (
            a.style.is_mac(),
            a.look.dark && a.style != sn_ui_core::winstyle::WinStyle::MacAqua,
        ) {
            (false, _) => mac_window::TitleAppearance::System,
            (true, false) => mac_window::TitleAppearance::Light,
            (true, true) => mac_window::TitleAppearance::Dark,
        };
        if self.window_appearance != Some(wanted) {
            mac_window::set_appearance(window, wanted);
            self.window_appearance = Some(wanted);
        }
        self.sono_prune(window);
        self.wf_prune(window);
        self.phase_prune(window);
        self.harmonics_prune();
        self.demod_prune();
        self.ofdm_prune();
    }
}

impl Render for Frame {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.kit.begin_frame();
        self.tools.keys.clear();
        self.tools_poll(cx);
        self.ensure_poll(cx);
        let s = scheme(cx);
        let vp = window.viewport_size();
        let (vw, vh) = (f32::from(vp.width) as i32, f32::from(vp.height) as i32);
        if !window.is_maximized() && !window.is_fullscreen() {
            self.last_size = Some((f32::from(vp.width), f32::from(vp.height)));
        }
        let client = Rect::new(
            0,
            bar_h(),
            vw,
            (vh - bar_h() - crate::win95::bars::STATUS_H).max(0),
        );
        self.mdi.set_client(client.w - 4, client.h - 4);
        self.prepare_frame(window, cx);
        let contents = self.child_contents(window, cx);
        let max_controls = self.maximized_controls(cx, vw);
        let x0 = if max_controls.is_some() {
            maximized_bar_x0()
        } else {
            0
        };
        let mut root = div()
            .id("frame")
            .relative()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(s.of(Role::Face))
            .font_family(crate::win95::text::font(false).family)
            .text_size(px(crate::win95::text::content_size()))
            .line_height(px(LINE as f32))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|v: &mut Self, ev: &KeyDownEvent, window, cx| {
                v.key_down(ev, window, cx);
            }))
            .on_modifiers_changed(
                cx.listener(|v: &mut Self, ev: &ModifiersChangedEvent, _, cx| v.modifiers(ev, cx)),
            )
            .on_action(cx.listener(Self::native_command))
            .child(capture_layer(cx))
            .child(pointer_layer(cx));
        if let Some(c) = max_controls {
            root = root.child(c);
        }
        let bar_hooks = Self::menu_hooks(false);
        let bar_w = if x0 > 0 && !mdi_buttons_left() {
            vw - 3 * mdi::btn_w() - 8
        } else {
            vw
        };
        if bar_h() > 0 {
            root = root.child(menu_bar(
                cx,
                window,
                Rect::new(x0, 0, bar_w - x0, bar_h()),
                0,
                &self.menus,
                &self.track,
                &bar_hooks,
            ));
        }
        let modal = self.msgbox.is_some()
            || self.loader.is_some()
            || self.params.is_some()
            || self.phase_load_open();
        let frame_active = !modal && window.is_window_active();
        root = root.child(mdi_client(
            &self.kit,
            cx,
            &self.mdi,
            client,
            contents,
            self.outline,
            frame_active,
            &Self::mdi_hooks(),
        ));
        for el in self.status_bar(cx, vw, vh) {
            root = root.child(el);
        }
        let root = self.overlays(root, window, cx, x0, vw, vh);
        if cx.has_active_drag() {
            // Files dragged from the desktop: the whole frame takes them, over
            // the child windows and dialogs (SA 6.2.6.3).
            let target = div()
                .id("drop")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .occlude()
                .on_drop(cx.listener(|v: &mut Self, paths: &ExternalPaths, _, cx| {
                    v.open_dropped(paths.paths());
                    cx.notify();
                }));
            return root.child(target);
        }
        root
    }
}

impl Frame {
    /// Menus, dialogs, popup menus and the message box, on top of everything.
    fn overlays(
        &mut self,
        mut root: gpui::Stateful<gpui::Div>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
        x0: i32,
        vw: i32,
        vh: i32,
    ) -> gpui::Stateful<gpui::Div> {
        if let Some(i) = self.track.bar {
            let measure = |t: &str| {
                crate::win95::text::width_in(crate::win95::text::Face::Menu, window, t, false)
            };
            let titles = crate::win95::menu_view::bar_layout(&self.menus, x0, &measure);
            let at = titles.get(i).map_or((x0, bar_h()), |t| (t.x, bar_h()));
            let menus = Menus::Bar(self.menus.clone());
            if let Some(p) = menu_popups(
                cx,
                window,
                &menus,
                &self.track,
                at,
                bar_h(),
                &Self::menu_hooks(false),
            ) {
                root = root.child(p);
            }
        }
        if let Some(els) = self.loader_view(window, cx, vw, vh) {
            for el in els {
                root = root.child(el);
            }
        }
        if let Some(el) = self.params_view(window, cx, vw, vh) {
            root = root.child(el);
        }
        if let Some(el) = self.phase_load_view(cx, vw, vh) {
            root = root.child(el);
        }
        if let Some(p) = self.popup.clone() {
            if !self.popup_track.is_open() {
                self.popup = None;
            } else if let Some(el) = menu_popups(
                cx,
                window,
                &Menus::Popup(p.items),
                &self.popup_track,
                p.at,
                0,
                &Self::menu_hooks(true),
            ) {
                root = root.child(el);
            }
        }
        if self.msgbox_new
            && let Some(mb) = self.msgbox.as_mut()
        {
            mb.place(window, vw, vh);
            self.msgbox_new = false;
            let first = mb.buttons.answers().first().copied();
            self.kit.focus = first.map(|a| MsgBox::button_id(a).into());
        }
        if let Some(mb) = self.msgbox.clone() {
            let on_answer = Rc::new(
                |v: &mut Self, a, _: &mut Window, cx: &mut Context<'_, Self>| {
                    v.answer(a, cx);
                    cx.notify();
                },
            );
            let l = lens(|v: &mut Self| v.msgbox.as_mut());
            root = root.child(message_box(&mut self.kit, cx, window, &mb, l, on_answer));
        }
        root
    }
}
