//! The work the WF window's buttons start, as `sa-app`'s `app_wf.rs`: the
//! LPF/RRC filter (its result replaces the window's signal under the same
//! name), Copy (`(wfc0)`), ACF (`(wf_acf)`) and the CCF drawn over the
//! source; Revert to original; and Count freqs (Find tones in the visible span
//! between the H markers, then SA's frequency grid).

use std::sync::Arc;

use serde_json::json;
use sn_analysis::histogram::Histogram;
use sn_analysis::tones::{ToneGrid, count_grid, count_grid_hz};
use sn_core::job::{self, JobError};
use sn_core::{MeasurementKind, NodeId, Signal};
use sn_dsp::ops::Edit;
use sn_dsp::overview::Overview;
use sn_session::{UI_KEY_TONE_GRID, UI_KEY_TONE_TOL, UI_KEY_TONES};
use sn_ui_core::app_glue::tone_tolerance;
use sn_ui_core::jobs::{analysis_to_job, dsp_to_job};
use sn_ui_core::t;
use sn_ui_core::wf_plot::CcfOverlay;
use sn_ui_core::wf_tools::{self, GridLabel};

use super::Frame;
use crate::app::model::{AppJob, JobKind, Output, WinKind};

/// Where the result of a WF job goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WfDest {
    /// The filter result replaces the signal shown in window `win`.
    Replace { win: u32 },
    /// A new WF window named with the copy tag `(wfc0)`.
    CopyWindow,
    /// A new WF window named `(wf_acf)`.
    AcfWindow,
    /// A CCF drawn over the signal of window `win`.
    Overlay { win: u32, fragment: (usize, usize) },
}

/// Results of the WF window's jobs.
pub(crate) enum WfOutput {
    /// A filter, Copy, ACF or CCF finished.
    Derived {
        parent: NodeId,
        edit: Edit,
        signal: Signal,
        /// The CCF's envelope (CCF only).
        overview: Option<Arc<Overview>>,
        dest: WfDest,
    },
    /// Find tones finished.
    Tones {
        node: NodeId,
        tones: Vec<f64>,
        tol_hz: f64,
    },
}

/// A Find tones request: the span, the value range and the smoothing width.
#[derive(Debug, Clone, Copy)]
struct TonesRequest {
    start: usize,
    end: usize,
    range: (f64, f64),
    smooth: f64,
}

impl Frame {
    /// Run `edit` on `parent` for WF window `caller`; the result goes to `dest`.
    pub(crate) fn start_wf_job(&mut self, parent: NodeId, edit: Edit, dest: WfDest, caller: u32) {
        let Some(signal) = self.graph.get(parent).map(|n| n.signal.clone()) else {
            self.status = t!("app-signal-gone");
            return;
        };
        // SA's size limits of an ACF or CCF still apply.
        if let Some(Err(e)) = sn_ui_core::correlation::check(&edit, &signal) {
            self.alert(&e);
            return;
        }
        let mut job = AppJob::new(
            self.generation,
            true,
            job::spawn(edit.label(), move |ctx| {
                ctx.set_total(1);
                let stop = || ctx.is_cancelled();
                let out = edit.run(&signal, &stop).map_err(dsp_to_job)?;
                let overview = match dest {
                    WfDest::Overlay { .. } => Some(Arc::new(
                        Overview::build(&out, Overview::DEFAULT_BLOCK, &stop)
                            .map_err(dsp_to_job)?,
                    )),
                    _ => None,
                };
                ctx.advance(1);
                Ok(Output::Wf(Box::new(WfOutput::Derived {
                    parent,
                    edit,
                    signal: out,
                    overview,
                    dest,
                })))
            }),
        );
        job.origin = Some(parent);
        job.caller = Some(caller);
        if let WfDest::Replace { win } = dest {
            // Only the newest filter of a window counts.
            self.wf_stop_filters(win);
            job.kind = JobKind::WfFilter;
        }
        self.jobs.push(job);
    }

    /// Stop the running filter jobs of WF window `win`.
    pub(crate) fn wf_stop_filters(&mut self, win: u32) {
        for j in self
            .jobs
            .iter_mut()
            .filter(|j| j.kind == JobKind::WfFilter && j.caller == Some(win) && !j.cancelled)
        {
            j.cancel();
        }
    }

    /// A job of a WF window finished.
    pub(crate) fn on_wf_output(&mut self, out: WfOutput, caller: Option<u32>) {
        match out {
            WfOutput::Derived {
                parent,
                edit,
                signal,
                overview,
                dest,
            } => self.on_wf_derived(parent, &edit, signal, overview, dest, caller),
            WfOutput::Tones {
                node,
                tones,
                tol_hz,
            } => self.wf_on_tones(node, tones, tol_hz),
        }
    }

    /// A WF job finished: draw the CCF over its source, show the filtered
    /// signal in the same window, or open the result in a new WF window
    /// (`(wfc0)`, `(wf_acf)`).
    pub(crate) fn on_wf_derived(
        &mut self,
        parent: NodeId,
        edit: &Edit,
        signal: Signal,
        overview: Option<Arc<Overview>>,
        dest: WfDest,
        caller: Option<u32>,
    ) {
        if let WfDest::Overlay { win, fragment } = dest {
            if let (Some(overview), Some(ww)) = (overview, self.wf.wins.get_mut(&win))
                && ww.node == parent
            {
                ww.ccf = Some(CcfOverlay {
                    signal,
                    overview,
                    fragment,
                });
            }
            return;
        }
        let label = edit.label();
        let Some(id) = self.add_edit_node(parent, edit, signal.clone()) else {
            return;
        };
        self.inherit_ui(parent, id, edit, &signal);
        let parent_name = self.name_of(parent);
        let name = match dest {
            // SA: the filter redraws the same window, under the same name.
            WfDest::Replace { .. } => parent_name,
            WfDest::CopyWindow => format!("{parent_name}{}", self.titles.wf_copy()),
            WfDest::AcfWindow => format!("{parent_name}(wf_acf)"),
            // Drawn over its source and returned above.
            WfDest::Overlay { .. } => return,
        };
        let wave = self.ui.get(&parent).map(|p| (p.wave_scale, p.wave_offset));
        if let Some(nui) = self.ui.get_mut(&id) {
            nui.name.clone_from(&name);
            if let (WfDest::Replace { .. }, Some((scale, offset))) = (dest, wave) {
                nui.wave_scale = scale;
                nui.wave_offset = offset;
                nui.fit_wave = false;
            }
        }
        if let WfDest::Replace { win } = dest
            && !self.wf_replace_in_window(win, parent, id)
        {
            // The window shows another signal now (Revert, or a newer
            // filter): a window under the parent's name would be a second
            // window with the same title.
            self.discard_unused(id);
            self.status = t!("app-job-cancelled", name = label);
            return;
        }
        if !matches!(dest, WfDest::Replace { .. }) {
            let caller = caller.filter(|c| self.mdi.get(*c).is_some());
            self.open_window(id, WinKind::Waveform, caller);
        }
        self.start_overview(id, signal);
        self.mark_dirty();
        self.status = t!(
            "app-derived-opened",
            label = label,
            node = name,
            view = WinKind::Waveform.label()
        );
    }

    /// Show `id` (the filter result of `parent`) in window `win` if it still
    /// shows `parent`; the original is kept for Revert.
    fn wf_replace_in_window(&mut self, win: u32, parent: NodeId, id: NodeId) -> bool {
        if self.node_of(win) != Some(parent) {
            return false;
        }
        let original = self
            .wf
            .wins
            .get(&win)
            .and_then(|w| w.original)
            .unwrap_or(parent);
        self.wf_rebind_window(win, id);
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.original = Some(original);
        }
        true
    }

    /// The revert button: clear the CCF view and go back to the signal before
    /// the LPF/RRC filter (the filtered signal is dropped once nothing shows it).
    pub(crate) fn wf_revert(&mut self, win: u32, node: NodeId) {
        // A filter still running would replace the window's signal again.
        self.wf_stop_filters(win);
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        ww.ccf = None;
        let Some(orig) = ww.original else {
            return;
        };
        if self.graph.get(orig).is_none() || self.discarded.contains(&orig) {
            return;
        }
        self.wf_rebind_window(win, orig);
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.original = None;
        }
        self.discard_unused(node);
        self.mark_dirty();
    }

    /// The tones found last on `node` (kept in the session as `sa-app` keeps
    /// them).
    pub(crate) fn wf_tones(&self, node: NodeId) -> Vec<f64> {
        self.ui
            .get(&node)
            .and_then(|u| u.kept.get(UI_KEY_TONES))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// The tones of the last Find tones (any signal), else those of `node`.
    pub(crate) fn wf_last_tones_of(&self, node: NodeId) -> Vec<f64> {
        if self.wf.last_tones.is_empty() {
            self.wf_tones(node)
        } else {
            self.wf.last_tones.clone()
        }
    }

    /// How many tones [`Frame::wf_last_tones_of`] would return, without
    /// copying or decoding them (the toolbar asks on every frame).
    pub(crate) fn wf_tone_count(&self, node: NodeId) -> usize {
        if self.wf.last_tones.is_empty() {
            self.ui
                .get(&node)
                .and_then(|u| u.kept.get(UI_KEY_TONES))
                .and_then(serde_json::Value::as_array)
                // As `wf_tones` decodes them: all numbers, or none.
                .filter(|a| a.iter().all(serde_json::Value::is_number))
                .map_or(0, Vec::len)
        } else {
            self.wf.last_tones.len()
        }
    }

    /// A Find tones job for `node` is running.
    fn wf_finding_tones(&self, node: NodeId) -> bool {
        self.jobs
            .iter()
            .any(|j| j.kind == JobKind::FindTones && j.origin == Some(node) && !j.cancelled)
    }

    /// Count freqs (WF popup, Sum on): find the tones between the H markers in
    /// the visible span, then fit SA's frequency grid to them.
    pub(crate) fn wf_start_count_freqs(&mut self, win: u32) {
        let Some(ww) = self.wf.wins.get(&win) else {
            return;
        };
        let node = ww.node;
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let Some(range) = ww.markers.freq_range() else {
            self.alert(&format!(
                "{}\n{}",
                t!("sw-check-settings"),
                t!("wf-count-needs-h")
            ));
            return;
        };
        let (start, end) = ww.view.as_ref().map_or((0, signal.len()), |v| {
            wf_tools::visible_span(v, f64::from(ww.plot.w), signal.len())
        });
        if self.wf_finding_tones(node) {
            self.status = t!("wf-count-tones-busy");
            return;
        }
        let scale = self.ui.get(&node).map_or(1.0, |u| u.wave_scale);
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.count_pending = Some(range);
            ww.tones_ready = false;
        }
        let smooth = wf_tools::tones_smoothing(scale);
        self.wf_start_find_tones(
            node,
            signal,
            TonesRequest {
                start,
                end,
                range,
                smooth,
            },
        );
    }

    /// Run Find tones on `node` in the background.
    fn wf_start_find_tones(&mut self, node: NodeId, signal: Signal, r: TonesRequest) {
        let mut job = AppJob::new(
            self.generation,
            true,
            job::spawn(t!("app-job-find-tones"), move |ctx| {
                ctx.set_total(1);
                ctx.checkpoint()?;
                let stop = || ctx.is_cancelled();
                let h =
                    Histogram::build_with_stop(&signal, r.start, r.end, 4000, Some(r.range), &stop)
                        .map_err(|e| match analysis_to_job(e) {
                            JobError::Failed(m) => {
                                JobError::Failed(t!("app-find-tones-failed", error = m))
                            }
                            other => other,
                        })?;
                ctx.checkpoint()?;
                let mut tones = h.peaks(0.15, r.smooth);
                tones.sort_by(f64::total_cmp);
                ctx.advance(1);
                Ok(Output::Wf(Box::new(WfOutput::Tones {
                    node,
                    tones,
                    tol_hz: tone_tolerance(h.bin_width(), r.smooth),
                })))
            }),
        );
        job.kind = JobKind::FindTones;
        job.origin = Some(node);
        self.jobs.push(job);
    }

    /// Find tones finished: keep the tones (the session and resynthesis use
    /// them) and let a waiting Count freqs go on.
    pub(crate) fn wf_on_tones(&mut self, node: NodeId, tones: Vec<f64>, tol_hz: f64) {
        let name = self.name_of(node);
        let count = tones.len();
        if let Some(u) = self.ui.get_mut(&node) {
            u.kept.insert(UI_KEY_TONES.to_owned(), json!(tones));
            u.kept.insert(UI_KEY_TONE_TOL.to_owned(), json!(tol_hz));
            u.kept.remove(UI_KEY_TONE_GRID);
        }
        self.wf.last_tones = tones;
        for ww in self.wf.wins.values_mut().filter(|w| w.node == node) {
            if ww.count_pending.is_some() {
                ww.tones_ready = true;
            }
        }
        self.status = t!("app-tones-found", count = count, node = name);
        self.mark_dirty();
        self.wf_finish_counts();
    }

    /// Settle every waiting Count freqs whose Find tones is over (found,
    /// failed or cancelled), whatever the window shows (WF or VMW). The poll
    /// loop calls it, so a pending Count freqs never keeps it running.
    pub(crate) fn wf_finish_counts(&mut self) {
        let waiting: Vec<u32> = self
            .wf
            .wins
            .iter()
            .filter(|(_, w)| w.count_pending.is_some())
            .map(|(id, _)| *id)
            .collect();
        for win in waiting {
            self.wf_finish_count_freqs(win);
        }
    }

    /// Once the tones of a Count freqs are in, fit the grid and show SA's
    /// result box (`Freq grid: 32`, and `Delta: 75,12 Hz` on a frequency
    /// trace), or SA's error. A failed Find tones has shown its own error.
    pub(crate) fn wf_finish_count_freqs(&mut self, win: u32) {
        let Some(ww) = self.wf.wins.get(&win) else {
            return;
        };
        let node = ww.node;
        let Some(range) = ww.count_pending else {
            return;
        };
        if self.wf_finding_tones(node) {
            return;
        }
        let ready = ww.tones_ready;
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.count_pending = None;
            ww.tones_ready = false;
        }
        if !ready {
            return;
        }
        let (lo, hi) = (Some(range.0), Some(range.1));
        let tones = self.wf_tones(node);
        let tol = self
            .ui
            .get(&node)
            .and_then(|u| u.kept.get(UI_KEY_TONE_TOL))
            .and_then(serde_json::Value::as_f64)
            .filter(|t| t.is_finite() && *t > 0.0);
        let fit = match tol {
            Some(tol) => count_grid_hz(&tones, lo, hi, tol),
            None => count_grid(&tones, lo, hi, 0.2),
        };
        match fit {
            Ok(g) => self.wf_show_grid(win, node, g),
            Err(e) => self.alert(&format!(
                "{}\n{}",
                t!("sw-check-settings"),
                t!("wf-count-freqs-failed", error = e.to_string())
            )),
        }
    }

    /// Show Count freqs' result box for grid `g` in window `win`.
    fn wf_show_grid(&mut self, win: u32, node: NodeId, g: ToneGrid) {
        let hz = self.wf_is_hz(node);
        if let Some(u) = self.ui.get_mut(&node) {
            u.kept.insert(UI_KEY_TONE_GRID.to_owned(), json!(g));
        }
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.labels = vec![GridLabel {
                lines: wf_tools::grid_lines(g.size, g.spacing, hz),
                offset: None,
            }];
        }
        self.mark_dirty();
    }

    /// `SigNexora tools` > `Measure` > dT: record the span between the signal's V
    /// markers as the period.
    pub(crate) fn wf_record_dt(&mut self, node: NodeId) {
        let Some(fs) = self.graph.get(node).map(|n| n.signal.sample_rate()) else {
            return;
        };
        let dt = self.ui.get(&node).and_then(|u| u.markers.readout(fs).d_t);
        if let Some(d) = dt {
            self.record_with(
                &MeasurementKind::Period,
                d,
                None,
                node,
                sn_ui_core::app_glue::tool_id::V_MARKERS,
            );
        }
    }
}
