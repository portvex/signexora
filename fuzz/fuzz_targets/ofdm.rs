//! Run the OFDM chain (Find CT, Get Br, channels, List of K) on arbitrary samples
//! with arbitrary search bounds.
#![no_main]

use libfuzzer_sys::fuzz_target;
use num_complex::Complex32;
use sn_core::Signal;
use sn_ofdm::{
    Baseband, BrParams, CtParams, Grid, KParams, SymbolView, channel_symbols, find_ct, get_br,
    list_of_k, lu_spectrum, occupancy,
};

fn f64_at(data: &[u8], at: usize) -> f64 {
    data.get(at..at + 8)
        .and_then(|b| b.try_into().ok())
        .map_or(0.0, f64::from_le_bytes)
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 24 {
        return;
    }
    let (head, body) = data.split_at(24);
    let rate = f64_at(head, 0);
    let min_br = f64_at(head, 8);
    let max_br = f64_at(head, 16);
    let samples: Vec<Complex32> = body
        .chunks_exact(2)
        .map(|c| match c {
            [a, b] => Complex32::new(f32::from(*a as i8) / 128.0, f32::from(*b as i8) / 128.0),
            _ => Complex32::new(0.0, 0.0),
        })
        .collect();
    let Ok(signal) = Signal::complex(samples, rate) else {
        return;
    };
    let stop = || false;
    let Ok(bb) = Baseband::from_signal(&signal, 0, signal.len(), &stop) else {
        return;
    };
    let params = CtParams {
        min_br,
        max_br,
        symbols: 8,
        start: 0,
    };
    let Ok(ct) = find_ct(&bb, &params, &stop) else {
        return;
    };
    let grid = Grid::from_ct(&ct);
    let _ = lu_spectrum(&bb, &grid, 0);
    let _ = occupancy(&bb, &grid, 0, 50);
    let _ = channel_symbols(&bb, &grid, &grid.bins(), 1, 10, 1, SymbolView::Relative);
    let br = BrParams {
        block: 8,
        max_symbols: 200,
    };
    if let Ok(est) = get_br(&bb, &ct, &br, &stop) {
        let _ = list_of_k(&KParams::from_br(&est));
        let _ = channel_symbols(
            &bb,
            &Grid::from_br(&est).with_virtual_shift(3),
            &[0],
            0,
            5,
            2,
            SymbolView::PilotReferenced { pilot: 0 },
        );
    }
});
