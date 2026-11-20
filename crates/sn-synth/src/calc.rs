//! OCG's calculators for OFDM with a cyclic prefix, as pure functions.
//!
//! At a sample rate `Fd`, an OFDM symbol is `LS = LU + LG` samples: `LU` the useful
//! part (FFT size) and `LG` the guard (cyclic prefix). Then:
//! - channel spacing `Sh = Fd / LU`, symbol rate `Br = Fd / LS`,
//! - `K = LG / LU = Sh / Br - 1` (independent of the recording's rate),
//! - guard duration `LG / Fd = (Sh - Br) / (Sh Br)`.
//!
//! [`cp_from_lengths`] goes from lengths to rates, [`cp_from_rates`] back from measured
//! rates to the nearest whole lengths at a given rate, [`cp_candidates`] lists whole
//! LU/LG pairs (and the rate each needs) for a measured `Br` and `K`, and
//! [`k_list_within`] (or [`k_list_from_measured_rates`]) lists the simplest fractions
//! within the measurement uncertainty of a measured K.
//!
//! These are fraction-based approximations of the same question SA's "List of K"
//! answers, not SA's method: SA tries sample rates `n Br` between SR and 2 SR,
//! measures LS, LU, LG and the triangle amplitude at each, drops odd LU and implausible
//! candidates, and sorts by K or amplitude. Here nothing is measured from the signal.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{Result, require, require_positive};

/// Largest number of LU values [`cp_candidates`] scans.
pub const MAX_LU_RANGE: usize = 1 << 20;
/// Largest denominator [`k_list`] accepts.
pub const MAX_K_DENOMINATOR: u32 = 1 << 16;
/// Largest LU [`cp_candidates`] accepts (2^40 samples), well inside the integers an
/// `f64` holds exactly.
pub const MAX_LU: u64 = 1 << 40;
/// Largest guard or useful length [`cp_from_rates`] gives, in samples.
const MAX_CP_LENGTH: f64 = 1e12;
/// Most fractions [`k_list_within`] returns.
pub const MAX_K_COUNT: usize = 4096;

/// OFDM-with-CP parameters derived from `Fd`, `LU` and `LG`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CpParameters {
    /// Sample rate in Hz.
    pub fd: f64,
    /// Useful length in samples.
    pub lu: f64,
    /// Guard length in samples.
    pub lg: f64,
    /// Symbol length `LU + LG` in samples.
    pub ls: f64,
    /// `K = LG / LU`.
    pub k: f64,
    /// Symbol rate `Fd / LS` in Bd.
    pub symbol_rate: f64,
    /// Channel spacing `Fd / LU` in Hz.
    pub spacing: f64,
    /// Whole symbol duration `LS / Fd` in seconds.
    pub symbol_duration: f64,
    /// Useful-part duration `LU / Fd` in seconds.
    pub useful_duration: f64,
    /// Guard duration `LG / Fd` in seconds.
    pub guard_duration: f64,
}

/// Rates and durations of OFDM with a CP from the sample rate `fd` and the lengths `lu`
/// and `lg` in samples (fractional lengths are allowed).
pub fn cp_from_lengths(fd: f64, lu: f64, lg: f64) -> Result<CpParameters> {
    require_positive(fd, "fd", "Hz")?;
    require_positive(lu, "lu", "samples")?;
    require(
        lg.is_finite() && lg >= 0.0,
        "lg",
        format!("must be a finite number of at least 0 samples (got {lg})"),
    )?;
    let ls = lu + lg;
    Ok(CpParameters {
        fd,
        lu,
        lg,
        ls,
        k: lg / lu,
        symbol_rate: fd / ls,
        spacing: fd / lu,
        symbol_duration: ls / fd,
        useful_duration: lu / fd,
        guard_duration: lg / fd,
    })
}

/// `K = spacing / br - 1` from a symbol rate and a channel spacing.
pub fn k_from_rates(br: f64, spacing: f64) -> Result<f64> {
    require_positive(br, "br", "Bd")?;
    require_positive(spacing, "spacing", "Hz")?;
    require(
        spacing >= br,
        "spacing",
        format!(
            "the channel spacing ({spacing} Hz) cannot be below the symbol rate ({br} Bd) \
             in OFDM with a cyclic prefix"
        ),
    )?;
    Ok(spacing / br - 1.0)
}

/// Whole lengths for measured rates at a sample rate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CpFromRates {
    /// `K = spacing / br - 1`.
    pub k: f64,
    /// Exact (fractional) useful length `fd / spacing`.
    pub lu_exact: f64,
    /// Exact (fractional) guard length `fd / br - fd / spacing`.
    pub lg_exact: f64,
    /// Nearest whole useful length.
    pub lu: u64,
    /// Nearest whole guard length.
    pub lg: u64,
    /// `lu - lu_exact` in samples.
    pub lu_error: f64,
    /// `lg - lg_exact` in samples.
    pub lg_error: f64,
    /// The parameters the whole lengths give at `fd`.
    pub rounded: CpParameters,
}

/// From a symbol rate `br`, a channel spacing and a sample rate `fd`: K, the exact and
/// nearest whole LU and LG, and their errors.
pub fn cp_from_rates(br: f64, spacing: f64, fd: f64) -> Result<CpFromRates> {
    let k = k_from_rates(br, spacing)?;
    require_positive(fd, "fd", "Hz")?;
    let lu_exact = fd / spacing;
    let lg_exact = fd / br - lu_exact;
    require(
        lu_exact <= MAX_CP_LENGTH,
        "fd",
        "the useful length would be above 10^12 samples",
    )?;
    require(
        lg_exact.is_finite() && lg_exact <= MAX_CP_LENGTH,
        "br",
        "the guard length would be above 10^12 samples; check the symbol rate",
    )?;
    let lu = lu_exact.round().max(1.0);
    let lg = lg_exact.round().max(0.0);
    Ok(CpFromRates {
        k,
        lu_exact,
        lg_exact,
        lu: lu as u64,
        lg: lg as u64,
        lu_error: lu - lu_exact,
        lg_error: lg - lg_exact,
        rounded: cp_from_lengths(fd, lu, lg)?,
    })
}

/// The best entries of a search, best first, with how many it found in all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ranked<T> {
    /// At most the requested number of entries, best first.
    pub best: Vec<T>,
    /// How many entries the search found, including those not kept.
    pub total: usize,
}

/// Keep the `count` smallest of `items` under `cmp`, sorted, and count them all. Uses
/// memory for at most `2 * count` items whatever the number of items.
fn keep_best<T>(
    items: impl Iterator<Item = T>,
    count: usize,
    cmp: impl Fn(&T, &T) -> Ordering,
) -> Ranked<T> {
    let mut best: Vec<T> = Vec::new();
    let mut total = 0_usize;
    let limit = count.saturating_mul(2).max(1);
    for item in items {
        total += 1;
        if count == 0 {
            continue;
        }
        best.push(item);
        if best.len() >= limit {
            // Move the `count` best to the front (in any order) and drop the rest.
            best.select_nth_unstable_by(count - 1, &cmp);
            best.truncate(count);
        }
    }
    best.sort_by(&cmp);
    best.truncate(count);
    Ranked { best, total }
}

/// One whole-length solution for a symbol rate and K.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CpCandidate {
    /// Useful length.
    pub lu: u64,
    /// Guard length (nearest whole number to `k * lu`).
    pub lg: u64,
    /// Sample rate `br * (lu + lg)` that makes these lengths exact for `br`.
    pub fd: f64,
    /// `lg - k * lu` in samples.
    pub lg_error: f64,
    /// `lg / lu`, the K these lengths really have.
    pub k: f64,
}

/// Whole LU/LG pairs for a symbol rate `br` and constant `k`, for every LU in
/// `lu_min..=lu_max` whose guard `k * lu` is within `tolerance` samples of a whole
/// number: the best `count` of them, sorted by the size of the guard error, then by
/// LU, and how many there are in all. Odd LU are kept.
pub fn cp_candidates(
    br: f64,
    k: f64,
    lu_min: u64,
    lu_max: u64,
    tolerance: f64,
    count: usize,
) -> Result<Ranked<CpCandidate>> {
    require_positive(br, "br", "Bd")?;
    require(
        k.is_finite() && (0.0..=16.0).contains(&k),
        "k",
        format!("must be between 0 and 16 (got {k})"),
    )?;
    require(
        lu_min >= 1 && lu_min <= lu_max,
        "lu_min",
        format!("need 1 <= lu_min <= lu_max (got {lu_min} and {lu_max})"),
    )?;
    require(
        lu_max - lu_min < MAX_LU_RANGE as u64,
        "lu_max",
        format!("scan at most {MAX_LU_RANGE} LU values"),
    )?;
    require(
        lu_max <= MAX_LU,
        "lu_max",
        format!("LU above 2^40 samples is not supported (got {lu_max})"),
    )?;
    require(
        tolerance.is_finite() && (0.0..=0.5).contains(&tolerance),
        "tolerance",
        format!("must be between 0 and 0.5 samples (got {tolerance})"),
    )?;
    let found = (lu_min..=lu_max).filter_map(|lu| {
        let exact = k * lu as f64;
        let lg = exact.round();
        let err = lg - exact;
        (err.abs() <= tolerance).then(|| CpCandidate {
            lu,
            lg: lg as u64,
            fd: br * (lu as f64 + lg),
            lg_error: err,
            k: lg / lu as f64,
        })
    });
    Ok(keep_best(found, count, |a, b| {
        a.lg_error
            .abs()
            .total_cmp(&b.lg_error.abs())
            .then(a.lu.cmp(&b.lu))
    }))
}

/// A simple fraction near a measured K.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KFraction {
    /// Numerator.
    pub p: u64,
    /// Denominator (the smallest LU this K allows).
    pub q: u64,
}

/// A fraction with its distance from the measured K.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KCandidate {
    /// The fraction, in lowest terms.
    pub fraction: KFraction,
    /// Its value `p / q`.
    pub value: f64,
    /// `value - k`.
    pub error: f64,
}

fn check_k_and_denominator(k: f64, max_denominator: u32) -> Result<()> {
    require(
        k.is_finite() && (0.0..=16.0).contains(&k),
        "k",
        format!("must be between 0 and 16 (got {k})"),
    )?;
    require(
        (1..=MAX_K_DENOMINATOR).contains(&max_denominator),
        "max_denominator",
        format!("must be between 1 and {MAX_K_DENOMINATOR} (got {max_denominator})"),
    )
}

/// Fractions `p / q` in lowest terms with `q <= max_denominator` nearest to `k`, at most
/// `count` of them, sorted by distance (ties: smaller denominator first), with how
/// many fractions were considered in all. For each denominator the two fractions
/// either side of `k` are considered.
///
/// Only right for a K known almost exactly: some fraction with a large denominator
/// always lies within about `1 / q^2` of any K, so a slightly wrong measured K ranks
/// such fractions above the simple one it came from. For a measured K use
/// [`k_list_within`].
pub fn k_list(k: f64, max_denominator: u32, count: usize) -> Result<Ranked<KCandidate>> {
    check_k_and_denominator(k, max_denominator)?;
    let found = (1..=u64::from(max_denominator)).flat_map(|q| {
        let below = (k * q as f64).floor().max(0.0) as u64;
        [below, below + 1]
            .into_iter()
            .filter(move |p| gcd(*p, q) == 1)
            .map(move |p| {
                let value = p as f64 / q as f64;
                KCandidate {
                    fraction: KFraction { p, q },
                    value,
                    error: value - k,
                }
            })
    });
    Ok(keep_best(found, count, |a, b| {
        a.error
            .abs()
            .total_cmp(&b.error.abs())
            .then(a.fraction.q.cmp(&b.fraction.q))
    }))
}

/// Fractions `p / q` in lowest terms with `q <= max_denominator` within `tolerance` of
/// `k`, at most `count` of them (up to [`MAX_K_COUNT`]), simplest first: sorted by
/// denominator, then by distance. Use the uncertainty of the measured K as the
/// tolerance (see [`k_list_from_measured_rates`]), so the simple K a signal was built
/// with comes first even when the measurement is slightly off.
pub fn k_list_within(
    k: f64,
    tolerance: f64,
    max_denominator: u32,
    count: usize,
) -> Result<Vec<KCandidate>> {
    check_k_and_denominator(k, max_denominator)?;
    require(
        tolerance.is_finite() && (0.0..=16.0).contains(&tolerance),
        "tolerance",
        format!("must be between 0 and 16 (got {tolerance})"),
    )?;
    require(
        count <= MAX_K_COUNT,
        "count",
        format!("list at most {MAX_K_COUNT} fractions (got {count})"),
    )?;
    let mut out = Vec::new();
    for q in 1..=u64::from(max_denominator) {
        if out.len() >= count {
            break;
        }
        let qf = q as f64;
        let lo = ((k - tolerance) * qf).ceil().max(0.0) as u64;
        let hi = ((k + tolerance) * qf).floor().max(0.0) as u64;
        let mut row: Vec<KCandidate> = (lo..=hi)
            .filter(|p| gcd(*p, q) == 1)
            .map(|p| {
                let value = p as f64 / qf;
                KCandidate {
                    fraction: KFraction { p, q },
                    value,
                    error: value - k,
                }
            })
            .filter(|c| c.error.abs() <= tolerance)
            .collect();
        row.sort_by(|a, b| a.error.abs().total_cmp(&b.error.abs()));
        out.extend(row);
    }
    out.truncate(count);
    Ok(out)
}

/// [`k_list_within`] for K measured from a symbol rate `br` and a channel spacing,
/// each with an absolute uncertainty (`br_error` in Bd, `spacing_error` in Hz). The
/// tolerance is the resulting uncertainty of `K = spacing / br - 1`:
/// `(1 + K) (br_error / br + spacing_error / spacing)`.
pub fn k_list_from_measured_rates(
    br: f64,
    br_error: f64,
    spacing: f64,
    spacing_error: f64,
    max_denominator: u32,
    count: usize,
) -> Result<Vec<KCandidate>> {
    let k = k_from_rates(br, spacing)?;
    for (e, name) in [(br_error, "br_error"), (spacing_error, "spacing_error")] {
        require(
            e.is_finite() && e >= 0.0,
            name,
            format!("must be a finite number of at least 0 (got {e})"),
        )?;
    }
    let tolerance = (1.0 + k) * (br_error / br + spacing_error / spacing);
    k_list_within(k, tolerance.min(16.0), max_denominator, count)
}

/// [`k_list`] from a symbol rate and channel spacing known almost exactly (see the
/// caveat there; for measured rates use [`k_list_from_measured_rates`]).
pub fn k_list_from_rates(
    br: f64,
    spacing: f64,
    max_denominator: u32,
    count: usize,
) -> Result<Ranked<KCandidate>> {
    k_list(k_from_rates(br, spacing)?, max_denominator, count)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn lu64_lg20_at_7000() {
        let p = cp_from_lengths(7000.0, 64.0, 20.0).unwrap();
        assert!(close(p.ls, 84.0, 0.0));
        assert!(close(p.symbol_rate, 83.333, 5e-4), "{}", p.symbol_rate);
        assert!(close(p.spacing, 109.375, 1e-12));
        assert!(close(p.k, 0.3125, 1e-15));
        assert!(close(p.guard_duration, 20.0 / 7000.0, 1e-15));
        assert!(close(p.symbol_duration, 0.012, 1e-15));
        // The guard duration identity (Sh - Br) / (Sh Br).
        let g = (p.spacing - p.symbol_rate) / (p.spacing * p.symbol_rate);
        assert!(close(g, p.guard_duration, 1e-15));
        // And back.
        let r = cp_from_rates(p.symbol_rate, p.spacing, 7000.0).unwrap();
        assert_eq!((r.lu, r.lg), (64, 20));
        assert!(r.lu_error.abs() < 1e-9 && r.lg_error.abs() < 1e-9);
    }

    #[test]
    fn thirty_channel_example() {
        // 30 channels, spacing 80 Hz, Fd 6400, LS 96, LG 16: 66.67 Bd.
        let p = cp_from_lengths(6400.0, 80.0, 16.0).unwrap();
        assert!(close(p.spacing, 80.0, 1e-12));
        assert!(close(p.symbol_rate, 66.667, 5e-4));
        assert!(close(p.ls, 96.0, 0.0));
    }

    #[test]
    fn cis12_k_is_two_thirds() {
        let p = cp_from_lengths(7200.0, 36.0, 24.0).unwrap();
        assert!(close(p.k, 2.0 / 3.0, 1e-15));
        let k = k_from_rates(p.symbol_rate, p.spacing).unwrap();
        assert!(close(k, 2.0 / 3.0, 1e-12));
        let list = k_list_from_rates(p.symbol_rate, p.spacing, 256, 10)
            .unwrap()
            .best;
        assert_eq!(list[0].fraction, KFraction { p: 2, q: 3 });
        assert!(list[0].error.abs() < 1e-12);
        // Nothing but reduced fractions.
        assert!(list.iter().all(|c| gcd(c.fraction.p, c.fraction.q) == 1));
    }

    #[test]
    fn mil_188_110b_example() {
        // Br 43.298284 Bd, K = 17/64, LS 162: Fd = 162 x Br = 7014.32 Hz.
        let br = 43.298_284;
        let c = cp_candidates(br, 17.0 / 64.0, 100, 200, 1e-9, 100)
            .unwrap()
            .best;
        let hit = c.iter().find(|c| c.lu == 128).unwrap();
        assert_eq!(hit.lg, 34);
        assert!(close(hit.fd, 7014.32, 0.005), "{}", hit.fd);
        assert!(close(hit.fd, 162.0 * br, 1e-9));
        // Only multiples of 64 give a whole guard exactly.
        assert!(c.iter().all(|c| c.lu % 64 == 0));
        // The K list from the measured rates puts 17/64 first.
        let spacing = 7014.32 / 128.0;
        let list = k_list_from_rates(br, spacing, 256, 5).unwrap().best;
        assert_eq!(list[0].fraction, KFraction { p: 17, q: 64 });
        // Inverse at the native rate.
        let r = cp_from_rates(br, spacing, 162.0 * br).unwrap();
        assert_eq!((r.lu, r.lg), (128, 34));
    }

    #[test]
    fn k_list_orders_by_error() {
        let list = k_list(0.25, 32, 6).unwrap().best;
        assert_eq!(list[0].fraction, KFraction { p: 1, q: 4 });
        for w in list.windows(2) {
            assert!(w[0].error.abs() <= w[1].error.abs());
        }
        assert_eq!(
            k_list(0.0, 8, 1).unwrap().best[0].fraction,
            KFraction { p: 0, q: 1 }
        );
    }

    #[test]
    fn simple_k_comes_first_when_the_measurement_is_slightly_off() {
        for (p, q) in [(7, 8), (2, 3), (1, 9), (1, 2), (1, 4), (5, 16)] {
            let k_true = p as f64 / q as f64;
            for off in [1.001, 0.999] {
                let k = k_true * off;
                let tolerance = (1.0 + k_true) * 2e-3;
                let list = k_list_within(k, tolerance, 256, 5).unwrap();
                assert_eq!(
                    list[0].fraction,
                    KFraction { p, q },
                    "{p}/{q} x {off}: {list:?}"
                );
                for w in list.windows(2) {
                    assert!(w[0].fraction.q <= w[1].fraction.q);
                }
                assert!(list.iter().all(|c| c.error.abs() <= tolerance));
            }
        }
        // From measured rates: Br 120 Bd and spacing 200 Hz (K = 2/3), each 0.1% off.
        let list = k_list_from_measured_rates(120.12, 0.15, 199.8, 0.25, 256, 3).unwrap();
        assert_eq!(list[0].fraction, KFraction { p: 2, q: 3 });
        // The plain distance ranking loses 7/8 here, which is why it is not for measured K.
        let plain = k_list(0.875 * 1.001, 256, 5).unwrap().best;
        assert!(plain.iter().all(|c| c.fraction != KFraction { p: 7, q: 8 }));
        assert!(k_list_within(0.5, f64::NAN, 16, 5).is_err());
        assert!(k_list_within(0.5, 0.01, 16, MAX_K_COUNT + 1).is_err());
        assert!(k_list_from_measured_rates(100.0, -1.0, 150.0, 0.0, 16, 5).is_err());
    }

    #[test]
    fn candidates_with_tolerance_are_sorted() {
        let c = cp_candidates(100.0, 0.3, 10, 60, 0.2, 1000).unwrap().best;
        assert_ne!(c.len(), 0);
        for w in c.windows(2) {
            assert!(w[0].lg_error.abs() <= w[1].lg_error.abs());
        }
        assert!(c.iter().all(|c| c.lg_error.abs() <= 0.2));
    }

    #[test]
    fn bounded_results_keep_the_best_and_count_them_all() {
        // With a tolerance of half a sample every LU is a hit.
        let all = cp_candidates(100.0, 0.25, 1, 4000, 0.5, 5000).unwrap();
        let few = cp_candidates(100.0, 0.25, 1, 4000, 0.5, 7).unwrap();
        assert_eq!((all.total, few.total), (4000, 4000));
        assert_eq!(all.best.len(), 4000);
        assert_eq!(few.best, all.best[..7]);
        let none = cp_candidates(100.0, 0.25, 1, 4000, 0.5, 0).unwrap();
        assert_eq!((none.best.len(), none.total), (0, 4000));
        // A large scan keeps only what was asked for.
        let big = cp_candidates(100.0, 0.3, 1, MAX_LU_RANGE as u64, 0.5, 10).unwrap();
        assert_eq!((big.best.len(), big.total), (10, MAX_LU_RANGE));
        for w in big.best.windows(2) {
            assert!(w[0].lg_error.abs() <= w[1].lg_error.abs());
        }

        let all = k_list(0.3, 512, usize::MAX).unwrap();
        let few = k_list(0.3, 512, 9).unwrap();
        assert_eq!(all.total, few.total);
        assert_eq!(all.best.len(), all.total);
        assert_eq!(few.best, all.best[..9]);
    }

    #[test]
    fn validation() {
        assert!(cp_from_lengths(0.0, 64.0, 20.0).is_err());
        assert!(cp_from_lengths(7000.0, 0.0, 20.0).is_err());
        assert!(cp_from_lengths(7000.0, 64.0, -1.0).is_err());
        assert!(k_from_rates(100.0, 50.0).is_err());
        assert!(cp_from_rates(f64::NAN, 100.0, 7000.0).is_err());
        assert!(cp_candidates(100.0, 0.25, 0, 10, 0.1, 10).is_err());
        assert!(cp_candidates(100.0, 0.25, 1, u64::MAX, 0.1, 10).is_err());
        // Above 2^40 the f64 arithmetic would report every LU as an exact hit.
        assert!(cp_candidates(100.0, 0.3, u64::MAX - 10, u64::MAX, 0.1, 10).is_err());
        assert!(cp_candidates(100.0, 0.3, MAX_LU - 10, MAX_LU, 0.1, 10).is_ok());
        // A tiny symbol rate gives an enormous guard: an error naming the rate.
        let e = cp_from_rates(1e-300, 1.0, 7000.0).unwrap_err().to_string();
        assert!(e.contains("br") && e.contains("guard"), "{e}");
        assert!(k_list(0.25, 0, 5).is_err());
        assert!(k_list(f64::INFINITY, 16, 5).is_err());
    }
}
