//! Wall-time benchmarks of reading memory-mapped recordings.
//!
//! Run with `cargo bench -p sn-io --bench perf` (optionally followed by `-- <filter>`).
//! The shared harness in `benches/common/bench.rs` runs each case once untimed and
//! checks its output, then times `RUNS` runs and prints the fastest, median and
//! slowest wall time.

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
use sn_io::{LoadOptions, open};
use sn_io::{WavFormat, write_wav};

use bench::Bench;

const MATERIALIZE: &str = "materialize 2s@8M i16 iq";
const READ: &str = "read_complex 64k blocks 2s@8M i16 iq";

fn main() {
    let Some(b) = Bench::from_args("sn-io") else {
        return;
    };
    // Skip the costly setup when the filter selects none of these cases.
    if !b.wanted(&[MATERIALIZE, READ]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // 2 s of 8 MS/s I/Q as 16-bit stereo WAV (64 MB).
    let n = 16_000_000;
    let iq: Vec<Complex32> = (0..n)
        .map(|i| {
            let ph = 0.001 * i as f32;
            Complex32::new(0.5 * ph.cos(), 0.5 * ph.sin())
        })
        .collect();
    let path = dir.path().join("iq.wav");
    write_wav(&path, &Signal::complex(iq, 8e6).unwrap(), WavFormat::Pcm16).unwrap();
    let s = open(&path, &LoadOptions::default()).unwrap();
    assert_eq!(s.len(), n, "the written file must hold every frame");
    b.run(
        MATERIALIZE,
        || s.materialize().unwrap(),
        |m: &Signal| assert_eq!(m.len(), n, "{MATERIALIZE}: wrong length"),
    );
    let mut buf = vec![Complex32::new(0.0, 0.0); 1 << 16];
    b.run(
        READ,
        || {
            let mut pos = 0;
            while pos < s.len() {
                let got = s.read_complex(pos, &mut buf);
                assert!(got > 0, "read_complex returned 0 at {pos}");
                pos += got;
            }
            pos
        },
        |&pos| assert_eq!(pos, n, "{READ}: read {pos} frames, expected {n}"),
    );
}
