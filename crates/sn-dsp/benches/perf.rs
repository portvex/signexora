//! Wall-time benchmarks of the sn-dsp hot paths on realistic sizes.
//!
//! Run with `cargo bench -p sn-dsp --bench perf` (optionally followed by `-- <filter>`
//! to run only the cases whose name contains the filter). The shared harness in
//! `benches/common/bench.rs` runs each case once untimed and checks its output,
//! then times `RUNS` runs and prints the fastest, median and slowest wall time.

// A benchmark binary, not production code: setup failures and failed sanity
// checks may panic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

#[path = "../../../benches/common/bench.rs"]
mod bench;

use sn_core::{Complex32, SampleKind, Signal};
use sn_dsp::detect;
use sn_dsp::fir::Fir;
use sn_dsp::mix::mix_in_place;
use sn_dsp::overview::Overview;
use sn_dsp::resample::Resampler;
use sn_dsp::spectrum::{ColumnRequest, Combine, SpectrumEngine, sonogram_columns};
use sn_dsp::window::WindowKind;

use bench::{Bench, never};

/// Checks that an output holds `want` values.
fn check_len<T>(name: &str, want: usize) -> impl FnOnce(&Vec<T>) + '_ {
    move |v| assert_eq!(v.len(), want, "{name}: {} values, expected {want}", v.len())
}

/// Frequency in Hz and amplitude of the tone that [`audio`] puts over its noise.
const TONE_HZ: f64 = 1234.5;
const TONE_AMP: f32 = 0.5;

/// Checks that a spectrum column of [`audio`] is finite and shows its tone: the
/// loudest bin must lie within one bin width of [`TONE_HZ`] and at least 30 dB above
/// the median bin. A column of `DB_FLOOR`, permuted bins or a lost window scaling
/// all fail this. A complex engine shows a real tone at both +f and -f with equal
/// power, so the check compares the absolute frequency.
fn check_tone<'a>(
    name: &'a str,
    engine: &'a SpectrumEngine,
    fs: f64,
) -> impl FnOnce(&Vec<f32>) + 'a {
    move |v| {
        let bins = engine.bins();
        assert_eq!(v.len(), bins, "{name}: {} bins, expected {bins}", v.len());
        assert!(v.iter().all(|x| x.is_finite()), "{name}: non-finite bin");
        let (k, peak) = v
            .iter()
            .copied()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("non-empty column");
        let hz = engine.bin_frequency(k, fs).abs();
        let df = fs / engine.size() as f64;
        assert!(
            (hz - TONE_HZ).abs() <= df,
            "{name}: peak at {hz:.2} Hz (bin {k}), expected {TONE_HZ} Hz within {df:.3} Hz"
        );
        let mut sorted = v.clone();
        sorted.sort_by(f32::total_cmp);
        let median = sorted[sorted.len() / 2];
        assert!(
            peak - median >= 30.0,
            "{name}: peak {peak:.1} dB is only {:.1} dB above the median {median:.1} dB",
            peak - median
        );
    }
}

/// Pseudo-random real samples (an xorshift generator, so no extra dependency).
fn noise_real(n: usize, seed: u64) -> Vec<f32> {
    let mut s = seed | 1;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            ((s >> 40) as f32 / (1u64 << 24) as f32) - 0.5
        })
        .collect()
}

fn audio(seconds: f64) -> Signal {
    let fs = 48_000.0;
    let n = (seconds * fs) as usize;
    let mut v = noise_real(n, 7);
    for (i, x) in v.iter_mut().enumerate() {
        *x = 0.1 * *x
            + TONE_AMP * (std::f32::consts::TAU * TONE_HZ as f32 * i as f32 / 48_000.0).sin();
    }
    Signal::real(v, fs).unwrap()
}

fn iq(n: usize, fs: f64) -> Signal {
    let re = noise_real(n, 3);
    let im = noise_real(n, 5);
    let v: Vec<Complex32> = re
        .iter()
        .zip(&im)
        .map(|(a, b)| Complex32::new(*a, *b))
        .collect();
    Signal::complex(v, fs).unwrap()
}

// Case names, shared by the input guards in `main` and the cases themselves.
const SONO_A60: &str = "sonogram 60s@48k 1600 cols fft2048";
const SONO_RF: &str = "sonogram 2s@8M iq 1600 cols fft4096";
const COL_A60: &str = "column_db mean 60s@48k fft65536";
const COL_A20: &str = "column_db mean 20s@48k real fft65536";
const FIR_A60: &str = "fir lowpass 60s@48k";
const FIR_RF: &str = "fir bandpass 2s@8M iq";
const ENV_A20: &str = "detect envelope 20s@48k";
const MAM_A20: &str = "detect modified_am 20s@48k";
const RES_A60: &str = "resample 60s 48k->44.1k";
const RES_RF: &str = "resample 2s 8M->250k iq";
const MIX_RF: &str = "mix 16M samples";
const OVW_RF: &str = "overview 2s@8M iq";

fn main() {
    let Some(b) = Bench::from_args("sn-dsp") else {
        return;
    };
    // Build each input only when a selected case uses it: the 16M-sample IQ
    // recording alone takes a few seconds and 128 MB.
    let a60 = b
        .wanted(&[SONO_A60, COL_A60, FIR_A60, RES_A60])
        .then(|| audio(60.0));
    let a20 = b.wanted(&[COL_A20, ENV_A20, MAM_A20]).then(|| audio(20.0));
    // A 2 s slice of an 8 MS/s complex recording.
    let rf = b
        .wanted(&[SONO_RF, FIR_RF, RES_RF, MIX_RF, OVW_RF])
        .then(|| iq(16_000_000, 8e6));
    println!("rayon threads: {}", rayon::current_num_threads());

    if let Some(a60) = &a60 {
        // Sonogram: a 1600-column view of the whole 60 s file (zoomed out, peak hold).
        let e = SpectrumEngine::new(2048, WindowKind::default(), SampleKind::Real).unwrap();
        let req = ColumnRequest {
            start: 0.0,
            samples_per_column: a60.len() as f64 / 1600.0,
            hop: 512,
            max_frames_per_column: 64,
            combine: Combine::PeakHold,
        };
        b.run(
            SONO_A60,
            || sonogram_columns(&e, a60, &req, 0..1600, &never).unwrap(),
            check_len(SONO_A60, 1600),
        );
    }
    if let Some(rf) = &rf {
        let e8 = SpectrumEngine::new(4096, WindowKind::default(), SampleKind::Complex).unwrap();
        let req8 = ColumnRequest {
            start: 0.0,
            samples_per_column: rf.len() as f64 / 1600.0,
            hop: 2048,
            max_frames_per_column: 64,
            combine: Combine::Mean,
        };
        b.run(
            SONO_RF,
            || sonogram_columns(&e8, rf, &req8, 0..1600, &never).unwrap(),
            check_len(SONO_RF, 1600),
        );
    }

    // Averaged spectrum (used by Measurement of Freq and ADP): many frames, one column.
    if let Some(a60) = &a60 {
        let big = SpectrumEngine::new(1 << 16, WindowKind::Hann, SampleKind::Complex).unwrap();
        let frames: Vec<usize> = (0..a60.len() - (1 << 16)).step_by(1 << 15).collect();
        b.run(
            COL_A60,
            || big.column_db(a60, &frames, Combine::Mean, &never).unwrap(),
            check_tone(COL_A60, &big, a60.sample_rate()),
        );
    }
    if let Some(a20) = &a20 {
        let adp = SpectrumEngine::new(1 << 16, WindowKind::Hann, SampleKind::Real).unwrap();
        let room = a20.len() - (1 << 16);
        let frames20: Vec<usize> = (0..=room)
            .step_by(((1 << 15) as usize).max(room / 2047))
            .collect();
        b.run(
            COL_A20,
            || {
                adp.column_db(a20, &frames20, Combine::Mean, &never)
                    .unwrap()
            },
            check_tone(COL_A20, &adp, a20.sample_rate()),
        );
    }

    time_domain(&b, a60.as_ref(), a20.as_ref(), rf.as_ref());
}

/// FIR filtering, detectors, resampling, mixing and the waveform overview.
/// Each input is `None` when no selected case uses it.
fn time_domain(b: &Bench, a60: Option<&Signal>, a20: Option<&Signal>, rf: Option<&Signal>) {
    // FIR filtering.
    if let Some(a60) = a60.filter(|_| b.selects(FIR_A60)) {
        let lp = Fir::lowpass(3000.0, 200.0, 80.0, 48_000.0).unwrap();
        println!("  lowpass taps: {}", lp.len());
        b.run(
            FIR_A60,
            || lp.apply(a60, 0, a60.len(), &never).unwrap(),
            check_len(FIR_A60, a60.len()),
        );
    }
    if let Some(rf) = rf.filter(|_| b.selects(FIR_RF)) {
        let bp = Fir::complex_bandpass(-100e3, 100e3, 20e3, 80.0, 8e6).unwrap();
        println!("  8M bandpass taps: {}", bp.len());
        b.run(
            FIR_RF,
            || bp.apply(rf, 0, rf.len(), &never).unwrap(),
            check_len(FIR_RF, rf.len()),
        );
    }
    if let Some(a20) = a20 {
        b.run(
            ENV_A20,
            || detect::envelope(a20, &never).unwrap(),
            check_len(ENV_A20, a20.len()),
        );
        b.run(
            MAM_A20,
            || detect::modified_am(a20, &never).unwrap(),
            check_len(MAM_A20, a20.len()),
        );
    }

    // Resampling.
    if let Some(a60) = a60 {
        let r = Resampler::new(48_000.0, 44_100.0).unwrap();
        b.run(
            RES_A60,
            || r.apply(a60, 0, a60.len(), &never).unwrap(),
            check_len(RES_A60, r.output_len(a60.len())),
        );
    }
    let Some(rf) = rf else {
        return;
    };
    let dec = Resampler::new(8e6, 250e3).unwrap();
    b.run(
        RES_RF,
        || dec.apply(rf, 0, rf.len(), &never).unwrap(),
        check_len(RES_RF, dec.output_len(rf.len())),
    );

    // Mixing. The buffer starts at 1 + 0j, so after the untimed first run each
    // sample must equal the oscillator itself; the closure returns a few of them for
    // the check. The timed runs keep rotating the same buffer, which does not change
    // the work done, and their outputs are not checked.
    if b.selects(MIX_RF) {
        let n = rf.len();
        let (hz, fs) = (123_456.7, 8e6);
        // Samples to check: both ends and both sides of the first boundary between
        // the mixer's parallel blocks (1 << 14 samples each), where a wrong
        // per-block phase offset would show.
        let probes = [0, 1, (1 << 14) - 1, 1 << 14, (1 << 14) + 1, n / 2, n - 1];
        let mut buf = vec![Complex32::new(1.0, 0.0); n];
        b.run(
            MIX_RF,
            || {
                mix_in_place(&mut buf, 0, hz, fs);
                probes.map(|k| buf[k])
            },
            |got| {
                for (k, v) in probes.iter().zip(got) {
                    let ph = std::f64::consts::TAU * (*k as f64 * hz / fs).rem_euclid(1.0);
                    let want = Complex32::new(ph.cos() as f32, ph.sin() as f32);
                    assert!(
                        (v - want).norm() <= 1e-5,
                        "{MIX_RF}: sample {k} is {v}, expected {want}"
                    );
                }
            },
        );
    }

    // Waveform overview: one block per DEFAULT_BLOCK samples (the last may be
    // partial), each with an ordered envelope inside the noise range of [`iq`].
    b.run(
        OVW_RF,
        || Overview::build(rf, Overview::DEFAULT_BLOCK, &never).unwrap(),
        |o| {
            let block = Overview::DEFAULT_BLOCK;
            let want = rf.len().div_ceil(block);
            assert_eq!(
                o.blocks(),
                want,
                "{OVW_RF}: {} blocks, expected {want}",
                o.blocks()
            );
            for i in 0..want {
                let e = o.span(rf, i * block, (i + 1) * block);
                let im = e.im.expect("complex signal has an imaginary envelope");
                for (part, m) in [("re", e.re), ("im", im)] {
                    assert!(
                        -0.5 <= m.min && m.min <= m.max && m.max <= 0.5,
                        "{OVW_RF}: block {i} {part} envelope {}..{} out of order or range",
                        m.min,
                        m.max
                    );
                }
            }
        },
    );
}
