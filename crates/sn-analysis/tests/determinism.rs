//! Results must not depend on the number of worker threads: the parallel sums use
//! fixed block sizes and combine the blocks in order.

// Test helpers outside `#[test]` functions may unwrap too (project rule: tests may).
#![allow(clippy::unwrap_used)]

use sn_analysis::adp::estimate_baud;
use sn_analysis::histogram::Histogram;
use sn_analysis::line::{LineEstimate, measure_line};
use sn_core::{Complex32, Signal};
use sn_synth::{PskParams, add_awgn_snr, psk, random_symbols, tone};

fn never() -> bool {
    false
}

/// Runs `f` in a pool of one thread and in a pool of eight.
fn on_1_and_8_threads<T: Send>(f: impl Fn() -> T + Sync) -> (T, T) {
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    (one.install(&f), eight.install(&f))
}

fn bits(e: &LineEstimate) -> [u64; 4] {
    [
        e.frequency.to_bits(),
        e.level_db.to_bits(),
        e.snr_db.to_bits(),
        e.uncertainty.to_bits(),
    ]
}

#[test]
fn results_do_not_depend_on_the_thread_count() {
    let fs = 48_000.0;
    // 3e6 samples: many full-record blocks, and several locate blocks.
    let n: u32 = 3_000_000;
    let x: Vec<Complex32> = (0..n)
        .map(|k| {
            let ph = std::f64::consts::TAU * 1234.5678 * f64::from(k) / fs;
            Complex32::new((0.3 * ph.cos()) as f32, (0.3 * ph.sin()) as f32)
        })
        .collect();
    let complex = add_awgn_snr(&Signal::complex(x, fs).unwrap(), 10.0, 1).unwrap();
    let secs = f64::from(n) / fs;
    let near_zero = add_awgn_snr(&tone(3.3, 0.25, fs, secs).unwrap(), 10.0, 2).unwrap();
    let near_nyquist =
        add_awgn_snr(&tone(fs / 2.0 - 3.3, 0.25, fs, secs).unwrap(), 10.0, 3).unwrap();
    for (name, s, lo, hi) in [
        ("complex", &complex, 1000.0, 1500.0),
        ("real near 0 Hz", &near_zero, 0.0, 40.0),
        ("real near fs/2", &near_nyquist, fs / 2.0 - 40.0, fs / 2.0),
    ] {
        let (a, b) = on_1_and_8_threads(|| measure_line(s, 0, s.len(), lo, hi, &never).unwrap());
        assert_eq!(bits(&a), bits(&b), "{name}: {a:?} vs {b:?}");
    }

    let syms = random_symbols(20_000, 2, 5);
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
    let bpsk = add_awgn_snr(&bpsk, 10.0, 4).unwrap();
    let (a, b) = on_1_and_8_threads(|| estimate_baud(&bpsk, 0, bpsk.len(), &never).unwrap());
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(&b) {
        assert_eq!(p.baud.to_bits(), q.baud.to_bits(), "{p:?} vs {q:?}");
        assert_eq!(p.score.to_bits(), q.score.to_bits(), "{p:?} vs {q:?}");
        assert_eq!(p.detector, q.detector);
        assert_eq!(p.refined, q.refined);
        assert_eq!(p.resolution.to_bits(), q.resolution.to_bits());
    }

    for range in [None, Some((-1.0, 1.0))] {
        let (a, b) = on_1_and_8_threads(|| {
            Histogram::build(&complex, 0, complex.len(), 4096, range).unwrap()
        });
        assert_eq!(a.lo.to_bits(), b.lo.to_bits());
        assert_eq!(a.hi.to_bits(), b.hi.to_bits());
        assert_eq!(a.counts, b.counts);
    }
}
