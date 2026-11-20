//! Channel impairments, applied in a fixed order, each optional:
//!
//! 1. frequency offset (Hz) and linear frequency drift (Hz/s): the instantaneous shift
//!    is `frequency_offset + drift * t`, with `t = 0` at the first sample. The shift
//!    must stay below Nyquist in size over the whole signal;
//! 2. static multipath: `y(t) = sum_i gain_i exp(j phase_i) x(t - delay_i)`. The list
//!    is the whole channel, so include the direct path (`delay 0, gain 1`) yourself.
//!    Fractional delays use windowed-sinc interpolation; samples before the start count
//!    as 0, and the output keeps the input length;
//! 3. sample-clock offset in ppm: the receiver's clock runs `clock_ppm` parts per
//!    million fast, so it takes `1 + ppm / 1e6` times as many samples of the same
//!    signal (which then looks that much lower in frequency and slower) while the
//!    labelled sample rate stays the same. Windowed-sinc resampling, no anti-alias
//!    filter (the limit of 5% keeps aliasing to the top 5% of the band);
//! 4. additive white Gaussian noise ([`NoiseLevel`]), with the same noise sequence as
//!    [`crate::add_awgn`];
//! 5. amplitude clipping: real samples are clamped to `+-clip`, complex samples keep
//!    their phase and have their magnitude limited to `clip`.
//!
//! Accuracy limits:
//! - Real signals go through an analytic (Hilbert) filter for steps 1 and 2, so a
//!   frequency shift and a tap phase act on the positive frequencies as on a real
//!   receiver; the real part is kept. The 255-tap filter (half-length 128 samples) is
//!   accurate from about 1% to 49% of the sample rate; content nearer 0 Hz or Nyquist
//!   is shifted less cleanly. Samples outside the record count as 0, so in the first
//!   and last 128 samples the filter is one-sided and the image is not suppressed.
//! - Fractional delays and the clock offset interpolate with a Kaiser-windowed sinc
//!   (64 taps, beta 8). Its error is below -75 dB up to 46% of the sample rate and
//!   grows quickly above that (about -17 dB at 48%). It reads zeros outside the record,
//!   so the last 32 samples of a delayed or resampled signal are tapered.
//!
//! Everything is serial and deterministic: the noise comes from the seed only.

use std::f64::consts::{PI, TAU};
use std::sync::OnceLock;

use rustfft::num_complex::Complex64;
use serde::{Deserialize, Serialize};
use sn_core::{Complex32, SampleKind, Signal};

use crate::noise::{awgn_complex_in_place, awgn_real_in_place, read_all};
use crate::{MAX_SAMPLES, Result, StopCheck, check_stop, never_stop, output_len, require};

/// Most multipath taps.
pub const MAX_TAPS: usize = 64;
/// Longest multipath delay in seconds.
pub const MAX_DELAY: f64 = 10.0;
/// Largest sample-clock offset in ppm (5%).
pub const MAX_CLOCK_PPM: f64 = 50_000.0;
/// Half-width of the interpolation kernel in samples.
const INTERP_HALF: i64 = 32;
/// Number of taps of the interpolation kernel.
const INTERP_TAPS: usize = 2 * INTERP_HALF as usize;
/// Kaiser window parameter of the interpolation kernel.
const INTERP_BETA: f64 = 8.0;
/// Points per sample of the tabulated interpolation kernel (clock offset).
const KERNEL_STEPS: usize = 1024;
/// Half-length of the Hilbert filter in samples.
const HILBERT_HALF: usize = 128;
/// What needs the samples in memory, for the error message of `read_all`.
const PURPOSE: &str = "the channel model";

/// One multipath tap.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tap {
    /// Delay in seconds, 0 to [`MAX_DELAY`].
    pub delay: f64,
    /// Linear gain.
    pub gain: f64,
    /// Phase in radians.
    pub phase: f64,
}

/// How much noise to add.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum NoiseLevel {
    /// SNR over the whole Nyquist band (as [`crate::add_awgn_snr`]).
    SnrFullBand {
        /// SNR in dB.
        db: f64,
    },
    /// SNR inside the occupied bandwidth: the noise power falling in `bandwidth` Hz is
    /// the signal power over the SNR. `None` takes the generator's occupied bandwidth
    /// (from [`ChannelHints`]).
    Snr {
        /// SNR in dB.
        db: f64,
        /// Occupied bandwidth in Hz.
        #[serde(default)]
        bandwidth: Option<f64>,
    },
    /// Es/N0 (as [`crate::add_awgn_esn0`]). `None` takes the generator's symbol rate.
    EsN0 {
        /// Es/N0 in dB.
        db: f64,
        /// Symbol rate in Bd.
        #[serde(default)]
        symbol_rate: Option<f64>,
    },
}

impl NoiseLevel {
    fn db(self) -> f64 {
        match self {
            Self::SnrFullBand { db } | Self::Snr { db, .. } | Self::EsN0 { db, .. } => db,
        }
    }
}

/// Channel impairments. The default changes nothing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChannelParams {
    /// Frequency offset in Hz.
    pub frequency_offset: f64,
    /// Linear frequency drift in Hz per second.
    pub drift: f64,
    /// Multipath taps; empty means no multipath.
    pub multipath: Vec<Tap>,
    /// Sample-clock offset in ppm (positive: the receiver clock runs fast).
    pub clock_ppm: f64,
    /// Additive white Gaussian noise.
    pub noise: Option<NoiseLevel>,
    /// Clipping level (peak amplitude).
    pub clip: Option<f64>,
}

/// Facts about the clean signal that [`NoiseLevel`] can take its defaults from.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ChannelHints {
    /// Occupied bandwidth in Hz.
    pub bandwidth: Option<f64>,
    /// Symbol rate in Bd.
    pub symbol_rate: Option<f64>,
    /// Start and end in seconds (on the clean signal's time axis) of the part where the
    /// signal is at its steady level, for example without the pulse ramps of a shaped
    /// linear modulation. The signal power for a noise level is measured there. `None`
    /// measures it over every sample.
    pub steady: Option<(f64, f64)>,
}

/// What a noise level is measured against, once the hints are filled in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum NoiseReference {
    /// The whole Nyquist band.
    FullBand,
    /// An occupied bandwidth in Hz.
    Bandwidth(f64),
    /// A symbol rate in Bd.
    SymbolRate(f64),
}

/// Width of the Nyquist band in Hz: `fs` for complex signals, `fs / 2` for real ones.
fn nyquist_band(sample_rate: f64, complex: bool) -> f64 {
    if complex {
        sample_rate
    } else {
        sample_rate / 2.0
    }
}

/// Fill in what `level` leaves to the hints and check it against the `band` Hz the
/// signal can occupy. Shared by [`ChannelParams::check_hints`] and the noise step, so
/// the two cannot disagree.
fn resolve_noise(level: NoiseLevel, hints: &ChannelHints, band: f64) -> Result<NoiseReference> {
    match level {
        NoiseLevel::SnrFullBand { .. } => Ok(NoiseReference::FullBand),
        NoiseLevel::Snr { bandwidth, .. } => {
            let b =
                bandwidth
                    .or(hints.bandwidth)
                    .ok_or_else(|| crate::Error::InvalidParameter {
                        name: "noise",
                        reason: "give the occupied bandwidth for the SNR".to_owned(),
                    })?;
            require(
                b.is_finite() && b > 0.0 && b <= band,
                "noise",
                format!(
                    "the occupied bandwidth {b} Hz must be above 0 and at most the {band} Hz \
                     the signal can occupy"
                ),
            )?;
            Ok(NoiseReference::Bandwidth(b))
        }
        NoiseLevel::EsN0 { symbol_rate, .. } => {
            let rs = symbol_rate.or(hints.symbol_rate).ok_or_else(|| {
                crate::Error::InvalidParameter {
                    name: "noise",
                    reason: "give the symbol rate for Es/N0".to_owned(),
                }
            })?;
            crate::require_positive(rs, "noise", "Bd")?;
            Ok(NoiseReference::SymbolRate(rs))
        }
    }
}

/// Number of samples a clock offset of `ppm` turns `n` samples into:
/// `floor((n - 1) (1 + ppm / 1e6)) + 1`, or 0 for an empty signal.
pub(crate) fn clocked_len(n: usize, ppm: f64) -> Result<usize> {
    if n == 0 || ppm == 0.0 {
        return Ok(n);
    }
    let ratio = 1.0 + ppm * 1e-6;
    output_len(((n - 1) as f64 * ratio).floor() + 1.0, "clock_ppm")
}

impl ChannelParams {
    /// True when applying the channel would change nothing.
    pub fn is_identity(&self) -> bool {
        self.frequency_offset == 0.0
            && self.drift == 0.0
            && self.multipath.is_empty()
            && self.clock_ppm == 0.0
            && self.noise.is_none()
            && self.clip.is_none()
    }

    /// Check every field for a signal at `sample_rate`.
    pub fn validate(&self, sample_rate: f64) -> Result<()> {
        crate::check_sample_rate(sample_rate)?;
        let nyquist = sample_rate / 2.0;
        require(
            self.frequency_offset.is_finite() && self.frequency_offset.abs() < nyquist,
            "frequency_offset",
            format!(
                "must be a finite number of Hz below Nyquist ({nyquist} Hz) in size (got {})",
                self.frequency_offset
            ),
        )?;
        require(
            self.drift.is_finite() && self.drift.abs() <= sample_rate,
            "drift",
            format!(
                "must be a finite number of Hz/s, at most the sample rate in size (got {})",
                self.drift
            ),
        )?;
        require(
            self.multipath.len() <= MAX_TAPS,
            "multipath",
            format!("at most {MAX_TAPS} taps (got {})", self.multipath.len()),
        )?;
        for t in &self.multipath {
            require(
                t.delay.is_finite() && (0.0..=MAX_DELAY).contains(&t.delay),
                "multipath",
                format!("a delay must be 0 to {MAX_DELAY} s (got {})", t.delay),
            )?;
            require(
                t.delay * sample_rate <= MAX_SAMPLES as f64,
                "multipath",
                "a delay is longer than the largest signal",
            )?;
            require(
                t.gain.is_finite() && t.phase.is_finite(),
                "multipath",
                "tap gains and phases must be finite",
            )?;
        }
        require(
            self.clock_ppm.is_finite() && self.clock_ppm.abs() <= MAX_CLOCK_PPM,
            "clock_ppm",
            format!(
                "must be between -{MAX_CLOCK_PPM} and {MAX_CLOCK_PPM} ppm (got {})",
                self.clock_ppm
            ),
        )?;
        if let Some(n) = self.noise {
            let db = n.db();
            require(
                db.is_finite() && (-100.0..=200.0).contains(&db),
                "noise",
                format!("the level must be between -100 and 200 dB (got {db})"),
            )?;
            match n {
                NoiseLevel::Snr {
                    bandwidth: Some(b), ..
                } => require(
                    b.is_finite() && b > 0.0,
                    "noise",
                    format!("the bandwidth must be a finite number above 0 Hz (got {b})"),
                )?,
                NoiseLevel::EsN0 {
                    symbol_rate: Some(r),
                    ..
                } => crate::require_positive(r, "noise", "Bd")?,
                _ => {}
            }
        }
        if let Some(c) = self.clip {
            crate::require_positive(c, "clip", "(peak amplitude)")?;
        }
        Ok(())
    }

    /// Check what the noise level needs from `hints` (the bandwidth for an in-band SNR,
    /// the symbol rate for Es/N0) for a real or `complex` signal at `sample_rate`. The
    /// same check runs when the channel is applied; this one needs no samples.
    pub fn check_hints(&self, hints: &ChannelHints, sample_rate: f64, complex: bool) -> Result<()> {
        if let Some(level) = self.noise {
            resolve_noise(level, hints, nyquist_band(sample_rate, complex))?;
        }
        Ok(())
    }

    /// Check that the frequency shift stays below Nyquist in size over a signal of
    /// `len` samples at `sample_rate`.
    pub fn check_shift(&self, len: usize, sample_rate: f64) -> Result<()> {
        let (lo, hi) = self.shift_range(len, sample_rate);
        let nyquist = sample_rate / 2.0;
        require(
            lo.abs() < nyquist && hi.abs() < nyquist,
            "drift",
            format!(
                "the frequency shift would run from {:.1} to {:.1} Hz over the signal, \
                 beyond Nyquist ({nyquist} Hz); lower the drift or shorten the signal",
                self.frequency_offset,
                self.frequency_offset + self.drift * len.saturating_sub(1) as f64 / sample_rate
            ),
        )
    }

    /// Smallest and largest frequency shift in Hz over a signal of `len` samples.
    pub(crate) fn shift_range(&self, len: usize, sample_rate: f64) -> (f64, f64) {
        let end = self.frequency_offset + self.drift * len.saturating_sub(1) as f64 / sample_rate;
        (
            self.frequency_offset.min(end),
            self.frequency_offset.max(end),
        )
    }

    /// Short English description of each impairment, in the order applied.
    pub fn describe(&self) -> Vec<String> {
        use crate::spec::fmt_num;
        let mut parts = Vec::new();
        if self.frequency_offset != 0.0 {
            parts.push(format!("offset {} Hz", fmt_num(self.frequency_offset)));
        }
        if self.drift != 0.0 {
            parts.push(format!("drift {} Hz/s", fmt_num(self.drift)));
        }
        if !self.multipath.is_empty() {
            let n = self.multipath.len();
            parts.push(format!(
                "multipath {n} tap{}",
                if n == 1 { "" } else { "s" }
            ));
        }
        if self.clock_ppm != 0.0 {
            parts.push(format!("clock {} ppm", fmt_num(self.clock_ppm)));
        }
        match self.noise {
            Some(NoiseLevel::SnrFullBand { db }) => {
                parts.push(format!("SNR {} dB (full band)", fmt_num(db)));
            }
            Some(NoiseLevel::Snr { db, .. }) => parts.push(format!("SNR {} dB", fmt_num(db))),
            Some(NoiseLevel::EsN0 { db, .. }) => parts.push(format!("Es/N0 {} dB", fmt_num(db))),
            None => {}
        }
        if let Some(c) = self.clip {
            parts.push(format!("clipped at {}", fmt_num(c)));
        }
        parts
    }
}

fn blackman(x: f64) -> f64 {
    // x in -1..=1
    0.42 + 0.5 * (PI * x).cos() + 0.08 * (TAU * x).cos()
}

/// Analytic signal of `x` (`x + j H{x}`) through a windowed FIR Hilbert transformer.
pub(crate) fn analytic(x: &[f32], stop: StopCheck<'_>) -> Result<Vec<Complex32>> {
    let half = HILBERT_HALF as f64;
    // Odd taps only: h[m] = 2 / (pi m) for odd m.
    let taps: Vec<(usize, f64)> = (1..=HILBERT_HALF)
        .step_by(2)
        .map(|m| (m, 2.0 / (PI * m as f64) * blackman(m as f64 / (half + 1.0))))
        .collect();
    let at = |i: usize, m: usize, ahead: bool| -> f64 {
        let j = if ahead {
            i.checked_add(m)
        } else {
            i.checked_sub(m)
        };
        j.and_then(|j| x.get(j)).map_or(0.0, |v| f64::from(*v))
    };
    let mut out = Vec::with_capacity(x.len());
    for (i, v) in x.iter().enumerate() {
        check_stop(stop, i)?;
        let h: f64 = taps
            .iter()
            .map(|(m, h)| h * (at(i, *m, false) - at(i, *m, true)))
            .sum();
        out.push(Complex32::new(*v, h as f32));
    }
    Ok(out)
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// Modified Bessel function of the first kind, order 0 (power series).
fn bessel_i0(x: f64) -> f64 {
    let q = (x / 2.0).powi(2);
    let mut term = 1.0;
    let mut sum = 1.0;
    for k in 1..100 {
        term *= q / f64::from(k * k);
        sum += term;
        if term < 1e-17 * sum {
            break;
        }
    }
    sum
}

/// The interpolation kernel at `d` samples from its centre: a sinc under a Kaiser
/// window of half-width [`INTERP_HALF`]; 0 outside the window.
fn kernel(d: f64) -> f64 {
    let x = d / INTERP_HALF as f64;
    if x.abs() > 1.0 {
        return 0.0;
    }
    sinc(d) * bessel_i0(INTERP_BETA * (1.0 - x * x).sqrt()) / bessel_i0(INTERP_BETA)
}

/// [`kernel`] at `k / KERNEL_STEPS` for `k = 0..=INTERP_HALF * KERNEL_STEPS + 1`, built
/// once. The kernel is even, so only `d >= 0` is stored.
fn kernel_table() -> &'static [f64] {
    static TABLE: OnceLock<Vec<f64>> = OnceLock::new();
    TABLE.get_or_init(|| {
        (0..=INTERP_HALF as usize * KERNEL_STEPS + 1)
            .map(|k| kernel(k as f64 / KERNEL_STEPS as f64))
            .collect()
    })
}

/// [`kernel`] by linear interpolation in [`kernel_table`]; error below 1e-6.
fn kernel_fast(table: &[f64], d: f64) -> f64 {
    let x = d.abs() * KERNEL_STEPS as f64;
    let k = x.floor();
    let frac = x - k;
    let k = k as usize;
    match (table.get(k), table.get(k + 1)) {
        (Some(a), Some(b)) => a + (b - a) * frac,
        _ => 0.0,
    }
}

fn c64(v: Complex32) -> Complex64 {
    Complex64::new(f64::from(v.re), f64::from(v.im))
}

fn c32(v: Complex64) -> Complex32 {
    Complex32::new(v.re as f32, v.im as f32)
}

/// Working buffer of the channel.
enum Buf {
    Real(Vec<f32>),
    Complex(Vec<Complex32>),
}

impl Buf {
    fn len(&self) -> usize {
        match self {
            Self::Real(x) => x.len(),
            Self::Complex(z) => z.len(),
        }
    }

    /// Mean power over `range` (clamped to the buffer; the whole buffer when empty).
    fn power(&self, range: std::ops::Range<usize>) -> f64 {
        let n = self.len();
        let (lo, hi) = (range.start.min(n), range.end.min(n));
        let (lo, hi) = if hi > lo { (lo, hi) } else { (0, n) };
        if hi == lo {
            return 0.0;
        }
        let sum: f64 = match self {
            Self::Real(x) => x
                .get(lo..hi)
                .unwrap_or_default()
                .iter()
                .map(|v| f64::from(*v).powi(2))
                .sum(),
            Self::Complex(z) => z
                .get(lo..hi)
                .unwrap_or_default()
                .iter()
                .map(|v| f64::from(v.norm_sqr()))
                .sum(),
        };
        sum / (hi - lo) as f64
    }
}

/// Apply `params` to `signal`. The noise comes from `seed`; `hints` fill in noise
/// levels left at `None`.
pub fn apply_channel(
    signal: &Signal,
    params: &ChannelParams,
    seed: u64,
    hints: &ChannelHints,
) -> Result<Signal> {
    apply_channel_with_stop(signal, params, seed, hints, &never_stop)
}

/// [`apply_channel`] that calls `stop` now and then and fails with
/// [`crate::Error::Cancelled`] when it returns true.
pub fn apply_channel_with_stop(
    signal: &Signal,
    params: &ChannelParams,
    seed: u64,
    hints: &ChannelHints,
    stop: StopCheck<'_>,
) -> Result<Signal> {
    let fs = signal.sample_rate();
    params.validate(fs)?;
    if params.is_identity() {
        return Ok(signal.clone());
    }
    let complex = signal.kind() == SampleKind::Complex;
    params.check_hints(hints, fs, complex)?;
    let rotate = params.frequency_offset != 0.0 || params.drift != 0.0;
    if rotate {
        params.check_shift(signal.len(), fs)?;
    }
    let mut buf = if complex {
        Buf::Complex(read_all(
            signal,
            Complex32::new(0.0, 0.0),
            Signal::read_complex,
            PURPOSE,
        )?)
    } else {
        Buf::Real(read_all(signal, 0.0_f32, Signal::read_real, PURPOSE)?)
    };
    if rotate || !params.multipath.is_empty() {
        // Take the samples out of `buf`, so a real input is dropped once it has been
        // turned into its analytic signal.
        let mut z = match std::mem::replace(&mut buf, Buf::Complex(Vec::new())) {
            Buf::Real(x) => analytic(&x, stop)?,
            Buf::Complex(z) => z,
        };
        if rotate {
            shift(&mut z, params.frequency_offset, params.drift, fs, stop)?;
        }
        if !params.multipath.is_empty() {
            z = multipath(&z, &params.multipath, fs, stop)?;
        }
        buf = if complex {
            Buf::Complex(z)
        } else {
            Buf::Real(z.iter().map(|v| v.re).collect())
        };
    }
    if params.clock_ppm != 0.0 {
        buf = clock(buf, params.clock_ppm, stop)?;
    }
    if let Some(level) = params.noise {
        let ratio = 1.0 + params.clock_ppm * 1e-6;
        let window = hints.steady.map_or(0..0, |(a, b)| {
            let lo = (a * fs * ratio).ceil().max(0.0);
            let hi = (b * fs * ratio).floor() + 1.0;
            // Saturating casts; out-of-range bounds are clamped by Buf::power.
            (lo as usize)..(hi.max(0.0) as usize)
        });
        let power = buf.power(window);
        let sigma = noise_sigma(power, level, hints, fs, complex)?;
        match &mut buf {
            Buf::Real(x) => awgn_real_in_place(x, sigma, seed)?,
            Buf::Complex(z) => awgn_complex_in_place(z, sigma, seed)?,
        }
    }
    if let Some(c) = params.clip {
        clip(&mut buf, c);
    }
    let out = match buf {
        Buf::Real(x) => Signal::real(x, fs)?,
        Buf::Complex(z) => Signal::complex(z, fs)?,
    };
    Ok(out.with_center_frequency(signal.center_frequency()))
}

fn shift(z: &mut [Complex32], offset: f64, drift: f64, fs: f64, stop: StopCheck<'_>) -> Result<()> {
    for (n, v) in z.iter_mut().enumerate() {
        check_stop(stop, n)?;
        let t = n as f64 / fs;
        let cycles = (offset * t + 0.5 * drift * t * t).rem_euclid(1.0);
        *v = c32(c64(*v) * Complex64::from_polar(1.0, TAU * cycles));
    }
    Ok(())
}

/// One multipath tap with its interpolation weights worked out once.
struct PreparedTap {
    gain: Complex64,
    /// Index offset of the first input sample read: output `i` reads from
    /// `i - lag` onwards.
    lag: i64,
    /// Weights of the input samples `i - lag + m` for `m = 0..`; a single weight 1 for a
    /// whole-sample delay.
    weights: Vec<f64>,
}

fn prepare_tap(t: &Tap, fs: f64) -> PreparedTap {
    let d = t.delay * fs;
    let whole = d.floor();
    let frac = d - whole;
    let whole = whole as i64;
    let gain = Complex64::from_polar(t.gain, t.phase);
    if frac == 0.0 {
        return PreparedTap {
            gain,
            lag: whole,
            weights: vec![1.0],
        };
    }
    // Output i sits at input position i - d = (i - whole - 1) + (1 - frac); the kernel
    // reads the samples from INTERP_HALF - 1 before that base to INTERP_HALF after it.
    let pos = 1.0 - frac;
    let weights = (0..INTERP_TAPS)
        .map(|m| kernel(pos - (m as f64 - (INTERP_HALF - 1) as f64)))
        .collect();
    PreparedTap {
        gain,
        lag: whole + 1 + (INTERP_HALF - 1),
        weights,
    }
}

fn multipath(
    z: &[Complex32],
    taps: &[Tap],
    fs: f64,
    stop: StopCheck<'_>,
) -> Result<Vec<Complex32>> {
    let prepared: Vec<PreparedTap> = taps.iter().map(|t| prepare_tap(t, fs)).collect();
    let mut out = Vec::with_capacity(z.len());
    for i in 0..z.len() {
        check_stop(stop, i)?;
        let mut acc = Complex64::new(0.0, 0.0);
        for t in &prepared {
            let start = i as i64 - t.lag;
            let mut sum = Complex64::new(0.0, 0.0);
            for (m, w) in t.weights.iter().enumerate() {
                let j = start + m as i64;
                if let Some(v) = usize::try_from(j).ok().and_then(|j| z.get(j)) {
                    sum += c64(*v) * w;
                }
            }
            acc += t.gain * sum;
        }
        out.push(c32(acc));
    }
    Ok(out)
}

/// Value at fractional position `t` of a sequence read by `get` (0 outside), with the
/// tabulated kernel. Exact at whole positions.
fn interpolate(t: f64, len: usize, table: &[f64], get: impl Fn(usize) -> Complex64) -> Complex64 {
    let base = t.floor();
    let frac = t - base;
    let base = base as i64;
    if frac == 0.0 {
        return usize::try_from(base)
            .ok()
            .filter(|i| *i < len)
            .map_or(Complex64::new(0.0, 0.0), &get);
    }
    let mut acc = Complex64::new(0.0, 0.0);
    for k in (base - INTERP_HALF + 1)..=(base + INTERP_HALF) {
        let Ok(i) = usize::try_from(k) else { continue };
        if i >= len {
            break;
        }
        acc += get(i) * kernel_fast(table, t - k as f64);
    }
    acc
}

fn clock(buf: Buf, ppm: f64, stop: StopCheck<'_>) -> Result<Buf> {
    let ratio = 1.0 + ppm * 1e-6;
    let n = buf.len();
    let m = clocked_len(n, ppm)?;
    let table = kernel_table();
    Ok(match buf {
        Buf::Real(x) => {
            let get = |i: usize| Complex64::new(x.get(i).map_or(0.0, |v| f64::from(*v)), 0.0);
            let mut out = Vec::with_capacity(m);
            for j in 0..m {
                check_stop(stop, j)?;
                out.push(interpolate(j as f64 / ratio, n, table, get).re as f32);
            }
            Buf::Real(out)
        }
        Buf::Complex(z) => {
            let get = |i: usize| z.get(i).copied().map_or(Complex64::new(0.0, 0.0), c64);
            let mut out = Vec::with_capacity(m);
            for j in 0..m {
                check_stop(stop, j)?;
                out.push(c32(interpolate(j as f64 / ratio, n, table, get)));
            }
            Buf::Complex(out)
        }
    })
}

/// Noise standard deviation (per component for complex signals) for `level`, for a
/// signal of mean power `power`.
fn noise_sigma(
    power: f64,
    level: NoiseLevel,
    hints: &ChannelHints,
    fs: f64,
    complex: bool,
) -> Result<f64> {
    require(
        power.is_finite() && power > 0.0,
        "noise",
        "the signal is silent or not finite, so a noise level cannot be set from it",
    )?;
    let band = nyquist_band(fs, complex);
    let db = level.db();
    let per_component = |total: f64| -> f64 {
        if complex {
            (total / 2.0).sqrt()
        } else {
            total.sqrt()
        }
    };
    let total_from_snr = |bandwidth: f64| power / 10_f64.powf(db / 10.0) * band / bandwidth;
    let sigma = match resolve_noise(level, hints, band)? {
        NoiseReference::FullBand => per_component(total_from_snr(band)),
        NoiseReference::Bandwidth(b) => per_component(total_from_snr(b)),
        NoiseReference::SymbolRate(rs) => {
            // Same arithmetic as add_awgn_esn0.
            let n0 = power / (rs * 10_f64.powf(db / 10.0));
            (n0 * fs / 2.0).sqrt()
        }
    };
    require(
        sigma.is_finite(),
        "noise",
        "the noise level is not a finite number for this signal; the occupied bandwidth \
         or symbol rate is too small, or the SNR too low",
    )?;
    Ok(sigma)
}

fn clip(buf: &mut Buf, level: f64) {
    match buf {
        Buf::Real(x) => {
            let l = level as f32;
            for v in x {
                *v = v.clamp(-l, l);
            }
        }
        Buf::Complex(z) => {
            for v in z {
                let m = f64::from(v.norm());
                if m > level {
                    *v = c32(c64(*v) * (level / m));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rustfft::FftPlanner;
    use sn_core::Samples;

    use super::*;
    use crate::{Rng, add_awgn_esn0, tone};

    fn complex_tone(f: f64, fs: f64, n: usize) -> Signal {
        let z: Vec<Complex32> = (0..n)
            .map(|i| c32(Complex64::from_polar(1.0, TAU * f * i as f64 / fs)))
            .collect();
        Signal::complex(z, fs).unwrap()
    }

    fn as_complex(s: &Signal) -> Vec<Complex64> {
        match s.samples() {
            Samples::Complex(z) => z.iter().map(|v| c64(*v)).collect(),
            Samples::Real(x) => x
                .iter()
                .map(|v| Complex64::new(f64::from(*v), 0.0))
                .collect(),
            Samples::Lazy(_) => panic!("lazy"),
        }
    }

    fn spectrum(x: &[Complex64]) -> Vec<f64> {
        let mut b = x.to_vec();
        FftPlanner::new().plan_fft_forward(b.len()).process(&mut b);
        b.iter().map(Complex64::norm_sqr).collect()
    }

    /// Frequency of the strongest bin, signed.
    fn peak_frequency(x: &[Complex64], fs: f64) -> f64 {
        let s = spectrum(x);
        let n = s.len();
        let k = (0..n).max_by(|a, b| s[*a].total_cmp(&s[*b])).unwrap();
        let k = if k > n / 2 {
            k as f64 - n as f64
        } else {
            k as f64
        };
        k * fs / n as f64
    }

    #[test]
    fn identity_changes_nothing() {
        let s = tone(1000.0, 0.5, 8000.0, 0.1).unwrap();
        let out =
            apply_channel(&s, &ChannelParams::default(), 1, &ChannelHints::default()).unwrap();
        assert_eq!(as_complex(&s), as_complex(&out));
    }

    #[test]
    fn offset_moves_a_real_and_a_complex_tone() {
        let s = tone(1000.0, 0.5, 8000.0, 1.0).unwrap();
        let p = ChannelParams {
            frequency_offset: 250.0,
            ..ChannelParams::default()
        };
        let out = apply_channel(&s, &p, 1, &ChannelHints::default()).unwrap();
        assert_eq!(out.kind(), SampleKind::Real);
        // Real spectrum: look at the positive half only.
        let x = as_complex(&out);
        let sp = spectrum(&x);
        let k = (1..4000).max_by(|a, b| sp[*a].total_cmp(&sp[*b])).unwrap();
        assert_eq!(k, 1250);
        // The image at 750 Hz is suppressed by the Hilbert filter.
        assert!(sp[750] < sp[1250] * 1e-4, "image {}", sp[750] / sp[1250]);

        let c = complex_tone(500.0, 8000.0, 8000);
        let p = ChannelParams {
            frequency_offset: -900.0,
            ..ChannelParams::default()
        };
        let out = apply_channel(&c, &p, 1, &ChannelHints::default()).unwrap();
        assert!((peak_frequency(&as_complex(&out), 8000.0) + 400.0).abs() < 1.5);
    }

    #[test]
    fn drift_sweeps_the_frequency() {
        let c = complex_tone(0.0, 8000.0, 16_000);
        let p = ChannelParams {
            drift: 100.0,
            ..ChannelParams::default()
        };
        let z = as_complex(&apply_channel(&c, &p, 1, &ChannelHints::default()).unwrap());
        // Instantaneous frequency at t = 1.5 s is 150 Hz.
        let i = 12_000;
        let f = (z[i + 1] * z[i].conj()).arg() / TAU * 8000.0;
        assert!((f - 150.0).abs() < 0.1, "{f}");
    }

    #[test]
    fn multipath_delay_shows_in_the_autocorrelation() {
        let mut rng = Rng::new(4);
        let z: Vec<Complex32> = (0..20_000)
            .map(|_| Complex32::new(rng.normal() as f32, rng.normal() as f32))
            .collect();
        let s = Signal::complex(z, 8000.0).unwrap();
        let p = ChannelParams {
            multipath: vec![
                Tap {
                    delay: 0.0,
                    gain: 1.0,
                    phase: 0.0,
                },
                Tap {
                    delay: 0.0025,
                    gain: 0.6,
                    phase: 1.0,
                },
            ],
            ..ChannelParams::default()
        };
        let y = as_complex(&apply_channel(&s, &p, 1, &ChannelHints::default()).unwrap());
        let acf = |lag: usize| -> f64 {
            y.iter()
                .zip(&y[lag..])
                .map(|(a, b)| b * a.conj())
                .sum::<Complex64>()
                .norm()
        };
        let best = (1..200).max_by(|a, b| acf(*a).total_cmp(&acf(*b))).unwrap();
        assert_eq!(best, 20);
        // Normalised side peak about 0.6 / (1 + 0.36).
        let r = acf(20) / acf(0);
        assert!((r - 0.6 / 1.36).abs() < 0.03, "{r}");
    }

    #[test]
    fn fractional_delay_rotates_a_tone() {
        let f = 700.0;
        let fs = 8000.0;
        let c = complex_tone(f, fs, 4000);
        let delay = 2.37 / fs;
        let p = ChannelParams {
            multipath: vec![Tap {
                delay,
                gain: 1.0,
                phase: 0.0,
            }],
            ..ChannelParams::default()
        };
        let x = as_complex(&c);
        let y = as_complex(&apply_channel(&c, &p, 1, &ChannelHints::default()).unwrap());
        let want = -TAU * f * delay;
        for i in 100..3900 {
            let d = (y[i] * x[i].conj()).arg();
            let err = (d - want + PI).rem_euclid(TAU) - PI;
            assert!(err.abs() < 1e-3, "{i}: {err}");
            assert!((y[i].norm() - 1.0).abs() < 2e-3);
        }
    }

    #[test]
    fn clock_offset_stretches_the_signal() {
        let c = complex_tone(1000.0, 8000.0, 8000);
        let p = ChannelParams {
            clock_ppm: 2000.0,
            ..ChannelParams::default()
        };
        let y = as_complex(&apply_channel(&c, &p, 1, &ChannelHints::default()).unwrap());
        assert_eq!(y.len(), 8015); // floor(7999 x 1.002) + 1
        let i = 4000;
        let f = (y[i + 1] * y[i].conj()).arg() / TAU * 8000.0;
        assert!((f - 1000.0 / 1.002).abs() < 0.05, "{f}");
        // A real signal resamples too.
        let r = tone(1000.0, 0.5, 8000.0, 1.0).unwrap();
        let y = apply_channel(&r, &p, 1, &ChannelHints::default()).unwrap();
        assert_eq!(y.kind(), SampleKind::Real);
        assert_eq!(y.len(), 8015); // floor(7999 x 1.002) + 1
    }

    #[test]
    fn snr_full_band_and_in_band() {
        let c = complex_tone(300.0, 8000.0, 80_000);
        let clean = as_complex(&c);
        // Full band.
        let p = ChannelParams {
            noise: Some(NoiseLevel::SnrFullBand { db: 10.0 }),
            ..ChannelParams::default()
        };
        let y = as_complex(&apply_channel(&c, &p, 7, &ChannelHints::default()).unwrap());
        let noise: f64 = y
            .iter()
            .zip(&clean)
            .map(|(a, b)| (a - b).norm_sqr())
            .sum::<f64>()
            / y.len() as f64;
        let snr = 10.0 * (1.0 / noise).log10();
        assert!((snr - 10.0).abs() < 0.5, "{snr}");
        // In band: 1000 Hz of 8000 Hz.
        let p = ChannelParams {
            noise: Some(NoiseLevel::Snr {
                db: 10.0,
                bandwidth: None,
            }),
            ..ChannelParams::default()
        };
        let hints = ChannelHints {
            bandwidth: Some(1000.0),
            ..ChannelHints::default()
        };
        let y = as_complex(&apply_channel(&c, &p, 7, &hints).unwrap());
        let diff: Vec<Complex64> = y.iter().zip(&clean).map(|(a, b)| a - b).collect();
        let sp = spectrum(&diff);
        let n = sp.len();
        let total: f64 = sp.iter().sum();
        // Bins within -500..500 Hz.
        let half = n / 16;
        let inband: f64 = sp
            .iter()
            .enumerate()
            .filter(|(k, _)| *k < half || *k >= n - half)
            .map(|(_, v)| v)
            .sum();
        let noise_in =
            inband / total * (diff.iter().map(Complex64::norm_sqr).sum::<f64>() / n as f64);
        let snr = 10.0 * (1.0 / noise_in).log10();
        assert!((snr - 10.0).abs() < 0.5, "in-band {snr}");
        // Missing bandwidth is an error.
        assert!(apply_channel(&c, &p, 7, &ChannelHints::default()).is_err());
    }

    #[test]
    fn esn0_matches_add_awgn_esn0() {
        let s = tone(1000.0, 0.5, 8000.0, 0.5).unwrap();
        let p = ChannelParams {
            noise: Some(NoiseLevel::EsN0 {
                db: 12.0,
                symbol_rate: Some(100.0),
            }),
            ..ChannelParams::default()
        };
        let a = apply_channel(&s, &p, 3, &ChannelHints::default()).unwrap();
        let b = add_awgn_esn0(&s, 12.0, 100.0, 3).unwrap();
        assert_eq!(as_complex(&a), as_complex(&b));
    }

    #[test]
    fn clipping_limits_the_peak() {
        let s = tone(1000.0, 1.0, 8000.0, 0.1).unwrap();
        let p = ChannelParams {
            clip: Some(0.5),
            ..ChannelParams::default()
        };
        let y = as_complex(&apply_channel(&s, &p, 1, &ChannelHints::default()).unwrap());
        assert!(y.iter().all(|v| v.re.abs() <= 0.5 + 1e-7));
        assert!(y.iter().any(|v| (v.re.abs() - 0.5).abs() < 1e-7));
        let c = complex_tone(100.0, 8000.0, 100);
        let y = as_complex(&apply_channel(&c, &p, 1, &ChannelHints::default()).unwrap());
        assert!(y.iter().all(|v| (v.norm() - 0.5).abs() < 1e-6));
    }

    #[test]
    fn validation() {
        let s = tone(1000.0, 1.0, 8000.0, 0.1).unwrap();
        let h = ChannelHints::default();
        let bad = [
            ChannelParams {
                frequency_offset: 4000.0,
                ..ChannelParams::default()
            },
            ChannelParams {
                drift: f64::NAN,
                ..ChannelParams::default()
            },
            ChannelParams {
                clock_ppm: 1e6,
                ..ChannelParams::default()
            },
            ChannelParams {
                clip: Some(0.0),
                ..ChannelParams::default()
            },
            ChannelParams {
                multipath: vec![
                    Tap {
                        delay: -1.0,
                        gain: 1.0,
                        phase: 0.0
                    };
                    1
                ],
                ..ChannelParams::default()
            },
            ChannelParams {
                multipath: vec![
                    Tap {
                        delay: 0.0,
                        gain: 1.0,
                        phase: 0.0
                    };
                    MAX_TAPS + 1
                ],
                ..ChannelParams::default()
            },
            ChannelParams {
                noise: Some(NoiseLevel::EsN0 {
                    db: 10.0,
                    symbol_rate: None,
                }),
                ..ChannelParams::default()
            },
        ];
        for p in bad {
            assert!(apply_channel(&s, &p, 1, &h).is_err(), "{p:?}");
        }
    }

    /// Phase of `x` relative to `exp(j 2 pi f n / fs)` over `range`.
    fn tone_phase(x: &[Complex64], f: f64, fs: f64, range: std::ops::Range<usize>) -> f64 {
        range
            .map(|n| x[n] * Complex64::from_polar(1.0, -TAU * f * n as f64 / fs))
            .sum::<Complex64>()
            .arg()
    }

    fn wrap(a: f64) -> f64 {
        (a + PI).rem_euclid(TAU) - PI
    }

    #[test]
    fn real_tone_through_a_phased_fractional_tap() {
        let (f, fs) = (1000.0, 8000.0);
        let s = tone(f, 0.5, fs, 1.0).unwrap();
        let x = as_complex(&s);
        let d = 2.37;
        let p = ChannelParams {
            multipath: vec![Tap {
                delay: d / fs,
                gain: 1.0,
                phase: PI / 2.0,
            }],
            ..ChannelParams::default()
        };
        let out = apply_channel(&s, &p, 1, &ChannelHints::default()).unwrap();
        assert_eq!(out.kind(), SampleKind::Real);
        let y = as_complex(&out);
        // Away from the 128-sample Hilbert edges: the tone advanced by 90 degrees and
        // delayed by d samples, at the same amplitude.
        let mid = 1000..7000;
        let got = tone_phase(&y, f, fs, mid.clone()) - tone_phase(&x, f, fs, mid.clone());
        let want = PI / 2.0 - TAU * f * d / fs;
        assert!(wrap(got - want).abs() < 1e-3, "{got} vs {want}");
        let rms =
            (y[mid.clone()].iter().map(|v| v.re * v.re).sum::<f64>() / mid.len() as f64).sqrt();
        assert!((rms - 0.5 / 2_f64.sqrt()).abs() < 1e-3, "{rms}");
    }

    #[test]
    fn negative_clock_offset_shortens_and_raises_the_tone() {
        let c = complex_tone(1000.0, 8000.0, 8000);
        let p = ChannelParams {
            clock_ppm: -2000.0,
            ..ChannelParams::default()
        };
        let y = as_complex(&apply_channel(&c, &p, 1, &ChannelHints::default()).unwrap());
        assert_eq!(y.len(), 7984); // floor(7999 x 0.998) + 1
        assert_eq!(clocked_len(8000, -2000.0).unwrap(), 7984);
        let i = 4000;
        let f = (y[i + 1] * y[i].conj()).arg() / TAU * 8000.0;
        assert!((f - 1000.0 / 0.998).abs() < 0.05, "{f}");
    }

    #[test]
    fn in_band_snr_on_a_real_tone() {
        // Real signal: the noise spreads over 0..4000 Hz, so a 1000 Hz occupied band
        // holds a quarter of it and the total is 4 P / SNR.
        let s = tone(1000.0, 0.5, 8000.0, 10.0).unwrap();
        let p = ChannelParams {
            noise: Some(NoiseLevel::Snr {
                db: 10.0,
                bandwidth: Some(1000.0),
            }),
            ..ChannelParams::default()
        };
        let y = as_complex(&apply_channel(&s, &p, 5, &ChannelHints::default()).unwrap());
        let x = as_complex(&s);
        let var = y
            .iter()
            .zip(&x)
            .map(|(a, b)| (a - b).norm_sqr())
            .sum::<f64>()
            / x.len() as f64;
        let want = 0.125 / 10.0 * 4.0;
        assert!((var / want - 1.0).abs() < 0.03, "{var} vs {want}");
    }

    #[test]
    fn interpolation_holds_up_near_the_band_edge() {
        let (f, fs) = (0.45 * 8000.0, 8000.0);
        let c = complex_tone(f, fs, 4000);
        let p = ChannelParams {
            multipath: vec![Tap {
                delay: 0.5 / fs,
                gain: 1.0,
                phase: 0.0,
            }],
            ..ChannelParams::default()
        };
        let x = as_complex(&c);
        let y = as_complex(&apply_channel(&c, &p, 1, &ChannelHints::default()).unwrap());
        let want = x[2000] * Complex64::from_polar(1.0, -TAU * f * 0.5 / fs);
        for i in 100..3900 {
            let w = x[i] * Complex64::from_polar(1.0, -TAU * f * 0.5 / fs);
            assert!((y[i] - w).norm() < 1e-3, "{i}: {}", (y[i] - w).norm());
        }
        assert!((want.norm() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn drift_past_nyquist_is_refused() {
        let s = tone(1000.0, 0.5, 8000.0, 10.0).unwrap();
        let p = ChannelParams {
            drift: 800.0,
            ..ChannelParams::default()
        };
        let e = apply_channel(&s, &p, 1, &ChannelHints::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("drift") && e.contains("Nyquist"), "{e}");
        // The same drift over 1 s is fine.
        let short = tone(1000.0, 0.5, 8000.0, 1.0).unwrap();
        assert!(apply_channel(&short, &p, 1, &ChannelHints::default()).is_ok());
    }

    #[test]
    fn tiny_bandwidth_is_a_noise_error_not_a_sigma_error() {
        let s = tone(1000.0, 0.5, 8000.0, 0.1).unwrap();
        let p = ChannelParams {
            noise: Some(NoiseLevel::Snr {
                db: -100.0,
                bandwidth: Some(1e-300),
            }),
            ..ChannelParams::default()
        };
        let e = apply_channel(&s, &p, 1, &ChannelHints::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("'noise'") && e.contains("not a finite"), "{e}");
    }

    #[test]
    fn oversized_lazy_input_names_the_channel_model() {
        use std::sync::Arc;

        #[derive(Debug)]
        struct Huge;
        impl sn_core::SampleSource for Huge {
            fn len(&self) -> usize {
                MAX_SAMPLES + 1
            }
            fn kind(&self) -> SampleKind {
                SampleKind::Real
            }
            fn read_complex(&self, _: usize, out: &mut [Complex32]) -> usize {
                out.len()
            }
            fn read_real(&self, _: usize, out: &mut [f32]) -> usize {
                out.len()
            }
        }
        let s = Signal::lazy(Arc::new(Huge), 8000.0).unwrap();
        let p = ChannelParams {
            frequency_offset: 10.0,
            drift: 0.0,
            ..ChannelParams::default()
        };
        let e = apply_channel(&s, &p, 1, &ChannelHints::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("the channel model"), "{e}");
    }

    #[test]
    fn stop_cancels_the_channel() {
        let s = tone(1000.0, 0.5, 8000.0, 1.0).unwrap();
        let p = ChannelParams {
            frequency_offset: 10.0,
            ..ChannelParams::default()
        };
        let e = apply_channel_with_stop(&s, &p, 1, &ChannelHints::default(), &|| true).unwrap_err();
        assert!(matches!(e, crate::Error::Cancelled), "{e}");
    }

    #[test]
    fn deterministic() {
        let s = tone(1000.0, 1.0, 8000.0, 0.2).unwrap();
        let p = ChannelParams {
            frequency_offset: 13.0,
            drift: 1.0,
            multipath: vec![
                Tap {
                    delay: 0.0,
                    gain: 1.0,
                    phase: 0.0,
                },
                Tap {
                    delay: 0.000_31,
                    gain: 0.3,
                    phase: 2.0,
                },
            ],
            clock_ppm: 150.0,
            noise: Some(NoiseLevel::SnrFullBand { db: 15.0 }),
            clip: Some(1.2),
        };
        let a = apply_channel(&s, &p, 9, &ChannelHints::default()).unwrap();
        let b = apply_channel(&s, &p, 9, &ChannelHints::default()).unwrap();
        let bits = |s: &Signal| -> Vec<u32> {
            as_complex(s)
                .iter()
                .map(|v| (v.re as f32).to_bits())
                .collect()
        };
        assert_eq!(bits(&a), bits(&b));
    }
}
