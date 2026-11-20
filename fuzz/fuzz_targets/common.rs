//! Helpers shared by the fuzz targets.
#![allow(dead_code)]

use std::path::PathBuf;

use num_complex::Complex32;
use sn_core::Signal;

/// A per-process scratch directory for targets that must go through files.
pub fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sn-fuzz-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Read up to `max` samples of every kind, as the viewer would.
pub fn read_some(signal: &Signal, max: usize) {
    let n = signal.len().min(max);
    let mut c = vec![Complex32::new(0.0, 0.0); n];
    let _ = signal.read_complex(0, &mut c);
    let mut r = vec![0.0f32; n];
    let _ = signal.read_real(signal.len().saturating_sub(n), &mut r);
    let _ = signal.slice(signal.len() / 3, signal.len() / 2);
}
