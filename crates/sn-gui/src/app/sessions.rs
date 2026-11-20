//! Sessions (open, save, save as, restore with the windows), the analysis
//! report, scripts, the "save first?" question and quitting, as in `sa-app`.
//!
//! The file choosers are the platform's (GPUI's path prompts), as SA used the
//! standard Windows `Open` and `Save As` dialogs.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{Context, PathPromptOptions};
use serde_json::json;
use sn_core::job::{self, JobError};
use sn_core::{MeasurementContext, NodeId, Signal, SignalGraph};
use sn_session::{
    CaptureOptions, ReplayOptions, ReplayOutcome, ReplayProgress, SessionFile, UI_KEY_ADP,
    UI_KEY_COUNTERS, UI_KEY_INFO, UI_KEY_MARKERS, UI_KEY_NAME, UI_KEY_TONE_GRID, UI_KEY_TONES,
    UI_KEY_WINDOWS, replay,
};
use sn_ui_core::app_glue::{Confirm, confirm_texts};
use sn_ui_core::jobs::session_to_job;
use sn_ui_core::t;
use sn_ui_core::titles::Counters;

use super::dialogs::{BOX_TITLE, chooser_answer};
use super::model::{AppJob, JobKind, Output, SavedWin, WinKind};
use super::{ChildKind, Frame, MsgAction};
use crate::win95::dialog::{Answer, Buttons, MsgBox};
use crate::win95::marks::Icon;
use crate::win95::mdi;

/// A file chooser of the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Chooser {
    OpenSession,
    SaveSessionAs,
    Report(bool),
    RunScript,
}

/// Most lines of script output kept (a script printing in an endless loop
/// must not fill the memory before the user presses Stop).
const MAX_SCRIPT_LINES: usize = 100_000;

/// The lines a script printed, up to [`MAX_SCRIPT_LINES`].
#[derive(Debug, Default)]
struct ScriptLines {
    lines: Vec<String>,
    dropped: usize,
}

impl ScriptLines {
    fn push(&mut self, s: &str) {
        if self.lines.len() < MAX_SCRIPT_LINES {
            self.lines.push(s.to_owned());
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// The kept lines, ending with a note when some were dropped.
    fn finish(self) -> Vec<String> {
        let mut v = self.lines;
        if self.dropped > 0 {
            v.push(t!(
                "app-script-output-truncated",
                lines = MAX_SCRIPT_LINES.to_string(),
                dropped = self.dropped.to_string()
            ));
        }
        v
    }
}

impl Frame {
    /// Unsaved work that replacing the session or quitting would lose.
    pub(crate) fn unsaved(&self) -> bool {
        (self.dirty || self.saving_session()) && self.has_signals()
    }

    /// True while a session file is being written.
    pub(crate) fn saving_session(&self) -> bool {
        self.jobs.iter().any(|j| j.kind == JobKind::SaveSession)
    }

    /// Names of the running jobs that write a file.
    pub(crate) fn file_writes(&self) -> Vec<String> {
        self.jobs
            .iter()
            .filter(|j| j.writes_file())
            .map(|j| j.handle.name().to_owned())
            .collect()
    }

    /// Whether `c` must ask first.
    pub(crate) fn must_ask(&self, c: Confirm) -> bool {
        self.unsaved()
            || (c == Confirm::Quit && !self.file_writes().is_empty())
            || (c == Confirm::CloseAll && !self.jobs_stopped_by_closing_all().is_empty())
    }

    /// Show the "save first?" question for `c` as an SA message box: `Save
    /// session first`, the action, `Cancel`.
    pub(crate) fn confirm(&mut self, c: Confirm) {
        let unsaved = self.unsaved();
        let Some((title, question, go)) = confirm_texts(c, unsaved) else {
            return;
        };
        let mut lines = vec![question];
        if unsaved {
            lines.push(t!(
                "app-confirm-lost",
                signals = self.live_nodes().len(),
                measurements = self.measures.len()
            ));
        }
        let writes = if c == Confirm::Quit {
            self.file_writes()
        } else {
            Vec::new()
        };
        for w in &writes {
            lines.push(t!("app-confirm-write-running", name = w.as_str()));
        }
        if c == Confirm::CloseAll {
            for name in self.jobs_stopped_by_closing_all() {
                lines.push(t!("app-confirm-close-all-stops", name = name));
            }
        }
        let cancel = if writes.is_empty() {
            t!("app-cancel")
        } else {
            t!("app-confirm-wait")
        };
        let mb = if unsaved {
            let save = if self.session_path.is_some() {
                t!("app-confirm-save-first")
            } else {
                t!("app-confirm-save-first-ask")
            };
            MsgBox::new(
                &title,
                &lines.join("\n"),
                Some(Icon::Warning),
                Buttons::YesNoCancel,
            )
            .with_labels(vec![save, go, cancel])
        } else {
            MsgBox::new(
                &title,
                &lines.join("\n"),
                Some(Icon::Warning),
                Buttons::OkCancel,
            )
            .with_labels(vec![go, cancel])
        };
        self.ask(mb, MsgAction::Confirm(c));
    }

    /// The answer to the "save first?" question.
    pub(crate) fn confirm_answer(&mut self, c: Confirm, a: Answer, cx: &mut Context<'_, Self>) {
        match a {
            Answer::Yes => {
                self.after_save = Some(c);
                self.save_session(cx);
            }
            Answer::No | Answer::Ok => self.do_confirmed(c, cx),
            Answer::Cancel => {}
        }
    }

    /// Carry out `c` without asking.
    pub(crate) fn do_confirmed(&mut self, c: Confirm, cx: &mut Context<'_, Self>) {
        match c {
            Confirm::OpenSession => self.choose(Chooser::OpenSession, cx),
            Confirm::RunScript => self.choose(Chooser::RunScript, cx),
            Confirm::CloseAll => self.close_all_windows(),
            Confirm::Quit => {
                self.quit_now(cx);
            }
            Confirm::CloseWindow(id) => self.close_window_now(id as u32),
        }
    }

    /// Quit: remember the settings, abandon file writes, close. When the
    /// settings could not be saved, the user is told first and the app quits
    /// once the box is answered. Returns whether the app quit now.
    pub(crate) fn quit_now(&mut self, cx: &mut Context<'_, Self>) -> bool {
        let saved = self.save_prefs();
        self.abandon_file_writes();
        match saved {
            Ok(()) => {
                cx.quit();
                true
            }
            Err(msg) => {
                let mb = MsgBox::new(BOX_TITLE, &msg, Some(Icon::Error), Buttons::Ok);
                self.ask(mb, MsgAction::Quit);
                cx.notify();
                false
            }
        }
    }

    /// Stop the running file writes and wait for them, so each removes its
    /// partial temporary file before the app exits.
    fn abandon_file_writes(&mut self) {
        let (mut writes, rest): (Vec<_>, Vec<_>) =
            self.jobs.drain(..).partition(AppJob::writes_file);
        self.jobs = rest;
        for j in &mut writes {
            j.cancel();
        }
        for j in writes {
            drop(j.handle.join());
        }
    }

    /// Save the session to its file, or ask for one.
    pub(crate) fn save_session(&mut self, cx: &mut Context<'_, Self>) {
        match self.session_path.clone() {
            Some(path) => {
                self.write_session_to(path);
            }
            None => self.choose(Chooser::SaveSessionAs, cx),
        }
    }

    /// Ask the platform's file chooser, then carry on with the chosen path.
    pub(crate) fn choose(&mut self, what: Chooser, cx: &mut Context<'_, Self>) {
        if what == Chooser::SaveSessionAs && self.saving_session() {
            self.status = t!("app-session-save-running");
            self.after_save = None;
            return;
        }
        let dir = self.chooser_dir();
        let (open, name) = match what {
            Chooser::OpenSession | Chooser::RunScript => (true, ""),
            Chooser::SaveSessionAs => (false, "session.json"),
            Chooser::Report(true) => (false, "report.html"),
            Chooser::Report(false) => (false, "report.md"),
        };
        let suggested = self
            .session_path
            .as_ref()
            .filter(|_| what == Chooser::SaveSessionAs)
            .and_then(|p| p.file_name())
            .map_or_else(|| name.to_owned(), |n| n.to_string_lossy().into_owned());
        let rx = if open {
            let prompt = match what {
                Chooser::RunScript => t!("app-dialog-run-script"),
                _ => t!("app-dialog-open-session"),
            };
            let paths = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some(prompt.into()),
            });
            cx.spawn(async move |_, _| {
                chooser_answer(paths.await).map(|p| p.and_then(|v| v.into_iter().next()))
            })
        } else {
            let path = cx.prompt_for_new_path(&dir, Some(&suggested));
            cx.spawn(async move |_, _| chooser_answer(path.await))
        };
        cx.spawn(async move |this, cx| {
            let chosen = rx.await;
            this.update(cx, |v, cx| {
                v.chosen(what, chosen);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The folder a file chooser starts in: the session's folder, else the
    /// current folder, else the home folder. A folder that cannot be read is
    /// skipped quietly: the chooser lets the user pick any folder anyway.
    pub(crate) fn chooser_dir(&self) -> PathBuf {
        self.session_path
            .as_ref()
            .and_then(|p| p.parent())
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// The file chooser answered: `Ok(None)` cancelled, `Err` it could not
    /// be shown (the user is told why).
    pub(crate) fn chosen(&mut self, what: Chooser, answer: Result<Option<PathBuf>, String>) {
        let path = match answer {
            Ok(Some(path)) => path,
            Ok(None) => {
                if what == Chooser::SaveSessionAs {
                    self.after_save = None;
                }
                return;
            }
            Err(e) => {
                if what == Chooser::SaveSessionAs {
                    self.after_save = None;
                }
                let action = match what {
                    Chooser::OpenSession => t!("app-dialog-open-session"),
                    Chooser::SaveSessionAs => t!("app-dialog-save-session-as"),
                    Chooser::Report(_) => t!("app-dialog-export-report"),
                    Chooser::RunScript => t!("app-dialog-run-script"),
                };
                self.alert(&t!("app-chooser-failed", action = action, error = e));
                return;
            }
        };
        match what {
            Chooser::OpenSession => self.open_session_from(path),
            Chooser::SaveSessionAs => {
                if !self.write_session_to(path) {
                    self.after_save = None;
                }
            }
            Chooser::Report(html) => self.start_write_session(path, Some(html)),
            Chooser::RunScript => self.run_script_from(path),
        }
    }

    /// Start writing the session to `path`; false when a save is running.
    pub(crate) fn write_session_to(&mut self, path: PathBuf) -> bool {
        if self.saving_session() {
            self.status = t!("app-session-save-running");
            return false;
        }
        self.dirty = false;
        self.start_write_session(path, None);
        true
    }

    /// Probe the sources and write the session (`report` None) or a report in
    /// the background.
    pub(crate) fn start_write_session(&mut self, path: PathBuf, report: Option<bool>) {
        let mut session = self.capture_session();
        let name = t!("app-job-writing", path = path.display().to_string());
        let mut job = AppJob::new(
            self.generation,
            false,
            job::spawn(name, move |ctx| {
                ctx.set_total(1);
                ctx.checkpoint()?;
                session.probe_sources(path.parent());
                ctx.checkpoint()?;
                let written = match report {
                    None => sn_session::save(&path, &session),
                    Some(html) => {
                        let text = if html {
                            sn_session::report::html(&session)
                        } else {
                            sn_session::report::markdown(&session)
                        };
                        sn_session::report::save_report(&path, &text)
                    }
                };
                written.map_err(|e| {
                    let error = e.to_string();
                    JobError::Failed(if report.is_some() {
                        t!("app-export-report-failed", error = error)
                    } else {
                        t!("app-save-session-failed", error = error)
                    })
                })?;
                ctx.advance(1);
                Ok(Output::SessionWritten {
                    path,
                    report: report.is_some(),
                })
            }),
        );
        job.kind = if report.is_none() {
            JobKind::SaveSession
        } else {
            JobKind::ExportReport
        };
        self.jobs.push(job);
    }

    /// A session file or report was written for the current session.
    pub(crate) fn on_session_written(&mut self, path: PathBuf, report: bool) {
        let shown = path.display().to_string();
        self.status = if report {
            t!("app-wrote-report", path = shown)
        } else {
            t!("app-saved-session", path = shown)
        };
        if !report {
            self.session_path = Some(path);
            if let Some(c) = self.after_save.take() {
                // Ask again if something changed while the file was written.
                if self.must_ask(c) {
                    self.confirm(c);
                } else {
                    self.pending.push(c);
                }
            }
        }
    }

    /// The whole session with the UI state of every signal and the windows.
    pub(crate) fn capture_session(&self) -> SessionFile {
        let mut s = SessionFile::capture_with(
            &self.graph,
            &self.measures,
            &CaptureOptions {
                base_dir: None,
                probe_sources: false,
            },
        );
        s.nodes.retain(|n| !self.discarded.contains(&n.id));
        for m in &mut s.measurements {
            if m.measurement
                .source
                .is_some_and(|n| self.discarded.contains(&n))
            {
                m.measurement.source = None;
            }
        }
        for (id, nui) in &self.ui {
            if let Some(map) = s.node_ui_mut(*id) {
                for (k, v) in &nui.kept {
                    map.insert(k.clone(), v.clone());
                }
                let pair = |p: Option<(f64, f64)>| p.map(|(a, b)| json!([a, b]));
                map.insert(
                    UI_KEY_MARKERS.into(),
                    json!({"v": pair(nui.markers.v), "h": pair(nui.markers.h)}),
                );
                map.insert(UI_KEY_NAME.into(), json!(nui.name));
                map.insert(UI_KEY_INFO.into(), json!(nui.info));
            }
        }
        s.ui.insert(UI_KEY_WINDOWS.into(), json!(self.window_layout()));
        s.ui.insert(UI_KEY_COUNTERS.into(), json!(self.titles));
        s
    }

    /// The signal windows, bottom to top, relative to the client area.
    pub(crate) fn window_layout(&self) -> Vec<SavedWin> {
        self.signal_windows()
            .into_iter()
            .filter_map(|(id, node, kind)| {
                let c = self.mdi.get(id)?;
                let r = c.rect;
                let client = mdi::client_rect(r.w, r.h);
                Some(SavedWin {
                    node,
                    kind,
                    rect: [r.x as f32, r.y as f32, r.w as f32, r.h as f32],
                    title: c.title.clone(),
                    client: Some([client.w as f32, client.h as f32]),
                })
            })
            .collect()
    }

    /// Load the session at `path` and replay it in the background (the file
    /// is read and parsed there too, so a slow drive does not freeze the frame;
    /// a failure reaches the user through the job-failed alert).
    pub(crate) fn open_session_from(&mut self, path: PathBuf) {
        let base_dir = path.parent().map(Path::to_path_buf);
        let mut j = AppJob::new(
            self.generation,
            false,
            job::spawn(t!("app-job-replay"), move |ctx| {
                let session = sn_session::load(&path).map_err(|e| {
                    JobError::Failed(t!("app-open-session-failed", error = e.to_string()))
                })?;
                let stop = || ctx.is_cancelled();
                ctx.set_total((session.nodes.len() as u64).max(1));
                // The callback comes before each node: advance for the node
                // before it, and for the last one once replay returns.
                let mut progress = |p: ReplayProgress<'_>| {
                    ctx.set_total((p.total as u64).max(1));
                    ctx.advance(u64::from(p.done > 0));
                };
                let outcome = replay(&session, &ReplayOptions { base_dir }, &mut progress, &stop)
                    .map_err(session_to_job)?;
                ctx.advance(u64::from(!session.nodes.is_empty()));
                Ok(Output::Replayed {
                    path,
                    session: Box::new(session),
                    outcome: Box::new(outcome),
                })
            }),
        );
        j.replaces = Some(self.work_mark());
        self.jobs.push(j);
    }

    /// Run the Rhai script at `path` in the background.
    pub(crate) fn run_script_from(&mut self, path: PathBuf) {
        let mut j = AppJob::new(
            self.generation,
            false,
            job::spawn(t!("app-job-script"), move |ctx| {
                ctx.set_total(1);
                let cancel = ctx.clone();
                // The print hook runs on this thread: a RefCell is enough.
                let lines = Rc::new(RefCell::new(ScriptLines::default()));
                let sink = Rc::clone(&lines);
                let options = sn_script::ScriptOptions {
                    print: Some(Rc::new(move |s: &str| sink.borrow_mut().push(s))),
                    should_stop: Some(Arc::new(move || cancel.is_cancelled())),
                    ..sn_script::ScriptOptions::default()
                };
                let mut engine = sn_script::ScriptEngine::new(options);
                let result = engine.run_file(&path);
                let mut output = std::mem::take(&mut *lines.borrow_mut()).finish();
                if let Err(e) = result {
                    output.push(t!("app-script-error", error = e.to_string()));
                }
                let (graph, measures) = engine.into_parts();
                ctx.advance(1);
                Ok(Output::Script {
                    graph: Box::new(graph),
                    measures: Box::new(measures),
                    output,
                    path,
                })
            }),
        );
        j.replaces = Some(self.work_mark());
        self.jobs.push(j);
    }

    /// A script finished: its signals replace the session; its output opens
    /// in a window.
    pub(crate) fn on_script(
        &mut self,
        graph: SignalGraph,
        measures: MeasurementContext,
        output: Vec<String>,
        path: &Path,
    ) {
        let shown = path.display().to_string();
        if graph.is_empty() {
            self.status = t!("app-script-no-signals", path = shown.clone());
        } else {
            self.replace_session(graph, measures, &HashMap::new(), None);
            // The script's signals are in no session file yet: quitting or
            // opening a session must ask first.
            self.mark_dirty();
            self.open_leaf_windows();
            self.status = t!("app-script-finished", path = shown.clone());
        }
        let lines = if output.is_empty() {
            vec![t!("app-script-no-output")]
        } else {
            output
        };
        self.show_script_output(&t!("app-script-output", path = shown), lines);
    }

    /// Replace the session with a new graph. Every signal window closes; the
    /// caller opens the new session's windows.
    pub(crate) fn replace_session(
        &mut self,
        graph: SignalGraph,
        measures: MeasurementContext,
        names: &HashMap<NodeId, String>,
        counters: Option<Counters>,
    ) {
        self.generation = self.generation.wrapping_add(1);
        for j in self.jobs.iter_mut().filter(|j| j.node_bound) {
            j.cancel();
        }
        self.graph = graph;
        self.measures = measures;
        self.session_path = None;
        self.ui.clear();
        self.overviews.clear();
        for (id, _, _) in self.signal_windows() {
            self.mdi.close(id);
        }
        self.kinds
            .retain(|(_, k)| !matches!(k, ChildKind::Signal(..)));
        self.discarded.clear();
        self.titles = counters.unwrap_or_default();
        self.params = None;
        self.drop_node_questions();
        self.last_source = None;
        self.dirty = false;
        self.name_nodes(names);
        let ids: Vec<(NodeId, Signal)> = self
            .graph
            .nodes()
            .map(|n| (n.id, n.signal.clone()))
            .collect();
        let info = self.prefs.info_default;
        for (id, signal) in ids {
            self.ui.entry(id).or_default().info = info;
            self.start_overview(id, signal);
        }
    }

    /// Open a sonogram window for every result (signals nothing was derived
    /// from), for a session without a saved layout.
    pub(crate) fn open_leaf_windows(&mut self) {
        let leaves: Vec<NodeId> = self
            .graph
            .nodes()
            .filter(|n| self.graph.children(n.id).next().is_none())
            .map(|n| n.id)
            .collect();
        for id in leaves {
            self.open_window(id, WinKind::Sonogram, None);
        }
    }

    /// A replayed session: the new graph, the saved names, UI state and windows.
    pub(crate) fn apply_replay(&mut self, session: &SessionFile, outcome: ReplayOutcome) {
        let mut issues: Vec<String> = outcome
            .issues
            .iter()
            .map(|i| format!("{}: {}", i.label, i.message))
            .collect();
        let pairs: Vec<(NodeId, NodeId)> = outcome.id_map.iter().map(|(a, b)| (*a, *b)).collect();
        let names: HashMap<NodeId, String> = pairs
            .iter()
            .filter_map(|(old, new)| {
                let name = session.node(*old)?.ui.get(UI_KEY_NAME)?.as_str()?;
                Some((*new, name.to_owned()))
            })
            .collect();
        let counters = match session
            .ui
            .get(UI_KEY_COUNTERS)
            .map(|v| serde_json::from_value::<Counters>(v.clone()))
        {
            Some(Ok(c)) => Some(c),
            Some(Err(e)) => {
                issues.push(t!(
                    "app-replay-session-skipped",
                    what = "counters",
                    error = e.to_string()
                ));
                None
            }
            None => None,
        };
        self.replace_session(outcome.graph, outcome.measurements, &names, counters);
        for (old, new) in &pairs {
            if let Some(saved) = session.node(*old) {
                self.restore_node_ui(saved, *new, &mut issues);
            }
        }
        self.restore_windows(session, &pairs, &mut issues);
        if issues.is_empty() {
            self.status = t!("app-session-restored");
        } else {
            self.status = t!("app-session-restored-problems", count = issues.len());
            self.open_text(&t!("app-replay-problems-title"), issues);
        }
    }

    /// Read the saved UI state of one node onto `new`; values this front end
    /// does not use yet are kept for the next save.
    fn restore_node_ui(
        &mut self,
        saved: &sn_session::SessionNode,
        new: NodeId,
        issues: &mut Vec<String>,
    ) {
        let Some(nui) = self.ui.get_mut(&new) else {
            return;
        };
        let name = nui.name.clone();
        if let Some(m) = saved.ui.get(UI_KEY_MARKERS) {
            for (slot, key) in [(&mut nui.markers.v, "v"), (&mut nui.markers.h, "h")] {
                match m
                    .get(key)
                    .map(|v| serde_json::from_value::<Option<(f64, f64)>>(v.clone()))
                {
                    Some(Ok(pair)) => *slot = pair,
                    Some(Err(e)) => issues.push(t!(
                        "app-replay-skipped",
                        what = "markers",
                        node = name.clone(),
                        label = saved.label.clone(),
                        error = e.to_string()
                    )),
                    None => {}
                }
            }
        }
        if let Some(info) = saved
            .ui
            .get(UI_KEY_INFO)
            .and_then(serde_json::Value::as_bool)
        {
            nui.info = info;
        }
        for key in [
            UI_KEY_TONES,
            UI_KEY_TONE_GRID,
            UI_KEY_ADP,
            "tone_tolerance_hz",
        ] {
            if let Some(v) = saved.ui.get(key) {
                nui.kept.insert(key.to_owned(), v.clone());
            }
        }
    }

    /// Open the windows a session saved, at their places; a session without a
    /// layout opens a sonogram window per result.
    fn restore_windows(
        &mut self,
        session: &SessionFile,
        pairs: &[(NodeId, NodeId)],
        issues: &mut Vec<String>,
    ) {
        let saved: Option<Vec<SavedWin>> = match session
            .ui
            .get(UI_KEY_WINDOWS)
            .map(|v| serde_json::from_value(v.clone()))
        {
            Some(Ok(s)) => Some(s),
            Some(Err(e)) => {
                issues.push(t!(
                    "app-replay-session-skipped",
                    what = "windows",
                    error = e.to_string()
                ));
                None
            }
            None => None,
        };
        let Some(saved) = saved.filter(|s| !s.is_empty()) else {
            self.open_leaf_windows();
            return;
        };
        let map: HashMap<NodeId, NodeId> = pairs.iter().copied().collect();
        let mut missing = 0_usize;
        for w in saved {
            let Some(node) = map.get(&w.node).copied() else {
                missing += 1;
                continue;
            };
            let title = if w.title.is_empty() {
                self.title_for(node, w.kind)
            } else {
                w.title.clone()
            };
            if let (_, Some(issue)) = self.restore_window(&SavedWin { node, ..w }, &title) {
                issues.push(issue);
            }
        }
        if missing > 0 {
            issues.push(t!("app-replay-windows-missing", count = missing));
        }
        if self.signal_windows().is_empty() {
            self.open_leaf_windows();
        }
    }
}
