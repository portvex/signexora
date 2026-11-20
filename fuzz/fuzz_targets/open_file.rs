//! Probe and open arbitrary bytes as a recording: RIFF/RF64 WAV, the compressed
//! formats (symphonia) and raw I/Q. The first byte picks the file extension.
#![no_main]

use std::sync::Once;

use libfuzzer_sys::fuzz_target;
use sn_io::LoadOptions;

#[path = "common.rs"]
mod common;

const EXTENSIONS: [&str; 10] = [
    "wav", "rf64", "bw64", "mp3", "flac", "ogg", "cu8", "cs16", "cf32", "raw",
];

/// sn-io runs symphonia (third-party) under `catch_unwind`, so that a decoder bug
/// on malformed input becomes an error, as it does in the app. libFuzzer's panic
/// hook aborts before unwinding, which would report those known upstream bugs over
/// and over. Let panics raised inside symphonia's own source unwind to the guard;
/// every other panic (ours included) still aborts and is reported.
fn allow_guarded_decoder_panics() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let fuzzer_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let in_symphonia = info
                .location()
                .is_some_and(|l| l.file().contains("/symphonia"));
            if !in_symphonia {
                fuzzer_hook(info);
            }
        }));
    });
}

fuzz_target!(|data: &[u8]| {
    allow_guarded_decoder_panics();
    let Some((first, body)) = data.split_first() else {
        return;
    };
    let ext = EXTENSIONS
        .get(usize::from(*first) % EXTENSIONS.len())
        .copied()
        .unwrap_or("wav");
    let path = common::scratch().join(format!("input.{ext}"));
    if std::fs::write(&path, body).is_err() {
        return;
    }
    let Ok(info) = sn_io::probe(&path) else {
        return;
    };
    let _ = info.duration();
    let _ = info.default_selection();
    if let Ok(signal) = sn_io::open_probed(&info, &LoadOptions::default()) {
        common::read_some(&signal, 1 << 16);
    }
});
