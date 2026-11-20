//! SA's OFDM module window, `SA-OFDM` (`docs/SA_UI_SPEC.md` section 10, the
//! final 6.2.6.5 layout) on the Windows 95 kit, as `sa-app`'s
//! `ofdm_window.rs` draws it: the top bar (`Min Br-LS`, `Max Br-LS`,
//! `Total LS`, `Current LS`, `Survey`, `List of K`), the Survey spectrum or
//! the correlation triangle over the LU spectrum on the left with their
//! scroll bars, the constellations, the results list (or List of K) and the
//! controls on the right (`Symbols`, `Scale FFT`, `Shift Frequency`, Br,
//! `ADP`, `Show`, `Find CT`), and the status bar with the signal's name.
//!
//! The module is a child of `app` (declared there with a path), so it extends
//! [`Frame`] directly. The state and jobs are `sn_ui_core::ofdm`'s, kept per
//! signal as in `sa-app`.
//!
//! - `top.rs`: the top bar and the status bar.
//! - `plots.rs`: the plots and their scroll bars.
//! - `right.rs`: the constellations, the lists and the controls.
//! - `popup.rs`: the popup menus and their commands.

mod plots;
mod popup;
mod right;
#[cfg(test)]
mod tests;
mod top;

use std::collections::HashMap;

use gpui::{AnyElement, Context, IntoElement, Keystroke, ParentElement, Styled, Window, div};
use sn_core::{MeasurementKind, NodeId};
use sn_dsp::ops::Edit;
use sn_ui_core::ofdm::{Model, OfdmAction, OfdmState};
use sn_ui_core::t;

pub(crate) use popup::{OF_FIRST, OF_LAST};

use super::Frame;
use crate::app::model::WinKind;
use crate::win95::combo::ComboState;
use crate::win95::edit::{EditState, SpinState};
use crate::win95::geom::Rect;
use crate::win95::scrollbar::{ScrollState, THICKNESS};

/// Height of the top bar.
pub(crate) const TOP_H: i32 = 28;
/// Height of the status bar at the bottom of the form.
pub(crate) const STATUS_H: i32 = 22;
/// Width of the right column.
pub(crate) const RIGHT: i32 = 330;
/// Height of the controls at the bottom of the right column.
pub(crate) const CONTROLS_H: i32 = 100;
/// Client size of a new `SA-OFDM` window.
pub(crate) const CLIENT: (i32, i32) = (800, 460);

/// Where the parts of a window with a client area `cw` x `ch` go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub(crate) top: Rect,
    /// The plots (Survey, or the triangle over the spectrum).
    pub(crate) plots: Rect,
    /// The gain scroll bar right of the plots.
    pub(crate) gain: Rect,
    /// The cyan position scroll bar and the lavender pan scroll bar.
    pub(crate) pos: Rect,
    pub(crate) pan: Rect,
    /// The right column.
    pub(crate) right: Rect,
    pub(crate) status: Rect,
}

impl Layout {
    pub(crate) fn new(cw: i32, ch: i32) -> Self {
        let mid_h = (ch - TOP_H - STATUS_H).max(0);
        let lw = (cw - RIGHT - 4).max(THICKNESS);
        let plots_h = (mid_h - 2 * THICKNESS).max(0);
        Self {
            top: Rect::new(0, 0, cw, TOP_H),
            plots: Rect::new(0, TOP_H, lw - THICKNESS, plots_h),
            gain: Rect::new(lw - THICKNESS, TOP_H, THICKNESS, plots_h),
            pos: Rect::new(0, TOP_H + plots_h, lw, THICKNESS),
            pan: Rect::new(0, TOP_H + plots_h + THICKNESS, lw, THICKNESS),
            right: Rect::new(lw + 4, TOP_H, (cw - lw - 6).max(0), mid_h),
            status: Rect::new(2, ch - STATUS_H + 2, (cw - 4).max(0), STATUS_H - 4),
        }
    }

    /// The triangle (upper 45 %) and the LU spectrum (the rest).
    pub(crate) fn split(&self) -> (Rect, Rect) {
        let p = self.plots;
        let top_h = (f64::from(p.h) * 0.45) as i32;
        (
            Rect::new(p.x, p.y, p.w, top_h),
            Rect::new(p.x, p.y + top_h, p.w, p.h - top_h),
        )
    }
}

/// A field applied when it loses the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OField {
    Min,
    Max,
    Symbols,
    Scale,
    Shift,
    Br,
    Each,
    Start,
}

impl OField {
    const ALL: [Self; 8] = [
        Self::Min,
        Self::Max,
        Self::Symbols,
        Self::Scale,
        Self::Shift,
        Self::Br,
        Self::Each,
        Self::Start,
    ];

    /// The control id of the field in window `win`.
    pub(crate) fn id(self, win: u32) -> String {
        let name = match self {
            Self::Min => "min",
            Self::Max => "max",
            Self::Symbols => "symbols",
            Self::Scale => "scale",
            Self::Shift => "shift",
            Self::Br => "br",
            Self::Each => "each",
            Self::Start => "start",
        };
        format!("of{win}.{name}")
    }
}

/// One signal's OFDM module: `sn_ui_core`'s state and the window's controls.
pub(crate) struct OfdmWin {
    pub(crate) st: OfdmState,
    pub(crate) min: SpinState,
    pub(crate) max: SpinState,
    pub(crate) each: SpinState,
    pub(crate) start: SpinState,
    pub(crate) symbols: EditState,
    pub(crate) scale: EditState,
    pub(crate) shift: EditState,
    pub(crate) br: EditState,
    pub(crate) model: ComboState,
    pub(crate) symbols_bar: ScrollState,
    pub(crate) scale_bar: ScrollState,
    pub(crate) gain_bar: ScrollState,
    pub(crate) pos_bar: ScrollState,
    pub(crate) pan_bar: ScrollState,
    pub(crate) list_bar: ScrollState,
    /// The wheel's part line over the results list.
    pub(crate) list_wheel: crate::win95::scrollbar::Wheel,
    /// Samples (Survey) per position scroll bar unit.
    pub(crate) pos_unit: f64,
    /// Pixels per pan scroll bar unit.
    pub(crate) pan_unit: f64,
    /// The largest pan (pixels): the last full view of the spectrum.
    pub(crate) pan_max: f64,
    /// The last Survey position (samples) with a full FFT in the selection.
    pub(crate) survey_max: f64,
    /// The sample rate of the List of K line the popup menu was opened on.
    pub(crate) k_rate: Option<f64>,
    focused: Option<OField>,
    released: bool,
}

impl std::fmt::Debug for OfdmWin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OfdmWin")
            .field("st", &self.st)
            .finish_non_exhaustive()
    }
}

impl OfdmWin {
    fn new() -> Self {
        let labels: Vec<String> = Model::ALL.iter().map(|m| m.label()).collect();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        Self {
            st: OfdmState::default(),
            min: SpinState::new(20, 1, MAX_BR_LS - 1, 1),
            max: SpinState::new(400, 2, MAX_BR_LS, 1),
            each: SpinState::new(1, 1, 1000, 1),
            start: SpinState::new(0, 0, 1_000_000, 1),
            symbols: EditState::default(),
            scale: EditState::default(),
            shift: EditState::default(),
            br: EditState::default(),
            model: ComboState::new(&refs, Some(1)),
            symbols_bar: ScrollState::default(),
            scale_bar: ScrollState::default(),
            gain_bar: ScrollState::default(),
            pos_bar: ScrollState::default(),
            pan_bar: ScrollState::default(),
            list_bar: ScrollState::default(),
            list_wheel: crate::win95::scrollbar::Wheel::default(),
            pos_unit: 1.0,
            pan_unit: 1.0,
            pan_max: 0.0,
            survey_max: 0.0,
            k_rate: None,
            focused: None,
            released: false,
        }
    }

    /// The edit state of a field.
    fn edit_of(&mut self, f: OField) -> &mut EditState {
        match f {
            OField::Min => &mut self.min.edit,
            OField::Max => &mut self.max.edit,
            OField::Each => &mut self.each.edit,
            OField::Start => &mut self.start.edit,
            OField::Symbols => &mut self.symbols,
            OField::Scale => &mut self.scale,
            OField::Shift => &mut self.shift,
            OField::Br => &mut self.br,
        }
    }

    /// The text the state gives field `f`.
    fn state_text(&self, f: OField) -> String {
        let st = &self.st;
        match f {
            OField::Min => st.min_text.clone(),
            OField::Max => st.max_text.clone(),
            OField::Each => st.each.to_string(),
            OField::Start => st.start_plus.to_string(),
            OField::Symbols => st.symbols.to_string(),
            OField::Scale => st.scale_fft.to_string(),
            OField::Shift => st.shift_text.clone(),
            OField::Br => st.br_text.clone(),
        }
    }

    /// Show the state's values in the fields that do not have the focus.
    fn sync_fields(&mut self, focus: Option<OField>) {
        for f in OField::ALL {
            if Some(f) == focus {
                continue;
            }
            let text = self.state_text(f);
            let e = self.edit_of(f);
            if e.text != text {
                e.set_text(&text);
            }
        }
        let st = &self.st;
        self.min.value = i64::from(st.min_br);
        self.max.value = i64::from(st.max_br);
        self.each.value = st.each as i64;
        self.start.value = st.start_plus as i64;
        let m = Model::ALL.iter().position(|m| *m == st.model);
        self.model.selected = m;
    }

    /// Apply field `f`'s text (it lost the focus, or Enter). Returns an error
    /// to show on Enter.
    pub(crate) fn apply(&mut self, f: OField, enter: bool) -> Option<String> {
        let text = self.edit_of(f).text.clone();
        let int = text.trim().parse::<i64>().ok();
        if int.is_none()
            && let Some((key, lo, hi)) = int_field(f)
        {
            // Not a whole number: the field shows the value in use again; on
            // Enter the user is told why, as Shift and Br do.
            let shown = self.state_text(f);
            self.edit_of(f).set_text(&shown);
            return enter.then(|| {
                t!(
                    "edits-field-invalid",
                    field = t!(key),
                    text = text.trim().to_owned(),
                    range = t!("edits-range", lo = lo.to_string(), hi = hi.to_string())
                )
            });
        }
        let st = &mut self.st;
        match f {
            OField::Min | OField::Max => {
                let (mut lo, mut hi) = (i64::from(st.min_br), i64::from(st.max_br));
                let v = int.unwrap_or(if f == OField::Min { lo } else { hi });
                // Min leaves room for Max = Min + 1 inside the spin's range.
                if f == OField::Min {
                    lo = v.clamp(1, MAX_BR_LS - 1);
                } else {
                    hi = v.clamp(2, MAX_BR_LS);
                }
                st.set_bounds(lo as u32, hi as u32);
            }
            OField::Each => st.each = int.unwrap_or(st.each as i64).clamp(1, 1000) as usize,
            OField::Start => {
                st.start_plus = int.unwrap_or(st.start_plus as i64).clamp(0, 1_000_000) as usize;
            }
            OField::Symbols => {
                st.symbols = int.unwrap_or(st.symbols as i64).clamp(2, 500) as usize;
            }
            OField::Scale => {
                st.scale_fft = int.unwrap_or(st.scale_fft as i64).clamp(1, 40) as usize;
            }
            OField::Shift => {
                let e = st.type_shift(&text).err();
                return e.filter(|_| enter);
            }
            OField::Br => {
                let e = st.type_br(&text).err();
                return e.filter(|_| enter);
            }
        }
        None
    }
}

/// Highest Max Br-LS (the spins' limit).
const MAX_BR_LS: i64 = 1_000_000;

/// The label and limits of a field that takes a whole number.
fn int_field(f: OField) -> Option<(&'static str, i64, i64)> {
    Some(match f {
        OField::Min => ("ofdm-min-br", 1, MAX_BR_LS - 1),
        OField::Max => ("ofdm-max-br", 2, MAX_BR_LS),
        OField::Each => ("ofdm-each", 1, 1000),
        OField::Start => ("ofdm-start-plus", 0, 1_000_000),
        OField::Symbols => ("ofdm-symbols", 2, 500),
        OField::Scale => ("ofdm-scale-fft", 1, 40),
        OField::Shift | OField::Br => return None,
    })
}

/// The OFDM modules of the frame, per signal.
#[derive(Default)]
pub(crate) struct OfdmHub {
    pub(crate) nodes: HashMap<NodeId, OfdmWin>,
    /// The window whose popup menu is open.
    pub(crate) popup_win: Option<u32>,
}

impl std::fmt::Debug for OfdmHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OfdmHub")
            .field("nodes", &self.nodes.len())
            .finish_non_exhaustive()
    }
}

impl Frame {
    /// The OFDM module of `node`, made on first use.
    pub(crate) fn ofdm_win(&mut self, node: NodeId) -> &mut OfdmWin {
        self.ofdm.nodes.entry(node).or_insert_with(OfdmWin::new)
    }

    /// The samples the OFDM module of `node` works on (a signal `len` long):
    /// the span between the signal's V markers, or all of it. Find CT,
    /// Survey and Total LS all use it, so they agree on the span.
    pub(crate) fn ofdm_span(&self, node: NodeId, len: usize) -> (usize, usize) {
        self.ui
            .get(&node)
            .map(|u| u.markers)
            .unwrap_or_default()
            .sample_span(len)
    }

    /// Work running (or Show playing) in an OFDM module.
    pub(crate) fn ofdm_busy(&self) -> bool {
        self.ofdm
            .nodes
            .values()
            .any(|o| o.st.is_running() || o.st.playing)
    }

    /// Whether the OFDM module of `node` runs a job (closing it stops it).
    pub(crate) fn ofdm_running(&self, node: NodeId) -> bool {
        self.ofdm
            .nodes
            .get(&node)
            .is_some_and(|o| o.st.is_running())
    }

    /// Free the baseband and stop the work of modules whose window closed;
    /// forget those of signals that left the session.
    pub(crate) fn ofdm_prune(&mut self) {
        let open: Vec<NodeId> = self
            .signal_windows()
            .into_iter()
            .filter(|(_, _, k)| *k == WinKind::Ofdm)
            .map(|(_, n, _)| n)
            .collect();
        let live = self.live_nodes();
        self.ofdm.nodes.retain(|n, _| live.contains(n));
        for (n, o) in &mut self.ofdm.nodes {
            if open.contains(n) {
                o.released = false;
            } else if !o.released {
                o.st.release();
                o.released = true;
            }
        }
    }

    /// The body of `SA-OFDM` window `win` showing `node`.
    pub(crate) fn ofdm_body(
        &mut self,
        win: u32,
        node: NodeId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        self.ofdm_update(win, node, cx);
        let (cw, ch) = self.client_of(win);
        let l = Layout::new(cw, ch);
        let mut root = div().relative().size_full();
        for el in self.ofdm_top(win, node, l.top, window, cx) {
            root = root.child(el);
        }
        for el in self.ofdm_plots(win, node, l, cx) {
            root = root.child(el);
        }
        for el in self.ofdm_right(win, node, l.right, window, cx) {
            root = root.child(el);
        }
        root = root.child(self.ofdm_status(node, l.status, cx));
        root.into_any_element()
    }

    /// Apply fields that lost the focus, collect finished jobs, advance Show
    /// and carry out what the module asks for.
    fn ofdm_update(&mut self, win: u32, node: NodeId, cx: &mut Context<'_, Self>) {
        let Some(fs) = self.graph.get(node).map(|n| n.signal.sample_rate()) else {
            return;
        };
        let focus_now = OField::ALL
            .into_iter()
            .find(|f| self.kit.has_focus(&f.id(win)));
        let o = self.ofdm_win(node);
        o.st.init(fs);
        if let Some(f) = o.focused.filter(|f| Some(*f) != focus_now) {
            o.apply(f, false);
        }
        o.focused = focus_now;
        let action = o.st.poll();
        o.st.tick();
        o.sync_fields(focus_now);
        if let Some(a) = action {
            self.ofdm_action(win, node, a);
        }
        if self.ofdm_busy() {
            self.ensure_poll(cx);
        }
    }

    /// Carry out what the OFDM module asked for.
    pub(crate) fn ofdm_action(&mut self, win: u32, node: NodeId, a: OfdmAction) {
        match a {
            OfdmAction::RecordBr(br) => {
                self.record_with(
                    &MeasurementKind::BaudRate,
                    br,
                    None,
                    node,
                    sn_ui_core::app_glue::tool_id::OFDM_ADP,
                );
            }
            OfdmAction::Resample(rate) => {
                self.start_edit(win, Edit::Resample { rate });
            }
            OfdmAction::Error(e) => self.alert(&e),
        }
    }

    /// Show an error the module reported, if any.
    pub(crate) fn ofdm_error(&mut self, e: Option<String>) {
        if let Some(e) = e {
            self.alert(&e);
        }
    }

    /// The arrow keys on the active `SA-OFDM` window with the constellations
    /// shown: move the green channel marker (and restart Show). Returns
    /// whether the key was used.
    pub(crate) fn ofdm_arrow_key(&mut self, k: &Keystroke) -> bool {
        let d: i64 = match k.key.as_str() {
            "left" => -1,
            "right" => 1,
            _ => return false,
        };
        let Some(win) = self.mdi.active() else {
            return false;
        };
        let Some((node, WinKind::Ofdm)) = self.binding(win) else {
            return false;
        };
        let o = self.ofdm_win(node);
        if !o.st.panes {
            return false;
        }
        o.st.step_marker(d);
        let e = o.st.restart_show().err();
        self.ofdm_error(e);
        true
    }
}

#[cfg(debug_assertions)]
impl Frame {
    /// Debug builds: the OFDM scenes of `SA_GUI_SCENE`: the OCG preset
    /// `ofdm_lu64_lg20_7000` in `SA-OFDM`, in Survey (`ofdm-survey`), after
    /// Find CT and ADP (`ofdm`), with the List of K (`ofdm-k`) or with Show
    /// stopped half way (`ofdm-show`).
    pub(crate) fn ofdm_scene(&mut self, scene: &str) {
        let Some(signal) = sn_synth::GeneratorSpec::preset("ofdm_lu64_lg20_7000")
            .and_then(|s| sn_synth::generate(&s).ok())
            .map(|g| g.signal)
        else {
            return;
        };
        let op = sn_core::Operation::new("load", serde_json::json!({"path": "/demo/ofdm.wav"}));
        let node = self.on_loaded("ofdm.wav".into(), op, signal.clone(), &[]);
        let caller = self.find_window(node, WinKind::Sonogram);
        if self.open_window(node, WinKind::Ofdm, caller).is_none() || scene == "ofdm-survey" {
            return;
        }
        let o = self.ofdm_win(node);
        o.st.init(signal.sample_rate());
        o.st.start_ct(&signal, (0, signal.len()));
        wait(&mut o.st);
        if o.st.start_adp().is_ok() {
            wait(&mut o.st);
        }
        match scene {
            "ofdm-k" => {
                if o.st.start_k().is_ok() {
                    wait(&mut o.st);
                }
            }
            "ofdm-show" => {
                o.st.sel = 5;
                o.st.other = 3;
                if o.st.start_show().is_ok() {
                    wait(&mut o.st);
                    for _ in 0..40 {
                        o.st.tick();
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                    o.st.stop();
                }
            }
            _ => {}
        }
    }
}

/// Debug builds: run the module's job to its end (scenes).
#[cfg(debug_assertions)]
fn wait(st: &mut OfdmState) {
    for _ in 0..12_000 {
        let _ = st.poll();
        if !st.is_running() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
