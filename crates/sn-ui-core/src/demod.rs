//! State and logic of SA's FSK-2 / universal MFSK demodulator window
//! (`SA_UI_SPEC.md` section 8, report 7.6 and 11.4), shared by the front ends
//! (`sa-app`'s `demod_view.rs` holds the same logic for the egui app).
//!
//! The window recomputes as its controls change, as SA's does: the frequency track
//! (with the RRC filter when checked) is made once per filter setting, and the
//! symbol decisions are redone on it whenever Br, the phase, the levels or the main
//! markers move. `D` copies the decided symbols into the text panel.

use std::sync::Arc;

use sn_core::Signal;
use sn_core::job::{self, JobError, JobHandle};
use sn_demod::mfsk::{
    BitMap, Demodulated, Levels, MfskConfig, Polarity, Prefilter, SYMBOL_CHARS, SyncMode,
    bits_to_string, demodulate_track, detect_c4fm_inversion, frequency_track, symbols_to_bits,
};

use crate::format;
use crate::jobs::demod_to_job;
use crate::t;

/// Most levels (SA: 256).
pub const MAX_LEVELS: usize = 256;
/// Symbols on one page of the plot.
pub const PAGE: usize = 100;
/// The `Exactly` slider spans +- this fraction of a symbol.
pub const EXACT_SPAN: f64 = 0.05;
/// Most characters shown in the text panel; Copy and Save take everything.
pub const SHOWN_CHARS: usize = 20_000;

/// How the Data lines are written (SA: `Data as: Default map` or `Raw`; the rest
/// are SigNexora forms chosen from the text panel's popup menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataAs {
    /// SA's 32-character map, `x` for a level above 31.
    #[default]
    Default,
    /// Hex bytes, each followed by `x` (SA's Raw check box).
    Raw,
    /// Bits, natural binary map.
    BitsNatural,
    /// Bits, Gray map.
    BitsGray,
    /// APCO-25 C4FM dibits, polarity detected from long runs.
    C4fm,
}

impl DataAs {
    /// Every form, in the order of the `Data as` submenu.
    pub const ALL: [Self; 5] = [
        Self::Default,
        Self::Raw,
        Self::BitsNatural,
        Self::BitsGray,
        Self::C4fm,
    ];

    /// The name after `Data as:`.
    pub fn label(self) -> String {
        match self {
            Self::Default => t!("demod-as-default"),
            Self::Raw => t!("demod-as-raw"),
            Self::BitsNatural => t!("demod-as-bits"),
            Self::BitsGray => t!("demod-as-gray"),
            Self::C4fm => t!("demod-as-c4fm"),
        }
    }
}

/// What the track depends on.
#[derive(Debug, Clone, PartialEq)]
struct TrackKey {
    span: (usize, usize),
    filter: Prefilter,
    /// Rate the RRC filter is designed for (SA's cutoff), or Br.
    rate: f64,
}

/// The (filtered) frequency track of the selection.
#[derive(Debug, Clone)]
pub struct Track {
    /// The track values (Hz).
    pub values: Arc<Vec<f32>>,
    /// Its sample rate.
    pub fs: f64,
    /// 1st and 99th percentiles: where the main markers start.
    pub low: f64,
    /// The 99th percentile.
    pub high: f64,
}

/// What a demodulation depends on.
#[derive(Debug, Clone, PartialEq)]
struct DemodKey {
    track: TrackKey,
    levels: Levels,
    baud: f64,
    sync: SyncMode,
    adaptive: bool,
}

/// What the window asks the app to do.
#[derive(Debug, Clone, PartialEq)]
pub enum DemodAction {
    /// Show SA's error box with this text.
    Error(String),
}

/// Per-signal demodulator state: the window's controls and its results.
// The flags are SA's independent check boxes.
#[allow(clippy::struct_excessive_bools)]
pub struct DemodState {
    /// `RRC` check box and its cutoff combo (text) and alpha.
    pub rrc: bool,
    /// The cutoff combo's text (Hz).
    pub cutoff: String,
    /// RRC roll-off in use.
    pub alpha: f64,
    /// The alpha field's text.
    pub alpha_text: String,
    /// Br: the value in use and the text of its field (applied with Enter).
    pub baud: f64,
    /// The Br field's text.
    pub br_text: String,
    /// `A` check box (automatic bit sync) and a pending one-shot `A` button press.
    pub auto: bool,
    /// A pending one-shot `A` button press.
    pub auto_once: bool,
    /// `<= Roughly` (0..1 of a symbol) and `<= Exactly` (-1..1, scaled by
    /// [`EXACT_SPAN`]).
    pub rough: f64,
    /// `<= Exactly`, -1..1.
    pub exact: f64,
    /// Levels spin edit and the `Raw` check box (with the other output forms).
    pub levels: usize,
    /// The Levels field as typed (applied when it loses the focus).
    pub levels_text: String,
    /// How the Data lines are written.
    pub data_as: DataAs,
    /// The main markers: the outer edges of the lowest and highest level zones, in
    /// track units (bottom, top).
    pub markers: Option<(f64, f64)>,
    /// Shown value range (bottom, top).
    pub view: Option<(f64, f64)>,
    /// Page shown (0 based) and the text of the position field.
    pub page: usize,
    /// The page field's text (1 based).
    pub page_text: String,
    /// SigNexora: levels at the tones found in WF (Sum), integrate-and-dump filter when
    /// RRC is off, levels that follow the data, statistics over the plot.
    pub from_tones: bool,
    /// Integrate-and-dump filter when RRC is off.
    pub average: bool,
    /// Levels that follow the data.
    pub adaptive: bool,
    /// Statistics over the plot.
    pub info: bool,
    track: Option<(TrackKey, Track)>,
    track_job: Option<(TrackKey, JobHandle<Track>)>,
    failed_track: Option<TrackKey>,
    result: Option<(DemodKey, Demodulated)>,
    demod_job: Option<(DemodKey, JobHandle<Demodulated>)>,
    failed_demod: Option<DemodKey>,
    /// Symbols output with `D`, the level count and Br they were made with.
    data: Option<(Vec<u8>, usize, f64)>,
    /// The Data text in the current form, built once.
    text_cache: Option<(DataAs, Arc<String>)>,
    initialised: bool,
    /// Why the typed RRC cutoff is refused, shown at the cutoff field (not in a
    /// box: the text passes through invalid values while it is typed).
    pub cutoff_error: Option<String>,
    /// Why the last typed Br was refused (shown in the Br field's hover).
    pub br_refused: Option<String>,
}

impl Default for DemodState {
    fn default() -> Self {
        Self {
            rrc: false,
            cutoff: "33".to_owned(),
            alpha: 0.5,
            alpha_text: format::dec(0.5, 2),
            baud: 33.33,
            br_text: format::dec(33.33, 5),
            auto: true,
            auto_once: false,
            rough: 0.5,
            exact: 0.0,
            levels: 2,
            levels_text: "2".to_owned(),
            data_as: DataAs::Default,
            markers: None,
            view: None,
            page: 0,
            page_text: "1".to_owned(),
            from_tones: false,
            average: false,
            adaptive: false,
            info: false,
            track: None,
            track_job: None,
            failed_track: None,
            result: None,
            demod_job: None,
            failed_demod: None,
            data: None,
            text_cache: None,
            initialised: false,
            cutoff_error: None,
            br_refused: None,
        }
    }
}

impl std::fmt::Debug for DemodState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DemodState")
            .field("baud", &self.baud)
            .field("levels", &self.levels)
            .finish_non_exhaustive()
    }
}

/// Pre-fill values (SA copies a measured Br into every module that uses it).
#[derive(Debug, Clone, Default)]
pub struct Prefill {
    /// The last measured Br.
    pub br: Option<f64>,
    /// Tones found in WF (Sum), lowest first.
    pub tones: Vec<f64>,
    /// Size of the grid found by Count freqs.
    pub grid: Option<usize>,
}

/// The level centres for main markers at `bottom` and `top` (the outer edges of
/// the outer zones) and `n` levels: `n` equal zones.
pub fn grid_levels(bottom: f64, top: f64, n: usize) -> Levels {
    let n = n.clamp(2, MAX_LEVELS);
    let gap = (top - bottom) / n as f64;
    Levels::Grid {
        lo: bottom + gap / 2.0,
        hi: top - gap / 2.0,
        count: n,
    }
}

/// Main markers for `n` levels whose outer centres are `lo` and `hi`.
pub fn markers_around(lo: f64, hi: f64, n: usize) -> (f64, f64) {
    let gap = (hi - lo) / (n.max(2) - 1) as f64;
    (lo - gap / 2.0, hi + gap / 2.0)
}

/// The level boundary lines from bottom to top: the outer edges and every threshold.
pub fn boundaries(centres: &[f64]) -> Vec<f64> {
    let (Some(first), Some(last)) = (centres.first(), centres.last()) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(centres.len() + 1);
    let mids = centres.windows(2).filter_map(|w| match w {
        [a, b] => Some(0.5 * (a + b)),
        _ => None,
    });
    let gap_lo = centres.get(1).map_or(1.0, |c| c - first);
    let n = centres.len();
    let gap_hi = n
        .checked_sub(2)
        .and_then(|i| centres.get(i))
        .map_or(1.0, |c| last - c);
    out.push(first - gap_lo / 2.0);
    out.extend(mids);
    out.push(last + gap_hi / 2.0);
    out
}

/// One symbol as SA's default map writes it: `0`..`V`, `x` above 31.
fn map_char(s: u8) -> char {
    SYMBOL_CHARS
        .get(usize::from(s))
        .map_or('x', |c| char::from(*c))
}

/// The Data text of `symbols` (of `order` levels) in form `form`.
pub fn data_text(symbols: &[u8], order: usize, form: DataAs) -> String {
    use std::fmt::Write as _;
    let bits = |map: Result<BitMap, sn_demod::mfsk::Error>| {
        map.and_then(|m| symbols_to_bits(symbols, &m))
            .map_or_else(|e| format!("({e})"), |b| bits_to_string(&b, 0))
    };
    match form {
        DataAs::Default => symbols.iter().map(|s| map_char(*s)).collect(),
        DataAs::Raw => {
            let mut out = String::with_capacity(symbols.len() * 3);
            for s in symbols {
                // Writing to a String cannot fail.
                let _ = write!(out, "{s:02X}x");
            }
            out
        }
        DataAs::BitsNatural => bits(BitMap::natural(order)),
        DataAs::BitsGray => bits(BitMap::gray(order)),
        DataAs::C4fm => {
            let (map, note) = match detect_c4fm_inversion(symbols).polarity {
                Polarity::Inverted => (BitMap::c4fm_inverted(), t!("demod-polarity-inverted")),
                Polarity::Normal => (BitMap::c4fm(), t!("demod-polarity-normal")),
                Polarity::Unknown => (BitMap::c4fm(), t!("demod-polarity-unknown")),
            };
            format!("{note} {}", bits(Ok(map)))
        }
    }
}

/// The text panel's heading lines (SA: `SA - MFSK dem`, `Levels:`, `Fd:`, `Br:`,
/// `Data as:`, `Data:`).
pub fn heading(levels: usize, fs: f64, br: f64, form: DataAs) -> String {
    t!(
        "demod-heading",
        levels = levels.to_string(),
        fs = format::dec_trim(fs, 2),
        br = format::dec(br, 1),
        form = form.label()
    )
}

/// The 1st and 99th percentiles of `v` (finite values only).
fn percentiles(v: &[f32]) -> (f64, f64) {
    let step = v.len().div_ceil(20_000).max(1);
    let mut s: Vec<f32> = v
        .iter()
        .step_by(step)
        .copied()
        .filter(|x| x.is_finite())
        .collect();
    s.sort_by(f32::total_cmp);
    let at = |q: f64| {
        s.get(((s.len().saturating_sub(1)) as f64 * q).round() as usize)
            .map_or(0.0, |x| f64::from(*x))
    };
    let (lo, hi) = (at(0.01), at(0.99));
    if hi > lo {
        (lo, hi)
    } else {
        (lo - 1.0, lo + 1.0)
    }
}

/// Pages of [`PAGE`] symbols holding `symbols` symbols (at least 1).
pub fn pages(symbols: usize) -> usize {
    symbols.div_ceil(PAGE).max(1)
}

impl DemodState {
    /// A job is running.
    pub fn is_running(&self) -> bool {
        self.track_job.is_some() || self.demod_job.is_some()
    }

    /// Stop the work and free the track (the window was closed). The settings,
    /// the result and the output stay.
    pub fn release(&mut self) {
        self.track_job = None;
        self.demod_job = None;
        self.track = None;
        self.text_cache = None;
    }

    /// Fill Br, the RRC cutoff and the level count from earlier measurements the
    /// first time the window opens.
    pub fn init(&mut self, pre: &Prefill) {
        if self.initialised {
            return;
        }
        self.initialised = true;
        if let Some(b) = pre.br.filter(|b| b.is_finite() && *b > 0.0) {
            self.set_baud(b);
            // Whole Hz like SA, but a Br under 1 Bd keeps its decimals: rounding
            // it to 0 would make the cutoff invalid.
            self.cutoff = if self.baud >= 1.0 {
                format::dec_trim(self.baud.round(), 0)
            } else {
                format::dec_trim(self.baud, 2)
            };
        }
        let n = pre
            .grid
            .filter(|g| (2..=MAX_LEVELS).contains(g))
            .or_else(|| (pre.tones.len() >= 2).then_some(pre.tones.len()));
        if let Some(n) = n {
            self.set_levels(n);
        }
        if let (Some(lo), Some(hi)) = (pre.tones.first(), pre.tones.last())
            && hi > lo
        {
            self.markers = Some(markers_around(*lo, *hi, self.levels));
        }
    }

    /// Set Br and its field.
    pub fn set_baud(&mut self, b: f64) {
        self.baud = b.clamp(0.01, 1e7);
        self.br_text = format::dec(self.baud, 5);
        // A Br set by any means (the spin pairs, a prefill) replaces a refused one.
        self.br_refused = None;
    }

    /// Apply a typed Br. A text that is not a positive number shows the Br in
    /// use again and keeps why in [`Self::br_refused`]. The field's own text
    /// (the Br in use) leaves the Br unrounded.
    pub fn type_br(&mut self, text: &str) {
        if text == format::dec(self.baud, 5) {
            self.br_refused = None;
            return;
        }
        if let Some(b) = format::parse_number(text).filter(|b| *b > 0.0) {
            self.set_baud(b);
        } else {
            self.set_baud(self.baud);
            self.br_refused = Some(t!("demod-br-invalid", text = text.trim().to_owned()));
        }
    }

    /// Set Levels and its field, within 2..=256.
    pub fn set_levels(&mut self, n: usize) {
        self.levels = n.clamp(2, MAX_LEVELS);
        self.levels_text = self.levels.to_string();
    }

    /// The `A` button: a one-shot automatic bit sync whose phase is then kept
    /// in the manual sliders. The next [`Self::update`] takes the phase of the
    /// result shown when that result was made with automatic sync for exactly
    /// the current settings (no job would start for an identical key, and a
    /// pending one-shot would stick); otherwise the automatic run it starts
    /// gives the phase.
    pub fn press_auto(&mut self) {
        self.auto_once = true;
    }

    /// Keep `timing_phase` (samples), found by an automatic run at `baud` Bd on a
    /// track at `fs` Hz, in the manual sliders and end the one-shot.
    fn adopt_phase(&mut self, fs: f64, baud: f64, timing_phase: f64) {
        let period = fs / baud.max(1e-9);
        self.rough = (timing_phase / period.max(1e-9)).rem_euclid(1.0);
        self.exact = 0.0;
        self.auto = false;
        self.auto_once = false;
    }

    /// The latest finished track (it may be for older filter settings while a
    /// new one is computed).
    pub fn track(&self) -> Option<&Track> {
        self.track.as_ref().map(|(_, t)| t)
    }

    /// The latest demodulation.
    pub fn result(&self) -> Option<&Demodulated> {
        self.result.as_ref().map(|(_, d)| d)
    }

    /// Samples per symbol at the current Br.
    fn period(&self, fs: f64) -> f64 {
        fs / self.baud.max(1e-9)
    }

    /// The manual sampling phase in samples.
    pub fn manual_phase(&self, fs: f64) -> f64 {
        (self.rough + self.exact * EXACT_SPAN).rem_euclid(1.0) * self.period(fs)
    }

    /// The track for the current settings, or the error to show when the RRC
    /// cutoff is not a positive number.
    fn track_key(&self, span: (usize, usize)) -> Result<TrackKey, String> {
        let filter = if self.rrc {
            Prefilter::Rrc {
                alpha: self.alpha.clamp(0.01, 1.0),
            }
        } else if self.average {
            Prefilter::Average { fraction: 0.5 }
        } else {
            Prefilter::None
        };
        let rate = if self.rrc {
            format::parse_number(&self.cutoff)
                .filter(|c| *c > 0.0)
                .ok_or_else(|| t!("demod-cutoff-invalid", text = self.cutoff.trim().to_owned()))?
        } else {
            self.baud
        };
        Ok(TrackKey { span, filter, rate })
    }

    /// The decision levels in use: the tones (SigNexora) or the main markers.
    pub fn levels_now(&self, tones: &[f64]) -> Option<Levels> {
        if self.from_tones && tones.len() >= 2 {
            return Some(Levels::Explicit(tones.to_vec()));
        }
        let (b, t) = self.markers?;
        (t > b).then(|| grid_levels(b, t, self.levels))
    }

    /// The Data text in the current form (built once per output and form).
    pub fn data(&mut self) -> Option<Arc<String>> {
        let (symbols, order, _) = self.data.as_ref()?;
        if let Some((form, text)) = &self.text_cache
            && *form == self.data_as
        {
            return Some(Arc::clone(text));
        }
        let text = Arc::new(data_text(symbols, *order, self.data_as));
        self.text_cache = Some((self.data_as, Arc::clone(&text)));
        Some(text)
    }

    /// The level count and Br of the output, for the heading.
    pub fn output_header(&self) -> Option<(usize, f64)> {
        self.data.as_ref().map(|(_, n, b)| (*n, *b))
    }

    /// `D`: put the decided symbols into the text panel.
    pub fn output(&mut self) -> Result<(), String> {
        let Some(d) = self.result() else {
            return Err(t!("demod-nothing-yet"));
        };
        self.data = Some((d.symbols.clone(), d.layout.order(), d.measured_baud));
        self.text_cache = None;
        Ok(())
    }

    /// Poll the jobs and start the work the controls ask for. Returns an error to
    /// show and whether work is running.
    pub fn update(
        &mut self,
        signal: &Signal,
        span: (usize, usize),
        tones: &[f64],
    ) -> (Option<DemodAction>, bool) {
        let mut action = self.poll_track();
        let key = match self.track_key(span) {
            Ok(k) => {
                self.cutoff_error = None;
                k
            }
            Err(e) => {
                // No filter is designed for a cutoff other than the one typed;
                // nothing runs until it is valid. The field shows why.
                self.cutoff_error = Some(e);
                return (action, self.is_running());
            }
        };
        let track_ok = self.track.as_ref().is_some_and(|(k, _)| *k == key);
        if !track_ok
            && self.track_job.as_ref().is_none_or(|(k, _)| *k != key)
            && self.failed_track.as_ref() != Some(&key)
        {
            self.spawn_track(signal, key.clone());
        }
        if track_ok {
            action = action.or_else(|| self.poll_demod());
            // Nothing will run for an automatic key that already has a result or
            // failed: the one-shot must not stick (it would override the
            // manual sliders for good).
            if self.auto_once
                && let Some(dk) = self.demod_key(&key, signal.sample_rate(), tones)
            {
                if let Some((k, d)) = self.result.as_ref().filter(|(k, _)| *k == dk) {
                    let (baud, phase) = (k.baud, d.timing_phase);
                    let fs = self.track.as_ref().map_or(1.0, |(_, t)| t.fs);
                    self.adopt_phase(fs, baud, phase);
                } else if self.failed_demod.as_ref() == Some(&dk) {
                    self.auto_once = false;
                }
            }
            if let Some(dk) = self.demod_key(&key, signal.sample_rate(), tones)
                && self.result.as_ref().is_none_or(|(k, _)| *k != dk)
                && self.demod_job.as_ref().is_none_or(|(k, _)| *k != dk)
                && self.failed_demod.as_ref() != Some(&dk)
            {
                self.spawn_demod(dk);
            }
        }
        (action, self.is_running())
    }

    fn demod_key(&self, track: &TrackKey, fs: f64, tones: &[f64]) -> Option<DemodKey> {
        let sync = if self.auto || self.auto_once {
            SyncMode::Auto
        } else {
            SyncMode::Manual {
                phase: self.manual_phase(fs),
            }
        };
        Some(DemodKey {
            track: track.clone(),
            levels: self.levels_now(tones)?,
            baud: self.baud,
            sync,
            adaptive: self.adaptive,
        })
    }

    fn spawn_track(&mut self, signal: &Signal, key: TrackKey) {
        if let Some((_, h)) = self.track_job.take() {
            h.cancel();
        }
        // A demodulation of the old track decides instants that are wrong over
        // the new values: stop it rather than show its result.
        if let Some((_, h)) = self.demod_job.take() {
            h.cancel();
        }
        let s = signal.clone();
        let k = key.clone();
        let h = job::spawn(t!("demod-job-track"), move |ctx| {
            let stop = || ctx.is_cancelled();
            let part = s
                .slice(k.span.0, k.span.1.max(k.span.0))
                .map_err(|e| JobError::Failed(e.to_string()))?;
            let values = frequency_track(&part, k.rate, k.filter, &stop).map_err(demod_to_job)?;
            let (low, high) = percentiles(&values);
            Ok(Track {
                values: Arc::new(values),
                fs: part.sample_rate(),
                low,
                high,
            })
        });
        self.track_job = Some((key, h));
    }

    fn spawn_demod(&mut self, key: DemodKey) {
        let Some((_, track)) = &self.track else {
            return;
        };
        if let Some((_, h)) = self.demod_job.take() {
            h.cancel();
        }
        let (values, fs) = (Arc::clone(&track.values), track.fs);
        let mut cfg = MfskConfig::new(key.levels.clone(), key.baud);
        cfg.filter = key.track.filter;
        cfg.sync = key.sync;
        cfg.adaptive = key.adaptive;
        let h = job::spawn(t!("demod-job"), move |ctx| {
            let stop = || ctx.is_cancelled();
            demodulate_track(&values, fs, &cfg, &stop).map_err(demod_to_job)
        });
        self.demod_job = Some((key, h));
    }

    fn poll_track(&mut self) -> Option<DemodAction> {
        if !self
            .track_job
            .as_ref()
            .is_some_and(|(_, h)| h.is_finished())
        {
            return None;
        }
        let (key, h) = self.track_job.take()?;
        match h.join() {
            Ok(track) => {
                if self.markers.is_none() {
                    self.markers = Some(markers_around(track.low, track.high, self.levels));
                }
                self.failed_track = None;
                // The result's instants were decided on the old track: drawing
                // them over the new values would be wrong until it is redone.
                if self.result.as_ref().is_some_and(|(k, _)| k.track != key) {
                    self.result = None;
                }
                self.track = Some((key, track));
                None
            }
            Err(JobError::Cancelled) => None,
            Err(e) => {
                self.failed_track = Some(key);
                Some(DemodAction::Error(e.to_string()))
            }
        }
    }

    fn poll_demod(&mut self) -> Option<DemodAction> {
        if !self
            .demod_job
            .as_ref()
            .is_some_and(|(_, h)| h.is_finished())
        {
            return None;
        }
        let (key, h) = self.demod_job.take()?;
        match h.join() {
            Ok(d) => {
                // Made on another track than the one now kept: its instants are
                // wrong over these values, so it is neither shown nor used for A.
                let (_, track) = self.track.as_ref().filter(|(k, _)| *k == key.track)?;
                let fs = track.fs;
                if self.auto_once && key.sync == SyncMode::Auto {
                    // The A button: keep the phase it found, converted with the
                    // Br the run was made with.
                    self.adopt_phase(fs, key.baud, d.timing_phase);
                }
                self.failed_demod = None;
                self.result = Some((key, d));
                None
            }
            Err(JobError::Cancelled) => None,
            Err(e) => {
                self.failed_demod = Some(key);
                self.auto_once = false;
                Some(DemodAction::Error(e.to_string()))
            }
        }
    }

    /// Progress of the running work, for the progress field.
    /// `None` when idle, `Some(None)` while busy without a known fraction,
    /// `Some(Some(f))` with the fraction done.
    pub fn progress(&self) -> Option<Option<f32>> {
        self.track_job
            .as_ref()
            .map(|(_, h)| h.progress())
            .or_else(|| self.demod_job.as_ref().map(|(_, h)| h.progress()))
    }

    /// Stop the running work (the progress field's stop).
    pub fn stop(&mut self) {
        self.auto_once = false;
        if let Some((k, h)) = self.track_job.take() {
            h.cancel();
            self.failed_track = Some(k);
        }
        if let Some((k, h)) = self.demod_job.take() {
            h.cancel();
            self.failed_demod = Some(k);
        }
    }
}

/// RRC cutoff presets (Hz); the cutoff list offers Br first.
pub const CUTOFFS: [f64; 13] = [
    50.0, 75.0, 100.0, 150.0, 300.0, 600.0, 1200.0, 2400.0, 3600.0, 4800.0, 9600.0, 19_200.0,
    38_400.0,
];
/// Steps of the Br spin pairs: two coarse ones, then five digit steps.
pub const BR_STEPS: [f64; 7] = [1000.0, 100.0, 10.0, 1.0, 0.1, 0.01, 0.001];
/// How many of [`BR_STEPS`] are the coarse pairs (a gap follows them).
pub const BR_COARSE: usize = 2;
/// Zoom step of the vertical scale buttons.
pub const ZOOM: f64 = 1.25;

/// The cutoff list: Br first, then the presets other than Br.
pub fn cutoff_list(baud: f64) -> Vec<f64> {
    let mut list = vec![baud];
    list.extend(CUTOFFS.iter().copied().filter(|v| (v - baud).abs() > 1e-9));
    list
}

/// The text panel: the heading, then the data (at most [`SHOWN_CHARS`]
/// characters and a note when cut).
pub fn panel_text(st: &mut DemodState, fs: f64) -> String {
    let (levels, br) = st.output_header().unwrap_or((st.levels, st.baud));
    let mut text = heading(levels, fs, br, st.data_as);
    if let Some(data) = st.data() {
        text.push('\n');
        let cut = data
            .char_indices()
            .nth(SHOWN_CHARS)
            .map_or(data.len(), |(i, _)| i);
        text.push_str(data.get(..cut).unwrap_or_default());
        if cut < data.len() {
            text.push('\n');
            text.push_str(&t!("demod-truncated"));
        }
    }
    text
}

impl DemodState {
    /// The base value range: the main markers with a quarter of their span
    /// around them (or the track's percentiles, or -1..1).
    pub fn base_range(&self) -> (f64, f64) {
        let (b, t) = self
            .markers
            .or_else(|| self.track().map(|tr| (tr.low, tr.high)))
            .unwrap_or((-1.0, 1.0));
        let pad = (t - b).abs().max(1e-9) * 0.25;
        (b - pad, t + pad)
    }

    /// The value range shown (bottom, top).
    pub fn shown_range(&self) -> (f64, f64) {
        self.view.unwrap_or_else(|| self.base_range())
    }

    /// Zoom the value range by `f` (below 1 zooms in) around its centre.
    pub fn zoom(&mut self, f: f64) {
        let base = self.base_range();
        let (lo, hi) = self.shown_range();
        let full = base.1 - base.0;
        let h = ((hi - lo) * f).clamp(full / 64.0, full);
        let mid = f64::midpoint(lo, hi);
        self.view = Some((mid - h / 2.0, mid + h / 2.0));
    }

    /// The number of pages of the latest result (at least 1).
    pub fn page_count(&self) -> usize {
        self.result().map_or(1, |d| pages(d.symbols.len()))
    }

    /// Go to page `p` (0 based, clamped) and show it in the page field.
    pub fn go_to_page(&mut self, p: i64) {
        let total = self.page_count() as i64;
        self.page = p.clamp(0, total - 1) as usize;
        self.page_text = (self.page + 1).to_string();
    }

    /// Apply the typed alpha (0,05 to 1); an invalid value shows the alpha in
    /// use again.
    pub fn type_alpha(&mut self, text: &str) {
        if let Some(a) = format::parse_number(text) {
            self.alpha = a.clamp(0.05, 1.0);
        }
        self.alpha_text = format::dec(self.alpha, 2);
    }

    /// Apply the typed level count (2 to 256); an invalid value shows the
    /// count in use again.
    pub fn type_levels(&mut self, text: &str) {
        match text.trim().parse::<usize>() {
            Ok(n) => self.set_levels(n),
            Err(_) => self.set_levels(self.levels),
        }
    }

    /// Move the bottom (`top` false) or top main marker to value `v`, keeping
    /// a small gap to the other one within the shown range `(lo, hi)`.
    pub fn drag_marker(&mut self, top: bool, v: f64, (lo, hi): (f64, f64)) {
        let Some((b, t)) = self.markers else {
            return;
        };
        let min_gap = (hi - lo) / 200.0;
        self.markers = Some(if top {
            (b, v.max(b + min_gap))
        } else {
            (v.min(t - min_gap), t)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_and_levels_round_trip() {
        let (b, t) = markers_around(1000.0, 2000.0, 3);
        assert!((b - 750.0).abs() < 1e-9 && (t - 2250.0).abs() < 1e-9);
        let Levels::Grid { lo, hi, count } = grid_levels(b, t, 3) else {
            panic!("grid");
        };
        assert!((lo - 1000.0).abs() < 1e-9 && (hi - 2000.0).abs() < 1e-9);
        assert_eq!(count, 3);
        // The count is kept within SA's 2..=256.
        let Levels::Grid { count, .. } = grid_levels(0.0, 1.0, 1) else {
            panic!("grid");
        };
        assert_eq!(count, 2);
    }

    #[test]
    fn boundaries_are_outer_edges_and_thresholds() {
        assert_eq!(boundaries(&[0.0, 10.0, 20.0]), vec![-5.0, 5.0, 15.0, 25.0]);
        assert_eq!(boundaries(&[1.0, 3.0]), vec![0.0, 2.0, 4.0]);
        assert_eq!(boundaries(&[]), Vec::<f64>::new());
    }

    #[test]
    fn data_forms_follow_sa() {
        assert_eq!(data_text(&[0, 2, 31, 32], 33, DataAs::Default), "02Vx");
        assert_eq!(data_text(&[2, 12], 33, DataAs::Raw), "02x0Cx");
        assert_eq!(data_text(&[0, 3], 4, DataAs::BitsNatural), "0011");
        let h = heading(4, 22050.0, 4777.154, DataAs::Raw);
        assert!(h.starts_with("SA - MFSK dem\nLevels: 4\nFd: 22050\nBr: 4777"));
        assert!(h.ends_with("Data:"));
    }

    #[test]
    fn prefill_takes_br_grid_and_tones() {
        let mut st = DemodState::default();
        st.init(&Prefill {
            br: Some(4800.0),
            tones: vec![-1800.0, -600.0, 600.0, 1800.0],
            grid: Some(4),
        });
        assert!((st.baud - 4800.0).abs() < 1e-9);
        assert_eq!(st.cutoff, "4800");
        assert_eq!(st.levels, 4);
        assert_eq!(st.markers, Some((-2400.0, 2400.0)));
        // Only the first opening pre-fills.
        st.init(&Prefill {
            br: Some(1200.0),
            ..Prefill::default()
        });
        assert!((st.baud - 4800.0).abs() < 1e-9);
    }

    #[test]
    fn demodulates_a_two_level_track_and_outputs_it() {
        // 100 Bd, 8000 Hz: the pattern 0110 1001 repeated, levels -1000 and +1000 Hz.
        let pattern = [0, 1, 1, 0, 1, 0, 0, 1];
        let values: Vec<f32> = (0..400 * 80)
            .map(|i| {
                let bit = pattern[(i / 80) % pattern.len()];
                if bit == 1 { 1000.0 } else { -1000.0 }
            })
            .collect();
        let signal = Signal::real(values, 8000.0).unwrap();
        let mut st = DemodState::default();
        st.init(&Prefill {
            br: Some(100.0),
            ..Prefill::default()
        });
        let span = (0, signal.len());
        for _ in 0..3000 {
            let (action, busy) = st.update(&signal, span, &[]);
            assert_eq!(action, None);
            if !busy && st.result().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let (b, t) = st.markers.unwrap();
        assert!(b < -1000.0 && t > 1000.0, "markers around both levels");
        st.output().unwrap();
        let text = st.data().unwrap();
        assert!(text.len() > 300);
        assert!(text.contains("01101001"), "{text}");
        st.data_as = DataAs::Raw;
        assert!(st.data().unwrap().starts_with('0'));
        assert!(st.data().unwrap().contains("01x"));
    }

    #[test]
    fn manual_phase_and_pages() {
        let mut st = DemodState::default();
        st.set_baud(100.0);
        st.rough = 0.25;
        st.exact = 1.0;
        assert!((st.manual_phase(8000.0) - 24.0).abs() < 1e-9);
        assert_eq!(pages(0), 1);
        assert_eq!(pages(PAGE + 1), 2);
        let (lo, hi) = percentiles(&(0..1000).map(|i| i as f32).collect::<Vec<_>>());
        assert!((lo - 10.0).abs() < 1.5 && (hi - 989.0).abs() < 1.5);
    }

    /// A two-level 100 Bd track at 8000 Hz.
    fn two_level_signal() -> Signal {
        let pattern = [0, 1, 1, 0, 1, 0, 0, 1];
        let values: Vec<f32> = (0..400 * 80)
            .map(|i| {
                let bit = pattern[(i / 80) % pattern.len()];
                if bit == 1 { 1000.0 } else { -1000.0 }
            })
            .collect();
        let Ok(s) = Signal::real(values, 8000.0) else {
            panic!("signal");
        };
        s
    }

    /// Run `update` until no work is left; the actions it reported.
    fn settle(st: &mut DemodState, signal: &Signal) -> Vec<DemodAction> {
        let span = (0, signal.len());
        let mut actions = Vec::new();
        for _ in 0..3000 {
            let (action, busy) = st.update(signal, span, &[]);
            actions.extend(action);
            if !busy {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        actions
    }

    #[test]
    fn a_button_with_auto_on_takes_the_phase_and_does_not_stick() {
        let signal = two_level_signal();
        let mut st = DemodState::default();
        st.init(&Prefill {
            br: Some(100.0),
            ..Prefill::default()
        });
        assert!(st.auto);
        assert_eq!(settle(&mut st, &signal), Vec::new());
        assert!(st.result().is_some());
        st.press_auto();
        let _ = st.update(&signal, (0, signal.len()), &[]);
        assert!(
            !st.auto_once && !st.auto,
            "the one-shot took the phase at once"
        );
        // A manual slider move now gives a manual key.
        st.rough = (st.rough + 0.3).rem_euclid(1.0);
        let _ = settle(&mut st, &signal);
        let Some((key, _)) = &st.result else {
            panic!("no result");
        };
        assert!(
            matches!(key.sync, SyncMode::Manual { .. }),
            "{:?}",
            key.sync
        );
    }

    #[test]
    fn filter_change_drops_the_old_tracks_demodulation() {
        let signal = two_level_signal();
        let span = (0, signal.len());
        let mut st = DemodState::default();
        st.init(&Prefill {
            br: Some(100.0),
            ..Prefill::default()
        });
        let _ = settle(&mut st, &signal);
        assert!(st.result().is_some());
        // A new demodulation starts on the old track, then the filter changes.
        st.set_levels(3);
        let _ = st.update(&signal, span, &[]);
        st.average = !st.average;
        let new_track = st.track_key(span).unwrap();
        for _ in 0..3000 {
            let (_, busy) = st.update(&signal, span, &[]);
            // Once the new track is kept, no result of the old one may be shown
            // over it.
            if st.track.as_ref().is_some_and(|(k, _)| *k == new_track)
                && let Some((k, _)) = &st.result
            {
                assert_eq!(k.track, new_track, "a result for the old track came back");
            }
            if !busy {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            st.result
                .as_ref()
                .is_some_and(|(k, _)| k.track == new_track)
        );
    }

    #[test]
    fn a_button_ignores_a_result_for_another_br() {
        let signal = two_level_signal();
        let span = (0, signal.len());
        let mut st = DemodState::default();
        st.init(&Prefill {
            br: Some(100.0),
            ..Prefill::default()
        });
        let _ = settle(&mut st, &signal);
        let rough = st.rough;
        st.set_baud(101.0);
        st.press_auto();
        let _ = st.update(&signal, span, &[]);
        assert!(st.auto_once, "the shown result was made for 100 Bd");
        assert!((st.rough - rough).abs() < 1e-12);
        let _ = settle(&mut st, &signal);
        assert!(!st.auto_once && !st.auto);
        let key = st.result.as_ref().map(|(k, _)| k.baud);
        assert_eq!(key, Some(101.0));
    }

    #[test]
    fn prefilled_cutoff_of_a_slow_br_is_valid() {
        let mut st = DemodState {
            rrc: true,
            ..DemodState::default()
        };
        st.init(&Prefill {
            br: Some(0.3),
            ..Prefill::default()
        });
        assert!(st.track_key((0, 10)).is_ok(), "cutoff {}", st.cutoff);
    }

    #[test]
    fn a_button_with_auto_off_runs_once() {
        let signal = two_level_signal();
        let mut st = DemodState::default();
        st.init(&Prefill {
            br: Some(100.0),
            ..Prefill::default()
        });
        st.auto = false;
        let _ = settle(&mut st, &signal);
        st.press_auto();
        assert!(st.auto_once);
        let _ = settle(&mut st, &signal);
        assert!(!st.auto_once, "cleared once the automatic run finished");
    }

    #[test]
    fn stop_does_not_restart_and_clears_the_one_shot() {
        let signal = two_level_signal();
        let mut st = DemodState::default();
        st.init(&Prefill {
            br: Some(100.0),
            ..Prefill::default()
        });
        st.auto_once = true;
        let span = (0, signal.len());
        let _ = st.update(&signal, span, &[]);
        st.stop();
        assert!(!st.auto_once);
        let (_, busy) = st.update(&signal, span, &[]);
        assert!(!busy, "a stopped key is not started again");
    }

    #[test]
    fn invalid_rrc_cutoff_is_shown_at_the_field_and_runs_nothing() {
        let signal = two_level_signal();
        let mut st = DemodState {
            rrc: true,
            cutoff: String::new(),
            ..DemodState::default()
        };
        let span = (0, signal.len());
        // Cleared while a new cutoff is typed: no box, nothing runs, the field
        // says why.
        let (a, busy) = st.update(&signal, span, &[]);
        assert_eq!(a, None, "no box while typing");
        assert!(!busy);
        assert!(st.cutoff_error.is_some());
        st.cutoff = "2400".to_owned();
        let (_, busy) = st.update(&signal, span, &[]);
        assert!(st.cutoff_error.is_none());
        assert!(busy);
        st.release();
    }

    #[test]
    fn br_typed_then_left_is_applied() {
        let mut st = DemodState::default();
        st.type_br("1200");
        assert!((st.baud - 1200.0).abs() < 1e-9);
        assert!(st.br_refused.is_none());
        st.type_br("x");
        assert!((st.baud - 1200.0).abs() < 1e-9);
        assert!(st.br_refused.is_some());
        assert_eq!(st.br_text, format::dec(1200.0, 5));
        // A Br set with the spin pairs replaces the refused one and its reason.
        st.set_baud(1300.0);
        assert!(st.br_refused.is_none());
    }

    #[test]
    fn failed_track_is_reported_once() {
        let signal = two_level_signal();
        let mut st = DemodState::default();
        // An empty span cannot give a frequency track.
        let actions = {
            let mut v = Vec::new();
            for _ in 0..3000 {
                let (a, busy) = st.update(&signal, (10, 10), &[]);
                v.extend(a);
                if !busy && !v.is_empty() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            v
        };
        assert_eq!(actions.len(), 1, "{actions:?}");
        let (a, busy) = st.update(&signal, (10, 10), &[]);
        assert_eq!(a, None);
        assert!(!busy, "not respawned");
    }
}
