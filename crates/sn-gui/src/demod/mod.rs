//! SA's FSK-2 / universal MFSK demodulator window (`docs/SA_UI_SPEC.md`
//! section 8, `(f1)` windows) on the Windows 95 kit, as `sa-app`'s
//! `demod_window.rs` draws it: the top bar (`RRC` and its cutoff and alpha,
//! Br with its spin pairs, `A`, `<= Exactly` / `<= Roughly`, Levels and
//! `Raw`), the black plot with the level zones and its vertical scroll bar,
//! the cyan fixed-font text panel and the bottom bar (navigation, vertical
//! scale, progress and Stop, filter, revert, `D`).
//!
//! The module is a child of `app` (declared there with a path), so it extends
//! [`Frame`] directly. The state and jobs are `sn_ui_core::demod`'s, kept per
//! signal as in `sa-app`: closing the window frees the track and stops the
//! work, the settings and the output stay.
//!
//! - `top.rs`, `bottom.rs`: the two bars.
//! - `plot.rs`: the plot, its markers and scroll bar.
//! - `panel.rs`: the text panel.
//! - `popup.rs`: the popup menus and their commands.
//! - `ctl.rs`, `colors.rs`: controls and colours shared with `SA-OFDM`.

mod bottom;
pub(crate) mod colors;
pub(crate) mod ctl;
mod panel;
mod plot;
mod popup;
#[cfg(test)]
mod tests;
mod top;

use std::collections::HashMap;

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use sn_core::NodeId;
use sn_ui_core::demod::{DemodAction, DemodState, Prefill};

pub(crate) use popup::{DM_FIRST, DM_LAST};
pub(crate) use top::tip_area;

use super::Frame;
use crate::app::model::WinKind;
use crate::win95::edit::{EditState, SpinState};
use crate::win95::geom::Rect;
use crate::win95::scrollbar::{self, ScrollState, Wheel};

/// Height of the top bar (two rows of controls).
pub(crate) const TOP_H: i32 = 50;
/// Height of the bottom bar.
pub(crate) const BOT_H: i32 = 28;
/// Width of the text panel.
pub(crate) const TEXT_W: i32 = 200;
/// Client size of a new demodulator window.
pub(crate) const CLIENT: (i32, i32) = (770, 300);

/// Where the parts of a window with a client area `cw` x `ch` go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub(crate) top: Rect,
    pub(crate) plot: Rect,
    /// The plot's vertical scroll bar.
    pub(crate) vbar: Rect,
    pub(crate) text: Rect,
    pub(crate) bottom: Rect,
}

impl Layout {
    pub(crate) fn new(cw: i32, ch: i32) -> Self {
        let mid = (ch - TOP_H - BOT_H).max(0);
        let text_w = TEXT_W.min((cw - scrollbar::THICKNESS).max(0));
        let plot_w = (cw - text_w - scrollbar::THICKNESS).max(0);
        Self {
            top: Rect::new(0, 0, cw, TOP_H),
            plot: Rect::new(0, TOP_H, plot_w, mid),
            vbar: Rect::new(plot_w, TOP_H, scrollbar::THICKNESS, mid),
            text: Rect::new(plot_w + scrollbar::THICKNESS, TOP_H, text_w, mid),
            bottom: Rect::new(0, TOP_H + mid, cw, BOT_H),
        }
    }
}

/// A field that applies its text when it loses the focus (or on Enter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Field {
    Cutoff,
    Alpha,
    Br,
    Levels,
    Page,
}

impl Field {
    const ALL: [Self; 5] = [
        Self::Cutoff,
        Self::Alpha,
        Self::Br,
        Self::Levels,
        Self::Page,
    ];

    /// The control id of the field in window `win`.
    pub(crate) fn id(self, win: u32) -> String {
        let name = match self {
            Self::Cutoff => "cutoff",
            Self::Alpha => "alpha",
            Self::Br => "br",
            Self::Levels => "levels",
            Self::Page => "page",
        };
        format!("dm{win}.{name}")
    }
}

/// One signal's demodulator: `sn_ui_core`'s state and the window's controls.
pub(crate) struct DemodWin {
    pub(crate) st: DemodState,
    pub(crate) cutoff: EditState,
    pub(crate) alpha: EditState,
    pub(crate) br: EditState,
    pub(crate) levels: SpinState,
    pub(crate) page: EditState,
    pub(crate) alpha_bar: ScrollState,
    pub(crate) exact_bar: ScrollState,
    pub(crate) rough_bar: ScrollState,
    /// The plot's vertical scroll bar (tenths of the shown height, see
    /// `plot::vbar_state`).
    pub(crate) vbar: ScrollState,
    /// The text panel's scroll bar (lines).
    pub(crate) text_bar: ScrollState,
    /// The wheel's part line over the text panel.
    pub(crate) text_wheel: Wheel,
    /// The plot at the last layout (client coordinates).
    pub(crate) plot: Rect,
    /// The field that had the focus at the last render.
    focused: Option<Field>,
    /// The state's text last shown in each field ([`Field::ALL`] order): a
    /// focused field whose state text changed from outside (a spin pair, a
    /// slider, the cutoff list, the page buttons) shows the new text, so
    /// leaving the field does not apply its old text again.
    shown: [String; 5],
    /// The window was closed and its work freed.
    released: bool,
}

impl std::fmt::Debug for DemodWin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DemodWin")
            .field("st", &self.st)
            .finish_non_exhaustive()
    }
}

impl DemodWin {
    fn new() -> Self {
        Self {
            st: DemodState::default(),
            cutoff: EditState::default(),
            alpha: EditState::default(),
            br: EditState::default(),
            levels: SpinState::new(2, 2, sn_ui_core::demod::MAX_LEVELS as i64, 1),
            page: EditState::default(),
            alpha_bar: ScrollState::default(),
            exact_bar: ScrollState::default(),
            rough_bar: ScrollState::default(),
            vbar: ScrollState::default(),
            text_bar: ScrollState::default(),
            text_wheel: Wheel::default(),
            plot: Rect::default(),
            focused: None,
            shown: Default::default(),
            released: false,
        }
    }

    /// The edit state of a field.
    fn edit_of(&mut self, f: Field) -> &mut EditState {
        match f {
            Field::Cutoff => &mut self.cutoff,
            Field::Alpha => &mut self.alpha,
            Field::Br => &mut self.br,
            Field::Levels => &mut self.levels.edit,
            Field::Page => &mut self.page,
        }
    }

    /// Show the state's texts in the fields. The focused field keeps what is
    /// typed in it, unless its value was changed from outside since it was
    /// last shown.
    fn sync_fields(&mut self, focus: Option<Field>) {
        for (i, f) in Field::ALL.into_iter().enumerate() {
            let text = match f {
                Field::Cutoff => self.st.cutoff.clone(),
                Field::Alpha => self.st.alpha_text.clone(),
                Field::Br => self.st.br_text.clone(),
                Field::Levels => self.st.levels_text.clone(),
                Field::Page => self.st.page_text.clone(),
            };
            let outside = self.shown.get(i).is_some_and(|s| *s != text);
            if Some(f) == focus && !outside {
                continue;
            }
            let e = self.edit_of(f);
            if e.text != text {
                e.set_text(&text);
                if Some(f) == focus {
                    e.select_all();
                }
            }
            if let Some(s) = self.shown.get_mut(i) {
                *s = text;
            }
        }
        self.levels.value = self.st.levels as i64;
    }

    /// Apply field `f`'s text (it lost the focus, or Enter). Returns an error
    /// to show (a refused Br on Enter).
    pub(crate) fn apply(&mut self, f: Field, enter: bool) -> Option<String> {
        let text = self.edit_of(f).text.clone();
        match f {
            Field::Cutoff => text.clone_into(&mut self.st.cutoff),
            Field::Alpha => self.st.type_alpha(&text),
            Field::Br => {
                self.st.type_br(&text);
                if enter {
                    return self.st.br_refused.clone();
                }
            }
            Field::Levels => self.st.type_levels(&text),
            Field::Page => {
                if let Ok(p) = text.trim().parse::<i64>() {
                    self.st.go_to_page(p.saturating_sub(1));
                } else {
                    self.st.go_to_page(self.st.page as i64);
                }
            }
        }
        None
    }
}

/// The demodulators of the frame, per signal.
#[derive(Default)]
pub(crate) struct DemodHub {
    pub(crate) nodes: HashMap<NodeId, DemodWin>,
    /// The window whose popup menu is open.
    pub(crate) popup_win: Option<u32>,
}

impl std::fmt::Debug for DemodHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DemodHub")
            .field("nodes", &self.nodes.len())
            .finish_non_exhaustive()
    }
}

/// The tones found in WF (Sum) and the size of the grid Count freqs found,
/// from a signal's kept UI values (as sessions store them).
fn kept_tones(kept: &serde_json::Map<String, serde_json::Value>) -> (Vec<f64>, Option<usize>) {
    let tones: Vec<f64> = kept
        .get(sn_session::UI_KEY_TONES)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let grid = kept
        .get(sn_session::UI_KEY_TONE_GRID)
        .and_then(|g| g.get("size"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok());
    (tones, grid)
}

impl Frame {
    /// Open the demodulator window on `node` (24 px lower right from window
    /// `caller`). With `span`, the demodulator works on those samples: the
    /// signal's V markers are set to them first (WF's diamond button passes
    /// the fragment between its V markers, as SA does).
    pub(crate) fn open_demod(
        &mut self,
        node: NodeId,
        span: Option<(usize, usize)>,
        caller: Option<u32>,
    ) -> Option<u32> {
        if let Some((a, b)) = span
            && let Some(u) = self.ui.get_mut(&node)
        {
            u.markers.v = Some((a as f64, b as f64));
        }
        self.open_window(node, WinKind::Demod, caller)
    }

    /// The tones (WF Sum) and Count freqs grid the demodulator starts from:
    /// `node`'s own, else the latest any signal has (as `sa-app`'s last tones).
    pub(crate) fn demod_tones(&self, node: NodeId) -> (Vec<f64>, Option<usize>) {
        let own = self.ui.get(&node).map(|u| kept_tones(&u.kept));
        if let Some((t, g)) = own.filter(|(t, g)| !t.is_empty() || g.is_some()) {
            return (t, g);
        }
        self.live_nodes()
            .into_iter()
            .rev()
            .filter_map(|n| self.ui.get(&n).map(|u| kept_tones(&u.kept)))
            .find(|(t, _)| !t.is_empty())
            .unwrap_or_default()
    }

    /// The demodulator of `node`, made on first use.
    pub(crate) fn demod_win(&mut self, node: NodeId) -> &mut DemodWin {
        self.demod.nodes.entry(node).or_insert_with(DemodWin::new)
    }

    /// Work running in a demodulator.
    pub(crate) fn demod_busy(&self) -> bool {
        self.demod.nodes.values().any(|d| d.st.is_running())
    }

    /// Whether the demodulator of signal `node` runs work (closing its window
    /// stops it).
    pub(crate) fn demod_running(&self, node: NodeId) -> bool {
        self.demod
            .nodes
            .get(&node)
            .is_some_and(|d| d.st.is_running())
    }

    /// Free the work of demodulators whose window closed; forget those of
    /// signals that left the session.
    pub(crate) fn demod_prune(&mut self) {
        let open: Vec<NodeId> = self
            .signal_windows()
            .into_iter()
            .filter(|(_, _, k)| *k == WinKind::Demod)
            .map(|(_, n, _)| n)
            .collect();
        let live = self.live_nodes();
        self.demod.nodes.retain(|n, _| live.contains(n));
        for (n, d) in &mut self.demod.nodes {
            if open.contains(n) {
                d.released = false;
            } else if !d.released {
                d.st.release();
                d.released = true;
            }
        }
    }

    /// The body of demodulator window `win` showing `node`.
    pub(crate) fn demod_body(
        &mut self,
        win: u32,
        node: NodeId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        self.demod_update(win, node, cx);
        let (cw, ch) = self.client_of(win);
        let l = Layout::new(cw, ch);
        self.demod_win(node).plot = l.plot;
        let mut root = div().relative().size_full();
        for el in self.demod_top(win, node, l.top, cx) {
            root = root.child(el);
        }
        for el in self.demod_plot(win, node, l, window, cx) {
            root = root.child(el);
        }
        for el in self.demod_panel(win, node, l.text, cx) {
            root = root.child(el);
        }
        for el in self.demod_bottom(win, node, l.bottom, cx) {
            root = root.child(el);
        }
        root.into_any_element()
    }

    /// Pre-fill on the first opening, apply fields that lost the focus, run
    /// the work the controls ask for and show what it reports.
    fn demod_update(&mut self, win: u32, node: NodeId, cx: &mut Context<'_, Self>) {
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let br = self
            .measured(&sn_core::MeasurementKind::BaudRate)
            .map(|m| m.value);
        let (tones, grid) = self.demod_tones(node);
        let span = self
            .ui
            .get(&node)
            .map(|u| u.markers)
            .unwrap_or_default()
            .sample_span(signal.len());
        let focus_now = Field::ALL
            .into_iter()
            .find(|f| self.kit.has_focus(&f.id(win)));
        let d = self.demod_win(node);
        d.st.init(&Prefill {
            br,
            tones: tones.clone(),
            grid,
        });
        if let Some(f) = d.focused.filter(|f| Some(*f) != focus_now) {
            d.apply(f, false);
        }
        d.focused = focus_now;
        let (action, busy) = d.st.update(&signal, span, &tones);
        d.sync_fields(focus_now);
        self.sync_demod_bars(node);
        if let Some(DemodAction::Error(e)) = action {
            self.alert(&e);
        }
        if busy {
            self.ensure_poll(cx);
        }
    }

    /// Show `node`'s demodulator error `e` (a refused value, a failed job).
    pub(crate) fn demod_error(&mut self, e: Option<String>) {
        if let Some(e) = e {
            self.alert(&e);
        }
    }
}

#[cfg(debug_assertions)]
impl Frame {
    /// Debug builds: the `demod` scene of `SA_GUI_SCENE`: the generated MFSK-8
    /// recording in a demodulator window with 8 levels, the result decided and
    /// output to the text panel.
    pub(crate) fn demod_scene(&mut self) {
        let Some(signal) = super::sono::test_signal() else {
            return;
        };
        let op = sn_core::Operation::new("load", serde_json::json!({"path": "/demo/mfsk8.wav"}));
        let raw = self.on_loaded("mfsk8.wav".into(), op, signal, &[]);
        // SA's demodulator reads a frequency track: the phase detector's output.
        let Some(source) = self.graph.get(raw).map(|n| n.signal.clone()) else {
            return;
        };
        let edit = sn_dsp::ops::Edit::PhaseDetector;
        let Ok(track) = edit.run(&source, &|| false) else {
            return;
        };
        let caller = self.find_window(raw, WinKind::Sonogram);
        let Some(node) = self.on_derived(raw, &edit, track.clone(), caller) else {
            return;
        };
        let signal = track;
        let caller = self.find_window(node, WinKind::Sonogram);
        let Some(_) = self.open_demod(node, None, caller) else {
            return;
        };
        let d = self.demod_win(node);
        d.st.init(&Prefill {
            br: Some(75.0),
            tones: (0..8).map(|i| 1062.5 + 125.0 * f64::from(i)).collect(),
            grid: Some(8),
        });
        let span = (0, signal.len());
        for _ in 0..4000 {
            let (_, busy) = d.st.update(&signal, span, &[]);
            if !busy && d.st.result().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        d.st.output().ok();
    }
}
