//! Wall-time benchmarks of the sn-demod hot paths on realistic sizes.
//!
//! Run with `cargo bench -p sn-demod --bench perf` (optionally followed by
//! `-- <filter>`). The shared harness in `benches/common/bench.rs` runs each case
//! once untimed: that run warms up caches and the thread pool, and its output is
//! checked so that a broken demodulation cannot be reported as a speed-up. The case
//! then runs `RUNS` times and prints the fastest, median and slowest wall time.
//! Setup for a group of cases is skipped when the filter excludes every case in it.

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

use sn_core::{Complex32, Signal};
use sn_demod::mfsk::{Demodulated, Levels, MfskConfig, Prefilter, demodulate};
use sn_demod::phase::{self, PhaseConfig, PhaseResult, PllSpeed};
use sn_dsp::detect;
use sn_synth::{MfskParams, PskParams, add_awgn_snr, mfsk, psk, random_symbols};

use bench::{Bench, never};

/// Symbol error rate of `got` against `sent`, at the best alignment within a few
/// symbols (the demodulator may skip a partial symbol at either end).
fn symbol_error_rate(got: &[u8], sent: &[usize]) -> f64 {
    (-2_isize..=2)
        .map(|lag| {
            let pairs = got.iter().enumerate().filter_map(|(i, &g)| {
                let j = i.checked_add_signed(lag)?;
                sent.get(j).map(|&s| (g, s))
            });
            let (n, errors) = pairs.fold((0_usize, 0_usize), |(n, e), (g, s)| {
                (n + 1, e + usize::from(usize::from(g) != s))
            });
            if n == 0 {
                1.0
            } else {
                errors as f64 / n as f64
            }
        })
        .fold(1.0, f64::min)
}

fn check_mfsk<'a>(name: &'a str, sent: &'a [usize]) -> impl FnOnce(&Demodulated) + 'a {
    move |d| {
        let n = d.symbols.len();
        assert!(
            n.abs_diff(sent.len()) <= 1,
            "{name}: decided {n} symbols, expected {}",
            sent.len()
        );
        let ser = symbol_error_rate(&d.symbols, sent);
        assert!(ser < 1e-3, "{name}: symbol error rate {ser:.4} is too high");
    }
}

/// M-th power coherence of a constellation: |mean(p^M)| / mean(|p|^M). Close to 1
/// for a locked M-PSK constellation, close to 0 for a rotating or noisy one.
fn psk_coherence(points: &[Complex32], order: u32) -> f64 {
    let mut sum = Complex32::new(0.0, 0.0);
    let mut mag = 0.0_f64;
    for &p in points {
        let mut z = Complex32::new(1.0, 0.0);
        for _ in 0..order {
            z *= p;
        }
        sum += z;
        mag += f64::from(z.norm());
    }
    if mag > 0.0 {
        f64::from(sum.norm()) / mag
    } else {
        0.0
    }
}

fn check_phase(name: &str, symbols: usize, order: u32) -> impl FnOnce(&PhaseResult) + '_ {
    move |r| {
        let n = r.points.len();
        assert!(
            n.abs_diff(symbols) <= 1,
            "{name}: {n} symbol points, expected {symbols}"
        );
        let c = psk_coherence(&r.points, order);
        assert!(
            c > 0.7,
            "{name}: constellation coherence {c:.3}, not locked"
        );
    }
}

fn main() {
    let Some(b) = Bench::from_args("sn-demod") else {
        return;
    };
    let fs = 48_000.0;

    // 60 s of 8-FSK at 100 Bd at 15 dB SNR, demodulated from its phase-detector
    // track. Both cases use a prefilter: without one each decision sees a single
    // unfiltered track sample with noise over the full 24 kHz, and fails.
    let mfsk_avg = "mfsk demod 60s@48k avg";
    let mfsk_rrc = "mfsk demod 60s@48k rrc adaptive";
    if b.wanted(&[mfsk_avg, mfsk_rrc]) {
        let p = MfskParams {
            order: 8,
            baud: 100.0,
            spacing: 200.0,
            center: 2000.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let sent = random_symbols(6000, 8, 1);
        let s = add_awgn_snr(&mfsk(&p, &sent).unwrap(), 15.0, 2).unwrap();
        let z = detect::analytic(&s, &never).unwrap();
        let track = Signal::real(detect::inst_freq(&z, fs), fs).unwrap();
        let levels = Levels::Grid {
            lo: 1300.0,
            hi: 2700.0,
            count: 8,
        };
        let mut avg = MfskConfig::new(levels.clone(), p.baud);
        avg.filter = Prefilter::Average { fraction: 0.5 };
        b.run(
            mfsk_avg,
            || demodulate(&track, &avg, &never).unwrap(),
            check_mfsk(mfsk_avg, &sent),
        );
        let mut adaptive = MfskConfig::new(levels, p.baud);
        adaptive.adaptive = true;
        adaptive.filter = Prefilter::Rrc { alpha: 0.5 };
        b.run(
            mfsk_rrc,
            || demodulate(&track, &adaptive, &never).unwrap(),
            check_mfsk(mfsk_rrc, &sent),
        );
    }

    // 60 s of QPSK at 2400 Bd through PhasePlane. Every PhaseConfig field is set
    // here (carrier, baud and order from the synthesis parameters, a medium loop,
    // clock sync on, no differential display) so a change to
    // PhaseConfig::default() cannot change what is timed or break the lock check.
    let pp_plain = "phaseplane 60s@48k qpsk";
    let pp_rrc = "phaseplane 60s@48k qpsk rrc";
    if b.wanted(&[pp_plain, pp_rrc]) {
        let pp = PskParams {
            order: 4,
            baud: 2400.0,
            carrier: 1800.0,
            sample_rate: fs,
            amplitude: 0.5,
        };
        let symbols = 60 * 2400;
        let q = psk(&pp, &random_symbols(symbols, 4, 3)).unwrap();
        let q = add_awgn_snr(&q, 20.0, 4).unwrap();
        let order = u32::try_from(pp.order).unwrap();
        let cfg = PhaseConfig {
            carrier: pp.carrier,
            baud: pp.baud,
            order,
            pll: PllSpeed::Medium,
            clock_sync: true,
            rrc_alpha: None,
            diff: 0,
        };
        b.run(
            pp_plain,
            || phase::run(&q, 0, q.len(), &cfg, &never).unwrap(),
            check_phase(pp_plain, symbols, order),
        );
        let rrc = PhaseConfig {
            rrc_alpha: Some(0.35),
            ..cfg
        };
        b.run(
            pp_rrc,
            || phase::run(&q, 0, q.len(), &rrc, &never).unwrap(),
            check_phase(pp_rrc, symbols, order),
        );
    }
}
