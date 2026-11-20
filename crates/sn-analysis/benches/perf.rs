//! Wall-time benchmarks of the sn-analysis hot paths on realistic sizes.
//!
//! Run with `cargo bench -p sn-analysis --bench perf` (optionally followed by
//! `-- <filter>`). The shared harness in `benches/common/bench.rs` runs each case
//! once untimed and checks its output, then times `RUNS` runs and prints the
//! fastest, median and slowest wall time.

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

use sn_analysis::adp::estimate_baud;
use sn_analysis::histogram::Histogram;
use sn_analysis::line::{LineEstimate, measure_line};
use sn_analysis::raster::{Raster, RasterRequest, build};
use sn_core::{Complex32, Signal};
use sn_synth::{PskParams, add_awgn_snr, psk, random_symbols, tone};

use bench::{Bench, never};

/// Checks that a measured line lies within `tol` Hz of `want`.
fn check_line(name: &str, want: f64, tol: f64) -> impl FnOnce(&LineEstimate) + '_ {
    move |e| {
        assert!(
            (e.frequency - want).abs() <= tol,
            "{name}: line at {:.3} Hz, expected {want:.3} Hz",
            e.frequency
        );
    }
}

/// Checks that a histogram has `bins` bins and counted something.
fn check_histogram(name: &str, bins: usize) -> impl FnOnce(&Histogram) + '_ {
    move |h| {
        assert_eq!(h.counts.len(), bins, "{name}: wrong number of bins");
        assert!(
            h.counts.iter().any(|&c| c > 0),
            "{name}: every bin is empty"
        );
    }
}

/// 2 s of a complex tone at 12 345.678 Hz sampled at 8 MS/s, at 0 dB SNR.
fn iq_2s() -> Signal {
    let n_iq = 1 << 24;
    let iq: Vec<Complex32> = (0..n_iq)
        .map(|i| {
            let ph = std::f64::consts::TAU * 12_345.678 * f64::from(i) / 8e6;
            Complex32::new(ph.cos() as f32 * 0.3, ph.sin() as f32 * 0.3)
        })
        .collect();
    add_awgn_snr(&Signal::complex(iq, 8e6).unwrap(), 0.0, 2).unwrap()
}

/// 20 s of BPSK at 1200 Bd on an 1800 Hz carrier, at 10 dB SNR.
fn bpsk_20s(fs: f64) -> Signal {
    let syms = random_symbols(20 * 1200, 2, 3);
    let bpsk = psk(
        &PskParams {
            order: 2,
            baud: 1200.0,
            carrier: 1800.0,
            sample_rate: fs,
            amplitude: 0.5,
        },
        &syms,
    )
    .unwrap();
    add_awgn_snr(&bpsk, 10.0, 4).unwrap()
}

fn main() {
    let Some(b) = Bench::from_args("sn-analysis") else {
        return;
    };
    let fs = 48_000.0;
    // Each input is built only when a case that uses it passes the filter.
    let line_t60 = "measure_line 60s@48k real";
    let line_iq = "measure_line 2s@8M iq";
    let baud_bpsk = "estimate_baud 20s@48k bpsk";
    let hist_t60 = "histogram 60s@48k auto range";
    let hist_iq = "histogram 2s@8M fixed range";
    let raster_t60 = "raster 2300x4096 60s@48k";

    let t60 = b
        .wanted(&[line_t60, hist_t60, raster_t60])
        .then(|| add_awgn_snr(&tone(1234.567, 0.5, fs, 60.0).unwrap(), 0.0, 1).unwrap());
    let iq = b.wanted(&[line_iq, hist_iq]).then(iq_2s);
    let bpsk = b.wanted(&[baud_bpsk]).then(|| bpsk_20s(fs));

    if let Some(t60) = &t60 {
        b.run(
            line_t60,
            || measure_line(t60, 0, t60.len(), 1200.0, 1300.0, &never).unwrap(),
            check_line(line_t60, 1234.567, 0.5),
        );
    }
    if let Some(iq) = &iq {
        b.run(
            line_iq,
            || measure_line(iq, 0, iq.len(), 10e3, 15e3, &never).unwrap(),
            check_line(line_iq, 12_345.678, 5.0),
        );
    }
    if let Some(bpsk) = &bpsk {
        b.run(
            baud_bpsk,
            || estimate_baud(bpsk, 0, bpsk.len(), &never).unwrap(),
            |found| {
                assert!(
                    found.iter().any(|c| (c.baud - 1200.0).abs() < 12.0),
                    "{baud_bpsk}: no candidate near 1200 Bd in {:?}",
                    found.iter().map(|c| c.baud).collect::<Vec<_>>()
                );
            },
        );
    }
    if let Some(t60) = &t60 {
        b.run(
            hist_t60,
            || Histogram::build(t60, 0, t60.len(), 4096, None).unwrap(),
            check_histogram(hist_t60, 4096),
        );
    }
    if let Some(iq) = &iq {
        b.run(
            hist_iq,
            || Histogram::build(iq, 0, iq.len(), 4096, Some((-1.0, 1.0))).unwrap(),
            check_histogram(hist_iq, 4096),
        );
    }
    if let Some(t60) = &t60 {
        let req = RasterRequest {
            start: 0.0,
            period: 1234.5,
            strobe: 1,
            rows: 2300,
            cols: 4096,
        };
        b.run(
            raster_t60,
            || build(t60, &req).unwrap(),
            |r: &Raster| {
                assert_eq!(
                    (r.rows, r.cols),
                    (2300, 4096),
                    "{raster_t60}: wrong raster size"
                );
                assert_eq!(
                    r.values.len(),
                    r.rows * r.cols,
                    "{raster_t60}: wrong value count"
                );
            },
        );
    }
}
