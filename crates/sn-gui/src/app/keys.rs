//! Keyboard handling of the frame: menus (Alt, F10, Alt+letter, arrows),
//! message boxes and dialogs, MDI shortcuts (Ctrl+F4, Ctrl+F6), Tab between
//! controls, and SigNexora's shortcuts (`sn-ui-core::shortcuts`) that work without
//! the analysis windows.

use gpui::{Context, KeyDownEvent, Keystroke, ModifiersChangedEvent, Window};
use sn_ui_core::shortcuts::Command;
use sn_ui_core::t;

use super::Frame;
use super::model::WinKind;
use super::sessions::Chooser;
use super::tools::Tool;
use crate::win95::dialog::MSG_SCOPE;
use crate::win95::kit::dispatch_key;
use crate::win95::menu::{Key, Outcome, Source};

/// A menu navigation key for a GPUI key name. Letters come from the physical
/// key, not the typed character: on macOS Option+W types a symbol.
pub(crate) fn menu_key(key: &str, alt: bool) -> Option<Key> {
    Some(match key {
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "enter" => Key::Enter,
        "escape" => Key::Escape,
        "f10" => Key::Alt,
        _ => {
            let mut it = key.chars();
            let (Some(c), None) = (it.next(), it.next()) else {
                return None;
            };
            if !c.is_alphanumeric() {
                return None;
            }
            if alt { Key::AltChar(c) } else { Key::Char(c) }
        }
    })
}

/// The command of a key press (`sa-app`'s bindings: Cmd on macOS, Ctrl
/// elsewhere; both work here).
pub(crate) fn command_for(k: &Keystroke) -> Option<Command> {
    let m = k.modifiers;
    let cmd = (m.platform || m.control) && !m.alt;
    Some(match (k.key.as_str(), cmd, m.shift, m.alt) {
        ("s", true, true, _) => Command::SaveSessionAs,
        ("n", true, true, _) => Command::SignalGenerator,
        ("s", true, false, _) => Command::SaveSession,
        ("o", true, false, _) => Command::OpenRecording,
        ("w", true, false, _) => Command::CloseTab,
        ("tab", _, true, false) if m.control => Command::PrevTab,
        ("tab", _, false, false) if m.control => Command::NextTab,
        ("down", false, false, true) => Command::NextSignal,
        ("up", false, false, true) => Command::PrevSignal,
        ("f1", false, false, false) => Command::Guide,
        // Slash needs Shift on some keyboard layouts.
        ("/", true, _, _) => Command::ShortcutsWindow,
        // The plot keys of the sonogram window (`sa-app`'s extras).
        ("z", true, false, false) => Command::RestoreMarkers,
        ("space", false, false, false) => Command::Play,
        ("z", false, false, false) => Command::ZoomToSelection,
        ("backspace", false, false, false) => Command::ZoomBack,
        ("f", false, false, false) => Command::FullView,
        ("b", false, false, false) => Command::MeasureBr,
        ("c", false, false, false) => Command::MeasureSc,
        (d, true, false, _) if d.len() == 1 => {
            let n = d.chars().next()?.to_digit(10)?;
            Command::OpenView(usize::try_from(n).ok()?.checked_sub(1)?)
        }
        _ => return None,
    })
}

impl Frame {
    /// Carry out a keyboard command.
    pub(crate) fn run_command(&mut self, c: Command, cx: &mut Context<'_, Self>) {
        match c {
            Command::OpenRecording => self.open_loader(None),
            Command::SaveSession => {
                if self.has_signals() {
                    self.save_session(cx);
                }
            }
            Command::SaveSessionAs => {
                if self.has_signals() && !self.saving_session() {
                    self.choose(Chooser::SaveSessionAs, cx);
                }
            }
            Command::CloseTab => {
                if let Some(id) = self.mdi.active() {
                    self.close_child(id);
                }
            }
            Command::NextTab | Command::PrevTab => self.mdi.next(c == Command::PrevTab),
            Command::NextSignal | Command::PrevSignal => {
                self.step_signal(c == Command::NextSignal);
            }
            Command::OpenView(i) => self.open_view(i),
            Command::Guide => self.toggle_tool(Tool::Guide),
            Command::SignalGenerator => self.toggle_tool(Tool::Generator),
            Command::ShortcutsWindow => self.toggle_tool(Tool::Help),
            Command::RestoreMarkers
            | Command::Play
            | Command::ZoomToSelection
            | Command::ZoomBack
            | Command::FullView
            | Command::MeasureBr
            | Command::MeasureSc => {
                self.sono_key_command(c);
                self.wf_key_command(c);
            }
        }
    }

    /// Cmd+1 and on: open the active window's signal in view `i`.
    fn open_view(&mut self, i: usize) {
        match (self.active_node(), WinKind::ALL.get(i)) {
            (Some(n), Some(kind)) => {
                let caller = self.mdi.active();
                self.open_window(n, *kind, caller);
            }
            _ if !self.has_signals() => {
                self.status = t!("app-no-signals-hint", shortcut = "Ctrl+O");
            }
            _ => {
                self.status = t!(
                    "app-select-signal-first",
                    next = "Alt+Down",
                    prev = "Alt+Up"
                );
            }
        }
    }

    /// A modal box or dialog is open (the message box, the loader, the
    /// parameter dialog, PhasePlane's LoadFile): the menu bar must not open.
    pub(crate) fn modal_open(&self) -> bool {
        self.msgbox.is_some()
            || self.loader.is_some()
            || self.params.is_some()
            || self.phase_load_open()
    }

    /// The window turned active or inactive. Leaving it (Alt+Tab, a click
    /// in another window) sends the Alt release elsewhere, so forget that
    /// Alt is held: GNOME and KDE menus would keep their underlines, and the
    /// next modifier change would take it for Alt tapped alone.
    pub(super) fn window_activation(&mut self, active: bool) {
        if !active {
            crate::win95::menu_view::set_alt_held(false);
            self.alt_alone = false;
        }
    }

    pub(super) fn modifiers(&mut self, ev: &ModifiersChangedEvent, cx: &mut Context<'_, Self>) {
        let m = ev.modifiers;
        if crate::win95::menu_view::set_alt_held(m.alt) {
            // GNOME and KDE show the menus' mnemonic underlines.
            cx.notify();
        }
        let only_alt = m.alt && !m.control && !m.shift && !m.platform;
        if only_alt {
            self.alt_alone = true;
        } else if !m.alt && self.alt_alone {
            self.alt_alone = false;
            // With the menu in the macOS menu bar, Alt opens no menu.
            if !self.modal_open() && !crate::win95::style::native_menu() {
                if self.popup.is_some() {
                    self.popup = None;
                    self.popup_track.close();
                }
                let menus = self.menus.clone();
                self.track.key(Source::Bar(&menus), Key::Alt);
                cx.notify();
            }
        } else {
            self.alt_alone = false;
        }
    }

    pub(super) fn key_down(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.alt_alone = false;
        let k = &ev.keystroke;
        let m = k.modifiers;
        let ctrl = m.control || m.platform;
        if self.msgbox.is_some() {
            self.msgbox_key(ev, window, cx);
            cx.notify();
            return;
        }
        if self.popup.is_none() && !self.track.is_active() {
            if self.loader.is_some() {
                self.loader_key(ev, window, cx);
                cx.notify();
                return;
            }
            if self.params.is_some() {
                self.params_key(ev, window, cx);
                cx.notify();
                return;
            }
            if self.phase_load_open() {
                self.phase_load_key(ev);
                cx.notify();
                return;
            }
        }
        let key = menu_key(&k.key, m.alt);
        if self.popup.is_some() {
            if let (Some(key), Some(p)) = (key, self.popup.clone())
                && let Outcome::Invoke(id) = self.popup_track.key(Source::Popup(&p.items), key)
            {
                self.command(id, window, cx);
            }
            cx.notify();
            return;
        }
        let menus = self.menus.clone();
        if let Some(key) = key
            && !crate::win95::style::native_menu()
            && (self.track.is_active() || matches!(key, Key::AltChar(_) | Key::Alt))
        {
            match self.track.key(Source::Bar(&menus), key) {
                Outcome::Invoke(id) => self.command(id, window, cx),
                Outcome::Handled => {}
                Outcome::Ignored => self.other_key(ev, ctrl, window, cx),
            }
            cx.notify();
            return;
        }
        self.other_key(ev, ctrl, window, cx);
        cx.notify();
    }

    /// Keys that are not for the menus.
    fn other_key(
        &mut self,
        ev: &KeyDownEvent,
        ctrl: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let k = &ev.keystroke;
        let active = self.mdi.active();
        // Keys a focused text field uses itself (Alt+Up moves its caret).
        let typing = self
            .kit
            .focus
            .as_ref()
            .is_some_and(|f| !f.starts_with("msgbox"));
        if let Some(c) = command_for(k)
            && !(typing && c.needs_free_keys())
        {
            self.run_command(c, cx);
            return;
        }
        match (k.key.as_str(), ctrl) {
            ("f4", true) => {
                if let Some(id) = active {
                    self.close_child(id);
                }
            }
            ("f6", true) => self.mdi.next(k.modifiers.shift),
            ("tab", false) => {
                let scope = self
                    .kit
                    .focus_scope()
                    .map(|s| s.to_string())
                    .or_else(|| active.map(|id| format!("mdi{id}")));
                if let Some(scope) = scope {
                    self.kit.tab(&scope, k.modifiers.shift);
                }
            }
            (key @ ("enter" | "escape"), _) => {
                // One dispatch: a combo that closes its list with Escape must
                // not also make the window drop the focus or close.
                if dispatch_key(self, ev, window, cx) {
                    return;
                }
                if self.kit.popup.is_some() {
                    if key == "escape" {
                        self.kit.popup = None;
                    }
                } else if !ctrl && !self.tool_key(key) && key == "escape" {
                    // Escape leaves the focused control: the single keys act
                    // on the window again.
                    self.kit.focus = None;
                }
            }
            (c, false) if k.modifiers.alt && c.chars().count() == 1 => {
                if let Some(ch) = c.chars().next() {
                    self.tool_mnemonic(ch, cx);
                }
            }
            _ => {
                if !dispatch_key(self, ev, window, cx)
                    && !self.ofdm_arrow_key(k)
                    && !self.sono_arrow_key(k)
                {
                    self.wf_arrow_key(k);
                }
            }
        }
    }

    /// Keys while a message box is open: Tab between buttons, Enter, Escape.
    fn msgbox_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<'_, Self>) {
        let k = &ev.keystroke;
        let Some(buttons) = self.msgbox.as_ref().map(|m| m.buttons) else {
            return;
        };
        match k.key.as_str() {
            "tab" | "left" | "right" => {
                let back = k.modifiers.shift || k.key == "left";
                self.kit.tab(MSG_SCOPE, back);
            }
            "escape" => {
                if let Some(a) = buttons.escape() {
                    self.answer(a, cx);
                }
            }
            _ => {
                let used = dispatch_key(self, ev, window, cx);
                if !used
                    && k.key == "enter"
                    && let Some(a) = buttons.answers().first()
                {
                    self.answer(*a, cx);
                    return;
                }
                let letter = k.key.chars().next().filter(|_| k.key.chars().count() == 1);
                let labels: Vec<_> = buttons
                    .answers()
                    .iter()
                    .map(|a| {
                        (
                            *a,
                            self.msgbox
                                .as_ref()
                                .map(|m| m.label_of(*a))
                                .unwrap_or_default(),
                        )
                    })
                    .collect();
                if let Some((a, _)) = letter.and_then(|c| {
                    labels
                        .into_iter()
                        .find(|(_, l)| crate::win95::text::mnemonic(l).key == Some(c))
                }) {
                    self.answer(a, cx);
                }
            }
        }
    }
}

impl Frame {
    /// Keys of the parameter dialog: Tab between its controls, Enter is `Ok`,
    /// Escape is `Cancel`.
    fn params_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<'_, Self>) {
        match ev.keystroke.key.as_str() {
            "tab" => self
                .kit
                .tab(super::dialogs::PARAMS_SCOPE, ev.keystroke.modifiers.shift),
            "escape" if self.kit.popup.is_some() => {
                if !dispatch_key(self, ev, window, cx) {
                    self.kit.popup = None;
                }
            }
            "escape" => self.params_cancel(),
            "enter" if self.kit.popup.is_some() => {
                dispatch_key(self, ev, window, cx);
            }
            "enter" if self.kit.has_focus("pd.cancel") => self.params_cancel(),
            "enter" => self.params_ok(),
            _ => {
                dispatch_key(self, ev, window, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_menu_keys() {
        assert_eq!(menu_key("down", false), Some(Key::Down));
        assert_eq!(menu_key("f10", false), Some(Key::Alt));
        // Option+F, Option+W and Option+H type symbols on macOS; the
        // physical key opens the menu.
        assert_eq!(menu_key("f", true), Some(Key::AltChar('f')));
        assert_eq!(menu_key("w", true), Some(Key::AltChar('w')));
        assert_eq!(menu_key("h", true), Some(Key::AltChar('h')));
        assert_eq!(menu_key("x", false), Some(Key::Char('x')));
        assert_eq!(menu_key("tab", false), None);
        assert_eq!(menu_key("-", false), None);
    }

    fn ks(s: &str) -> Keystroke {
        Keystroke::parse(s).unwrap()
    }

    #[test]
    fn shortcuts_follow_sa_app() {
        assert_eq!(command_for(&ks("cmd-o")), Some(Command::OpenRecording));
        assert_eq!(command_for(&ks("ctrl-o")), Some(Command::OpenRecording));
        assert_eq!(command_for(&ks("cmd-s")), Some(Command::SaveSession));
        assert_eq!(command_for(&ks("space")), Some(Command::Play));
        assert_eq!(command_for(&ks("z")), Some(Command::ZoomToSelection));
        assert_eq!(command_for(&ks("cmd-z")), Some(Command::RestoreMarkers));
        assert_eq!(command_for(&ks("backspace")), Some(Command::ZoomBack));
        assert_eq!(command_for(&ks("shift-f")), None);
        assert_eq!(
            command_for(&ks("cmd-shift-s")),
            Some(Command::SaveSessionAs)
        );
        assert_eq!(command_for(&ks("cmd-w")), Some(Command::CloseTab));
        assert_eq!(command_for(&ks("ctrl-tab")), Some(Command::NextTab));
        assert_eq!(command_for(&ks("ctrl-shift-tab")), Some(Command::PrevTab));
        assert_eq!(command_for(&ks("alt-down")), Some(Command::NextSignal));
        assert_eq!(command_for(&ks("alt-up")), Some(Command::PrevSignal));
        assert_eq!(command_for(&ks("cmd-2")), Some(Command::OpenView(1)));
        assert_eq!(command_for(&ks("cmd-0")), None);
        assert_eq!(command_for(&ks("o")), None);
        assert_eq!(
            command_for(&ks("alt-o")),
            None,
            "Alt+O opens the Options menu"
        );
    }
}
