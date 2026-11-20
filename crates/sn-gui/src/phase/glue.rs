//! The PhasePlane window's link to the session, as `sa-app`'s `app_phase.rs`:
//! `LoadFile`, the ISD corrector's result as a signal of the session (tag
//! `(isd1)`, no window, as in SA), `SaveAs`, `Reset`, the signals the module
//! keeps alive, closing the window, and its popup menu commands.

use gpui::Context;
use sn_core::NodeId;
use sn_core::job::JobError;
use sn_dsp::ops::Edit;
use sn_ui_core::edits::SaveFormat;
use sn_ui_core::phase::{IsdDone, Mode};
use sn_ui_core::t;

use super::corrector::copy_text;
use crate::app::Frame;
use crate::app::menus::cmd;
use crate::app::model::WinKind;

impl Frame {
    /// `LoadFile` picked `node` (`title`) for window `win`: the module
    /// switches to its phase plane mode. A corrected signal of the previous
    /// file has no window: it leaves the session.
    pub(crate) fn phase_load_file(&mut self, win: u32, node: NodeId, title: String) {
        let Some(st) = self.phase_st(win) else {
            return;
        };
        let old = st.corrected;
        let old_loaded = st.loaded;
        st.load(node, title);
        st.error = None;
        if let Some(old) = old {
            self.discard_unused(old);
        }
        // The file loaded before may have stayed only for this module (its
        // window closed while it was loaded): nothing needs it now.
        if let Some(prev) = old_loaded.filter(|p| *p != node) {
            self.discard_unused(prev);
        }
    }

    /// `Auto`: one step of the corrector on the signal shown (the corrected
    /// one when the corrector is on, so steps add up).
    pub(crate) fn phase_auto(&mut self, win: u32) {
        let src = self.phase_st(win).and_then(|st| st.source());
        let signal = src
            .filter(|n| !self.discarded.contains(n))
            .and_then(|n| self.graph.get(n))
            .map(|n| n.signal.clone());
        let Some(st) = self.phase_st(win) else {
            return;
        };
        // Say why: the source left the session, or the settings are not
        // usable. A started step clears an earlier message.
        st.error = match (src, signal) {
            (Some(_), None) => Some(t!("pp-source-gone")),
            (_, Some(s)) => st.start_isd(&s).is_none().then(|| t!("pp-check-settings")),
            (None, None) => Some(t!("pp-check-settings")),
        };
    }

    /// `Reset`: stop a running step and switch the corrector off; the
    /// corrected signal leaves the session.
    pub(crate) fn phase_reset(&mut self, win: u32) {
        let Some(st) = self.phase_st(win) else {
            return;
        };
        st.isd_job = None;
        st.isd_from = None;
        if let Some(old) = st.corrected.take() {
            self.discard_unused(old);
        }
    }

    /// `SaveAs`: save the corrected signal as a WAV file.
    pub(crate) fn phase_save_corrected(&mut self, win: u32, cx: &mut Context<'_, Self>) {
        if let Some(c) = self.phase_st(win).and_then(|st| st.corrected) {
            self.choose_save(c, SaveFormat::WavPcm16, cx);
        }
    }

    /// A finished Auto step of the PhasePlane of `node`.
    pub(crate) fn on_isd_done(&mut self, node: NodeId, isd: Option<IsdDone>) {
        match isd {
            Some((Ok(out), Some((parent, edit)))) => self.add_corrected(node, parent, &edit, out),
            Some((Err(JobError::Cancelled), _) | (Ok(_), None)) | None => {}
            Some((Err(e), _)) => self.alert(&e.to_string()),
        }
    }

    /// Add the ISD corrector's result to the session (no window, as in SA)
    /// and switch the PhasePlane of `node` to it (Corr-ON).
    pub(crate) fn add_corrected(
        &mut self,
        node: NodeId,
        parent: NodeId,
        edit: &Edit,
        out: sn_core::Signal,
    ) {
        if self.discarded.contains(&parent) || self.graph.get(parent).is_none() {
            self.alert(&t!("pp-source-gone"));
            return;
        }
        let Some(id) = self.add_edit_node(parent, edit, out) else {
            return;
        };
        let name = self.titles.derived(&self.name_of(parent), edit);
        self.ui.entry(id).or_default().name = name;
        let old = self
            .phase
            .states
            .get_mut(&node)
            .and_then(|st| st.corrected.replace(id));
        self.mark_dirty();
        // The previous corrected signal is replaced: drop it unless something
        // else still shows or needs it.
        if let Some(old) = old.filter(|o| *o != id) {
            self.discard_unused(old);
        }
    }

    /// A PhasePlane still reads `n` (loaded, or its corrected result).
    pub(crate) fn phase_needs(&self, n: NodeId) -> bool {
        self.phase
            .states
            .values()
            .any(|st| st.loaded == Some(n) || st.corrected == Some(n))
    }

    /// Window `win` closes: stop its work. The ISD corrector's result has no
    /// window of its own, and the loaded working file may have lost its
    /// window meanwhile: both are returned to leave the session unless
    /// something else needs them.
    pub(crate) fn phase_closed(&mut self, win: u32) -> Vec<NodeId> {
        if self.binding(win).map(|(_, k)| k) != Some(WinKind::Phase) {
            return Vec::new();
        }
        let Some(st) = self.phase_st(win) else {
            return Vec::new();
        };
        st.release();
        let mut out: Vec<NodeId> = st.corrected.take().into_iter().collect();
        out.extend(st.loaded.take());
        st.mode = Mode::Spectrum;
        self.phase.wins.remove(&win);
        out
    }

    /// The tool job closing window `win` would stop: the PhasePlane's run.
    pub(crate) fn phase_jobs(&self, win: u32) -> Vec<String> {
        match self.binding(win) {
            Some((node, WinKind::Phase))
                if self
                    .phase
                    .states
                    .get(&node)
                    .is_some_and(sn_ui_core::phase::PhaseState::is_running) =>
            {
                vec![WinKind::Phase.label()]
            }
            _ => Vec::new(),
        }
    }

    /// Forget the states of a replaced session (node ids start again).
    pub(crate) fn phase_session_replaced(&mut self) {
        self.phase.states.clear();
        self.phase.wins.clear();
        self.phase.load = None;
    }

    /// A command of a PhasePlane popup menu.
    pub(crate) fn phase_command(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(win) = self.phase.popup_win.take() else {
            return;
        };
        let Some(st) = self.phase_st(win) else {
            return;
        };
        match id {
            cmd::PP_COPY_TEXT => {
                let text = st
                    .panel_lines()
                    .into_iter()
                    .map(|(_, l)| l)
                    .collect::<Vec<_>>()
                    .join("\n");
                copy_text(cx, text);
            }
            cmd::PP_COPY_CODES => {
                let codes = st.all_codes();
                copy_text(cx, codes);
            }
            cmd::PP_EYE => st.eye = !st.eye,
            _ => {}
        }
    }
}
