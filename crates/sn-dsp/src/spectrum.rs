//! Windowed FFT spectra in dBFS and sonogram columns.

use std::sync::Arc;

use rayon::prelude::*;
use rustfft::num_complex::Complex32 as FftComplex;
use rustfft::{Fft, FftPlanner};
use sn_core::{Complex32, SampleKind, Signal};

use crate::window::WindowKind;
use crate::{Error, Result};

/// Smallest and largest supported FFT sizes.
pub const FFT_SIZE_RANGE: std::ops::RangeInclusive<usize> = 16..=(1 << 22);

/// Floor for dB values, so silence does not produce `-inf`.
pub const DB_FLOOR: f32 = -300.0;

/// How several FFT frames that fall into one display column are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Combine {
    /// Keep the strongest value of each bin, so short bursts stay visible when zoomed
    /// out.
    #[default]
    PeakHold,
    /// Average power, which smooths noise.
    Mean,
}

/// A reusable windowed-FFT spectrum calculator for one FFT size, window and signal kind.
#[derive(Clone)]
pub struct SpectrumEngine {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    size: usize,
    kind: SampleKind,
    /// Scale applied to `|X|` so a full-scale tone reads about 0 dBFS.
    amplitude_scale: f32,
    /// Time derivative of the window, for frequency reassignment (HRes).
    dwindow: Vec<f32>,
    /// Move each bin's power to the frequency its phase implies (sharper lines).
    reassign: bool,
    /// Equivalent noise bandwidth of the window in bins, `N sum(w^2) / (sum w)^2`.
    enbw: f32,
}

/// Per-thread work buffers for [`SpectrumEngine`], reused across frames so a column
/// does not allocate once per FFT.
#[derive(Debug, Default)]
struct Scratch {
    buf: Vec<FftComplex>,
    samples: Vec<Complex32>,
    deriv: Vec<FftComplex>,
    reassigned: Vec<f32>,
    power: Vec<f32>,
    /// FFT work space, so `process_with_scratch` does not allocate per frame.
    fft_work: Vec<FftComplex>,
}

impl std::fmt::Debug for SpectrumEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpectrumEngine")
            .field("size", &self.size)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl SpectrumEngine {
    /// Build an engine for `size`-point FFTs of `kind` signals.
    pub fn new(size: usize, window: WindowKind, kind: SampleKind) -> Result<Self> {
        if !FFT_SIZE_RANGE.contains(&size) {
            return Err(Error::InvalidParameter {
                name: "fft_size",
                reason: format!(
                    "{size} is outside {}..={}",
                    FFT_SIZE_RANGE.start(),
                    FFT_SIZE_RANGE.end()
                ),
            });
        }
        let coeffs = window.coefficients(size);
        let sum: f32 = coeffs.iter().sum();
        if !(sum.is_finite() && sum > 0.0) {
            return Err(Error::InvalidParameter {
                name: "window",
                reason: format!("{window} has no energy at size {size}"),
            });
        }
        // A real tone of amplitude A gives |X| = A * sum / 2 at its bin; a complex
        // exponential gives A * sum.
        let amplitude_scale = match kind {
            SampleKind::Real => 2.0 / sum,
            SampleKind::Complex => 1.0 / sum,
        };
        let sum_sq: f64 = coeffs.iter().map(|w| f64::from(*w) * f64::from(*w)).sum();
        let enbw = (size as f64 * sum_sq / (f64::from(sum) * f64::from(sum))) as f32;
        if !(enbw.is_finite() && enbw > 0.0) {
            return Err(Error::InvalidParameter {
                name: "window",
                reason: format!("{window} has no usable energy at size {size}"),
            });
        }
        let fft = FftPlanner::<f32>::new().plan_fft_forward(size);
        // Central-difference derivative of the window (per sample).
        let dwindow: Vec<f32> = (0..size)
            .map(|i| {
                let next = coeffs.get(i + 1).copied().unwrap_or(0.0);
                let prev = if i == 0 {
                    0.0
                } else {
                    coeffs.get(i - 1).copied().unwrap_or(0.0)
                };
                (next - prev) / 2.0
            })
            .collect();
        Ok(Self {
            fft,
            window: coeffs,
            size,
            kind,
            amplitude_scale,
            dwindow,
            reassign: false,
            enbw,
        })
    }

    /// Enable frequency reassignment (SA's HRes role): each bin's power is moved to
    /// the frequency estimated from the phase slope, which sharpens spectral lines to
    /// about one bin. Costs a second FFT per frame.
    ///
    /// A line collects the power of the whole main lobe, which is the window's
    /// equivalent noise bandwidth (ENBW) times its peak, so the result is divided by
    /// the ENBW and a full-scale tone still reads 0 dBFS. Noise is spread evenly, so
    /// the noise floor then reads 10 log10(ENBW) dB lower than without reassignment
    /// (noise power per bin width instead of per ENBW).
    #[must_use]
    pub fn with_reassignment(mut self, on: bool) -> Self {
        self.reassign = on;
        self
    }

    /// True when reassignment is on.
    pub fn reassigns(&self) -> bool {
        self.reassign
    }

    /// FFT size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Signal kind this engine was built for.
    pub fn kind(&self) -> SampleKind {
        self.kind
    }

    /// Number of output bins: `size / 2 + 1` for real signals (0..=fs/2), `size` for
    /// complex signals (-fs/2..fs/2, DC in the middle).
    pub fn bins(&self) -> usize {
        match self.kind {
            SampleKind::Real => self.size / 2 + 1,
            SampleKind::Complex => self.size,
        }
    }

    /// Centre frequency of output bin `k` in Hz (relative to the signal's 0 Hz).
    pub fn bin_frequency(&self, k: usize, sample_rate: f64) -> f64 {
        let df = sample_rate / self.size as f64;
        match self.kind {
            SampleKind::Real => k as f64 * df,
            SampleKind::Complex => (k as f64 - (self.size / 2) as f64) * df,
        }
    }

    /// Fractional output-bin position of frequency `hz` (inverse of
    /// [`Self::bin_frequency`]). May lie outside `0..bins()`.
    pub fn frequency_to_bin(&self, hz: f64, sample_rate: f64) -> f64 {
        let df = sample_rate / self.size as f64;
        match self.kind {
            SampleKind::Real => hz / df,
            SampleKind::Complex => hz / df + (self.size / 2) as f64,
        }
    }

    /// Lowest and highest displayable frequency in Hz.
    pub fn frequency_range(&self, sample_rate: f64) -> (f64, f64) {
        (
            self.bin_frequency(0, sample_rate),
            self.bin_frequency(self.bins().saturating_sub(1), sample_rate),
        )
    }

    /// Power spectrum (linear, `|X|^2` scaled) of the frame starting at `start`,
    /// left in `scratch.power`. Samples past the end of the signal count as zero.
    fn frame_power(&self, signal: &Signal, start: usize, scratch: &mut Scratch) {
        let Scratch {
            buf,
            samples,
            deriv,
            reassigned,
            power: out,
            fft_work,
        } = scratch;
        fft_work.resize(
            self.fft.get_inplace_scratch_len(),
            FftComplex::new(0.0, 0.0),
        );
        samples.clear();
        samples.resize(self.size, Complex32::new(0.0, 0.0));
        signal.read_complex(start, samples);
        buf.clear();
        buf.extend(
            samples
                .iter()
                .zip(&self.window)
                .map(|(s, w)| FftComplex::new(s.re * w, s.im * w)),
        );
        self.fft.process_with_scratch(buf, fft_work);
        let mut scale2 = self.amplitude_scale * self.amplitude_scale;
        if self.reassign {
            deriv.clear();
            deriv.extend(
                samples
                    .iter()
                    .zip(&self.dwindow)
                    .map(|(s, w)| FftComplex::new(s.re * w, s.im * w)),
            );
            self.fft.process_with_scratch(deriv, fft_work);
            reassign_in_place(buf, deriv, reassigned);
            scale2 /= self.enbw;
        }
        let power = |c: &FftComplex| (c.re * c.re + c.im * c.im) * scale2;
        out.clear();
        match self.kind {
            SampleKind::Real => {
                out.extend(buf.iter().take(self.bins()).map(power));
                // DC and (for even sizes) Nyquist have no mirror image, so the
                // factor 2 in the real amplitude scale does not apply there.
                if let Some(dc) = out.first_mut() {
                    *dc *= 0.25;
                }
                if self.size.is_multiple_of(2)
                    && let Some(ny) = out.get_mut(self.size / 2)
                {
                    *ny *= 0.25;
                }
            }
            SampleKind::Complex => {
                // fftshift: negative frequencies first. For odd sizes the first
                // output bin is -floor(N/2), which sits at FFT index (N + 1) / 2.
                let half = self.size.div_ceil(2);
                out.extend(
                    buf.iter()
                        .skip(half)
                        .chain(buf.iter().take(half))
                        .map(power),
                );
            }
        }
    }

    /// Spectrum in dBFS of the frame starting at `start`.
    pub fn frame_db(&self, signal: &Signal, start: usize) -> Vec<f32> {
        let mut scratch = Scratch::default();
        self.frame_power(signal, start, &mut scratch);
        scratch.power.into_iter().map(to_db).collect()
    }

    /// One display column in dBFS from the frames starting at `starts`, combined as
    /// requested. An empty `starts` gives a column of [`DB_FLOOR`].
    ///
    /// Frames are transformed in parallel, a batch at a time, and combined in the
    /// order of `starts`, so the result is the same as a serial pass. `should_stop` is
    /// checked before each batch (each frame when serial); once it returns true the
    /// call gives [`Error::Cancelled`].
    pub fn column_db(
        &self,
        signal: &Signal,
        starts: &[usize],
        combine: Combine,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<f32>> {
        // Each in-flight frame holds about six buffers of `size` values (including
        // the FFT scratch); keep the batch within PARALLEL_FRAME_BYTES.
        let per_frame = self.size.saturating_mul(6 * size_of::<FftComplex>()).max(1);
        let batch = rayon::current_num_threads()
            .min(PARALLEL_FRAME_BYTES / per_frame)
            .min(starts.len());
        if batch < 2 {
            return self.column_db_with(
                signal,
                starts,
                combine,
                &mut Scratch::default(),
                should_stop,
            );
        }
        let mut slots: Vec<Scratch> = (0..batch).map(|_| Scratch::default()).collect();
        let mut acc = vec![0.0_f32; self.bins()];
        for group in starts.chunks(batch) {
            if should_stop() {
                return Err(Error::Cancelled);
            }
            slots
                .par_iter_mut()
                .zip(group.par_iter())
                .for_each(|(slot, s)| self.frame_power(signal, *s, slot));
            for slot in slots.iter().take(group.len()) {
                accumulate(&mut acc, &slot.power, combine);
            }
        }
        Ok(finish_column(acc, starts.len(), combine))
    }

    /// Serial version of [`Self::column_db`] using the caller's buffers.
    fn column_db_with(
        &self,
        signal: &Signal,
        starts: &[usize],
        combine: Combine,
        scratch: &mut Scratch,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<f32>> {
        let mut acc = vec![0.0_f32; self.bins()];
        for &s in starts {
            if should_stop() {
                return Err(Error::Cancelled);
            }
            self.frame_power(signal, s, scratch);
            accumulate(&mut acc, &scratch.power, combine);
        }
        Ok(finish_column(acc, starts.len(), combine))
    }
}

/// Bytes of frame buffers [`SpectrumEngine::column_db`] keeps in flight at once.
const PARALLEL_FRAME_BYTES: usize = 64 << 20;

/// Fold one frame's power into a column accumulator.
fn accumulate(acc: &mut [f32], power: &[f32], combine: Combine) {
    for (a, v) in acc.iter_mut().zip(power) {
        match combine {
            Combine::PeakHold => *a = a.max(*v),
            Combine::Mean => *a += *v,
        }
    }
}

/// The accumulated column of `frames` frames in dBFS.
fn finish_column(mut acc: Vec<f32>, frames: usize, combine: Combine) -> Vec<f32> {
    if combine == Combine::Mean && frames > 0 {
        let n = frames as f32;
        for a in &mut acc {
            *a /= n;
        }
    }
    acc.into_iter().map(to_db).collect()
}

/// Replace `buf` (spectrum with window h) by a spectrum whose power is moved to the
/// reassigned bins: `k_hat = k - Im(Xdh / Xh) * N / (2 pi)`. Magnitudes are stored as
/// `sqrt(power)` in the real part so the caller's power formula still applies.
fn reassign_in_place(buf: &mut [FftComplex], d: &[FftComplex], acc: &mut Vec<f32>) {
    let n = buf.len();
    let nf = n as f32;
    acc.clear();
    acc.resize(n, 0.0);
    for (k, (x, dx)) in buf.iter().zip(d).enumerate() {
        let p = x.re * x.re + x.im * x.im;
        if p <= 0.0 {
            continue;
        }
        // Im(dx * conj(x)) / |x|^2 = Im(dx / x)
        let im = (dx.im * x.re - dx.re * x.im) / p;
        let shift = -im * nf / std::f32::consts::TAU;
        if !shift.is_finite() || shift.abs() > 4.0 {
            // Far-off estimates come from noise; keep the power where it is.
            if let Some(a) = acc.get_mut(k) {
                *a += p;
            }
            continue;
        }
        let target = (k as f32 + shift).round() as i64;
        let t = target.rem_euclid(n as i64) as usize;
        if let Some(a) = acc.get_mut(t) {
            *a += p;
        }
    }
    for (b, a) in buf.iter_mut().zip(acc.iter()) {
        *b = FftComplex::new(a.sqrt(), 0.0);
    }
}

fn to_db(power: f32) -> f32 {
    if power > 0.0 {
        (10.0 * power.log10()).max(DB_FLOOR)
    } else {
        DB_FLOOR
    }
}

/// Which frames make up one display column.
///
/// A column covers samples `start..start + span`. Frames start every `hop` samples
/// inside it, are centred on their position, and at most `max_frames` are used (evenly
/// spread) so heavily zoomed-out views stay fast.
pub fn column_frame_starts(
    start: f64,
    span: f64,
    fft_size: usize,
    hop: usize,
    max_frames: usize,
) -> Vec<usize> {
    let hop = hop.max(1) as f64;
    let wanted = (span / hop).round().max(1.0);
    let count = wanted.min(max_frames.max(1) as f64) as usize;
    let step = span / count as f64;
    let half = fft_size as f64 / 2.0;
    (0..count)
        .map(|i| {
            let centre = start + (i as f64 + 0.5) * step;
            (centre - half).max(0.0).round() as usize
        })
        .collect()
}

/// Settings for [`sonogram_columns`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnRequest {
    /// First sample of the first column (may be fractional).
    pub start: f64,
    /// Samples per display column.
    pub samples_per_column: f64,
    /// Hop between FFT frames in samples (SA's "Shear").
    pub hop: usize,
    /// Maximum FFT frames combined per column.
    pub max_frames_per_column: usize,
    /// How frames are combined.
    pub combine: Combine,
}

/// Compute display columns `first..first + count` in parallel.
///
/// Column `c` covers samples `start + c * samples_per_column` onward. Returns
/// [`Error::Cancelled`] as soon as `should_stop` returns true.
pub fn sonogram_columns(
    engine: &SpectrumEngine,
    signal: &Signal,
    request: &ColumnRequest,
    columns: std::ops::Range<i64>,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<(i64, Vec<f32>)>> {
    let len = signal.len() as f64;
    let out: Vec<Option<(i64, Vec<f32>)>> = columns
        .into_par_iter()
        .map_init(Scratch::default, |scratch, c| {
            if should_stop() {
                return None;
            }
            let col_start = request.start + c as f64 * request.samples_per_column;
            let col = if col_start < 0.0 || col_start >= len {
                vec![DB_FLOOR; engine.bins()]
            } else {
                let starts = column_frame_starts(
                    col_start,
                    request.samples_per_column,
                    engine.size(),
                    request.hop,
                    request.max_frames_per_column,
                );
                match engine.column_db_with(signal, &starts, request.combine, scratch, should_stop)
                {
                    Ok(col) => col,
                    Err(_) => return None,
                }
            };
            Some((c, col))
        })
        .collect();
    if should_stop() {
        return Err(Error::Cancelled);
    }
    out.into_iter().map(|c| c.ok_or(Error::Cancelled)).collect()
}

#[cfg(test)]
mod tests {
    use sn_synth::tone;

    use super::*;

    fn argmax(v: &[f32]) -> usize {
        v.iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap()
    }

    #[test]
    fn real_tone_peaks_at_its_bin_near_0_dbfs() {
        let s = tone(1000.0, 1.0, 8000.0, 1.0).unwrap();
        let e = SpectrumEngine::new(1024, WindowKind::Hann, SampleKind::Real).unwrap();
        let db = e.frame_db(&s, 0);
        assert_eq!(db.len(), 513);
        let k = argmax(&db);
        assert_eq!(k, 128);
        assert!((e.bin_frequency(k, 8000.0) - 1000.0).abs() < 1e-9);
        assert!(db[k].abs() < 0.1, "peak {} dBFS", db[k]);
        assert!(db[300] < -60.0);
    }

    #[test]
    fn complex_spectrum_is_centred_and_signed() {
        let fs = 48_000.0;
        let f = -6000.0;
        let iq: Vec<Complex32> = (0..4096)
            .map(|i| {
                let ph = std::f64::consts::TAU * f * f64::from(i) / fs;
                Complex32::new(ph.cos() as f32 * 0.5, ph.sin() as f32 * 0.5)
            })
            .collect();
        let s = Signal::complex(iq, fs).unwrap();
        let e = SpectrumEngine::new(512, WindowKind::default(), SampleKind::Complex).unwrap();
        let db = e.frame_db(&s, 0);
        let k = argmax(&db);
        assert!((e.bin_frequency(k, fs) - f).abs() < 1e-6);
        assert!((e.frequency_to_bin(f, fs) - k as f64).abs() < 1e-9);
        assert!((db[k] - 20.0 * 0.5_f32.log10()).abs() < 0.1);
        let (lo, hi) = e.frequency_range(fs);
        assert!((lo + 24_000.0).abs() < 1e-6 && hi < 24_000.0);
    }

    #[test]
    fn peak_hold_keeps_a_short_burst() {
        // 1 s of silence with a 10 ms burst at 0.5 s.
        let fs = 8000.0;
        let mut v = vec![0.0_f32; 8000];
        for (i, x) in v.iter_mut().enumerate().skip(4000).take(80) {
            *x = (std::f64::consts::TAU * 1500.0 * i as f64 / fs).sin() as f32;
        }
        let s = Signal::real(v, fs).unwrap();
        let e = SpectrumEngine::new(64, WindowKind::Hann, SampleKind::Real).unwrap();
        let starts = column_frame_starts(0.0, 8000.0, 64, 32, 1000);
        let peak = e
            .column_db(&s, &starts, Combine::PeakHold, &|| false)
            .unwrap();
        let mean = e.column_db(&s, &starts, Combine::Mean, &|| false).unwrap();
        let k = argmax(&peak);
        assert!(peak[k] > mean[k] + 15.0);
        assert!(peak[k] > -3.0);
    }

    #[test]
    fn parallel_column_matches_serial_bit_for_bit() {
        // 37 and 1000 frames are not multiples of the batch (3 or 4 threads), so the
        // short last group and slot reuse are both exercised.
        let mut rng = sn_synth::Rng::new(11);
        let v: Vec<f32> = (0..40_000).map(|_| rng.normal() as f32).collect();
        let s = Signal::real(v, 8000.0).unwrap();
        for reassign in [false, true] {
            let e = SpectrumEngine::new(256, WindowKind::Hann, SampleKind::Real)
                .unwrap()
                .with_reassignment(reassign);
            for threads in [3, 4] {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .unwrap();
                for n in [37_usize, 1000] {
                    let starts: Vec<usize> = (0..n).map(|i| (i * 37) % 39_000).collect();
                    for combine in [Combine::Mean, Combine::PeakHold] {
                        let par = pool
                            .install(|| e.column_db(&s, &starts, combine, &|| false))
                            .unwrap();
                        let ser = e
                            .column_db_with(&s, &starts, combine, &mut Scratch::default(), &|| {
                                false
                            })
                            .unwrap();
                        let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
                        assert_eq!(
                            bits(&par),
                            bits(&ser),
                            "{threads} threads, {n} frames, {combine:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn column_db_is_cancellable() {
        let s = tone(500.0, 0.5, 8000.0, 1.0).unwrap();
        let e = SpectrumEngine::new(256, WindowKind::Hann, SampleKind::Real).unwrap();
        let starts: Vec<usize> = (0..100).map(|i| i * 64).collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        assert_eq!(
            pool.install(|| e.column_db(&s, &starts, Combine::Mean, &|| true)),
            Err(Error::Cancelled)
        );
        assert_eq!(
            e.column_db(&s, &starts, Combine::Mean, &|| true),
            Err(Error::Cancelled)
        );
        // Stops part way through (after the fifth check), serial and in batches of 2.
        for threads in [1, 2] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            let calls = std::sync::atomic::AtomicUsize::new(0);
            let stop = || calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 5;
            assert_eq!(
                pool.install(|| e.column_db(&s, &starts, Combine::Mean, &stop)),
                Err(Error::Cancelled)
            );
        }
    }

    #[test]
    fn columns_are_computed_and_cancellable() {
        let s = tone(500.0, 0.5, 8000.0, 2.0).unwrap();
        let e = SpectrumEngine::new(256, WindowKind::default(), SampleKind::Real).unwrap();
        let req = ColumnRequest {
            start: 0.0,
            samples_per_column: 160.0,
            hop: 64,
            max_frames_per_column: 8,
            combine: Combine::PeakHold,
        };
        let cols = sonogram_columns(&e, &s, &req, -2..10, &|| false).unwrap();
        assert_eq!(cols.len(), 12);
        assert!(
            cols[0]
                .1
                .iter()
                .all(|v| (*v - DB_FLOOR).abs() < f32::EPSILON)
        );
        assert_eq!(argmax(&cols[5].1), 16);
        assert_eq!(
            sonogram_columns(&e, &s, &req, 0..10, &|| true),
            Err(Error::Cancelled)
        );
    }

    #[test]
    fn reassignment_sharpens_an_off_bin_tone() {
        // 1007.8 Hz at 8 kHz, FFT 256: 31.25 Hz bins, tone 0.25 bin off centre.
        let fs = 8000.0;
        let s = tone(1007.8, 1.0, fs, 1.0).unwrap();
        let plain = SpectrumEngine::new(256, WindowKind::Hann, SampleKind::Real).unwrap();
        let sharp = plain.clone().with_reassignment(true);
        let share = |e: &SpectrumEngine| {
            let p: Vec<f64> = e
                .frame_db(&s, 1000)
                .iter()
                .map(|d| 10_f64.powf(f64::from(*d) / 10.0))
                .collect();
            let total: f64 = p.iter().sum();
            p.iter().copied().fold(0.0, f64::max) / total
        };
        let (a, b) = (share(&plain), share(&sharp));
        assert!(b > 0.9, "reassigned peak holds {b:.2} of the power");
        assert!(b > a + 0.3, "plain {a:.2} vs reassigned {b:.2}");
        assert_eq!(argmax(&sharp.frame_db(&s, 1000)), 32);
    }

    #[test]
    fn reassignment_keeps_tones_at_0_dbfs() {
        let s = tone(1000.0, 1.0, 8000.0, 1.0).unwrap();
        for w in [WindowKind::Kaiser { beta: 8.0 }, WindowKind::FlatTop] {
            let e = SpectrumEngine::new(1024, w, SampleKind::Real)
                .unwrap()
                .with_reassignment(true);
            let db = e.frame_db(&s, 0);
            let k = argmax(&db);
            assert_eq!(k, 128);
            assert!(db[k].abs() < 0.1, "{w}: peak {} dBFS", db[k]);
        }
    }

    #[test]
    fn real_dc_and_nyquist_read_their_true_level() {
        let fs = 8000.0;
        let dc = Signal::real(vec![0.5_f32; 1024], fs).unwrap();
        let e = SpectrumEngine::new(1024, WindowKind::Hann, SampleKind::Real).unwrap();
        let db = e.frame_db(&dc, 0);
        assert!(
            (db[0] - 20.0 * 0.5_f32.log10()).abs() < 0.05,
            "DC {}",
            db[0]
        );
        let ny: Vec<f32> = (0..1024)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let ny = Signal::real(ny, fs).unwrap();
        let db = e.frame_db(&ny, 0);
        assert!(
            (db[512] - 20.0 * 0.5_f32.log10()).abs() < 0.05,
            "Nyquist {}",
            db[512]
        );
    }

    #[test]
    fn odd_complex_sizes_label_bins_correctly() {
        let fs = 1700.0;
        let n = 17;
        for f in [-800.0, -300.0, 0.0, 300.0, 800.0] {
            let iq: Vec<Complex32> = (0..n)
                .map(|i| {
                    let ph = std::f64::consts::TAU * f * f64::from(i) / fs;
                    Complex32::new(ph.cos() as f32, ph.sin() as f32)
                })
                .collect();
            let s = Signal::complex(iq, fs).unwrap();
            let e = SpectrumEngine::new(n as usize, WindowKind::Rectangular, SampleKind::Complex)
                .unwrap();
            let k = argmax(&e.frame_db(&s, 0));
            assert!(
                (e.bin_frequency(k, fs) - f).abs() < 1e-6,
                "{f} Hz read at {}",
                e.bin_frequency(k, fs)
            );
        }
    }

    #[test]
    fn rejects_bad_sizes() {
        assert!(SpectrumEngine::new(4, WindowKind::Hann, SampleKind::Real).is_err());
        assert!(SpectrumEngine::new(1 << 23, WindowKind::Hann, SampleKind::Real).is_err());
        let nan = WindowKind::Kaiser { beta: f64::NAN };
        assert!(SpectrumEngine::new(256, nan, SampleKind::Real).is_err());
    }

    #[test]
    fn frame_starts_are_spread_and_capped() {
        let s = column_frame_starts(10_000.0, 1000.0, 256, 100, 4);
        assert_eq!(s.len(), 4);
        assert_eq!(s[0], 10_000 + 125 - 128);
        let one = column_frame_starts(0.0, 10.0, 256, 512, 8);
        assert_eq!(one, vec![0]);
    }
}
