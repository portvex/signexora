//! Wall-time benchmark harness shared by every crate's `benches/perf.rs`.
//!
//! Each bench binary includes this file with
//! `#[path = "../../../benches/common/bench.rs"] mod bench;` and declares
//! `harness = false` in its `[[bench]]` section.
//!
//! Run a crate's cases with `cargo bench -p <crate> --bench perf`, optionally
//! followed by `-- <filter>` to keep only the cases whose name contains the filter.
//! Each case first runs once untimed: that run warms up caches and the thread pool,
//! and its output goes to a check, so that a broken result cannot be reported as a
//! speed-up. The case then runs [`RUNS`] times and prints the fastest, median and
//! slowest wall time.
//!
//! `cargo test --all-targets` also runs bench binaries, without `--bench`; then
//! [`Bench::from_args`] returns `None` and the binary skips all of its cases.

// Not every bench binary uses every helper.
#![allow(dead_code)]

use std::hint::black_box;
use std::time::{Duration, Instant};

/// Timed runs per case (after the untimed checked run).
pub(crate) const RUNS: usize = 10;

/// A `should_stop` callback that never asks to stop.
pub(crate) fn never() -> bool {
    false
}

/// Case filter taken from the command line (the first argument that is not a flag).
pub(crate) struct Bench {
    filter: Option<String>,
}

impl Bench {
    /// The harness for this run, or `None` (after printing a note) when the binary
    /// was not started by `cargo bench`, for example by `cargo test --all-targets`.
    /// `krate` names the package in that note.
    pub(crate) fn from_args(krate: &str) -> Option<Self> {
        if !std::env::args().any(|a| a == "--bench") {
            println!("perf: skipped (run with `cargo bench -p {krate} --bench perf`)");
            return None;
        }
        Some(Self {
            filter: std::env::args().skip(1).find(|a| !a.starts_with('-')),
        })
    }

    /// True when at least one of `names` passes the filter. Used to skip the setup
    /// of a group whose cases would all be filtered out.
    pub(crate) fn wanted<S: AsRef<str>>(&self, names: &[S]) -> bool {
        names.iter().any(|n| self.selects(n.as_ref()))
    }

    /// True when the case `name` passes the filter.
    pub(crate) fn selects(&self, name: &str) -> bool {
        self.filter.as_deref().is_none_or(|w| name.contains(w))
    }

    /// Run `f` once untimed and pass its output to `check`, then time [`RUNS`] runs.
    pub(crate) fn run<T>(&self, name: &str, mut f: impl FnMut() -> T, check: impl FnOnce(&T)) {
        if !self.selects(name) {
            return;
        }
        check(&f());
        let mut times: Vec<Duration> = (0..RUNS)
            .map(|_| {
                let t = Instant::now();
                black_box(f());
                t.elapsed()
            })
            .collect();
        times.sort();
        let ms = |d: Option<&Duration>| d.map_or(f64::NAN, |d| d.as_secs_f64() * 1e3);
        println!(
            "{name:<44} min {:>9.2} ms   median {:>9.2} ms   max {:>9.2} ms",
            ms(times.first()),
            median_ms(&times),
            ms(times.last())
        );
    }
}

/// Median of sorted durations in milliseconds: the middle value for an odd count,
/// the mean of the two middle values for an even count, NaN when empty.
fn median_ms(sorted: &[Duration]) -> f64 {
    let n = sorted.len();
    let at = |i: usize| sorted.get(i).map_or(f64::NAN, |d| d.as_secs_f64() * 1e3);
    if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        at(n / 2)
    } else {
        f64::midpoint(at(n / 2 - 1), at(n / 2))
    }
}

/// A check for cases whose output has nothing cheap to verify.
pub(crate) fn unchecked<T>(_: &T) {}
