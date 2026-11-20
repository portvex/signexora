//! What the sonogram window's buttons and popup menu run besides edits, as
//! `sa-app` does: Measurement of Freq (Br, or Sc on a power-N result, with
//! SA's `Freq:` box), Auto define params (`Brate:` box), Save as, and playback
//! of the V span; and the results of their jobs.

use std::path::PathBuf;

use gpui::Context;
use serde_json::json;
use sn_analysis::adp::BaudCandidate;
use sn_analysis::line::LineEstimate;
use sn_core::job::{self, JobError};
use sn_core::{Measurement, MeasurementKind, NodeId, Signal};
use sn_ui_core::app_glue::{adp_quality, record_measurement, tool_id, tool_label, with_unit};
use sn_ui_core::edits::{MeasuredValue, SaveFormat, file_stem};
use sn_ui_core::jobs::{analysis_to_job, io_to_job};
use sn_ui_core::lineage::{self, FreqMap};
use sn_ui_core::playback::{self, Prepared};
use sn_ui_core::sono_tools::{self, FloatLabel};
use sn_ui_core::t;

use super::Frame;
use crate::app::dialogs::chooser_answer;
use crate::app::model::{AppJob, JobKind, Output};

/// Results of the sonogram window's own jobs.
pub(crate) enum SonoOutput {
    /// Measurement of Freq finished.
    Measured {
        node: NodeId,
        kind: MeasurementKind,
        estimate: LineEstimate,
        map: FreqMap,
        power: u32,
    },
    /// ADP finished.
    Adp {
        node: NodeId,
        candidates: Vec<BaudCandidate>,
    },
    /// A signal was saved.
    Saved { path: PathBuf },
    /// Audio is ready to play.
    Play { prepared: Prepared },
}

/// The V span (or the whole signal) and the H band of a signal.
struct Selection {
    signal: Signal,
    start: usize,
    end: usize,
    band: Option<(f64, f64)>,
}

impl Frame {
    /// The latest measurement of `kind` with its source, for pre-filling tools.
    pub(crate) fn measured(&self, kind: &MeasurementKind) -> Option<MeasuredValue> {
        self.measures.latest(kind).map(|m| MeasuredValue {
            value: m.value,
            from: match m.source {
                Some(n) => format!("{}, {}", self.name_of(n), tool_label(&m.tool)),
                None => tool_label(&m.tool),
            },
            other_axis: false,
        })
    }

    /// The latest Sc on the frequency axis of `node`: mapped through the
    /// lineage from the signal it is expressed on, or flagged when no mapping
    /// exists.
    pub(crate) fn sc_for(&self, node: NodeId) -> Option<MeasuredValue> {
        let mut mv = self.measured(&MeasurementKind::Carrier)?;
        let Some(src) = self
            .measures
            .latest(&MeasurementKind::Carrier)
            .and_then(|m| m.source)
        else {
            return Some(mv);
        };
        let axis = lineage::carrier_axis(&self.graph, src);
        if axis == node {
            return Some(mv);
        }
        match lineage::map_freq(&self.graph, axis, node, mv.value) {
            Some(v) => {
                if (v - mv.value).abs() > 1e-9 {
                    mv.from = t!(
                        "app-mapped-onto",
                        from = mv.from.clone(),
                        node = self.name_of(node)
                    );
                }
                mv.value = v;
            }
            None => mv.other_axis = true,
        }
        Some(mv)
    }

    /// The V span (or the whole signal) and the H band of `node`.
    fn selection(&self, node: NodeId) -> Option<Selection> {
        let n = self.graph.get(node)?;
        let m = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let (a, b) = m.sample_span(n.signal.len());
        Some(Selection {
            signal: n.signal.clone(),
            start: a,
            end: b,
            band: m.freq_range(),
        })
    }

    /// Start a job for `node`'s window (its progress field and Stop).
    fn push_node_job(&mut self, node: NodeId, kind: JobKind, handle: job::JobHandle<Output>) {
        let mut job = AppJob::new(self.generation, true, handle);
        job.origin = Some(node);
        job.kind = kind;
        self.jobs.push(job);
    }

    /// SA's Measurement of Freq: the line between the H markers. On a power-N
    /// result (SA's harmonics module) it is the carrier Sc, elsewhere the
    /// clock line Br.
    pub(crate) fn measure_freq(&mut self, node: NodeId) {
        let kind = match lineage::carrier_map(&self.graph, node) {
            Ok((_, power)) if power > 1 => MeasurementKind::Carrier,
            _ => MeasurementKind::BaudRate,
        };
        self.start_measure(node, kind);
    }

    pub(crate) fn start_measure(&mut self, node: NodeId, kind: MeasurementKind) {
        let Some(sel) = self.selection(node) else {
            return;
        };
        let Some((lo, hi)) = sel.band else {
            self.alert(&t!("app-set-h-markers-first"));
            return;
        };
        let (map, power) = if kind == MeasurementKind::Carrier {
            match lineage::carrier_map(&self.graph, node) {
                Ok(m) => m,
                Err(edit) => {
                    self.alert(&t!("app-sc-unmappable", edit = edit));
                    return;
                }
            }
        } else {
            (FreqMap::IDENTITY, 1)
        };
        let Selection {
            signal, start, end, ..
        } = sel;
        let handle = job::spawn(t!("app-job-measure-line"), move |ctx| {
            ctx.set_total(1);
            let stop = || ctx.is_cancelled();
            let estimate = sn_analysis::line::measure_line(&signal, start, end, lo, hi, &stop)
                .map_err(analysis_to_job)?;
            ctx.advance(1);
            Ok(Output::Sono(Box::new(SonoOutput::Measured {
                node,
                kind,
                estimate,
                map,
                power,
            })))
        });
        self.push_node_job(node, JobKind::Other, handle);
    }

    /// Auto define params on the V span.
    pub(crate) fn start_adp(&mut self, node: NodeId) {
        let Some(Selection {
            signal, start, end, ..
        }) = self.selection(node)
        else {
            return;
        };
        let handle = job::spawn(t!("app-job-adp"), move |ctx| {
            ctx.set_total(1);
            let stop = || ctx.is_cancelled();
            let candidates = sn_analysis::adp::estimate_baud(&signal, start, end, &stop)
                .map_err(analysis_to_job)?;
            ctx.advance(1);
            Ok(Output::Sono(Box::new(SonoOutput::Adp { node, candidates })))
        });
        self.push_node_job(node, JobKind::Other, handle);
    }

    /// `Save as`: ask the platform's file chooser, then write in the background.
    pub(crate) fn choose_save(
        &mut self,
        node: NodeId,
        format: SaveFormat,
        cx: &mut Context<'_, Self>,
    ) {
        let name = format!("{}.{}", file_stem(&self.name_of(node)), format.extension());
        let dir = self.chooser_dir();
        let path = cx.prompt_for_new_path(&dir, Some(&name));
        // The chooser does not block the app: the signal may be closed, or the
        // session replaced, before it answers.
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let answer = chooser_answer(path.await);
            this.update(cx, |v, cx| {
                v.save_chosen(node, format, generation, answer);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The `Save as` chooser answered for `node` in session `generation`.
    pub(crate) fn save_chosen(
        &mut self,
        node: NodeId,
        format: SaveFormat,
        generation: u64,
        answer: Result<Option<PathBuf>, String>,
    ) {
        match answer {
            Ok(None) => {}
            Ok(Some(p)) => {
                let live = generation == self.generation
                    && !self.discarded.contains(&node)
                    && self.graph.get(node).is_some();
                if live {
                    self.start_save(node, format, p);
                } else {
                    self.alert(&t!("app-signal-gone"));
                }
            }
            // The chooser could not be shown: nothing was written.
            Err(e) => self.alert(&t!(
                "app-chooser-failed",
                action = t!("sw-popup-save-as"),
                error = e
            )),
        }
    }

    /// Write `node`'s signal to `path` in `format` in the background.
    pub(crate) fn start_save(&mut self, node: NodeId, format: SaveFormat, path: PathBuf) {
        let Some(n) = self.graph.get(node) else {
            return;
        };
        let signal = n.signal.clone();
        let description = n.label.clone();
        let name = t!("app-job-saving", path = path.display().to_string());
        let handle = job::spawn(name, move |ctx| {
            ctx.set_total(1);
            let progress = |done: u64, total: u64| {
                ctx.set_total(total.max(1));
                ctx.set_done(done);
                ctx.is_cancelled()
            };
            let saved = format
                .write(&path, &signal, &description, &progress)
                .map_err(io_to_job)?;
            Ok(Output::Sono(Box::new(SonoOutput::Saved { path: saved })))
        });
        let mut job = AppJob::new(self.generation, false, handle);
        job.kind = JobKind::SaveSignal;
        job.origin = Some(node);
        self.jobs.push(job);
    }

    /// Audio is being prepared (a cancelled job winding down no longer counts).
    pub(crate) fn preparing_playback(&self) -> bool {
        self.jobs
            .iter()
            .any(|j| j.kind == JobKind::PreparePlayback && !j.cancelled)
    }

    /// Play the V-marker span (or the visible span) of window `win`.
    pub(crate) fn toggle_play(&mut self, win: u32, node: NodeId) {
        if self.sono.playback.is_playing() {
            self.sono.playback.stop();
            return;
        }
        if let Some(j) = self
            .jobs
            .iter_mut()
            .find(|j| j.kind == JobKind::PreparePlayback && !j.cancelled)
        {
            // A second press while the audio is prepared cancels it.
            j.cancel();
            self.status = t!("app-playback-cancelled");
            return;
        }
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let marked = self.ui.get(&node).and_then(|u| u.markers.time_range());
        let from_markers = marked.is_some();
        let shown = self.sono.wins.get(&win).and_then(|w| {
            w.view
                .map(|v| (v.start, v.x_to_sample(f64::from(w.plot.w))))
        });
        let Some((a, b)) = marked.or(shown) else {
            self.status = t!("app-nothing-to-play");
            return;
        };
        let name = self.name_of(node);
        let handle = job::spawn(t!("app-job-playback"), move |ctx| {
            ctx.set_total(1);
            let stop = || ctx.is_cancelled();
            let prepared = playback::prepare(&signal, a, b, &name, from_markers, &stop)?;
            ctx.checkpoint()?;
            ctx.advance(1);
            Ok(Output::Sono(Box::new(SonoOutput::Play { prepared })))
        });
        self.push_node_job(node, JobKind::PreparePlayback, handle);
    }

    /// Stop playback, or the preparation of the audio.
    pub(crate) fn stop_playback(&mut self) {
        self.sono.playback.stop();
        let mut cancelled = false;
        for j in self
            .jobs
            .iter_mut()
            .filter(|j| j.kind == JobKind::PreparePlayback && !j.cancelled)
        {
            j.cancel();
            cancelled = true;
        }
        if cancelled {
            self.status = t!("app-playback-cancelled");
        }
    }

    /// A cancelled playback: the same text as the Play-button cancel, unless a
    /// fresh playback is being prepared meanwhile.
    pub(crate) fn note_playback_cancelled(&mut self) {
        if !self.preparing_playback() {
            self.status = t!("app-playback-cancelled");
        }
    }

    /// A job of the sonogram window finished.
    pub(crate) fn on_sono_output(&mut self, out: SonoOutput) {
        match out {
            SonoOutput::Measured {
                node,
                kind,
                estimate,
                map,
                power,
            } => self.on_measured(node, &kind, &estimate, &map, power),
            SonoOutput::Adp { node, candidates } => self.on_adp(node, &candidates),
            SonoOutput::Saved { path } => {
                self.status = t!("app-saved", path = path.display().to_string());
            }
            SonoOutput::Play { prepared } => {
                self.status = match self.sono.playback.start(prepared) {
                    Ok(m) | Err(m) => m,
                };
            }
        }
    }

    /// Put a result box beside the selection of every sonogram window of `node`.
    fn add_label(&mut self, node: NodeId, text: &str) {
        let Some(fs) = self.graph.get(node).map(|n| n.signal.sample_rate()) else {
            return;
        };
        let markers = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let anchor = sono_tools::label_anchor(&markers, fs);
        for sw in self.sono.wins.values_mut().filter(|w| w.node == node) {
            sw.labels.push(FloatLabel::new(text.to_owned(), anchor));
        }
    }

    /// A line measurement finished: show SA's `Freq:` box, map it back and
    /// record it.
    fn on_measured(
        &mut self,
        node: NodeId,
        kind: &MeasurementKind,
        estimate: &LineEstimate,
        map: &FreqMap,
        power: u32,
    ) {
        self.add_label(node, &sono_tools::freq_label(estimate.frequency));
        let value = map.apply(estimate.frequency);
        let m = Measurement {
            value,
            uncertainty: Some(estimate.uncertainty * map.gain()),
            source: Some(node),
            tool: tool_id::MEASURE_LINE.into(),
        };
        let name = self.name_of(node);
        match self.measures.record(kind.clone(), m) {
            Ok(()) => {
                self.mark_dirty();
                let kind_s = kind.to_string();
                let value = with_unit(kind, value);
                let uncertainty = with_unit(kind, estimate.uncertainty * map.gain());
                let level = format!("{:.1}", estimate.level_db);
                let snr = format!("{:.1}", estimate.snr_db);
                self.status = if map.is_identity() {
                    t!(
                        "app-measured-line",
                        kind = kind_s,
                        value = value,
                        uncertainty = uncertainty,
                        node = name,
                        level = level,
                        snr = snr
                    )
                } else {
                    t!(
                        "app-measured-line-mapped",
                        kind = kind_s,
                        value = value,
                        uncertainty = uncertainty,
                        node = name,
                        level = level,
                        snr = snr,
                        line = sn_ui_core::format::rate(estimate.frequency),
                        power = power.to_string()
                    )
                };
            }
            Err(e) => {
                self.alert(&t!(
                    "app-not-recorded",
                    kind = kind.to_string(),
                    error = e.to_string()
                ));
            }
        }
    }

    /// Record a measurement with its uncertainty (half-width, value's unit).
    pub(crate) fn record_with(
        &mut self,
        kind: &MeasurementKind,
        value: f64,
        uncertainty: Option<f64>,
        node: NodeId,
        tool: &str,
    ) {
        let name = self.name_of(node);
        match record_measurement(
            &mut self.measures,
            kind,
            value,
            uncertainty,
            node,
            &name,
            tool,
        ) {
            Ok(status) => {
                self.mark_dirty();
                self.status = status;
            }
            Err(e) => self.alert(&e),
        }
    }

    /// ADP finished: record the best rate as Br and show SA's `Brate:` box.
    fn on_adp(&mut self, node: NodeId, candidates: &[BaudCandidate]) {
        if let Some(best) = candidates.first().copied() {
            let (u, tool) = adp_quality(&best);
            self.record_with(&MeasurementKind::BaudRate, best.baud, u, node, tool);
            self.add_label(node, &sono_tools::brate_label(best.baud));
        } else {
            self.alert(&t!("app-adp-none"));
        }
        // The session keeps the candidates of the latest ADP, as `sa-app` does.
        for u in self.ui.values_mut() {
            u.kept.remove(sn_session::UI_KEY_ADP);
        }
        if let Some(u) = self.ui.get_mut(&node) {
            u.kept
                .insert(sn_session::UI_KEY_ADP.to_owned(), json!(candidates));
        }
        self.mark_dirty();
    }

    /// Whether a finished job's result is dropped: cancelled playback whose
    /// audio arrived anyway (playing it would contradict "Playback cancelled"),
    /// or a cancelled WF filter that finished anyway (a newer press or Revert
    /// stopped it, so its signal must not replace the window's).
    pub(crate) fn discard_cancelled(
        kind: JobKind,
        cancelled: bool,
        result: &Result<Output, JobError>,
    ) -> bool {
        if !cancelled {
            return false;
        }
        match kind {
            JobKind::PreparePlayback => {
                matches!(result, Ok(Output::Sono(o)) if matches!(**o, SonoOutput::Play { .. }))
            }
            JobKind::WfFilter => result.is_ok(),
            _ => false,
        }
    }
}
