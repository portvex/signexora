//! PhasePlane module state and logic (`SA_UI_SPEC.md` section 6): the form's
//! control values, the constellation run and its playback through the file, the ISD
//! corrector's spectrum, the high-resolution averaged spectrum of the spectrum mode,
//! the angles histogram with its markers and the PSK demodulator output.
//!
//! Shared by the front ends (moved from `sa-app`'s `phase_view.rs`, which
//! keeps its own copy until the egui app is removed). The form is drawn by
//! each front end; [`crate::phase_plot`] holds the geometry of its plots.

use sn_core::job::{self, JobError, JobHandle};
use sn_core::{Complex32, NodeId, SampleKind, Signal};
/// The `PLL` group's speeds.
pub use sn_demod::phase::PllSpeed;
use sn_demod::phase::{PhaseConfig, PhaseResult, SPS, eye, offset_timing, rings, run};
use sn_dsp::spectrum::{Combine, SpectrumEngine};
use sn_dsp::window::WindowKind;

use crate::edits::MeasuredValue;
use crate::format;
use crate::jobs::{demod_to_job, dsp_to_job};
use crate::t;

/// The module's two modes: the spectrum on opening, the phase plane after `LoadFile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    /// The high-resolution averaged spectrum, on opening.
    Spectrum,
    /// The ISD corrector and the phase viewer, after `LoadFile`.
    Phase,
}

/// The ISD corrector's Mode group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IsdMode {
    /// Works on the clock line (envelope spectrum).
    Am,
    /// Works on the N-th harmonic, carrier and sidebands.
    #[default]
    Xn,
}

/// An entry of the `Output Dem as` list: `m` positions, `shifted` (`4-s`) for the
/// special modes of relative signals such as pi/4 DQPSK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemMode {
    /// Number of positions.
    pub m: u32,
    /// A shifted mode: the zero vector half a sector below 0.
    pub shifted: bool,
}

impl Default for DemMode {
    fn default() -> Self {
        Self {
            m: 2,
            shifted: false,
        }
    }
}

impl DemMode {
    /// SA's list: 2 to 32, then the shifted modes 2-s to 10-s.
    pub fn all() -> Vec<Self> {
        (2..=32)
            .map(|m| Self { m, shifted: false })
            .chain((2..=10).map(|m| Self { m, shifted: true }))
            .collect()
    }

    /// `8` or `4-s`.
    pub fn label(self) -> String {
        if self.shifted {
            format!("{}-s", self.m)
        } else {
            self.m.to_string()
        }
    }

    /// Angle of the zero vector in degrees: 0, or half a sector below 0 when shifted.
    fn zero(self) -> f64 {
        if self.shifted {
            -180.0 / f64::from(self.m.max(1))
        } else {
            0.0
        }
    }

    /// Angle of position `k` in degrees, in (-180, 180]: counter-clockwise from the
    /// zero vector.
    pub fn angle(self, k: u32) -> f64 {
        let a = self.zero() + f64::from(k) * 360.0 / f64::from(self.m.max(1));
        wrap180(a)
    }

    /// Code (position number) of a point at `deg` degrees.
    pub fn code(self, deg: f64) -> u32 {
        let m = self.m.max(1);
        let sector = 360.0 / f64::from(m);
        let k = ((deg - self.zero()) / sector)
            .round()
            .rem_euclid(f64::from(m));
        (k as u32) % m
    }
}

/// `deg` wrapped to (-180, 180].
pub fn wrap180(deg: f64) -> f64 {
    let a = deg.rem_euclid(360.0);
    if a > 180.0 { a - 360.0 } else { a }
}

/// Smallest angle between two directions, 0..=180 degrees.
pub fn angle_between(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 { 360.0 - d } else { d }
}

/// An angle as SA writes it in the code table: at least two integer digits
/// (`00,00`, `45,00`, `-135,00`).
pub fn table_angle(deg: f64) -> String {
    let text = format::dec(deg.abs(), 2);
    let int_len = text.chars().take_while(char::is_ascii_digit).count();
    let pad = if int_len < 2 { "0" } else { "" };
    let sign = if deg < -1e-9 { "-" } else { "" };
    format!("{sign}{pad}{text}")
}

/// The positions in the Blocks group of the ISD corrector.
pub const BLOCKS: [usize; 5] = [10, 20, 30, 40, 50];
/// `FFTSize` list of the spectrum mode.
pub const FFT_SIZES: [usize; 6] = [4096, 8192, 16_384, 32_768, 65_536, 131_072];
/// Scale list of the spectrum mode: the shown part of the band, `1/k`.
pub const SCALES: [u32; 5] = [1, 2, 4, 8, 16];
/// Bins of the angles histogram per half (0 to 180 degrees, 0.36 degrees each).
pub const HIST_BINS: usize = 500;
/// Most angle markers.
pub const MAX_MARKS: usize = 10;

/// Every value of the form's controls.
// Each flag is one of SA's check boxes.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    /// The `Carrier` field, Hz.
    pub carrier: String,
    /// The `BR` field, Bd.
    pub br: String,
    /// `n-Ary`: the modulation order, 1 to 32.
    pub order: u32,
    /// `Synch BR`: clock synchronisation.
    pub synch: bool,
    /// `NonStop`: start over at the end of the file.
    pub nonstop: bool,
    /// `RRC filter`.
    pub rrc: bool,
    /// Roll-off alpha of the RRC filter.
    pub alpha: f64,
    /// The `PLL` group.
    pub pll: PllSpeed,
    /// `AutoReset`.
    pub auto_reset: bool,
    /// `Transitions`: draw the paths between the points.
    pub transitions: bool,
    /// `Life`: only the last `Points...` symbols stay.
    pub life: bool,
    /// `Diff`, 0 to 3.
    pub diff: u8,
    /// `Angles`: the angles histogram replaces the corrector plot.
    pub angles: bool,
    /// `Vectors of dem`.
    pub vectors: bool,
    /// `Output Dem as`: the demodulator writes to the text panel.
    pub output_dem: bool,
    /// The mode of `Output Dem as`.
    pub dem: DemMode,
    /// Fine phase trim, degrees.
    pub trimming: i32,
    /// Real delay per symbol, seconds.
    pub delay: f64,
    /// Points shown at once with Life on.
    pub points: usize,
    /// Brightness of the points.
    pub contrast: u32,
    /// Constellation scale (its vertical scroll bar), 0..1.
    pub scale: f32,
    /// The corrector's `Mode` group.
    pub isd_mode: IsdMode,
    /// The corrector's `N`, 1 to 32.
    pub isd_n: u32,
    /// The corrector's `(B)`.
    pub isd_b: bool,
    /// The corrector's `Blocks` group.
    pub blocks: usize,
    /// Corrector plot vertical scale (its vertical scroll bar), 0..1.
    pub isd_scale: f32,
    /// Corrector position in the file (the scroll bar under it), 0..1.
    pub isd_pos: f32,
    /// `FFTSize` of the spectrum mode.
    pub fft: usize,
    /// `Scale` of the spectrum mode: the `1/zoom` part of the band.
    pub zoom: u32,
    /// `LowLevel`: the bottom of the spectrum, dB.
    pub low_level: f32,
    /// `Stretch` of the spectrum mode.
    pub stretch: bool,
    /// Spectrum navigation (the scroll bar under it), 0..1.
    pub spec_pos: f32,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            carrier: "1800".to_owned(),
            br: "2400".to_owned(),
            order: 2,
            synch: true,
            nonstop: true,
            rrc: false,
            alpha: 0.5,
            pll: PllSpeed::Medium,
            auto_reset: false,
            transitions: false,
            life: true,
            diff: 0,
            angles: false,
            vectors: false,
            output_dem: false,
            dem: DemMode::default(),
            trimming: 0,
            delay: 0.0,
            points: 1000,
            contrast: 40,
            scale: 0.5,
            isd_mode: IsdMode::Xn,
            isd_n: 1,
            isd_b: true,
            blocks: 30,
            isd_scale: 0.5,
            isd_pos: 0.0,
            fft: 16_384,
            zoom: 1,
            low_level: -100.0,
            stretch: false,
            spec_pos: 0.0,
        }
    }
}

impl Form {
    /// Carrier in Hz, if the field holds a number.
    pub fn carrier_hz(&self) -> Option<f64> {
        format::parse_number(&self.carrier)
    }

    /// BR in Bd, if the field holds a positive number.
    pub fn baud(&self) -> Option<f64> {
        format::parse_number(&self.br).filter(|b| *b > 0.0)
    }

    /// The demodulator settings, or `None` while Carrier or BR is not a number.
    pub fn config(&self) -> Option<PhaseConfig> {
        Some(PhaseConfig {
            carrier: self.carrier_hz()?,
            baud: self.baud()?,
            order: self.order.clamp(1, 32),
            pll: self.pll,
            clock_sync: self.synch,
            rrc_alpha: self.rrc.then_some(self.alpha),
            diff: self.diff.min(3),
        })
    }

    /// Constellation radius scale: points at RMS 1 sit at this fraction of the half
    /// side (the scroll bar runs from 0.15 to 1.2).
    pub fn plane_scale(&self) -> f32 {
        0.15 + 1.05 * (1.0 - self.scale.clamp(0.0, 1.0))
    }

    /// PSK order the corrector's X^N mode equalizes for: N, or n-Ary while N is 1.
    pub fn isd_order(&self) -> u32 {
        if self.isd_n >= 2 {
            self.isd_n
        } else {
            self.order.max(2)
        }
    }
}

/// A PhasePlane run with what is drawn from it computed once, off the UI thread.
#[allow(missing_debug_implementations)]
pub struct Shown {
    /// The demodulator's run.
    pub r: PhaseResult,
    /// The settings and signal it was computed for.
    cfg: PhaseConfig,
    source: NodeId,
    /// SA's quality number (a signal-to-noise ratio in dB).
    pub quality: Option<f32>,
    /// An offset mode (OQPSK and the like) was detected.
    pub offset: bool,
    /// Rings, loop offset and I/Q offset, for the quality number's hint.
    pub details: String,
    /// The eye diagram's traces.
    pub eye: Vec<Vec<f32>>,
    /// Only the start of the file was demodulated (see [`run_end`]).
    pub limited: bool,
}

/// Most symbols one PhasePlane run demodulates. The run keeps every symbol and an
/// eight-point trajectory for each, about 100 bytes per symbol, so this bounds the
/// kept result to roughly 100 MB however long the file or high the BR.
pub const MAX_RUN_SYMBOLS: usize = 1_000_000;

/// Most input samples one PhasePlane run reads. The demodulator holds several
/// full-length complex buffers at the input rate (the analytic signal, the matched
/// filter output and the low-pass output, at least 32 bytes per sample), so this
/// bounds that working memory to roughly 256 MB however oversampled the signal is.
pub const MAX_RUN_SAMPLES: usize = 8 * 1024 * 1024;

/// The end of the span a run demodulates in a file of `len` samples at `fs` Hz and
/// `baud` Bd, and whether it stops short of the file's end: at most
/// [`MAX_RUN_SYMBOLS`] symbols and at most [`MAX_RUN_SAMPLES`] input samples.
pub fn run_end(len: usize, fs: f64, baud: f64) -> (usize, bool) {
    let by_symbols = MAX_RUN_SYMBOLS as f64 * fs / baud.max(f64::MIN_POSITIVE);
    let most = if by_symbols.is_finite() {
        by_symbols.min(MAX_RUN_SAMPLES as f64)
    } else {
        MAX_RUN_SAMPLES as f64
    };
    if most < len as f64 {
        (most.floor().max(1.0) as usize, true)
    } else {
        (len, false)
    }
}

impl Shown {
    fn new(r: PhaseResult, cfg: PhaseConfig, source: NodeId) -> Self {
        let ring_text = rings(&r.points)
            .iter()
            .map(|v| format!("{v:.2}"))
            .collect::<Vec<_>>()
            .join(", ");
        let iq_offset = offset_timing(&r.trajectory);
        let details = t!(
            "pp-quality-details",
            count = r.points.len(),
            offset = format!("{:+.3}", r.carrier_offset),
            rings = ring_text,
            iq_offset = format!("{iq_offset:.2}")
        );
        Self {
            quality: quality(&r.points, cfg.order),
            offset: cfg.order >= 2 && iq_offset >= OFFSET_DETECT,
            details,
            eye: eye(&r.trajectory, 400),
            limited: false,
            r,
            cfg,
            source,
        }
    }

    /// Number of symbols.
    pub fn len(&self) -> usize {
        self.r.display.len()
    }

    /// The run holds no symbols.
    pub fn is_empty(&self) -> bool {
        self.r.display.is_empty()
    }
}

/// I/Q transition timing difference (symbols) from which an offset mode is reported.
const OFFSET_DETECT: f32 = 0.4;

/// SA's quality number: the ratio of the mean point power to the mean squared
/// distance from the nearest ideal M-PSK point, in dB. `None` without points.
pub fn quality(points: &[Complex32], order: u32) -> Option<f32> {
    if points.is_empty() {
        return None;
    }
    let n = points.len() as f32;
    let radius = points.iter().map(|p| p.norm()).sum::<f32>() / n;
    let sector = std::f32::consts::TAU / order.max(1) as f32;
    let err = points
        .iter()
        .map(|p| {
            let k = (p.arg() / sector).round();
            let ideal = Complex32::from_polar(radius, k * sector);
            (p - ideal).norm_sqr()
        })
        .sum::<f32>()
        / n;
    let power = points.iter().map(Complex32::norm_sqr).sum::<f32>() / n;
    (power > 0.0).then(|| 10.0 * (power / err.max(1e-9)).log10())
}

/// The playback of a run through the file.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Play {
    /// Start was pressed and Break was not.
    pub running: bool,
    /// Symbols shown so far (the playhead).
    pub pos: usize,
    /// First symbol drawn since the last Clear.
    pub clear_from: usize,
    /// First symbol of the demodulator output.
    pub dem_from: usize,
    /// Fraction of a symbol not shown yet.
    carry: f64,
    /// Time of the last step.
    last: Option<f64>,
}

/// Symbols shown per second with Delay 0.
const SYMBOLS_PER_SECOND: f64 = 4000.0;

impl Play {
    /// Advance the playhead to `now` (seconds) through `len` symbols. At the end the
    /// run starts over with `NonStop` and stops without it.
    pub fn step(&mut self, now: f64, len: usize, delay: f64, nonstop: bool) {
        let dt = self.last.map_or(0.0, |t| (now - t).clamp(0.0, 0.25));
        self.last = Some(now);
        let rate = if delay > 0.0 {
            (1.0 / delay).min(SYMBOLS_PER_SECOND)
        } else {
            SYMBOLS_PER_SECOND
        };
        self.carry += dt * rate;
        let whole = self.carry.floor();
        self.carry -= whole;
        self.pos = self.pos.saturating_add(whole as usize);
        if self.pos >= len {
            if nonstop && len > 0 {
                // The next pass starts from an empty display: everything shown
                // and the angle histogram begin again at the first symbol.
                self.pos = 0;
                self.clear_from = 0;
                self.dem_from = 0;
            } else {
                self.pos = len;
                self.running = false;
            }
        }
    }

    /// Start from the beginning of the file.
    pub fn restart(&mut self) {
        *self = Self {
            running: true,
            ..Self::default()
        };
    }

    /// The symbols drawn: the last `points` with Life on, everything since Clear
    /// otherwise.
    pub fn shown(&self, life: bool, points: usize) -> (usize, usize) {
        let from = if life {
            self.pos.saturating_sub(points).max(self.clear_from)
        } else {
            self.clear_from
        };
        (from.min(self.pos), self.pos)
    }
}

/// The angles histogram: counts of the constellation angles, positive and negative
/// halves, 0.36 degrees per bin.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AngleHist {
    /// Counts of the positive angles, 0.36 degrees per bin.
    pub pos: Vec<u32>,
    /// Counts of the negative angles.
    pub neg: Vec<u32>,
    /// Symbols `from..to` are counted, with this trim and run.
    from: usize,
    to: usize,
    trim: i32,
    generation: u64,
}

impl AngleHist {
    /// Bring the counts to symbols `from..to` of `points` (incrementally while only
    /// `to` grows).
    pub fn update(
        &mut self,
        points: &[Complex32],
        from: usize,
        to: usize,
        trim: i32,
        generation: u64,
    ) {
        let fresh = self.pos.len() == HIST_BINS
            && self.from == from
            && self.trim == trim
            && self.generation == generation
            && to >= self.to;
        if !fresh {
            self.pos = vec![0; HIST_BINS];
            self.neg = vec![0; HIST_BINS];
            self.from = from;
            self.to = from;
            self.trim = trim;
            self.generation = generation;
        }
        let add = points.get(self.to..to.min(points.len())).unwrap_or(&[]);
        for p in add {
            let a = wrap180(f64::from(p.arg().to_degrees()) + f64::from(trim));
            let bin = ((a.abs() / 0.36) as usize).min(HIST_BINS - 1);
            let half = if a >= 0.0 {
                &mut self.pos
            } else {
                &mut self.neg
            };
            if let Some(c) = half.get_mut(bin) {
                *c = c.saturating_add(1);
            }
        }
        self.to = to.max(self.from);
    }

    /// The highest count of both halves.
    pub fn peak(&self) -> u32 {
        self.pos.iter().chain(&self.neg).copied().max().unwrap_or(0)
    }
}

/// The line of angle marker `k` in the text panel: `1 90,00 1-0=44,28`.
pub fn mark_line(marks: &[f64], k: usize) -> String {
    let Some(a) = marks.get(k) else {
        return String::new();
    };
    let value = format::dec(*a, 2);
    match k.checked_sub(1).and_then(|p| marks.get(p)) {
        Some(prev) => format!(
            "{k} {value} {k}-{}={}",
            k - 1,
            format::dec_trim(angle_between(*a, *prev), 2)
        ),
        None => format!("{k} {value}"),
    }
}

/// The header the demodulator writes before its data.
pub fn dem_header(diff: u8, dem: DemMode) -> Vec<String> {
    let mut lines = vec![
        t!("pp-dem-view-mode", diff = diff.to_string()),
        t!("pp-dem-mode", mode = dem.label()),
        t!("pp-dem-absolute"),
        t!("pp-dem-coding"),
    ];
    for k in 0..dem.m {
        lines.push(format!("{k} ==> {}", table_angle(dem.angle(k))));
    }
    lines.push(t!("pp-dem-data"));
    lines
}

/// The codes of symbols `from..to` of `points` turned by `trim` degrees, one
/// character each (0-9, then a-v).
pub fn dem_codes(points: &[Complex32], from: usize, to: usize, dem: DemMode, trim: i32) -> String {
    points
        .get(from..to.min(points.len()))
        .unwrap_or(&[])
        .iter()
        .filter_map(|p| {
            let a = f64::from(p.arg().to_degrees()) + f64::from(trim);
            char::from_digit(dem.code(a) % 36, 36)
        })
        .collect()
}

/// The ISD corrector's spectrum: what it is for and the values.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrKey {
    /// The signal.
    pub source: NodeId,
    /// The corrector's mode.
    pub mode: IsdMode,
    /// The corrector's `N`.
    pub n: u32,
    /// Carrier, Hz.
    pub carrier: f64,
    /// BR, Bd.
    pub baud: f64,
    /// Blocks averaged.
    pub blocks: usize,
    /// Position in the file in thousandths.
    pub pos: u32,
}

/// The corrector plot's trace: dB at frequencies `lo..hi` (relative to the carrier's
/// N-th harmonic in X^N mode, absolute in AM mode).
#[derive(Debug, Clone, PartialEq)]
pub struct CorrSpec {
    /// What it was computed for.
    pub key: CorrKey,
    /// The span shown, Hz.
    pub lo: f64,
    /// The highest frequency shown, Hz.
    pub hi: f64,
    /// Bin `k` of `db` is at `f0 + k * df` Hz (the kept bins start up to one
    /// bin inside `lo`).
    pub f0: f64,
    /// Bin spacing, Hz.
    pub df: f64,
    /// Levels of the bins, dB.
    pub db: Vec<f32>,
}

/// Half the span the corrector shows around the carrier, in BR.
const CORR_HALF_SPAN: f64 = 2.0;

/// FFT size for a resolution of about a fiftieth of `baud` at `fs`.
fn corr_fft(fs: f64, baud: f64) -> usize {
    let want = (fs / (baud / 50.0).max(1e-3)).max(1.0);
    (want as usize).next_power_of_two().clamp(512, 65_536)
}

/// The corrector's spectrum of `signal` for `key` (X^N: the analytic signal with its
/// phase multiplied by N, magnitude kept, mixed down by N times the carrier; AM: the
/// envelope).
pub fn corrector_spectrum(
    signal: &Signal,
    key: &CorrKey,
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<CorrSpec, JobError> {
    let fs = signal.sample_rate();
    let fft = corr_fft(fs, key.baud);
    let hop = fft / 2;
    let need = hop * (key.blocks + 1);
    let room = signal.len().saturating_sub(need);
    let start = (room as f64 * f64::from(key.pos) / 1000.0) as usize;
    let end = (start + need).min(signal.len());
    let part = signal
        .slice(start, end)
        .map_err(|e| JobError::Failed(e.to_string()))?;
    let z = sn_dsp::detect::analytic(&part, stop).map_err(dsp_to_job)?;
    let (samples, kind, lo, hi) = match key.mode {
        IsdMode::Xn => {
            let n = key.n.max(1);
            let nf = f64::from(n);
            let mut v: Vec<Complex32> = z
                .iter()
                .map(|c| Complex32::from_polar(c.norm(), c.arg() * n as f32))
                .collect();
            sn_dsp::mix::mix_in_place(&mut v, 0, -nf * key.carrier, fs);
            let half = (CORR_HALF_SPAN * key.baud).min(fs / 2.0);
            let s = Signal::complex(v, fs).map_err(|e| JobError::Failed(e.to_string()))?;
            (s, SampleKind::Complex, -half, half)
        }
        IsdMode::Am => {
            let mut env: Vec<f32> = z.iter().map(|c| c.norm()).collect();
            let mean = env.iter().sum::<f32>() / env.len().max(1) as f32;
            for e in &mut env {
                *e -= mean;
            }
            let hi = (2.0 * CORR_HALF_SPAN * key.baud).min(fs / 2.0);
            let s = Signal::real(env, fs).map_err(|e| JobError::Failed(e.to_string()))?;
            (s, SampleKind::Real, 0.0, hi)
        }
    };
    let engine = SpectrumEngine::new(fft, WindowKind::default(), kind).map_err(dsp_to_job)?;
    let starts: Vec<usize> = (0..key.blocks.max(1)).map(|b| b * hop).collect();
    let col = engine
        .column_db(&samples, &starts, Combine::Mean, stop)
        .map_err(dsp_to_job)?;
    let kept: Vec<(f64, f32)> = col
        .iter()
        .enumerate()
        .map(|(k, v)| (engine.bin_frequency(k, fs), *v))
        .filter(|(f, _)| (lo..=hi).contains(f))
        .collect();
    let f0 = kept.first().map_or(lo, |(f, _)| *f);
    Ok(CorrSpec {
        key: key.clone(),
        lo,
        hi,
        f0,
        df: fs / fft as f64,
        db: kept.into_iter().map(|(_, v)| v).collect(),
    })
}

/// What the spectrum mode's spectrum was computed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiresKey {
    /// The signal.
    pub node: NodeId,
    /// First sample.
    pub start: usize,
    /// End of the span (exclusive).
    pub end: usize,
    /// FFT size.
    pub fft: usize,
}

/// The high-resolution averaged spectrum: bin `k` is at `f0 + k * df` Hz.
#[derive(Debug, Clone, PartialEq)]
pub struct HiresSpec {
    /// What it was computed for.
    pub key: HiresKey,
    /// Frequency of bin 0, Hz.
    pub f0: f64,
    /// Bin spacing, Hz.
    pub df: f64,
    /// Levels of the bins, dB.
    pub db: Vec<f32>,
}

/// Most frames averaged into the high-resolution spectrum.
const HIRES_FRAMES: usize = 4000;

/// The averaged spectrum of samples `key.start..key.end` of `signal`. A span shorter
/// than the FFT is windowed on its own length and zero-padded to the FFT size, so no
/// sample outside it contributes and a tone reads the same level as on a long span.
pub fn hires_spectrum(
    signal: &Signal,
    key: HiresKey,
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<HiresSpec, JobError> {
    let fs = signal.sample_rate();
    let end = key.end.min(signal.len());
    let span = end.saturating_sub(key.start);
    if span < key.fft {
        // The window is applied to the span here, so the engine's is rectangular.
        let engine = SpectrumEngine::new(key.fft, WindowKind::Rectangular, signal.kind())
            .map_err(dsp_to_job)?;
        let padded = windowed_padded(signal, key.start, span, key.fft)?;
        let db = engine
            .column_db(&padded, &[0], Combine::Mean, stop)
            .map_err(dsp_to_job)?;
        return Ok(HiresSpec {
            key,
            f0: engine.bin_frequency(0, fs),
            df: fs / key.fft as f64,
            db,
        });
    }
    let engine =
        SpectrumEngine::new(key.fft, WindowKind::default(), signal.kind()).map_err(dsp_to_job)?;
    let frames = (span.saturating_sub(key.fft) / (key.fft / 2).max(1) + 1).clamp(1, HIRES_FRAMES);
    let step = if frames > 1 {
        span.saturating_sub(key.fft) / (frames - 1)
    } else {
        0
    };
    let starts: Vec<usize> = (0..frames).map(|i| key.start + i * step).collect();
    let db = engine
        .column_db(signal, &starts, Combine::Mean, stop)
        .map_err(dsp_to_job)?;
    Ok(HiresSpec {
        key,
        f0: engine.bin_frequency(0, fs),
        df: fs / key.fft as f64,
        db,
    })
}

/// Samples `start..start + span` of `signal` multiplied by the default window of
/// length `span`, followed by zeros up to `len` samples. The samples are also scaled
/// by `len / sum(w)`, so a rectangular `len`-point FFT (which divides by `len`) gives
/// a tone the same level as the default window over a full frame (which divides by
/// that window's sum).
fn windowed_padded(
    signal: &Signal,
    start: usize,
    span: usize,
    len: usize,
) -> Result<Signal, JobError> {
    let fs = signal.sample_rate();
    let span = span.min(len);
    let w = WindowKind::default().coefficients(span);
    let sum: f32 = w.iter().sum();
    let gain = if sum.is_finite() && sum > 0.0 {
        len as f32 / sum
    } else {
        0.0
    };
    let failed = |e: sn_core::Error| JobError::Failed(e.to_string());
    match signal.kind() {
        SampleKind::Real => {
            let mut v = vec![0.0f32; len];
            if let Some(dst) = v.get_mut(..span) {
                signal.read_real(start, dst);
                for (x, c) in dst.iter_mut().zip(&w) {
                    *x *= c * gain;
                }
            }
            Signal::real(v, fs).map_err(failed)
        }
        SampleKind::Complex => {
            let mut v = vec![Complex32::new(0.0, 0.0); len];
            if let Some(dst) = v.get_mut(..span) {
                signal.read_complex(start, dst);
                for (x, c) in dst.iter_mut().zip(&w) {
                    *x *= c * gain;
                }
            }
            Signal::complex(v, fs).map_err(failed)
        }
    }
}

/// The spectrum mode's frequency markers (Fl and Fu), Hz on the signal's axis.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpecMarks {
    /// The lower marker.
    pub fl: Option<f64>,
    /// The upper marker.
    pub fu: Option<f64>,
}

/// The whole module's state, per signal.
#[derive(Default)]
pub struct PhaseState {
    /// The window the form was last filled for: a new window starts afresh.
    pub window: Option<u64>,
    /// Spectrum or phase plane mode.
    pub mode: Mode,
    /// The values of the form's controls.
    pub form: Form,
    /// The file loaded with `LoadFile`, and its title.
    pub loaded: Option<NodeId>,
    /// The title of the loaded file's window.
    pub loaded_title: String,
    /// The ISD corrector's result (Corr-ON), a signal derived from the loaded one.
    pub corrected: Option<NodeId>,
    /// The playback through the file.
    pub play: Play,
    shown: Option<Shown>,
    /// Bumped with every new run, so drawings derived from it are rebuilt.
    pub generation: u64,
    job: Option<JobHandle<Shown>>,
    /// The ISD corrector's Auto step running.
    pub isd_job: Option<JobHandle<Signal>>,
    /// The corrector's spectrum.
    pub corr: Option<CorrSpec>,
    corr_job: Option<(CorrKey, JobHandle<CorrSpec>)>,
    /// The spectrum mode's spectrum.
    pub hires: Option<HiresSpec>,
    hires_job: Option<(HiresKey, JobHandle<HiresSpec>)>,
    /// The band the spectrum mode shows (the caller's H markers, or all of it).
    pub band: Option<(f64, f64)>,
    /// The spectrum mode's frequency markers.
    pub spec_marks: SpecMarks,
    /// Angle markers, signed degrees.
    pub marks: Vec<f64>,
    /// The angles histogram.
    pub hist: AngleHist,
    /// The last error, shown on the form.
    pub error: Option<String>,
    /// The `LoadFile` dialog is open.
    pub load_open: bool,
    /// The file selected in it.
    pub load_sel: Option<NodeId>,
    /// Show the eye diagram in the constellation square (SigNexora extra).
    pub eye: bool,
    /// The signal and edit of the running Auto step.
    pub isd_from: Option<(NodeId, sn_dsp::ops::Edit)>,
}

impl std::fmt::Debug for PhaseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhaseState")
            .field("mode", &self.mode)
            .field("form", &self.form)
            .finish_non_exhaustive()
    }
}

impl PhaseState {
    /// Start afresh for window `win` (SA opens the module in its spectrum mode), with
    /// Carrier and BR pre-filled from the last Sc and Br measurements.
    pub fn open(
        &mut self,
        win: u64,
        sc: Option<&MeasuredValue>,
        br: Option<&MeasuredValue>,
        band: Option<(f64, f64)>,
    ) {
        if self.window == Some(win) {
            return;
        }
        self.release();
        self.window = Some(win);
        self.mode = Mode::Spectrum;
        self.loaded = None;
        self.corrected = None;
        self.shown = None;
        self.corr = None;
        self.hires = None;
        self.play = Play::default();
        self.marks.clear();
        self.band = band;
        self.spec_marks = SpecMarks::default();
        if let Some(c) = sc.filter(|c| !c.other_axis) {
            self.form.carrier = format::dec_trim(c.value, 2);
        }
        if let Some(b) = br {
            self.form.br = format::dec_trim(b.value, 2);
        }
    }

    /// Load `node` (`LoadFile`): the module switches to its phase plane mode.
    pub fn load(&mut self, node: NodeId, title: String) {
        self.loaded = Some(node);
        self.loaded_title = title;
        self.corrected = None;
        self.isd_job = None;
        self.mode = Mode::Phase;
        self.play = Play::default();
        self.shown = None;
        self.job = None;
        self.form.isd_pos = 0.0;
    }

    /// The signal the constellation is drawn from: the corrected one when the ISD
    /// corrector is on.
    pub fn source(&self) -> Option<NodeId> {
        self.corrected.or(self.loaded)
    }

    /// A job is running.
    pub fn is_running(&self) -> bool {
        self.job.is_some()
            || self.isd_job.is_some()
            || self.corr_job.is_some()
            || self.hires_job.is_some()
            || self.play.running
    }

    /// The window closed: stop every job and the playback; settings stay.
    pub fn release(&mut self) {
        self.job = None;
        self.isd_job = None;
        self.isd_from = None;
        self.corr_job = None;
        self.hires_job = None;
        self.play.running = false;
        self.error = None;
        self.window = None;
        self.load_open = false;
    }

    /// The last finished run (it may be for older settings or an earlier source
    /// while a new one is computed).
    pub fn shown(&self) -> Option<&Shown> {
        self.shown.as_ref()
    }

    /// A note about the shown run that is not an error: only its start was
    /// demodulated (see [`run_end`]). Draw it in the normal text colour.
    pub fn notice(&self) -> Option<String> {
        self.shown
            .as_ref()
            .filter(|s| s.limited)
            .map(|s| t!("pp-run-limited", symbols = s.len().to_string()))
    }

    /// The run is being computed.
    pub fn computing(&self) -> bool {
        self.job.is_some()
    }

    /// Start or Break.
    pub fn start_or_break(&mut self) {
        if self.play.running {
            self.play.running = false;
            self.job = None;
        } else {
            self.play.restart();
            self.hist = AngleHist::default();
        }
    }

    /// Clear the display: what is drawn starts again from the playhead.
    pub fn clear(&mut self) {
        self.play.clear_from = self.play.pos;
        self.play.dem_from = self.play.pos;
        self.hist = AngleHist::default();
    }

    /// Per frame: collect finished jobs, (re)start the run when its settings or
    /// signal changed while running, and advance the playback to `now`.
    pub fn update(&mut self, now: f64, source: Option<(NodeId, &Signal)>) {
        self.poll();
        if self.play.running && self.job.is_none() {
            if let (Some(cfg), Some((id, signal))) = (self.form.config(), source) {
                if self
                    .shown
                    .as_ref()
                    .is_none_or(|s| s.cfg != cfg || s.source != id)
                {
                    self.spawn(cfg, id, signal);
                }
            } else {
                // Carrier or BR is not a number, or there is no file: say so
                // rather than stop silently or play a run for other settings.
                self.error = Some(t!("pp-check-settings"));
                self.play.running = false;
            }
        }
        if self.play.running && self.job.is_none() {
            match &self.shown {
                Some(s) => self
                    .play
                    .step(now, s.len(), self.form.delay, self.form.nonstop),
                None => self.play.running = false,
            }
        } else {
            self.play.last = None;
        }
    }

    fn spawn(&mut self, cfg: PhaseConfig, id: NodeId, signal: &Signal) {
        self.error = None;
        let s = signal.clone();
        self.job = Some(job::spawn(t!("phase-job"), move |ctx| {
            let stop = || ctx.is_cancelled();
            let (end, limited) = run_end(s.len(), s.sample_rate(), cfg.baud);
            let r = run(&s, 0, end, &cfg, &stop).map_err(demod_to_job)?;
            ctx.checkpoint()?;
            let mut shown = Shown::new(r, cfg, id);
            shown.limited = limited;
            Ok(shown)
        }));
    }

    fn poll(&mut self) {
        if self.job.as_ref().is_some_and(JobHandle::is_finished)
            && let Some(h) = self.job.take()
        {
            match h.join() {
                Ok(r) => {
                    self.play.pos = self.play.pos.min(r.len());
                    self.shown = Some(r);
                    self.generation = self.generation.wrapping_add(1);
                }
                Err(JobError::Cancelled) => {}
                Err(e) => {
                    self.error = Some(e.to_string());
                    self.play.running = false;
                }
            }
        }
        if self.corr_job.as_ref().is_some_and(|(_, h)| h.is_finished())
            && let Some((_, h)) = self.corr_job.take()
        {
            match h.join() {
                Ok(c) => self.corr = Some(c),
                Err(JobError::Cancelled) => {}
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        if self
            .hires_job
            .as_ref()
            .is_some_and(|(_, h)| h.is_finished())
            && let Some((_, h)) = self.hires_job.take()
        {
            match h.join() {
                Ok(s) => self.hires = Some(s),
                Err(JobError::Cancelled) => {}
                Err(e) => self.error = Some(e.to_string()),
            }
        }
    }

    /// Keep the corrector's spectrum up to date with `key` on `signal`.
    pub fn want_corr(&mut self, key: CorrKey, signal: &Signal) {
        if self.corr.as_ref().is_some_and(|c| c.key == key)
            || self.corr_job.as_ref().is_some_and(|(k, _)| *k == key)
        {
            return;
        }
        let s = signal.clone();
        let k = key.clone();
        let h = job::spawn(t!("pp-job-corrector"), move |ctx| {
            let stop = || ctx.is_cancelled();
            corrector_spectrum(&s, &k, &stop)
        });
        self.corr_job = Some((key, h));
    }

    /// Keep the spectrum mode's spectrum up to date with `key` on `signal`.
    pub fn want_hires(&mut self, key: HiresKey, signal: &Signal) {
        if self.hires.as_ref().is_some_and(|c| c.key == key)
            || self.hires_job.as_ref().is_some_and(|(k, _)| *k == key)
        {
            return;
        }
        let s = signal.clone();
        let h = job::spawn(t!("pp-job-spectrum"), move |ctx| {
            let stop = || ctx.is_cancelled();
            hires_spectrum(&s, key, &stop)
        });
        self.hires_job = Some((key, h));
    }

    /// The spectrum is being computed.
    pub fn spectrum_busy(&self) -> bool {
        self.hires_job.is_some() || self.corr_job.is_some()
    }

    /// Run one Auto step of the ISD corrector on `signal` (the corrected signal when
    /// the corrector is on): returns the edit it runs.
    pub fn start_isd(&mut self, signal: &Signal) -> Option<sn_dsp::ops::Edit> {
        let edit = self.isd_edit()?;
        let from = self.source()?;
        self.isd_from = Some((from, edit.clone()));
        let s = signal.clone();
        let e = edit.clone();
        self.isd_job = Some(job::spawn(t!("pp-job-isd"), move |ctx| {
            ctx.set_total(1);
            let stop = || ctx.is_cancelled();
            let out = e.run(&s, &stop).map_err(dsp_to_job)?;
            ctx.advance(1);
            Ok(out)
        }));
        Some(edit)
    }

    /// The equalizer the corrector's settings ask for. (B) off trains on half the
    /// blocks: SA's "other, slightly worse" algorithm.
    pub fn isd_edit(&self) -> Option<sn_dsp::ops::Edit> {
        let mode = match self.form.isd_mode {
            IsdMode::Am => sn_dsp::equalize::EqMode::Am,
            IsdMode::Xn => sn_dsp::equalize::EqMode::Xn {
                n: self.form.isd_order(),
            },
        };
        let blocks = if self.form.isd_b {
            self.form.blocks
        } else {
            (self.form.blocks / 2).max(5)
        };
        Some(sn_dsp::ops::Edit::Equalize {
            carrier: self.form.carrier_hz()?,
            baud: self.form.baud()?,
            mode,
            blocks,
        })
    }

    /// The finished Auto step's corrected signal (or its error), with the signal and
    /// edit it was made from.
    pub fn take_isd(&mut self) -> Option<IsdDone> {
        if !self.isd_job.as_ref().is_some_and(JobHandle::is_finished) {
            return None;
        }
        let from = self.isd_from.take();
        self.isd_job.take().map(|h| (h.join(), from))
    }

    /// Add an angle marker at `deg` (left click on the histogram); at most ten.
    pub fn add_mark(&mut self, deg: f64) {
        if self.marks.len() < MAX_MARKS {
            self.marks.push(deg);
        }
    }

    /// The text panel: the angle markers' lines (with their marker index), or the
    /// demodulator output.
    pub fn panel_lines(&self) -> Vec<(Option<usize>, String)> {
        if !self.marks.is_empty() {
            return (0..self.marks.len())
                .map(|k| (Some(k), mark_line(&self.marks, k)))
                .collect();
        }
        let Some(s) = self.shown.as_ref().filter(|_| self.form.output_dem) else {
            return Vec::new();
        };
        let mut lines: Vec<(Option<usize>, String)> = dem_header(self.form.diff, self.form.dem)
            .into_iter()
            .map(|l| (None, l))
            .collect();
        let from = self
            .play
            .dem_from
            .max(self.play.pos.saturating_sub(PANEL_CODES));
        lines.push((
            None,
            dem_codes(
                &s.r.display,
                from,
                self.play.pos,
                self.form.dem,
                self.form.trimming,
            ),
        ));
        lines
    }

    /// Every code written since Start (for copying).
    pub fn all_codes(&self) -> String {
        self.shown.as_ref().map_or_else(String::new, |s| {
            dem_codes(
                &s.r.display,
                self.play.dem_from,
                self.play.pos,
                self.form.dem,
                self.form.trimming,
            )
        })
    }

    /// Bring the angles histogram up to the playhead.
    pub fn update_hist(&mut self) {
        let Some(s) = &self.shown else {
            return;
        };
        let (from, to) = (self.play.clear_from, self.play.pos);
        self.hist
            .update(&s.r.display, from, to, self.form.trimming, self.generation);
    }
}

/// A finished Auto step: the corrected signal or the error, and what it was made from.
pub type IsdDone = (
    Result<Signal, JobError>,
    Option<(NodeId, sn_dsp::ops::Edit)>,
);

/// Codes kept in the text panel (the latest ones; all of them can be copied).
const PANEL_CODES: usize = 6000;

/// Points of the trajectory per symbol.
pub const TRAJ_PER_SYMBOL: usize = SPS as usize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dem_modes_follow_sa_list_and_coding() {
        let all = DemMode::all();
        assert_eq!(all.len(), 31 + 9);
        assert_eq!(all.first().map(|d| d.label()), Some("2".to_owned()));
        assert_eq!(all.last().map(|d| d.label()), Some("10-s".to_owned()));
        let d8 = DemMode {
            m: 8,
            shifted: false,
        };
        // R139/a2: 0 ==> 00,00 ... 4 ==> 180,00, 5 ==> -135,00, 7 ==> -45,00.
        assert!((d8.angle(4) - 180.0).abs() < 1e-9);
        assert!((d8.angle(5) + 135.0).abs() < 1e-9);
        assert!((d8.angle(7) + 45.0).abs() < 1e-9);
        assert_eq!(d8.code(44.0), 1);
        assert_eq!(d8.code(-46.0), 7);
        assert_eq!(d8.code(179.0), 4);
        assert_eq!(d8.code(-179.0), 4);
        // 4-s: the zero vector half a sector below 0 (the light blue vector of R139/a3).
        let d4s = DemMode {
            m: 4,
            shifted: true,
        };
        assert!((d4s.angle(0) + 45.0).abs() < 1e-9);
        assert!((d4s.angle(1) - 45.0).abs() < 1e-9);
        assert_eq!(d4s.code(-40.0), 0);
        assert_eq!(d4s.code(130.0), 2);
    }

    #[test]
    fn angles_wrap_and_format_like_sa() {
        assert!((wrap180(270.0) + 90.0).abs() < 1e-9);
        assert!((wrap180(-180.0) - 180.0).abs() < 1e-9);
        assert!((angle_between(-135.72, 180.0) - 44.28).abs() < 1e-9);
        assert!((angle_between(90.0, 45.72) - 44.28).abs() < 1e-9);
        // The separator follows the interface language; compare with a point.
        let pt = |s: String| s.replace(',', ".");
        assert_eq!(pt(table_angle(0.0)), "00.00");
        assert_eq!(pt(table_angle(45.0)), "45.00");
        assert_eq!(pt(table_angle(-135.0)), "-135.00");
        // R136/a5: absolute values, then the difference to the previous marker.
        let marks = [45.72, 90.0, 135.0, 180.0, -135.72];
        assert_eq!(pt(mark_line(&marks, 0)), "0 45.72");
        assert_eq!(pt(mark_line(&marks, 1)), "1 90.00 1-0=44.28");
        assert_eq!(pt(mark_line(&marks, 2)), "2 135.00 2-1=45");
        assert_eq!(pt(mark_line(&marks, 4)), "4 -135.72 4-3=44.28");
    }

    #[test]
    fn playback_wraps_with_nonstop_and_stops_without() {
        let mut p = Play::default();
        p.restart();
        p.step(0.0, 100, 0.0, false);
        assert_eq!(p.pos, 0);
        p.step(0.01, 100, 0.0, false);
        assert_eq!(p.pos, 40);
        p.step(0.05, 100, 0.0, false);
        assert_eq!(p.pos, 100);
        assert!(!p.running, "stops at the end without NonStop");
        p.restart();
        p.step(0.0, 100, 0.0, true);
        p.step(0.03, 100, 0.0, true);
        assert_eq!(p.pos, 0, "starts over with NonStop");
        assert!(p.running);
        // Delay: one symbol per Delay seconds.
        p.restart();
        p.step(0.0, 100, 0.1, false);
        p.step(0.2, 100, 0.1, false);
        assert_eq!(p.pos, 2);
        // Life shows the last Points symbols; without it everything since Clear.
        p.pos = 50;
        p.clear_from = 10;
        assert_eq!(p.shown(true, 20), (30, 50));
        assert_eq!(p.shown(true, 100), (10, 50));
        assert_eq!(p.shown(false, 20), (10, 50));
    }

    #[test]
    fn histogram_splits_signs_and_counts_incrementally() {
        let pts = [
            Complex32::from_polar(1.0, 0.5_f32.to_radians()),
            Complex32::from_polar(1.0, -90.0_f32.to_radians()),
            Complex32::from_polar(1.0, 179.9_f32.to_radians()),
        ];
        let mut h = AngleHist::default();
        h.update(&pts, 0, 2, 0, 1);
        assert_eq!(h.pos.iter().sum::<u32>(), 1);
        assert_eq!(h.neg.iter().sum::<u32>(), 1);
        assert_eq!(h.neg.get(250), Some(&1), "-90 degrees in bin 250");
        h.update(&pts, 0, 3, 0, 1);
        assert_eq!(h.pos.iter().sum::<u32>(), 2);
        assert_eq!(h.pos.get(HIST_BINS - 1), Some(&1));
        // A trim turns every angle and rebuilds the counts.
        h.update(&pts, 0, 1, 10, 1);
        assert_eq!(h.pos.get(29), Some(&1), "10.5 degrees");
        assert_eq!(h.peak(), 1);
    }

    #[test]
    fn quality_rises_with_cleaner_points_and_codes_use_the_dem_mode() {
        let clean: Vec<Complex32> = (0..64)
            .map(|k| Complex32::from_polar(1.0, (k % 4) as f32 * std::f32::consts::FRAC_PI_2))
            .collect();
        let noisy: Vec<Complex32> = clean
            .iter()
            .enumerate()
            .map(|(k, p)| p * Complex32::from_polar(1.0, if k % 2 == 0 { 0.2 } else { -0.2 }))
            .collect();
        let q_clean = quality(&clean, 4).unwrap_or(0.0);
        let q_noisy = quality(&noisy, 4).unwrap_or(0.0);
        assert!(q_clean > 60.0, "{q_clean}");
        assert!(q_noisy < 20.0 && q_noisy > 10.0, "{q_noisy}");
        assert_eq!(quality(&[], 4), None);
        let d4 = DemMode {
            m: 4,
            shifted: false,
        };
        assert_eq!(dem_codes(&clean, 0, 6, d4, 0), "012301");
        assert_eq!(dem_codes(&clean, 4, 999, d4, 90).len(), 60);
        assert!(dem_codes(&clean, 0, 2, d4, 90).starts_with("12"));
    }

    #[test]
    fn form_builds_the_config_and_pre_fills_once_per_window() {
        let mut f = Form {
            carrier: "1800,02".to_owned(),
            rrc: true,
            ..Form::default()
        };
        let cfg = f.config();
        assert_eq!(cfg.map(|c| c.carrier), Some(1800.02));
        assert_eq!(cfg.and_then(|c| c.rrc_alpha), Some(0.5));
        f.br = "abc".to_owned();
        assert!(f.config().is_none());
        assert_eq!(f.isd_order(), 2, "N 1 falls back to n-Ary");
        f.isd_n = 8;
        assert_eq!(f.isd_order(), 8);

        let mut st = PhaseState::default();
        let sc = MeasuredValue {
            value: 1729.32,
            from: String::new(),
            other_axis: false,
        };
        st.open(7, Some(&sc), None, None);
        assert_eq!(st.mode, Mode::Spectrum);
        assert_eq!(format::parse_number(&st.form.carrier), Some(1729.32));
        st.form.carrier = "5".to_owned();
        st.open(7, Some(&sc), None, None);
        assert_eq!(st.form.carrier, "5", "the same window keeps its fields");
        st.load(NodeId::from_raw(3), "x".to_owned());
        assert_eq!(st.mode, Mode::Phase);
        assert_eq!(st.source(), Some(NodeId::from_raw(3)));
        st.open(8, Some(&sc), None, None);
        assert_eq!(
            st.mode,
            Mode::Spectrum,
            "a new window starts in spectrum mode"
        );
    }

    #[test]
    fn start_with_bad_settings_stops_with_an_error() {
        let s = Signal::real(vec![0.0f32; 1000], 8000.0).expect("signal");
        let mut st = PhaseState::default();
        st.form.carrier = "1000".to_owned();
        st.form.br = "abc".to_owned();
        st.start_or_break();
        assert!(st.play.running);
        st.update(0.0, Some((NodeId::from_raw(1), &s)));
        assert!(!st.play.running);
        assert!(st.error.is_some());
        assert!(!st.computing());

        let mut st = PhaseState::default();
        st.form.carrier = "1000".to_owned();
        st.form.br = "100".to_owned();
        st.start_or_break();
        st.update(0.0, None);
        assert!(!st.play.running && st.error.is_some(), "no file loaded");
    }

    #[test]
    fn limited_run_is_a_notice_not_an_error() {
        let fs = 8000.0;
        let x: Vec<f32> = (0..8000)
            .map(|i| (std::f64::consts::TAU * 1000.0 * f64::from(i) / fs).cos() as f32)
            .collect();
        let s = Signal::real(x, fs).expect("signal");
        let mut st = PhaseState::default();
        st.form.carrier = "1000".to_owned();
        st.form.br = "100".to_owned();
        let cfg = st.form.config().expect("config");
        let r = run(&s, 0, s.len(), &cfg, &|| false).expect("run");
        st.shown = Some(Shown::new(r, cfg, NodeId::from_raw(1)));
        assert!(st.notice().is_none(), "a full run has no notice");
        if let Some(shown) = st.shown.as_mut() {
            shown.limited = true;
        }
        let note = st.notice().expect("a limited run says so");
        assert!(st.error.is_none(), "a limited run is not an error");
        assert!(!note.contains("shorter file"), "{note}");
    }

    #[test]
    fn run_end_caps_the_symbols_and_the_samples() {
        assert_eq!(run_end(80_000, 8000.0, 100.0), (80_000, false));
        // Heavily oversampled: the sample bound applies before the symbol bound.
        let (end, limited) = run_end(usize::MAX / 2, 8000.0, 100.0);
        assert!(limited);
        assert_eq!(end, MAX_RUN_SAMPLES);
        let (end, limited) = run_end(600_000_000, 1e6, 1000.0);
        assert!(limited && end == MAX_RUN_SAMPLES);
        // Two samples per symbol: the symbol bound applies first.
        let (end, limited) = run_end(usize::MAX / 2, 2000.0, 1000.0);
        assert!(limited);
        assert_eq!(end, MAX_RUN_SYMBOLS * 2);
        // A high BR shortens the span further.
        let (end, limited) = run_end(10_000_000, 8000.0, 1e6);
        assert!(limited && end == 8000);
    }

    #[test]
    fn hires_short_span_uses_only_its_samples() {
        // Silence then a loud tone: a short span in the silence must stay silent.
        let fs = 8000.0;
        let mut x = vec![0.0f32; 1000];
        x.extend(
            (0..8000).map(|i| (std::f64::consts::TAU * 1000.0 * f64::from(i) / fs).cos() as f32),
        );
        let s = Signal::real(x, fs).expect("signal");
        let key = HiresKey {
            node: NodeId::from_raw(0),
            start: 0,
            end: 500,
            fft: 4096,
        };
        let h = hires_spectrum(&s, key, &|| false).expect("hires");
        let loud = HiresKey {
            start: 1000,
            end: 1500,
            ..key
        };
        let t = hires_spectrum(&s, loud, &|| false).expect("hires");
        let peak = |v: &[f32]| v.iter().copied().fold(f32::MIN, f32::max);
        assert!(peak(&t.db) > peak(&h.db) + 40.0);
    }

    #[test]
    fn hires_short_span_reads_a_tone_at_the_long_span_level() {
        // A full-scale tone on a bin centre: a span of a quarter FFT must read within
        // 0.5 dB of a span of four FFTs.
        let fs = 8192.0;
        let fft = 4096;
        let x: Vec<f32> = (0..4 * fft)
            .map(|i| (std::f64::consts::TAU * 1024.0 * i as f64 / fs).cos() as f32)
            .collect();
        let s = Signal::real(x, fs).expect("signal");
        let key = |end| HiresKey {
            node: NodeId::from_raw(0),
            start: 0,
            end,
            fft,
        };
        let peak = |v: &[f32]| v.iter().copied().fold(f32::MIN, f32::max);
        let long = hires_spectrum(&s, key(4 * fft), &|| false).expect("hires");
        let short = hires_spectrum(&s, key(fft / 4), &|| false).expect("hires");
        let (a, b) = (peak(&long.db), peak(&short.db));
        assert!(a.abs() < 0.5, "long span peak {a} dB");
        assert!((a - b).abs() < 0.5, "long {a} dB, short {b} dB");
    }

    #[test]
    fn corrector_spectrum_shows_the_carrier_line_at_zero() {
        // A tone at 1000 Hz: in X^2 mode its second harmonic, mixed down by twice the
        // carrier, sits at 0 Hz.
        let fs = 8000.0;
        let x: Vec<f32> = (0..32_000)
            .map(|i| (std::f64::consts::TAU * 1000.0 * f64::from(i) / fs).cos() as f32)
            .collect();
        let s = Signal::real(x, fs).expect("signal");
        let key = CorrKey {
            source: NodeId::from_raw(0),
            mode: IsdMode::Xn,
            n: 2,
            carrier: 1000.0,
            baud: 400.0,
            blocks: 10,
            pos: 0,
        };
        let c = corrector_spectrum(&s, &key, &|| false).expect("spectrum");
        assert!((c.lo + 800.0).abs() < 1e-9 && (c.hi - 800.0).abs() < 1e-9);
        let peak =
            c.db.iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map_or(0, |(k, _)| k);
        assert!(
            c.f0 >= c.lo && c.f0 - c.lo < c.df,
            "f0 {} lo {}",
            c.f0,
            c.lo
        );
        let f = c.f0 + peak as f64 * c.df;
        assert!(f.abs() <= c.df, "line at {f} Hz");
        let h = hires_spectrum(
            &s,
            HiresKey {
                node: NodeId::from_raw(0),
                start: 0,
                end: s.len(),
                fft: 4096,
            },
            &|| false,
        )
        .expect("hires");
        let k =
            h.db.iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map_or(0, |(k, _)| k);
        assert!((h.f0 + h.df * k as f64 - 1000.0).abs() < 2.0);
    }
}
