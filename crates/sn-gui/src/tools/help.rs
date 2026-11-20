//! Help > Use help...: every keyboard shortcut in SigNexora's four sections, and
//! a button that opens the workflow guide (`sa-app`'s shortcuts window). The
//! keys are written the Windows way (Ctrl); on macOS Cmd works as well.

use gpui::{AnyElement, Context, Window};
use sn_ui_core::shortcuts::{Command, Section};
use sn_ui_core::t;

use super::form::{Builder, Tone, measurer};
use super::{Btn, Tool};
use crate::app::Frame;
use crate::app::model::WinKind;
use crate::win95::geom::Rect;

/// Every app shortcut as the menus write it, in `sa-app`'s order.
pub(crate) const BINDINGS: [(&str, Command); 24] = [
    ("Ctrl+Shift+S", Command::SaveSessionAs),
    ("Ctrl+Shift+N", Command::SignalGenerator),
    ("Ctrl+S", Command::SaveSession),
    ("Ctrl+O", Command::OpenRecording),
    ("Ctrl+W", Command::CloseTab),
    ("Ctrl+Shift+Tab", Command::PrevTab),
    ("Ctrl+Tab", Command::NextTab),
    ("Alt+Down", Command::NextSignal),
    ("Alt+Up", Command::PrevSignal),
    ("Ctrl+1", Command::OpenView(0)),
    ("Ctrl+2", Command::OpenView(1)),
    ("Ctrl+3", Command::OpenView(2)),
    ("Ctrl+4", Command::OpenView(3)),
    ("Ctrl+5", Command::OpenView(4)),
    ("Ctrl+6", Command::OpenView(5)),
    ("F1", Command::Guide),
    ("Ctrl+/", Command::ShortcutsWindow),
    ("Ctrl+Z", Command::RestoreMarkers),
    ("Space", Command::Play),
    ("Z", Command::ZoomToSelection),
    ("Backspace", Command::ZoomBack),
    ("F", Command::FullView),
    ("B", Command::MeasureBr),
    ("C", Command::MeasureSc),
];

/// Keys the plots read themselves, listed under the plot commands.
const PLOT_KEYS: [(&str, &str); 3] = [
    ("Left, Right", "keys-nudge-v"),
    ("Up, Down", "keys-nudge-h"),
    (
        "Shift+Left, Shift+Right, Shift+Up, Shift+Down",
        "keys-nudge-fast",
    ),
];

/// Windows' own keys.
const INTERFACE: [(&str, &str); 7] = [
    ("Tab, Shift+Tab", "keys-focus"),
    ("Enter", "keys-activate"),
    ("Escape", "keys-escape"),
    ("Alt, F10", "gui-keys-menu"),
    ("Alt+letter", "gui-keys-menu-letter"),
    ("Ctrl+F4", "keys-close-tab"),
    ("Ctrl+F6, Ctrl+Shift+F6", "gui-keys-next-window"),
];

/// The key of `c` as the menus write it (empty if it has none).
pub(crate) fn key_of(c: Command) -> String {
    BINDINGS
        .iter()
        .find(|(_, b)| *b == c)
        .map(|(k, _)| (*k).to_owned())
        .unwrap_or_default()
}

/// The rows (keys, what they do) of `section`.
pub(crate) fn rows(section: Section) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = BINDINGS
        .iter()
        .filter(|(_, c)| c.section() == section)
        .filter_map(|(k, c)| {
            let what = match c {
                Command::OpenView(i) => t!(c.key(), view = WinKind::ALL.get(*i)?.label()),
                _ => t!(c.key()),
            };
            Some(((*k).to_owned(), what))
        })
        .collect();
    let extra: &[(&str, &str)] = match section {
        Section::Plot => &PLOT_KEYS,
        Section::Interface => &INTERFACE,
        Section::General | Section::Views => &[],
    };
    out.extend(extra.iter().map(|(k, what)| ((*k).to_owned(), t!(what))));
    out
}

impl Frame {
    /// The shortcuts window's client area.
    pub(crate) fn help_body(
        &mut self,
        win: u32,
        cw: i32,
        ch: i32,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let measure = measurer(window);
        let mut b = Builder::new(&measure, cw - crate::win95::scrollbar::THICKNESS);
        b.line().button(
            Btn::OpenGuide,
            &t!("keys-open-guide", key = key_of(Command::Guide)),
        );
        b.wrapped(&t!("keys-intro"), Tone::Dim);
        for section in Section::ALL {
            b.group(&t!(section.key()));
            for (keys, what) in rows(section) {
                b.row(&keys).text(&what, Tone::Plain);
            }
        }
        let form = b.finish();
        self.form_view(Tool::Help, win, &form, Rect::new(0, 0, cw, ch), cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::Keystroke;

    use super::*;

    #[test]
    fn every_listed_key_runs_its_command() {
        for (label, c) in BINDINGS {
            let k = label
                .to_lowercase()
                .replace("ctrl+", "ctrl-")
                .replace("shift+", "shift-")
                .replace("alt+", "alt-");
            let ks = Keystroke::parse(&k).unwrap();
            assert_eq!(
                crate::app::keys::command_for(&ks),
                Some(c),
                "{label} does not run {c:?}"
            );
        }
        for s in Section::ALL {
            assert_ne!(rows(s).len(), 0);
        }
        assert_eq!(rows(Section::Views).len(), 6);
    }
}
