//! The toolkit-independent logic of SA's sonogram work window
//! (`docs/SA_UI_SPEC.md` section 4): the bottom toolbar's order, enable rules
//! and operations, the popup menu, the Info overlay, the top toolbar's lists,
//! the zoom steps, marker hit tests and the floating result boxes.
//!
//! The front ends draw these; the rules and texts are the same in both.

use sn_dsp::ops::Edit;

use crate::format;
use crate::markers::{MarkerId, Markers};
use crate::t;
use crate::view::{Extent, View};

/// The bottom toolbar's buttons (SA 6.2.6.7, left to right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    /// Zoom the band between the H markers to the window.
    ZoomIn,
    /// Zoom back one step.
    ZoomBack,
    /// Reset the zoom.
    ZoomReset,
    /// Stop the work running for this window.
    Abort,
    /// Play the V-marker span.
    Play,
    /// Stop playback.
    StopPlay,
    /// Keep only the band between the H markers.
    BandSelect,
    /// Delete the V x H rectangle.
    AreaDelete,
    /// Delete the band between the H markers.
    BandReject,
    /// Quadrature AM detector.
    AmDetector,
    /// Modified AM detector (MFSK).
    ModifiedAm,
    /// Double half-wave detector.
    HalfWave,
    /// First derivative.
    Derivative,
    /// The harmonics (power-of-N) module.
    Power,
    /// The PhasePlane module.
    PhasePlane,
    /// Autocorrelation of the V span.
    Acf,
    /// Cross-correlation of the V span.
    Ccf,
    /// The OFDM module.
    Ofdm,
    /// Phase detector (frequency demodulator).
    PhaseDetector,
    /// Hard limiter.
    Limiter,
    /// I/Q to real.
    IqToReal,
    /// Shift the H band to base, direct.
    ShiftToBase,
    /// Shift the H band to base, inverted.
    ShiftToBaseInverted,
    /// In-band inverse (whole spectrum with the H markers collapsed).
    Inverse,
    /// Copy the V span to a new window.
    Copy,
    /// `R`: resample to the rate in the value field.
    Resample,
    /// `S`: shift the spectrum by the value field.
    Shift,
}

/// One slot of the bottom toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// A toolbar button.
    Button(Button),
    /// The slightly wider gap SA leaves between groups.
    Gap,
    /// The progress field.
    Progress,
    /// The value field of R and S.
    Value,
}

/// The bottom toolbar, left to right.
pub const BOTTOM_TOOLBAR: [Slot; 33] = [
    Slot::Button(Button::ZoomIn),
    Slot::Button(Button::ZoomBack),
    Slot::Button(Button::ZoomReset),
    Slot::Gap,
    Slot::Progress,
    Slot::Gap,
    Slot::Button(Button::Abort),
    Slot::Button(Button::Play),
    Slot::Button(Button::StopPlay),
    Slot::Button(Button::BandSelect),
    Slot::Button(Button::AreaDelete),
    Slot::Button(Button::BandReject),
    Slot::Button(Button::AmDetector),
    Slot::Button(Button::ModifiedAm),
    Slot::Button(Button::HalfWave),
    Slot::Button(Button::Derivative),
    Slot::Button(Button::Power),
    Slot::Button(Button::PhasePlane),
    Slot::Button(Button::Acf),
    Slot::Button(Button::Ccf),
    Slot::Button(Button::Ofdm),
    Slot::Button(Button::PhaseDetector),
    Slot::Button(Button::Limiter),
    Slot::Button(Button::IqToReal),
    Slot::Button(Button::ShiftToBase),
    Slot::Button(Button::ShiftToBaseInverted),
    Slot::Button(Button::Inverse),
    Slot::Button(Button::Copy),
    Slot::Gap,
    Slot::Button(Button::Resample),
    Slot::Value,
    Slot::Button(Button::Shift),
    Slot::Gap,
];

/// Every button of the bottom toolbar, in toolbar order.
pub const BUTTONS: [Button; 27] = [
    Button::ZoomIn,
    Button::ZoomBack,
    Button::ZoomReset,
    Button::Abort,
    Button::Play,
    Button::StopPlay,
    Button::BandSelect,
    Button::AreaDelete,
    Button::BandReject,
    Button::AmDetector,
    Button::ModifiedAm,
    Button::HalfWave,
    Button::Derivative,
    Button::Power,
    Button::PhasePlane,
    Button::Acf,
    Button::Ccf,
    Button::Ofdm,
    Button::PhaseDetector,
    Button::Limiter,
    Button::IqToReal,
    Button::ShiftToBase,
    Button::ShiftToBaseInverted,
    Button::Inverse,
    Button::Copy,
    Button::Resample,
    Button::Shift,
];

/// What the enable rules look at.
// Independent conditions, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolState {
    /// The V markers select a span (not collapsed).
    pub v: bool,
    /// The H markers select a band.
    pub h: bool,
    /// There is a zoom step to go back to.
    pub history: bool,
    /// The window's signal has work running.
    pub running: bool,
    /// Audio is playing (or being prepared).
    pub playing: bool,
    /// The signal is I/Q.
    pub complex: bool,
}

impl ToolState {
    /// The marker part of the state from `m`.
    #[must_use]
    pub fn with_markers(mut self, m: &Markers) -> Self {
        self.v = m.time_range().is_some();
        self.h = m.freq_range().is_some();
        self
    }
}

/// What a disabled button lacks, or `None` when it is enabled. Buttons that need
/// markers are disabled while the relevant pair is collapsed (SA 6.2).
pub fn missing(b: Button, s: ToolState) -> Option<String> {
    let need_h = || (!s.h).then(|| t!("edits-need-h"));
    let need_v = || (!s.v).then(|| t!("edits-need-v"));
    let need_vh = || (!(s.v && s.h)).then(|| t!("edits-need-vh"));
    match b {
        Button::ZoomIn
        | Button::BandSelect
        | Button::BandReject
        | Button::Power
        | Button::PhaseDetector
        | Button::ShiftToBase
        | Button::ShiftToBaseInverted => need_h(),
        Button::AreaDelete | Button::PhasePlane | Button::Ofdm => need_vh(),
        Button::Acf | Button::Ccf | Button::Copy => need_v(),
        Button::ZoomBack => (!s.history).then(|| t!("sw-need-history")),
        Button::Abort => (!s.running).then(|| t!("sw-need-running")),
        Button::StopPlay => (!s.playing).then(|| t!("sw-need-playing")),
        Button::IqToReal => (!s.complex).then(|| t!("edits-need-complex")),
        Button::ZoomReset
        | Button::Play
        | Button::AmDetector
        | Button::ModifiedAm
        | Button::HalfWave
        | Button::Derivative
        | Button::Limiter
        | Button::Inverse
        | Button::Resample
        | Button::Shift => None,
    }
}

impl Button {
    /// Fluent key of the hint (tooltip).
    pub const fn hint_key(self) -> &'static str {
        match self {
            Self::ZoomIn => "sw-hint-zoom-in",
            Self::ZoomBack => "sw-hint-zoom-back",
            Self::ZoomReset => "sw-hint-zoom-reset",
            Self::Abort => "sw-hint-abort",
            Self::Play => "sw-hint-play",
            Self::StopPlay => "sw-hint-stop-play",
            Self::BandSelect => "sw-hint-band-select",
            Self::AreaDelete => "sw-hint-area-delete",
            Self::BandReject => "sw-hint-band-reject",
            Self::AmDetector => "sw-hint-am",
            Self::ModifiedAm => "sw-hint-modified-am",
            Self::HalfWave => "sw-hint-half-wave",
            Self::Derivative => "sw-hint-derivative",
            Self::Power => "sw-hint-power",
            Self::PhasePlane => "sw-hint-phase-plane",
            Self::Acf => "sw-hint-acf",
            Self::Ccf => "sw-hint-ccf",
            Self::Ofdm => "sw-hint-ofdm",
            Self::PhaseDetector => "sw-hint-phase-detector",
            Self::Limiter => "sw-hint-limiter",
            Self::IqToReal => "sw-hint-iq",
            Self::ShiftToBase => "sw-hint-shift-base",
            Self::ShiftToBaseInverted => "sw-hint-shift-base-inverted",
            Self::Inverse => "sw-hint-inverse",
            Self::Copy => "sw-hint-copy",
            Self::Resample => "sw-hint-resample",
            Self::Shift => "sw-hint-shift",
        }
    }

    /// The tooltip: the hint, and for a disabled button also what it lacks.
    pub fn tooltip(self, why_disabled: Option<&str>) -> String {
        crate::i18n::tooltip(self.hint_key(), why_disabled)
    }
}

/// Toolbar requests that change the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewAction {
    /// Zoom to the marker selection (time and, where available, frequency).
    ZoomToSelection,
    /// Stretch the band between the H markers over the whole window (SA's zoom in).
    ZoomBand,
    /// Return to the previous view.
    Back,
    /// Show the whole signal.
    Full,
}

/// Most zoom steps remembered for Zoom back.
const HISTORY_MAX: usize = 64;

/// Apply a view request. `use_freq` is false for views without a frequency axis.
pub fn apply_action(
    action: ViewAction,
    view: &mut View,
    history: &mut Vec<View>,
    markers: &Markers,
    extent: Extent,
    width: f64,
    use_freq: bool,
) {
    match action {
        ViewAction::ZoomToSelection => {
            let time = markers.time_range();
            let freq = if use_freq { markers.freq_range() } else { None };
            if time.is_some() || freq.is_some() {
                history.push(*view);
                view.show(time, freq, extent, width);
            }
        }
        ViewAction::ZoomBand => {
            if use_freq && let Some(band) = markers.freq_range() {
                history.push(*view);
                view.show(None, Some(band), extent, width);
            }
        }
        ViewAction::Back => {
            if let Some(prev) = history.pop() {
                *view = prev;
                view.resized(extent, width);
            }
        }
        ViewAction::Full => {
            history.push(*view);
            let keep = (view.f_lo, view.f_hi);
            *view = View::fit(extent, width);
            if !use_freq {
                (view.f_lo, view.f_hi) = keep;
            }
        }
    }
    if history.len() > HISTORY_MAX {
        history.drain(..1);
    }
}

/// What a bottom toolbar button asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// Change the view of this window.
    View(ViewAction),
    /// Run an edit; its result opens in a new window.
    Edit(Edit),
    /// SA's `x^n`: the harmonics module (power 2) as a new `(q)` window.
    Power,
    /// Open the PhasePlane module on this signal.
    PhasePlane,
    /// Open the OFDM module on this signal.
    Ofdm,
    /// Play the V span.
    Play,
    /// Stop playback.
    StopPlay,
    /// Stop the work running for this signal.
    Abort,
    /// Show SA's error box with this text.
    Error(String),
}

/// What button `b` asks for, given the markers and the R/S `value` text.
pub fn run_button(b: Button, m: &Markers, value: &str) -> Option<Run> {
    let h = m.freq_range();
    let v = m
        .time_range()
        .map(|(a, b)| (a.max(0.0) as usize, b.max(0.0) as usize));
    let edit = |e: Edit| Some(Run::Edit(e));
    match b {
        Button::ZoomIn => Some(Run::View(ViewAction::ZoomBand)),
        Button::ZoomBack => Some(Run::View(ViewAction::Back)),
        Button::ZoomReset => Some(Run::View(ViewAction::Full)),
        Button::Abort => Some(Run::Abort),
        Button::Play => Some(Run::Play),
        Button::StopPlay => Some(Run::StopPlay),
        Button::BandSelect => h.and_then(|(lo, hi)| edit(Edit::BandSelect { lo, hi })),
        Button::BandReject => h.and_then(|(lo, hi)| edit(Edit::BandReject { lo, hi })),
        Button::AreaDelete => match (v, h) {
            (Some((start, end)), Some((lo, hi))) => edit(Edit::AreaDelete { start, end, lo, hi }),
            _ => None,
        },
        Button::AmDetector => edit(Edit::AmDetector),
        Button::ModifiedAm => edit(Edit::ModifiedAm),
        Button::HalfWave => edit(Edit::HalfWave),
        Button::Derivative => edit(Edit::Derivative),
        Button::Power => Some(Run::Power),
        Button::PhasePlane => Some(Run::PhasePlane),
        Button::Ofdm => Some(Run::Ofdm),
        Button::Acf => v.and_then(|(start, end)| edit(Edit::Acf { start, end })),
        Button::Ccf => v.and_then(|(start, end)| edit(Edit::Ccf { start, end })),
        Button::Copy => v.and_then(|(start, end)| edit(Edit::Copy { start, end })),
        Button::PhaseDetector => edit(Edit::PhaseDetector),
        Button::Limiter => edit(Edit::Limiter),
        Button::IqToReal => edit(Edit::IqToReal),
        Button::ShiftToBase | Button::ShiftToBaseInverted => h.and_then(|(lo, hi)| {
            edit(Edit::ShiftToBase {
                lo,
                hi,
                inverted: b == Button::ShiftToBaseInverted,
            })
        }),
        // With the H markers collapsed: SA 6.2.5.4's fast whole-spectrum inversion.
        Button::Inverse => match h {
            Some((lo, hi)) => edit(Edit::InvertBand { lo, hi }),
            None => edit(Edit::Invert),
        },
        Button::Resample | Button::Shift => Some(rs_run(b == Button::Resample, value)),
    }
}

/// The R (`resample`) or S operation for the typed R/S value: the edit, or an
/// error that says what is wrong with the value.
pub fn rs_run(resample: bool, text: &str) -> Run {
    match format::parse_number(text) {
        Some(rate) if resample && rate > 0.0 => Run::Edit(Edit::Resample { rate }),
        Some(hz) if !resample => Run::Edit(Edit::Shift { hz }),
        _ => {
            let text = text.trim().to_owned();
            let why = if resample {
                t!("sw-rs-resample-invalid", text = text)
            } else {
                t!("sw-rs-shift-invalid", text = text)
            };
            Run::Error(format!("{}\n{why}", t!("sw-check-settings")))
        }
    }
}

/// Items of the right-click popup menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupItem {
    /// `Save as` (submenu of file formats).
    SaveAs,
    /// `Wave Form module`.
    WaveForm,
    /// `Measurement of Freq`.
    MeasureFreq,
    /// `Auto define params`.
    Adp,
    /// `Correction of BR`.
    CorrectBr,
    /// `Cancel`.
    Cancel,
    /// A separator line.
    Separator,
}

/// SA 6.2's popup menu: `Save as`, `Wave Form module`, `Measurement of Freq` (needs
/// V and H markers), `Auto define params` (needs V markers), `Correction of BR`,
/// `Cancel`. Items that need markers are left out while the markers are collapsed.
pub fn popup_items(v: bool, h: bool) -> Vec<PopupItem> {
    let mut items = vec![PopupItem::SaveAs, PopupItem::Separator, PopupItem::WaveForm];
    if v && h {
        items.push(PopupItem::MeasureFreq);
    }
    if v {
        items.push(PopupItem::Adp);
    }
    items.extend([
        PopupItem::Separator,
        PopupItem::CorrectBr,
        PopupItem::Cancel,
    ]);
    items
}

/// The harmonics module's popup menu (M-p13): `Measurement of Freq` (with V and
/// H markers), a separator and `Cancel`.
pub fn harmonics_popup_items(v: bool, h: bool) -> Vec<PopupItem> {
    let mut items = Vec::new();
    if v && h {
        items.push(PopupItem::MeasureFreq);
    }
    items.extend([PopupItem::Separator, PopupItem::Cancel]);
    items
}

/// SA's Image list (lower display limit), in dB below the strongest component.
pub const IMAGE_RANGES: [f32; 6] = [40.0, 50.0, 65.0, 83.0, 100.0, 120.0];

/// An Image value as SA writes it: `- 50 db`.
pub fn image_text(db: f32) -> String {
    format!("- {db:.0} db")
}

/// Presets of the R/S value field (SA 6.2.1.8 added a list).
pub const VALUE_PRESETS: [f64; 13] = [
    2000.0, 4000.0, 6000.0, 8000.0, 11_025.0, 12_000.0, 16_000.0, 22_050.0, 24_000.0, 32_000.0,
    44_100.0, 48_000.0, 96_000.0,
];

/// The preset list of the R/S field for a signal sampled at `fs`: its own rate
/// first, then the presets other than it.
pub fn value_presets(fs: f64) -> Vec<f64> {
    let mut out = vec![fs];
    out.extend(
        VALUE_PRESETS
            .iter()
            .copied()
            .filter(|p| (p - fs).abs() > 1e-9),
    );
    out
}

/// The R/S field's first text: the signal's own rate.
pub fn value_text(fs: f64) -> String {
    format::dec_trim(fs, 2)
}

/// The shown band as the Fl and Fu fields read it (integer Hz) and whether it is
/// zoomed. `rf` is the recording's centre frequency (0 for audio).
pub fn shown_band(extent: Option<Extent>, view: Option<View>, rf: f64) -> (String, String, bool) {
    let Some(extent) = extent else {
        return (String::new(), String::new(), false);
    };
    let (lo, hi) = view.map_or((extent.f_min, extent.f_max), |v| (v.f_lo, v.f_hi));
    let span = (extent.f_max - extent.f_min).abs().max(1e-9);
    let zoomed = (lo - extent.f_min).abs() > span * 1e-6 || (extent.f_max - hi).abs() > span * 1e-6;
    let text = |f: f64| format!("{:.0}", f + rf);
    (text(lo), text(hi), zoomed)
}

/// Br the Info overlay shows before any measurement (SA's default).
pub const DEFAULT_BR: f64 = 33.33;

/// The lines of SA's Info overlay, in SA's order: `Fu`, `Fc`, `Fl`, `dF`, `t1`, `t2`,
/// `dT`, `S`, `Br`, `Sc`. Markers that are off read as collapsed at 0. A recording
/// with a centre frequency shows absolute frequencies.
pub fn info_lines(m: &Markers, fs: f64, rf: f64, br: Option<f64>, sc: Option<f64>) -> Vec<String> {
    let r = m.readout(fs);
    let hz = |v: f64| {
        if rf == 0.0 {
            format::dec_trim(v, 0)
        } else {
            format::freq(v + rf)
        }
    };
    let (lo, hi) = (r.f_lo.unwrap_or(0.0), r.f_hi.unwrap_or(0.0));
    let fc = f64::midpoint(lo, hi);
    let fc_text = if rf == 0.0 {
        format::dec_trim(fc, 1)
    } else {
        format::freq(fc + rf)
    };
    let sc_text = match sc {
        Some(s) if rf == 0.0 => format::dec_trim(s, 1),
        Some(s) => format::freq(s + rf),
        None => fc_text.clone(),
    };
    let (t1, t2) = (r.t1.unwrap_or(0.0), r.t2.unwrap_or(0.0));
    vec![
        format!("Fu = {}", hz(hi)),
        format!("Fc = {fc_text}"),
        format!("Fl = {}", hz(lo)),
        format!("dF = {}", format::dec_trim(hi - lo, 0)),
        format!("t1 = {}", format::time_short(t1)),
        format!("t2 = {}", format::time_short(t2)),
        format!("dT = {}", format::time_long(t2 - t1)),
        format!("S = {}", format::dec(r.s.unwrap_or(0.0), 2)),
        format!("Br = {}", format::dec(br.unwrap_or(DEFAULT_BR), 2)),
        format!("Sc = {sc_text}"),
    ]
}

/// Collapse both marker pairs onto `(sample, hz)` (SA: double-click). Collapsed
/// markers select nothing, so the tools that need a selection are disabled.
pub fn collapse_markers(markers: &mut Markers, sample: f64, hz: f64) {
    *markers = Markers {
        v: Some((sample, sample)),
        h: Some((hz, hz)),
        locked: None,
        active: None,
    };
}

/// Grab distance of a marker line, pixels.
pub const GRAB_PX: f64 = 6.0;
/// Size of a marker handle (SA 6.2: solid triangles on the top and left edges).
pub const HANDLE: f64 = 7.0;

/// Which marker (if any) has its line within grab distance of `(x, y)`, a point
/// relative to the top left of a plot `height` pixels high.
pub fn marker_near(
    markers: &Markers,
    view: &View,
    height: f64,
    (x, y): (f64, f64),
) -> Option<MarkerId> {
    crate::markers::nearest(markers, GRAB_PX, |id, v| {
        Some(if id.is_vertical() {
            (view.sample_to_x(v) - x).abs()
        } else {
            (view.freq_to_y(v, height) - y).abs()
        })
    })
}

/// Which marker (if any) has its handle under `(x, y)` (relative to the plot's
/// top left): V handles hang from the top edge, H handles stick out of the left
/// edge.
pub fn handle_near(
    markers: &Markers,
    view: &View,
    height: f64,
    (x, y): (f64, f64),
) -> Option<MarkerId> {
    crate::markers::nearest(markers, HANDLE + 2.0, |id, v| {
        if id.is_vertical() {
            (y <= 2.0 * HANDLE).then(|| (view.sample_to_x(v) - x).abs())
        } else {
            (x <= 2.0 * HANDLE).then(|| (view.freq_to_y(v, height) - y).abs())
        }
    })
}

/// A floating result box (SA: Measurement of Freq and ADP answer with a small box
/// in the plot near the selection; the left button moves it, a right click
/// removes it).
#[derive(Debug, Clone, PartialEq)]
pub struct FloatLabel {
    /// E.g. `Freq: 4777,34 Hz`.
    pub text: String,
    /// Where it was measured (sample, Hz): the box is first put beside it.
    pub anchor: (f64, f64),
    /// Offset of the box from the plot's top left once placed, pixels.
    pub offset: Option<(i32, i32)>,
}

impl FloatLabel {
    /// A new box for `text` measured at `anchor`, not placed yet.
    pub fn new(text: String, anchor: (f64, f64)) -> Self {
        Self {
            text,
            anchor,
            offset: None,
        }
    }

    /// Where the box of `size` goes in a plot of `plot` size: where it was
    /// placed, else beside its anchor; always inside the plot.
    pub fn place(&mut self, view: &View, size: (i32, i32), plot: (i32, i32)) -> (i32, i32) {
        let offset = *self.offset.get_or_insert_with(|| {
            let x = view.sample_to_x(self.anchor.0) + 8.0;
            let y = view.freq_to_y(self.anchor.1, f64::from(plot.1)) - f64::from(size.1) - 4.0;
            (x.round() as i32, y.round() as i32)
        });
        let offset = clamp_label_offset(offset, size, plot);
        self.offset = Some(offset);
        offset
    }
}

/// A label offset (from the plot's top left) that keeps a label of `size` inside a
/// plot of `plot` size.
pub fn clamp_label_offset(offset: (i32, i32), size: (i32, i32), plot: (i32, i32)) -> (i32, i32) {
    (
        offset.0.max(0).min((plot.0 - size.0).max(0)),
        offset.1.max(0).min((plot.1 - size.1).max(0)),
    )
}

/// The text of a Measurement of Freq result box.
pub fn freq_label(hz: f64) -> String {
    t!("sw-label-freq", value = format::dec(hz, 2))
}

/// The text of an ADP result box.
pub fn brate_label(baud: f64) -> String {
    t!("sw-label-brate", value = format::dec(baud, 2))
}

/// Where a result box for the selection in `m` is first placed: beside the top
/// right corner of the selection (a quarter of `fs` without H markers).
pub fn label_anchor(m: &Markers, fs: f64) -> (f64, f64) {
    let s = m.time_range().map_or(0.0, |(_, b)| b);
    let f = m.freq_range().map_or(fs / 4.0, |(_, hi)| hi);
    (s, f)
}

/// The status bar readout of a plot position, such as
/// `t = 00:12.345  f = 1800.30 Hz  -42 dB`, for a signal sampled at `fs` whose
/// 0 Hz is at `rf`.
pub fn cursor_readout(
    sample: f64,
    hz: Option<f64>,
    level_db: Option<f32>,
    fs: f64,
    rf: f64,
) -> String {
    let mut parts = vec![format!(
        "t = {}",
        format::time_short(sample / fs.max(f64::MIN_POSITIVE))
    )];
    if let Some(f) = hz {
        parts.push(format!("f = {} Hz", format::dec(f + rf, 2)));
    }
    if let Some(db) = level_db.filter(|d| d.is_finite()) {
        parts.push(format!("{} dB", format::dec(f64::from(db), 0)));
    }
    parts.join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_markers_leave_only_the_sa_tools_enabled() {
        // SA 6.2 (R283/a1): with both pairs collapsed only zoom reset, progress,
        // stop, play, the detectors, dT, IQ, inverse, R and S stay enabled.
        let collapsed = ToolState {
            complex: true,
            ..ToolState::default()
        };
        let enabled: Vec<Button> = BUTTONS
            .into_iter()
            .filter(|b| missing(*b, collapsed).is_none())
            .collect();
        assert_eq!(
            enabled,
            vec![
                Button::ZoomReset,
                Button::Play,
                Button::AmDetector,
                Button::ModifiedAm,
                Button::HalfWave,
                Button::Derivative,
                Button::Limiter,
                Button::IqToReal,
                Button::Inverse,
                Button::Resample,
                Button::Shift,
            ]
        );
        let all = ToolState {
            v: true,
            h: true,
            history: true,
            running: true,
            playing: true,
            complex: true,
        };
        assert!(BUTTONS.iter().all(|b| missing(*b, all).is_none()));
    }

    #[test]
    fn toolbar_lists_every_button_once_in_sa_order() {
        let order: Vec<Button> = BOTTOM_TOOLBAR
            .iter()
            .filter_map(|s| match s {
                Slot::Button(b) => Some(*b),
                _ => None,
            })
            .collect();
        assert_eq!(order, BUTTONS.to_vec());
        assert_eq!(
            BOTTOM_TOOLBAR.iter().position(|s| *s == Slot::Progress),
            Some(4)
        );
        for b in BUTTONS {
            assert_ne!(t!(b.hint_key()), b.hint_key(), "{b:?} has no hint");
        }
    }

    #[test]
    fn popup_menus_follow_sa_6_2() {
        use PopupItem::*;
        assert_eq!(
            popup_items(true, true),
            vec![
                SaveAs,
                Separator,
                WaveForm,
                MeasureFreq,
                Adp,
                Separator,
                CorrectBr,
                Cancel
            ]
        );
        assert_eq!(
            popup_items(false, false),
            vec![SaveAs, Separator, WaveForm, Separator, CorrectBr, Cancel]
        );
        assert_eq!(
            harmonics_popup_items(true, true),
            vec![MeasureFreq, Separator, Cancel]
        );
        assert_eq!(harmonics_popup_items(false, true), vec![Separator, Cancel]);
    }

    #[test]
    fn rs_buttons_check_the_value() {
        assert!(matches!(rs_run(true, "-5"), Run::Error(_)));
        assert!(matches!(rs_run(true, "0"), Run::Error(_)));
        assert_eq!(rs_run(false, "-5"), Run::Edit(Edit::Shift { hz: -5.0 }));
        assert_eq!(
            rs_run(true, "8000,5"),
            Run::Edit(Edit::Resample { rate: 8000.5 })
        );
        let Run::Error(why) = rs_run(true, "abc") else {
            panic!("abc accepted");
        };
        assert!(why.contains("abc"), "{why}");
    }

    #[test]
    fn info_overlay_lists_sa_values_in_sa_order() {
        let m = Markers {
            v: Some((0.0, 461_312.0)),
            h: Some((1516.0, 6411.0)),
            ..Markers::default()
        };
        let lines = info_lines(&m, 8000.0, 0.0, None, None);
        let names: Vec<&str> = lines.iter().filter_map(|l| l.split(" = ").next()).collect();
        assert_eq!(
            names,
            ["Fu", "Fc", "Fl", "dF", "t1", "t2", "dT", "S", "Br", "Sc"]
        );
        assert_eq!(lines[0], "Fu = 6411");
        assert_eq!(lines[6], "dT = 00:57.664.000");
        let mut c = Markers::default();
        collapse_markers(&mut c, 800.0, 1000.0);
        let lines = info_lines(&c, 8000.0, 0.0, Some(75.0), None);
        assert_eq!(lines[3], "dF = 0");
        assert_eq!(lines[6], "dT = 00:00.000.000");
    }

    #[test]
    fn handles_are_found_only_near_their_edges() {
        let view = View {
            start: 0.0,
            samples_per_px: 10.0,
            f_lo: 0.0,
            f_hi: 4000.0,
        };
        let m = Markers {
            v: Some((1000.0, 3000.0)),
            h: Some((1000.0, 3000.0)),
            ..Markers::default()
        };
        assert_eq!(
            handle_near(&m, &view, 200.0, (101.0, 3.0)),
            Some(MarkerId::V1)
        );
        assert_eq!(handle_near(&m, &view, 200.0, (101.0, 100.0)), None);
        assert_eq!(
            handle_near(&m, &view, 200.0, (3.0, 150.0)),
            Some(MarkerId::H1)
        );
        assert_eq!(
            marker_near(&m, &view, 200.0, (102.0, 100.0)),
            Some(MarkerId::V1)
        );
        assert_eq!(marker_near(&m, &view, 200.0, (150.0, 100.0)), None);
    }

    #[test]
    fn labels_stay_inside_the_plot() {
        assert_eq!(
            clamp_label_offset((500, -40), (80, 20), (300, 200)),
            (220, 0)
        );
        assert_eq!(clamp_label_offset((10, 10), (400, 300), (300, 200)), (0, 0));
        let view = View {
            start: 0.0,
            samples_per_px: 10.0,
            f_lo: 0.0,
            f_hi: 4000.0,
        };
        let mut l = FloatLabel::new("Freq: 1 Hz".into(), (1000.0, 2000.0));
        assert_eq!(l.place(&view, (60, 18), (300, 200)), (108, 78));
        assert_eq!(l.offset, Some((108, 78)));
    }

    #[test]
    fn value_presets_start_with_the_rate() {
        let p = value_presets(8000.0);
        assert!((p[0] - 8000.0).abs() < 1e-9);
        assert_eq!(p.iter().filter(|v| (**v - 8000.0).abs() < 1e-9).count(), 1);
        assert_eq!(image_text(50.0), "- 50 db");
    }
}
