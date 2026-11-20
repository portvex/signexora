//! Open a SigMF recording from arbitrary metadata text and data bytes. The input is
//! split at the first NUL byte: metadata before it, samples after it.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sn_io::LoadOptions;

#[path = "common.rs"]
mod common;

fuzz_target!(|data: &[u8]| {
    let split = data.iter().position(|b| *b == 0).unwrap_or(data.len());
    let (meta, rest) = data.split_at(split);
    let samples = rest.get(1..).unwrap_or(&[]);
    let dir = common::scratch();
    let meta_path = dir.join("rec.sigmf-meta");
    if std::fs::write(&meta_path, meta).is_err()
        || std::fs::write(dir.join("rec.sigmf-data"), samples).is_err()
    {
        return;
    }
    if let Ok(signal) = sn_io::open(&meta_path, &LoadOptions::default()) {
        common::read_some(&signal, 1 << 16);
    }
});
