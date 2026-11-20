//! The signal generator (OCG) without its window: the actions it hands the
//! app, the preview (a shortened copy of the spec generated on a background
//! job), the OFDM calculators' glue, and the text forms of its list fields.

use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sn_core::job::{self, JobError, JobHandle};
use sn_core::{Complex32, SampleKind};
use sn_dsp::spectrum::{Combine, SpectrumEngine};
use sn_dsp::window::WindowKind;
use sn_synth::calc::{self, KCandidate};
use sn_synth::{GeneratorSpec, Length, NoiseLevel, OfdmModulation, PilotPattern, Plan, SignalKind};

use crate::edits::SaveFormat;
use crate::t;

/// How long the parameters must stay unchanged before a new preview is made.
pub const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(300);
/// Most samples the preview generates.
pub const PREVIEW_MAX_SAMPLES: usize = 1 << 18;
/// Most symbols the preview generates.
pub const PREVIEW_MAX_SYMBOLS: usize = 256;
/// Longest preview of tones and noise in seconds.
pub const PREVIEW_MAX_SECONDS: f64 = 0.5;
/// Span of the waveform preview in seconds.
pub const PREVIEW_WAVE_SECONDS: f64 = 0.05;
/// Largest FFT of the spectrum preview.
pub const PREVIEW_FFT: usize = 1024;
/// Most frames averaged into the spectrum preview.
pub const PREVIEW_FRAMES: usize = 256;
/// Most positions a list field may expand to.
pub const MAX_LIST: usize = 65_536;
/// Largest K the calculators accept (as `sn_synth::calc`).
pub const MAX_K: f64 = 16.0;

/// Largest measurement uncertainty the K list accepts, in percent of each rate.
pub const MAX_K_UNCERTAINTY_PCT: f64 = 10.0;

/// Every OFDM channel modulation, in menu order.
pub const OFDM_MODULATIONS: [OfdmModulation; 7] = [
    OfdmModulation::Bpsk,
    OfdmModulation::Qpsk,
    OfdmModulation::Pi4Dqpsk,
    OfdmModulation::Psk8,
    OfdmModulation::Qam16,
    OfdmModulation::Qam64,
    OfdmModulation::Pilot,
];

/// What the user asked for this frame.
#[derive(Debug, Clone)]
pub enum GeneratorAction {
    /// Generate `spec` as a new root signal, then save it in `save` format if set.
    Generate {
        /// What to generate.
        spec: Box<GeneratorSpec>,
        /// Save the result in this format.
        save: Option<SaveFormat>,
    },
}

/// What the preview shows.
#[derive(Debug, Clone)]
pub struct Preview {
    /// Mean power spectrum in dB, low to high frequency.
    pub spectrum: Vec<f32>,
    /// Frequency of the first spectrum bin in Hz.
    pub lo_hz: f64,
    /// Frequency of the last spectrum bin in Hz.
    pub hi_hz: f64,
    /// The first [`PREVIEW_WAVE_SECONDS`] of I (or the real signal).
    pub wave_i: Vec<f32>,
    /// The same span of Q, for a complex signal.
    pub wave_q: Option<Vec<f32>>,
    /// Seconds the waveform covers.
    pub wave_seconds: f64,
}

/// A K fraction to copy into the OFDM editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseFraction {
    /// Numerator (LG per unit).
    pub p: u64,
    /// Denominator (LU per unit).
    pub q: u64,
}

/// The List of K: the simplest fractions within the measurement uncertainty of K.
/// From rates, each of Br and the spacing is `pct` percent uncertain; for K entered
/// directly, its tolerance is that of a K measured from two rates each `pct` percent
/// uncertain, `(1 + K) * 2 * pct / 100`.
pub fn k_candidates(
    from_rates: bool,
    br: f64,
    spacing: f64,
    k: f64,
    pct: f64,
    max_den: u32,
    count: usize,
) -> Result<Vec<KCandidate>, String> {
    if !(pct.is_finite() && (0.0..=MAX_K_UNCERTAINTY_PCT).contains(&pct)) {
        return Err(t!(
            "gen-calc-uncertainty-bad",
            max = crate::format::dec_trim(MAX_K_UNCERTAINTY_PCT, 2)
        ));
    }
    let rel = pct / 100.0;
    let r = if from_rates {
        calc::k_list_from_measured_rates(br, br * rel, spacing, spacing * rel, max_den, count)
    } else {
        let tolerance = ((1.0 + k) * 2.0 * rel).min(MAX_K);
        calc::k_list_within(k, tolerance, max_den, count)
    };
    r.map_err(|e| e.to_string())
}

/// Name of a kind in the kind menu.
pub fn kind_label(k: SignalKind) -> String {
    match k {
        SignalKind::Tones => t!("gen-kind-tones"),
        SignalKind::Noise => t!("gen-kind-noise"),
        SignalKind::Fsk => t!("gen-kind-fsk"),
        SignalKind::Linear => t!("gen-kind-linear"),
        SignalKind::C4fm => t!("gen-kind-c4fm"),
        SignalKind::Ofdm => t!("gen-kind-ofdm"),
    }
}

/// The spec's complex flag, for when it does not validate.
pub fn spec_is_complex(spec: &GeneratorSpec) -> bool {
    match spec {
        GeneratorSpec::Tones(s) => s.complex,
        GeneratorSpec::Noise(s) => s.complex,
        GeneratorSpec::Fsk(s) => s.complex,
        GeneratorSpec::Linear(s) => s.complex,
        GeneratorSpec::C4fm(_) => false,
        GeneratorSpec::Ofdm(s) => s.complex,
    }
}

/// LU and LG as whole sample counts the generator accepts, or `None`.
/// [`check_lengths`] says why not.
pub fn whole_lengths(lu: f64, lg: f64) -> Option<(usize, usize)> {
    check_lengths(lu, lg).ok()
}

/// LU and LG as whole sample counts the generator accepts, or the reason
/// (translated) they are not, for a disabled `Use in OFDM` button.
pub fn check_lengths(lu: f64, lg: f64) -> Result<(usize, usize), String> {
    let whole = |x: f64| x.is_finite() && x >= 0.0 && x.fract() == 0.0;
    if !whole(lu) || lu < 2.0 {
        return Err(t!("gen-calc-lu-not-whole"));
    }
    if !whole(lg) {
        return Err(t!("gen-calc-lg-not-whole"));
    }
    if lu > sn_synth::OFDM_MAX_LU as f64 {
        return Err(t!(
            "gen-calc-lu-too-large",
            max = sn_synth::OFDM_MAX_LU.to_string()
        ));
    }
    if lg > sn_synth::OFDM_MAX_LG as f64 {
        return Err(t!(
            "gen-calc-lg-too-large",
            max = sn_synth::OFDM_MAX_LG.to_string()
        ));
    }
    Ok((lu as usize, lg as usize))
}

/// LU and LG for K = `f.p / f.q` from the current LU: the smallest multiple of the
/// denominator not below it, and the matching LG; `None` when they would not
/// fit the generator's limits.
pub fn fraction_lengths(lu_now: usize, f: UseFraction) -> Option<(usize, usize)> {
    if f.q == 0 {
        return None;
    }
    let m = (lu_now as u64).div_ceil(f.q).max(1);
    let lu = usize::try_from(f.q.checked_mul(m)?).ok()?;
    let lg = usize::try_from(f.p.checked_mul(m)?).ok()?;
    (lu <= sn_synth::OFDM_MAX_LU && lg <= sn_synth::OFDM_MAX_LG).then_some((lu, lg))
}

/// A new random seed, different from `old` (kept below 2^31 so it edits exactly).
pub fn new_seed(old: u64) -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut rng = sn_synth::Rng::new(nanos ^ old.rotate_left(17));
    loop {
        let s = rng.next_u64() % (1 << 31);
        if s != old {
            return s;
        }
    }
}

/// `x` with at most `decimals` decimals and no trailing zeros.
pub fn trim(x: f64, decimals: usize) -> String {
    let s = format!("{x:.decimals$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    };
    if s == "-0" { "0".to_owned() } else { s }
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// `p/q` in lowest terms.
pub fn fraction(p: usize, q: usize) -> String {
    let g = gcd(p, q).max(1);
    format!("{}/{}", p / g, q / g)
}

/// Parse "3, 7, 10-12" into positions. Spaces and empty items are ignored; the error
/// is a message naming the item that could not be read.
pub fn parse_list(text: &str) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for item in text
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let range = parse_range(item).ok_or_else(|| t!("gen-list-bad", item = item.to_owned()))?;
        let count = range.end().saturating_sub(*range.start()).saturating_add(1);
        if out.len().saturating_add(count) > MAX_LIST {
            return Err(t!("gen-list-too-long", max = MAX_LIST.to_string()));
        }
        out.extend(range);
    }
    Ok(out)
}

/// "7" or "10-12" (also "10..12").
fn parse_range(item: &str) -> Option<RangeInclusive<usize>> {
    let (a, b) = item
        .split_once("..")
        .or_else(|| item.split_once('-'))
        .unwrap_or((item, item));
    let a: usize = a.trim().parse().ok()?;
    let b: usize = b.trim().parse().ok()?;
    (a <= b).then_some(a..=b)
}

/// Write positions back as text, runs of three or more as ranges.
pub fn format_list(list: &[usize]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while let Some(&start) = list.get(i) {
        let mut j = i;
        while list
            .get(j + 1)
            .is_some_and(|n| Some(*n) == list.get(j).and_then(|p| p.checked_add(1)))
        {
            j += 1;
        }
        let end = list.get(j).copied().unwrap_or(start);
        if j >= i + 2 {
            parts.push(format!("{start}-{end}"));
        } else {
            parts.extend(
                list.get(i..=j)
                    .unwrap_or_default()
                    .iter()
                    .map(usize::to_string),
            );
        }
        i = j + 1;
    }
    parts.join(", ")
}

/// A modulation name in a forgiving form: lower case without separators.
fn squash(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The OFDM modulation named `name` ("QAM-16", "qam16", "pi/4-DQPSK", "pilot").
pub fn modulation_named(name: &str) -> Option<OfdmModulation> {
    let want = squash(name);
    OFDM_MODULATIONS.into_iter().find(|m| {
        let serde_name = serde_json::to_value(m)
            .ok()
            .and_then(|v| v.as_str().map(squash))
            .unwrap_or_default();
        squash(m.name()) == want || serde_name == want
    })
}

/// Parse "0: pilot, 5-6: QAM-16" into per-position overrides.
pub fn parse_overrides(text: &str) -> Result<Vec<(usize, OfdmModulation)>, String> {
    let mut out = Vec::new();
    for item in text
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let bad = || t!("gen-overrides-bad", item = item.to_owned());
        let (pos, name) = item.split_once([':', '=']).ok_or_else(bad)?;
        let m = modulation_named(name).ok_or_else(bad)?;
        let range = parse_range(pos.trim()).ok_or_else(bad)?;
        let count = range.end().saturating_sub(*range.start()).saturating_add(1);
        if out.len().saturating_add(count) > MAX_LIST {
            return Err(t!("gen-list-too-long", max = MAX_LIST.to_string()));
        }
        out.extend(range.map(|p| (p, m)));
    }
    Ok(out)
}

/// Write overrides back as text.
pub fn format_overrides(list: &[(usize, OfdmModulation)]) -> String {
    list.iter()
        .map(|(p, m)| format!("{p}: {}", m.name()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why the preview shows no signal.
#[derive(Debug, Clone, PartialEq)]
pub enum NoPreview {
    /// The preview job failed (drawn as an error).
    Failed(String),
    /// One symbol is too long to preview; a note, not a failure: Generate
    /// still works.
    TooLong(String),
}

/// A short copy of `spec` for the preview: at most [`PREVIEW_MAX_SYMBOLS`] symbols
/// or [`PREVIEW_MAX_SECONDS`], and at most [`PREVIEW_MAX_SAMPLES`] samples in
/// all, the pulse filter's tail included (a small roll-off adds up to
/// `MAX_PULSE_SPAN` symbols). A spec whose single symbol with its tail is longer
/// than that has no preview; the error says so.
pub fn preview_spec(spec: &GeneratorSpec, plan: &Plan) -> Result<GeneratorSpec, String> {
    let mut short = spec.clone();
    let fs = spec.common().sample_rate;
    let max = PREVIEW_MAX_SAMPLES as f64;
    match (plan.symbol_rate, plan.symbols) {
        (Some(rate), Some(symbols)) if rate > 0.0 => {
            let per_symbol = (fs / rate).max(1.0);
            // The size of a one-symbol signal is one symbol plus the fixed
            // overhead (the pulse tail) every preview carries.
            short.common_mut().length = Length::Symbols(1);
            let one = short
                .plan()
                .map_or(per_symbol, |p| (p.samples as f64).max(per_symbol));
            if one > max {
                return Err(t!(
                    "gen-preview-symbol-too-long",
                    samples = format!("{one:.0}")
                ));
            }
            let tail = one - per_symbol;
            let fit = ((max - tail) / per_symbol).floor() as usize;
            let n = symbols.min(fit.clamp(1, PREVIEW_MAX_SYMBOLS)).max(1);
            short.common_mut().length = Length::Symbols(n);
        }
        _ => {
            let seconds = plan
                .duration(fs)
                .min(PREVIEW_MAX_SECONDS)
                .min(max / fs)
                .max(1.0 / fs);
            short.common_mut().length = Length::Seconds(seconds);
        }
    }
    Ok(short)
}

/// Generate `spec` and work out its spectrum and the start of its waveform.
pub fn build_preview(
    spec: &GeneratorSpec,
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<Preview, JobError> {
    // Cancellable while it generates, so Cancel and a newer edit stop it.
    let made =
        sn_synth::generate_with_stop(spec, stop).map_err(|e| crate::jobs::synth_to_job(&e))?;
    if stop() {
        return Err(JobError::Cancelled);
    }
    let s = &made.signal;
    let n = s.len();
    let fs = s.sample_rate();
    let mut fft = PREVIEW_FFT;
    while fft > 16 && fft > n {
        fft /= 2;
    }
    let (spectrum, lo_hz, hi_hz) = if n >= fft && fft >= 16 {
        let engine = SpectrumEngine::new(fft, WindowKind::Hann, s.kind())
            .map_err(crate::jobs::dsp_to_job)?;
        let last = n - fft;
        let hop = (fft / 2).max(1).max(last / PREVIEW_FRAMES);
        let starts: Vec<usize> = (0..=last).step_by(hop).collect();
        let column = engine
            .column_db(s, &starts, Combine::Mean, stop)
            .map_err(crate::jobs::dsp_to_job)?;
        let (lo, hi) = engine.frequency_range(fs);
        // A complex signal is tagged with its centre frequency: show the same
        // absolute axis the app shows once it is generated.
        let rf = match s.kind() {
            SampleKind::Complex => s.center_frequency(),
            SampleKind::Real => 0.0,
        };
        (column, lo + rf, hi + rf)
    } else {
        (Vec::new(), 0.0, fs / 2.0)
    };
    let m = n
        .min((PREVIEW_WAVE_SECONDS * fs).ceil() as usize)
        .min(1 << 16);
    let (wave_i, wave_q) = match s.kind() {
        SampleKind::Real => {
            let mut v = vec![0.0_f32; m];
            let got = s.read_real(0, &mut v);
            v.truncate(got);
            (v, None)
        }
        SampleKind::Complex => {
            let mut v = vec![Complex32::new(0.0, 0.0); m];
            let got = s.read_complex(0, &mut v);
            v.truncate(got);
            (
                v.iter().map(|z| z.re).collect(),
                Some(v.iter().map(|z| z.im).collect()),
            )
        }
    };
    let wave_seconds = wave_i.len() as f64 / fs;
    Ok(Preview {
        spectrum,
        lo_hz,
        hi_hz,
        wave_i,
        wave_q,
        wave_seconds,
    })
}

/// A calculator result to copy into the OFDM editor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UseLengths {
    /// The sample rate to set, if the calculator has one.
    pub fd: Option<f64>,
    /// The useful length in samples.
    pub lu: usize,
    /// The guard length in samples.
    pub lg: usize,
}

/// The texts of the OFDM list fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListTexts {
    /// The unused carriers.
    pub unused: String,
    /// The per-carrier modulation overrides.
    pub overrides: String,
    /// The pilot carrier positions.
    pub pilots: String,
}

/// The preview and the spec it was made from (or why there is none).
pub type Shown = (GeneratorSpec, Result<Arc<Preview>, NoPreview>);

/// The generator's state without its window: the spec being edited, the
/// specs of the other kinds (so switching kinds keeps the edits), the K
/// fraction the calculators refused, and the preview (a shortened copy of the
/// spec generated on a background job once the parameters have settled).
pub struct GenState {
    /// The spec being edited.
    pub spec: GeneratorSpec,
    /// The last spec of each other kind.
    stash: HashMap<SignalKind, GeneratorSpec>,
    /// Why the last K fraction picked was not used, with the LU and LG it was
    /// refused for: shown only while they stand.
    fraction_error: Option<(String, (usize, usize))>,
    /// The preview shown, with the spec it was made from.
    pub preview: Option<Shown>,
    preview_job: Option<(GeneratorSpec, JobHandle<Preview>)>,
    /// A spec waiting for the debounce, and since when.
    preview_wanted: Option<(GeneratorSpec, Instant)>,
    /// Show the waveform instead of the spectrum.
    pub show_wave: bool,
}

impl std::fmt::Debug for GenState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenState")
            .field("spec", &self.spec)
            .finish_non_exhaustive()
    }
}

impl Drop for GenState {
    fn drop(&mut self) {
        self.stop_preview();
    }
}

impl GenState {
    /// The state editing `spec`.
    pub fn new(spec: GeneratorSpec) -> Self {
        Self {
            spec,
            stash: HashMap::new(),
            fraction_error: None,
            preview: None,
            preview_job: None,
            preview_wanted: None,
            show_wave: false,
        }
    }

    /// The list fields' texts for the current spec.
    pub fn list_texts(&self) -> ListTexts {
        match &self.spec {
            GeneratorSpec::Ofdm(o) => ListTexts {
                unused: format_list(&o.unused),
                overrides: format_overrides(&o.overrides),
                pilots: match &o.pilots {
                    PilotPattern::Positions(p) => format_list(p),
                    _ => String::new(),
                },
            },
            _ => ListTexts::default(),
        }
    }

    /// Parse the list fields into the spec; the first problem as a message.
    pub fn apply_texts(&mut self, texts: &ListTexts) -> Result<(), String> {
        let GeneratorSpec::Ofdm(o) = &mut self.spec else {
            return Ok(());
        };
        let field = |key: &str, e: String| t!("gen-field-error", field = t!(key), error = e);
        let unused = parse_list(&texts.unused).map_err(|e| field("gen-ofdm-unused", e))?;
        let overrides =
            parse_overrides(&texts.overrides).map_err(|e| field("gen-ofdm-overrides", e))?;
        o.unused = unused;
        o.overrides = overrides;
        if let PilotPattern::Positions(p) = &mut o.pilots {
            *p = parse_list(&texts.pilots).map_err(|e| field("gen-ofdm-pilots", e))?;
        }
        Ok(())
    }

    /// Show the spec of `kind`: the one edited last, or its defaults.
    pub fn switch_kind(&mut self, kind: SignalKind) {
        if self.spec.kind() == kind {
            return;
        }
        let next = self
            .stash
            .remove(&kind)
            .unwrap_or_else(|| GeneratorSpec::default_for(kind));
        let old = std::mem::replace(&mut self.spec, next);
        self.stash.insert(old.kind(), old);
    }

    /// Replace the spec as a whole (a preset, the defaults).
    pub fn set_spec(&mut self, spec: GeneratorSpec) {
        let kind = spec.kind();
        self.stash.remove(&kind);
        let old = std::mem::replace(&mut self.spec, spec);
        if old.kind() != kind {
            self.stash.insert(old.kind(), old);
        }
    }

    /// Copy calculator lengths into the OFDM editor.
    pub fn use_lengths(&mut self, u: UseLengths) {
        self.switch_kind(SignalKind::Ofdm);
        if let GeneratorSpec::Ofdm(o) = &mut self.spec {
            o.lu = u.lu;
            o.lg = u.lg;
            if let Some(fd) = u.fd {
                o.common.sample_rate = fd;
            }
        }
    }

    /// Copy a K fraction into the OFDM editor: the smallest LU that is a
    /// multiple of the denominator and at least the current LU, and the
    /// matching LG. A fraction that would not fit leaves the editor as it was
    /// and says why.
    pub fn use_fraction(&mut self, f: UseFraction) {
        self.switch_kind(SignalKind::Ofdm);
        self.fraction_error = None;
        if let GeneratorSpec::Ofdm(o) = &mut self.spec {
            if let Some((lu, lg)) = fraction_lengths(o.lu, f) {
                (o.lu, o.lg) = (lu, lg);
            } else {
                let why = t!(
                    "gen-calc-fraction-too-big",
                    k = format!("{}/{}", f.p, f.q),
                    lu = sn_synth::OFDM_MAX_LU.to_string(),
                    lg = sn_synth::OFDM_MAX_LG.to_string()
                );
                self.fraction_error = Some((why, (o.lu, o.lg)));
            }
        }
    }

    /// The refused K fraction's message while the OFDM spec still has the LU
    /// and LG it was refused for.
    pub fn fraction_error(&self) -> Option<&str> {
        let (why, at) = self.fraction_error.as_ref()?;
        match &self.spec {
            GeneratorSpec::Ofdm(o) if (o.lu, o.lg) == *at => Some(why.as_str()),
            _ => None,
        }
    }

    /// A preview job is running.
    pub fn preview_running(&self) -> bool {
        self.preview_job.is_some()
    }

    /// A preview is being made or waits for the debounce.
    pub fn preview_busy(&self) -> bool {
        self.preview_job.is_some() || self.preview_wanted.is_some()
    }

    /// Stop the preview work (the window closed).
    pub fn stop_preview(&mut self) {
        if let Some((_, h)) = self.preview_job.take() {
            h.cancel();
        }
        self.preview_wanted = None;
    }

    /// Collect a finished preview and start a new one once the spec has
    /// settled. `valid` is the plan of a spec that validates.
    pub fn poll_preview(&mut self, valid: Option<&Plan>) {
        let done = self
            .preview_job
            .as_ref()
            .is_some_and(|(_, h)| h.is_finished());
        if done && let Some((spec, h)) = self.preview_job.take() {
            match h.join() {
                Ok(p) => self.preview = Some((spec, Ok(Arc::new(p)))),
                Err(JobError::Cancelled) => {}
                Err(e) => self.preview = Some((spec, Err(NoPreview::Failed(e.to_string())))),
            }
        }
        let Some(plan) = valid else {
            self.preview_wanted = None;
            return;
        };
        let current = self.preview.as_ref().is_some_and(|(s, _)| *s == self.spec);
        let running = self
            .preview_job
            .as_ref()
            .is_some_and(|(s, _)| *s == self.spec);
        if current || running {
            self.preview_wanted = None;
            return;
        }
        let now = Instant::now();
        let since = match &self.preview_wanted {
            Some((s, t)) if *s == self.spec => *t,
            _ => {
                self.preview_wanted = Some((self.spec.clone(), now));
                now
            }
        };
        let first = self.preview.is_none() && self.preview_job.is_none();
        if first || now.duration_since(since) >= PREVIEW_DEBOUNCE {
            self.preview_wanted = None;
            if let Some((_, h)) = self.preview_job.take() {
                h.cancel();
            }
            match preview_spec(&self.spec, plan) {
                Ok(short) => {
                    let handle = job::spawn(t!("gen-preview"), move |ctx| {
                        let stop = || ctx.is_cancelled();
                        build_preview(&short, &stop)
                    });
                    self.preview_job = Some((self.spec.clone(), handle));
                }
                Err(e) => self.preview = Some((self.spec.clone(), Err(NoPreview::TooLong(e)))),
            }
        }
    }
}

/// The complex flag of the spec's kind (C4FM is always real).
pub fn set_complex(spec: &mut GeneratorSpec, on: bool) {
    match spec {
        GeneratorSpec::Tones(s) => s.complex = on,
        GeneratorSpec::Noise(s) => s.complex = on,
        GeneratorSpec::Fsk(s) => s.complex = on,
        GeneratorSpec::Linear(s) => s.complex = on,
        GeneratorSpec::C4fm(_) => {}
        GeneratorSpec::Ofdm(s) => s.complex = on,
    }
}

/// The noise choice: 0 none, 1 SNR over the full band, 2 SNR in band, 3 Es/N0.
pub fn noise_kind(noise: Option<&NoiseLevel>) -> usize {
    match noise {
        None => 0,
        Some(NoiseLevel::SnrFullBand { .. }) => 1,
        Some(NoiseLevel::Snr { .. }) => 2,
        Some(NoiseLevel::EsN0 { .. }) => 3,
    }
}

/// Choose noise kind `i` (as [`noise_kind`]), keeping the level in dB (20 dB
/// when there was no noise).
pub fn pick_noise(noise: &mut Option<NoiseLevel>, i: usize) {
    let db = match noise {
        Some(
            NoiseLevel::SnrFullBand { db }
            | NoiseLevel::Snr { db, .. }
            | NoiseLevel::EsN0 { db, .. },
        ) => *db,
        None => 20.0,
    };
    *noise = match i {
        1 => Some(NoiseLevel::SnrFullBand { db }),
        2 => Some(NoiseLevel::Snr {
            db,
            bandwidth: None,
        }),
        3 => Some(NoiseLevel::EsN0 {
            db,
            symbol_rate: None,
        }),
        _ => None,
    };
}

#[cfg(test)]
mod tests {
    use sn_synth::{PRESET_NAMES, Shaping};

    use super::*;

    #[test]
    fn k_list_finds_the_simple_k_of_slightly_off_measured_rates() {
        // K = 7/8 from Br 100 Bd and spacing 187.5 Hz, Br measured 0.1% high: the
        // plain distance ranking misses 7/8, the uncertainty-based list puts it first.
        let br = 100.1;
        let spacing = 187.5;
        let plain = calc::k_list_from_rates(br, spacing, 256, 5).unwrap().best;
        assert!(plain.iter().all(|c| c.fraction.q != 8));
        let list = k_candidates(true, br, spacing, 0.0, 0.1, 256, 5).unwrap();
        assert_eq!((list[0].fraction.p, list[0].fraction.q), (7, 8));
        // K entered directly, 0.1% off.
        let list = k_candidates(false, 0.0, 0.0, 0.875 * 1.001, 0.1, 256, 5).unwrap();
        assert_eq!((list[0].fraction.p, list[0].fraction.q), (7, 8));
        // No uncertainty: only an exact fraction qualifies.
        let exact = k_candidates(false, 0.0, 0.0, 0.875 * 1.001, 0.0, 256, 5).unwrap();
        assert_eq!(exact.len(), 0, "{exact:?}");
        assert!(k_candidates(true, br, spacing, 0.0, f64::NAN, 256, 5).is_err());
        assert!(k_candidates(true, br, spacing, 0.0, 50.0, 256, 5).is_err());
    }

    #[test]
    fn lists_parse_and_print() {
        assert_eq!(parse_list("3, 7, 10-12").unwrap(), vec![3, 7, 10, 11, 12]);
        assert_eq!(parse_list(" ").unwrap(), Vec::<usize>::new());
        assert_eq!(parse_list("1..3;5").unwrap(), vec![1, 2, 3, 5]);
        assert!(parse_list("3, x").is_err());
        assert!(parse_list("5-2").is_err());
        assert!(parse_list("0-99999999").is_err());
        assert_eq!(format_list(&[3, 7, 10, 11, 12]), "3, 7, 10-12");
        assert_eq!(format_list(&[1, 2, 5]), "1, 2, 5");
        assert_eq!(format_list(&[]), "");
        for text in ["3, 7, 10-12", "0-5, 9", "4"] {
            assert_eq!(format_list(&parse_list(text).unwrap()), text);
        }
    }

    #[test]
    fn overrides_accept_display_and_json_names() {
        let o = parse_overrides("0: pilot, 5-6: QAM-16; 9=pi4_dqpsk, 2: 8-PSK").unwrap();
        assert_eq!(
            o,
            vec![
                (0, OfdmModulation::Pilot),
                (5, OfdmModulation::Qam16),
                (6, OfdmModulation::Qam16),
                (9, OfdmModulation::Pi4Dqpsk),
                (2, OfdmModulation::Psk8),
            ]
        );
        assert_eq!(parse_overrides(&format_overrides(&o)).unwrap(), o);
        assert!(parse_overrides("3").is_err());
        assert!(parse_overrides("3: qam1024").is_err());
    }

    #[test]
    fn numbers_and_fractions_read_well() {
        assert_eq!(trim(83.333_333, 4), "83.3333");
        assert_eq!(trim(1200.0, 3), "1200");
        assert_eq!(trim(-0.000_01, 3), "0");
        assert_eq!(fraction(20, 64), "5/16");
        assert_eq!(fraction(0, 64), "0/1");
        assert_eq!(whole_lengths(64.0, 20.0), Some((64, 20)));
        assert_eq!(whole_lengths(64.5, 20.0), None);
        assert_eq!(whole_lengths(1.0, 0.0), None);
        // Each refusal says why.
        let why = |lu: f64, lg: f64| check_lengths(lu, lg).unwrap_err();
        assert_eq!(why(64.5, 20.0), t!("gen-calc-lu-not-whole"));
        assert_eq!(why(1.0, 0.0), t!("gen-calc-lu-not-whole"));
        assert_eq!(why(64.0, 0.5), t!("gen-calc-lg-not-whole"));
        assert!(why(1e6, 0.0).contains("65"), "{}", why(1e6, 0.0));
        assert_ne!(why(64.0, 1e6), why(1e6, 0.0));
        let s = new_seed(5);
        assert_ne!(s, 5);
        assert!(s < 1 << 31);
    }

    #[test]
    fn previews_are_short_and_drawable() {
        let mut specs: Vec<GeneratorSpec> = SignalKind::ALL
            .into_iter()
            .map(GeneratorSpec::default_for)
            .collect();
        specs.extend(PRESET_NAMES.iter().filter_map(|n| GeneratorSpec::preset(n)));
        let mut iq = GeneratorSpec::default_for(SignalKind::Noise);
        if let GeneratorSpec::Noise(n) = &mut iq {
            n.complex = true;
        }
        specs.push(iq);
        for spec in specs {
            let plan = spec.plan().unwrap();
            let short = preview_spec(&spec, &plan).unwrap();
            let short_plan = short.plan().unwrap();
            // The pulse tail counts against the budget too.
            assert!(short_plan.samples <= PREVIEW_MAX_SAMPLES + 1, "{spec:?}");
            let p = build_preview(&short, &|| false).unwrap();
            assert!(p.spectrum.len() > 16, "{spec:?}");
            assert_ne!(p.wave_i.len(), 0);
            assert_eq!(p.wave_q.is_some(), plan.complex);
            assert!(p.hi_hz > p.lo_hz);
        }
    }

    #[test]
    fn a_long_pulse_tail_counts_against_the_preview() {
        // A small roll-off RRC: one symbol fits a preview, its 240-symbol tail
        // does not.
        let mut spec = GeneratorSpec::default_for(SignalKind::Linear);
        if let GeneratorSpec::Linear(l) = &mut spec {
            l.shaping = Shaping::RootRaisedCosine { alpha: 0.01 };
            l.common.sample_rate = 48_000.0;
            l.baud = 48_000.0 / 2000.0;
            l.carrier = 0.0;
            l.complex = true;
            l.common.length = Length::Symbols(100);
        }
        let plan = spec.plan().unwrap();
        assert!(preview_spec(&spec, &plan).is_err());
        // A shorter symbol: the preview holds its tail and fewer symbols.
        if let GeneratorSpec::Linear(l) = &mut spec {
            l.baud = 48_000.0 / 200.0;
        }
        let plan = spec.plan().unwrap();
        let short = preview_spec(&spec, &plan).unwrap();
        assert!(short.plan().unwrap().samples <= PREVIEW_MAX_SAMPLES + 1);
    }

    #[test]
    fn calculator_results_reach_the_ofdm_editor() {
        let mut g = GenState::new(GeneratorSpec::default_for(SignalKind::Linear));
        g.use_lengths(UseLengths {
            fd: Some(7000.0),
            lu: 64,
            lg: 20,
        });
        let GeneratorSpec::Ofdm(o) = &g.spec else {
            panic!("not OFDM")
        };
        assert_eq!((o.lu, o.lg, o.common.sample_rate), (64, 20, 7000.0));
        g.use_fraction(UseFraction { p: 2, q: 3 });
        let GeneratorSpec::Ofdm(o) = &g.spec else {
            panic!("not OFDM")
        };
        assert_eq!((o.lu, o.lg), (66, 44));
        if let GeneratorSpec::Ofdm(o) = &mut g.spec {
            o.lu = 4096;
        }
        g.use_fraction(UseFraction { p: 16, q: 1 });
        assert!(g.fraction_error().is_some());
        if let GeneratorSpec::Ofdm(o) = &mut g.spec {
            o.lu = 2048;
        }
        assert!(g.fraction_error().is_none());
        // Switching away and back keeps the edits.
        g.switch_kind(SignalKind::Fsk);
        g.switch_kind(SignalKind::Ofdm);
        assert!(matches!(&g.spec, GeneratorSpec::Ofdm(o) if o.lu == 2048));
    }

    #[test]
    fn list_fields_follow_the_spec_and_errors_block() {
        let mut g = GenState::new(GeneratorSpec::default_for(SignalKind::Ofdm));
        let texts = ListTexts {
            unused: "3, 7".into(),
            overrides: "0: pilot".into(),
            pilots: String::new(),
        };
        assert!(g.apply_texts(&texts).is_ok());
        let GeneratorSpec::Ofdm(o) = &g.spec else {
            panic!("not OFDM")
        };
        assert_eq!(o.unused, vec![3, 7]);
        assert_eq!(o.overrides, vec![(0, OfdmModulation::Pilot)]);
        assert_eq!(g.list_texts().unused, "3, 7");
        let bad = ListTexts {
            unused: "3, seven".into(),
            ..texts
        };
        let e = g.apply_texts(&bad).unwrap_err();
        assert!(e.contains("seven"), "{e}");
    }

    #[test]
    fn a_symbol_longer_than_the_preview_gets_a_note() {
        let mut spec = GeneratorSpec::default_for(SignalKind::Linear);
        if let GeneratorSpec::Linear(l) = &mut spec {
            l.baud = 1.0;
        }
        spec.common_mut().sample_rate = 1_000_000.0;
        spec.common_mut().length = Length::Symbols(50);
        let plan = spec.plan().unwrap();
        let mut g = GenState::new(spec);
        g.poll_preview(Some(&plan));
        assert!(matches!(&g.preview, Some((_, Err(NoPreview::TooLong(_))))));
        assert!(!g.preview_busy());
    }

    #[test]
    fn noise_choices_keep_the_level() {
        let mut n = None;
        pick_noise(&mut n, 2);
        assert_eq!(noise_kind(n.as_ref()), 2);
        if let Some(NoiseLevel::Snr { db, .. }) = &mut n {
            *db = 7.0;
        }
        pick_noise(&mut n, 3);
        assert!(matches!(n, Some(NoiseLevel::EsN0 { db, .. }) if (db - 7.0).abs() < 1e-12));
        pick_noise(&mut n, 0);
        assert_eq!(n, None);
    }
}
