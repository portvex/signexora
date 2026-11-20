//! The signal windows: opening them in SA's cascade, closing them (SA's undo),
//! dropping results nothing shows any more, and their bodies.
//!
//! [`Frame::signal_body`] sends each window to its analysis view (sonogram,
//! waveform, raster, PhasePlane, demodulator, OFDM).

use gpui::{AnyElement, Context, IntoElement, Styled, Window, canvas};
use sn_core::NodeId;
use sn_ui_core::app_glue::{Confirm, join_names};
use sn_ui_core::{shortcuts, t, titles};

use super::model::{SavedWin, WinKind};
use super::{ChildKind, Frame, MsgAction};
use crate::win95::colors::{hsla, sa_colors};
use crate::win95::dialog::{Buttons, MsgBox};
use crate::win95::geom::{Edge, Rect};
use crate::win95::marks::Icon;
use crate::win95::mdi;
use crate::win95::paint;
use crate::win95::text;
use crate::win95::widget::size_of;

/// SA's title of the OFDM module window.
const OFDM_TITLE: &str = "SA-OFDM";

impl Frame {
    /// The signal and view of window `win`, if it is a signal window.
    pub(crate) fn binding(&self, win: u32) -> Option<(NodeId, WinKind)> {
        self.kinds.iter().find_map(|(id, k)| match k {
            ChildKind::Signal(n, kind) if *id == win => Some((*n, *kind)),
            _ => None,
        })
    }

    /// The window of `node` showing `kind`, if open.
    pub(crate) fn find_window(&self, node: NodeId, kind: WinKind) -> Option<u32> {
        self.kinds.iter().find_map(|(id, k)| match k {
            ChildKind::Signal(n, w) if *n == node && *w == kind => Some(*id),
            _ => None,
        })
    }

    /// Some window shows `node`.
    pub(crate) fn shows(&self, node: NodeId) -> bool {
        self.kinds
            .iter()
            .any(|(_, k)| matches!(k, ChildKind::Signal(n, _) if *n == node))
    }

    /// The signal windows, bottom to top.
    pub(crate) fn signal_windows(&self) -> Vec<(u32, NodeId, WinKind)> {
        self.mdi
            .children()
            .iter()
            .filter_map(|c| self.binding(c.id).map(|(n, k)| (c.id, n, k)))
            .collect()
    }

    /// The title bar text of a window of `kind` on `node`.
    pub(crate) fn title_for(&mut self, node: NodeId, kind: WinKind) -> String {
        let name = self.name_of(node);
        match kind {
            WinKind::Ofdm => OFDM_TITLE.to_owned(),
            WinKind::Demod => {
                let tag = self.titles.demod();
                titles::window_title(&format!("{name}{tag}"))
            }
            WinKind::Sonogram | WinKind::Waveform | WinKind::Raster | WinKind::Phase => {
                titles::window_title(&name)
            }
        }
    }

    /// Open a window of `kind` on `node`, 24 px lower right from window
    /// `caller`; a window of that kind already open on it comes to the front.
    pub(crate) fn open_window(
        &mut self,
        node: NodeId,
        kind: WinKind,
        caller: Option<u32>,
    ) -> Option<u32> {
        if self.graph.get(node).is_none() || self.discarded.contains(&node) {
            return None;
        }
        if let Some(id) = self.find_window(node, kind) {
            self.mdi.activate(id);
            return Some(id);
        }
        let title = self.title_for(node, kind);
        let (cw, ch) = kind.client_size();
        Some(self.open_child(
            &title,
            cw,
            ch,
            kind.resizable(),
            caller,
            ChildKind::Signal(node, kind),
        ))
    }

    /// Open a window at a place a session saved (relative to the client area).
    /// The place is kept inside the client area; one that is not a number
    /// (a damaged file) is skipped and the window opens in the cascade,
    /// returning the issue to report.
    pub(crate) fn restore_window(
        &mut self,
        saved: &SavedWin,
        title: &str,
    ) -> (u32, Option<String>) {
        let rect = saved.rect_in(crate::win95::style::metrics());
        let (cw, ch) = saved.kind.client_size();
        let id = self.open_child(
            title,
            cw,
            ch,
            saved.kind.resizable(),
            None,
            ChildKind::Signal(saved.node, saved.kind),
        );
        if !rect.iter().all(|v| v.is_finite()) {
            let issue = t!(
                "app-replay-session-skipped",
                what = "windows",
                error = format!("{title}: {:?}", saved.rect)
            );
            return (id, Some(issue));
        }
        let (cw, ch) = self.mdi.client;
        let r = restored_rect(rect, cw, ch);
        self.mdi.set_rect(id, r);
        (id, None)
    }

    /// The jobs (named) that closing window `win` would stop: the tool jobs
    /// of the PhasePlane, demodulator and OFDM windows, and, when closing
    /// drops the window's signal from the session, the signal's own running
    /// edits (an ACF, a resample, Find tones).
    pub(crate) fn jobs_stopped_by_closing(&self, win: u32) -> Vec<String> {
        let mut names = match self.binding(win) {
            Some((n, WinKind::Demod)) if self.demod_running(n) => {
                vec![t!("app-job-demodulator")]
            }
            Some((n, WinKind::Ofdm)) if self.ofdm_running(n) => vec![WinKind::Ofdm.label()],
            Some((_, WinKind::Phase)) => self.phase_jobs(win),
            _ => Vec::new(),
        };
        if let Some(n) = self.node_of(win)
            && self.closing_discards(win, n)
        {
            for j in self.jobs.iter().filter(|j| {
                j.origin == Some(n)
                    && !j.cancelled
                    && !j.writes_file()
                    && !matches!(
                        j.kind,
                        super::model::JobKind::Overview | super::model::JobKind::PreparePlayback
                    )
            }) {
                let name = j.handle.name().to_owned();
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        names
    }

    /// Closing window `win` drops `n` from the session: no other window shows
    /// it and nothing live was derived from it or reads it.
    fn closing_discards(&self, win: u32, n: NodeId) -> bool {
        let other = self
            .kinds
            .iter()
            .any(|(id, k)| *id != win && matches!(k, ChildKind::Signal(m, _) if *m == n));
        !other
            && !self.discarded.contains(&n)
            && !self
                .graph
                .children(n)
                .any(|c| !self.discarded.contains(&c.id))
            && !self.phase_needs(n)
    }

    /// The jobs Windows > `CloseAllWindows` would stop: tool jobs of every
    /// window and the app's own jobs bound to the session's signals.
    pub(crate) fn jobs_stopped_by_closing_all(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        let tools = self
            .signal_windows()
            .into_iter()
            .flat_map(|(w, _, _)| self.jobs_stopped_by_closing(w));
        let app_jobs = self
            .jobs
            .iter()
            .filter(|j| {
                j.node_bound
                    && !j.cancelled
                    && !matches!(
                        j.kind,
                        super::model::JobKind::Overview | super::model::JobKind::PreparePlayback
                    )
            })
            .map(|j| j.handle.name().to_owned());
        for name in tools.chain(app_jobs) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }

    /// Close window `win`; ask first when that would stop a running tool job.
    pub(crate) fn close_window(&mut self, win: u32) {
        let stopping = self.jobs_stopped_by_closing(win);
        if stopping.is_empty() {
            self.close_window_now(win);
            return;
        }
        let title = self
            .mdi
            .get(win)
            .map(|c| c.title.clone())
            .unwrap_or_default();
        let text = t!(
            "app-confirm-close-tab-question",
            count = stopping.len(),
            name = title,
            jobs = join_names(&stopping)
        );
        let mb = MsgBox::new(
            &t!("app-confirm-close-tab-title"),
            &text,
            Some(Icon::Question),
            Buttons::OkCancel,
        )
        .with_labels(vec![
            t!("app-confirm-close-and-stop"),
            t!("app-confirm-keep-tab"),
        ]);
        self.ask(mb, MsgAction::CloseWindow(win));
    }

    /// Close window `win` (SA's undo). When no other window shows its signal, a
    /// result nothing open was derived from is dropped from the session, and so
    /// are hidden signals it was made from that nothing needs any more.
    pub(crate) fn close_window_now(&mut self, win: u32) {
        let stopped = self.jobs_stopped_by_closing(win);
        let title = self
            .mdi
            .get(win)
            .map(|c| c.title.clone())
            .unwrap_or_default();
        let node = self.node_of(win);
        let phase_nodes = self.phase_closed(win);
        self.mdi.close(win);
        self.kinds.retain(|(k, _)| *k != win);
        self.texts.remove(&win);
        self.kit.focus = None;
        if !stopped.is_empty() {
            self.status = t!(
                "app-closed-tab-stopped",
                count = stopped.len(),
                name = title,
                jobs = join_names(&stopped)
            );
        }
        for n in phase_nodes {
            self.discard_unused(n);
        }
        if let Some(n) = node
            && !self.shows(n)
        {
            self.discard_unused(n);
        }
    }

    /// Drop `start` from the session if no window shows it and nothing live
    /// was derived from it, then the same for its inputs.
    pub(crate) fn discard_unused(&mut self, start: NodeId) {
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            if self.discarded.contains(&n) || self.shows(n) {
                continue;
            }
            // A PhasePlane still reading the signal (loaded, or its corrected
            // result) keeps it too.
            let needed = self
                .graph
                .children(n)
                .any(|c| !self.discarded.contains(&c.id))
                || self.phase_needs(n);
            if needed {
                // Hidden: its results keep working.
                continue;
            }
            self.discarded.insert(n);
            // Free the samples; the node stays so ids and lineage stay valid.
            self.graph.release_signal(n);
            self.mark_dirty();
            // A running file write keeps going: it holds its own copy of the
            // signal, and the user asked for the file.
            for j in self
                .jobs
                .iter_mut()
                .filter(|j| j.origin == Some(n) && !j.writes_file())
            {
                j.cancel();
            }
            self.ui.remove(&n);
            self.overviews.remove(&n);
            if let Some(node) = self.graph.get(n) {
                stack.extend(node.inputs.iter().copied());
            }
        }
    }

    /// Windows > `CloseAllWindows`: close every window and start afresh (SA's
    /// way to begin a new analysis). The session file stays as it was.
    pub(crate) fn close_all_windows(&mut self) {
        self.replace_session(
            sn_core::SignalGraph::default(),
            sn_core::MeasurementContext::default(),
            &std::collections::HashMap::new(),
            None,
        );
        self.tools_all_closed();
        self.mdi.close_all();
        self.kinds.clear();
        self.texts.clear();
        self.status = String::new();
    }

    /// Ask before an action that would replace or discard unsaved work.
    pub(crate) fn request(&mut self, c: Confirm, cx: &mut Context<'_, Self>) {
        if self.must_ask(c) {
            self.confirm(c);
        } else {
            self.do_confirmed(c, cx);
        }
    }

    /// Bring the next (or previous) signal of the session forward, opening its
    /// sonogram window if none is open (a hidden signal comes back this way).
    pub(crate) fn step_signal(&mut self, forward: bool) {
        let order = self.live_nodes();
        let current = self
            .active_node()
            .and_then(|s| order.iter().position(|n| *n == s));
        let Some(n) = shortcuts::step(order.len(), current, forward)
            .and_then(|i| order.get(i))
            .copied()
        else {
            return;
        };
        let existing = self
            .signal_windows()
            .into_iter()
            .rev()
            .find(|(_, node, _)| *node == n)
            .map(|(w, _, _)| w);
        if let Some(w) = existing {
            self.mdi.activate(w);
        } else {
            let caller = self.mdi.active();
            self.open_window(n, WinKind::Sonogram, caller);
        }
        self.status = t!("keys-selected-signal", signal = self.name_of(n));
    }

    /// The signal window under the pointer, if any.
    pub(crate) fn hovered_window(&self) -> Option<u32> {
        let (px, py) = self.pointer?;
        let (x, y) = (px - 2, py - crate::win95::menu_view::bar_h() - 2);
        let (cw, ch) = self.mdi.client;
        if x < 0 || y < 0 || x >= cw || y >= ch {
            return None;
        }
        self.mdi
            .children()
            .iter()
            .rev()
            .find(|c| self.mdi.shown_rect(c.id).is_some_and(|r| r.contains(x, y)))
            .map(|c| c.id)
            .filter(|id| self.binding(*id).is_some())
    }

    /// The status bar's `SamplingRate = ...` and source path: the window under
    /// the pointer, else the active one; kept after the last window closes as
    /// SA does, cleared when no window is open.
    pub(crate) fn update_source(&mut self) {
        let node = self
            .hovered_window()
            .or_else(|| self.mdi.active().filter(|id| self.binding(*id).is_some()))
            .and_then(|w| self.node_of(w));
        if let Some(src) = node.and_then(|n| self.source_of(n)) {
            self.last_source = Some(src);
        } else if self.signal_windows().is_empty() {
            self.last_source = None;
        }
    }

    /// The client area of signal window `win`: its analysis view, or a
    /// notice when its signal is gone.
    pub(crate) fn signal_body(
        &mut self,
        win: u32,
        node: NodeId,
        kind: WinKind,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        if kind == WinKind::Sonogram && self.graph.get(node).is_some() {
            return self.sono_body(win, node, window, cx);
        }
        if let Some(el) = self.wf_signal_body(win, node, kind, window, cx) {
            return el;
        }
        if kind == WinKind::Phase && self.graph.get(node).is_some() {
            return self.phase_body(win, node, window, cx);
        }
        if kind == WinKind::Demod && self.graph.get(node).is_some() {
            return self.demod_body(win, node, window, cx);
        }
        if kind == WinKind::Ofdm && self.graph.get(node).is_some() {
            return self.ofdm_body(win, node, window, cx);
        }
        gone_body(cx)
    }
}

/// The body of a window whose signal is gone: SA's black plot area with one
/// line saying so.
fn gone_body(cx: &mut Context<'_, Frame>) -> AnyElement {
    let line = t!("app-tab-signal-gone");
    let sa = sa_colors(cx);
    canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let o = b.origin;
            let r = Rect::new(0, 0, w, h);
            paint::fill(window, o, r, hsla(sa.plot_bg));
            let s = crate::win95::colors::scheme(cx);
            paint::edge(window, o, r, Edge::ThinSunken, &s);
            text::draw(
                window,
                cx,
                paint::at(o, 8, 8),
                &line,
                true,
                hsla(sa.plot_text),
            );
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}

/// A saved window rectangle `[x, y, w, h]` (finite) as a rectangle inside a
/// client area of `cw` x `ch`: the size between the drag minimum and the
/// client area, the place inside it.
fn restored_rect([x, y, w, h]: [f32; 4], cw: i32, ch: i32) -> Rect {
    let (min_w, min_h) = mdi::min_track();
    let (max_w, max_h) = (cw.max(min_w), ch.max(min_h));
    // `as` saturates for f32 to i32, and the clamps bound the rest.
    let w = (w.round() as i32).clamp(min_w, max_w);
    let h = (h.round() as i32).clamp(min_h, max_h);
    let x = (x.round() as i32).clamp(0, (max_w - w).max(0));
    let y = (y.round() as i32).clamp(0, (max_h - h).max(0));
    Rect::new(x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_rects_stay_inside_the_client_area() {
        let r = restored_rect([10.0, 20.0, 300.0, 200.0], 800, 600);
        assert_eq!(r, Rect::new(10, 20, 300, 200));
        let r = restored_rect([1e30, -1e30, 1e30, 1e6], 800, 600);
        assert_eq!(r, Rect::new(0, 0, 800, 600));
        let r = restored_rect([0.0, 0.0, 1.0, 1.0], 800, 600);
        assert_eq!((r.w, r.h), mdi::min_track());
        // A tiny (or negative) client area must not overflow.
        let r = restored_rect([5.0, 5.0, f32::MAX, f32::MAX], -50, -50);
        assert_eq!(r, Rect::new(0, 0, mdi::min_track().0, mdi::min_track().1));
    }
}
