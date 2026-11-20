//! The session model of the frame, as in `sa-app`: the signal graph, the UI
//! state of every signal, the background jobs and what their results do.
//!
//! Like Signals Analyzer, almost every operation opens its result in a new MDI
//! child window, and closing a window is the undo (see `windows.rs`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;
use sn_core::job::{self, JobError, JobHandle};
use sn_core::{MeasurementContext, NodeId, Operation, SampleUnit, Signal, SignalGraph};
use sn_dsp::ops::Edit;
use sn_dsp::overview::Overview;
use sn_io::LoadOptions;
use sn_session::{ReplayOutcome, SessionFile};
use sn_ui_core::app_glue::{fit_range, keeps_amplitude};
use sn_ui_core::jobs::{dsp_to_job, io_to_job};
use sn_ui_core::lineage::{self, Carried};
use sn_ui_core::markers::Markers;
use sn_ui_core::sono_settings::Settings;
use sn_ui_core::{format, t};

use super::{Frame, MsgAction};
use crate::win95::dialog::{Buttons, MsgBox};
use crate::win95::marks::Icon;
use crate::win95::progress::Progress;

/// What a signal window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) enum WinKind {
    /// SA's sonogram work window.
    Sonogram,
    /// WF (waveform) window.
    Waveform,
    /// VMW raster.
    Raster,
    /// PhasePlane module.
    Phase,
    /// FSK/MFSK demodulator.
    Demod,
    /// OFDM module.
    Ofdm,
}

impl WinKind {
    /// Every kind, in the order Cmd+1 and on open them.
    pub(crate) const ALL: [Self; 6] = [
        Self::Sonogram,
        Self::Waveform,
        Self::Raster,
        Self::Phase,
        Self::Demod,
        Self::Ofdm,
    ];

    /// The view's name.
    pub(crate) fn label(self) -> String {
        match self {
            Self::Sonogram => t!("app-tab-sonogram"),
            Self::Waveform => t!("app-tab-waveform"),
            Self::Raster => t!("app-tab-raster"),
            Self::Phase => t!("app-tab-phase"),
            Self::Demod => t!("app-tab-demod"),
            Self::Ofdm => t!("app-tab-ofdm"),
        }
    }

    /// Client-area size of a new window of this kind (SA's sonogram child is
    /// about 800 x 300, wide enough here for both toolbars in one row; the
    /// PhasePlane form has a fixed size).
    pub(crate) fn client_size(self) -> (i32, i32) {
        match self {
            Self::Sonogram => super::sono::CLIENT,
            Self::Waveform => (720, 300),
            Self::Raster => (680, 400),
            Self::Phase => super::phase::CLIENT,
            Self::Demod => super::demod::CLIENT,
            Self::Ofdm => super::ofdm::CLIENT,
        }
    }

    /// SA's PhasePlane window does not scale.
    pub(crate) fn resizable(self) -> bool {
        self != Self::Phase
    }
}

/// A signal window as a session stores it (the same format as `sa-app`, so
/// sessions open in either front end).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SavedWin {
    pub(crate) node: NodeId,
    pub(crate) kind: WinKind,
    /// Left, top, width and height relative to the client area (the outer
    /// rectangle, frame and caption included).
    pub(crate) rect: [f32; 4],
    /// The window's title; empty in old sessions (a new one is made).
    #[serde(default)]
    pub(crate) title: String,
    /// Width and height of the window's client area. The frame and caption
    /// depend on the Windows style, so a window is restored around this
    /// size; sessions saved without it had Windows 95's frames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) client: Option<[f32; 2]>,
}

impl SavedWin {
    /// The outer `[x, y, w, h]` to restore, framed with metrics `m`: around
    /// the saved client area, or (an older session) `rect` taken as framed
    /// with Windows 95's metrics.
    ///
    /// Sessions saved by the development builds of Phase 11 before `client`
    /// was recorded were framed with the style chosen then, which the session
    /// does not say. Those are taken as Windows 95 too, so a window saved
    /// under a style with a larger frame (XP to 10) grows once by the frame
    /// difference when reopened (8 x 12 px under Windows 7, say); saving the
    /// session again records `client` and the size stays put from then on.
    pub(crate) fn rect_in(&self, m: crate::win95::style::Metrics) -> [f32; 4] {
        let [x, y, w, h] = self.rect;
        let (fw, cap) = (m.frame as f32, m.caption as f32);
        if let Some([cw, ch]) = self.client {
            [x, y, cw + 2.0 * fw, ch + 2.0 * fw + cap]
        } else {
            let old = crate::win95::style::Metrics::CLASSIC;
            let dw = 2.0 * (fw - old.frame as f32);
            let dh = dw + cap - old.caption as f32;
            [x, y, w + dw, h + dh]
        }
    }
}

/// UI state of one signal, shared by its windows (the analysis windows keep
/// their own per-window state in their modules).
#[derive(Debug, Clone)]
pub(crate) struct NodeUi {
    /// SA's window name: `test(0)(c1)`.
    pub(crate) name: String,
    pub(crate) markers: Markers,
    /// Sonogram toolbar settings.
    pub(crate) settings: Settings,
    /// The Info overlay is shown.
    pub(crate) info: bool,
    /// Waveform half range.
    pub(crate) wave_scale: f32,
    /// Waveform: draw the Q channel.
    pub(crate) show_q: bool,
    /// Fit the waveform scale to the signal once its overview arrives.
    pub(crate) fit_wave: bool,
    /// Waveform offset found by the fit.
    pub(crate) wave_offset: f64,
    /// UI values of a restored session this front end does not use yet (tones,
    /// tone grid, ADP candidates): saved back unchanged.
    pub(crate) kept: serde_json::Map<String, serde_json::Value>,
}

impl Default for NodeUi {
    fn default() -> Self {
        Self {
            name: String::new(),
            markers: Markers::default(),
            settings: Settings::default(),
            info: false,
            wave_scale: 1.0,
            show_q: true,
            fit_wave: false,
            wave_offset: 0.0,
            kept: serde_json::Map::new(),
        }
    }
}

/// Results of background jobs.
pub(crate) enum Output {
    Loaded {
        label: String,
        operation: Operation,
        signal: Signal,
        /// Problems met while loading (damaged packets replaced with silence).
        notes: Vec<String>,
    },
    /// A signal made by the signal generator; saved in `save` format if set.
    Generated {
        label: String,
        operation: Operation,
        signal: Signal,
        save: Option<sn_ui_core::edits::SaveFormat>,
    },
    Overview {
        node: NodeId,
        overview: Arc<Overview>,
    },
    Derived {
        parent: NodeId,
        edit: Edit,
        signal: Signal,
    },
    /// A session file (`report` false) or an analysis report was written.
    SessionWritten { path: PathBuf, report: bool },
    Replayed {
        path: PathBuf,
        session: Box<SessionFile>,
        outcome: Box<ReplayOutcome>,
    },
    Script {
        graph: Box<SignalGraph>,
        measures: Box<MeasurementContext>,
        output: Vec<String>,
        path: PathBuf,
    },
    /// A job of a sonogram window (measurement, ADP, save, playback).
    Sono(Box<super::sono::SonoOutput>),
    /// A job of a WF window (filter, Copy, ACF, CCF, Find tones).
    Wf(Box<super::wf::WfOutput>),
}

/// Jobs the app treats specially.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobKind {
    Other,
    /// Builds a waveform overview (not stopped by a window's Stop button).
    Overview,
    /// Writes the session file; a failure marks the session unsaved again.
    SaveSession,
    /// Writes an analysis report.
    ExportReport,
    /// Writes a signal file (the sonogram window's `Save as`).
    SaveSignal,
    /// Prepares audio for playback.
    PreparePlayback,
    /// Finds the tones of a WF window (Count freqs).
    FindTones,
    /// Generates a signal (the generator's buttons wait for it).
    Generate,
    /// The LPF/RRC filter of a WF window, whose result replaces the window's
    /// signal (a newer press or Revert stops it).
    WfFilter,
}

/// The state of the session work, see [`Frame::work_mark`].
pub(crate) type WorkMark = (u64, u64);

/// A background job started by the app.
pub(crate) struct AppJob {
    pub(crate) handle: JobHandle<Output>,
    /// Session generation the job was started in.
    pub(crate) generation: u64,
    /// The result refers to nodes by id, so it is dropped once the session is
    /// replaced.
    pub(crate) node_bound: bool,
    /// The signal whose window shows the job's progress (and whose Stop button
    /// cancels it); `None` for work shown in the status bar.
    pub(crate) origin: Option<NodeId>,
    /// The window the job was started from: its result opens offset from it.
    pub(crate) caller: Option<u32>,
    pub(crate) kind: JobKind,
    /// The user cancelled the job; a result that arrives anyway is kept but
    /// reported.
    pub(crate) cancelled: bool,
    /// For a session replay or a script: the state of the session the user
    /// agreed to replace when starting it (see [`Frame::work_mark`]). Work
    /// done while it ran is not replaced without asking again.
    pub(crate) replaces: Option<WorkMark>,
}

impl std::fmt::Debug for AppJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppJob")
            .field("name", &self.handle.name())
            .field("origin", &self.origin)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl AppJob {
    pub(crate) fn new(generation: u64, node_bound: bool, handle: JobHandle<Output>) -> Self {
        Self {
            handle,
            generation,
            node_bound,
            origin: None,
            caller: None,
            kind: JobKind::Other,
            cancelled: false,
            replaces: None,
        }
    }

    /// The job's progress for a progress field; `tick` moves a busy one.
    pub(crate) fn progress(&self, tick: u32) -> Progress {
        match self.handle.progress() {
            Some(p) if p > 0.0 && p < 1.0 => Progress::Done(p),
            _ => Progress::Busy(tick),
        }
    }

    /// Writes a file: quitting while it runs abandons it.
    pub(crate) fn writes_file(&self) -> bool {
        matches!(
            self.kind,
            JobKind::SaveSignal | JobKind::SaveSession | JobKind::ExportReport
        )
    }

    /// Ask the job to stop and remember that its result is no longer wanted.
    pub(crate) fn cancel(&mut self) {
        self.handle.cancel();
        self.cancelled = true;
    }
}

impl Frame {
    /// The signal of the active window.
    pub(crate) fn active_node(&self) -> Option<NodeId> {
        self.mdi.active().and_then(|id| self.node_of(id))
    }

    /// The signal window `win` shows.
    pub(crate) fn node_of(&self, win: u32) -> Option<NodeId> {
        self.binding(win).map(|(n, _)| n)
    }

    /// Signals still in the session, in creation order.
    pub(crate) fn live_nodes(&self) -> Vec<NodeId> {
        self.graph
            .nodes()
            .map(|n| n.id)
            .filter(|id| !self.discarded.contains(id))
            .collect()
    }

    /// The session holds signals.
    pub(crate) fn has_signals(&self) -> bool {
        self.graph.nodes().any(|n| !self.discarded.contains(&n.id))
    }

    /// The window name of a signal (`test(0)(c1)`).
    pub(crate) fn name_of(&self, node: NodeId) -> String {
        match self.ui.get(&node) {
            Some(n) if !n.name.is_empty() => n.name.clone(),
            _ => self
                .graph
                .get(node)
                .map_or_else(|| node.to_string(), |n| n.label.clone()),
        }
    }

    /// The sample rate and source path the status bar shows for `node`.
    pub(crate) fn source_of(&self, node: NodeId) -> Option<(f64, String)> {
        let rate = self.graph.get(node)?.signal.sample_rate();
        let mut cur = node;
        for _ in 0..10_000 {
            match self.graph.get(cur).and_then(|n| n.inputs.first().copied()) {
                Some(p) => cur = p,
                None => break,
            }
        }
        let root = self.graph.get(cur)?;
        let path = root
            .operation
            .params
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map_or_else(|| root.label.clone(), str::to_owned);
        Some((rate, path))
    }

    /// The settings to remember: those of the active window's signal, if any.
    pub(crate) fn current_prefs(&self) -> sn_ui_core::prefs::Prefs {
        let mut p = self.prefs;
        if let Some((id, cur)) = self
            .active_node()
            .and_then(|n| self.ui.get(&n).map(|u| (n, u)))
        {
            if self.active_kind() == Some(WinKind::Sonogram) {
                p.sonogram = cur.settings;
            }
            if self
                .graph
                .get(id)
                .is_some_and(|n| n.signal.unit() != SampleUnit::Hertz)
            {
                p.wave_scale = cur.wave_scale;
            }
            p.show_q = cur.show_q;
        }
        p
    }

    /// The kind of the active window, if it shows a signal.
    pub(crate) fn active_kind(&self) -> Option<WinKind> {
        self.mdi
            .active()
            .and_then(|id| self.binding(id))
            .map(|(_, k)| k)
    }

    /// Load `path` with `options` on a background job; the result opens in a
    /// new sonogram window.
    pub(crate) fn start_load(&mut self, path: PathBuf, options: LoadOptions) {
        if options.raw.is_some() {
            self.prefs.raw = options.raw;
        }
        let label = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        self.jobs.push(AppJob::new(
            self.generation,
            false,
            job::spawn(t!("app-job-loading", name = label.clone()), move |ctx| {
                ctx.set_total(1);
                let stop = || ctx.is_cancelled();
                let opened = sn_io::open_cancellable(&path, &options, &stop).map_err(io_to_job)?;
                ctx.advance(1);
                let operation = Operation::new(
                    "load",
                    json!({
                        "path": path.display().to_string(),
                        "channels": options.channels,
                        "scaling": options.scaling,
                        "raw": options.raw,
                    }),
                );
                Ok(Output::Loaded {
                    label,
                    operation,
                    signal: opened.signal,
                    notes: opened.notes,
                })
            }),
        ));
    }

    /// Build the waveform overview of `node` in the background.
    pub(crate) fn start_overview(&mut self, node: NodeId, signal: Signal) {
        let mut job = AppJob::new(
            self.generation,
            true,
            job::spawn(t!("app-job-overview"), move |ctx| {
                ctx.set_total(1);
                let stop = || ctx.is_cancelled();
                let overview =
                    Overview::build(&signal, Overview::DEFAULT_BLOCK, &stop).map_err(dsp_to_job)?;
                ctx.advance(1);
                Ok(Output::Overview {
                    node,
                    overview: Arc::new(overview),
                })
            }),
        );
        job.kind = JobKind::Overview;
        job.origin = Some(node);
        self.jobs.push(job);
    }

    /// Run `edit` on the signal of window `win` (the API the analysis windows
    /// use): SA's question before an ACF or CCF first, then a job whose result
    /// opens in a new window offset from `win`, named with the edit's tag.
    pub(crate) fn start_edit(&mut self, win: u32, edit: Edit) {
        if let Some(node) = self.node_of(win) {
            self.start_edit_on(node, edit, Some(win));
        }
    }

    /// Run `edit` on `parent`, asking first before an ACF or CCF and refusing
    /// one over SA's size limit.
    pub(crate) fn start_edit_on(&mut self, parent: NodeId, edit: Edit, caller: Option<u32>) {
        if let Some(signal) = self.graph.get(parent).map(|n| n.signal.clone())
            && let Some(ask) = sn_ui_core::correlation::check(&edit, &signal)
        {
            match ask {
                Ok(text) => self.ask_correlation(parent, edit, caller, &text),
                Err(e) => self.alert(&e),
            }
            return;
        }
        self.run_edit(parent, edit, caller);
    }

    /// Run `edit` on `parent` without asking.
    pub(crate) fn run_edit(&mut self, parent: NodeId, edit: Edit, caller: Option<u32>) {
        let Some(signal) = self.graph.get(parent).map(|n| n.signal.clone()) else {
            self.status = t!("app-signal-gone");
            return;
        };
        let mut job = AppJob::new(
            self.generation,
            true,
            job::spawn(edit.label(), move |ctx| {
                ctx.set_total(1);
                let stop = || ctx.is_cancelled();
                let out = edit.run(&signal, &stop).map_err(dsp_to_job)?;
                ctx.advance(1);
                Ok(Output::Derived {
                    parent,
                    edit,
                    signal: out,
                })
            }),
        );
        job.origin = Some(parent);
        job.caller = caller;
        self.jobs.push(job);
    }

    /// Collect the jobs that finished and act on their results.
    pub(crate) fn collect_finished_jobs(&mut self) {
        let (done, running): (Vec<_>, Vec<_>) =
            self.jobs.drain(..).partition(|j| j.handle.is_finished());
        self.jobs = running;
        for job in done {
            self.collect_job(job);
        }
    }

    /// Handle one finished job.
    fn collect_job(&mut self, job: AppJob) {
        let current = job.generation == self.generation;
        let name = job.handle.name().to_owned();
        let result = job.handle.join();
        if job.kind == JobKind::SaveSession && result.is_err() && current {
            // The session is not on disk; do not quit or replace it. (Not new
            // work: the change counter stays.)
            self.dirty = true;
            self.after_save = None;
        }
        if job.node_bound && (!current || job.origin.is_some_and(|n| self.discarded.contains(&n))) {
            // Started on a replaced session, or on a result whose windows closed.
            return;
        }
        if !current && let Ok(Output::SessionWritten { path, report }) = &result {
            let path = path.display().to_string();
            self.status = if *report {
                t!("app-wrote-report", path = path)
            } else {
                t!("app-saved-session-replaced", path = path)
            };
            return;
        }
        if Self::discard_cancelled(job.kind, job.cancelled, &result) {
            match job.kind {
                JobKind::PreparePlayback => self.note_playback_cancelled(),
                // A newer press or Revert stopped the filter: say so, as for a
                // filter stopped before it finished.
                JobKind::WfFilter => self.status = t!("app-job-cancelled", name = name),
                _ => {}
            }
            return;
        }
        match result {
            Err(JobError::Cancelled) if job.kind == JobKind::PreparePlayback => {
                self.note_playback_cancelled();
            }
            Ok(out) if !job.cancelled && self.must_confirm_replace(job.replaces, &out) => {
                self.hold_replace(out);
            }
            Ok(out) if job.cancelled => {
                self.handle_output(out, job.caller);
                self.status = t!("app-cancel-too-late", name = name.clone());
            }
            Ok(out) => self.handle_output(out, job.caller),
            Err(JobError::Cancelled) => self.status = t!("app-job-cancelled", name = name),
            Err(e) => {
                self.alert(&t!("app-job-failed", name = name, error = e.to_string()));
            }
        }
    }

    /// Note work the session would lose: it is unsaved, and a replay or
    /// script started before it asks again before replacing it.
    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
        self.changes = self.changes.wrapping_add(1);
    }

    /// What the session holds now, to tell whether work was done since a
    /// replay or script started: the session generation and the count of
    /// changes (signals derived, measurements recorded, tones found, ...).
    pub(crate) fn work_mark(&self) -> WorkMark {
        (self.generation, self.changes)
    }

    /// `out` would replace a session that changed after the user agreed to
    /// replace it (files loaded or results derived while the job ran).
    fn must_confirm_replace(&self, agreed: Option<WorkMark>, out: &Output) -> bool {
        let replaces = match out {
            Output::Replayed { .. } => true,
            Output::Script { graph, .. } => !graph.is_empty(),
            _ => false,
        };
        replaces && self.has_signals() && agreed != Some(self.work_mark())
    }

    /// Keep `out` and ask before it replaces the newer work.
    fn hold_replace(&mut self, out: Output) {
        let (title, question) = if matches!(out, Output::Script { .. }) {
            (
                t!("app-confirm-script-title"),
                t!("app-confirm-script-question"),
            )
        } else {
            (
                t!("app-confirm-open-title"),
                t!("app-confirm-open-question"),
            )
        };
        // An earlier held result keeps its own question (queued before this one).
        self.held.push_back(out);
        let mb = MsgBox::new(&title, &question, Some(Icon::Question), Buttons::OkCancel)
            .with_labels(vec![t!("app-confirm-replace")]);
        self.ask(mb, MsgAction::ApplyHeld);
    }

    /// The question about a held replay or script result was answered.
    pub(crate) fn held_answer(&mut self, replace: bool) {
        let Some(out) = self.held.pop_front() else {
            return;
        };
        if replace {
            self.handle_output(out, None);
        }
    }

    fn handle_output(&mut self, out: Output, caller: Option<u32>) {
        match out {
            Output::Loaded {
                label,
                operation,
                signal,
                notes,
            } => {
                self.on_loaded(label, operation, signal, &notes);
            }
            Output::Generated {
                label,
                operation,
                signal,
                save,
            } => {
                let id = self.on_loaded(label, operation, signal, &[]);
                if let Some(format) = save {
                    self.tools.save_after = Some((id, format));
                }
            }
            Output::Overview { node, overview } => {
                if let (Some(nui), Some(n)) = (self.ui.get_mut(&node), self.graph.get(node))
                    && nui.fit_wave
                {
                    nui.fit_wave = false;
                    if let Some((offset, scale)) = fit_range(&n.signal, &overview) {
                        nui.wave_offset = offset;
                        nui.wave_scale = scale;
                    }
                }
                self.overviews.insert(node, overview);
            }
            Output::Derived {
                parent,
                edit,
                signal,
            } => {
                self.on_derived(parent, &edit, signal, caller);
            }
            Output::SessionWritten { path, report } => self.on_session_written(path, report),
            Output::Replayed {
                path,
                session,
                outcome,
            } => {
                self.apply_replay(&session, *outcome);
                self.session_path = Some(path);
            }
            Output::Script {
                graph,
                measures,
                output,
                path,
            } => self.on_script(*graph, *measures, output, &path),
            Output::Sono(o) => self.on_sono_output(*o),
            Output::Wf(o) => self.on_wf_output(*o, caller),
        }
    }

    /// A recording was loaded: name it `name(0)`, open its sonogram window.
    pub(crate) fn on_loaded(
        &mut self,
        label: String,
        operation: Operation,
        signal: Signal,
        notes: &[String],
    ) -> NodeId {
        let samples = format::count(signal.len() as u64);
        let rate = signal.sample_rate().to_string();
        let name = if operation.name == sn_session::OP_GENERATE {
            self.titles.source("ocg")
        } else {
            self.titles.source(&label)
        };
        let shown = label.clone();
        let id = self.graph.add_source(label, operation, signal.clone());
        let p = self.current_prefs();
        let nui = NodeUi {
            name,
            settings: p.sonogram,
            wave_scale: p.wave_scale,
            show_q: p.show_q,
            info: p.info_default,
            ..NodeUi::default()
        };
        self.ui.insert(id, nui);
        self.mark_dirty();
        self.open_window(id, WinKind::Sonogram, None);
        self.start_overview(id, signal);
        self.status = if notes.is_empty() {
            t!("app-loaded", label = shown, samples = samples, rate = rate)
        } else {
            t!(
                "app-loaded-notes",
                label = shown,
                samples = samples,
                rate = rate,
                notes = notes.join(" ")
            )
        };
        id
    }

    /// Add the result of `edit` on `parent` to the graph; a failure is shown
    /// in SA's error box.
    pub(crate) fn add_edit_node(
        &mut self,
        parent: NodeId,
        edit: &Edit,
        signal: Signal,
    ) -> Option<NodeId> {
        let label = edit.label();
        let added = serde_json::to_value(edit)
            .map_err(|e| t!("app-edit-unrecordable", error = e.to_string()))
            .and_then(|params| {
                self.graph
                    .add_derived(
                        label.clone(),
                        Operation::new("edit", params),
                        &[parent],
                        signal,
                    )
                    .map_err(|e| e.to_string())
            });
        match added {
            Ok(id) => Some(id),
            Err(e) => {
                self.alert(&t!("app-derive-failed", label = label, error = e));
                None
            }
        }
    }

    /// An edit finished: a new signal named with the edit's tag, in a new
    /// window offset from the caller (an ACF or CCF in a WF window, as in SA).
    pub(crate) fn on_derived(
        &mut self,
        parent: NodeId,
        edit: &Edit,
        signal: Signal,
        caller: Option<u32>,
    ) -> Option<NodeId> {
        let label = edit.label();
        let id = self.add_edit_node(parent, edit, signal.clone())?;
        let carried = self.inherit_ui(parent, id, edit, &signal);
        let name = self.titles.derived(&self.name_of(parent), edit);
        if let Some(nui) = self.ui.get_mut(&id) {
            nui.name.clone_from(&name);
        }
        let caller = caller
            .filter(|c| self.mdi.get(*c).is_some())
            .or_else(|| self.find_window(parent, WinKind::Sonogram));
        let kind = if matches!(edit, Edit::Acf { .. } | Edit::Ccf { .. }) {
            WinKind::Waveform
        } else {
            WinKind::Sonogram
        };
        // The harmonics module's St^n shows its result in the same window.
        if !self.retarget_harmonics(caller, parent, edit, id) {
            self.open_window(id, kind, caller);
        }
        self.start_overview(id, signal);
        self.mark_dirty();
        let view = kind.label();
        self.status = match carried {
            Carried::Kept => t!(
                "app-derived-opened",
                label = label,
                node = name,
                view = view
            ),
            Carried::Mapped => t!(
                "app-derived-opened-mapped",
                label = label,
                node = name,
                view = view
            ),
            Carried::Cleared => t!(
                "app-derived-opened-cleared",
                label = label,
                node = name,
                view = view
            ),
        };
        Some(id)
    }

    /// The UI state a derived signal starts with: the parent's settings, and
    /// the parent's markers moved onto its own axes.
    pub(crate) fn inherit_ui(
        &mut self,
        parent: NodeId,
        id: NodeId,
        edit: &Edit,
        signal: &Signal,
    ) -> Carried {
        let (parent_fs, parent_kind, same_unit) =
            self.graph
                .get(parent)
                .map_or((signal.sample_rate(), signal.kind(), true), |n| {
                    (
                        n.signal.sample_rate(),
                        n.signal.kind(),
                        n.signal.unit() == signal.unit(),
                    )
                });
        let inherit = self.ui.get(&parent).cloned();
        let mut nui = NodeUi {
            info: self.prefs.info_default,
            ..NodeUi::default()
        };
        let mut carried = Carried::Kept;
        if let Some(p) = inherit {
            let (m, c) = lineage::inherit_markers(
                p.markers,
                edit,
                parent_fs,
                parent_kind,
                signal.sample_rate(),
                signal.len(),
            );
            nui.markers = m;
            carried = c;
            nui.settings = p.settings;
            nui.info = p.info;
            nui.show_q = p.show_q;
            if same_unit && keeps_amplitude(edit) {
                nui.wave_scale = p.wave_scale;
                nui.wave_offset = p.wave_offset;
            } else {
                nui.fit_wave = true;
            }
        }
        self.ui.insert(id, nui);
        carried
    }

    /// The progress of the work running for `node` (its window's progress
    /// field).
    pub(crate) fn node_progress(&self, node: NodeId) -> Progress {
        self.jobs
            .iter()
            .filter(|j| j.origin == Some(node) && !j.cancelled && j.kind != JobKind::Overview)
            .map(|j| j.progress(self.tick))
            .next_back()
            .unwrap_or(Progress::Idle)
    }

    /// Stop the work running for `node` (its window's Stop button).
    pub(crate) fn abort_node(&mut self, node: NodeId) {
        let mut playback = false;
        for j in self
            .jobs
            .iter_mut()
            .filter(|j| j.origin == Some(node) && !j.cancelled && j.kind != JobKind::Overview)
        {
            playback |= j.kind == JobKind::PreparePlayback;
            j.cancel();
        }
        if playback {
            // Said at once, as `sa-app` does, not when the job winds down.
            self.note_playback_cancelled();
        }
    }

    /// The job the status bar shows (work not tied to a window), if any.
    pub(crate) fn frame_job(&self) -> Option<usize> {
        self.jobs
            .iter()
            .rposition(|j| j.origin.is_none() && !j.cancelled)
    }

    /// The window name counters (for sessions and tests).
    pub(crate) fn name_nodes(&mut self, saved: &HashMap<NodeId, String>) {
        // (id, parent, label, operation name, edit)
        type Row = (NodeId, Option<NodeId>, String, String, Option<Edit>);
        let nodes: Vec<Row> = self
            .graph
            .nodes()
            .map(|n| {
                (
                    n.id,
                    n.inputs.first().copied(),
                    n.label.clone(),
                    n.operation.name.clone(),
                    lineage::edit_of(n),
                )
            })
            .collect();
        for (id, parent, label, op, edit) in nodes {
            let name = match (saved.get(&id), parent, edit) {
                (Some(s), _, _) => s.clone(),
                (None, None, _) if op == sn_session::OP_GENERATE => self.titles.source("ocg"),
                (None, None, _) => self.titles.source(&label),
                (None, Some(p), Some(e)) => self.titles.derived(&self.name_of(p), &e),
                (None, Some(p), None) => format!("{}({op})", self.name_of(p)),
            };
            self.ui.entry(id).or_default().name = name;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_kinds_store_like_sa_app() {
        let w = SavedWin {
            node: NodeId::from_raw(2),
            kind: WinKind::Waveform,
            rect: [1.0, 2.0, 300.0, 200.0],
            title: "[TempDir] a(0).wav".into(),
            client: None,
        };
        let v = serde_json::to_value(&w).unwrap();
        assert_eq!(v["kind"], "Waveform");
        let back: SavedWin =
            serde_json::from_value(json!({"node": 2, "kind": "Phase", "rect": [0, 0, 10, 10]}))
                .unwrap();
        assert_eq!(back.kind, WinKind::Phase);
        assert_eq!(back.title, "");
        assert!(!WinKind::Phase.resizable());
        assert_eq!(back.client, None, "older sessions have no client size");
        assert!(v.get("client").is_none());
    }

    #[test]
    fn saved_windows_keep_their_client_area_across_styles() {
        use crate::win95::style::Metrics;
        let old = SavedWin {
            node: NodeId::from_raw(2),
            kind: WinKind::Waveform,
            rect: [5.0, 6.0, 300.0, 200.0],
            title: String::new(),
            client: None,
        };
        // Windows 95 frames: 4 + 18; Windows 7: 8 + 22.
        assert_eq!(old.rect_in(Metrics::CLASSIC), [5.0, 6.0, 300.0, 200.0]);
        assert_eq!(old.rect_in(Metrics::AERO), [5.0, 6.0, 308.0, 212.0]);
        let new = SavedWin {
            client: Some([292.0, 174.0]),
            ..old
        };
        assert_eq!(new.rect_in(Metrics::AERO), [5.0, 6.0, 308.0, 212.0]);
        assert_eq!(new.rect_in(Metrics::FLAT), [5.0, 6.0, 304.0, 210.0]);
        let v = serde_json::to_value(&new).unwrap();
        let back: SavedWin = serde_json::from_value(v).unwrap();
        assert_eq!(back.client, Some([292.0, 174.0]));
    }
}
