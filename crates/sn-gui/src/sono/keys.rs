//! `sa-app`'s invisible keyboard extras on the active sonogram window (SA
//! documents no keys here): Space plays or stops, Z zooms to the selection,
//! Backspace zooms back, F shows everything, B and C measure Br and Sc,
//! Cmd+Z restores the markers before the last sweep, and the arrow keys move
//! the marker moved last (Shift: 10 pixels).

use gpui::Keystroke;
use sn_core::{MeasurementKind, NodeId};
use sn_ui_core::shortcuts::Command;
use sn_ui_core::sono_tools::ViewAction;
use sn_ui_core::t;

use super::Frame;
use crate::app::model::WinKind;

impl Frame {
    /// The active window if it is a sonogram window, with its signal.
    fn active_sono(&self) -> Option<(u32, NodeId)> {
        let win = self.mdi.active()?;
        match self.binding(win)? {
            (node, WinKind::Sonogram) => Some((win, node)),
            _ => None,
        }
    }

    /// A plot command of the keyboard on the active sonogram window.
    pub(crate) fn sono_key_command(&mut self, c: Command) {
        let Some((win, node)) = self.active_sono() else {
            return;
        };
        let action = match c {
            Command::RestoreMarkers => {
                self.restore_markers(win, node);
                None
            }
            Command::Play => {
                self.toggle_play(win, node);
                None
            }
            Command::ZoomToSelection => Some(ViewAction::ZoomToSelection),
            Command::ZoomBack => Some(ViewAction::Back),
            Command::FullView => Some(ViewAction::Full),
            Command::MeasureBr | Command::MeasureSc => {
                let h_set = self
                    .ui
                    .get(&node)
                    .is_some_and(|u| u.markers.freq_range().is_some());
                if !h_set {
                    self.status = t!("app-set-h-markers-first");
                } else if c == Command::MeasureBr {
                    self.start_measure(node, MeasurementKind::BaudRate);
                } else {
                    self.start_measure(node, MeasurementKind::Carrier);
                }
                None
            }
            _ => None,
        };
        if let (Some(a), Some(sw)) = (action, self.sono.wins.get_mut(&win)) {
            sw.action = Some(a);
        }
    }

    /// Cmd+Z: swap the markers with those before the last sweep or collapse.
    fn restore_markers(&mut self, win: u32, node: NodeId) {
        let name = self.name_of(node);
        let Some(prev) = self.sono.wins.get_mut(&win).and_then(|w| w.undo.take()) else {
            return;
        };
        if let Some(u) = self.ui.get_mut(&node) {
            let now = std::mem::replace(&mut u.markers, prev);
            if let Some(sw) = self.sono.wins.get_mut(&win) {
                sw.undo = Some(now);
            }
            self.mark_dirty();
            self.status = t!("app-markers-restored", node = name);
        }
    }

    /// The arrow keys move the marker moved last (Shift: 10 pixels). Returns
    /// whether the key was used.
    pub(crate) fn sono_arrow_key(&mut self, k: &Keystroke) -> bool {
        let Some((win, node)) = self.active_sono() else {
            return false;
        };
        let m = k.modifiers;
        if m.control || m.platform || m.alt {
            return false;
        }
        let (dx, dy) = match k.key.as_str() {
            "left" => (-1.0, 0.0),
            "right" => (1.0, 0.0),
            "up" => (0.0, 1.0),
            "down" => (0.0, -1.0),
            _ => return false,
        };
        let step = if m.shift { 10.0 } else { 1.0 };
        let Some((view, h)) = self
            .sono
            .wins
            .get(&win)
            .and_then(|w| w.view.map(|v| (v, f64::from(w.plot.h))))
        else {
            return false;
        };
        let Some(u) = self.ui.get_mut(&node) else {
            return false;
        };
        let Some(id) = u.markers.active else {
            return false;
        };
        let Some(pos) = u.markers.get(id) else {
            return false;
        };
        let moved = if id.is_vertical() && dx != 0.0 {
            u.markers.set(id, pos + dx * step * view.samples_per_px)
        } else if !id.is_vertical() && dy != 0.0 {
            let hz_per_px = (view.f_hi - view.f_lo) / h.max(1.0);
            u.markers.set(id, pos + dy * step * hz_per_px)
        } else {
            false
        };
        if moved {
            self.mark_dirty();
        }
        moved
    }
}
