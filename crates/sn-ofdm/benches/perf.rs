//! Wall-time benchmarks of the OFDM analysis chain on realistic sizes.
//!
//! Run with `cargo bench -p sn-ofdm --bench perf` (optionally followed by
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

use sn_ofdm::{
    Baseband, BrEstimate, BrParams, ChannelSymbols, CtParams, CtResult, Grid, KCandidate, KParams,
    Occupancy, OrthoParams, SymbolView, channel_symbols, check_orthogonality, find_ct, get_br,
    list_of_k, occupancy,
};
use sn_synth::{OfdmModulation, OfdmParams, add_awgn_snr, ofdm, ofdm_real};

use bench::{Bench, never};

fn params(fs: f64, lu: usize, lg: usize, channels: usize, symbols: usize) -> OfdmParams {
    OfdmParams {
        sample_rate: fs,
        lu,
        lg,
        channels,
        first_bin: -(channels as i64 / 2),
        unused: vec![],
        modulation: OfdmModulation::Qpsk,
        overrides: vec![],
        symbols,
        frequency_offset: 0.0,
        amplitude: 0.3,
        seed: 1,
    }
}

/// The cases of [`chain`] for `label`, in order.
fn chain_cases(label: &str) -> Vec<String> {
    [
        "find_ct",
        "get_br",
        "list_of_k+ortho",
        "occupancy",
        "channel_symbols",
    ]
    .iter()
    .map(|c| format!("{c} {label}"))
    .collect()
}

/// What a chain is expected to find in the synthetic signal.
#[derive(Clone, Copy)]
struct Expect {
    /// Useful length.
    lu: usize,
    /// Guard length.
    lg: usize,
    /// Active carriers synthesised.
    channels: usize,
    /// OFDM symbols synthesised.
    symbols: usize,
}

/// Fewest symbols the grid may lose at the edges of the span (a partial first or
/// last symbol, and symbol 0, which the relative view skips).
const EDGE_SYMBOLS: usize = 3;

/// Check that a baseband built from `n` samples kept every sample.
fn check_baseband(name: &str, n: usize) -> impl FnOnce(&Baseband) + '_ {
    move |bb: &Baseband| {
        assert_eq!(
            bb.len(),
            n,
            "{name}: baseband holds the wrong number of samples"
        );
    }
}

fn chain(b: &Bench, label: &str, bb: &Baseband, br_range: (f64, f64), want: Expect) {
    let cases = chain_cases(label);
    let ct_params = CtParams::new(br_range.0, br_range.1);
    // The setup of later cases (Find CT, Get Br) only runs when one of them is wanted.
    let later = |from: usize| b.wanted(cases.get(from..).unwrap_or_default());
    b.run(
        &cases[0],
        || find_ct(bb, &ct_params, &never).unwrap(),
        |ct: &CtResult| {
            assert_eq!(
                (ct.lu, ct.lg),
                (want.lu, want.lg),
                "{}: wrong LU/LG",
                cases[0]
            );
        },
    );
    if !later(1) {
        return;
    }
    let ct = find_ct(bb, &ct_params, &never).unwrap();
    b.run(
        &cases[1],
        || get_br(bb, &ct, &BrParams::default(), &never).unwrap(),
        |br: &BrEstimate| {
            assert!(
                (br.lu - want.lu as f64).abs() < 1.0,
                "{}: LU {:.3}, expected {}",
                cases[1],
                br.lu,
                want.lu
            );
        },
    );
    if !later(2) {
        return;
    }
    let br = get_br(bb, &ct, &BrParams::default(), &never).unwrap();
    b.run(
        &cases[2],
        || {
            let list = list_of_k(&KParams::from_br(&br)).unwrap();
            check_orthogonality(bb, &br, &list, &OrthoParams::default(), &never).unwrap()
        },
        |ks: &Vec<KCandidate>| {
            let k = want.lg as f64 / want.lu as f64;
            let first = ks
                .first()
                .unwrap_or_else(|| panic!("{}: no K candidate", cases[2]));
            assert!(
                first.leakage_db.is_some() && (first.k - k).abs() < 1e-9,
                "{}: best K {}/{} (checked: {}), expected {k}",
                cases[2],
                first.k_num,
                first.k_den,
                first.leakage_db.is_some()
            );
        },
    );
    let grid = Grid::from_br(&br);
    b.run(
        &cases[3],
        || occupancy(bb, &grid, 0, usize::MAX).unwrap(),
        |occ: &Occupancy| {
            assert_eq!(
                occ.count(),
                want.channels,
                "{}: wrong number of occupied carriers",
                cases[3]
            );
        },
    );
    let bins = grid.bins();
    b.run(
        &cases[4],
        || channel_symbols(bb, &grid, &bins, 0, usize::MAX, 1, SymbolView::Relative).unwrap(),
        |cs: &ChannelSymbols| {
            let n = cs.symbols.len();
            assert!(
                n < want.symbols && n + EDGE_SYMBOLS >= want.symbols,
                "{}: {n} symbols, expected {} less at most {EDGE_SYMBOLS}",
                cases[4],
                want.symbols
            );
            assert_eq!(cs.values.len(), bins.len(), "{}: wrong row count", cases[4]);
            assert!(
                cs.values.iter().all(|row| row.len() == n),
                "{}: a row does not hold {n} symbols",
                cases[4]
            );
        },
    );
}

/// Whether any case of the chain `label`, or its baseband case, is wanted.
fn chain_wanted(b: &Bench, label: &str) -> bool {
    b.selects(&format!("baseband {label}")) || b.wanted(&chain_cases(label))
}

fn main() {
    let Some(b) = Bench::from_args("sn-ofdm") else {
        return;
    };

    // 24 s of real audio-band OFDM at 48 kHz (LU 256, LG 32).
    let label = "24s@48k real";
    if chain_wanted(&b, label) {
        let s = ofdm_real(&params(48_000.0, 256, 32, 100, 4000), 12_000.0)
            .unwrap()
            .signal;
        let s = add_awgn_snr(&s, 20.0, 3).unwrap();
        let name = format!("baseband {label}");
        b.run(
            &name,
            || Baseband::from_signal(&s, 0, s.len(), &never).unwrap(),
            check_baseband(&name, s.len()),
        );
        if b.wanted(&chain_cases(label)) {
            let bb = Baseband::from_signal(&s, 0, s.len(), &never).unwrap();
            chain(
                &b,
                label,
                &bb,
                (100.0, 300.0),
                Expect {
                    lu: 256,
                    lg: 32,
                    channels: 100,
                    symbols: 4000,
                },
            );
        }
    }

    // A wideband complex OFDM slice: 2.6 M samples at 2 MS/s (LU 2048, LG 512).
    let label = "1.3s@2M iq";
    if chain_wanted(&b, label) {
        let w = ofdm(&params(2e6, 2048, 512, 1700, 1000)).unwrap().signal;
        let w = add_awgn_snr(&w, 20.0, 4).unwrap();
        let name = format!("baseband {label}");
        b.run(
            &name,
            || Baseband::from_signal(&w, 0, w.len(), &never).unwrap(),
            check_baseband(&name, w.len()),
        );
        if b.wanted(&chain_cases(label)) {
            let wb = Baseband::from_signal(&w, 0, w.len(), &never).unwrap();
            chain(
                &b,
                label,
                &wb,
                (500.0, 1200.0),
                Expect {
                    lu: 2048,
                    lg: 512,
                    channels: 1700,
                    symbols: 1000,
                },
            );
        }
    }
}
