//! Precise frequency of a spectral line (SA: "Measurement of Freq", 0.1 Hz).
//!
//! 1. Coarse: averaged FFT spectrum of the selection, strongest bin in the band.
//! 2. Locate: the band around the coarse peak is shifted to 0 Hz, decimated and
//!    transformed with zero padding, which finds the main lobe of the full-record
//!    spectrum (a long record has many sidelobes inside one coarse bin).
//! 3. Fine: golden-section search for the maximum of the exact Hann-windowed DTFT over
//!    the whole selection, within one full-record bin of that lobe. Coherent
//!    integration over the full span gives resolution far below one FFT bin. For a
//!    real signal the search maximises the energy of a Hann-weighted least-squares
//!    sinusoid fit instead, which models the negative-frequency image and so stays
//!    unbiased for tones a few bins from 0 Hz or fs/2.

use rayon::prelude::*;
use sn_core::{Complex32, SampleKind, Signal};
use sn_dsp::spectrum::{Combine, SpectrumEngine};
use sn_dsp::window::WindowKind;

use crate::{Error, Result};

/// Result of a line measurement.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LineEstimate {
    /// Line frequency in Hz (relative to the signal's 0 Hz).
    pub frequency: f64,
    /// Line amplitude in dBFS.
    pub level_db: f64,
    /// Line power above the median of the band (without the line), in dB, in the
    /// averaged coarse spectrum (so it includes that spectrum's processing gain).
    pub snr_db: f64,
    /// One-sigma uncertainty of the frequency in Hz: the Cramer-Rao bound for the
    /// per-sample SNR, widened for the Hann window. For a real signal it is the
    /// standard deviation of the weighted fit of a real sinusoid. It grows within about
    /// one bin of 0 Hz and fs/2, where the tone and its image overlap, and there it is
    /// limited to the self-consistent spread of an estimate pinned at the edge (at most
    /// one bin). An estimate closer to the edge than that spread is reported at the
    /// edge, with that spread as its uncertainty and the level of the fit that far
    /// inside; such a level is only indicative, as a tone a few hundredths of a bin
    /// from the edge shows too little of a cycle to tell its amplitude from its
    /// frequency. Infinite when the noise floor cannot be estimated.
    pub uncertainty: f64,
}

/// Largest selection measured, in samples.
pub const MAX_SAMPLES: usize = 1 << 26;

/// Hann efficiency: the windowed full-record estimate has a larger spread than the
/// Cramer-Rao bound by this factor (in standard deviation). For a peak search on a
/// window `w` over centred time `t` it is
/// `sqrt(sum w^2 t^2 * sum t^2 / (sum w t^2)^2)`, which for a long Hann window
/// converges to about 1.53.
const HANN_CRB_FACTOR: f64 = 1.53;

/// Measure the strongest line in `lo..hi` Hz within samples `start..end`.
pub fn measure_line(
    signal: &Signal,
    start: usize,
    end: usize,
    lo: f64,
    hi: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<LineEstimate> {
    let fs = signal.sample_rate();
    let end = end.min(signal.len());
    let n = end.saturating_sub(start);
    if n < 64 {
        return Err(Error::Invalid(
            "select at least 64 samples with the V markers".into(),
        ));
    }
    if n > MAX_SAMPLES {
        return Err(Error::Invalid(format!(
            "the selection holds {n} samples; select at most {MAX_SAMPLES}"
        )));
    }
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    if lo.is_nan() || hi.is_nan() || hi <= lo {
        return Err(Error::Invalid("set the H markers around the line".into()));
    }
    let real = signal.kind() == SampleKind::Real;
    let (lo, hi) = search_band(lo, hi, fs, real)?;

    // ---- coarse: averaged spectrum ------------------------------------------
    let fft = n
        .next_power_of_two()
        .clamp(256, 1 << 18)
        .min(n.next_power_of_two());
    let fft = if fft > n { (fft / 2).max(64) } else { fft };
    let engine = SpectrumEngine::new(fft.max(16), WindowKind::Hann, SampleKind::Complex)?;
    // Frames spread evenly over the selection (at most 4096 of them).
    let room = n.saturating_sub(fft);
    let hop = (fft / 2).max(room / 4095).max(1);
    let frames: Vec<usize> = (0..=room).step_by(hop).map(|f| start + f).collect();
    if should_stop() {
        return Err(Error::Cancelled);
    }
    let db = engine.column_db(signal, &frames, Combine::Mean, should_stop)?;
    let usable = |k: usize| !real || engine.bin_frequency(k, fs) >= 0.0;
    let band: Vec<(usize, f32)> = db
        .iter()
        .copied()
        .enumerate()
        .filter(|(k, _)| {
            let f = engine.bin_frequency(*k, fs);
            f >= lo && f <= hi && usable(*k)
        })
        .collect();
    let (k0, peak_db) = band
        .iter()
        .copied()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .ok_or_else(|| Error::NotFound("the H markers do not cover any spectrum bins".into()))?;
    let median = noise_median(&db, &band, k0, &usable).unwrap_or(peak_db);
    let f0 = engine.bin_frequency(k0, fs);
    let bin = fs / fft as f64;

    // ---- fine: locate the main lobe, then golden-section on the exact DTFT -----
    let x = read(signal, start, n);
    let a = (f0 - 1.5 * bin).max(lo);
    let b = (f0 + 1.5 * bin).min(hi);
    let rough = locate(&x, f0, fs, (fft / 8).max(1), a, b, should_stop)?;
    let frequency = refine(&x, fs, real, rough, a, b, should_stop)?;
    let sigma2 = sample_noise_variance(median, frames.len(), fft);
    // A real estimate closer to 0 Hz or fs/2 than the self-consistent spread there
    // cannot be told apart from the edge: pin it there (see [`edge_spread`]).
    let spread = if real {
        edge_spread(&x, frequency, fs, sigma2)
    } else {
        None
    };
    let pinned = spread.filter(|s| s.distance < s.u);
    let frequency = pinned.map_or(frequency, |s| s.edge);
    // Symmetric Hann of length n: sum of weights is (n - 1) / 2.
    let wsum = (n as f64 - 1.0) / 2.0;
    // Amplitude per complex component. For a real signal it comes from the solved
    // least-squares fit: the captured energy alone depends on the tone's phase near
    // 0 Hz and fs/2, where the cos and sin columns are not orthogonal. For a pinned
    // estimate the fit is taken at the typical distance `u` from the edge.
    let fit = real.then(|| real_fit(&x, pinned.map_or(frequency, |s| s.inside()), fs));
    let amp_c = if let Some(fit) = &fit {
        0.5 * fit.amplitude
    } else {
        dtft_power(&x, frequency, fs).sqrt() / wsum
    };
    // The coarse peak is the same quantity (amplitude per component) seen through
    // an averaged spectrum; the full-record value cannot be far below it unless the
    // search missed the line.
    let coarse = 10_f64.powf(f64::from(peak_db) / 20.0);
    if amp_c < coarse * 10_f64.powf(-10.0 / 20.0) {
        return Err(Error::NotFound(
            "the line could not be resolved; narrow the H markers or select a cleaner span".into(),
        ));
    }
    let amplitude = if real { 2.0 * amp_c } else { amp_c };
    let snr_db = f64::from(peak_db - median);

    let uncertainty = crb_uncertainty(&Crb {
        fs,
        n,
        fft,
        frames: frames.len(),
        noise_median_db: median,
        amplitude,
        real,
        real_sd: match (pinned, &fit) {
            (Some(s), _) => Some(s.u),
            (None, Some(fit)) => real_frequency_sd(&x, frequency, fs, fit, sigma2, spread),
            (None, None) => None,
        },
    });
    Ok(LineEstimate {
        frequency,
        level_db: 20.0 * amplitude.max(1e-30).log10(),
        snr_db,
        uncertainty,
    })
}

/// Fine stage: golden-section search within one full-record bin of `rough` (and
/// within `a..b`), then parabolic steps with a shrinking spacing. A real estimate that
/// runs into 0 Hz or fs/2 is pinned there (see [`pinned_edge`]).
fn refine(
    x: &[Complex32],
    fs: f64,
    real: bool,
    rough: f64,
    a: f64,
    b: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<f64> {
    let full_bin = fs / x.len().max(1) as f64;
    // A real tone is a pair of complex tones at +f and -f. Near 0 Hz or fs/2 the
    // image leaks into the peak and biases a plain DTFT search, so real signals fit
    // both components (a weighted least-squares sinusoid) instead.
    let power = |f: f64| {
        if real {
            real_fit(x, f, fs).power
        } else {
            dtft_power(x, f, fs)
        }
    };
    let ga = (rough - full_bin).max(a);
    let gb = (rough + full_bin).min(b);
    if gb <= ga {
        return Ok(rough);
    }
    let tol = 1e-3 * full_bin;
    let mut f = golden_max(&power, ga, gb, tol, should_stop)?;
    if real && let Some(edge) = pinned_edge(f, ga, gb, fs, tol) {
        return Ok(edge);
    }
    // The golden search stops at 1e-3 of a bin, which at high SNR is far above the
    // Cramer-Rao bound; parabolic steps on the exact metric with a shrinking spacing
    // close the gap (near 0 Hz and fs/2 the real-fit metric is not quadratic over the
    // first spacing, so one step would leave a bias of about 1e-4 bin). Each step
    // evaluates and may land up to h outside `ga..gb`. For a real signal the metric is
    // even about 0 Hz and fs/2, so beyond those edges it would accept the mirror
    // point: keep the result inside the searched interval.
    let mut h = 1e-2 * full_bin;
    for _ in 0..4 {
        if should_stop() {
            return Err(Error::Cancelled);
        }
        f = parabolic_step(&power, f, h).clamp(ga, gb);
        h *= 0.1;
    }
    Ok(if power(f) >= power(rough) { f } else { rough })
}

/// The edge of the real spectrum (0 Hz or fs/2) that the golden search on `ga..gb`
/// ran into: the edge is an end of the searched interval and the result `f` is within
/// two search tolerances of it.
///
/// The real-fit metric is discontinuous there. As the trial frequency tends to the
/// edge the cos and sin columns tend to a constant (or `(-1)^k`) and a ramp, while at
/// the edge itself only the cos column is left. When the data look more like a ramp
/// than like any sinusoid inside the band (noise, or a tone a few hundredths of a bin
/// from the edge), the supremum of the metric is approached at the edge but never
/// reached, and the search stops half a tolerance inside it. There the solved sin
/// coefficient is the ramp's slope over a vanishing frequency: the amplitude and the
/// linearised spread are meaningless. Such an estimate is pinned at the edge instead.
fn pinned_edge(f: f64, ga: f64, gb: f64, fs: f64, tol: f64) -> Option<f64> {
    let nyquist = 0.5 * fs.abs();
    let near = 2.0 * tol;
    if ga <= 0.0 && f - ga <= near {
        Some(0.0)
    } else if gb >= nyquist && gb - f <= near {
        Some(nyquist)
    } else {
        None
    }
}

/// The H marker band to search. A real signal's spectrum is symmetric about 0 Hz and
/// fs/2, and so is the fine search metric: a band spanning either edge would give it
/// two equal peaks (+f and -f, or f and fs - f), so real signals keep to 0..fs/2.
fn search_band(lo: f64, hi: f64, fs: f64, real: bool) -> Result<(f64, f64)> {
    if !real {
        return Ok((lo, hi));
    }
    let (lo, hi) = (lo.max(0.0), hi.min(fs / 2.0));
    if hi <= lo {
        return Err(Error::Invalid(
            "the signal is real: set the H markers between 0 Hz and half the sample rate".into(),
        ));
    }
    Ok((lo, hi))
}

/// Noise floor in dB: the median of the band bins without the line's main lobe
/// (bins within 2 of `k0`). When the markers bracket the line tightly, the bins
/// 3..=64 away from it (`usable` ones) are used instead.
fn noise_median(
    db: &[f32],
    band: &[(usize, f32)],
    k0: usize,
    usable: &dyn Fn(usize) -> bool,
) -> Option<f32> {
    let mut noise: Vec<f32> = band
        .iter()
        .filter(|(k, _)| k.abs_diff(k0) > 2)
        .map(|b| b.1)
        .collect();
    if noise.len() < 16 {
        noise = db
            .iter()
            .copied()
            .enumerate()
            .filter(|(k, _)| (3..=64).contains(&k.abs_diff(k0)) && usable(*k))
            .map(|b| b.1)
            .collect();
    }
    noise.sort_by(f32::total_cmp);
    noise.get(noise.len() / 2).copied()
}

/// Inputs of [`crb_uncertainty`].
struct Crb {
    fs: f64,
    /// Samples in the full record.
    n: usize,
    /// Coarse FFT size.
    fft: usize,
    /// Frames averaged in the coarse spectrum.
    frames: usize,
    /// Median noise bin of the coarse spectrum, dB (engine scale).
    noise_median_db: f32,
    /// Tone amplitude (peak).
    amplitude: f64,
    real: bool,
    /// For a real signal, the standard deviation of the windowed fit's frequency in
    /// Hz (see [`real_frequency_sd`]).
    real_sd: Option<f64>,
}

/// One-sigma frequency uncertainty: the Cramer-Rao bound with the per-sample SNR.
///
/// The median of an average of K exponential bins is about (1 - 1/(9K))^3 of their
/// mean; the spectrum's noise per bin is sigma^2 * ENBW / fft (ENBW of a symmetric
/// Hann: 1.5 N / (N - 1)). The bound `6 / (snr N (N^2 - 1))` holds for a complex
/// tone with `snr = A^2 / sigma^2`. A real tone A cos is a complex tone of amplitude
/// A / 2 (plus its mirror), so its complex-equivalent SNR is A^2 / (4 sigma^2); this
/// is the usual `12 / (SNR N (N^2 - 1))` with SNR = A^2 / (2 sigma^2).
///
/// For a real signal the far-from-edge formula above understates the spread within
/// about a bin of 0 Hz or fs/2, where the cos and sin columns are correlated; there
/// the standard deviation from [`real_frequency_sd`] is used when it is available.
fn crb_uncertainty(c: &Crb) -> f64 {
    let sigma2 = sample_noise_variance(c.noise_median_db, c.frames, c.fft);
    let power = c.amplitude * c.amplitude;
    let snr = if c.real {
        power / (4.0 * sigma2)
    } else {
        power / sigma2
    };
    let nf = c.n as f64;
    if let Some(sd) = c.real_sd {
        return if sd.is_finite() && sd > 0.0 {
            sd
        } else {
            f64::INFINITY
        };
    }
    if snr.is_finite() && snr > 0.0 {
        HANN_CRB_FACTOR * c.fs / std::f64::consts::TAU * (6.0 / (snr * nf * (nf * nf - 1.0))).sqrt()
    } else {
        f64::INFINITY
    }
}

/// Noise variance per (complex) sample from the median noise bin of the coarse
/// spectrum (see [`crb_uncertainty`]).
fn sample_noise_variance(noise_median_db: f32, frames: usize, fft: usize) -> f64 {
    let k_frames = frames.max(1) as f64;
    let median_to_mean = (1.0 - 1.0 / (9.0 * k_frames)).powi(3);
    let noise_bin = 10_f64.powf(f64::from(noise_median_db) / 10.0) / median_to_mean;
    let fft = fft as f64;
    let enbw = 1.5 * fft / (fft - 1.0);
    noise_bin * fft / enbw
}

/// The frequency in `a..b` where the full-record Hann-windowed spectrum of `x` peaks,
/// to within a quarter of a full-record bin (`f0` itself when `a..b` holds no grid
/// point).
///
/// `x` is shifted down by `f0`, windowed, low-pass filtered and decimated by `d`
/// (a third-order CIC), and the short decimated record is transformed with heavy
/// zero padding. Searching that grid finds the main lobe; a golden-section search on
/// the exact DTFT alone could lock onto a sidelobe when the record is long.
fn locate(
    x: &[Complex32],
    f0: f64,
    fs: f64,
    d: usize,
    a: f64,
    b: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<f64> {
    let n = x.len();
    let d = d.max(1);
    let h = cic3(d);
    let outputs = n / d + 3;
    let mut y = vec![(0.0_f64, 0.0_f64); outputs];
    let mstep = -std::f64::consts::TAU * f0 / fs;
    let hstep = std::f64::consts::TAU / (n as f64 - 1.0);
    // Output m sums samples m d .. m d + 3 d - 2, so blocks of outputs are
    // independent: each reads the samples its outputs need and adds them in the same
    // ascending order as a single pass. The block size depends only on `d`.
    let per_block = (LOCATE_BLOCK / d).max(LOCATE_MIN_OUTPUTS);
    y.par_chunks_mut(per_block)
        .enumerate()
        .try_for_each(|(b, ys)| {
            if should_stop() {
                return Err(Error::Cancelled);
            }
            let m0 = b * per_block;
            let k0 = m0 * d;
            let k1 = ((m0 + ys.len()) * d + 2 * d).min(n);
            let mut mix = Phasor::new_at(mstep, k0);
            let mut hann = Phasor::new_at(hstep, k0);
            for (k, v) in (k0..k1).zip(x.get(k0..k1).unwrap_or(&[])) {
                let (mr, mi) = mix.next_value();
                let w = 0.5 - 0.5 * hann.next_value().0;
                let (xr, xi) = (f64::from(v.re) * w, f64::from(v.im) * w);
                let (zr, zi) = (xr * mr - xi * mi, xr * mi + xi * mr);
                for j in 0..3 {
                    let Some(m) = (k / d).checked_sub(j) else {
                        continue;
                    };
                    let Some(local) = m.checked_sub(m0) else {
                        continue;
                    };
                    if let (Some(hv), Some(o)) = (h.get(k - m * d), ys.get_mut(local)) {
                        o.0 += zr * hv;
                        o.1 += zi * hv;
                    }
                }
            }
            Ok(())
        })?;
    let peak = y
        .iter()
        .map(|(r, i)| r.hypot(*i))
        .fold(0.0_f64, f64::max)
        .max(1e-300);
    let ys: Vec<Complex32> = y
        .iter()
        .map(|(r, i)| Complex32::new((r / peak) as f32, (i / peak) as f32))
        .collect();
    let rate = fs / d as f64;
    let size = (outputs.next_power_of_two() * 8).clamp(64, 1 << 22);
    let engine = SpectrumEngine::new(size, WindowKind::Rectangular, SampleKind::Complex)?;
    let spec = engine.frame_db(&Signal::complex(ys, rate)?, 0);
    spec.iter()
        .enumerate()
        .map(|(k, v)| (f0 + engine.bin_frequency(k, rate), *v))
        .filter(|(f, _)| *f >= a && *f <= b)
        .max_by(|p, q| p.1.total_cmp(&q.1))
        // An interval narrower than the grid step: the coarse peak is the best guess.
        .map_or(Ok(f0.clamp(a, b)), |p| Ok(p.0))
}

/// Samples per parallel block of [`locate`], when that gives at least
/// [`LOCATE_MIN_OUTPUTS`] outputs per block.
const LOCATE_BLOCK: usize = 1 << 18;

/// Fewest outputs per parallel block of [`locate`]. A block of `p` outputs reads
/// `(p + 2) d` samples (the CIC spans three outputs), so neighbouring blocks redo
/// `2 d` samples; with at least 64 outputs that overlap stays near 3% of a block.
const LOCATE_MIN_OUTPUTS: usize = 64;

/// Impulse response of three cascaded boxcars of length `d` (length `3d - 2`).
fn cic3(d: usize) -> Vec<f64> {
    // Box * box is a triangle; the third box is a running sum over d taps.
    let tri: Vec<f64> = (0..2 * d - 1)
        .map(|i| (i + 1).min(2 * d - 1 - i) as f64)
        .collect();
    let mut prefix = Vec::with_capacity(tri.len() + 1);
    prefix.push(0.0_f64);
    let mut acc = 0.0;
    for v in &tri {
        acc += v;
        prefix.push(acc);
    }
    let p = |i: usize| prefix.get(i.min(tri.len())).copied().unwrap_or(0.0);
    (0..3 * d - 2)
        .map(|i| p(i + 1) - p((i + 1).saturating_sub(d)))
        .collect()
}

/// `e^{j step k}` for k = 0, 1, 2, ... by recurrence in f64, renormalised every 1024
/// steps so it does not drift.
struct Phasor {
    re: f64,
    im: f64,
    sr: f64,
    si: f64,
    count: usize,
}

impl Phasor {
    fn new(step: f64) -> Self {
        Self {
            re: 1.0,
            im: 0.0,
            sr: step.cos(),
            si: step.sin(),
            count: 0,
        }
    }

    /// The current value (cos, sin), then advance by one step.
    fn next_value(&mut self) -> (f64, f64) {
        let out = (self.re, self.im);
        let nr = self.re * self.sr - self.im * self.si;
        self.im = self.re * self.si + self.im * self.sr;
        self.re = nr;
        self.count += 1;
        if self.count.is_multiple_of(1024) {
            let m = self.re.hypot(self.im);
            if m > 0.0 {
                self.re /= m;
                self.im /= m;
            }
        }
        out
    }

    /// [`Self::new`] advanced to index `k0`: the value `e^{j step k0}` is computed
    /// directly, so a block of a long record can start anywhere.
    fn new_at(step: f64, k0: usize) -> Self {
        let mut p = Self::new(step);
        if k0 > 0 {
            let ph = step * k0 as f64;
            p.re = ph.cos();
            p.im = ph.sin();
        }
        p
    }

    /// [`Self::real_line`] advanced to index `k0` (see [`Self::new_at`]). At the
    /// edges the value stays exactly 1 or -1 with a zero sine.
    fn real_line_at(f: f64, fs: f64, k0: usize) -> Self {
        let mut p = Self::real_line(f, fs);
        if k0 > 0 {
            match real_edge(f, fs) {
                Some(Edge::Zero) => {}
                Some(Edge::Nyquist) => p.re = if k0.is_multiple_of(2) { 1.0 } else { -1.0 },
                None => {
                    let ph = std::f64::consts::TAU * f / fs * k0 as f64;
                    p.re = ph.cos();
                    p.im = ph.sin();
                }
            }
        }
        p
    }

    /// `e^{j 2 pi f k / fs}` for a real-sinusoid model at `f`. Within `1e-9 fs` of
    /// 0 Hz or fs/2 the frequency is snapped to that edge and the step is set exactly
    /// (1 or -1, with a zero sine), so the sin column is exactly zero there instead of
    /// round-off that grows with k.
    fn real_line(f: f64, fs: f64) -> Self {
        let (sr, si) = match real_edge(f, fs) {
            Some(Edge::Zero) => (1.0, 0.0),
            Some(Edge::Nyquist) => (-1.0, 0.0),
            None => {
                let step = std::f64::consts::TAU * f / fs;
                (step.cos(), step.sin())
            }
        };
        Self {
            re: 1.0,
            im: 0.0,
            sr,
            si,
            count: 0,
        }
    }
}

/// An edge of the real spectrum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Zero,
    Nyquist,
}

/// The edge of the real spectrum (0 Hz or fs/2) that `f` is within `1e-9 fs` of.
fn real_edge(f: f64, fs: f64) -> Option<Edge> {
    let tol = 1e-9 * fs.abs();
    if f.abs() <= tol {
        Some(Edge::Zero)
    } else if (f.abs() - 0.5 * fs.abs()).abs() <= tol {
        Some(Edge::Nyquist)
    } else {
        None
    }
}

/// One parabolic-interpolation step towards the maximum of `f` near `x`, using
/// `f(x - h)`, `f(x)` and `f(x + h)`. Returns `x` when the three values do not form
/// a peak, the vertex leaves `x - h..x + h`, or the step does not raise `f`.
fn parabolic_step(f: &dyn Fn(f64) -> f64, x: f64, h: f64) -> f64 {
    let (fl, fc, fr) = (f(x - h), f(x), f(x + h));
    let curvature = fl - 2.0 * fc + fr;
    if !(curvature < 0.0 && curvature.is_finite()) {
        return x;
    }
    let delta = 0.5 * h * (fl - fr) / curvature;
    if !(delta.is_finite() && delta.abs() <= h) {
        return x;
    }
    let v = x + delta;
    if f(v) >= fc { v } else { x }
}

/// Maximum of a unimodal function on `a..b` by golden-section search.
fn golden_max(
    f: &dyn Fn(f64) -> f64,
    mut a: f64,
    mut b: f64,
    tol: f64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<f64> {
    let g = (5_f64.sqrt() - 1.0) / 2.0;
    let mut c = b - g * (b - a);
    let mut d = a + g * (b - a);
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..60 {
        if should_stop() {
            return Err(Error::Cancelled);
        }
        if fc > fd {
            b = d;
            d = c;
            fd = fc;
            c = b - g * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + g * (b - a);
            fd = f(d);
        }
        if (b - a).abs() < tol {
            break;
        }
    }
    Ok(f64::midpoint(a, b))
}

fn read(signal: &Signal, start: usize, n: usize) -> Vec<Complex32> {
    let mut x = vec![Complex32::new(0.0, 0.0); n];
    signal.read_complex(start, &mut x);
    x
}

/// `|sum w[k] x[k] e^{-j 2 pi f k / fs}|^2` with the symmetric Hann window of length
/// `x.len()` computed on the fly (no window is stored).
fn dtft_power(x: &[Complex32], f: f64, fs: f64) -> f64 {
    let n = x.len();
    let step = -std::f64::consts::TAU * f / fs;
    let hstep = std::f64::consts::TAU / (n.max(2) as f64 - 1.0);
    let (re, im) = blocks(
        x,
        |k0, xs| dtft_sums(xs, k0, step, hstep),
        |a, b| (a.0 + b.0, a.1 + b.1),
    );
    re * re + im * im
}

/// The part of the [`dtft_power`] sum over `xs`, the samples starting at index `k0`
/// of the record, with mixer step `step` and Hann step `hstep` (rad/sample).
fn dtft_sums(xs: &[Complex32], k0: usize, step: f64, hstep: f64) -> (f64, f64) {
    let mut rot = Phasor::new_at(step, k0);
    let mut hann = Phasor::new_at(hstep, k0);
    let (mut re, mut im) = (0.0_f64, 0.0_f64);
    for v in xs {
        let (pr, pi) = rot.next_value();
        let wk = 0.5 - 0.5 * hann.next_value().0;
        let (xr, xi) = (f64::from(v.re) * wk, f64::from(v.im) * wk);
        re += xr * pr - xi * pi;
        im += xr * pi + xi * pr;
    }
    (re, im)
}

/// Samples per block of the full-record sums. Fixed (not tied to the thread count),
/// so the partial sums, and the result, do not depend on the machine.
const BLOCK: usize = 1 << 16;

/// `f(k0, x[k0..(k0 + BLOCK).min(x.len())])` for each block of `x` (`k0` a multiple
/// of [`BLOCK`]) in parallel, combined with `combine` in block order (a deterministic
/// reduction). Every block is [`BLOCK`] samples long except the last, which may be
/// shorter.
fn blocks<T: Send + Default>(
    x: &[Complex32],
    f: impl Fn(usize, &[Complex32]) -> T + Sync,
    combine: impl Fn(T, T) -> T,
) -> T {
    let parts: Vec<T> = x
        .par_chunks(BLOCK)
        .enumerate()
        .map(|(b, xs)| f(b * BLOCK, xs))
        .collect();
    parts.into_iter().fold(T::default(), combine)
}

/// Element-wise sum of two arrays of partial sums.
fn add_arrays<const N: usize>(mut a: [f64; N], b: [f64; N]) -> [f64; N] {
    for (x, y) in a.iter_mut().zip(b) {
        *x += y;
    }
    a
}

/// Energy captured by the best real sinusoid `c cos(w k) + d sin(w k)` at `f`, in a
/// least-squares fit to the real part of `x` weighted by the symmetric Hann window,
/// scaled by half the window sum so that away from 0 Hz and fs/2 it equals
/// [`dtft_power`] (where the cos and sin columns are orthogonal with equal weight).
///
/// With `W = sum w`, `Gc + j Gs = sum w e^{j 2 w k}`, `bc = sum w x cos` and
/// `bs = sum w x sin`, the normal matrix is `S = [[W + Gc, Gs], [Gs, W - Gc]] / 2` and
/// the captured energy is `b^T S^-1 b`. A noiseless tone is fitted with no residual at
/// its true frequency, where the captured energy reaches its upper bound (the weighted
/// energy of `x`), so the image term no longer pulls the maximum; what remains is
/// rounding of the f32 samples and the search tolerance, which grow as the metric
/// flattens within about one full-record bin of 0 Hz or fs/2.
///
/// The returned amplitude is `hypot(c, d)` of the solved fit `S^-1 b`, which unlike
/// the captured energy does not depend on the tone's phase.
struct RealFit {
    power: f64,
    amplitude: f64,
    /// Fitted cos coefficient (0 when no fit was possible).
    c: f64,
    /// Fitted sin coefficient (0 when the sin column was dropped).
    d: f64,
}

/// The weighted least-squares real sinusoid fit at `f` (see [`RealFit`]).
fn real_fit(x: &[Complex32], f: f64, fs: f64) -> RealFit {
    let n = x.len();
    let hstep = std::f64::consts::TAU / (n.max(2) as f64 - 1.0);
    let [bc, bs, wsum, gc, gs] =
        blocks(x, |k0, xs| real_fit_sums(xs, k0, f, fs, hstep), add_arrays);
    let (scc, sss, scs) = (f64::midpoint(wsum, gc), f64::midpoint(wsum, -gc), 0.5 * gs);
    let det = scc * sss - scs * scs;
    let scale = 0.5 * wsum;
    if det > 1e-12 * wsum * wsum {
        let c = (sss * bc - scs * bs) / det;
        let d = (scc * bs - scs * bc) / det;
        RealFit {
            // b^T S^-1 b = b . theta
            power: scale * (bc * c + bs * d),
            amplitude: c.hypot(d),
            c,
            d,
        }
    } else if scc > 0.0 {
        // At exactly 0 Hz or fs/2 the sin column vanishes: fit the cos alone.
        RealFit {
            power: scale * bc * bc / scc,
            amplitude: (bc / scc).abs(),
            c: bc / scc,
            d: 0.0,
        }
    } else {
        RealFit {
            power: 0.0,
            amplitude: 0.0,
            c: 0.0,
            d: 0.0,
        }
    }
}

/// Variance of the frequency (rad/sample) of the Hann-weighted least-squares fit of
/// `c cos(w k) + d sin(w k)` with unknowns `(c, d, w)`, per unit noise variance of the
/// real samples.
///
/// With the model's gradient `g_k = [cos, sin, t (d cos - c sin)]` (`t` the time from
/// the record centre, which only reparametrises the phase), `A = sum w g g^T` and
/// `B = sum w^2 g g^T`, the weighted fit's covariance is `sigma^2 A^-1 B A^-1`. Far
/// from 0 Hz and fs/2 this is the Cramer-Rao bound times the Hann factor; near them the
/// cos and sin columns are correlated and the variance grows, which this captures.
/// When the sin column is negligible against the cos column (a fit at 0 Hz or fs/2,
/// where it vanishes) it is dropped. `None` when the frequency column is negligible
/// too (as it is at those edges, where the fitted sin coefficient is zero) or no
/// usable bound can be formed otherwise. The tests are absolute, against the cos
/// column's energy and the model's scale, not relative to the product of the
/// diagonal, which a matrix made of round-off can pass.
fn real_frequency_variance(x: &[Complex32], f: f64, fs: f64, fit: &RealFit) -> Option<f64> {
    let n = x.len();
    let hstep = std::f64::consts::TAU / (n.max(2) as f64 - 1.0);
    let centre = (n as f64 - 1.0) / 2.0;
    // Upper triangles of A and B ([00, 01, 02, 11, 12, 22] each), then the sum of
    // w t^2.
    let sums: [f64; 13] = blocks(
        x,
        |k0, xs| variance_sums(xs, k0, f, fs, hstep, centre, fit),
        add_arrays,
    );
    let [
        a00,
        a01,
        a02,
        a11,
        a12,
        a22,
        b00,
        b01,
        b02,
        b11,
        b12,
        b22,
        t2,
    ] = sums;
    // Third column of the adjugate of the symmetric A: v = A^-1 e3 = adj / det.
    let adj = [
        a01 * a12 - a02 * a11,
        a01 * a02 - a00 * a12,
        a00 * a11 - a01 * a01,
    ];
    let det = a02 * adj[0] + a12 * adj[1] + a22 * adj[2];
    // The frequency column's energy against the fitted amplitude's times sum w t^2.
    let freq_floor = 1e-9 * (fit.c * fit.c + fit.d * fit.d) * t2;
    if !(a00 > 0.0 && a22.is_finite() && a22 > freq_floor) {
        return None;
    }
    let scale = a00 * a11 * a22;
    let variance = if a11 > 1e-9 * a00 && det.is_finite() && det > 1e-9 * scale {
        let v = adj.map(|e| e / det);
        v[0] * v[0] * b00
            + v[1] * v[1] * b11
            + v[2] * v[2] * b22
            + 2.0 * (v[0] * v[1] * b01 + v[0] * v[2] * b02 + v[1] * v[2] * b12)
    } else {
        // Drop the sin column: unknowns (c, w).
        let det2 = a00 * a22 - a02 * a02;
        if !(det2.is_finite() && det2 > 1e-9 * a00 * a22) {
            return None;
        }
        let (v0, v2) = (-a02 / det2, a00 / det2);
        v0 * v0 * b00 + v2 * v2 * b22 + 2.0 * v0 * v2 * b02
    };
    (variance.is_finite() && variance > 0.0).then_some(variance)
}

/// Standard deviation in Hz of the frequency of the Hann-weighted real-sinusoid fit at
/// `f`, for noise variance `sigma2` per sample (see [`real_frequency_variance`]).
///
/// Near 0 Hz or fs/2 the linearised spread grows roughly as 1/distance from the edge.
/// Within one full-record bin of an edge (`spread`, from [`edge_spread`]) the reported
/// value is the smaller of the linearised spread and the self-consistent spread `u`
/// there; the two agree where the distance is `u`. `None` when no usable value can be
/// formed.
fn real_frequency_sd(
    x: &[Complex32],
    f: f64,
    fs: f64,
    fit: &RealFit,
    sigma2: f64,
    spread: Option<EdgeSpread>,
) -> Option<f64> {
    if !(sigma2.is_finite() && sigma2 > 0.0) {
        return None;
    }
    let linear =
        real_frequency_variance(x, f, fs, fit).and_then(|v| positive(hz_sd(v, fs, sigma2)));
    let Some(s) = spread else {
        return linear;
    };
    if linear.is_some_and(|sd| sd <= s.distance) {
        return linear;
    }
    Some(linear.map_or(s.u, |sd| sd.min(s.u)))
}

/// `sd` when it is finite and positive.
fn positive(sd: f64) -> Option<f64> {
    (sd.is_finite() && sd > 0.0).then_some(sd)
}

/// A frequency variance in (rad/sample)^2 per unit noise variance as a standard
/// deviation in Hz for noise variance `sigma2`.
fn hz_sd(variance: f64, fs: f64, sigma2: f64) -> f64 {
    fs.abs() / std::f64::consts::TAU * (sigma2 * variance).sqrt()
}

/// Where a real estimate stands against the nearer edge of the real spectrum, with
/// the self-consistent spread there (see [`edge_spread`]).
#[derive(Debug, Clone, Copy)]
struct EdgeSpread {
    /// The nearer edge, 0 Hz or fs/2.
    edge: f64,
    /// +1 when frequencies inside the band are above the edge, -1 below.
    inward: f64,
    /// Distance of the estimate from the edge, Hz.
    distance: f64,
    /// Self-consistent spread, Hz (at most one full-record bin).
    u: f64,
}

impl EdgeSpread {
    /// The frequency `u` inside the edge.
    fn inside(&self) -> f64 {
        self.edge + self.inward * self.u
    }
}

/// The self-consistent spread of a real estimate within one full-record bin of 0 Hz
/// or fs/2, for noise variance `sigma2` per sample.
///
/// At the edge itself the linearised spread is undefined (the derivative of
/// `cos(pi k + phi)` in the frequency vanishes with the sin column), and just inside it
/// the fit degenerates: the cos and sin columns tend to a constant (or `(-1)^k`) and a
/// ramp, so a solved fit there can be the ramp's slope over a vanishing frequency, with
/// an amplitude far above the data and a linearised spread far below the truth. An
/// estimate closer to the edge than `u`, the distance at which the spread of a fit `u`
/// inside the edge equals `u`, cannot be told apart from the edge, and the noise moves
/// it about as far as it moves a tone at distance `u`. `u` is found by a few
/// geometric-mean steps and kept within one bin. `None` beyond one bin of the edges or
/// when no usable value can be formed.
fn edge_spread(x: &[Complex32], f: f64, fs: f64, sigma2: f64) -> Option<EdgeSpread> {
    if !(sigma2.is_finite() && sigma2 > 0.0) {
        return None;
    }
    let nyquist = 0.5 * fs.abs();
    let (edge, inward) = if f.abs() <= 0.5 * nyquist {
        (0.0, 1.0)
    } else {
        (nyquist, -1.0)
    };
    let distance = (f.abs() - edge).abs();
    let bin = fs.abs() / x.len().max(1) as f64;
    if distance > bin {
        return None;
    }
    let mut u = 0.01 * bin;
    for _ in 0..8 {
        let g = edge + inward * u;
        let fit_g = real_fit(x, g, fs);
        let sd = real_frequency_variance(x, g, fs, &fit_g)
            .and_then(|v| positive(hz_sd(v, fs, sigma2)))?;
        let next = (u * sd).sqrt().min(bin);
        let done = (next - u).abs() <= 0.01 * u;
        u = next;
        if done {
            break;
        }
    }
    positive(u).map(|u| EdgeSpread {
        edge,
        inward,
        distance,
        u,
    })
}

/// The part of the [`real_fit`] sums `[bc, bs, wsum, gc, gs]` over `xs`, the samples
/// starting at index `k0` of the record, with Hann step `hstep`.
fn real_fit_sums(xs: &[Complex32], k0: usize, f: f64, fs: f64, hstep: f64) -> [f64; 5] {
    let mut rot = Phasor::real_line_at(f, fs, k0);
    let mut hann = Phasor::new_at(hstep, k0);
    let (mut bc, mut bs, mut wsum, mut gc, mut gs) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0, 0.0);
    for v in xs {
        let (c, s) = rot.next_value();
        let wk = 0.5 - 0.5 * hann.next_value().0;
        let xw = f64::from(v.re) * wk;
        bc += xw * c;
        bs += xw * s;
        wsum += wk;
        gc += wk * (c * c - s * s);
        gs += wk * 2.0 * c * s;
    }
    [bc, bs, wsum, gc, gs]
}

/// The part of the [`real_frequency_variance`] sums over `xs`, the samples starting
/// at index `k0` of the record: the upper triangles of A and B
/// (`[00, 01, 02, 11, 12, 22]` each), then the sum of `w t^2`.
fn variance_sums(
    xs: &[Complex32],
    k0: usize,
    f: f64,
    fs: f64,
    hstep: f64,
    centre: f64,
    fit: &RealFit,
) -> [f64; 13] {
    let mut rot = Phasor::real_line_at(f, fs, k0);
    let mut hann = Phasor::new_at(hstep, k0);
    let mut a = [0.0_f64; 6];
    let mut b = [0.0_f64; 6];
    let mut t2 = 0.0_f64;
    for k in k0..k0 + xs.len() {
        let (c, s) = rot.next_value();
        let wk = 0.5 - 0.5 * hann.next_value().0;
        let t = k as f64 - centre;
        t2 += wk * t * t;
        let g = [c, s, t * (fit.d * c - fit.c * s)];
        let products = [
            g[0] * g[0],
            g[0] * g[1],
            g[0] * g[2],
            g[1] * g[1],
            g[1] * g[2],
            g[2] * g[2],
        ];
        for ((ai, bi), p) in a.iter_mut().zip(b.iter_mut()).zip(products) {
            *ai += wk * p;
            *bi += wk * wk * p;
        }
    }
    let [a0, a1, a2, a3, a4, a5] = a;
    let [b0, b1, b2, b3, b4, b5] = b;
    [a0, a1, a2, a3, a4, a5, b0, b1, b2, b3, b4, b5, t2]
}

#[cfg(test)]
mod tests {
    use sn_synth::{add_awgn_snr, tone};

    use super::*;

    #[test]
    fn measures_a_noisy_tone_far_below_one_bin() {
        let fs = 48_000.0;
        let s = add_awgn_snr(&tone(2399.76, 0.25, fs, 2.0).unwrap(), 10.0, 3).unwrap();
        let e = measure_line(&s, 0, s.len(), 2000.0, 2800.0, &|| false).unwrap();
        assert!((e.frequency - 2399.76).abs() < 0.01, "{e:?}");
        assert!((e.level_db - 20.0 * 0.25_f64.log10()).abs() < 0.5, "{e:?}");
        assert!(e.snr_db > 20.0);
        assert!(e.uncertainty > 0.0 && e.uncertainty < 0.01, "{e:?}");
    }

    /// Mean reported uncertainty over the empirical RMS error, over 30 noise seeds.
    fn uncertainty_ratio(clean: &Signal, f: f64, noisy: impl Fn(&Signal, u64) -> Signal) -> f64 {
        let mut errs = Vec::new();
        let mut reported = Vec::new();
        for seed in 0..30 {
            let s = noisy(clean, 100 + seed);
            let (lo, hi) = (f - 400.0, f + 400.0);
            let e = measure_line(&s, 0, s.len(), lo, hi, &|| false).unwrap();
            errs.push(e.frequency - f);
            reported.push(e.uncertainty);
        }
        let rms = (errs.iter().map(|v| v * v).sum::<f64>() / errs.len() as f64).sqrt();
        let mean_u = reported.iter().sum::<f64>() / reported.len() as f64;
        mean_u / rms
    }

    #[test]
    fn uncertainty_matches_the_spread_over_noise_seeds() {
        let fs = 48_000.0;
        let f = 2399.76;
        let real = tone(f, 0.25, fs, 0.5).unwrap();
        let r = uncertainty_ratio(&real, f, |s, seed| add_awgn_snr(s, 0.0, seed).unwrap());
        assert!(
            (0.75..1.33).contains(&r),
            "real tone: reported/empirical {r:.3}"
        );

        let x: Vec<Complex32> = (0..24_000)
            .map(|n| {
                let ph = std::f64::consts::TAU * f * f64::from(n) / fs;
                Complex32::new((0.25 * ph.cos()) as f32, (0.25 * ph.sin()) as f32)
            })
            .collect();
        let complex = Signal::complex(x, fs).unwrap();
        let r = uncertainty_ratio(&complex, f, |s, seed| add_awgn_snr(s, 0.0, seed).unwrap());
        assert!(
            (0.75..1.33).contains(&r),
            "complex tone: reported/empirical {r:.3}"
        );
        // At high SNR the search itself must not dominate the error.
        let r = uncertainty_ratio(&complex, f, |s, seed| add_awgn_snr(s, 40.0, seed).unwrap());
        assert!(
            (0.6..1.6).contains(&r),
            "complex tone at 40 dB: reported/empirical {r:.3}"
        );
        let r = uncertainty_ratio(&real, f, |s, seed| add_awgn_snr(s, 50.0, seed).unwrap());
        assert!(
            (0.6..1.6).contains(&r),
            "real tone at 50 dB: reported/empirical {r:.3}"
        );
    }

    #[test]
    fn real_tones_near_dc_and_nyquist_are_not_biased_by_the_image() {
        // A real tone a few bins from 0 Hz (or fs/2) leaks its negative-frequency
        // image into the peak; the two-component fit must remove that bias.
        let fs = 8000.0;
        for f in [3.3, 1.7, 5.1, fs / 2.0 - 3.3] {
            let clean = tone(f, 0.25, fs, 1.0).unwrap();
            let (lo, hi) = if f < 100.0 {
                (0.0, 40.0)
            } else {
                (fs / 2.0 - 40.0, fs / 2.0)
            };
            let e = measure_line(&clean, 0, clean.len(), lo, hi, &|| false).unwrap();
            assert!((e.frequency - f).abs() < 1e-5, "noiseless {f}: {e:?}");
            let mut worst = 0.0_f64;
            for seed in 0..10 {
                let s = add_awgn_snr(&clean, 40.0, 7 + seed).unwrap();
                let e = measure_line(&s, 0, s.len(), lo, hi, &|| false).unwrap();
                worst = worst.max((e.frequency - f).abs() / e.uncertainty);
            }
            assert!(worst < 3.5, "{f} Hz: worst error {worst:.2} sigma");
        }
    }

    /// A real tone `amp cos(2 pi f t + phase)`, `secs` long.
    fn real_tone(f: f64, amp: f64, phase: f64, fs: f64, secs: f64) -> Signal {
        let n = (fs * secs) as usize;
        let x: Vec<f32> = (0..n)
            .map(|k| (amp * (std::f64::consts::TAU * f * k as f64 / fs + phase).cos()) as f32)
            .collect();
        Signal::real(x, fs).unwrap()
    }

    #[test]
    fn real_tones_within_a_bin_of_the_edges_stay_positive_and_level_is_phase_free() {
        let fs = 8000.0;
        // Markers spanning 0 Hz (or fs/2) must not let the search land on the image.
        for phase in [0.0, 1.0, 2.0] {
            let s = real_tone(0.4, 0.25, phase, fs, 1.0);
            let e = measure_line(&s, 0, s.len(), -40.0, 40.0, &|| false).unwrap();
            assert!(
                e.frequency > 0.0 && (e.frequency - 0.4).abs() < 1e-3,
                "{e:?}"
            );
            let f = fs / 2.0 - 0.4;
            let s = real_tone(f, 0.25, phase, fs, 1.0);
            let e = measure_line(&s, 0, s.len(), f - 40.0, f + 40.0, &|| false).unwrap();
            assert!(
                e.frequency < fs / 2.0 && (e.frequency - f).abs() < 1e-3,
                "{e:?}"
            );
        }
        // The level comes from the solved fit, not from the phase-dependent energy.
        let truth = 20.0 * 0.25_f64.log10();
        for f in [0.4, 0.8, 1.2, fs / 2.0 - 0.8] {
            for phase in [
                0.0,
                std::f64::consts::FRAC_PI_4,
                std::f64::consts::FRAC_PI_2,
            ] {
                let s = real_tone(f, 0.25, phase, fs, 1.0);
                let e = measure_line(&s, 0, s.len(), f - 40.0, f + 40.0, &|| false).unwrap();
                assert!(
                    (e.level_db - truth).abs() < 0.02,
                    "{f} Hz, phase {phase}: {e:?}"
                );
            }
        }
        // Markers entirely below 0 Hz on a real signal are rejected.
        let s = real_tone(100.0, 0.25, 0.0, fs, 1.0);
        assert!(measure_line(&s, 0, s.len(), -200.0, -10.0, &|| false).is_err());
    }

    #[test]
    fn real_lines_at_the_edges_never_leave_zero_to_nyquist() {
        let fs = 8000.0;
        let n = 8000;
        // A DC offset (alone and with noise) and a near-0 Hz cosine, markers from or
        // across 0 Hz; and the same at fs/2.
        let dc = Signal::real(vec![0.25_f32; n], fs).unwrap();
        let cases = [
            (dc.clone(), 0.0, 40.0),
            (dc.clone(), -40.0, 40.0),
            (add_awgn_snr(&dc, 30.0, 11).unwrap(), 0.0, 40.0),
            (real_tone(0.001, 0.25, 0.0, fs, 1.0), -40.0, 40.0),
        ];
        for (s, lo, hi) in &cases {
            let e = measure_line(s, 0, s.len(), *lo, *hi, &|| false).unwrap();
            assert!(e.frequency >= 0.0, "{lo}..{hi}: {e:?}");
        }
        let nyq: Vec<f32> = (0..n)
            .map(|k| if k % 2 == 0 { 0.25 } else { -0.25 })
            .collect();
        let nyq = Signal::real(nyq, fs).unwrap();
        for s in [nyq.clone(), add_awgn_snr(&nyq, 30.0, 12).unwrap()] {
            let e =
                measure_line(&s, 0, s.len(), fs / 2.0 - 40.0, fs / 2.0 + 40.0, &|| false).unwrap();
            assert!(e.frequency <= fs / 2.0, "{e:?}");
        }
    }

    #[test]
    fn uncertainty_stays_sane_for_lines_at_zero_and_nyquist() {
        // Estimates that land exactly on 0 Hz or fs/2 must not invert a model column
        // made of round-off: the reported uncertainty stays finite and close to the
        // spread over noise seeds.
        let fs = 8000.0;
        let n = 8000;
        let nyq: Vec<f32> = (0..n)
            .map(|k| if k % 2 == 0 { 0.25 } else { -0.25 })
            .collect();
        let nyq = Signal::real(nyq, fs).unwrap();
        let dc = Signal::real(vec![0.25_f32; n], fs).unwrap();
        for (clean, lo, hi, f) in [
            (&nyq, fs / 2.0 - 40.0, fs / 2.0, fs / 2.0),
            (&dc, 0.0, 40.0, 0.0),
        ] {
            let mut errs = Vec::new();
            let mut reported = Vec::new();
            let mut on_edge = 0;
            for seed in 0..40_u64 {
                let s = add_awgn_snr(clean, 30.0, 500 + seed).unwrap();
                let e = measure_line(&s, 0, s.len(), lo, hi, &|| false).unwrap();
                assert!(e.uncertainty.is_finite(), "{f} Hz seed {seed}: {e:?}");
                if (e.frequency - f).abs() <= 1e-9 * fs {
                    on_edge += 1;
                }
                errs.push(e.frequency - f);
                reported.push(e.uncertainty);
            }
            assert!(on_edge > 0, "{f} Hz: no estimate landed on the edge");
            let rms = (errs.iter().map(|v| v * v).sum::<f64>() / errs.len() as f64).sqrt();
            // Before the fix, estimates on the edge reported about 1e9 Hz. Seeds that
            // land just inside report the linearised spread, which is larger the closer
            // they land.
            for u in &reported {
                let r = u / rms;
                assert!(
                    (0.25..4.0).contains(&r),
                    "{f} Hz: reported {u} vs rms {rms}"
                );
            }
            let mean_u = reported.iter().sum::<f64>() / reported.len() as f64;
            let r = mean_u / rms;
            assert!((0.6..1.7).contains(&r), "{f} Hz: reported/empirical {r:.3}");
        }
    }

    #[test]
    fn uncertainty_holds_for_real_tones_within_a_bin_of_the_edges() {
        let fs = 8000.0;
        for f in [0.3, 0.6, fs / 2.0 - 0.45] {
            let (lo, hi) = if f < 100.0 {
                (0.0, 40.0)
            } else {
                (fs / 2.0 - 40.0, fs / 2.0)
            };
            let mut errs = Vec::new();
            let mut reported = Vec::new();
            for seed in 0..60_u64 {
                let phase = seed as f64 * 2.399;
                let clean = real_tone(f, 0.25, phase, fs, 1.0);
                let s = add_awgn_snr(&clean, 30.0, 300 + seed).unwrap();
                let e = measure_line(&s, 0, s.len(), lo, hi, &|| false).unwrap();
                errs.push(e.frequency - f);
                reported.push(e.uncertainty);
            }
            let rms = (errs.iter().map(|v| v * v).sum::<f64>() / errs.len() as f64).sqrt();
            let mean_u = reported.iter().sum::<f64>() / reported.len() as f64;
            let r = mean_u / rms;
            assert!((0.6..1.7).contains(&r), "{f} Hz: reported/empirical {r:.3}");
        }
    }

    #[test]
    fn real_tones_hundredths_of_a_bin_from_the_edges_keep_level_and_uncertainty() {
        // Just inside 0 Hz or fs/2 the real fit degenerates into a constant plus a
        // ramp. Estimates there used to report levels up to 75 dB too high (above full
        // scale) and uncertainties 1e3 to 1e4 times too small.
        let fs = 8000.0;
        let truth = 20.0 * 0.25_f64.log10();
        for off in [0.05, 0.1] {
            for f in [off, fs / 2.0 - off] {
                let clean = tone(f, 0.25, fs, 1.0).unwrap();
                let (lo, hi) = if f < 100.0 {
                    (0.0, 40.0)
                } else {
                    (fs / 2.0 - 40.0, fs / 2.0)
                };
                for snr in [0.0, 10.0] {
                    let mut errs = Vec::new();
                    let mut reported = Vec::new();
                    for seed in 0..40_u64 {
                        let s = add_awgn_snr(&clean, snr, 700 + seed).unwrap();
                        let e = measure_line(&s, 0, s.len(), lo, hi, &|| false).unwrap();
                        let err = e.frequency - f;
                        let case = format!("{f} Hz at {snr} dB, seed {seed}: {e:?}");
                        assert!(e.uncertainty.is_finite() && e.uncertainty > 0.0, "{case}");
                        assert!(err.abs() / e.uncertainty < 8.0, "{case}");
                        // A tone 0.05 bin from the edge shows a twentieth of a cycle,
                        // so its level can read low; it must never read far above.
                        assert!((-14.0..8.0).contains(&(e.level_db - truth)), "{case}");
                        errs.push(err);
                        reported.push(e.uncertainty);
                    }
                    let rms = (errs.iter().map(|v| v * v).sum::<f64>() / errs.len() as f64).sqrt();
                    let mean_u = reported.iter().sum::<f64>() / reported.len() as f64;
                    let r = mean_u / rms;
                    assert!(
                        (0.6..1.7).contains(&r),
                        "{f} Hz at {snr} dB: reported/empirical {r:.3}"
                    );
                }
            }
        }
    }

    #[test]
    fn noiseless_real_tones_near_the_edges_converge() {
        // The real-fit metric is not quadratic near 0 Hz and fs/2: a single parabolic
        // step after the golden search left about 1e-4 bin of bias there.
        let fs = 8000.0;
        for off in [0.1, 0.3, 0.5, 0.9, 1.5, 3.0] {
            for f in [off, fs / 2.0 - off] {
                let s = tone(f, 0.25, fs, 1.0).unwrap();
                let (lo, hi) = if f < 100.0 {
                    (0.0, 40.0)
                } else {
                    (fs / 2.0 - 40.0, fs / 2.0)
                };
                let e = measure_line(&s, 0, s.len(), lo, hi, &|| false).unwrap();
                assert!((e.frequency - f).abs() < 2e-6, "{f} Hz: {e:?}");
            }
        }
    }

    #[test]
    fn tight_markers_keep_the_snr() {
        let fs = 48_000.0;
        let s = add_awgn_snr(&tone(2400.0, 0.25, fs, 2.0).unwrap(), 10.0, 5).unwrap();
        let wide = measure_line(&s, 0, s.len(), 2000.0, 2800.0, &|| false).unwrap();
        let tight = measure_line(&s, 0, s.len(), 2399.0, 2401.0, &|| false).unwrap();
        assert!((tight.frequency - 2400.0).abs() < 0.01, "{tight:?}");
        assert!(
            (tight.snr_db - wide.snr_db).abs() < 3.0,
            "tight {} dB vs wide {} dB",
            tight.snr_db,
            wide.snr_db
        );
    }

    #[test]
    fn long_records_lock_onto_the_main_lobe() {
        // Many full-record bins per coarse bin: the fine search must not stop on a
        // sidelobe or a noise lobe.
        let fs = 100_000.0;
        let f = 10_000.0;
        for seed in 0..3 {
            let s = add_awgn_snr(&tone(f, 0.25, fs, 60.0).unwrap(), 10.0, seed).unwrap();
            let e = measure_line(&s, 0, s.len(), 9_000.0, 11_000.0, &|| false).unwrap();
            assert!((e.frequency - f).abs() < 1e-3, "seed {seed}: {e:?}");
            assert!((e.level_db - 20.0 * 0.25_f64.log10()).abs() < 0.5, "{e:?}");
        }
    }

    #[test]
    fn works_on_complex_and_negative_frequencies() {
        let fs = 8000.0;
        let x: Vec<Complex32> = (0..40_000)
            .map(|n| {
                let ph = -std::f64::consts::TAU * 1234.567 * f64::from(n) / fs;
                Complex32::new(ph.cos() as f32, ph.sin() as f32)
            })
            .collect();
        let s = Signal::complex(x, fs).unwrap();
        let e = measure_line(&s, 0, s.len(), -1500.0, -1000.0, &|| false).unwrap();
        assert!((e.frequency + 1234.567).abs() < 0.001, "{e:?}");
    }

    /// The value of `p` after `k0` calls of `next_value`.
    fn advanced(mut p: Phasor, k0: usize) -> (f64, f64) {
        for _ in 0..k0 {
            p.next_value();
        }
        p.next_value()
    }

    fn close(a: (f64, f64), b: (f64, f64), tol: f64) -> bool {
        (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol
    }

    #[test]
    fn phasors_seeded_at_an_index_match_the_recurrence() {
        let ks = [0, 1, 2, 1023, 1024, 65_536, 1_000_001];
        for step in [0.0, 1e-6, 0.1234, -0.987, 3.0, std::f64::consts::PI] {
            for k0 in ks {
                let seeded = Phasor::new_at(step, k0).next_value();
                let serial = advanced(Phasor::new(step), k0);
                assert!(
                    close(seeded, serial, 1e-9),
                    "step {step} k0 {k0}: {seeded:?} vs {serial:?}"
                );
            }
        }
        let fs = 48_000.0;
        for f in [0.0, fs / 2.0, -fs / 2.0, 1234.567, fs / 2.0 - 0.37] {
            for k0 in ks {
                let seeded = Phasor::real_line_at(f, fs, k0).next_value();
                let serial = advanced(Phasor::real_line(f, fs), k0);
                assert!(
                    close(seeded, serial, 1e-9),
                    "f {f} k0 {k0}: {seeded:?} vs {serial:?}"
                );
                if real_edge(f, fs).is_some() {
                    // At the edges both are exact: +-1 with a zero sine.
                    assert_eq!(seeded, serial, "f {f} k0 {k0}");
                    assert_eq!(seeded.1, 0.0);
                }
            }
        }
        // The Nyquist edge alternates sign with the parity of k0.
        assert_eq!(
            Phasor::real_line_at(fs / 2.0, fs, 7).next_value(),
            (-1.0, 0.0)
        );
        assert_eq!(
            Phasor::real_line_at(fs / 2.0, fs, 8).next_value(),
            (1.0, 0.0)
        );
    }

    /// A record spanning several blocks plus a short last block.
    fn block_test_record() -> Vec<Complex32> {
        (0..3 * BLOCK + 17)
            .map(|k| {
                let t = k as f64;
                Complex32::new(
                    (0.5 * (0.0123 * t).cos() + 0.1 * (0.7 * t).sin()) as f32,
                    (0.5 * (0.0123 * t).sin() - 0.2 * (1.9 * t).cos()) as f32,
                )
            })
            .collect()
    }

    fn assert_rel(got: &[f64], want: &[f64], scale: &[f64], what: &str) {
        for (i, ((g, w), s)) in got.iter().zip(want).zip(scale).enumerate() {
            let tol = 1e-12 * w.abs().max(*s);
            assert!((g - w).abs() <= tol, "{what}[{i}]: {g} vs {w}");
        }
    }

    #[test]
    fn block_sums_match_a_single_serial_pass() {
        let x = block_test_record();
        let n = x.len();
        let nf = n as f64;
        let fs = 48_000.0;
        let hstep = std::f64::consts::TAU / (nf - 1.0);
        for f in [0.0, 94.0, 1234.567, fs / 2.0 - 3.3, fs / 2.0] {
            let step = -std::f64::consts::TAU * f / fs;
            let (re, im) = blocks(
                &x,
                |k0, xs| dtft_sums(xs, k0, step, hstep),
                |a, b| (a.0 + b.0, a.1 + b.1),
            );
            let (sre, sim) = dtft_sums(&x, 0, step, hstep);
            assert_rel(&[re, im], &[sre, sim], &[nf, nf], &format!("dtft {f}"));
            let power = dtft_power(&x, f, fs);
            assert!((power - (sre * sre + sim * sim)).abs() <= 1e-11 * power.max(nf * nf));

            let got = blocks(&x, |k0, xs| real_fit_sums(xs, k0, f, fs, hstep), add_arrays);
            let want = real_fit_sums(&x, 0, f, fs, hstep);
            assert_rel(&got, &want, &[nf; 5], &format!("real fit {f}"));

            let fit = RealFit {
                power: 0.0,
                amplitude: 0.0,
                c: 0.4,
                d: -0.3,
            };
            let centre = (nf - 1.0) / 2.0;
            let got = blocks(
                &x,
                |k0, xs| variance_sums(xs, k0, f, fs, hstep, centre, &fit),
                add_arrays,
            );
            let want = variance_sums(&x, 0, f, fs, hstep, centre, &fit);
            let tri = [nf, nf, nf * nf, nf, nf * nf, nf * nf * nf];
            let mut scale = [0.0; 13];
            for (i, v) in scale.iter_mut().enumerate() {
                *v = tri.get(i % 6).copied().unwrap_or(nf * nf * nf);
            }
            assert_rel(&got, &want, &scale, &format!("variance {f}"));
        }
    }

    #[test]
    fn rejects_bad_selections() {
        let s = tone(100.0, 0.5, 1000.0, 1.0).unwrap();
        assert!(measure_line(&s, 0, 10, 50.0, 150.0, &|| false).is_err());
        assert!(measure_line(&s, 0, 1000, 150.0, 150.0, &|| false).is_err());
        assert!(matches!(
            measure_line(&s, 0, 1000, 50.0, 150.0, &|| true),
            Err(Error::Cancelled)
        ));
    }
}
