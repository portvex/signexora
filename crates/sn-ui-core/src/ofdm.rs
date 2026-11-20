//! State and logic of SA's OFDM module window, `SA-OFDM` (`SA_UI_SPEC.md` section
//! 10, report 7.9, 11.6 and 12.3): Find CT, ADP (SA's Get Br), the LU spectrum with
//! its channel markers, the per-channel constellations, Show, Each / Start+, the
//! models, the frequency and virtual shifts, the Survey (overview) mode and the List
//! of K, shared by the front ends (`sa-app`'s `ofdm_view.rs` holds the same
//! logic for the egui app).

use std::sync::Arc;
use std::time::Instant;

use sn_core::job::{self, JobError, JobHandle};
use sn_core::{Complex32, Signal};
use sn_ofdm::{
    Baseband, BrEstimate, BrParams, ChannelSymbols, CtCandidate, CtParams, CtResult, Grid,
    KCandidate, KParams, LuSpectrum, OrthoParams, SymbolView, Triangle, channel_symbols_with_stop,
    check_orthogonality, ct_triangle, find_ct, get_br, list_of_k, lu_spectrum,
};

use crate::format;
use crate::jobs::ofdm_to_job as fail;
use crate::t;

/// Survey FFT size (SA: always 1024).
pub const SURVEY_FFT: usize = 1024;
/// Symbols per second while Show plays.
pub const SHOW_RATE: f64 = 20.0;
/// Fewest symbols Show needs ahead (SA: 10) and the smallest Symbols value (SA: 6).
const SHOW_MIN_LEFT: usize = 10;
const MIN_SYMBOLS: usize = 6;
/// Candidates checked for orthogonality by List of K.
const K_CHECK: usize = 8;

/// Signal model (report 11.6): how the constellations read each channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Model {
    /// Every channel as generated: absolute values.
    A,
    /// Every channel is its own pilot: relative values.
    #[default]
    B,
    /// Data referenced to a pilot channel (the red marker's).
    C,
}

impl Model {
    /// Every model, in the `Mode` list's order.
    pub const ALL: [Self; 3] = [Self::A, Self::B, Self::C];

    /// The model's name in the `Mode` list.
    pub fn label(self) -> String {
        match self {
            Self::A => t!("ofdm-model-a"),
            Self::B => t!("ofdm-model-b"),
            Self::C => t!("ofdm-model-c"),
        }
    }
}

/// What the window asks the app to do.
#[derive(Debug, Clone, PartialEq)]
pub enum OfdmAction {
    /// Record the symbol rate from ADP as Br.
    RecordBr(f64),
    /// Resample the signal to this rate (a new window).
    Resample(f64),
    /// Show SA's error box.
    Error(String),
}

/// The work a job does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// Find CT.
    FindCt,
    /// ADP (SA's Get Br).
    Adp,
    /// List of K.
    ListK,
    /// Show.
    Show,
}

/// Finished background work.
enum Done {
    Ct(Arc<Baseband>, Box<CtResult>, usize),
    Br(Box<BrEstimate>),
    K(Vec<KCandidate>),
    Show(Box<ShowData>),
}

/// The symbols of the two marked channels for Show.
#[derive(Debug, Clone)]
pub struct ShowData {
    /// The symbols of the green and the red channel.
    pub symbols: ChannelSymbols,
    /// Display scale per channel (1.3 x the 90th percentile magnitude).
    pub scales: Vec<f32>,
}

/// How the List of K is sorted (its popup menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KSort {
    /// By the calculator's ranking (SA: AM).
    #[default]
    Am,
    /// By the native sample rate.
    Sr,
    /// By K.
    K,
}

/// What a cached triangle was computed for: origin, LS, LU and Symbols.
type TriangleKey = (usize, usize, usize, usize);

/// One line of the results list.
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// A candidate LS/LG found by Find CT.
    Candidate(CtCandidate),
    /// `Normal finish, time N sec.` (or stopped by the user).
    Finish {
        /// Stopped by the user.
        stopped: bool,
        /// Seconds the search took.
        secs: u64,
    },
    /// The search failed (the error box says why).
    Failed {
        /// Seconds the search took.
        secs: u64,
    },
}

/// Per-signal OFDM state: the window's controls and its results.
// The flags are SA's independent check boxes and view switches.
#[allow(clippy::struct_excessive_bools)]
pub struct OfdmState {
    /// `Min Br-LS` and `Max Br-LS` (integer Bd).
    pub min_br: u32,
    /// `Max Br-LS`.
    pub max_br: u32,
    /// The `Min Br-LS` field's text.
    pub min_text: String,
    /// The `Max Br-LS` field's text.
    pub max_text: String,
    /// `Survey` check box: the overview spectrum.
    pub survey: bool,
    /// Start of the survey FFT window, in samples from the selection start.
    pub survey_pos: f64,
    /// Symbols folded into the triangle (and ADP's block).
    pub symbols: usize,
    /// `Scale FFT`.
    pub scale_fft: usize,
    /// Frequency removed before the FFT (Shift Frequency), Hz.
    pub shift: f64,
    /// The Shift Frequency field's text.
    pub shift_text: String,
    /// Virtual shift in channel steps.
    pub vshift: i64,
    /// Br field: ADP's value, or a typed one.
    pub br_text: String,
    /// What the app last put in the Br field: leaving the field with this text
    /// unchanged applies nothing (it may be ADP's rounded value, or the "0" shown
    /// before there is a Br).
    br_shown: String,
    br_typed: Option<f64>,
    /// Signal model of the constellations (`Mode`).
    pub model: Model,
    /// `Each`: show every Nth symbol.
    pub each: usize,
    /// `Start+`: the first symbol Show shows.
    pub start_plus: usize,
    /// Symbol shown (Current LS outside Find CT).
    pub symbol: usize,
    /// Where the LU window sits on the good slope: samples before the peak.
    pub lu_back: Option<usize>,
    /// Channel markers: the selected (green) one and the other (red) one.
    pub sel: i64,
    /// The other (red) channel marker.
    pub other: i64,
    /// Spectrum pan (pixels from the first bin) and gain.
    pub pan: Option<f64>,
    /// Spectrum gain (the vertical scroll bar).
    pub gain: f64,
    /// Constellation panes shown.
    pub panes: bool,
    /// The List of K is shown instead of the results.
    pub k_shown: bool,
    /// How the List of K is sorted.
    pub k_sort: KSort,
    /// The List of K line picked.
    pub k_selected: Option<usize>,
    /// The results line in use.
    pub selected_line: Option<usize>,
    baseband: Option<Arc<Baseband>>,
    /// Search start used by Find CT (baseband index).
    search_start: usize,
    /// Find CT's result.
    pub ct: Option<CtResult>,
    /// The results list.
    pub lines: Vec<Line>,
    /// ADP's result.
    pub br: Option<BrEstimate>,
    /// (rank, candidate) from List of K.
    k_list: Vec<(usize, KCandidate)>,
    /// The symbols Show plays.
    pub show: Option<ShowData>,
    /// Show's position in them.
    pub show_pos: usize,
    /// Show is playing.
    pub playing: bool,
    play_clock: Option<Instant>,
    /// The triangle and the spectrum for their key, or why they could not be
    /// computed. A failure is kept for its key too, so it is not retried on every
    /// frame, and each plot draws only its own error.
    triangle: Option<(TriangleKey, Result<Triangle, String>)>,
    spectrum: Option<((Grid, usize), Result<LuSpectrum, String>)>,
    job: Option<(Work, Instant, JobHandle<Done>)>,
    initialised: bool,
}

impl Default for OfdmState {
    fn default() -> Self {
        Self {
            min_br: 20,
            max_br: 400,
            min_text: "20".to_owned(),
            max_text: "400".to_owned(),
            survey: true,
            survey_pos: 0.0,
            symbols: 30,
            scale_fft: 2,
            shift: 0.0,
            shift_text: format::dec(0.0, 4),
            vshift: 0,
            br_text: "0".to_owned(),
            br_shown: "0".to_owned(),
            br_typed: None,
            model: Model::B,
            each: 1,
            start_plus: 0,
            symbol: 0,
            lu_back: None,
            sel: 0,
            other: 0,
            pan: None,
            gain: 1.0,
            panes: false,
            k_shown: false,
            k_sort: KSort::Am,
            k_selected: None,
            selected_line: None,
            baseband: None,
            search_start: 0,
            ct: None,
            lines: Vec::new(),
            br: None,
            k_list: Vec::new(),
            show: None,
            show_pos: 0,
            playing: false,
            play_clock: None,
            triangle: None,
            spectrum: None,
            job: None,
            initialised: false,
        }
    }
}

impl std::fmt::Debug for OfdmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OfdmState")
            .field("min_br", &self.min_br)
            .field("max_br", &self.max_br)
            .finish_non_exhaustive()
    }
}

/// Display scale of one constellation: 1.3 x the 90th percentile magnitude.
pub fn constellation_scale(vals: &[Complex32]) -> f32 {
    let mut mags: Vec<f32> = vals
        .iter()
        .map(|v| v.norm())
        .filter(|m| m.is_finite())
        .collect();
    // Nearest rank: the smallest value with at least 90 % of the values at or
    // below it.
    let k = (mags.len() * 9).div_ceil(10).saturating_sub(1);
    let p90 = if k < mags.len() {
        *mags.select_nth_unstable_by(k, f32::total_cmp).1
    } else {
        1.0
    };
    p90.max(1e-12) * 1.3
}

/// LS for a Br bound: `round(fs / br)`.
pub fn ls_of(fs: f64, br: u32) -> usize {
    if br == 0 {
        0
    } else {
        (fs / f64::from(br)).round() as usize
    }
}

/// Greatest common divisor.
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// `LG/LU` in lowest terms (SA's Magic K).
pub fn magic_k(lg: usize, lu: usize) -> (usize, usize) {
    let g = gcd(lg, lu).max(1);
    (lg / g, lu / g)
}

/// A results-list line as SA writes it: `LS=496; LG=143; Br=22,228; Sh=31,232`.
pub fn candidate_line(c: &CtCandidate, fs: f64) -> String {
    t!(
        "ofdm-line",
        ls = c.ls.to_string(),
        lg = c.lg.to_string(),
        br = format::dec(fs / c.ls.max(1) as f64, 3),
        sh = format::dec(fs / c.lu.max(1) as f64, 3)
    )
}

/// A List of K line as SA writes it: `SR=10937,75; LS=245; LU=196; LG=49; K=1/4`.
pub fn k_line(c: &KCandidate) -> String {
    t!(
        "ofdm-k-line",
        sr = format::dec(c.sample_rate, 2),
        ls = c.ls.to_string(),
        lu = c.lu.to_string(),
        lg = c.lg.to_string(),
        num = c.k_num.to_string(),
        den = c.k_den.to_string()
    )
}

/// The List of K sorted as asked: (rank, candidate).
pub fn sorted_k(list: &[(usize, KCandidate)], by: KSort) -> Vec<(usize, KCandidate)> {
    let mut v = list.to_vec();
    match by {
        KSort::Am => v.sort_by_key(|(rank, _)| *rank),
        KSort::Sr => v.sort_by(|a, b| a.1.sample_rate.total_cmp(&b.1.sample_rate)),
        KSort::K => v.sort_by(|a, b| a.1.k.total_cmp(&b.1.k)),
    }
    v
}

/// The channel count and frequency span between the two markers (`CH_Sel`, `dFreqs`).
pub fn channel_span(sel: i64, other: i64, spacing: f64) -> (u64, f64) {
    let n = sel.abs_diff(other) + 1;
    (n, (sel - other).unsigned_abs() as f64 * spacing)
}

/// Results lines for a finished search: the candidates weakest first, so the
/// winner is last (SA lists each new best below the previous one), then the
/// finish line.
fn result_lines(ct: &CtResult, secs: u64) -> Vec<Line> {
    let mut c = ct.candidates.clone();
    c.sort_by(|a, b| a.score.total_cmp(&b.score));
    let mut lines: Vec<Line> = c.into_iter().map(Line::Candidate).collect();
    lines.push(Line::Finish {
        stopped: false,
        secs,
    });
    lines
}

/// Most samples (symbols x LS) a triangle drawn on the UI thread may use; the
/// lag product allocates one complex value per sample.
const TRIANGLE_BUDGET: usize = 1 << 20;

/// The Symbols count for a triangle shown in the window: at least 2, and few
/// enough that `symbols * ls` stays within [`TRIANGLE_BUDGET`].
fn display_symbols(symbols: usize, ls: usize) -> usize {
    symbols.max(2).min((TRIANGLE_BUDGET / ls.max(1)).max(2))
}

impl OfdmState {
    /// A job is running.
    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    /// The running work: (work, seconds since it started, progress 0..1 when
    /// the job reports it).
    pub fn running(&self) -> Option<(Work, f64, Option<f32>)> {
        self.job
            .as_ref()
            .map(|(w, t, h)| (*w, t.elapsed().as_secs_f64(), h.progress()))
    }

    /// Drop the baseband copy and stop a running job (the window was closed). The
    /// results and settings stay; the next step that needs the baseband asks for
    /// Find CT again.
    pub fn release(&mut self) {
        self.job = None;
        self.baseband = None;
        // Without the baseband the triangle and spectrum views would stay blank:
        // back to the survey view, and Find CT runs again.
        self.survey = true;
        self.playing = false;
        self.spectrum = None;
        self.triangle = None;
    }

    /// SA's starting bounds: `Min Br-LS` 20 and `Max Br-LS` at LS 20, the first time
    /// the window opens.
    pub fn init(&mut self, fs: f64) {
        if self.initialised {
            return;
        }
        self.initialised = true;
        let max = (fs / 20.0).round().clamp(21.0, 1e6) as u32;
        self.set_bounds(20, max);
    }

    /// Set the search bounds and their fields.
    pub fn set_bounds(&mut self, min: u32, max: u32) {
        self.min_br = min.max(1);
        self.max_br = max.max(self.min_br + 1);
        self.min_text = self.min_br.to_string();
        self.max_text = self.max_br.to_string();
    }

    /// Bounds around a measured Br (SigNexora: the results popup's `Bounds around Br`).
    pub fn around(&mut self, br: f64) {
        if br.is_finite() && br > 0.0 {
            self.set_bounds(
                (br * 0.8).floor().max(1.0) as u32,
                (br * 1.25).ceil() as u32,
            );
        }
    }

    /// The List of K, sorted as chosen.
    pub fn k_list(&self) -> Vec<(usize, KCandidate)> {
        sorted_k(&self.k_list, self.k_sort)
    }

    /// The symbol grid in use: ADP's (or Find CT's), with the typed Br, the LU
    /// position on the good slope, the frequency shift and the virtual shift.
    pub fn grid(&self) -> Option<Grid> {
        let mut g = match (&self.br, &self.ct) {
            (Some(br), _) => Grid::from_br(br),
            (None, Some(ct)) => Grid::from_ct(ct),
            (None, None) => return None,
        };
        if let Some(b) = self.br_typed.filter(|b| *b > 0.0) {
            let ls = g.sample_rate / b;
            if ls >= g.lu as f64 {
                g.ls = ls;
            }
        }
        let back = self.lu_back.unwrap_or(g.lg / 2).min(g.lg);
        g.window_offset = g.lg - back;
        g.freq_offset = self.shift;
        Some(g.with_virtual_shift(self.vshift))
    }

    /// Symbols in the selection on the current grid.
    pub fn symbol_count(&self) -> usize {
        match (&self.baseband, self.grid()) {
            (Some(bb), Some(g)) => g.symbol_count(bb.len()),
            _ => 0,
        }
    }

    /// Set the frequency shift and its field.
    pub fn set_shift(&mut self, hz: f64) {
        self.shift = hz;
        self.shift_text = format::dec(hz, 4);
        self.spectrum = None;
        self.clamp_symbol();
    }

    /// Apply a typed Br (the Br field, Enter). A value that is not a number, not
    /// positive, or above fs/LU (its LS would be shorter than LU) is refused with
    /// a message, and the field shows the Br in use again.
    pub fn type_br(&mut self, text: &str) -> Result<(), String> {
        // Leaving the field untouched changes nothing: its text is the Br in use
        // (ADP's, rounded for display) or the "0" shown before there is one.
        if text.trim() == self.br_shown.trim() {
            return Ok(());
        }
        let grid = self.grid();
        let max = grid.as_ref().map(|g| g.sample_rate / g.lu.max(1) as f64);
        match (format::parse_number(text), max) {
            (Some(b), Some(max)) if b > 0.0 && b <= max => {
                self.br_typed = Some(b);
                text.clone_into(&mut self.br_shown);
                self.clamp_symbol();
                Ok(())
            }
            (Some(b), None) if b > 0.0 => {
                self.br_typed = Some(b);
                text.clone_into(&mut self.br_shown);
                Ok(())
            }
            (_, max) => {
                let in_use = self
                    .br_typed
                    .or_else(|| self.br.as_ref().map(|b| b.br))
                    .or_else(|| grid.as_ref().map(|g| g.sample_rate / g.ls.max(1.0)))
                    .unwrap_or(0.0);
                self.show_br_text(format::dec(in_use, 6));
                let text = text.trim().to_owned();
                Err(max.map_or_else(
                    || t!("ofdm-br-invalid-nogrid", text = text.clone()),
                    |m| {
                        t!(
                            "ofdm-br-invalid",
                            text = text.clone(),
                            max = format::dec(m, 2)
                        )
                    },
                ))
            }
        }
    }

    /// Put `text` in the Br field as the app's own value.
    fn show_br_text(&mut self, text: String) {
        self.br_shown.clone_from(&text);
        self.br_text = text;
    }

    /// Keep the shown symbol inside the grid after it changed.
    fn clamp_symbol(&mut self) {
        let n = self.symbol_count();
        if n > 0 {
            self.symbol = self.symbol.min(n - 1);
        }
    }

    /// The triangle around symbol `self.symbol` (peak near the middle), recomputed
    /// when the symbol, the lengths or the Symbols count change. `None` when there
    /// is nothing to draw yet (no baseband or grid); `Some(Err)` says why it
    /// could not be computed for the current settings.
    pub fn triangle(&mut self) -> Option<Result<&Triangle, &str>> {
        let (bb, g) = (self.baseband.clone()?, self.grid()?);
        let ls = g.ls.round().max(1.0) as usize;
        let start = g.start + self.symbol as f64 * g.ls - ls as f64 / 2.0;
        let origin = start.round().max(0.0) as usize;
        let key = (origin, ls, g.lu, display_symbols(self.symbols, ls));
        if self.triangle.as_ref().is_none_or(|(k, _)| *k != key) {
            let t = ct_triangle(&bb, ls, g.lu, origin, key.3).map_err(|e| e.to_string());
            self.triangle = Some((key, t));
        }
        self.triangle
            .as_ref()
            .map(|(_, t)| t.as_ref().map_err(String::as_str))
    }

    /// The LU spectrum of the shown symbol, as [`Self::triangle`].
    pub fn spectrum(&mut self) -> Option<Result<&LuSpectrum, &str>> {
        let (bb, g) = (self.baseband.clone()?, self.grid()?);
        let key = (g, self.symbol);
        if self.spectrum.as_ref().is_none_or(|(k, _)| *k != key) {
            let s = lu_spectrum(&bb, &key.0, key.1).map_err(|e| e.to_string());
            self.spectrum = Some((key, s));
        }
        self.spectrum
            .as_ref()
            .map(|(_, s)| s.as_ref().map_err(String::as_str))
    }

    /// Move the LU marker to triangle index `m` (clamped to the good slope).
    pub fn place_lu(&mut self, m: usize) {
        let Some((_, Ok(t))) = &self.triangle else {
            return;
        };
        let (peak, ls, lg) = (t.peak, t.ls, t.lg);
        self.lu_back = Some(lu_back_at(m, peak, ls, lg));
        self.spectrum = None;
    }

    fn spawn(
        &mut self,
        work: Work,
        name: String,
        f: impl FnOnce(&(dyn Fn() -> bool + Sync)) -> Result<Done, JobError> + Send + 'static,
    ) {
        if let Some((_, _, h)) = self.job.take() {
            h.cancel();
        }
        self.job = Some((
            work,
            Instant::now(),
            job::spawn(name, move |ctx| {
                let stop = || ctx.is_cancelled();
                f(&stop)
            }),
        ));
    }

    /// Find CT on samples `span` of `signal`, from the survey position when Survey
    /// is on.
    pub fn start_ct(&mut self, signal: &Signal, span: (usize, usize)) {
        let s = signal.clone();
        let start = if self.survey {
            self.survey_pos.max(0.0) as usize
        } else {
            0
        };
        let params = CtParams {
            min_br: f64::from(self.min_br),
            max_br: f64::from(self.max_br),
            symbols: self.symbols.max(3),
            start,
        };
        self.spawn(Work::FindCt, t!("ofdm-job-ct"), move |stop| {
            let bb = Baseband::from_signal(&s, span.0, span.1, stop).map_err(|e| fail(&e))?;
            let ct = find_ct(&bb, &params, stop).map_err(|e| fail(&e))?;
            Ok(Done::Ct(Arc::new(bb), Box::new(ct), start))
        });
    }

    /// ADP: the precise Br and the frequency position.
    pub fn start_adp(&mut self) -> Result<(), String> {
        let (Some(bb), Some(ct)) = (self.baseband.clone(), self.ct.clone()) else {
            return Err(t!("ofdm-needs-ct"));
        };
        let params = BrParams {
            block: self.symbols.max(MIN_SYMBOLS),
            ..BrParams::default()
        };
        self.spawn(Work::Adp, t!("ofdm-job-br"), move |stop| {
            get_br(&bb, &ct, &params, stop)
                .map(|b| Done::Br(Box::new(b)))
                .map_err(|e| fail(&e))
        });
        Ok(())
    }

    /// List of K (stage 1, then the orthogonality check of the top candidates).
    pub fn start_k(&mut self) -> Result<(), String> {
        let (Some(bb), Some(br)) = (self.baseband.clone(), self.br.clone()) else {
            return Err(t!("ofdm-needs-br"));
        };
        self.spawn(Work::ListK, t!("ofdm-job-k"), move |stop| {
            let list = list_of_k(&KParams::from_br(&br)).map_err(|e| fail(&e))?;
            let p = OrthoParams {
                top: K_CHECK,
                ..OrthoParams::default()
            };
            check_orthogonality(&bb, &br, &list, &p, stop)
                .map(Done::K)
                .map_err(|e| fail(&e))
        });
        Ok(())
    }

    /// Show: the two marked channels symbol by symbol from `Start+`, every `Each`.
    pub fn start_show(&mut self) -> Result<(), String> {
        let (Some(bb), Some(grid)) = (self.baseband.clone(), self.grid()) else {
            return Err(t!("ofdm-needs-ct"));
        };
        let total = grid.symbol_count(bb.len());
        let first = self.start_plus;
        if self.symbols < MIN_SYMBOLS || total < first + SHOW_MIN_LEFT {
            return Err(t!("ofdm-check-settings"));
        }
        let every = self.each.max(1);
        let count = (total - first).div_ceil(every);
        let view = match self.model {
            Model::A => SymbolView::Absolute,
            Model::B => SymbolView::Relative,
            Model::C => SymbolView::PilotReferenced { pilot: self.other },
        };
        let bins = vec![self.sel, self.other];
        self.spawn(Work::Show, t!("ofdm-job-show"), move |stop| {
            let symbols =
                channel_symbols_with_stop(&bb, &grid, &bins, first, count, every, view, stop)
                    .map_err(|e| fail(&e))?;
            let scales = symbols
                .values
                .iter()
                .map(|v| constellation_scale(v))
                .collect();
            Ok(Done::Show(Box::new(ShowData { symbols, scales })))
        });
        Ok(())
    }

    /// Break: stop the running job or the playback.
    pub fn stop(&mut self) {
        if let Some((work, t, h)) = self.job.take() {
            h.cancel();
            if work == Work::FindCt {
                self.drop_end_lines();
                self.lines.push(Line::Finish {
                    stopped: true,
                    secs: t.elapsed().as_secs(),
                });
            }
        }
        self.playing = false;
        self.play_clock = None;
    }

    /// Drop the stopped or failed end lines added after the kept result, so a
    /// new one replaces them: at most one such line follows the last normal finish.
    fn drop_end_lines(&mut self) {
        let keep = self
            .lines
            .iter()
            .rposition(|l| matches!(l, Line::Finish { stopped: false, .. }))
            .map_or(0, |i| i + 1);
        let mut i = 0;
        self.lines.retain(|l| {
            let stale =
                i >= keep && matches!(l, Line::Failed { .. } | Line::Finish { stopped: true, .. });
            i += 1;
            !stale
        });
    }

    /// Quick recompute of results line `i` (SA): the triangle for that LS and LU.
    pub fn pick_line(&mut self, i: usize) -> Result<(), String> {
        let Some(Line::Candidate(c)) = self.lines.get(i).cloned() else {
            return Ok(());
        };
        if self.is_running() {
            // A running ADP or List of K works on the current CT: changing it
            // under the job would store its result on the wrong grid.
            return Err(t!("ofdm-pick-busy"));
        }
        let (Some(bb), Some(ct)) = (self.baseband.clone(), self.ct.as_mut()) else {
            return Err(t!("ofdm-baseband-freed"));
        };
        let tri = ct_triangle(
            &bb,
            c.ls,
            c.lu,
            self.search_start,
            display_symbols(self.symbols, c.ls),
        )
        .map_err(|e| e.to_string())?;
        ct.ls = c.ls;
        ct.lu = c.lu;
        ct.lg = c.lg;
        ct.start = self.search_start + tri.peak;
        ct.triangle = tri;
        self.selected_line = Some(i);
        self.after_new_grid();
        Ok(())
    }

    /// Clear the results (SA: double-click in the results field).
    pub fn reset(&mut self) {
        self.ct = None;
        self.br = None;
        self.lines.clear();
        self.k_list.clear();
        self.k_shown = false;
        self.show = None;
        self.panes = false;
        self.playing = false;
        self.selected_line = None;
        self.show_br_text("0".to_owned());
        self.br_typed = None;
        self.triangle = None;
        self.spectrum = None;
    }

    /// A new grid: ADP, the K list and Show start again.
    fn after_new_grid(&mut self) {
        self.br = None;
        self.br_typed = None;
        self.show_br_text("0".to_owned());
        self.k_list.clear();
        self.k_shown = false;
        self.show = None;
        self.playing = false;
        self.lu_back = None;
        self.symbol = 0;
        self.vshift = 0;
        self.triangle = None;
        self.spectrum = None;
        if let Some(ct) = &self.ct {
            let g = Grid::from_ct(ct);
            self.set_shift(g.freq_offset);
        }
    }

    /// Advance Show by the time passed. Returns true while playing.
    pub fn tick(&mut self) -> bool {
        if !self.playing {
            return false;
        }
        let Some(data) = &self.show else {
            self.playing = false;
            return false;
        };
        let n = data.symbols.symbols.len();
        let clock = self.play_clock.get_or_insert_with(Instant::now);
        let due = (clock.elapsed().as_secs_f64() * SHOW_RATE) as usize;
        if due > 0 {
            // Keep the fraction of a symbol already elapsed, so the rate does
            // not depend on the frame rate.
            let used = std::time::Duration::from_secs_f64(due as f64 / SHOW_RATE);
            *clock = clock.checked_add(used).unwrap_or_else(Instant::now);
            self.show_pos = (self.show_pos + due).min(n.saturating_sub(1));
        }
        if let Some(s) = data.symbols.symbols.get(self.show_pos) {
            self.symbol = *s;
        }
        if self.show_pos + 1 >= n {
            self.playing = false;
            self.play_clock = None;
        }
        self.playing
    }

    /// Poll the job. Returns what the app should do.
    pub fn poll(&mut self) -> Option<OfdmAction> {
        if !self.job.as_ref().is_some_and(|(_, _, h)| h.is_finished()) {
            return None;
        }
        let (work, started, h) = self.job.take()?;
        match h.join() {
            Ok(done) => self.finished(done, started),
            Err(JobError::Cancelled) => None,
            Err(e) => {
                if work == Work::FindCt {
                    // Not "stopped by the user": the search failed. The earlier
                    // search's lines stay (its grid is still the one in the plots
                    // and Show), and the failure is added after them.
                    self.drop_end_lines();
                    self.lines.push(Line::Failed {
                        secs: started.elapsed().as_secs(),
                    });
                }
                Some(OfdmAction::Error(e.to_string()))
            }
        }
    }

    fn finished(&mut self, done: Done, started: Instant) -> Option<OfdmAction> {
        match done {
            Done::Ct(bb, ct, start) => {
                self.baseband = Some(bb);
                self.search_start = start;
                self.lines = result_lines(&ct, started.elapsed().as_secs());
                self.selected_line = self.lines.len().checked_sub(2);
                self.ct = Some(*ct);
                self.survey = false;
                self.pan = None;
                self.panes = false;
                self.after_new_grid();
                None
            }
            Done::Br(br) => {
                let rate = br.br;
                self.show_br_text(format::dec(rate, 6));
                self.br_typed = None;
                self.set_shift(br.freq_offset);
                self.br = Some(*br);
                // The K list and Show were made on the old grid.
                self.k_list.clear();
                self.k_shown = false;
                self.k_selected = None;
                self.show = None;
                self.playing = false;
                self.play_clock = None;
                self.triangle = None;
                self.clamp_symbol();
                Some(OfdmAction::RecordBr(rate))
            }
            Done::K(list) => {
                self.k_list = list.into_iter().enumerate().collect();
                self.k_shown = true;
                self.k_selected = None;
                None
            }
            Done::Show(data) => {
                self.show_pos = 0;
                self.show = Some(*data);
                self.panes = true;
                self.playing = true;
                self.play_clock = None;
                None
            }
        }
    }

    /// Move the selected (green) marker by `d` channels (arrow keys in dynamic mode).
    pub fn step_marker(&mut self, d: i64) {
        let Some(g) = self.grid() else {
            return;
        };
        let bins = g.bins();
        let (lo, hi) = (
            bins.first().copied().unwrap_or(0),
            bins.last().copied().unwrap_or(0),
        );
        self.sel = (self.sel + d).clamp(lo, hi);
    }
}

/// Shift Frequency digit steps, Hz.
pub const SHIFT_STEPS: [f64; 4] = [10.0, 1.0, 0.1, 0.01];

/// Pixels per channel for a Scale FFT value.
pub fn pitch(scale: usize) -> f64 {
    scale.max(1) as f64 * 1.75
}

/// The spectrum pan a window starts with: at DC for a real recording (its
/// channels are all above DC), else at the first channel.
pub fn default_pan(signal: &Signal, lu: usize, pitch: f64) -> f64 {
    if signal.kind() == sn_core::SampleKind::Real {
        (lu / 2) as f64 * pitch
    } else {
        0.0
    }
}

/// The text of results line `line` (SA's formats).
pub fn line_text(line: &Line, fs: f64) -> String {
    match line {
        Line::Candidate(cand) => candidate_line(cand, fs),
        Line::Finish {
            stopped: false,
            secs,
        } => t!("ofdm-finish", secs = secs.to_string()),
        Line::Finish {
            stopped: true,
            secs,
        } => t!("ofdm-stopped", secs = secs.to_string()),
        Line::Failed { secs } => t!("ofdm-failed", secs = secs.to_string()),
    }
}

/// The hint of a List of K line: K, the LU error, ICI and even or odd LU.
pub fn k_hint(c: &KCandidate) -> String {
    let ici = c
        .ici_db
        .map_or_else(|| "-".to_owned(), |v| format::dec(v, 1));
    t!(
        "ofdm-k.hover",
        k = format::dec(c.k, 5),
        err = format::dec(c.lu_error, 4),
        ici = ici,
        lu = if c.even_lu {
            t!("ofdm-k-lu-even")
        } else {
            t!("ofdm-k-lu-odd")
        }
    )
}

/// What the wait screen says is computed.
pub fn wait_note(work: Work) -> String {
    match work {
        Work::FindCt => t!("ofdm-wait-ct"),
        Work::Adp => t!("ofdm-wait-adp"),
        Work::ListK | Work::Show => t!("ofdm-wait-k"),
    }
}

/// The wait screen's `Remained Time` line and the fraction its green blocks
/// show: the job's progress, or a running fraction when it reports none.
pub fn wait_progress(secs: f64, progress: Option<f32>) -> (Option<String>, f32) {
    match progress {
        Some(p) if p > 0.0 => {
            let left = secs * f64::from((1.0 - p) / p);
            (Some(t!("ofdm-wait-left", secs = format!("{left:.0}"))), p)
        }
        _ => (None, (secs as f32 * 0.25).fract()),
    }
}

/// The Survey spectrum (dB, `SURVEY_FFT / 2` bins or all of them for a
/// complex signal) of the window starting `start` samples into `signal`.
pub fn survey_db(signal: &Signal, start: usize) -> Vec<f32> {
    use sn_dsp::spectrum::SpectrumEngine;
    use sn_dsp::window::WindowKind;
    SpectrumEngine::new(SURVEY_FFT, WindowKind::Hann, signal.kind())
        .map(|e| e.frame_db(signal, start))
        .unwrap_or_default()
}

impl OfdmState {
    /// The cyan info block over the triangle (SA's formats).
    pub fn info_lines(&self, fs: f64) -> Vec<String> {
        let Some(g) = self.grid() else {
            return Vec::new();
        };
        let mut lines = vec![
            t!("ofdm-info-br", br = format::dec(fs / g.ls.max(1e-9), 3)),
            t!("ofdm-info-sh", sh = format::dec(g.spacing(), 3)),
        ];
        if self.br.is_some() {
            let (num, den) = magic_k(g.lg, g.lu);
            lines.push(t!(
                "ofdm-info-magic",
                num = num.to_string(),
                den = den.to_string()
            ));
            lines.push(t!(
                "ofdm-info-k",
                k = format::dec(g.lg as f64 / g.lu.max(1) as f64, 8)
            ));
        } else {
            lines.extend([String::new(), String::new()]);
        }
        lines.push(t!("ofdm-info-lg", lg = g.lg.to_string()));
        lines.push(t!("ofdm-info-lu", lu = g.lu.to_string()));
        let freq = |b: i64| format::dec_trim(g.bin_frequency(b), 3);
        lines.push(t!(
            "ofdm-info-up",
            ch = self.sel.to_string(),
            freq = freq(self.sel)
        ));
        lines.push(t!(
            "ofdm-info-down",
            ch = self.other.to_string(),
            freq = freq(self.other)
        ));
        let (count, df) = channel_span(self.sel, self.other, g.spacing());
        lines.push(t!(
            "ofdm-info-sel",
            count = count.to_string(),
            df = format::dec_trim(df, 3)
        ));
        if self.vshift != 0 {
            lines.push(t!("ofdm-info-vshift", bins = self.vshift.to_string()));
        }
        lines
    }

    /// `Current LS`: the symbol shown, 0 while Find CT runs.
    pub fn current_ls(&self) -> usize {
        match self.running() {
            Some((Work::FindCt, ..)) => 0,
            _ => self.symbol,
        }
    }

    /// The plots show the Survey spectrum (Survey on, or no Find CT yet).
    pub fn surveying(&self) -> bool {
        self.survey || self.ct.is_none()
    }

    /// `Show` reads `Break` while anything runs or plays.
    pub fn stoppable(&self) -> bool {
        self.is_running() || self.playing
    }

    /// Restart Show after the green marker moved (arrow keys), when Show was
    /// shown and nothing but Show runs (replacing an ADP or List of K job
    /// would cancel it without a word).
    pub fn restart_show(&mut self) -> Result<(), String> {
        if self.show.is_some() && self.running().is_none_or(|(w, ..)| w == Work::Show) {
            self.start_show()
        } else {
            Ok(())
        }
    }

    /// A click on spectrum channel `b`: the nearer marker moves there; a
    /// double click places both.
    pub fn click_channel(&mut self, b: i64, double: bool) {
        if double {
            self.sel = b;
            self.other = b;
        } else if b.abs_diff(self.sel) <= b.abs_diff(self.other) {
            self.sel = b;
        } else {
            self.other = b;
        }
    }

    /// Apply the typed Shift Frequency. Untouched text changes nothing; an
    /// invalid value puts the shift in use back and returns why.
    pub fn type_shift(&mut self, text: &str) -> Result<(), String> {
        if text.trim() == format::dec(self.shift, 4) {
            return Ok(());
        }
        if let Some(v) = format::parse_number(text) {
            self.set_shift(v);
            Ok(())
        } else {
            let v = self.shift;
            self.set_shift(v);
            Err(t!("ofdm-shift-invalid", text = text.trim().to_owned()))
        }
    }
}

/// The LU marker's distance back from the peak (`0..=lg`) for a click on
/// triangle index `m`. It inverts the drawing position `(peak + ls - back) % ls`,
/// so a click past the peak is on the wrapped part; a click outside the good
/// slope goes to its nearer end (just right of the peak is 0, just left of the
/// slope is `lg`).
pub fn lu_back_at(m: usize, peak: usize, ls: usize, lg: usize) -> usize {
    let ls = ls.max(1);
    let raw = (peak % ls + ls - m % ls) % ls;
    if raw <= lg {
        raw
    } else if ls - raw < raw - lg {
        0
    } else {
        lg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_k_and_channel_span() {
        assert_eq!(magic_k(143, 353), (143, 353));
        assert_eq!(magic_k(26, 108), (13, 54));
        assert_eq!(magic_k(0, 64), (0, 1));
        let (n, df) = channel_span(7, 5, 98.765_5);
        assert_eq!(n, 3);
        assert!((df - 197.531).abs() < 1e-9);
        assert_eq!(channel_span(0, 0, 10.0).0, 1);
    }

    #[test]
    fn bounds_follow_sa() {
        let mut st = OfdmState::default();
        st.init(11025.0);
        assert_eq!((st.min_br, st.max_br), (20, 551));
        assert_eq!(ls_of(11025.0, 20), 551);
        assert_eq!(ls_of(11025.0, 551), 20);
        assert_eq!(ls_of(1.0, 0), 0);
        st.around(100.0);
        assert_eq!((st.min_br, st.max_br), (80, 125));
        assert_eq!(st.max_text, "125");
    }

    #[test]
    fn lines_are_written_as_sa_writes_them() {
        let c = CtCandidate {
            ls: 496,
            lu: 353,
            lg: 143,
            coherence: 0.9,
            score: 10.0,
        };
        let line = candidate_line(&c, 11025.0);
        assert!(line.starts_with("LS=496; LG=143; Br=22"), "{line}");
        let k = KCandidate {
            k_num: 1,
            k_den: 4,
            k: 0.25,
            ls: 245,
            lu: 196,
            lg: 49,
            sample_rate: 10937.75,
            lu_error: 0.0,
            even_lu: true,
            hits: 1,
            score: 0.0,
            leakage_db: None,
            ici_db: None,
        };
        let line = k_line(&k);
        assert!(line.contains("LS=245; LU=196; LG=49; K=1/4"), "{line}");
        let mut b = k.clone();
        b.k = 0.1;
        b.sample_rate = 9000.0;
        let list = vec![(0, k), (1, b)];
        assert_eq!(sorted_k(&list, KSort::Am)[0].0, 0);
        assert_eq!(sorted_k(&list, KSort::Sr)[0].0, 1);
        assert_eq!(sorted_k(&list, KSort::K)[0].0, 1);
    }

    /// Poll until the job ends; the action it gave, if any.
    fn finish(st: &mut OfdmState) -> Option<OfdmAction> {
        for _ in 0..12_000 {
            if let Some(a) = st.poll() {
                return Some(a);
            }
            if !st.is_running() {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("job did not finish");
    }

    #[test]
    fn find_ct_adp_list_of_k_and_show_on_a_known_signal() {
        // OCG test signal of article 371: LU 64, LG 20, Fd 7000.
        let spec = sn_synth::GeneratorSpec::preset("ofdm_lu64_lg20_7000").unwrap();
        let signal = sn_synth::generate(&spec).unwrap().signal;
        let mut st = OfdmState::default();
        st.init(signal.sample_rate());
        st.start_ct(&signal, (0, signal.len()));
        assert_eq!(finish(&mut st), None);
        let ct = st.ct.clone().unwrap();
        assert_eq!((ct.ls, ct.lu, ct.lg), (84, 64, 20));
        assert!(!st.survey, "Find CT leaves the Survey view");
        assert!(matches!(
            st.lines.last(),
            Some(Line::Finish { stopped: false, .. })
        ));
        assert!(matches!(st.triangle(), Some(Ok(_))));
        assert!(matches!(st.spectrum(), Some(Ok(_))));
        st.start_adp().unwrap();
        let Some(OfdmAction::RecordBr(br)) = finish(&mut st) else {
            panic!("ADP records Br");
        };
        assert!((br - 7000.0 / 84.0).abs() < 0.05, "{br}");
        assert!(st.br_text.starts_with("83"));
        // Leaving the untouched field keeps ADP's exact Br, not its rounded text.
        let shown = st.br_text.clone();
        assert_eq!(st.type_br(&shown), Ok(()));
        assert_eq!(st.br_typed, None);
        st.start_k().unwrap();
        assert_eq!(finish(&mut st), None);
        assert!(st.k_shown && !st.k_list().is_empty());
        st.sel = 5;
        st.other = 3;
        st.start_show().unwrap();
        assert_eq!(finish(&mut st), None);
        let show = st.show.clone().unwrap();
        assert_eq!(show.symbols.bins, vec![5, 3]);
        assert!(st.playing && st.panes);
        st.stop();
        assert!(!st.playing);
        // Quick recompute of a results line keeps the window consistent.
        st.pick_line(0).unwrap();
        assert!(st.br.is_none() && st.ct.is_some());
    }

    #[test]
    fn untouched_br_field_applies_nothing_and_no_grid_message_has_no_limit() {
        let mut st = OfdmState::default();
        // Click in and out of the default "0": nothing typed, nothing refused.
        assert_eq!(st.type_br("0"), Ok(()));
        assert_eq!((st.br_typed, st.br_text.as_str()), (None, "0"));
        // Before Find CT there is no fs/LU limit to name.
        assert_eq!(
            st.type_br("12a"),
            Err(t!("ofdm-br-invalid-nogrid", text = "12a"))
        );
        assert_eq!(st.br_typed, None);
    }

    #[test]
    fn repeated_failures_and_stops_keep_one_end_line() {
        let mut st = OfdmState {
            lines: vec![
                Line::Finish {
                    stopped: false,
                    secs: 1,
                },
                Line::Failed { secs: 2 },
                Line::Finish {
                    stopped: true,
                    secs: 3,
                },
            ],
            ..OfdmState::default()
        };
        st.drop_end_lines();
        st.lines.push(Line::Failed { secs: 4 });
        assert_eq!(
            st.lines,
            vec![
                Line::Finish {
                    stopped: false,
                    secs: 1
                },
                Line::Failed { secs: 4 }
            ]
        );
    }

    #[test]
    fn show_refuses_too_few_symbols() {
        let mut st = OfdmState::default();
        assert_eq!(st.start_show(), Err(t!("ofdm-needs-ct")), "no Find CT yet");
        let Some(spec) = sn_synth::GeneratorSpec::preset("ofdm_lu64_lg20_7000") else {
            panic!("preset");
        };
        let Ok(made) = sn_synth::generate(&spec) else {
            panic!("generate");
        };
        let signal = made.signal;
        st.init(signal.sample_rate());
        st.start_ct(&signal, (0, signal.len()));
        for _ in 0..3000 {
            let _ = st.poll();
            if !st.is_running() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(st.ct.is_some(), "Find CT found the grid");
        // Too few symbols.
        let symbols = st.symbols;
        st.symbols = 3;
        assert_eq!(st.start_show(), Err(t!("ofdm-check-settings")));
        // Too few symbols left after Start+.
        st.symbols = symbols;
        st.start_plus = st.symbol_count().saturating_sub(SHOW_MIN_LEFT - 1);
        assert_eq!(st.start_show(), Err(t!("ofdm-check-settings")));
        st.start_plus = 0;
        assert_eq!(st.start_show(), Ok(()));
        st.stop();
    }

    #[test]
    fn clicking_past_the_peak_maps_to_the_wrapped_marker() {
        let ls = 100;
        let (peak, lg) = (5_usize, 30_usize);
        // Drawn at (peak + ls - back) % ls; the inverse used by place_lu.
        for back in 0..=lg {
            let m = (peak + ls - back) % ls;
            assert_eq!(lu_back_at(m, peak, ls, lg), back);
        }
        // Just right of the peak: the nearer end is the peak itself.
        assert_eq!(lu_back_at(peak + 1, peak, ls, lg), 0);
        // Just left of the good slope: its far end.
        assert_eq!(lu_back_at((peak + ls - lg - 1) % ls, peak, ls, lg), lg);
        assert_eq!(display_symbols(500, 65_536), 16);
        assert_eq!(display_symbols(1, 64), 2);
        assert_eq!(display_symbols(30, 64), 30);
    }

    #[test]
    fn constellation_scale_uses_the_90th_percentile() {
        let v: Vec<Complex32> = (1..=10).map(|i| Complex32::new(i as f32, 0.0)).collect();
        assert!((constellation_scale(&v) - 9.0 * 1.3).abs() < 1e-5);
        let w: Vec<Complex32> = (1..=100).map(|i| Complex32::new(i as f32, 0.0)).collect();
        assert!((constellation_scale(&w) - 90.0 * 1.3).abs() < 1e-4);
        assert!((constellation_scale(&[]) - 1.3).abs() < 1e-5);
    }
}
