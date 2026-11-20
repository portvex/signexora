//! End-to-end tests of the FSK/MFSK demodulator on synthetic signals.

use super::{
    BitMap, Demodulated, Error, Levels, MAX_CLOCK_ERROR, MfskConfig, Polarity, Prefilter, SyncMode,
    demodulate, demodulate_track, detect_c4fm_inversion, eye_diagram, frequency_track,
    suggest_levels, symbols_to_bits,
};
use sn_core::{SampleUnit, Signal};
use sn_dsp::ops::Edit;
use sn_synth::{
    C4FM_DEVIATIONS, C4fmParams, MfskParams, Rng, add_awgn_snr, c4fm, c4fm_level_of_dibit, mfsk,
    random_symbols,
};

fn phase_detector(s: &Signal) -> Signal {
    Edit::PhaseDetector.run(s, &|| false).unwrap()
}

/// Symbol errors against the truth, matching each decision to the transmitted symbol
/// by its sampling instant. Skips `edge` symbols at both ends.
fn errors(
    d: &Demodulated,
    truth: &[usize],
    true_baud: f64,
    fs: f64,
    edge: usize,
) -> (usize, usize) {
    let mut errs = 0;
    let mut n = 0;
    for (s, t) in d.symbols.iter().zip(&d.instants) {
        let k = (t * true_baud / fs).floor() as usize;
        if k < edge || k + edge >= truth.len() {
            continue;
        }
        n += 1;
        if usize::from(*s) != truth[k] {
            errs += 1;
        }
    }
    (errs, n)
}

fn mfsk_case(
    order: usize,
    spacing: f64,
    snr_db: f64,
    seed: u64,
) -> (Signal, Vec<usize>, MfskParams) {
    let p = MfskParams {
        order,
        baud: 100.0,
        spacing,
        center: 1500.0,
        sample_rate: 8000.0,
        amplitude: 0.5,
    };
    let sym = random_symbols(3000, order, seed);
    let s = add_awgn_snr(&mfsk(&p, &sym).unwrap(), snr_db, seed + 1).unwrap();
    (phase_detector(&s), sym, p)
}

fn tones_of(p: &MfskParams) -> Vec<f64> {
    (0..p.order).map(|k| p.tone_frequency(k)).collect()
}

fn run(track: &Signal, cfg: &MfskConfig) -> Demodulated {
    demodulate(track, cfg, &|| false).unwrap()
}

#[test]
fn fsk2_moderate_snr() {
    let (track, sym, p) = mfsk_case(2, 200.0, 7.0, 11);
    assert_eq!(track.unit(), SampleUnit::Hertz);
    let mut cfg = MfskConfig::new(Levels::Explicit(tones_of(&p)), p.baud);
    cfg.filter = Prefilter::Rrc { alpha: 0.5 };
    let d = run(&track, &cfg);
    let (e, n) = errors(&d, &sym, p.baud, p.sample_rate, 10);
    let ser = e as f64 / n as f64;
    println!(
        "FSK-2 7 dB: SER {ser:.4} ({e}/{n}) quality {:.3} eye {:.3} clock {:+.5}",
        d.quality, d.eye_opening, d.clock_error
    );
    assert!(n > 2900);
    assert!(ser < 0.01);
    assert!(d.clock_error.abs() < 2e-4);
}

#[test]
fn mfsk8_and_mfsk16_moderate_snr() {
    for (order, snr) in [(8, 10.0), (16, 15.0)] {
        let (track, sym, p) = mfsk_case(order, 100.0, snr, 20 + order as u64);
        let mut cfg = MfskConfig::new(
            Levels::Grid {
                lo: p.tone_frequency(0),
                hi: p.tone_frequency(order - 1),
                count: order,
            },
            p.baud,
        );
        cfg.filter = Prefilter::Average { fraction: 0.5 };
        let d = run(&track, &cfg);
        let (e, n) = errors(&d, &sym, p.baud, p.sample_rate, 10);
        let ser = e as f64 / n as f64;
        println!(
            "MFSK-{order} {snr} dB: SER {ser:.4} ({e}/{n}) quality {:.3} eye {:.3}",
            d.quality, d.eye_opening
        );
        assert!(ser < 0.01, "MFSK-{order}: SER {ser}");
    }
}

#[test]
fn manual_sync_on_a_clean_signal() {
    let (track, sym, p) = mfsk_case(4, 200.0, 40.0, 5);
    let mut cfg = MfskConfig::new(Levels::Explicit(tones_of(&p)), p.baud);
    cfg.sync = SyncMode::Manual { phase: 40.0 };
    let d = run(&track, &cfg);
    let (e, n) = errors(&d, &sym, p.baud, p.sample_rate, 2);
    assert_eq!(e, 0, "{e}/{n}");
    assert!((d.timing_phase - 40.0).abs() < 1e-9);
    assert!(d.quality > 0.9);
}

#[test]
fn long_runs_do_not_break_sync() {
    let p = MfskParams {
        order: 2,
        baud: 100.0,
        spacing: 200.0,
        center: 1500.0,
        sample_rate: 8000.0,
        amplitude: 0.5,
    };
    let mut sym = random_symbols(300, 2, 1);
    sym.extend(std::iter::repeat_n(0, 700));
    sym.extend(random_symbols(300, 2, 2));
    sym.extend(std::iter::repeat_n(1, 900));
    sym.extend(random_symbols(300, 2, 3));
    let s = add_awgn_snr(&mfsk(&p, &sym).unwrap(), 7.0, 9).unwrap();
    let track = phase_detector(&s);
    let mut cfg = MfskConfig::new(Levels::Explicit(tones_of(&p)), p.baud);
    cfg.filter = Prefilter::Rrc { alpha: 0.5 };
    let d = run(&track, &cfg);
    let (e, n) = errors(&d, &sym, p.baud, p.sample_rate, 10);
    println!(
        "long runs: {e}/{n} errors, {} symbols out of {}",
        d.symbols.len(),
        sym.len()
    );
    // No slips: one decision per transmitted symbol.
    assert!(d.symbols.len().abs_diff(sym.len()) <= 1);
    assert!((e as f64) / (n as f64) < 0.01);
    // Every instant is still close to mid-symbol after the runs.
    let period = p.sample_rate / p.baud;
    for t in d.instants.iter().skip(2300) {
        let frac = (t / period).fract();
        assert!(
            (frac - 0.5).abs() < 0.2,
            "instant {t} at {frac} of a symbol"
        );
    }
}

/// Demodulate a 4-FSK signal sent `ppm` off the nominal 100 Bd and check that the
/// drift is followed: SER below 1 %, no slips, and the clock error measured.
fn check_clock_offset(ppm: f64, block_symbols: usize, max_clock_error: f64) {
    check_clock_offset_n(ppm, block_symbols, max_clock_error, 6000);
}

/// [`check_clock_offset`] with `n_sym` symbols.
fn check_clock_offset_n(ppm: f64, block_symbols: usize, max_clock_error: f64, n_sym: usize) {
    let true_baud = 100.0 * (1.0 + ppm * 1e-6);
    let p = MfskParams {
        order: 4,
        baud: true_baud,
        spacing: 200.0,
        center: 1500.0,
        sample_rate: 8000.0,
        amplitude: 0.5,
    };
    let sym = random_symbols(n_sym, 4, 77);
    let s = add_awgn_snr(&mfsk(&p, &sym).unwrap(), 10.0, 78).unwrap();
    let track = phase_detector(&s);
    let mut cfg = MfskConfig::new(Levels::Explicit(tones_of(&p)), 100.0);
    cfg.filter = Prefilter::Rrc { alpha: 0.5 };
    cfg.block_symbols = block_symbols;
    cfg.max_clock_error = max_clock_error;
    let d = run(&track, &cfg);
    let (e, n) = errors(&d, &sym, true_baud, p.sample_rate, 10);
    let ser = e as f64 / n as f64;
    println!(
        "clock {ppm:+} ppm, block {block_symbols}: measured {:+.6} ({:.4} Bd), SER {ser:.4} ({e}/{n})",
        d.clock_error, d.measured_baud
    );
    assert!(ser < 0.01, "{ppm} ppm, block {block_symbols}: SER {ser}");
    assert!(
        (d.clock_error - ppm * 1e-6).abs() < 1e-4,
        "{ppm} ppm, block {block_symbols}: clock error {}",
        d.clock_error
    );
    // Every symbol sent must be decided once: no slips.
    assert!(
        d.symbols.len().abs_diff(sym.len()) <= 2,
        "{} symbols decided, {} sent; first instant {:.1}, last {:.1}",
        d.symbols.len(),
        sym.len(),
        d.instants[0],
        d.instants[d.instants.len() - 1]
    );
}

#[test]
fn clock_offset_is_tracked() {
    for ppm in [1000.0, -1000.0] {
        check_clock_offset(ppm, 64, 0.005);
    }
}

#[test]
fn clock_offset_at_the_default_limit_is_tracked() {
    for ppm in [5000.0, -5000.0] {
        check_clock_offset(ppm, 64, 0.005);
    }
}

#[test]
fn long_blocks_follow_their_allowed_drift() {
    check_clock_offset(2000.0, 256, 0.002);
    check_clock_offset(-1000.0, 1024, 0.001);
}

#[test]
fn large_clock_errors_are_followed_with_long_blocks() {
    for block in [1024, 4096] {
        for ppm in [25_000.0, -25_000.0, 20_000.0] {
            check_clock_offset(ppm, block, MAX_CLOCK_ERROR);
        }
    }
    check_clock_offset(20_000.0, 2048, MAX_CLOCK_ERROR);
}

#[test]
fn one_percent_over_a_long_selection_is_followed() {
    check_clock_offset_n(10_000.0, 4096, MAX_CLOCK_ERROR, 20_000);
}

#[test]
fn drift_is_followed_on_a_selection_shorter_than_one_block() {
    // 1500 symbols with 4096-symbol blocks: the block is cut to the selection.
    for ppm in [20_000.0, -15_000.0, 25_000.0, -25_000.0, 10_000.0] {
        check_clock_offset_n(ppm, 4096, MAX_CLOCK_ERROR, 1500);
    }
}

/// A frequency track (Hz) with rectangular 4-level symbols whose levels drift and
/// compress slowly, plus Gaussian noise.
fn drifting_track(n_sym: usize, sps: usize, seed: u64) -> (Signal, Vec<usize>) {
    let sym = random_symbols(n_sym, 4, seed);
    let mut rng = Rng::new(seed + 100);
    let total = n_sym * sps;
    let mut v = Vec::with_capacity(total);
    for i in 0..total {
        let x = i as f64 / total as f64;
        let offset = 90.0 * (std::f64::consts::TAU * x).sin();
        let gain = 1.0 - 0.25 * x;
        let s = sym[i / sps] as f64;
        v.push((1000.0 + offset + gain * 100.0 * s + 12.0 * rng.normal()) as f32);
    }
    let sig = Signal::real(v, sps as f64 * 100.0)
        .unwrap()
        .with_unit(SampleUnit::Hertz);
    (sig, sym)
}

#[test]
fn adaptive_levels_follow_drift() {
    let (track, sym) = drifting_track(5000, 20, 3);
    let levels = Levels::Grid {
        lo: 1000.0,
        hi: 1300.0,
        count: 4,
    };
    let mut cfg = MfskConfig::new(levels, 100.0);
    let fixed = run(&track, &cfg);
    cfg.adaptive = true;
    let adaptive = run(&track, &cfg);
    let (ef, n) = errors(&fixed, &sym, 100.0, 2000.0, 10);
    let (ea, na) = errors(&adaptive, &sym, 100.0, 2000.0, 10);
    let (sf, sa) = (ef as f64 / n as f64, ea as f64 / na as f64);
    println!(
        "drift: fixed SER {sf:.4} quality {:.3}; adaptive SER {sa:.4} quality {:.3}; final levels {:?}",
        fixed.quality, adaptive.quality, adaptive.final_levels
    );
    assert!(sf > 0.05, "the drift should defeat fixed levels");
    assert!(sa < 0.01);
    assert!(adaptive.quality > fixed.quality);
}

#[test]
fn tone_grid_of_32_with_unused_tones() {
    let p = MfskParams {
        order: 32,
        baud: 50.0,
        spacing: 50.0,
        center: 2000.0,
        sample_rate: 8000.0,
        amplitude: 0.5,
    };
    let used = [0_usize, 1, 3, 7, 12, 20, 31];
    let mut rng = Rng::new(4);
    let sym: Vec<usize> = (0..2000).map(|_| used[rng.below(used.len())]).collect();
    let s = add_awgn_snr(&mfsk(&p, &sym).unwrap(), 20.0, 5).unwrap();
    let track = phase_detector(&s);

    // Levels found from the histogram, as SA's Count freqs would.
    let edge = track.len() / 20;
    let fine = Edit::Median { window: 41 }.run(&track, &|| false).unwrap();
    let suggestion = suggest_levels(&fine, edge, track.len() - edge, 4.0).unwrap();
    let Levels::Grid { lo, hi, count } = suggestion.grid else {
        panic!("expected a grid")
    };
    println!(
        "suggested grid: {count} levels {lo:.1}..{hi:.1} Hz from {:?}",
        suggestion.tones
    );
    assert_eq!(count, 32);
    assert!((lo - p.tone_frequency(0)).abs() < 5.0);
    assert!((hi - p.tone_frequency(31)).abs() < 5.0);

    let mut cfg = MfskConfig::new(suggestion.grid, p.baud);
    cfg.filter = Prefilter::Average { fraction: 0.5 };
    let d = run(&track, &cfg);
    let (e, n) = errors(&d, &sym, p.baud, p.sample_rate, 10);
    println!("32-tone grid: {e}/{n} errors, quality {:.3}", d.quality);
    assert!((e as f64) / (n as f64) < 0.01);
    for (k, c) in d.level_counts.iter().enumerate() {
        if !used.contains(&k) {
            assert!(*c <= 5, "unused level {k} got {c} decisions");
        }
    }
    let eye = eye_diagram(
        &frequency_track(
            &track,
            p.baud,
            Prefilter::Average { fraction: 0.5 },
            &|| false,
        )
        .unwrap(),
        &d.instants,
        p.sample_rate / p.baud,
        200,
        256,
    );
    assert_eq!(eye.traces.len(), 200);
    assert!(eye.points <= 256);
}

fn c4fm_case(levels: &[usize], snr_db: f64) -> Signal {
    let p = C4fmParams::default();
    let s = add_awgn_snr(&c4fm(&p, levels).unwrap(), snr_db, 31).unwrap();
    phase_detector(&s)
}

fn c4fm_dibits(n: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut d = Vec::with_capacity(n);
    while d.len() < n {
        if rng.below(40) == 0 {
            // APCO-25 always holds runs of zero bits (article 412).
            d.extend(std::iter::repeat_n(0_u8, 10));
        } else {
            d.push(rng.below(4) as u8);
        }
    }
    d.truncate(n);
    d
}

#[test]
fn c4fm_dibits_and_inversion() {
    let dibits = c4fm_dibits(4000, 8);
    let levels: Vec<usize> = dibits.iter().map(|d| c4fm_level_of_dibit(*d)).collect();
    let carrier = C4fmParams::default().carrier;
    let centres: Vec<f64> = C4FM_DEVIATIONS.iter().map(|d| carrier + d).collect();
    for inverted in [false, true] {
        let sent: Vec<usize> = if inverted {
            levels.iter().map(|l| 3 - l).collect()
        } else {
            levels.clone()
        };
        let track = c4fm_case(&sent, 15.0);
        let mut cfg = MfskConfig::new(Levels::Explicit(centres.clone()), 4800.0);
        cfg.filter = Prefilter::Rrc { alpha: 0.2 };
        let d = run(&track, &cfg);
        let (e, n) = errors(&d, &sent, 4800.0, 48_000.0, 10);
        let report = detect_c4fm_inversion(&d.symbols);
        println!(
            "C4FM inverted={inverted}: {e}/{n} symbol errors, quality {:.3}, eye {:.3}, {report:?}",
            d.quality, d.eye_opening
        );
        assert!((e as f64) / (n as f64) < 0.01);
        let expected = if inverted {
            Polarity::Inverted
        } else {
            Polarity::Normal
        };
        assert_eq!(report.polarity, expected);
        let map = if inverted {
            BitMap::c4fm_inverted()
        } else {
            BitMap::c4fm()
        };
        // Dibits out must equal dibits in (compare away from the edges).
        let bits = symbols_to_bits(&d.symbols, &map).unwrap();
        let first = (d.instants[0] * 4800.0 / 48_000.0).floor() as usize;
        let mut bit_errors = 0;
        let mut compared = 0;
        for (i, pair) in bits.chunks(2).enumerate().skip(10) {
            let k = first + i;
            if k + 10 >= dibits.len() {
                break;
            }
            compared += 1;
            if pair[0] * 2 + pair[1] != dibits[k] {
                bit_errors += 1;
            }
        }
        assert!(compared > 3900);
        assert!(
            bit_errors * 100 < compared,
            "{bit_errors}/{compared} dibit errors"
        );
    }
}

#[test]
fn rectangular_c4fm_without_filter() {
    let p = C4fmParams {
        shaped: false,
        ..C4fmParams::default()
    };
    let sym = random_symbols(3000, 4, 12);
    let s = add_awgn_snr(&c4fm(&p, &sym).unwrap(), 25.0, 13).unwrap();
    let track = phase_detector(&s);
    let centres: Vec<f64> = C4FM_DEVIATIONS.iter().map(|d| p.carrier + d).collect();
    let d = run(&track, &MfskConfig::new(Levels::Explicit(centres), 4800.0));
    let (e, n) = errors(&d, &sym, 4800.0, 48_000.0, 10);
    println!(
        "rectangular C4FM, no filter: {e}/{n}, quality {:.3}",
        d.quality
    );
    assert!((e as f64) / (n as f64) < 0.01);
}

#[test]
fn rejects_bad_input() {
    let iq = Signal::complex(vec![sn_core::Complex32::new(0.0, 0.0); 1000], 8000.0).unwrap();
    let cfg = MfskConfig::new(Levels::Explicit(vec![0.0, 1.0]), 100.0);
    assert!(demodulate(&iq, &cfg, &|| false).is_err());
    let short = Signal::real(vec![0.0_f32; 100], 8000.0).unwrap();
    assert!(demodulate(&short, &cfg, &|| false).is_err());
    let fast = MfskConfig::new(Levels::Explicit(vec![0.0, 1.0]), 6000.0);
    let long = Signal::real(vec![0.0_f32; 10_000], 8000.0).unwrap();
    let err = demodulate(&long, &fast, &|| false).unwrap_err();
    assert!(err.to_string().contains("samples per symbol"), "{err}");
}

#[test]
fn cancellation_is_reported() {
    let (track, _, p) = mfsk_case(2, 200.0, 20.0, 3);
    let mut cfg = MfskConfig::new(Levels::Explicit(tones_of(&p)), p.baud);
    assert_eq!(
        demodulate(&track, &cfg, &|| true).unwrap_err(),
        Error::Cancelled
    );
    cfg.sync = SyncMode::Manual { phase: 40.0 };
    assert_eq!(
        demodulate(&track, &cfg, &|| true).unwrap_err(),
        Error::Cancelled
    );
}

fn invalid_name(r: Result<Demodulated, Error>) -> &'static str {
    match r {
        Err(Error::Invalid { name, .. }) => name,
        other => panic!("expected an invalid setting, got {other:?}"),
    }
}

#[test]
fn out_of_range_settings_are_named() {
    let long = Signal::real(vec![0.0_f32; 10_000], 8000.0).unwrap();
    let base = MfskConfig::new(Levels::Explicit(vec![0.0, 1.0]), 100.0);
    let cases: Vec<(&str, MfskConfig)> = vec![
        (
            "adapt_rate",
            MfskConfig {
                adapt_rate: 0.9,
                ..base.clone()
            },
        ),
        (
            "block_symbols",
            MfskConfig {
                block_symbols: 8,
                ..base.clone()
            },
        ),
        (
            "max_clock_error",
            MfskConfig {
                max_clock_error: MAX_CLOCK_ERROR * 2.0,
                ..base.clone()
            },
        ),
        (
            "alpha",
            MfskConfig {
                filter: Prefilter::Rrc { alpha: 0.0 },
                ..base.clone()
            },
        ),
        (
            "fraction",
            MfskConfig {
                filter: Prefilter::Average { fraction: 1.5 },
                ..base.clone()
            },
        ),
        (
            "baud",
            MfskConfig {
                baud: f64::NAN,
                ..base.clone()
            },
        ),
    ];
    for (name, cfg) in cases {
        assert_eq!(invalid_name(demodulate(&long, &cfg, &|| false)), name);
    }
    // A raw track with a bad sample rate.
    let track = vec![0.0_f32; 10_000];
    for fs in [f64::NAN, 0.0, -8000.0, f64::INFINITY] {
        assert_eq!(
            invalid_name(demodulate_track(&track, fs, &base, &|| false)),
            "sample_rate"
        );
    }
}

#[test]
fn frequency_track_checks_its_settings() {
    let long = Signal::real(vec![0.0_f32; 10_000], 8000.0).unwrap();
    let name = |baud: f64, filter: Prefilter| match frequency_track(&long, baud, filter, &|| false)
    {
        Err(Error::Invalid { name, .. }) => name,
        other => panic!("expected an invalid setting, got {other:?}"),
    };
    assert_eq!(
        name(100.0, Prefilter::Average { fraction: 0.0 }),
        "fraction"
    );
    assert_eq!(
        name(100.0, Prefilter::Average { fraction: f64::NAN }),
        "fraction"
    );
    assert_eq!(name(f64::NAN, Prefilter::None), "baud");
    assert_eq!(name(100.0, Prefilter::Rrc { alpha: 2.0 }), "alpha");
    assert!(
        frequency_track(&long, 100.0, Prefilter::Average { fraction: 0.5 }, &|| {
            false
        })
        .is_ok()
    );
}

#[test]
fn manual_phase_outside_the_track_is_refused() {
    let (track, _, p) = mfsk_case(2, 200.0, 20.0, 3);
    let mut cfg = MfskConfig::new(Levels::Explicit(tones_of(&p)), p.baud);
    cfg.sync = SyncMode::Manual {
        phase: track.len() as f64 + 10.0,
    };
    assert_eq!(invalid_name(demodulate(&track, &cfg, &|| false)), "phase");
    cfg.sync = SyncMode::Manual { phase: -1e9 };
    assert_eq!(invalid_name(demodulate(&track, &cfg, &|| false)), "phase");
}

#[test]
fn non_finite_samples_are_refused() {
    let mut v = vec![1000.0_f32; 10_000];
    v[1234] = f32::NAN;
    let cfg = MfskConfig::new(Levels::Explicit(vec![900.0, 1100.0]), 100.0);
    let err = demodulate_track(&v, 8000.0, &cfg, &|| false).unwrap_err();
    assert!(
        matches!(&err, Error::Invalid { name: "signal", reason } if reason.contains("1234")),
        "{err}"
    );
}

/// Largest distance, in symbols, between a decision instant and the centre of the
/// symbol it samples, on a clean 4-FSK selection of `n_sym` symbols sent `ppm` off
/// the nominal 100 Bd. Also returns the symbol errors.
fn short_selection_timing(n_sym: usize, ppm: f64) -> (f64, usize) {
    let true_baud = 100.0 * (1.0 + ppm * 1e-6);
    let p = MfskParams {
        order: 4,
        baud: true_baud,
        spacing: 200.0,
        center: 1500.0,
        sample_rate: 8000.0,
        amplitude: 0.5,
    };
    let sym = random_symbols(n_sym, 4, 91);
    let s = add_awgn_snr(&mfsk(&p, &sym).unwrap(), 40.0, 92).unwrap();
    let track = phase_detector(&s);
    let mut cfg = MfskConfig::new(Levels::Explicit(tones_of(&p)), 100.0);
    cfg.max_clock_error = MAX_CLOCK_ERROR;
    let d = run(&track, &cfg);
    let sps = p.sample_rate / true_baud;
    let mut worst = 0.0_f64;
    for t in &d.instants {
        let x = t / sps;
        worst = worst.max((x - x.floor() - 0.5).abs());
    }
    let (e, _) = errors(&d, &sym, true_baud, p.sample_rate, 0);
    (worst, e)
}

#[test]
fn drift_is_followed_on_very_short_selections() {
    for n_sym in [10, 11, 20, 23] {
        for ppm in [20_000.0, -20_000.0] {
            let (worst, e) = short_selection_timing(n_sym, ppm);
            println!("{n_sym} symbols, {ppm:+} ppm: worst instant {worst:.3} symbol, {e} errors");
            assert!(
                worst < 0.08,
                "{n_sym} symbols, {ppm} ppm: instant {worst} symbol off"
            );
            assert_eq!(e, 0, "{n_sym} symbols, {ppm} ppm");
        }
    }
}
