//! SA's WF (waveform) window as plain data (`docs/SA_UI_SPEC.md` section 7):
//! the bottom toolbar's order and enable rules, the popup menu, the LPF field,
//! the short-interval frequency, the Info overlay, the place of a new
//! window's markers, the DC offset scroll bar and the SigNexora operations of the
//! popup's `SigNexora tools` submenu. The same rules as `sa-app`'s
//! `wf_window.rs` and `waveform.rs`, independent of the UI toolkit.

use sn_core::{SampleKind, SampleUnit, Signal};
use sn_dsp::ops::Edit;

use crate::edits::{MeasuredValue, Note, ParamKind, lowpass_default, median_default};
use crate::markers::Markers;
use crate::sono_tools::DEFAULT_BR;
use crate::view::View;
use crate::{format, t};

/// The bottom toolbar's buttons (SA 6.2.5.9, left to right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WfButton {
    /// Two arrows pointing outwards: stretch the time axis.
    ZoomIn,
    /// Two arrows pointing inwards: compress the time axis.
    ZoomOut,
    /// Up/down arrows: larger vertical scale.
    ScaleUp,
    /// Hourglass: smaller vertical scale.
    ScaleDown,
    /// `Z+`: zoom to the V markers.
    ZoomPlus,
    /// `Z-`: back one zoom step.
    ZoomMinus,
    /// Run the LPF (or RRC) filter on the window.
    Lpf,
    /// Revert to the original (undo the filter, clear the CCF view).
    Revert,
    /// Reverse FSK/MFSK synthesis.
    Synth,
    /// Save as a 16-bit WAV.
    Save,
    /// Copy the V fragment to a new window `(wfc0)`.
    Copy,
    /// Send the V fragment to the FSK/MFSK demodulator.
    Demod,
    /// Fit the vertical scale to the signal.
    Fit,
    /// ACF of the V fragment in a new window `(wf_acf)`.
    Acf,
    /// CCF of the V fragment, drawn over the source.
    Ccf,
}

/// One slot of the bottom toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WfSlot {
    /// A button.
    Button(WfButton),
    /// The wider gap SA leaves between groups.
    Gap,
    /// The progress field.
    Progress,
}

/// The bottom toolbar, left to right.
pub const WF_TOOLBAR: [WfSlot; 20] = [
    WfSlot::Button(WfButton::ZoomIn),
    WfSlot::Button(WfButton::ZoomOut),
    WfSlot::Button(WfButton::ScaleUp),
    WfSlot::Button(WfButton::ScaleDown),
    WfSlot::Button(WfButton::ZoomPlus),
    WfSlot::Button(WfButton::ZoomMinus),
    WfSlot::Gap,
    WfSlot::Progress,
    WfSlot::Gap,
    WfSlot::Button(WfButton::Lpf),
    WfSlot::Button(WfButton::Revert),
    WfSlot::Button(WfButton::Synth),
    WfSlot::Button(WfButton::Save),
    WfSlot::Button(WfButton::Copy),
    WfSlot::Gap,
    WfSlot::Button(WfButton::Demod),
    WfSlot::Button(WfButton::Fit),
    WfSlot::Button(WfButton::Acf),
    WfSlot::Button(WfButton::Ccf),
    WfSlot::Gap,
];

/// Every button of the bottom toolbar, in order.
pub const WF_BUTTONS: [WfButton; 15] = [
    WfButton::ZoomIn,
    WfButton::ZoomOut,
    WfButton::ScaleUp,
    WfButton::ScaleDown,
    WfButton::ZoomPlus,
    WfButton::ZoomMinus,
    WfButton::Lpf,
    WfButton::Revert,
    WfButton::Synth,
    WfButton::Save,
    WfButton::Copy,
    WfButton::Demod,
    WfButton::Fit,
    WfButton::Acf,
    WfButton::Ccf,
];

/// What the enable rules look at.
// Independent conditions, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WfTools {
    /// There is a zoom step to go back to.
    pub history: bool,
    /// The window shows a filtered signal or a CCF view.
    pub revert: bool,
    /// Tones and a baud rate are known (reverse synthesis).
    pub synth: bool,
    /// The window shows a result, not the loaded file (SA greys Save for `(0)`).
    pub derived: bool,
}

impl WfButton {
    /// Fluent key of the hint.
    pub const fn hint_key(self) -> &'static str {
        match self {
            Self::ZoomIn => "wf-hint-zoom-in",
            Self::ZoomOut => "wf-hint-zoom-out",
            Self::ScaleUp => "wf-hint-scale-up",
            Self::ScaleDown => "wf-hint-scale-down",
            Self::ZoomPlus => "wf-hint-zoom-plus",
            Self::ZoomMinus => "wf-hint-zoom-minus",
            Self::Lpf => "wf-hint-lpf",
            Self::Revert => "wf-hint-revert",
            Self::Synth => "wf-hint-synth",
            Self::Save => "wf-hint-save",
            Self::Copy => "wf-hint-copy",
            Self::Demod => "wf-hint-demod",
            Self::Fit => "wf-hint-fit",
            Self::Acf => "wf-hint-acf",
            Self::Ccf => "wf-hint-ccf",
        }
    }

    /// The tooltip: the hint, and why the button is disabled.
    pub fn tooltip(self, why_disabled: Option<&str>) -> String {
        crate::i18n::tooltip(self.hint_key(), why_disabled)
    }
}

/// What a disabled button lacks, or `None` when it is enabled. SA's WF keeps the
/// marker tools enabled with collapsed markers and answers `Please check settings!`.
pub fn missing(b: WfButton, s: WfTools) -> Option<String> {
    match b {
        WfButton::ZoomMinus => (!s.history).then(|| t!("sw-need-history")),
        WfButton::Revert => (!s.revert).then(|| t!("wf-need-revert")),
        WfButton::Synth => (!s.synth).then(|| t!("app-resynth.disabled")),
        WfButton::Save => (!s.derived).then(|| t!("wf-need-derived")),
        WfButton::ZoomIn
        | WfButton::ZoomOut
        | WfButton::ScaleUp
        | WfButton::ScaleDown
        | WfButton::ZoomPlus
        | WfButton::Lpf
        | WfButton::Copy
        | WfButton::Demod
        | WfButton::Fit
        | WfButton::Acf
        | WfButton::Ccf => None,
    }
}

/// Items of the SA part of the popup menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WfPopupItem {
    /// `Save as` with its formats.
    SaveAs,
    /// `Count freqs` (only with Sum on).
    CountFreqs,
    /// `Cancel`.
    Cancel,
}

/// SA 6.2.5's WF popup menu: `Save as`, `Count freqs` (only with Sum on), `Cancel`.
pub fn popup_items(sum: bool) -> Vec<WfPopupItem> {
    let mut items = vec![WfPopupItem::SaveAs];
    if sum {
        items.push(WfPopupItem::CountFreqs);
    }
    items.push(WfPopupItem::Cancel);
    items
}

/// SA's LPF presets (Hz); the field also takes any typed value.
pub const WF_LPF_PRESETS: [f64; 12] = [
    33.33, 50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 600.0, 1200.0, 2400.0, 3000.0, 4800.0,
];

/// The LPF value of a new window: Br when known, else SA's `33`.
pub fn lpf_default(br: Option<f64>) -> String {
    br.map_or_else(|| "33".to_owned(), |b| format::dec_trim(b, 2))
}

/// The LPF button: a low-pass at the LPF value, or an RRC filter for that baud rate
/// while RRC is on. `None` for a value that is not a cutoff below `fs / 2`.
pub fn filter_edit(lpf: &str, rrc: bool, alpha: f64, fs: f64) -> Option<Edit> {
    let v = format::parse_number(lpf).filter(|v| *v > 0.0 && *v < fs / 2.0)?;
    Some(if rrc {
        Edit::Rrc { baud: v, alpha }
    } else {
        Edit::Lowpass { cutoff: v }
    })
}

/// The RRC roll-off (SA's `0,50` by default) of a stored value (0 means unset).
pub fn rrc_alpha(stored: f64) -> f64 {
    if stored > 0.0 { stored } else { 0.5 }
}

/// The RRC roll-off at slider position `v` (0 to 1): 0.05 to 1 in steps of 0.01.
pub fn alpha_of_slider(v: f64) -> f64 {
    let a = (0.05 + v.clamp(0.0, 1.0) * 0.95).clamp(0.05, 1.0);
    (a * 100.0).round() / 100.0
}

/// The slider position (0 to 1) of roll-off `alpha`.
pub fn slider_of_alpha(alpha: f64) -> f64 {
    ((alpha - 0.05) / 0.95).clamp(0.0, 1.0)
}

/// Longest V fragment the short-interval frequency reads, in samples.
pub const SHORT_FREQ_MAX: usize = 4_000_000;

/// SA's short-interval frequency measurement: the frequency of `x` from its
/// crossings of the mean (first to last crossing), and the whole periods counted.
pub fn short_freq(x: &[f32], fs: f64) -> Option<(f64, usize)> {
    short_freq_with(x.len(), fs, |off, buf| {
        let src = x.get(off..).unwrap_or_default();
        let k = src.len().min(buf.len());
        if let (Some(d), Some(s)) = (buf.get_mut(..k), src.get(..k)) {
            d.copy_from_slice(s);
        }
        k
    })
}

/// [`short_freq`] over `n` samples that `read(offset, buf)` delivers in chunks
/// (returning how many it wrote): one pass for the mean, one for the crossings,
/// both through the same small buffer.
pub fn short_freq_with(
    n: usize,
    fs: f64,
    mut read: impl FnMut(usize, &mut [f32]) -> usize,
) -> Option<(f64, usize)> {
    const CHUNK: usize = 1 << 16;
    let mut buf = vec![0.0_f32; CHUNK.min(n.max(1))];
    let mut pass = |f: &mut dyn FnMut(usize, f32)| {
        let mut off = 0;
        while off < n {
            let want = (n - off).min(buf.len());
            let Some(chunk) = buf.get_mut(..want) else {
                break;
            };
            let got = read(off, chunk).min(want);
            for (i, v) in chunk.iter().take(got).enumerate() {
                f(off + i, *v);
            }
            if got < want {
                break;
            }
            off += got;
        }
    };
    let (mut sum, mut count) = (0.0_f64, 0_usize);
    pass(&mut |_, v| {
        sum += f64::from(v);
        count += 1;
    });
    let mean = sum / count.max(1) as f64;
    let (mut first, mut last, mut crossings) = (None::<f64>, 0.0_f64, 0_usize);
    let mut prev: Option<f64> = None;
    pass(&mut |i, v| {
        let q = f64::from(v) - mean;
        if let Some(p) = prev
            && p < 0.0
            && q >= 0.0
        {
            // Rising crossing, interpolated between the samples.
            let at = (i - 1) as f64 + p / (p - q);
            first.get_or_insert(at);
            last = at;
            crossings += 1;
        }
        prev = Some(q);
    });
    let first = first?;
    let periods = crossings.checked_sub(1).filter(|p| *p > 0)?;
    let span = (last - first) / fs;
    (span > 0.0).then(|| (periods as f64 / span, periods))
}

/// The frequency field's text for the V fragment of `markers` on `signal`:
/// `<frequency>-<whole periods>` (` Hz` after the frequency unless the signal
/// is already in Hz). `None` when the fragment is too short or too long.
pub fn short_freq_text(signal: &Signal, markers: &Markers) -> Option<String> {
    let (a, b) = markers.time_range()?;
    let (a, b) = (a.max(0.0) as usize, (b.max(0.0) as usize).min(signal.len()));
    let n = b
        .checked_sub(a)
        .filter(|n| *n >= 3 && *n <= SHORT_FREQ_MAX)?;
    // Read in fixed chunks into one reused buffer rather than allocating the
    // whole span (up to 16 MB) inside the click handler.
    let (hz, periods) = short_freq_with(n, signal.sample_rate(), |off, buf| {
        signal.read_real(a + off, buf)
    })?;
    let unit = if signal.unit() == SampleUnit::Hertz {
        ""
    } else {
        " Hz"
    };
    Some(format!("{}{unit}-{periods}", format::dec(hz, 2)))
}

/// The SA Info overlay of the WF window: `t1`, `t2`, `dT`, `S`, `Br`; a frequency
/// trace (after the phase detector) adds `Fu`, `Fc`, `Fl`, `dF` from the H markers
/// and the vertical resolution `Hz/pix` first.
pub fn info_lines(m: &Markers, fs: f64, br: Option<f64>, hz_per_px: Option<f64>) -> Vec<String> {
    let r = m.readout(fs);
    let mut lines = Vec::new();
    if let Some(per_px) = hz_per_px {
        let (lo, hi) = (r.f_lo.unwrap_or(0.0), r.f_hi.unwrap_or(0.0));
        lines.extend([
            format!("Fu = {}", format::dec_trim(hi, 2)),
            format!("Fc = {}", format::dec_trim(f64::midpoint(lo, hi), 2)),
            format!("Fl = {}", format::dec_trim(lo, 2)),
            format!("dF = {}", format::dec_trim(hi - lo, 2)),
            format!("Hz/pix = {}", format::dec_trim(per_px, 3)),
        ]);
    }
    let (t1, t2) = (r.t1.unwrap_or(0.0), r.t2.unwrap_or(0.0));
    lines.extend([
        format!("t1 = {}", format::time_short(t1)),
        format!("t2 = {}", format::time_short(t2)),
        format!("dT = {}", format::time_long(t2 - t1)),
        format!("S = {}", format::dec_trim(r.s.unwrap_or(0.0), 2)),
        format!("Br = {}", format::dec(br.unwrap_or(DEFAULT_BR), 2)),
    ]);
    lines
}

/// The markers of a new WF window, as SA places them: the V pair collapsed in
/// the middle of the signal, the H pair at the top and bottom of a plot showing
/// `offset - scale..=offset + scale`.
pub fn initial_markers(len: usize, offset: f64, scale: f64) -> Markers {
    let mid = len as f64 / 2.0;
    Markers {
        v: Some((mid, mid)),
        h: Some((offset + 0.98 * scale, offset - 0.98 * scale)),
        locked: None,
        active: None,
    }
}

/// The thumb position (0 at the top, 1 at the bottom) of a DC offset in
/// `-range..=range`: the top shows the highest offset.
pub fn offset_to_thumb(offset: f64, range: f64) -> f64 {
    if range <= 0.0 {
        return 0.5;
    }
    ((range - offset) / (2.0 * range)).clamp(0.0, 1.0)
}

/// The DC offset of thumb position `t` (inverse of [`offset_to_thumb`]).
pub fn thumb_to_offset(t: f64, range: f64) -> f64 {
    range - t.clamp(0.0, 1.0) * 2.0 * range
}

/// The visible sample span of a plot `width` pixels wide.
pub fn visible_span(view: &View, width: f64, len: usize) -> (usize, usize) {
    let a = view.x_to_sample(0.0).max(0.0) as usize;
    let b = (view.x_to_sample(width).max(0.0) as usize).min(len);
    (a.min(b), b)
}

/// Histograms (Sum) are computed for visible spans up to this many samples.
pub const MAX_SUM_SAMPLES: usize = 4_000_000;

/// Smallest and largest half range of the vertical scale.
pub const SCALE_LIMITS: (f32, f32) = (1e-6, 1e9);

/// The smoothing width of Find tones (Count freqs) for a vertical half range.
pub fn tones_smoothing(scale: f32) -> f64 {
    2.0 * f64::from(scale) / 400.0
}

/// SA's Count freqs result box (`Freq grid: 32` / `Delta: 75,12 Hz`), moved with
/// the left button and removed with a right click.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridLabel {
    /// The lines of the box.
    pub lines: Vec<String>,
    /// Offset of the box from the plot's top left; `None` until first placed
    /// (in the middle of the plot).
    pub offset: Option<(i32, i32)>,
}

/// The lines of a Count freqs box for a grid of `size` slots `spacing` Hz apart;
/// the Delta line only on a frequency (Hz) trace.
pub fn grid_lines(size: usize, spacing: f64, hz: bool) -> Vec<String> {
    let mut lines = vec![t!("wf-label-grid", size = size.to_string())];
    if hz {
        lines.push(t!("wf-label-delta", value = format::dec(spacing, 2)));
    }
    lines
}

/// One item of the `Operations` submenu of `SigNexora tools`.
#[derive(Debug, Clone, PartialEq)]
pub enum OpItem {
    /// A separator line.
    Separator,
    /// A section title (shown greyed).
    Section(String),
    /// An operation.
    Op {
        /// The menu text.
        label: String,
        /// What it does; `None` when disabled.
        run: Option<OpRun>,
    },
}

/// What an operation of the `Operations` submenu does.
#[derive(Debug, Clone, PartialEq)]
pub enum OpRun {
    /// Run the edit; the result opens in a new window.
    Edit(Edit),
    /// Ask for values first.
    Dialog(ParamKind, Vec<Note>),
    /// Reverse FSK/MFSK synthesis from the last tones and Br.
    Resynth,
}

/// What the `Operations` submenu needs to know.
#[derive(Debug, Clone, Copy)]
pub struct OpsCtx<'a> {
    /// The signal of the window.
    pub signal: &'a Signal,
    /// The signal's own (sonogram) markers.
    pub markers: &'a Markers,
    /// The latest Br with its source.
    pub br: Option<&'a MeasuredValue>,
    /// The latest Sc on this signal's axis.
    pub sc: Option<&'a MeasuredValue>,
    /// Tones and Br are known (reverse synthesis).
    pub resynth: bool,
}

fn op(label: String, run: Option<OpRun>) -> OpItem {
    OpItem::Op { label, run }
}

fn as_samples((a, b): (f64, f64)) -> (usize, usize) {
    (a.max(0.0) as usize, b.max(0.0) as usize)
}

/// `sa-app`'s signal operations of the WF popup (`SigNexora tools` >
/// `Operations`): the spectrum, whole-signal, time, filter, detector and ISD
/// sections, then resynthesis.
pub fn signal_ops(c: &OpsCtx<'_>) -> Vec<OpItem> {
    let mut out = spectrum_ops(c);
    out.extend(whole_ops(c));
    out.extend(time_filter_ops(c));
    out.extend(detector_ops(c));
    out.extend(isd_ops(c));
    out.push(OpItem::Separator);
    out.push(op(t!("app-resynth"), c.resynth.then_some(OpRun::Resynth)));
    out
}

fn spectrum_ops(c: &OpsCtx<'_>) -> Vec<OpItem> {
    let h = c.markers.freq_range();
    let v = c.markers.time_range();
    let band = |f: fn(f64, f64) -> Edit| h.map(|(lo, hi)| OpRun::Edit(f(lo, hi)));
    let area = match (h, v) {
        (Some((lo, hi)), Some(v)) => {
            let (start, end) = as_samples(v);
            Some(OpRun::Edit(Edit::AreaDelete { start, end, lo, hi }))
        }
        _ => None,
    };
    vec![
        OpItem::Section(t!("edits-section-spectrum")),
        op(
            t!("edits-band-select"),
            band(|lo, hi| Edit::BandSelect { lo, hi }),
        ),
        op(
            t!("edits-band-reject"),
            band(|lo, hi| Edit::BandReject { lo, hi }),
        ),
        op(t!("edits-area-delete"), area),
        op(
            t!("edits-shift-to-base"),
            band(|lo, hi| Edit::ShiftToBase {
                lo,
                hi,
                inverted: false,
            }),
        ),
        op(
            t!("edits-shift-to-base-inverted"),
            band(|lo, hi| Edit::ShiftToBase {
                lo,
                hi,
                inverted: true,
            }),
        ),
        op(
            t!("edits-invert-band"),
            band(|lo, hi| Edit::InvertBand { lo, hi }),
        ),
    ]
}

fn whole_ops(c: &OpsCtx<'_>) -> Vec<OpItem> {
    let fs = c.signal.sample_rate();
    let complex = c.signal.kind() == SampleKind::Complex;
    let measured = match (c.br, c.markers.readout(fs).s) {
        (Some(b), _) => Some((
            b.value,
            t!("edits-note-measured-br", from = b.from.as_str()),
        )),
        (None, Some(s)) => Some((s, t!("edits-note-measured-s"))),
        (None, None) => None,
    };
    let baud = measured.map(|(existing, note)| {
        OpRun::Dialog(
            ParamKind::Baud {
                existing,
                required: existing.round(),
            },
            vec![note.into()],
        )
    });
    vec![
        OpItem::Separator,
        OpItem::Section(t!("edits-section-whole")),
        op(
            t!("edits-shift"),
            Some(OpRun::Dialog(ParamKind::Shift { hz: 0.0 }, Vec::new())),
        ),
        op(t!("edits-invert"), Some(OpRun::Edit(Edit::Invert))),
        op(
            t!("edits-resample"),
            Some(OpRun::Dialog(ParamKind::Resample { rate: fs }, Vec::new())),
        ),
        op(
            t!("edits-iq-to-real"),
            complex.then_some(OpRun::Edit(Edit::IqToReal)),
        ),
        op(t!("edits-baud-correction"), baud),
        op(t!("edits-derivative"), Some(OpRun::Edit(Edit::Derivative))),
        op(t!("edits-limiter"), Some(OpRun::Edit(Edit::Limiter))),
    ]
}

fn time_filter_ops(c: &OpsCtx<'_>) -> Vec<OpItem> {
    let fs = c.signal.sample_rate();
    let v = c.markers.time_range();
    let copy = v.map(|v| {
        let (start, end) = as_samples(v);
        OpRun::Edit(Edit::Copy { start, end })
    });
    let (cutoff, note) = lowpass_default(fs, c.br.map(|b| (b.value, b.from.as_str())));
    let median = match c.br.and_then(|b| median_default(fs, b.value, &b.from)) {
        Some((window, note)) => OpRun::Dialog(ParamKind::Median { window }, vec![note]),
        None => OpRun::Dialog(
            ParamKind::Median { window: 5 },
            vec![t!("edits-note-window-default").into()],
        ),
    };
    let (baud, rrc_note) = match (c.br, c.markers.readout(fs).s) {
        (Some(b), _) => (b.value, t!("edits-note-br-from", from = b.from.as_str())),
        (None, Some(s)) => (s, t!("edits-note-br-s")),
        (None, None) => (fs / 10.0, t!("edits-note-br-guessed-10")),
    };
    vec![
        OpItem::Separator,
        OpItem::Section(t!("edits-section-time")),
        op(t!("edits-copy"), copy),
        OpItem::Separator,
        OpItem::Section(t!("edits-section-filters")),
        op(
            t!("edits-lowpass"),
            Some(OpRun::Dialog(ParamKind::Lowpass { cutoff }, vec![note])),
        ),
        op(t!("edits-median"), Some(median)),
        op(
            t!("edits-rrc"),
            Some(OpRun::Dialog(
                ParamKind::Rrc { baud, alpha: 0.35 },
                vec![rrc_note.into()],
            )),
        ),
    ]
}

fn detector_ops(c: &OpsCtx<'_>) -> Vec<OpItem> {
    let v = c.markers.time_range().map(as_samples);
    vec![
        OpItem::Separator,
        OpItem::Section(t!("edits-section-detectors")),
        op(t!("edits-am-detector"), Some(OpRun::Edit(Edit::AmDetector))),
        op(t!("edits-modified-am"), Some(OpRun::Edit(Edit::ModifiedAm))),
        op(t!("edits-half-wave"), Some(OpRun::Edit(Edit::HalfWave))),
        op(
            t!("edits-phase-detector"),
            Some(OpRun::Edit(Edit::PhaseDetector)),
        ),
        op(
            t!("edits-power"),
            Some(OpRun::Dialog(ParamKind::Power { n: 2 }, Vec::new())),
        ),
        op(
            t!("edits-acf"),
            v.map(|(start, end)| OpRun::Edit(Edit::Acf { start, end })),
        ),
        op(
            t!("edits-ccf"),
            v.map(|(start, end)| OpRun::Edit(Edit::Ccf { start, end })),
        ),
    ]
}

fn isd_ops(c: &OpsCtx<'_>) -> Vec<OpItem> {
    let fs = c.signal.sample_rate();
    let h = c.markers.freq_range();
    // Carrier: the measured Sc (power-N path), else the centre of the H band,
    // else a quarter of the rate.
    let (carrier, carrier_note) = match (c.sc, h) {
        (Some(sc), _) => (
            sc.value,
            Note {
                text: t!("edits-note-carrier-sc", from = sc.source()),
                warn: sc.other_axis,
            },
        ),
        (None, Some((lo, hi))) => (f64::midpoint(lo, hi), t!("edits-note-carrier-h").into()),
        (None, None) => (fs / 4.0, t!("edits-note-carrier-guessed").into()),
    };
    let (baud, baud_note) = match c.br {
        Some(b) => (
            b.value,
            Note {
                text: t!("edits-note-br-from", from = b.source()),
                warn: b.other_axis,
            },
        ),
        None => (fs / 8.0, t!("edits-note-br-guessed-8").into()),
    };
    let notes = vec![carrier_note, baud_note];
    let isd = |xn: Option<u32>| {
        Some(OpRun::Dialog(
            ParamKind::Equalize {
                carrier,
                baud,
                blocks: 30,
                xn,
            },
            notes.clone(),
        ))
    };
    vec![
        OpItem::Separator,
        OpItem::Section(t!("edits-section-isd")),
        op(t!("edits-isd-am"), isd(None)),
        op(t!("edits-isd-xn"), isd(Some(8))),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_lists_every_button_once_in_sa_order() {
        let order: Vec<WfButton> = WF_TOOLBAR
            .iter()
            .filter_map(|s| match s {
                WfSlot::Button(b) => Some(*b),
                _ => None,
            })
            .collect();
        assert_eq!(order, WF_BUTTONS.to_vec());
        let progress = WF_TOOLBAR.iter().position(|s| *s == WfSlot::Progress);
        assert_eq!(progress, Some(7));
        assert_eq!(WF_TOOLBAR[9], WfSlot::Button(WfButton::Lpf));
        for b in WF_BUTTONS {
            assert_ne!(t!(b.hint_key()), b.hint_key(), "{b:?} has no hint");
        }
    }

    #[test]
    fn enable_rules_follow_sa_wf() {
        let none = WfTools::default();
        let disabled: Vec<WfButton> = WF_BUTTONS
            .into_iter()
            .filter(|b| missing(*b, none).is_some())
            .collect();
        assert_eq!(
            disabled,
            vec![
                WfButton::ZoomMinus,
                WfButton::Revert,
                WfButton::Synth,
                WfButton::Save
            ]
        );
        let all = WfTools {
            history: true,
            revert: true,
            synth: true,
            derived: true,
        };
        assert!(WF_BUTTONS.iter().all(|b| missing(*b, all).is_none()));
    }

    #[test]
    fn popup_shows_count_freqs_only_with_sum() {
        use WfPopupItem::*;
        assert_eq!(popup_items(false), vec![SaveAs, Cancel]);
        assert_eq!(popup_items(true), vec![SaveAs, CountFreqs, Cancel]);
    }

    #[test]
    fn lpf_button_runs_lpf_or_rrc() {
        assert_eq!(
            filter_edit("75,5", false, 0.5, 8000.0),
            Some(Edit::Lowpass { cutoff: 75.5 })
        );
        assert_eq!(
            filter_edit("75", true, 0.35, 8000.0),
            Some(Edit::Rrc {
                baud: 75.0,
                alpha: 0.35
            })
        );
        assert_eq!(filter_edit("0", false, 0.5, 8000.0), None);
        assert_eq!(filter_edit("4000", false, 0.5, 8000.0), None);
        assert_eq!(filter_edit("x", false, 0.5, 8000.0), None);
        assert_eq!(lpf_default(None), "33");
        assert_eq!(lpf_default(Some(75.0)), "75");
        assert!((rrc_alpha(0.0) - 0.5).abs() < 1e-12);
        assert!((alpha_of_slider(slider_of_alpha(0.35)) - 0.35).abs() < 1e-12);
        assert!((alpha_of_slider(0.0) - 0.05).abs() < 1e-12);
        assert!((alpha_of_slider(1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn short_interval_frequency_counts_periods() {
        let fs = 8000.0;
        let x: Vec<f32> = (0..800)
            .map(|i| (2.0 * std::f64::consts::PI * 100.0 * f64::from(i) / fs + 0.3).sin() as f32)
            .collect();
        let (hz, periods) = short_freq(&x, fs).unwrap();
        assert!((hz - 100.0).abs() < 0.01, "{hz}");
        assert_eq!(periods, 9);
        assert!(short_freq(&[1.0, 1.0, 1.0], fs).is_none());
        let s = Signal::real(x, fs).unwrap();
        let m = Markers {
            v: Some((0.0, 800.0)),
            ..Markers::default()
        };
        let text = short_freq_text(&s, &m).unwrap();
        assert!(text.ends_with(" Hz-9"), "{text}");
        assert!(short_freq_text(&s, &Markers::default()).is_none());
    }

    #[test]
    fn info_overlay_follows_sa_wf() {
        let m = Markers {
            v: Some((8000.0, 16_000.0)),
            h: Some((2300.68, 1500.0)),
            ..Markers::default()
        };
        let lines = info_lines(&m, 8000.0, None, None);
        let names: Vec<&str> = lines.iter().filter_map(|l| l.split(" = ").next()).collect();
        assert_eq!(names, ["t1", "t2", "dT", "S", "Br"]);
        assert_eq!(lines[0], "t1 = 00:01.000");
        assert_eq!(lines[2], "dT = 00:01.000.000");
        assert_eq!(lines[3], "S = 1");
        let lines = info_lines(&m, 8000.0, Some(75.0), Some(3.414));
        assert_eq!(lines.len(), 10);
        assert!(lines[0].starts_with("Fu = 2300"));
        assert!(lines[4].starts_with("Hz/pix = 3"));
        let mut c = Markers::default();
        crate::sono_tools::collapse_markers(&mut c, 10.0, 0.0);
        assert_eq!(info_lines(&c, 8000.0, None, None)[3], "S = 0");
    }

    #[test]
    fn offset_bar_maps_both_ways() {
        assert!((offset_to_thumb(0.0, 2.0) - 0.5).abs() < 1e-12);
        assert!(
            (offset_to_thumb(2.0, 2.0)).abs() < 1e-12,
            "top shows +range"
        );
        assert!((offset_to_thumb(-9.0, 2.0) - 1.0).abs() < 1e-12);
        for t in [0.0, 0.25, 0.5, 1.0] {
            assert!((offset_to_thumb(thumb_to_offset(t, 3.0), 3.0) - t).abs() < 1e-12);
        }
    }

    #[test]
    fn new_window_markers_are_collapsed_with_h_at_the_edges() {
        let m = initial_markers(1000, 0.0, 1.0);
        assert_eq!(m.v, Some((500.0, 500.0)));
        assert!(m.time_range().is_none());
        let (lo, hi) = m.freq_range().unwrap_or_default();
        assert!(lo < -0.9 && hi > 0.9);
    }

    #[test]
    fn operations_need_their_markers() {
        let s = Signal::real(vec![0.0; 100], 8000.0).unwrap();
        let none = Markers::default();
        let ops = signal_ops(&OpsCtx {
            signal: &s,
            markers: &none,
            br: None,
            sc: None,
            resynth: false,
        });
        let enabled = |ops: &[OpItem], label: &str| {
            ops.iter()
                .any(|o| matches!(o, OpItem::Op { label: l, run: Some(_) } if l == label))
        };
        assert!(!enabled(&ops, &t!("edits-band-select")));
        assert!(!enabled(&ops, &t!("edits-copy")));
        assert!(!enabled(&ops, &t!("edits-iq-to-real")));
        assert!(enabled(&ops, &t!("edits-am-detector")));
        assert!(!enabled(&ops, &t!("app-resynth")));
        let m = Markers {
            v: Some((10.0, 50.0)),
            h: Some((100.0, 200.0)),
            ..Markers::default()
        };
        let ops = signal_ops(&OpsCtx {
            signal: &s,
            markers: &m,
            br: None,
            sc: None,
            resynth: true,
        });
        assert!(enabled(&ops, &t!("edits-band-select")));
        assert!(enabled(&ops, &t!("edits-copy")));
        assert!(enabled(&ops, &t!("app-resynth")));
        assert_eq!(grid_lines(8, 100.0, false).len(), 1);
        assert_eq!(grid_lines(8, 100.0, true)[0], "Freq grid: 8");
    }
}
