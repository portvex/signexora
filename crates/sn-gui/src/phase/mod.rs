//! SA's PhasePlane module window (`docs/SA_UI_SPEC.md` section 6) on the
//! Windows 95 kit: a fixed-size form that opens in its spectrum mode (Carrier
//! and BR pre-filled from the last Sc and Br) and switches to the phase plane
//! mode after `LoadFile`: the ISD corrector in the upper half, the phase
//! viewer in the lower half and SA's status bar, as `sa-app`'s
//! `phase_window.rs` draws them.
//!
//! The module is a child of `app` (declared there with a path), so it extends
//! [`Frame`] directly. The state and computations live in `sn_ui_core::phase`
//! (one [`PhaseState`] per signal, as in `sa-app`), the geometry in
//! `sn_ui_core::phase_plot`; this module keeps the controls' widget states per
//! window.
//!
//! - `spectrum.rs`: the spectrum mode's toolbar, spectrum and markers.
//! - `corrector.rs`: the ISD corrector, the angles histogram, the text panel.
//! - `viewer.rs`: the phase viewer and the status bar.
//! - `load.rs`: the `SA - select file for PhasePlane` dialog.
//! - `glue.rs`: the link to the session (pre-fill, the ISD result, closing).

mod corrector;
mod glue;
mod load;
mod spectrum;
#[cfg(test)]
mod tests;
mod viewer;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, RenderImage,
    StatefulInteractiveElement, Styled, Window, div,
};
use sn_core::NodeId;
use sn_ui_core::color::Rgba8;
use sn_ui_core::phase::{DemMode, FFT_SIZES, Form, PhaseState, SCALES};
use sn_ui_core::phase_plot::{FORM, Hover};

pub(crate) use load::LoadDialog;

use super::Frame;
use crate::win95::bars::tooltip;
use crate::win95::button::Changed;
use crate::win95::combo::ComboState;
use crate::win95::edit::EditState;
use crate::win95::geom::Rect;
use crate::win95::kit::{Action, action};
use crate::win95::scrollbar::ScrollState;
use crate::win95::trackbar::TrackState;
use crate::win95::widget::place;

/// Units of the form's scroll bars: an arrow moves a twentieth, a click on the
/// track a fifth (as `sa-app`'s scroll bars).
const SCROLL_UNITS: i32 = 20;
/// Page of the form's scroll bars, in units.
const SCROLL_PAGE: i32 = 4;

/// A scroll bar over `0..=1` at `value`.
fn unit_scroll(value: f32) -> ScrollState {
    let pos = (value.clamp(0.0, 1.0) * SCROLL_UNITS as f32).round() as i32;
    ScrollState::new(0, SCROLL_UNITS + SCROLL_PAGE - 1, SCROLL_PAGE, pos)
}

/// The value `0..=1` of a scroll bar made by [`unit_scroll`].
fn scroll_value(s: &ScrollState) -> f32 {
    s.pos as f32 / SCROLL_UNITS as f32
}

/// The widget states of one PhasePlane window.
pub(crate) struct PhaseWin {
    pub(crate) node: NodeId,
    pub(crate) scale: ComboState,
    pub(crate) fft: ComboState,
    pub(crate) isd_n: ComboState,
    pub(crate) nary: ComboState,
    pub(crate) diff: ComboState,
    pub(crate) dem: ComboState,
    pub(crate) carrier: EditState,
    pub(crate) br: EditState,
    pub(crate) spec_nav: ScrollState,
    pub(crate) isd_vs: ScrollState,
    pub(crate) isd_hs: ScrollState,
    pub(crate) const_vs: ScrollState,
    pub(crate) low_level: TrackState,
    pub(crate) alpha: TrackState,
    pub(crate) trimming: TrackState,
    pub(crate) delay: TrackState,
    pub(crate) points: TrackState,
    pub(crate) contrast: TrackState,
    /// The constellation image with what it was drawn from.
    image: Option<(String, Arc<RenderImage>)>,
    /// The pointer readouts of the status bar.
    pub(crate) hover: Hover,
}

impl std::fmt::Debug for PhaseWin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhaseWin")
            .field("node", &self.node)
            .field("hover", &self.hover)
            .finish_non_exhaustive()
    }
}

/// A combo box of `items` with `sel` selected.
fn combo(items: &[String], sel: Option<usize>) -> ComboState {
    ComboState::new(&items.iter().map(String::as_str).collect::<Vec<_>>(), sel)
}

/// The items of the `Scale` list.
pub(crate) fn scale_items() -> Vec<String> {
    SCALES.iter().map(|s| format!("1/{s}")).collect()
}

fn numbers(r: std::ops::RangeInclusive<u32>) -> Vec<String> {
    r.map(|n| n.to_string()).collect()
}

impl PhaseWin {
    fn new(node: NodeId, f: &Form) -> Self {
        let ffts: Vec<String> = FFT_SIZES.iter().map(ToString::to_string).collect();
        let dems: Vec<String> = DemMode::all().iter().map(|d| d.label()).collect();
        Self {
            node,
            scale: combo(&scale_items(), None),
            fft: combo(&ffts, None),
            isd_n: combo(&numbers(1..=32), None),
            nary: combo(&numbers(1..=32), None),
            diff: combo(&numbers(0..=3), None),
            dem: combo(&dems, None),
            carrier: EditState::new(&f.carrier),
            br: EditState::new(&f.br),
            spec_nav: unit_scroll(0.0),
            isd_vs: unit_scroll(0.5),
            isd_hs: unit_scroll(0.0),
            const_vs: unit_scroll(0.5),
            low_level: TrackState::new(-140, -20, -100, 10),
            alpha: TrackState::new(1, 20, 10, 2),
            trimming: TrackState::new(-45, 45, 0, 5),
            delay: TrackState::new(0, 100, 0, 10),
            points: TrackState::new(10, 5000, 1000, 100),
            contrast: TrackState::new(1, 255, 40, 10),
            image: None,
            hover: Hover::default(),
        }
    }

    /// Show the form's values in the controls (they change from the plots,
    /// the pre-fill and the other window of the same signal).
    fn sync(&mut self, f: &Form) {
        self.scale.selected = SCALES.iter().position(|s| *s == f.zoom);
        self.fft.selected = FFT_SIZES.iter().position(|n| *n == f.fft);
        self.isd_n.selected = (f.isd_n as usize).checked_sub(1);
        self.nary.selected = (f.order as usize).checked_sub(1);
        self.diff.selected = Some(usize::from(f.diff.min(3)));
        self.dem.selected = DemMode::all().iter().position(|d| *d == f.dem);
        if self.carrier.text != f.carrier {
            self.carrier.set_text(&f.carrier);
        }
        if self.br.text != f.br {
            self.br.set_text(&f.br);
        }
        let sync_scroll = |s: &mut ScrollState, v: f32| {
            if (scroll_value(s) - v).abs() > 0.5 / SCROLL_UNITS as f32 {
                *s = unit_scroll(v);
            }
        };
        sync_scroll(&mut self.spec_nav, f.spec_pos);
        sync_scroll(&mut self.isd_vs, f.isd_scale);
        sync_scroll(&mut self.isd_hs, f.isd_pos);
        sync_scroll(&mut self.const_vs, f.scale);
        self.low_level.set(f.low_level.round() as i32);
        self.alpha.set((f.alpha * 20.0).round() as i32);
        self.trimming.set(f.trimming);
        self.delay.set((f.delay * 1000.0).round() as i32);
        self.points.set(i32::try_from(f.points).unwrap_or(i32::MAX));
        self.contrast
            .set(i32::try_from(f.contrast).unwrap_or(i32::MAX));
    }
}

/// The PhasePlane windows of the frame: one state per signal (as `sa-app`
/// keeps it), the widget states per window and the `LoadFile` dialog.
pub(crate) struct PhaseHub {
    pub(crate) states: HashMap<NodeId, PhaseState>,
    pub(crate) wins: HashMap<u32, PhaseWin>,
    /// The `LoadFile` dialog.
    pub(crate) load: Option<LoadDialog>,
    /// The window whose popup menu is open.
    pub(crate) popup_win: Option<u32>,
    /// The clock of the playback.
    start: Instant,
    /// Images replaced since the last frame, to free from the sprite atlas.
    stale: Vec<Arc<RenderImage>>,
    /// The session generation the states belong to.
    generation: u64,
}

impl Default for PhaseHub {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            wins: HashMap::new(),
            load: None,
            popup_win: None,
            start: Instant::now(),
            stale: Vec::new(),
            generation: 0,
        }
    }
}

impl std::fmt::Debug for PhaseHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhaseHub")
            .field("states", &self.states.len())
            .field("wins", &self.wins.len())
            .finish_non_exhaustive()
    }
}

/// A GPUI image (BGRA) of `w` x `h` pixels.
pub(crate) fn render_image(px: &[Rgba8], w: usize, h: usize) -> Option<Arc<RenderImage>> {
    let mut bytes = Vec::with_capacity(px.len() * 4);
    for p in px {
        bytes.extend_from_slice(&[p.b, p.g, p.r, p.a]);
    }
    let buf = image::RgbaImage::from_raw(u32::try_from(w).ok()?, u32::try_from(h).ok()?, bytes)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buf)])))
}

/// A rectangle of the form.
pub(crate) const fn at(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::new(x, y, w, h)
}

/// An invisible element at `r` that shows `text` as a tooltip (placed before
/// the control it explains).
pub(crate) fn tip(id: String, r: Rect, text: &str) -> AnyElement {
    place(r)
        .id(gpui::ElementId::Name(id.into()))
        .tooltip(tooltip(text))
        .into_any_element()
}

impl Frame {
    /// The state of PhasePlane window `win`'s signal.
    pub(crate) fn phase_st(&mut self, win: u32) -> Option<&mut PhaseState> {
        let node = self.phase.wins.get(&win)?.node;
        self.phase.states.get_mut(&node)
    }

    /// The widget states of PhasePlane window `win`.
    pub(crate) fn phase_win(&mut self, win: u32) -> Option<&mut PhaseWin> {
        self.phase.wins.get_mut(&win)
    }

    /// An action on window `win`'s state.
    pub(crate) fn on_st(win: u32, f: impl Fn(&mut PhaseState) + 'static) -> Action<Self> {
        action(move |v: &mut Self, _, _| {
            if let Some(st) = v.phase_st(win) {
                f(st);
            }
        })
    }

    /// A check box callback setting a flag of window `win`'s form.
    pub(crate) fn form_flag(win: u32, f: fn(&mut Form) -> &mut bool) -> Changed<Self, bool> {
        Rc::new(move |v: &mut Self, on, _, _| {
            if let Some(st) = v.phase_st(win) {
                *f(&mut st.form) = on;
            }
        })
    }

    /// Seconds since the frame started (the playback clock).
    pub(crate) fn phase_now(&self) -> f64 {
        self.phase.start.elapsed().as_secs_f64()
    }

    /// Work that needs the poll loop: a playback, a run or a spectrum.
    pub(crate) fn phase_busy(&self) -> bool {
        self.phase.wins.values().any(|w| {
            self.phase.states.get(&w.node).is_some_and(|st| {
                st.is_running() || st.computing() || st.spectrum_busy() || st.isd_job.is_some()
            })
        })
    }

    /// Forget the widget states of windows that closed, and free replaced
    /// images.
    pub(crate) fn phase_prune(&mut self, window: &mut Window) {
        if self.phase.generation != self.generation {
            self.phase.generation = self.generation;
            self.phase_session_replaced();
        }
        // The state of a signal that left the session goes with it (its
        // demodulator result, eye traces and histograms are large).
        let (d, g) = (&self.discarded, &self.graph);
        self.phase
            .states
            .retain(|n, _| !d.contains(n) && g.get(*n).is_some());
        let open: Vec<u32> = self
            .signal_windows()
            .into_iter()
            .filter(|(_, _, k)| *k == super::model::WinKind::Phase)
            .map(|(w, _, _)| w)
            .collect();
        let gone: Vec<u32> = self
            .phase
            .wins
            .keys()
            .copied()
            .filter(|w| !open.contains(w))
            .collect();
        for w in gone {
            if let Some(mut pw) = self.phase.wins.remove(&w)
                && let Some((_, img)) = pw.image.take()
            {
                self.phase.stale.push(img);
            }
        }
        if self
            .phase
            .load
            .as_ref()
            .is_some_and(|d| !open.contains(&d.win))
        {
            self.phase.load = None;
        }
        for img in std::mem::take(&mut self.phase.stale) {
            window.drop_image(img).ok();
        }
    }

    /// The body of PhasePlane window `win` showing `node`.
    pub(crate) fn phase_body(
        &mut self,
        win: u32,
        node: NodeId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        self.phase_update(win, node);
        let mut root = div().relative().size_full();
        let Some(st) = self.phase.states.get(&node) else {
            return root.into_any_element();
        };
        let phase_mode = st.mode == sn_ui_core::phase::Mode::Phase;
        let mut els = Vec::new();
        if phase_mode {
            els.extend(self.corrector_area(win, window, cx));
        } else {
            els.extend(self.spectrum_area(win, node, window, cx));
        }
        els.extend(self.viewer_area(win, window, cx));
        for el in els {
            root = root.child(el);
        }
        root.into_any_element()
    }

    /// Per frame: open the state for this window, follow the loaded file,
    /// collect finished jobs, advance the playback and keep the spectra and
    /// the histogram up to date (as `sa-app`'s `phase_body`).
    fn phase_update(&mut self, win: u32, node: NodeId) {
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return;
        };
        let sc = self.sc_for(node);
        let br = self.measured(&sn_core::MeasurementKind::BaudRate);
        let markers = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let span = markers.sample_span(signal.len());
        let src_id = self.phase.states.get(&node).and_then(PhaseState::source);
        // A discarded node is still in the graph but no longer in the session.
        let source = src_id
            .filter(|id| !self.discarded.contains(id))
            .and_then(|id| self.graph.get(id).map(|n| (id, n.signal.clone())));
        let now = self.phase_now();
        let st = self.phase.states.entry(node).or_default();
        st.open(
            u64::from(win),
            sc.as_ref(),
            br.as_ref(),
            markers.freq_range(),
        );
        if src_id.is_some() && source.is_none() {
            // The loaded file left the session.
            st.loaded = None;
            st.corrected = None;
            st.mode = sn_ui_core::phase::Mode::Spectrum;
        }
        st.update(now, source.as_ref().map(|(id, s)| (*id, s)));
        let form = st.form.clone();
        match st.mode {
            sn_ui_core::phase::Mode::Spectrum => st.want_hires(
                sn_ui_core::phase::HiresKey {
                    node,
                    start: span.0,
                    end: span.1,
                    fft: form.fft,
                },
                &signal,
            ),
            sn_ui_core::phase::Mode::Phase => {
                if let Some((id, s)) = &source {
                    want_corr(st, *id, s);
                }
                if form.angles {
                    st.update_hist();
                }
            }
        }
        let isd = st.take_isd();
        let pw = self
            .phase
            .wins
            .entry(win)
            .or_insert_with(|| PhaseWin::new(node, &form));
        if pw.node != node {
            *pw = PhaseWin::new(node, &form);
        }
        pw.sync(&form);
        self.on_isd_done(node, isd);
    }

    /// The constellation image of window `win`, `side` pixels square, made
    /// again when what it shows changed.
    pub(crate) fn constellation_image(
        &mut self,
        win: u32,
        side: usize,
        cx: &gpui::App,
    ) -> Option<Arc<RenderImage>> {
        let pal = crate::win95::colors::palette(cx);
        let node = self.phase.wins.get(&win)?.node;
        let st = self.phase.states.get(&node)?;
        let key = format!(
            "{}:{}:{}:{}:{:?}:{}:{:p}",
            st.generation, st.play.pos, st.play.clear_from, st.eye, st.form, side, pal
        );
        let pw = self.phase.wins.get(&win)?;
        if let Some((k, img)) = &pw.image
            && *k == key
        {
            return Some(img.clone());
        }
        let px = sn_ui_core::phase_plot::constellation_pixels(st, side, pal);
        let img = render_image(&px, side, side)?;
        let pw = self.phase.wins.get_mut(&win)?;
        if let Some((_, old)) = pw.image.replace((key, img.clone())) {
            self.phase.stale.push(old);
        }
        Some(img)
    }
}

/// Keep the corrector's spectrum of `st` up to date on `signal` (`id`).
fn want_corr(st: &mut PhaseState, id: NodeId, signal: &sn_core::Signal) {
    let f = &st.form;
    if let (Some(carrier), Some(baud)) = (f.carrier_hz(), f.baud()) {
        let key = sn_ui_core::phase::CorrKey {
            source: id,
            mode: f.isd_mode,
            n: f.isd_n,
            carrier,
            baud,
            blocks: f.blocks,
            pos: (f.isd_pos * 1000.0).round() as u32,
        };
        st.want_corr(key, signal);
    }
}

/// The form's size as the window's client area.
pub(crate) const CLIENT: (i32, i32) = FORM;

/// A generated QPSK test recording for window captures and tests: 600 Bd on
/// 1800 Hz at 8000 Hz, about 4 s, with noise at 20 dB SNR.
#[cfg(any(test, debug_assertions))]
pub(crate) fn psk_signal() -> Option<sn_core::Signal> {
    let p = sn_synth::PskParams {
        order: 4,
        baud: 600.0,
        carrier: 1800.0,
        sample_rate: 8000.0,
        amplitude: 0.5,
    };
    let symbols = sn_synth::random_symbols(2400, 4, 5);
    let clean = sn_synth::psk(&p, &symbols).ok()?;
    sn_synth::add_awgn_snr(&clean, 20.0, 3).ok()
}

#[cfg(debug_assertions)]
impl Frame {
    /// Debug builds: the scenes of `SA_GUI_SCENE` for this step: `phase` (a
    /// PhasePlane window in its spectrum mode), `phase-plane` (a QPSK
    /// recording loaded and running, with Vectors of dem and the demodulator
    /// output) and `harmonics` (the `(q)` window of its power 2).
    pub(crate) fn phase_scene(&mut self, scene: &str) {
        let Some(signal) = psk_signal() else {
            return;
        };
        let op = sn_core::Operation::new("load", serde_json::json!({"path": "/demo/qpsk.wav"}));
        let node = self.on_loaded("qpsk.wav".into(), op, signal.clone(), &[]);
        let Some(win) = self.find_window(node, super::model::WinKind::Sonogram) else {
            return;
        };
        if let Some(u) = self.ui.get_mut(&node) {
            u.markers = sn_ui_core::markers::Markers {
                v: Some((0.0, 16_000.0)),
                h: Some((1000.0, 2600.0)),
                ..sn_ui_core::markers::Markers::default()
            };
        }
        if scene == "harmonics" {
            self.start_edit(win, sn_dsp::ops::Edit::Power { n: 2 });
            return;
        }
        let Some(pw) = self.open_window(node, super::model::WinKind::Phase, Some(win)) else {
            return;
        };
        self.phase_update(pw, node);
        if scene == "phase-plane"
            && let Some(st) = self.phase_st(pw)
        {
            st.load(node, "[TempDir] qpsk(0).wav".into());
            st.form.carrier = "1800".into();
            st.form.br = "600".into();
            st.form.order = 4;
            st.form.vectors = true;
            st.form.output_dem = true;
            st.form.dem.m = 4;
            st.start_or_break();
        }
    }
}
