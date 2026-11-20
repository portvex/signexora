//! SA's harmonics (power-of-N) module (`docs/SA_UI_SPEC.md` section 5,
//! M-p12, M-p13) on the Windows 95 kit, as `sa-app`'s `harmonics_window.rs`.
//!
//! The module is a sonogram window titled `...(q).wav` (SA's `x^n` opens it at
//! power 2), with the sonogram window's top toolbar, plot, Info and the
//! harmonics popup menu (`Measurement of Freq`, whose result is recorded as Sc
//! and pre-fills PhasePlane's Carrier, and `Cancel`). Its bottom toolbar
//! keeps the zoom buttons, the progress field and Stop, then `<` and `<<`
//! (carrier down by 1 and 10 Hz), the carrier field, `>>` and `>`, `St^n`
//! (compute the chosen power again and show it in the same window), `<`, the
//! power field, `>`, and the `Control` check box that draws a red line at the
//! carrier. The sonogram window calls in through its hooks
//! (`harmonics_toolbar`, `control_line`).
//!
//! The module is a child of `app` (declared there with a path), so it extends
//! [`Frame`] directly; the controls' logic is `sn_ui_core::harmonics`.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{AnyElement, Context};
use sn_core::NodeId;
use sn_dsp::ops::Edit;
use sn_ui_core::harmonics::HarmonicsState;
use sn_ui_core::lineage;
use sn_ui_core::sono_tools::{self, Button};
use sn_ui_core::t;

use super::Frame;
use super::sono::icons;
use crate::win95::bars::{TOOL, Tool, tool_button};
use crate::win95::button::{Ctl, check_box, push_button};
use crate::win95::edit::{EditState, FieldLook, edit};
use crate::win95::geom::Rect;
use crate::win95::kit::{action, lens};
use crate::win95::label::{Align, label};
use crate::win95::progress::progress_field;

/// Width of the progress field.
const PROGRESS_W: i32 = 110;
/// Width of the carrier field.
const CARRIER_W: i32 = 72;
/// Width of the power field.
const POWER_W: i32 = 26;
/// Size of the `<`, `<<`, `>>`, `>` buttons.
const STEP: (i32, i32) = (22, 20);
/// SA's red for a carrier that is not a number.
const INVALID: u32 = 0xC0_00_00;

/// The edit fields of one `(q)` window.
#[derive(Clone, Debug, Default)]
pub(crate) struct HarmWin {
    pub(crate) carrier: EditState,
    pub(crate) power: EditState,
}

/// The harmonics module's state: the controls per signal (moved to the new
/// result by `St^n`, as in `sa-app`), the edit fields per window.
#[derive(Debug, Default)]
pub(crate) struct HarmonicsHub {
    pub(crate) states: HashMap<NodeId, HarmonicsState>,
    pub(crate) wins: HashMap<u32, HarmWin>,
}

/// The controls of the toolbar after Stop, left to right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ctrl {
    Down,
    DownFast,
    Carrier,
    UpFast,
    Up,
    Compute,
    PowerDown,
    Power,
    PowerUp,
    Control,
}

impl Ctrl {
    pub(crate) const ALL: [Self; 10] = [
        Self::Down,
        Self::DownFast,
        Self::Carrier,
        Self::UpFast,
        Self::Up,
        Self::Compute,
        Self::PowerDown,
        Self::Power,
        Self::PowerUp,
        Self::Control,
    ];

    /// Width of the control, and the gap before it.
    fn size(self) -> (i32, i32) {
        match self {
            Self::Down => (STEP.0, 4),
            Self::Carrier => (CARRIER_W, 1),
            Self::Compute => (TOOL, 12),
            Self::Power => (POWER_W, 1),
            Self::Control => (80, 6),
            _ => (STEP.0, 1),
        }
    }
}

/// Where the parts of a harmonics toolbar go: the zoom buttons, the progress
/// field, Stop, and the controls with their widths.
pub(crate) type Places = (Vec<(Button, i32)>, i32, i32, Vec<(Ctrl, i32, i32)>);

/// The left edge of each part of a harmonics toolbar starting at `x0`: the
/// three zoom buttons, the progress field, Stop, then [`Ctrl::ALL`].
pub(crate) fn places(x0: i32) -> Places {
    let mut x = x0;
    let mut zoom = Vec::new();
    for b in [Button::ZoomIn, Button::ZoomBack, Button::ZoomReset] {
        zoom.push((b, x));
        x += TOOL + 1;
    }
    let progress = x + 5;
    x = progress + PROGRESS_W + 6;
    let stop = x;
    x += TOOL;
    let mut ctrls = Vec::new();
    for c in Ctrl::ALL {
        let (w, gap) = c.size();
        x += gap;
        ctrls.push((c, x, w));
        x += w;
    }
    (zoom, progress, stop, ctrls)
}

impl Frame {
    /// Give a power-N (`(q)`) window's signal the harmonics module's
    /// controls: the power of its edit, the carrier from the last Sc on the
    /// input's axis or the middle of the input's H markers.
    pub(crate) fn ensure_harmonics(&mut self, node: NodeId) {
        if self.harmonics.states.contains_key(&node) {
            return;
        }
        let Some(n) = self.graph.get(node) else {
            return;
        };
        let Some(Edit::Power { n: power }) = lineage::edit_of(n) else {
            return;
        };
        let Some(parent) = n.inputs.first().copied() else {
            return;
        };
        let carrier = self.harmonics_carrier(parent);
        self.harmonics
            .states
            .insert(node, HarmonicsState::new(power, carrier));
    }

    /// The carrier a new harmonics window starts with.
    fn harmonics_carrier(&self, parent: NodeId) -> f64 {
        if let Some(sc) = self.sc_for(parent).filter(|s| !s.other_axis) {
            return sc.value;
        }
        self.ui
            .get(&parent)
            .and_then(|u| u.markers.freq_range())
            .map_or(0.0, |(lo, hi)| f64::midpoint(lo, hi))
    }

    /// Where the Control line of `(q)` signal `node` goes (Hz on its axis),
    /// while `Control` is checked.
    pub(crate) fn harmonics_line(&self, node: NodeId) -> Option<f64> {
        let h = self.harmonics.states.get(&node).filter(|h| h.control)?;
        let c = h.carrier_hz()?;
        let parent = self.graph.get(node)?.inputs.first().copied()?;
        lineage::map_freq(&self.graph, parent, node, c)
    }

    /// `St^n` in the harmonics window `win` of `node`: compute power `n` of
    /// its input; the result replaces the window's signal.
    pub(crate) fn harmonics_compute(&mut self, node: NodeId, win: u32, n: u32) {
        if self
            .harmonics
            .states
            .get(&node)
            .is_some_and(|h| h.pending.is_some())
        {
            // One St^n at a time: the window shows its progress and can stop it.
            self.status = t!("hm-busy");
            return;
        }
        let Some(parent) = self.graph.get(node).and_then(|x| x.inputs.first().copied()) else {
            return;
        };
        if let Some(h) = self.harmonics.states.get_mut(&node) {
            h.pending = Some(n);
        }
        let before = self.jobs.len();
        self.start_edit_on(parent, Edit::Power { n }, Some(win));
        if self.jobs.len() > before
            && let Some(j) = self.jobs.last_mut()
        {
            // The job belongs to the `(q)` window: its progress field shows
            // it, its Stop button cancels it, and St^n waits for it.
            j.origin = Some(node);
        } else if let Some(h) = self.harmonics.states.get_mut(&node) {
            h.pending = None;
        }
    }

    /// When `edit` is the power a harmonics window `caller` asked for, show
    /// its result `id` in that window (keeping its display settings) instead
    /// of a new one. Returns true when it did.
    pub(crate) fn retarget_harmonics(
        &mut self,
        caller: Option<u32>,
        parent: NodeId,
        edit: &Edit,
        id: NodeId,
    ) -> bool {
        let Edit::Power { n } = edit else {
            return false;
        };
        let Some(win) = caller.filter(|c| self.mdi.get(*c).is_some()) else {
            return false;
        };
        let Some((old, kind)) = self.binding(win) else {
            return false;
        };
        let pending = self.harmonics.states.get(&old).and_then(|h| h.pending);
        let same_input =
            self.graph.get(old).and_then(|x| x.inputs.first().copied()) == Some(parent);
        if kind != super::model::WinKind::Sonogram
            || pending != Some(*n)
            || !same_input
            || old == id
        {
            return false;
        }
        let mut h = self.harmonics.states.remove(&old);
        if let Some(h) = h.as_mut() {
            h.pending = None;
            h.power = *n;
            h.power_text = n.to_string();
        }
        if let Some(h) = h {
            self.harmonics.states.insert(id, h);
        }
        let prev = self.ui.get(&old).map(|u| (u.settings, u.info));
        if let (Some((settings, info)), Some(next)) = (prev, self.ui.get_mut(&id)) {
            next.settings = settings;
            next.info = info;
        }
        for (_, k) in self.kinds.iter_mut().filter(|(w, _)| *w == win) {
            *k = super::ChildKind::Signal(id, kind);
        }
        self.discard_unused(old);
        true
    }

    /// Clear `St^n`'s mark once its job is gone (stopped, failed or shown
    /// elsewhere), so the window can start another.
    fn harmonics_settle(&mut self, node: NodeId) {
        let running = self
            .jobs
            .iter()
            .any(|j| j.origin == Some(node) && j.kind != super::model::JobKind::Overview);
        if !running && let Some(h) = self.harmonics.states.get_mut(&node) {
            h.pending = None;
        }
    }

    /// Show the controls' values in window `win`'s fields.
    fn sync_harmonics(&mut self, win: u32, node: NodeId) {
        let power_id = format!("hm{win}.power");
        let typing_power = self.kit.has_focus(&power_id);
        let Some(h) = self.harmonics.states.get_mut(&node) else {
            return;
        };
        let w = self.harmonics.wins.entry(win).or_default();
        if w.carrier.text != h.carrier {
            w.carrier.set_text(&h.carrier);
        }
        // Leaving the power field restores the power in use when the text is
        // not one.
        if !typing_power && !h.commit_power_text() {
            h.power_text = h.power.to_string();
        }
        if w.power.text != h.power_text {
            w.power.set_text(&h.power_text);
        }
        w.carrier.look = h.carrier_hz().is_none().then_some(FieldLook {
            bg: 0xFF_FF_FF,
            fg: INVALID,
            border: true,
        });
    }

    /// The harmonics module's bottom toolbar of window `win` (signal `node`)
    /// in `r` (client coordinates).
    pub(crate) fn harmonics_bar(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        self.ensure_harmonics(node);
        self.harmonics_settle(node);
        self.sync_harmonics(win, node);
        let tools = self.tool_state(win, node);
        let y = r.y + (r.h - TOOL) / 2;
        let (zoom, progress, stop, ctrls) = places(r.x + 2);
        let mut out = Vec::new();
        for (b, x) in zoom.into_iter().chain([(Button::Abort, stop)]) {
            let why = sono_tools::missing(b, tools);
            let enabled = why.is_none();
            let glyph = icons::button_glyph(b, enabled);
            let tip = b.tooltip(why.as_deref());
            let id = format!("hm{win}.b.{b:?}");
            out.push(tool_button(
                &self.kit,
                cx,
                Tool {
                    id: &id,
                    at: (x, y),
                    glyph: &glyph,
                    enabled,
                    checked: false,
                    tip: &tip,
                },
                action(move |v: &mut Self, _, _| v.press_button(win, b)),
            ));
        }
        let p = self.node_progress(node);
        out.push(progress_field(
            cx,
            Rect::new(progress, y + 2, PROGRESS_W, TOOL - 4),
            p,
        ));
        for (c, x, w) in ctrls {
            out.extend(self.harmonics_ctrl(
                win,
                node,
                c,
                Rect::new(x, y, w, TOOL),
                tools.running,
                cx,
            ));
        }
        out
    }

    /// One control of the harmonics toolbar at `r`.
    fn harmonics_ctrl(
        &mut self,
        win: u32,
        node: NodeId,
        c: Ctrl,
        r: Rect,
        running: bool,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(h) = self.harmonics.states.get(&node).cloned() else {
            return Vec::new();
        };
        let scope = format!("mdi{win}");
        let step = |d: i64| {
            action(move |v: &mut Self, _, _| {
                if let Some(h) = v.harmonics_of(win) {
                    h.step_carrier(d);
                }
            })
        };
        let power_step = |d: i32| {
            action(move |v: &mut Self, _, _| {
                if let Some(h) = v.harmonics_of(win) {
                    h.step_power(d);
                }
            })
        };
        let small = Rect::new(r.x, r.y + (r.h - STEP.1) / 2, STEP.0, STEP.1);
        let (text, hint, on) = match c {
            Ctrl::Down => ("<", t!("hm-carrier-down"), step(-1)),
            Ctrl::DownFast => ("<<", t!("hm-carrier-down-fast"), step(-10)),
            Ctrl::UpFast => (">>", t!("hm-carrier-up-fast"), step(10)),
            Ctrl::Up => (">", t!("hm-carrier-up"), step(1)),
            Ctrl::PowerDown => ("<", t!("hm-power-down"), power_step(-1)),
            Ctrl::PowerUp => (">", t!("hm-power-up"), power_step(1)),
            Ctrl::Carrier | Ctrl::Power => return self.harmonics_field(win, c, r, &h, cx),
            Ctrl::Compute => {
                return self.compute_button(win, node, r, running || h.pending.is_some(), cx);
            }
            Ctrl::Control => return self.control_box(win, r, h.control, cx),
        };
        let id = format!("hm{win}.{c:?}");
        vec![
            crate::app::phase::tip(format!("{id}.tip"), small, &hint),
            push_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: small,
                    label: text,
                    enabled: true,
                },
                false,
                on,
            ),
        ]
    }

    /// The carrier or power field.
    fn harmonics_field(
        &mut self,
        win: u32,
        c: Ctrl,
        r: Rect,
        h: &HarmonicsState,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let carrier = c == Ctrl::Carrier;
        let Some(st) = self.harmonics.wins.get(&win).map(|w| {
            if carrier {
                w.carrier.clone()
            } else {
                w.power.clone()
            }
        }) else {
            return Vec::new();
        };
        let id = format!("hm{win}.{}", if carrier { "carrier" } else { "power" });
        let scope = format!("mdi{win}");
        let hint = match (carrier, h.carrier_hz().is_some()) {
            (true, true) => t!("hm-carrier.hover"),
            (true, false) => t!("hm-carrier-invalid"),
            (false, _) => t!("hm-power.hover"),
        };
        let field = Rect::new(r.x, r.y + 1, r.w, r.h - 2);
        vec![
            crate::app::phase::tip(format!("{id}.tip"), field, &hint),
            edit(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: field,
                    label: "",
                    enabled: true,
                },
                &st,
                lens(move |v: &mut Self| {
                    let w = v.harmonics.wins.get_mut(&win)?;
                    Some(if carrier {
                        &mut w.carrier
                    } else {
                        &mut w.power
                    })
                }),
                Some(action(move |v: &mut Self, _, _| {
                    v.harmonics_typed(win, carrier);
                })),
            ),
        ]
    }

    /// Typing in the carrier or power field. Intermediate keystrokes ("1" on
    /// the way to "16") stay in the power field; a valid value is taken.
    fn harmonics_typed(&mut self, win: u32, carrier: bool) {
        let Some(text) = self.harmonics.wins.get(&win).map(|w| {
            if carrier {
                w.carrier.text.clone()
            } else {
                w.power.text.clone()
            }
        }) else {
            return;
        };
        if let Some(h) = self.harmonics_of(win) {
            if carrier {
                h.carrier = text;
            } else {
                h.power_text = text;
                h.commit_power_text();
            }
        }
    }

    /// The controls of window `win`'s signal.
    pub(crate) fn harmonics_of(&mut self, win: u32) -> Option<&mut HarmonicsState> {
        let node = self.node_of(win)?;
        self.harmonics.states.get_mut(&node)
    }

    /// `St^n`.
    fn compute_button(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        busy: bool,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let glyph = icons::button_glyph(Button::Power, !busy);
        let tip = if busy {
            format!("{}\n{}", t!("hm-compute"), t!("hm-busy"))
        } else {
            t!("hm-compute")
        };
        let id = format!("hm{win}.compute");
        vec![tool_button(
            &self.kit,
            cx,
            Tool {
                id: &id,
                at: (r.x, r.y),
                glyph: &glyph,
                enabled: !busy,
                checked: false,
                tip: &tip,
            },
            action(move |v: &mut Self, _, _| {
                let n = v.harmonics.states.get(&node).map(|h| h.power);
                if let Some(n) = n {
                    v.harmonics_compute(node, win, n);
                }
            }),
        )]
    }

    /// `Control`: SA's caption and its check box.
    fn control_box(
        &mut self,
        win: u32,
        r: Rect,
        on: bool,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let id = format!("hm{win}.control");
        let scope = format!("mdi{win}");
        let caption = t!("hm-control");
        let label_w = r.w - 16;
        vec![
            crate::app::phase::tip(format!("{id}.tip"), r, &t!("hm-control.hover")),
            label(
                cx,
                Rect::new(r.x, r.y, label_w, r.h),
                &caption,
                false,
                true,
                Align::Right,
            ),
            check_box(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: Rect::new(r.x + label_w + 3, r.y, 14, r.h),
                    label: "",
                    enabled: true,
                },
                on,
                Rc::new(move |v: &mut Self, on, _, _| {
                    if let Some(h) = v.harmonics_of(win) {
                        h.control = on;
                    }
                }),
            ),
        ]
    }

    /// Forget the fields of windows that closed and the controls of signals
    /// that left the session.
    pub(crate) fn harmonics_prune(&mut self) {
        let open: Vec<(u32, NodeId)> = self
            .signal_windows()
            .into_iter()
            .map(|(w, n, _)| (w, n))
            .collect();
        self.harmonics
            .wins
            .retain(|w, _| open.iter().any(|(o, _)| o == w));
        let discarded = &self.discarded;
        let graph = &self.graph;
        self.harmonics
            .states
            .retain(|n, _| !discarded.contains(n) && graph.get(*n).is_some());
    }
}
