//! End-to-end OFDM analysis on synthetic signals with known parameters
//! (development plan, section 5: OFDM fixtures and golden assertions).

// Test helpers outside `#[test]` functions may unwrap too (project rule: tests may).
#![allow(clippy::unwrap_used)]

use num_complex::Complex64;
use sn_core::{Complex32, Signal};
use sn_ofdm::*;
use sn_synth::{OfdmModulation, OfdmParams, add_awgn, add_awgn_snr, ofdm, ofdm_real};

fn never() -> bool {
    false
}

fn base(fs: f64, lu: usize, lg: usize, channels: usize, first_bin: i64) -> OfdmParams {
    OfdmParams {
        sample_rate: fs,
        lu,
        lg,
        channels,
        first_bin,
        unused: vec![],
        modulation: OfdmModulation::Qpsk,
        overrides: vec![],
        symbols: 400,
        frequency_offset: 0.0,
        amplitude: 0.3,
        seed: 1,
    }
}

fn resample(s: &Signal, rate: f64) -> Signal {
    resample_to_rate(s, rate, &never).unwrap()
}

fn full(s: &Signal) -> Baseband {
    Baseband::from_signal(s, 0, s.len(), &never).unwrap()
}

struct Run {
    bb: Baseband,
    ct: CtResult,
    br: BrEstimate,
}

fn analyse(s: &Signal, min_br: f64, max_br: f64) -> Run {
    let bb = full(s);
    let ct = find_ct(&bb, &CtParams::new(min_br, max_br), &never).unwrap();
    let br = get_br(&bb, &ct, &BrParams::default(), &never).unwrap();
    println!(
        "CT: LS {} LU {} LG {} start {} coherence {:.3} margin {:.2} confidence {:.2}",
        ct.ls, ct.lu, ct.lg, ct.start, ct.triangle.coherence, ct.margin, ct.confidence
    );
    println!(
        "Br: {:.6} Bd  LS {:.5} LU {:.4} K {:.5} offset {:.3} Hz start {:.2}",
        br.br, br.ls, br.lu, br.k, br.freq_offset, br.start
    );
    Run { bb, ct, br }
}

fn rel_err(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs()
}

/// Stage 1 and stage 2, printed; returns the final ranking.
fn list_and_check(r: &Run) -> Vec<KCandidate> {
    let list = list_of_k(&KParams::from_br(&r.br)).unwrap();
    let ranked = check_orthogonality(&r.bb, &r.br, &list, &OrthoParams::default(), &never).unwrap();
    for c in ranked.iter().take(5) {
        println!(
            "  K {}/{}  LS {} LU {} LG {}  SR {:.2}  LU error {:.4}  ICI {:.1} dB  guard {:.1} dB",
            c.k_num,
            c.k_den,
            c.ls,
            c.lu,
            c.lg,
            c.sample_rate,
            c.lu_error,
            c.ici_db.unwrap_or(f64::NAN),
            c.leakage_db.unwrap_or(f64::NAN)
        );
    }
    ranked
}

fn rank_of(list: &[KCandidate], num: usize, den: usize) -> Option<usize> {
    list.iter().position(|c| c.k_num == num && c.k_den == den)
}

/// Phase distance in degrees from the nearest odd multiple of 45 degrees.
fn qpsk_error_deg(v: Complex32) -> f64 {
    f64::from(v.arg()).to_degrees().rem_euclid(90.0) - 45.0
}

fn rms(v: impl Iterator<Item = f64>) -> f64 {
    let (s, n) = v.fold((0.0, 0), |(s, n), x| (s + x * x, n + 1));
    (s / f64::from(n.max(1))).sqrt()
}

#[test]
fn article_371_lu64_lg20_real_7000() {
    let p = base(7000.0, 64, 20, 24, -12);
    let s = ofdm_real(&p, 1750.0).unwrap().signal;
    let r = analyse(&s, 50.0, 150.0);
    assert_eq!((r.ct.ls, r.ct.lu, r.ct.lg), (84, 64, 20));
    assert_eq!(r.ct.triangle.lg_width, 20, "triangle width measures LG");
    assert!(r.ct.confidence > 0.5, "confidence {}", r.ct.confidence);
    assert!(rel_err(r.br.br, 7000.0 / 84.0) < 1e-5, "Br {}", r.br.br);
    assert!((r.br.k - 20.0 / 64.0).abs() < 1e-3, "K {}", r.br.k);
    // The triangle peak does not move when stepping by whole symbols.
    for j in [0, 7, 50, 300] {
        let t = ct_triangle(&r.bb, 84, 64, r.ct.start + j * 84, 40).unwrap();
        assert_eq!(t.peak, 0, "step {j}");
    }
    // At a 1% wrong sample rate the triangle rolls as you step.
    let wrong = full(&resample(&s, 7070.0));
    let a = ct_triangle(&wrong, 85, 65, 0, 40).unwrap();
    let b = ct_triangle(&wrong, 85, 65, 200 * 85, 40).unwrap();
    assert!(
        a.peak.abs_diff(b.peak) >= 2,
        "peaks {} and {}",
        a.peak,
        b.peak
    );
}

#[test]
fn article_382_dqpsk_30_channels_with_noise() {
    let p = OfdmParams {
        modulation: OfdmModulation::Pi4Dqpsk,
        frequency_offset: 13.7,
        ..base(6400.0, 80, 16, 30, -15)
    };
    let s = add_awgn_snr(&ofdm(&p).unwrap().signal, 15.0, 9).unwrap();
    let r = analyse(&s, 40.0, 120.0);
    assert_eq!((r.ct.ls, r.ct.lu, r.ct.lg), (96, 80, 16));
    assert!(rel_err(r.br.br, 6400.0 / 96.0) < 1e-4, "Br {}", r.br.br);
    assert!(
        (r.br.freq_offset - 13.7).abs() < 0.3,
        "offset {}",
        r.br.freq_offset
    );
    let grid = Grid::from_br(&r.br);
    assert!((grid.spacing() - 80.0).abs() < 1e-9);
    let occ = occupancy(&r.bb, &grid, 0, 100).unwrap();
    assert_eq!(occ.count(), 30);
    assert_eq!(
        (occ.occupied.first(), occ.occupied.last()),
        (Some(&-15), Some(&14))
    );
    // Relative constellations of pi/4-DQPSK cluster at odd multiples of 45 degrees.
    let bins = [-15, -3, 0, 14];
    let rel = channel_symbols(&r.bb, &grid, &bins, 0, 300, 1, SymbolView::Relative).unwrap();
    assert_eq!(
        rel.symbols.first(),
        Some(&1),
        "relative view starts at symbol 1"
    );
    for (b, vals) in rel.bins.iter().zip(&rel.values) {
        let e = rms(vals.iter().map(|v| qpsk_error_deg(*v)));
        println!("bin {b}: relative phase RMS error {e:.2} deg");
        assert!(e < 9.0, "bin {b}: {e} deg");
    }
    // The absolute view looks like PSK-8: steps of 45 degrees, not 90.
    let abs = channel_symbols(&r.bb, &grid, &[0], 1, 300, 1, SymbolView::Absolute).unwrap();
    let odd = abs.values[0]
        .iter()
        .filter(|v| qpsk_error_deg(**v).abs() < 22.5)
        .count();
    assert!(
        (90..210).contains(&odd),
        "{odd} of 300 on the odd diagonals"
    );
}

#[test]
fn virtual_shift_restores_native_bins_and_pilot_reference_works() {
    // Bin 0 of the generator sits 15 channel steps up, so 15 * LS / LU is not a whole
    // number and the absolute phase turns by 0.6875 cycles per symbol unless the
    // virtual shift puts the channels back on their native bins (model A).
    let p = OfdmParams {
        overrides: vec![(0, OfdmModulation::Pilot)],
        ..base(7000.0, 64, 20, 24, -12)
    };
    let out = ofdm_real(&p, 15.0 * 7000.0 / 64.0).unwrap();
    let r = analyse(&out.signal, 50.0, 150.0);
    let grid = Grid::from_br(&r.br);
    assert!(r.br.freq_offset.abs() < 0.05, "offset {}", r.br.freq_offset);
    // Symbol 0 of the grid is the first complete symbol of the recording.
    assert!(r.br.start < 1.0, "start {}", r.br.start);
    let native = grid.clone().with_virtual_shift(15);
    let chans = [0_usize, 5, 17];
    let bins: Vec<i64> = chans.iter().map(|c| out.bins[*c]).collect();
    let got = channel_symbols(&r.bb, &native, &bins, 0, 200, 1, SymbolView::Absolute).unwrap();
    // The real output is scaled by sqrt(2) to keep the RMS; the analytic filter keeps it.
    let scale = 0.3 * std::f64::consts::SQRT_2 / 24_f64.sqrt();
    for (c, vals) in chans.iter().zip(&got.values) {
        let err = rms(vals.iter().zip(&got.symbols).map(|(v, k)| {
            let want = out.symbols[*k][*c];
            f64::from((v / scale as f32 - want).norm())
        }));
        println!("channel {c}: absolute EVM {err:.3}");
        assert!(err < 0.1, "channel {c}: EVM {err}");
    }
    // Without the shift the same channels rotate from symbol to symbol.
    let shifted: Vec<i64> = bins.iter().map(|b| b + 15).collect();
    let raw = channel_symbols(&r.bb, &grid, &shifted, 0, 3, 1, SymbolView::Absolute).unwrap();
    let step = f64::from((raw.values[1][1] * raw.values[1][0].conj()).arg());
    let sent = f64::from((out.symbols[1][5] * out.symbols[0][5].conj()).arg());
    let extra = (step - sent).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU;
    assert!(
        (extra - 0.6875).abs() < 0.02,
        "extra rotation {extra} cycles"
    );
    // Pilot-referenced view of QPSK channels (model C reading).
    let pil = channel_symbols(
        &r.bb,
        &native,
        &bins[1..],
        0,
        200,
        3,
        SymbolView::PilotReferenced { pilot: bins[0] },
    )
    .unwrap();
    assert_eq!(pil.symbols[1], 3, "every 3rd symbol");
    for vals in &pil.values {
        assert!(rms(vals.iter().map(|v| qpsk_error_deg(*v))) < 5.0);
    }
    let spec = lu_spectrum(&r.bb, &native, 10).unwrap();
    assert_eq!(spec.bins.len(), 64);
    assert!((spec.frequencies[32] - 15.0 * 7000.0 / 64.0).abs() < 0.1);
}

#[test]
fn cis12_like_native_and_resampled() {
    let p = base(7200.0, 36, 24, 14, 2);
    let s = ofdm(&p).unwrap().signal;
    let r0 = analyse(&s, 60.0, 200.0);
    assert_eq!((r0.ct.ls, r0.ct.lu, r0.ct.lg), (60, 36, 24));
    let r = analyse(&resample(&s, 8000.0), 60.0, 200.0);
    assert!(rel_err(r.br.br, 120.0) < 1e-4, "Br {}", r.br.br);
    assert!((r.br.k - 2.0 / 3.0).abs() < 1e-3, "K {}", r.br.k);
    let ranked = list_and_check(&r);
    assert_eq!(rank_of(&ranked, 2, 3), Some(0));
    let top = &ranked[0];
    assert_eq!((top.ls, top.lu, top.lg), (70, 42, 28));
    assert!(rel_err(top.sample_rate, 8400.0) < 1e-4);
}

#[test]
fn quarter_k_real_resampled_then_native() {
    // WinDrm-like K = 1/4: LU 112, LG 28 at 6000 Hz, recorded at 8000 Hz.
    let p = base(6000.0, 112, 28, 40, -20);
    let s = resample(&ofdm_real(&p, 1400.0).unwrap().signal, 8000.0);
    let r = analyse(&s, 30.0, 100.0);
    assert!(rel_err(r.br.br, 6000.0 / 140.0) < 1e-4, "Br {}", r.br.br);
    let ranked = list_and_check(&r);
    assert_eq!(rank_of(&ranked, 1, 4), Some(0));
    // Resample to the chosen native rate: Find CT then gives whole LS/LU/LG.
    let top = &ranked[0];
    let sr = native_rate(r.br.br, top.lu, top.lg).unwrap();
    assert!(rel_err(sr, 190.0 * 6000.0 / 140.0) < 1e-4, "SR {sr}");
    let r2 = analyse(&resample(&s, sr), 30.0, 100.0);
    assert_eq!((r2.ct.ls, r2.ct.lu, r2.ct.lg), (190, 152, 38));
    assert!(rel_err(r2.br.ls, 190.0) < 1e-5, "LS {}", r2.br.ls);
}

#[test]
fn quarter_k_short_cp_complex() {
    // K = 1/4 with a short CP: LU 64, LG 16 at 8000 Hz.
    let p = base(8000.0, 64, 16, 40, -20);
    let r = analyse(&ofdm(&p).unwrap().signal, 60.0, 200.0);
    assert_eq!((r.ct.ls, r.ct.lu, r.ct.lg), (80, 64, 16));
    let ranked = list_and_check(&r);
    assert_eq!(rank_of(&ranked, 1, 4), Some(0));
}

#[test]
fn lu64_lg20_resampled_ranks_5_16_first() {
    let p = base(7000.0, 64, 20, 24, -12);
    let s = ofdm_real(&p, 1750.0).unwrap().signal;
    let r = analyse(&resample(&s, 8000.0), 50.0, 150.0);
    assert!(rel_err(r.br.br, 7000.0 / 84.0) < 1e-4, "Br {}", r.br.br);
    let ranked = list_and_check(&r);
    assert_eq!(rank_of(&ranked, 5, 16), Some(0));
}

#[test]
fn mil_like_17_64_near_first() {
    // MIL-188-110B 39-tone-like: LS 162, LU 128, LG 34 (K = 17/64), recorded at 8000.
    let p = OfdmParams {
        symbols: 300,
        ..base(7000.0, 128, 34, 39, -19)
    };
    let r = analyse(&resample(&ofdm(&p).unwrap().signal, 8000.0), 30.0, 60.0);
    assert!(rel_err(r.br.br, 7000.0 / 162.0) < 1e-4, "Br {}", r.br.br);
    let ranked = list_and_check(&r);
    let rank = rank_of(&ranked, 17, 64).unwrap();
    assert!(rank <= 2, "17/64 ranked {rank}");
}

#[test]
fn stage2_demotes_a_non_orthogonal_candidate() {
    let p = base(8000.0, 64, 16, 40, -20);
    let r = analyse(&ofdm(&p).unwrap().signal, 60.0, 200.0);
    let good = KCandidate {
        k_num: 1,
        k_den: 4,
        k: 0.25,
        ls: 80,
        lu: 64,
        lg: 16,
        sample_rate: 8000.0,
        lu_error: 0.1,
        even_lu: true,
        hits: 1,
        score: 1.0,
        leakage_db: None,
        ici_db: None,
    };
    let bad = KCandidate {
        k_num: 15,
        k_den: 65,
        lu: 65,
        lg: 15,
        score: 0.0,
        ..good.clone()
    };
    let ranked =
        check_orthogonality(&r.bb, &r.br, &[bad, good], &OrthoParams::default(), &never).unwrap();
    println!("{:?}", ranked.iter().map(|c| c.ici_db).collect::<Vec<_>>());
    assert_eq!(
        ranked[0].lu, 64,
        "stage 2 overrides stage 1 when clearly better"
    );
    assert!(ranked[1].ici_db.unwrap() > ranked[0].ici_db.unwrap() + 20.0);
}

#[test]
fn odd_lu_is_found() {
    let p = base(6300.0, 63, 21, 20, -10);
    let r = analyse(&ofdm(&p).unwrap().signal, 50.0, 150.0);
    assert_eq!((r.ct.ls, r.ct.lu, r.ct.lg), (84, 63, 21));
    assert!(rel_err(r.br.br, 75.0) < 1e-5);
}

#[test]
fn cancellation_and_bad_input() {
    let p = OfdmParams {
        symbols: 60,
        ..base(7000.0, 64, 20, 24, -12)
    };
    let bb = full(&ofdm(&p).unwrap().signal);
    let stop = || true;
    assert_eq!(
        find_ct(&bb, &CtParams::new(50.0, 150.0), &stop).unwrap_err(),
        Error::Cancelled
    );
    let ct = find_ct(&bb, &CtParams::new(50.0, 150.0), &never).unwrap();
    assert_eq!(
        get_br(&bb, &ct, &BrParams::default(), &stop).unwrap_err(),
        Error::Cancelled
    );
    // Bounds the wrong way round, and a span too short for the slowest rate.
    assert!(matches!(
        find_ct(&bb, &CtParams::new(150.0, 50.0), &never),
        Err(Error::Invalid(_))
    ));
    let short = Baseband::from_samples(bb.samples()[..600].to_vec(), 7000.0).unwrap();
    let Err(Error::Invalid(msg)) = find_ct(&short, &CtParams::new(30.0, 150.0), &never) else {
        panic!("expected an error")
    };
    assert!(msg.contains("Raise MinBr"), "{msg}");
    assert!(native_rate(0.0, 64, 16).is_err());
    assert!(
        channel_symbols(
            &bb,
            &Grid::from_ct(&ct),
            &[40],
            0,
            1,
            1,
            SymbolView::Absolute
        )
        .is_err()
    );
}

#[test]
fn noisy_resampled_lu64_lg20_lu_is_unbiased() {
    // The fractional LU must not drift with noise (it used to read 73.21 at 5 dB and
    // 73.29 at 0 dB against 73.14). Over several noise draws the error is centred on
    // zero; at 10 dB K = 5/16 stays in the top 3, at 5 dB the scatter of about 0.03
    // samples (400 symbols) can push it lower, but K itself stays within 1e-3.
    let p = base(7000.0, 64, 20, 24, -12);
    let s = resample(&ofdm_real(&p, 1750.0).unwrap().signal, 8000.0);
    let true_lu = 64.0 * 8000.0 / 7000.0;
    for snr in [10.0, 5.0] {
        let mut errors = Vec::new();
        for seed in 1..=4 {
            let r = analyse(&add_awgn_snr(&s, snr, seed).unwrap(), 50.0, 150.0);
            let err = r.br.lu - true_lu;
            println!("SNR {snr} dB seed {seed}: LU error {err:+.4}");
            assert!(err.abs() < 0.06, "SNR {snr} seed {seed}: LU {}", r.br.lu);
            assert!(
                (r.br.k - 5.0 / 16.0).abs() < 1e-3,
                "SNR {snr}: K {}",
                r.br.k
            );
            if snr >= 10.0 {
                let ranked = list_and_check(&r);
                let rank = rank_of(&ranked, 5, 16);
                assert!(
                    rank.is_some_and(|r| r <= 2),
                    "seed {seed}: 5/16 ranked {rank:?}"
                );
            }
            errors.push(err);
        }
        let mean = errors.iter().sum::<f64>() / errors.len() as f64;
        assert!(mean.abs() < 0.015, "SNR {snr}: mean LU error {mean}");
    }
}

#[test]
fn oversampled_and_few_channel_lu_is_exact() {
    // Narrow bands (x[n] close to x[n-1]) used to give LU - 1, and few channels an ACF
    // side peak several lags below LU.
    for (lu, lg, channels) in [
        (144, 18, 16),
        (128, 32, 12),
        (64, 16, 8),
        (72, 8, 6),
        (64, 16, 3),
    ] {
        let fs = 8000.0;
        let ls = (lu + lg) as f64;
        let p = base(fs, lu, lg, channels, -(channels as i64) / 2);
        let r = analyse(&ofdm(&p).unwrap().signal, fs / ls / 1.6, fs / ls * 1.6);
        assert_eq!(
            (r.ct.ls, r.ct.lu, r.ct.lg),
            (lu + lg, lu, lg),
            "{channels} channels"
        );
        assert!((r.br.lu - lu as f64).abs() < 0.01, "Br LU {}", r.br.lu);
    }
}

#[test]
fn occupancy_counts_56_of_64() {
    // More than 75 % of the bins hold channels (like DRM at its native rate).
    let p = base(8000.0, 64, 16, 56, -28);
    let r = analyse(&ofdm(&p).unwrap().signal, 60.0, 200.0);
    let occ = occupancy(&r.bb, &Grid::from_br(&r.br), 0, 200).unwrap();
    println!(
        "floor {} peak {} threshold {}",
        occ.floor_db, occ.peak_db, occ.threshold_db
    );
    assert_eq!(occ.count(), 56);
    assert_eq!(
        (occ.occupied.first(), occ.occupied.last()),
        (Some(&-28), Some(&27))
    );
}

#[test]
fn channel_at_dc_keeps_its_values() {
    // Removing the record mean must not leak into bin 0.
    let p = base(8000.0, 64, 16, 40, -20);
    let out = ofdm(&p).unwrap();
    let r = analyse(&out.signal, 60.0, 200.0);
    let grid = Grid::from_br(&r.br);
    let got = channel_symbols(&r.bb, &grid, &out.bins, 0, 300, 1, SymbolView::Absolute).unwrap();
    let scale = 0.3 / 40_f64.sqrt();
    // Symbol 0 of the grid is the first complete symbol, which the generator sent first.
    let k0 = 0;
    for ((b, vals), c) in got.bins.iter().zip(&got.values).zip(0..) {
        let err = rms(vals.iter().zip(&got.symbols).map(|(v, k)| {
            let want = out.symbols[k + k0][c];
            f64::from((v / scale as f32 - want).norm())
        }));
        assert!(err < 1e-3, "bin {b}: EVM {err}");
    }
}

#[test]
fn clean_signal_is_confident_when_2ls_is_in_range() {
    // 2 LS folds the same CPs: it is not a rival answer and must not cut the margin.
    let s = ofdm(&base(7200.0, 36, 24, 14, 2)).unwrap().signal;
    let ct = find_ct(&full(&s), &CtParams::new(60.0, 200.0), &never).unwrap();
    assert_eq!((ct.ls, ct.lu), (60, 36));
    assert!(ct.margin > 2.0, "margin {}", ct.margin);
    assert!(ct.confidence > 0.5, "confidence {}", ct.confidence);
}

#[test]
fn noise_only_is_not_a_confident_triangle() {
    let zeros = Signal::complex(vec![Complex32::new(0.0, 0.0); 40_000], 8000.0).unwrap();
    let bb = full(&add_awgn(&zeros, 0.1, 5).unwrap());
    match find_ct(&bb, &CtParams::new(50.0, 150.0), &never) {
        Err(Error::NotFound(_)) => {}
        Ok(ct) => assert!(ct.confidence < 0.3, "confidence {}", ct.confidence),
        Err(e) => panic!("unexpected error {e}"),
    }
}

#[test]
fn bad_parameters() {
    let p = OfdmParams {
        symbols: 60,
        ..base(8000.0, 64, 16, 40, -20)
    };
    let s = ofdm(&p).unwrap().signal;
    let bb = full(&s);
    let ct = find_ct(&bb, &CtParams::new(60.0, 200.0), &never).unwrap();
    let invalid = |r: Result<()>| matches!(r, Err(Error::Invalid(_)));
    // Grid::check: window offset above LG, non-finite start or LS, LU + LG above LS.
    let good = Grid::from_ct(&ct);
    let bad_grids = [
        Grid {
            window_offset: good.lg + 1,
            ..good.clone()
        },
        Grid {
            start: f64::NAN,
            ..good.clone()
        },
        Grid {
            ls: f64::INFINITY,
            ..good.clone()
        },
        Grid {
            lg: good.lg + 4,
            ..good.clone()
        },
    ];
    for g in &bad_grids {
        assert!(invalid(lu_spectrum(&bb, g, 0).map(|_| ())), "{g:?}");
    }
    // list_of_k.
    let kp = KParams {
        sample_rate: 8000.0,
        br: 100.0,
        lu: 64.0,
        tolerance: 0.15,
        max_factor: 2.0,
        max_candidates: 50,
    };
    for tolerance in [0.0, 0.5, f64::NAN] {
        let r = list_of_k(&KParams {
            tolerance,
            ..kp.clone()
        });
        assert!(invalid(r.map(|_| ())), "tolerance {tolerance}");
    }
    assert!(invalid(
        list_of_k(&KParams {
            lu: 80.0,
            ..kp.clone()
        })
        .map(|_| ())
    ));
    for max_factor in [f64::NAN, f64::INFINITY, 0.5] {
        let r = list_of_k(&KParams {
            max_factor,
            ..kp.clone()
        });
        assert!(invalid(r.map(|_| ())), "max factor {max_factor}");
    }
    let r = list_of_k(&KParams {
        lu: 64.5,
        tolerance: 1e-6,
        max_factor: 1.0,
        ..kp.clone()
    });
    assert!(matches!(r, Err(Error::NotFound(_))), "{r:?}");
    // get_br with a block below 3.
    let bp = BrParams {
        block: 2,
        ..BrParams::default()
    };
    assert!(invalid(get_br(&bb, &ct, &bp, &never).map(|_| ())));
    // Occupancy from past the end, a triangle that starts too late.
    assert!(invalid(occupancy(&bb, &good, 10_000, 5).map(|_| ())));
    assert!(invalid(
        ct_triangle(&bb, 80, 64, bb.len() - 100, 40).map(|_| ())
    ));
    // Spans of fewer than 16 or more than MAX_SAMPLES samples.
    assert!(invalid(
        Baseband::from_signal(&s, 0, 15, &never).map(|_| ())
    ));
    let huge = Signal::real(vec![0.0_f32; MAX_SAMPLES + 1], 8000.0).unwrap();
    assert!(invalid(
        Baseband::from_signal(&huge, 0, huge.len(), &never).map(|_| ())
    ));
}

#[test]
fn symbol_count_matches_window_start() {
    // start + k * ls lands on .5, which window_start rounds up.
    let g = Grid {
        sample_rate: 8000.0,
        ls: 10.0,
        lu: 8,
        lg: 2,
        start: 0.5,
        window_offset: 2,
        freq_offset: 0.0,
        virtual_shift: 0,
    };
    let n = g.symbol_count(30);
    assert_eq!(n, 2);
    assert!(g.window_start(n - 1) + g.lu <= 30);
    let bb = Baseband::from_samples(vec![Complex32::new(1.0, 0.0); 30], 8000.0).unwrap();
    assert!(lu_spectrum(&bb, &g, 1).is_ok());
    assert!(lu_spectrum(&bb, &g, 2).is_err());
    assert_eq!(occupancy(&bb, &g, 0, 10).unwrap().symbols, 2);
}

#[test]
fn huge_indices_are_errors_not_overflows() {
    let p = OfdmParams {
        symbols: 60,
        ..base(8000.0, 64, 16, 40, -20)
    };
    let r = analyse(&ofdm(&p).unwrap().signal, 60.0, 200.0);
    let g = Grid::from_br(&r.br);
    assert!(lu_spectrum(&r.bb, &g, usize::MAX).is_err());
    assert!(occupancy(&r.bb, &g, 1, usize::MAX).is_ok());
    assert!(occupancy(&r.bb, &g, usize::MAX, usize::MAX).is_err());
    let one = channel_symbols(&r.bb, &g, &[0], 2, 3, usize::MAX, SymbolView::Absolute).unwrap();
    assert_eq!(one.symbols, vec![2]);
    let far = Grid {
        start: 1e30,
        ..g.clone()
    };
    assert_eq!(far.symbol_count(r.bb.len()), 0);
    assert!(lu_spectrum(&r.bb, &far, 0).is_err());
}

#[test]
fn relative_view_is_the_same_with_any_step() {
    let p = OfdmParams {
        modulation: OfdmModulation::Pi4Dqpsk,
        symbols: 60,
        ..base(8000.0, 64, 16, 40, -20)
    };
    let r = analyse(&ofdm(&p).unwrap().signal, 60.0, 200.0);
    let g = Grid::from_br(&r.br);
    let bins = [-5, 0, 7];
    let all = channel_symbols(&r.bb, &g, &bins, 1, 20, 1, SymbolView::Relative).unwrap();
    let odd = channel_symbols(&r.bb, &g, &bins, 1, 10, 2, SymbolView::Relative).unwrap();
    for (a, o) in all.values.iter().zip(&odd.values) {
        for (j, k) in odd.symbols.iter().enumerate() {
            let i = all.symbols.iter().position(|s| s == k).unwrap();
            assert!((a[i] - o[j]).norm() < 1e-6, "symbol {k}");
        }
    }
}

#[test]
fn channel_stages_stop_when_cancelled() {
    let p = OfdmParams {
        symbols: 60,
        ..base(8000.0, 64, 16, 40, -20)
    };
    let r = analyse(&ofdm(&p).unwrap().signal, 60.0, 200.0);
    let g = Grid::from_br(&r.br);
    assert!(matches!(
        occupancy_with_stop(&r.bb, &g, 0, 50, &|| true),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        channel_symbols_with_stop(&r.bb, &g, &[0], 0, 50, 1, SymbolView::Absolute, &|| true),
        Err(Error::Cancelled)
    ));
    // Without a stop request the variants match the plain functions.
    assert_eq!(
        occupancy_with_stop(&r.bb, &g, 0, 50, &never).unwrap(),
        occupancy(&r.bb, &g, 0, 50).unwrap()
    );
}

/// 300 symbols of LU 256, LG 64: more than four symbol batches, and more CP positions
/// than one parallel block of the LU search.
fn long_record() -> Run {
    let p = OfdmParams {
        modulation: OfdmModulation::Pi4Dqpsk,
        symbols: 300,
        ..base(8000.0, 256, 64, 100, -50)
    };
    let s = add_awgn_snr(&ofdm(&p).unwrap().signal, 25.0, 5).unwrap();
    let r = analyse(&s, 15.0, 40.0);
    assert_eq!((r.ct.lu, r.ct.lg), (256, 64));
    r
}

#[test]
fn relative_view_matches_absolute_across_symbol_batches() {
    let r = long_record();
    let g = Grid::from_br(&r.br);
    let bins = [-50, -7, 0, 23, 49];
    let rel = channel_symbols(&r.bb, &g, &bins, 1, 150, 1, SymbolView::Relative).unwrap();
    let abs = channel_symbols(&r.bb, &g, &bins, 0, 151, 1, SymbolView::Absolute).unwrap();
    assert_eq!(rel.symbols, (1..=150).collect::<Vec<_>>());
    assert!(rel.symbols.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(abs.symbols, (0..=150).collect::<Vec<_>>());
    for (c, (rv, av)) in rel.values.iter().zip(&abs.values).enumerate() {
        assert_eq!(rv.len(), 150);
        for (j, k) in rel.symbols.iter().enumerate() {
            let cur = Complex64::new(av[*k].re.into(), av[*k].im.into());
            let prev = Complex64::new(av[k - 1].re.into(), av[k - 1].im.into());
            let want = cur * prev.conj() / prev.norm();
            let got = Complex64::new(rv[j].re.into(), rv[j].im.into());
            assert!(
                (got - want).norm() <= 1e-4 * want.norm().max(1e-3),
                "bin {} symbol {k}: {got} vs {want}",
                bins[c]
            );
        }
    }
}

/// Everything the parallel stages produce, as bits.
fn analysis_bits(r: &Run) -> Vec<u64> {
    let ct = find_ct(&r.bb, &CtParams::new(15.0, 40.0), &never).unwrap();
    let br = get_br(&r.bb, &ct, &BrParams::default(), &never).unwrap();
    let g = Grid::from_br(&br);
    let occ = occupancy(&r.bb, &g, 0, usize::MAX).unwrap();
    let rel =
        channel_symbols(&r.bb, &g, &g.bins(), 0, usize::MAX, 1, SymbolView::Relative).unwrap();
    assert!(rel.symbols.len() > 256, "{} symbols", rel.symbols.len());
    let mut bits: Vec<u64> = [br.ls, br.lu, br.lg, br.freq_offset, br.start, br.coherence]
        .iter()
        .map(|v| v.to_bits())
        .collect();
    bits.extend(occ.power_db.iter().map(|v| u64::from(v.to_bits())));
    for vals in &rel.values {
        bits.extend(
            vals.iter()
                .flat_map(|v| [u64::from(v.re.to_bits()), u64::from(v.im.to_bits())]),
        );
    }
    bits
}

#[test]
fn results_do_not_depend_on_the_thread_count() {
    let r = long_record();
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| analysis_bits(&r))
    };
    let one = run(1);
    let four = run(4);
    assert_eq!(one.len(), four.len());
    assert!(one == four, "outputs differ between 1 and 4 threads");
}
