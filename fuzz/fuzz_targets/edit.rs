//! Run any edit, with any parameters, on short real and complex signals. Every
//! edit must return a signal or an error: no panic, no runaway allocation.
#![no_main]

use std::sync::LazyLock;

use libfuzzer_sys::fuzz_target;
use num_complex::Complex32;
use sn_core::Signal;
use sn_dsp::ops::Edit;

#[path = "common.rs"]
mod common;

static INPUTS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
    let n = 4000;
    let real: Vec<f32> = (0..n)
        .map(|i| (i as f32 * 0.31).sin() * 0.5 + (i as f32 * 0.07).cos() * 0.2)
        .collect();
    let complex: Vec<Complex32> = (0..n)
        .map(|i| Complex32::from_polar(0.5, i as f32 * 0.23))
        .collect();
    [Signal::real(real, 8000.0), Signal::complex(complex, 8000.0)]
        .into_iter()
        .flatten()
        .collect()
});

fuzz_target!(|data: &[u8]| {
    let Ok(edit) = serde_json::from_slice::<Edit>(data) else {
        return;
    };
    let _ = edit.label();
    for input in INPUTS.iter() {
        if let Ok(out) = edit.run(input, &|| false) {
            common::read_some(&out, 1 << 12);
        }
    }
});
