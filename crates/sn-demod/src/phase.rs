//! PhasePlane: carrier and symbol-clock recovery and the constellation (report 7.4).

use std::sync::Arc;

use sn_analysis::histogram::Histogram;
use sn_core::{Complex32, Signal};
use sn_dsp::detect::analytic;
use sn_dsp::fir::Fir;
use sn_dsp::mix::mix_in_place;
use sn_dsp::resample::Resampler;

use crate::{Error, Result, invalid};

/// Samples per symbol used internally.
pub const SPS: u32 = 8;

/// Carrier-loop speed (SA 6.2.6: Slow, Medium, Quick, Hard; 6.0: Slowly, Media, Quick).
///
/// Every speed is a second-order decision-directed loop run once per symbol, with
/// damping 0.707 and the one-sided noise bandwidth `B_L*T` (relative to the symbol rate)
/// given for each variant. A wider loop follows faster phase changes but lets through
/// more noise: the phase jitter grows with `B_L*T`, and with it the risk of cycle slips
/// (the loop jumping by one constellation sector, which rotates every later absolute
/// code). A frequency ramp of R rad per symbol squared leaves a steady phase error of
/// R / ki rad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum PllSpeed {
    /// Very soft (`B_L*T` = 0.004): for exact angle measurement.
    Slow,
    /// Soft but effective (`B_L*T` = 0.018): the normal working mode.
    #[default]
    Medium,
    /// Aggressive (`B_L*T` = 0.085): pulls in large mistuning fast; not for measuring.
    Quick,
    /// Hardest (SA 6.2.6, `B_L*T` = 0.13, about 1.5 times Quick): follows strong phase
    /// noise and fast drift better than Quick, at the cost of cycle-slip immunity. It
    /// needs a clean signal, about 20 dB Es/N0 for 8-PSK; on weaker signals use Quick.
    Hard,
}

impl PllSpeed {
    /// One-sided noise bandwidth of the loop times the symbol period.
    pub fn noise_bandwidth(self) -> f64 {
        match self {
            Self::Slow => 0.004,
            Self::Medium => 0.018,
            Self::Quick => 0.085,
            Self::Hard => 0.13,
        }
    }

    /// Proportional and integral gains `(kp, ki)` of the loop, from the noise
    /// bandwidth and a damping of 0.707 (the standard second-order loop design).
    fn gains(self) -> (f64, f64) {
        let zeta = std::f64::consts::FRAC_1_SQRT_2;
        let theta = self.noise_bandwidth() / (zeta + 1.0 / (4.0 * zeta));
        let d = 1.0 + 2.0 * zeta * theta + theta * theta;
        (4.0 * zeta * theta / d, 4.0 * theta * theta / d)
    }
}

/// PhasePlane settings.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PhaseConfig {
    /// Carrier frequency in Hz (SA: Carrier, pre-filled from Sc).
    pub carrier: f64,
    /// Keying rate in Bd (SA: BR, pre-filled from Br).
    pub baud: f64,
    /// Modulation order 1..=32 (SA: n-Ary). 1 turns carrier tracking off.
    pub order: u32,
    /// Carrier-loop speed.
    pub pll: PllSpeed,
    /// Symbol-clock synchronisation (SA: Synch BR).
    pub clock_sync: bool,
    /// Root-raised-cosine matched filter roll-off. `None` applies the matched filter
    /// for rectangular pulses instead: a moving average over one symbol.
    pub rrc_alpha: Option<f64>,
    /// Phase-difference display order 0..=3 (SA: Diff).
    pub diff: u8,
}

impl Default for PhaseConfig {
    fn default() -> Self {
        Self {
            carrier: 1800.0,
            baud: 2400.0,
            order: 4,
            pll: PllSpeed::Medium,
            clock_sync: true,
            rrc_alpha: None,
            diff: 0,
        }
    }
}

/// Everything PhasePlane shows.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseResult {
    /// Symbol points after carrier removal, normalised to RMS 1 (Diff 0).
    pub points: Vec<Complex32>,
    /// Points as displayed (after the requested Diff order).
    pub display: Vec<Complex32>,
    /// Baseband trajectory (8 samples per symbol, de-rotated), for transitions mode.
    pub trajectory: Vec<Complex32>,
    /// Angle of each displayed point in degrees, 0..360.
    pub angles: Vec<f32>,
    /// Absolute symbol codes (angle sector of each point, counter-clockwise from 0).
    pub codes: Vec<u32>,
    /// Relative codes: difference of consecutive absolute codes, mod order.
    pub relative_codes: Vec<u32>,
    /// Mean residual carrier offset in Hz: the least-squares slope of the unwrapped loop
    /// phase over the last 3/4 of the symbols (the final loop frequency when fewer than
    /// 3 symbols remain), folded to +-baud/(2 * order) because an order-M loop cannot
    /// tell offsets baud/M apart (0 for order 1). It is an average over that span, not
    /// the final value; each cycle slip in the span biases it by up to
    /// 1.5 * (baud / order) / N Hz (6u(1-u) * (baud / order) / N for a slip at
    /// fraction u of the span), N being the number of symbols in the span.
    pub carrier_offset: f64,
    /// Average symbol period found by the clock loop, in input samples.
    pub symbol_period: f64,
}

/// Cubic (Catmull-Rom) interpolation of `x` at fractional index `t`.
fn interp(x: &[Complex32], t: f64) -> Option<Complex32> {
    let i = t.floor();
    let mu = (t - i) as f32;
    let i = i as i64;
    let at = |k: i64| usize::try_from(k).ok().and_then(|k| x.get(k)).copied();
    let (p0, p1, p2, p3) = (at(i - 1)?, at(i)?, at(i + 1)?, at(i + 2)?);
    let a = p1 * 2.0;
    let b = p2 - p0;
    let c = p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3;
    let d = (p1 - p2) * 3.0 + p3 - p0;
    Some((a + b * mu + c * mu * mu + d * mu * mu * mu) * 0.5)
}

/// The rectangular matched filter's output at the input rate, shared between the
/// baseband filter and the symbol read-out.
type Matched = Arc<[Complex32]>;

/// Complex baseband with the carrier removed.
///
/// Returns the baseband at [`SPS`] samples per symbol and, with `rect_matched`, the
/// output of the matched filter for rectangular pulses at the input rate.
///
/// The matched filter (a moving average over exactly one symbol, integrate and dump)
/// runs at the input rate before any band limiting, so its output is exact: triangular
/// pulses that peak at the symbol ends. Band limiting would round those peaks (at 8
/// samples per symbol by a few degrees for 8-PSK), so the symbol values are read from
/// the input-rate output, and the 8 samples per symbol version serves the clock loop,
/// the trajectory and the eye.
///
/// The lowpass before resampling sits at 1.25 times the baud without the matched
/// filter (enough for any RRC roll-off up to 1) and at 2 times the baud after it. The
/// resampler's own filter keeps everything above 3.6 times the baud out.
fn baseband(
    input: &Signal,
    carrier: f64,
    baud: f64,
    rect_matched: bool,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<(Vec<Complex32>, Option<Matched>)> {
    let fs = input.sample_rate();
    if !(baud.is_finite() && baud > 0.0 && baud * 2.0 <= fs) {
        return Err(invalid(
            "baud",
            format!(
                "must be above 0 and at most half the sample rate ({} Hz); got {baud}",
                fs / 2.0
            ),
        ));
    }
    if !carrier.is_finite() {
        return Err(invalid("carrier", "must be a finite frequency in Hz"));
    }
    let mut z = analytic(input, should_stop)?;
    mix_in_place(&mut z, 0, -carrier, fs);
    // The matched output is shared between the filter input and the caller, so
    // it is held once; the analytic signal is dropped as soon as it is used.
    let (bb, matched) = if rect_matched {
        let m = symbol_average(&z, fs / baud);
        drop(z);
        (Signal::complex(Arc::clone(&m), fs)?, Some(m))
    } else {
        (Signal::complex(z, fs)?, None)
    };
    let factor = if rect_matched { 2.0 } else { 1.25 };
    let cutoff = (baud * factor).min(fs * 0.45);
    let transition = (baud * 0.5).min(fs * 0.05).max(1.0);
    let lp = Fir::lowpass(cutoff, transition, 60.0, fs)?;
    let filtered = Signal::complex(lp.apply(&bb, 0, bb.len(), should_stop)?, fs)?;
    let r = Resampler::new(fs, f64::from(SPS) * baud)?;
    Ok((r.apply(&filtered, 0, filtered.len(), should_stop)?, matched))
}

/// Linear interpolation of `x` at fractional index `t` (exact on the piecewise-linear
/// output of the rectangular matched filter).
fn lerp(x: &[Complex32], t: f64) -> Option<Complex32> {
    if t.is_nan() || t < 0.0 {
        return None;
    }
    let i = t.floor() as usize;
    let f = (t - i as f64) as f32;
    let a = *x.get(i)?;
    Some(x.get(i + 1).map_or(a, |b| a + (b - a) * f))
}

/// Causal moving average over exactly `len` samples (`len` >= 1, fractional allowed:
/// the oldest sample gets the fractional weight). Output `n` integrates the `len`
/// samples ending at `n` (integrate and dump), so rectangular pulses peak at their
/// ends.
fn symbol_average(x: &[Complex32], len: f64) -> Matched {
    let len = len.max(1.0);
    let whole = len.floor() as usize;
    let frac = len - whole as f64;
    let mut prefix: Vec<(f64, f64)> = Vec::with_capacity(x.len() + 1);
    let (mut re, mut im) = (0.0_f64, 0.0_f64);
    prefix.push((re, im));
    for v in x {
        re += f64::from(v.re);
        im += f64::from(v.im);
        prefix.push((re, im));
    }
    let at = |i: usize| prefix.get(i).copied().unwrap_or((0.0, 0.0));
    (0..x.len())
        .map(|n| {
            // Whole samples n + 1 - whole ..= n, plus `frac` of the one before.
            let a = (n + 1).saturating_sub(whole);
            let (hr, hi) = at(n + 1);
            let (lr, li) = at(a);
            let (mut sr, mut si) = (hr - lr, hi - li);
            if frac > 0.0
                && let Some(old) = a.checked_sub(1).and_then(|j| x.get(j))
            {
                sr += frac * f64::from(old.re);
                si += frac * f64::from(old.im);
            }
            Complex32::new((sr / len) as f32, (si / len) as f32)
        })
        .collect()
}

/// Gardner timing recovery on `bb` (SPS samples per symbol). Returns symbol samples,
/// the fractional indices they were taken at, and the average period in samples of
/// `bb`.
fn gardner(
    bb: &[Complex32],
    sync: bool,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<(Vec<Complex32>, Vec<f64>, f64)> {
    let sps = f64::from(SPS);
    // Start at the phase with the largest average power (best eye opening).
    let start = (0..SPS)
        .max_by(|a, b| {
            let pw = |ph: u32| {
                bb.iter()
                    .skip(ph as usize)
                    .step_by(SPS as usize)
                    .take(2000)
                    .map(Complex32::norm_sqr)
                    .sum::<f32>()
            };
            pw(*a).total_cmp(&pw(*b))
        })
        .map_or(0.0, f64::from);
    let (mut t, mut period) = (start + sps, sps);
    let (mut out, mut idx) = (Vec::new(), Vec::new());
    let mut prev: Option<Complex32> = None;
    let (kp, ki) = (0.02, 0.0002);
    let mut count = 0_usize;
    while let (Some(y), Some(mid)) = (interp(bb, t), interp(bb, t - period / 2.0)) {
        count += 1;
        if count.is_multiple_of(4096) && should_stop() {
            return Err(Error::Cancelled);
        }
        // Record the instant `y` was actually taken at, before the correction.
        out.push(y);
        idx.push(t);
        if sync && let Some(p) = prev {
            let e = f64::from(((y - p) * mid.conj()).re)
                / f64::from(y.norm_sqr() + p.norm_sqr() + 1e-12);
            period = (period - ki * e).clamp(sps * 0.98, sps * 1.02);
            t -= kp * e;
        }
        prev = Some(y);
        t += period;
    }
    if out.len() < 16 {
        return Err(invalid(
            "selection",
            "fewer than 16 symbols: select more signal or check the baud rate",
        ));
    }
    // Average period over the whole run, not the final (jittering) loop state.
    let avg = match (idx.first(), idx.last()) {
        (Some(a), Some(b)) if idx.len() > 1 => (b - a) / (idx.len() - 1) as f64,
        _ => period,
    };
    Ok((out, idx, avg))
}

fn normalise(v: &mut [Complex32]) {
    let rms = (v.iter().map(Complex32::norm_sqr).sum::<f32>() / v.len().max(1) as f32).sqrt();
    if rms > 0.0 {
        for c in v {
            *c /= rms;
        }
    }
}

/// Apply the phase-difference operator `order` times: `p[n] * conj(p[n-1]) / |p[n-1]|`.
pub fn difference(points: &[Complex32], order: u8) -> Vec<Complex32> {
    let mut v = points.to_vec();
    for _ in 0..order.min(3) {
        let mut prev: Option<Complex32> = None;
        v = v
            .iter()
            .map(|c| {
                let d = prev.map_or(*c, |p| c * p.conj() / p.norm().max(1e-12));
                prev = Some(*c);
                d
            })
            .collect();
    }
    v
}

/// Run PhasePlane on samples `start..end`.
pub fn run(
    signal: &Signal,
    start: usize,
    end: usize,
    cfg: &PhaseConfig,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<PhaseResult> {
    if !(1..=32).contains(&cfg.order) {
        return Err(invalid("n-Ary", "must be between 1 and 32"));
    }
    let end = end.min(signal.len());
    let part = signal.slice(start, end.max(start))?;
    let (bb, matched) = baseband(
        &part,
        cfg.carrier,
        cfg.baud,
        cfg.rrc_alpha.is_none(),
        should_stop,
    )?;
    let rate = f64::from(SPS) * cfg.baud;
    let bb = match cfg.rrc_alpha {
        Some(a) => {
            let f = Fir::rrc(cfg.baud, a, 8, rate)?;
            let s = Signal::complex(bb, rate)?;
            f.apply(&s, 0, s.len(), should_stop)?
        }
        None => bb,
    };
    let (mut sym, idx, period) = gardner(&bb, cfg.clock_sync, should_stop)?;
    if let Some(z) = &matched {
        // Exact integrate-and-dump values at the instants the clock loop found
        // (the resampler has no delay: baseband sample t is input sample t * fs / rate).
        let scale = part.sample_rate() / rate;
        for (y, t) in sym.iter_mut().zip(&idx) {
            if let Some(v) = lerp(z, t * scale) {
                *y = v;
            }
        }
    }
    normalise(&mut sym);

    // Order-M decision PLL on the symbols.
    // The loop state is kept in f64 and the phase wrapped to 0..2 pi, so long
    // recordings with a residual offset keep full angle precision.
    let (kp, ki) = cfg.pll.gains();
    let m = cfg.order;
    let mf = f64::from(m);
    let (mut theta, mut freq) = (0.0_f64, 0.0_f64);
    // Unwrapped loop phase, kept for the carrier offset estimate.
    let mut unwrapped = 0.0_f64;
    let mut track = Vec::with_capacity(sym.len());
    let mut thetas = Vec::with_capacity(sym.len());
    let points: Vec<Complex32> = sym
        .iter()
        .map(|y| {
            let th = theta as f32;
            let r = y * Complex32::from_polar(1.0, -th);
            thetas.push(th);
            track.push(unwrapped);
            if m > 1 && r.norm() > 1e-6 {
                let e = f64::from((r / r.norm()).powu(m).arg()) / mf;
                freq += ki * e;
                let step = freq + kp * e;
                unwrapped += step;
                theta = (theta + step).rem_euclid(std::f64::consts::TAU);
            }
            r
        })
        .collect();

    let display = difference(&points, cfg.diff);
    let sector = std::f32::consts::TAU / m as f32;
    let codes: Vec<u32> = points
        .iter()
        .map(|p| ((p.arg().rem_euclid(std::f32::consts::TAU) / sector).round() as u32) % m)
        .collect();
    let mut last = None;
    let relative_codes: Vec<u32> = codes
        .iter()
        .map(|c| {
            let r = last.map_or(*c, |l| (c + m - l) % m);
            last = Some(*c);
            r
        })
        .collect();
    let angles = display
        .iter()
        .map(|p| p.arg().to_degrees().rem_euclid(360.0))
        .collect();

    // Trajectory: baseband between symbols, de-rotated with the loop phase.
    let rms = (bb.iter().map(Complex32::norm_sqr).sum::<f32>() / bb.len().max(1) as f32)
        .sqrt()
        .max(1e-12);
    let mut trajectory = Vec::new();
    for (k, t) in idx.iter().enumerate().take(20_000) {
        let th = thetas.get(k).copied().unwrap_or(0.0);
        let rot = Complex32::from_polar(1.0 / rms, -th);
        for s in 0..SPS {
            if let Some(v) = interp(&bb, t + f64::from(s)) {
                trajectory.push(v * rot);
            }
        }
    }
    let carrier_offset = offset_hz(&track, freq, m, cfg.baud);
    Ok(PhaseResult {
        points,
        display,
        trajectory,
        angles,
        codes,
        relative_codes,
        carrier_offset,
        // `period` is in baseband samples; sample_rate / rate converts to input samples.
        symbol_period: period * signal.sample_rate() / rate,
    })
}

/// Residual carrier offset in Hz from the unwrapped loop phase `track` (one value per
/// symbol), falling back to the final loop frequency `freq` on very short tracks.
fn offset_hz(track: &[f64], freq: f64, order: u32, baud: f64) -> f64 {
    if order < 2 {
        return 0.0;
    }
    let per_symbol = locked_slope(track).unwrap_or(freq);
    // An order-M loop cannot tell offsets baud / M apart: report the one nearest 0.
    let sector = std::f64::consts::TAU / f64::from(order);
    let folded = per_symbol - (per_symbol / sector).round() * sector;
    folded / std::f64::consts::TAU * baud
}

/// Least-squares slope (radians per symbol) of the loop phase over its last three
/// quarters, where the loop has locked. The final loop frequency alone is a single
/// noisy sample; the slope uses every symbol. `None` with fewer than 3 symbols.
fn locked_slope(track: &[f64]) -> Option<f64> {
    let skip = track.len() / 4;
    let span = track.get(skip..)?;
    if span.len() < 3 {
        return None;
    }
    let n = span.len() as f64;
    let mid = (n - 1.0) / 2.0;
    let first = span.first().copied().unwrap_or(0.0);
    let (mut sxy, mut sxx) = (0.0_f64, 0.0_f64);
    for (k, y) in span.iter().enumerate() {
        let x = k as f64 - mid;
        sxy += x * (y - first);
        sxx += x * x;
    }
    (sxx > 0.0).then(|| sxy / sxx)
}

/// Histogram of angles (degrees) in 1000 bins of 0.36 degrees (SA's resolution).
pub fn angle_histogram(angles: &[f32]) -> Vec<u32> {
    let mut h = vec![0_u32; 1000];
    for a in angles {
        let k = (a.rem_euclid(360.0) / 0.36) as usize;
        if let Some(c) = h.get_mut(k.min(999)) {
            *c += 1;
        }
    }
    h
}

/// Rings of the constellation: radii (relative to RMS) where point magnitudes cluster.
/// PSK has one ring; QAM-16 has three (ratios about 0.45 : 1 : 1.34).
pub fn rings(points: &[Complex32]) -> Vec<f32> {
    let mags: Vec<f32> = points.iter().map(|p| p.norm()).collect();
    let Ok(s) = Signal::real(mags, 1.0) else {
        return Vec::new();
    };
    let Ok(h) = Histogram::build(&s, 0, s.len(), 400, Some((0.0, 2.0))) else {
        return Vec::new();
    };
    let mut r: Vec<f32> = h.peaks(0.25, 0.03).iter().map(|v| *v as f32).collect();
    r.sort_by(f32::total_cmp);
    r
}

/// Eye diagram traces: the real part of the trajectory folded at two symbols.
pub fn eye(trajectory: &[Complex32], max_traces: usize) -> Vec<Vec<f32>> {
    let span = 2 * SPS as usize;
    let (symbols, _) = trajectory.as_chunks::<{ SPS as usize }>();
    symbols
        .iter()
        .take(max_traces.saturating_add(1))
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| {
            w.iter()
                .flat_map(|c| c.iter().map(|v| v.re))
                .take(span)
                .collect()
        })
        .collect()
}

/// Offset-PSK check: in OQPSK the I and Q transitions are half a symbol apart. Returns
/// the timing difference of I and Q transitions in symbols (0..0.5) for the rotation
/// that maximises it; about 0.5 means an offset mode.
pub fn offset_timing(trajectory: &[Complex32]) -> f32 {
    let sps = SPS as f32;
    let phase_of = |vals: &[f32]| {
        // Phase of the baud line in the transition energy.
        let (mut re, mut im) = (0.0_f32, 0.0_f32);
        for (n, w) in vals.windows(2).enumerate() {
            if let [a, b] = w {
                let e = (b - a) * (b - a);
                let ph = std::f32::consts::TAU * n as f32 / sps;
                re += e * ph.cos();
                im -= e * ph.sin();
            }
        }
        im.atan2(re)
    };
    [0.0_f32, std::f32::consts::FRAC_PI_4]
        .iter()
        .map(|rot| {
            let r = Complex32::from_polar(1.0, *rot);
            let i: Vec<f32> = trajectory.iter().map(|c| (c * r).re).collect();
            let q: Vec<f32> = trajectory.iter().map(|c| (c * r).im).collect();
            let d = (phase_of(&i) - phase_of(&q))
                .abs()
                .rem_euclid(std::f32::consts::TAU);
            let d = d.min(std::f32::consts::TAU - d);
            d / std::f32::consts::TAU
        })
        .fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use sn_synth::{PskParams, add_awgn_esn0, add_awgn_snr, psk, random_symbols};

    use super::*;

    fn psk_signal(order: usize, carrier: f64, baud: f64, snr: f64) -> (Signal, Vec<usize>) {
        let p = PskParams {
            order,
            baud,
            carrier,
            sample_rate: 9600.0,
            amplitude: 0.4,
        };
        let sy = random_symbols(3000, order, 4);
        (add_awgn_snr(&psk(&p, &sy).unwrap(), snr, 2).unwrap(), sy)
    }

    #[test]
    fn eight_psk_resolves_into_eight_clusters() {
        let (s, _) = psk_signal(8, 1800.0, 1200.0, 25.0);
        let cfg = PhaseConfig {
            carrier: 1800.0,
            baud: 1200.0,
            order: 8,
            ..Default::default()
        };
        let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
        assert!(r.points.len() > 2900);
        // Points inside their 8-PSK decision sector (+/-20 of the 22.5 degrees).
        let inside = r
            .angles
            .iter()
            .skip(100)
            .filter(|a| {
                let e = *a - (*a / 45.0).round() * 45.0;
                e.abs() < 20.0
            })
            .count();
        let n = r.angles.len() - 100;
        assert!(
            inside as f32 > 0.98 * n as f32,
            "{inside} of {n} points in their sector"
        );
        assert_eq!(
            angle_histogram(&r.angles).iter().sum::<u32>() as usize,
            r.angles.len()
        );
        assert_eq!(rings(&r.points).len(), 1, "PSK has one ring");
        // QAM-16 points have three rings.
        let qam: Vec<Complex32> = (0..4000)
            .map(|k| {
                let lv = [-3.0_f32, -1.0, 1.0, 3.0];
                Complex32::new(lv[k % 4], lv[(k / 4) % 4]) / 10_f32.sqrt()
            })
            .collect();
        assert_eq!(rings(&qam).len(), 3);
    }

    #[test]
    fn small_carrier_error_is_tracked_and_reported() {
        let (s, _) = psk_signal(4, 1803.0, 1200.0, 25.0);
        let cfg = PhaseConfig {
            carrier: 1800.0,
            baud: 1200.0,
            order: 4,
            pll: PllSpeed::Quick,
            ..Default::default()
        };
        let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
        assert!(
            (r.carrier_offset - 3.0).abs() < 0.5,
            "offset {}",
            r.carrier_offset
        );
    }

    #[test]
    fn hard_loop_tracks_and_reports_carrier_error() {
        let (s, _) = psk_signal(4, 1803.0, 1200.0, 25.0);
        let cfg = PhaseConfig {
            carrier: 1800.0,
            baud: 1200.0,
            order: 4,
            pll: PllSpeed::Hard,
            ..Default::default()
        };
        let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
        assert!(
            (r.carrier_offset - 3.0).abs() < 0.5,
            "offset {}",
            r.carrier_offset
        );
    }

    #[test]
    fn loop_designs_are_stable_and_ordered() {
        let speeds = [
            PllSpeed::Slow,
            PllSpeed::Medium,
            PllSpeed::Quick,
            PllSpeed::Hard,
        ];
        let mut last = 0.0;
        for pll in speeds {
            let (kp, ki) = pll.gains();
            // z^2 + (kp + ki - 2) z + (1 - kp): both roots inside the unit circle.
            assert!(kp > 0.0 && kp < 2.0 && ki > 0.0 && 4.0 - 2.0 * kp - ki > 0.0);
            // Damping about 0.707: ki / kp^2 near 1 / (4 zeta^2) = 0.5.
            let ratio = ki / (kp * kp);
            assert!((0.5..0.7).contains(&ratio), "{pll:?}: ki/kp^2 {ratio}");
            assert!(pll.noise_bandwidth() > last, "{pll:?} not wider");
            last = pll.noise_bandwidth();
        }
    }

    #[test]
    fn carrier_offset_is_estimated_over_the_whole_track() {
        // 8-PSK, 1200 Bd, 2.4 Hz offset, 20 dB Es/N0: the final loop frequency of the
        // wide loops alone scatters by Hz; the slope estimate must not.
        let (carrier, baud, offset) = (2400.0, 1200.0, 2.4);
        let p = PskParams {
            order: 8,
            baud,
            carrier: carrier + offset,
            sample_rate: 9600.0,
            amplitude: 0.4,
        };
        for seed in 1..=4 {
            let sy = random_symbols(6000, 8, seed);
            let s = add_awgn_esn0(&psk(&p, &sy).unwrap(), 20.0, baud, seed).unwrap();
            for pll in [PllSpeed::Quick, PllSpeed::Hard] {
                let cfg = PhaseConfig {
                    carrier,
                    baud,
                    order: 8,
                    pll,
                    ..Default::default()
                };
                let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
                assert!(
                    (r.carrier_offset - offset).abs() < 0.05,
                    "{pll:?} seed {seed}: offset {}",
                    r.carrier_offset
                );
            }
        }
    }

    /// Real 8-PSK at `fs` with a Wiener phase noise of `step_deg` RMS per symbol.
    fn psk_with_phase_noise(
        sy: &[usize],
        carrier: f64,
        baud: f64,
        step_deg: f64,
        seed: u64,
    ) -> Signal {
        let fs = 9600.0;
        let order = 8.0;
        // Box-Muller on a small LCG: deterministic Gaussian steps.
        let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let mut uniform = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 11) as f64 + 0.5) / (1_u64 << 53) as f64
        };
        let mut phase = 0.0_f64;
        let phases: Vec<f64> = sy
            .iter()
            .map(|_| {
                let g = (-2.0 * uniform().ln()).sqrt() * (std::f64::consts::TAU * uniform()).cos();
                phase += g * step_deg.to_radians();
                phase
            })
            .collect();
        let sps = fs / baud;
        let n = (sy.len() as f64 * sps) as usize;
        let w = std::f64::consts::TAU * carrier / fs;
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let k = ((i as f64 / sps) as usize).min(sy.len() - 1);
                let ph = std::f64::consts::TAU * sy[k] as f64 / order + phases[k];
                (0.4 * (w * i as f64 + ph).cos()) as f32
            })
            .collect();
        Signal::real(x, fs).unwrap()
    }

    /// Relative-code errors against the transmitted differential symbols, at the best
    /// alignment, over the points after the first `skip`.
    fn relative_code_errors(r: &PhaseResult, sy: &[usize], order: u32, skip: usize) -> usize {
        let m = order as usize;
        (-4_i64..=4)
            .map(|lag| {
                r.relative_codes
                    .iter()
                    .enumerate()
                    .skip(skip)
                    .filter(|(k, c)| {
                        let j = *k as i64 + lag;
                        match (usize::try_from(j - 1), usize::try_from(j)) {
                            (Ok(a), Ok(b)) if b < sy.len() => {
                                (sy[b] + m - sy[a]) % m != **c as usize
                            }
                            _ => false,
                        }
                    })
                    .count()
            })
            .min()
            .unwrap()
    }

    #[test]
    fn hard_loop_follows_phase_noise_better_than_quick() {
        // 8-PSK with a 3 degree per symbol phase random walk at 22 dB Es/N0: the wider
        // Hard loop follows the phase better than Quick and must not slip.
        let (carrier, baud) = (2400.0, 600.0);
        let sy = random_symbols(6000, 8, 11);
        let clean = psk_with_phase_noise(&sy, carrier, baud, 3.0, 5);
        let s = add_awgn_esn0(&clean, 22.0, baud, 6).unwrap();
        let (mut rms, mut errs) = (Vec::new(), Vec::new());
        for pll in [PllSpeed::Quick, PllSpeed::Hard] {
            let cfg = PhaseConfig {
                carrier,
                baud,
                order: 8,
                pll,
                ..Default::default()
            };
            let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
            let tail = &r.angles[200..r.angles.len() - 20];
            let e = rms_angle_error(tail, 45.0);
            let errors = relative_code_errors(&r, &sy, 8, 200);
            println!("phase noise {pll:?}: RMS {e:.2} deg, {errors} relative code errors");
            if pll == PllSpeed::Hard {
                assert_eq!(errors, 0, "Hard slipped or decided wrongly");
            }
            rms.push(e);
            errs.push(errors);
        }
        assert!(
            rms[1] < rms[0],
            "Hard {} deg vs Quick {} deg",
            rms[1],
            rms[0]
        );
        assert!(
            errs[1] <= errs[0],
            "Hard {} errors vs Quick {}",
            errs[1],
            errs[0]
        );
    }

    #[test]
    fn hard_loop_on_noise_alone_stays_bounded() {
        let silent = Signal::real(vec![0.0_f32; 9600 * 2], 9600.0).unwrap();
        let s = sn_synth::add_awgn(&silent, 0.3, 9).unwrap();
        let cfg = PhaseConfig {
            carrier: 1800.0,
            baud: 2400.0,
            order: 8,
            pll: PllSpeed::Hard,
            ..Default::default()
        };
        let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
        assert!(r.points.len() > 1000);
        assert!(
            r.points
                .iter()
                .all(|p| p.re.is_finite() && p.im.is_finite())
        );
        assert!(r.carrier_offset.is_finite());
        assert!(
            r.carrier_offset.abs() <= 2400.0 / 16.0 + 1e-9,
            "offset {}",
            r.carrier_offset
        );
    }

    #[test]
    fn diff_makes_dqpsk_style_rotation_visible() {
        let pts: Vec<Complex32> = [0.0_f32, 45.0, 90.0, 135.0]
            .iter()
            .map(|d| Complex32::from_polar(1.0, d.to_radians()))
            .collect();
        let d1 = difference(&pts, 1);
        for p in d1.iter().skip(1) {
            assert!((p.arg().to_degrees() - 45.0).abs() < 1e-3);
        }
        assert_eq!(difference(&pts, 0), pts);
    }

    #[test]
    fn codes_and_bad_order() {
        let (s, _) = psk_signal(2, 1800.0, 1200.0, 30.0);
        let cfg = PhaseConfig {
            carrier: 1800.0,
            baud: 1200.0,
            order: 2,
            ..Default::default()
        };
        let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
        assert!(r.codes.iter().all(|c| *c < 2));
        assert_eq!(r.codes.len(), r.relative_codes.len());
        assert_ne!(eye(&r.trajectory, 100).len(), 0);
        assert!(run(&s, 0, s.len(), &PhaseConfig { order: 0, ..cfg }, &|| false).is_err());
    }

    /// RMS distance in degrees of the angles from the nearest multiple of `step`.
    fn rms_angle_error(angles: &[f32], step: f32) -> f32 {
        let n = angles.len().max(1) as f32;
        (angles
            .iter()
            .map(|a| {
                let e = a - (a / step).round() * step;
                e * e
            })
            .sum::<f32>()
            / n)
            .sqrt()
    }

    #[test]
    fn rectangular_eight_psk_angles_are_exact_without_noise() {
        // Carrier and baud well inside the band, so the real signal's spectral image
        // does not overlap the wanted sidelobes (that overlap cannot be filtered out).
        let (carrier, baud) = (2400.0, 300.0);
        let (s, _) = psk_signal(8, carrier, baud, 100.0);
        // Every speed must lock on a clean signal; the wide loops jitter a little more.
        for (pll, sync, bound) in [
            (PllSpeed::Slow, true, 1.0),
            (PllSpeed::Medium, true, 1.0),
            (PllSpeed::Quick, true, 3.0),
            (PllSpeed::Hard, true, 3.0),
        ] {
            let cfg = PhaseConfig {
                carrier,
                baud,
                order: 8,
                pll,
                clock_sync: sync,
                ..Default::default()
            };
            let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
            let tail = &r.angles[200..r.angles.len() - 20];
            let rms = rms_angle_error(tail, 45.0);
            println!("rect 8-PSK {pll:?} sync {sync}: RMS angle error {rms:.3} deg");
            assert!(
                rms < bound,
                "{pll:?} sync {sync}: RMS angle error {rms} deg"
            );
            let samples_per_symbol = 9600.0 / baud;
            assert!(
                (r.symbol_period - samples_per_symbol).abs() < 0.01,
                "symbol period {}",
                r.symbol_period
            );
        }
    }

    #[test]
    fn cancellation_and_eye_bounds() {
        let (s, _) = psk_signal(4, 1800.0, 1200.0, 30.0);
        let cfg = PhaseConfig {
            carrier: 1800.0,
            baud: 1200.0,
            order: 4,
            ..Default::default()
        };
        assert_eq!(
            run(&s, 0, s.len(), &cfg, &|| true).unwrap_err(),
            Error::Cancelled
        );
        let r = run(&s, 0, s.len(), &cfg, &|| false).unwrap();
        let all = eye(&r.trajectory, usize::MAX);
        assert_eq!(all.len(), r.trajectory.len() / SPS as usize - 1);
        let bad = PhaseConfig {
            baud: f64::NAN,
            ..cfg
        };
        assert!(matches!(
            run(&s, 0, s.len(), &bad, &|| false),
            Err(Error::Invalid { name: "baud", .. })
        ));
    }
}
