//! FSK/MFSK tone grids (SA: "Count freqs").
//!
//! SA counts the size of the *grid* the tones are formed on, not the number of tones
//! actually used: with tones at grid positions 0, 1, 3 and 31 the answer is 32. This
//! module finds the smallest regular grid between the outermost tones (or the level
//! markers) that every observed tone fits.

use crate::{Error, Result};

/// A regular tone grid.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToneGrid {
    /// Number of grid positions between (and including) the outermost tones.
    pub size: usize,
    /// Spacing between grid positions in Hz.
    pub spacing: f64,
    /// Lowest grid position in Hz.
    pub lowest: f64,
    /// How many grid positions hold an observed tone.
    pub occupied: usize,
}

/// Largest grid searched (SA reliably reached about 260, sometimes 300-320).
pub const MAX_GRID: usize = 400;

/// Fit a grid to `tones` (Hz). `lo`/`hi` override the outermost tones (SA: the H
/// markers on the extreme channels). `tolerance` is the allowed misfit as a fraction
/// of the spacing, in `(0, 0.5)` (0.2 is a good default).
///
/// A tolerance relative to the spacing is loose for small grids (wide spacing); when
/// the measurement accuracy of the tones is known, prefer [`count_grid_hz`].
pub fn count_grid(
    tones: &[f64],
    lo: Option<f64>,
    hi: Option<f64>,
    tolerance: f64,
) -> Result<ToneGrid> {
    fit(tones, lo, hi, tolerance, f64::INFINITY)
}

/// Fit a grid to `tones` (Hz) with an absolute tolerance: every tone must lie within
/// `tol_hz` of its grid position (and within 0.2 of the spacing). Use the accuracy
/// of the tone estimates, e.g. twice the histogram bin width or the smoothing width.
/// `lo`/`hi` as for [`count_grid`].
pub fn count_grid_hz(
    tones: &[f64],
    lo: Option<f64>,
    hi: Option<f64>,
    tol_hz: f64,
) -> Result<ToneGrid> {
    if !(tol_hz.is_finite() && tol_hz > 0.0) {
        return Err(Error::Invalid(
            "the tone tolerance must be a positive number of Hz".into(),
        ));
    }
    fit(tones, lo, hi, 0.2, tol_hz)
}

fn fit(
    tones: &[f64],
    lo: Option<f64>,
    hi: Option<f64>,
    tolerance: f64,
    tol_hz: f64,
) -> Result<ToneGrid> {
    if !(tolerance > 0.0 && tolerance < 0.5) {
        return Err(Error::Invalid(
            "the tolerance must be between 0 and 0.5 of the spacing".into(),
        ));
    }
    if lo.is_some_and(|v| !v.is_finite()) || hi.is_some_and(|v| !v.is_finite()) {
        return Err(Error::Invalid(
            "the level markers must be at finite frequencies".into(),
        ));
    }
    let finite: Vec<f64> = tones.iter().copied().filter(|t| t.is_finite()).collect();
    let min = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let max = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let both_markers = lo.is_some() && hi.is_some();
    if finite.len() < if both_markers { 1 } else { 2 } {
        return Err(Error::Invalid(
            "mark at least two tones, or one tone and two level markers on the outermost \
             tones"
                .into(),
        ));
    }
    let lo = lo.unwrap_or(min);
    let hi = hi.unwrap_or(max);
    if hi <= lo {
        return Err(Error::Invalid(
            "the outermost tones (or level markers) must differ in frequency".into(),
        ));
    }
    for size in 2..=MAX_GRID {
        let spacing = (hi - lo) / (size - 1) as f64;
        let allowed = (tolerance * spacing).min(tol_hz);
        // Tones outside the grid (beyond the markers) do not belong to it.
        let inside: Vec<f64> = finite
            .iter()
            .copied()
            .filter(|t| *t >= lo - allowed && *t <= hi + allowed)
            .collect();
        if inside.is_empty() {
            continue;
        }
        let last = (size - 1) as f64;
        let positions: Vec<f64> = inside
            .iter()
            .map(|t| ((t - lo) / spacing).round().clamp(0.0, last))
            .collect();
        // The coarse prediction comes from the measured ends, whose errors enter it as
        // a convex combination (at most `allowed` together) on top of each tone's own
        // `allowed`, so an in-tolerance tone can sit up to twice `allowed` from it.
        //
        // When the absolute tolerance binds (`tol_hz`, a physical measurement
        // accuracy), the check allows twice `tol_hz` and the minimax test below at
        // `tol_hz` decides. When the relative tolerance binds, it stays at
        // `tolerance` of the spacing: the minimax test alone can shift the grid by
        // `allowed` from both ends, which at a fraction of a wide spacing lets a
        // coarse grid swallow tones between its positions (markers at 1000 and 1300
        // Hz with tones at 1100 and 1200 Hz would read as a 2-grid). Rounding to the
        // nearest position stays correct since twice `tol_hz` is then below 0.4 of
        // the spacing.
        let relative = tolerance * spacing;
        let coarse_allowed = if tol_hz < relative {
            2.0 * tol_hz
        } else {
            relative
        };
        let coarse_fit = inside
            .iter()
            .zip(&positions)
            .all(|(t, p)| (t - (lo + p * spacing)).abs() <= coarse_allowed);
        if !coarse_fit {
            continue;
        }
        // The real test: some regular grid with this many positions must hold every
        // tone (the ends included) within `allowed` of its position. This is a
        // minimax fit, not least squares: a least-squares line through points that
        // all lie within `tol_hz` of the true grid can leave residuals above it (ends
        // at +e and the interior at -e give 1.2 e at the ends), which would reject the
        // true grid and let a finer, wrong one through.
        let mut pts: Vec<(f64, f64)> = positions
            .iter()
            .copied()
            .zip(inside.iter().copied())
            .collect();
        pts.push((0.0, lo));
        pts.push((last, hi));
        if !fits_within(&pts, allowed, hi - lo, last) {
            continue;
        }
        // Report the least-squares offset and spacing (the ends counted twice).
        let (a, s) = least_squares(&pts).unwrap_or((lo, spacing));
        let mut used: Vec<i64> = positions.iter().map(|p| *p as i64).collect();
        used.sort_unstable();
        used.dedup();
        return Ok(ToneGrid {
            size,
            spacing: s,
            lowest: a,
            occupied: used.len(),
        });
    }
    Err(Error::NotFound(format!(
        "no regular grid of up to {MAX_GRID} positions fits these tones; check the markers \
         and the filtering after the phase detector"
    )))
}

/// Whether some line `y = a + s x` passes within `allowed` of every point of `pts`
/// (x, y). `span` is the distance between the ends of the grid, at x = 0 and `last`.
///
/// For a slope `s` the spread `max(y - s x) - min(y - s x)` is convex and piecewise
/// linear in `s`, and an offset exists exactly when the spread is at most twice
/// `allowed`. The minimum is found by ternary search over the slopes that the two
/// ends permit.
fn fits_within(pts: &[(f64, f64)], allowed: f64, span: f64, last: f64) -> bool {
    let spread = |s: f64| {
        let (lo, hi) = pts
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |acc, p| {
                let r = p.1 - s * p.0;
                (acc.0.min(r), acc.1.max(r))
            });
        hi - lo
    };
    if allowed.is_infinite() {
        return true;
    }
    let scale = pts.iter().fold(0.0_f64, |m, p| m.max(p.1.abs()));
    if !(last > 0.0 && allowed >= 0.0 && span.is_finite() && scale.is_finite()) {
        return false;
    }
    // Any feasible slope keeps both ends within `allowed` of the line.
    let slack = 1e-9 * (scale + allowed);
    let mut a = (span - 2.0 * allowed - slack) / last;
    let mut b = (span + 2.0 * allowed + slack) / last;
    for _ in 0..200 {
        let m1 = a + (b - a) / 3.0;
        let m2 = b - (b - a) / 3.0;
        if spread(m1) <= spread(m2) {
            b = m2;
        } else {
            a = m1;
        }
    }
    spread(f64::midpoint(a, b)) <= 2.0 * allowed + slack
}

/// Least-squares line `y = a + s x` through `pts` (x, y). None when degenerate.
fn least_squares(pts: &[(f64, f64)]) -> Option<(f64, f64)> {
    let n = pts.len() as f64;
    let mx = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let my = pts.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    if sxx.is_nan() || sxx <= 0.0 {
        return None;
    }
    let s = sxy / sxx;
    let a = my - s * mx;
    (a.is_finite() && s.is_finite() && s > 0.0).then_some((a, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_and_sparse_grids() {
        let full: Vec<f64> = (0..8).map(|k| 1000.0 + 242.0 * f64::from(k)).collect();
        let g = count_grid(&full, None, None, 0.2).unwrap();
        assert_eq!((g.size, g.occupied), (8, 8));
        assert!((g.spacing - 242.0).abs() < 1e-9);
        // 32-tone grid, only some tones used.
        let used = [0, 1, 3, 7, 12, 20, 31];
        let sparse: Vec<f64> = used.iter().map(|k| 500.0 + 31.25 * f64::from(*k)).collect();
        let g = count_grid(&sparse, None, None, 0.2).unwrap();
        assert_eq!((g.size, g.occupied), (32, 7));
        // With no known spacing, three equally spaced tones form a 3-grid.
        let g = count_grid(&[0.0, 40.0, 80.0], None, None, 0.2).unwrap();
        assert_eq!(g.size, 3);
        assert!(count_grid(&[5.0], None, None, 0.2).is_err());
    }

    #[test]
    fn sa_example_sparse_81_grid() {
        // SA's example: channels 0 and 80 present on a 1-Hz-like grid with about a
        // quarter of the channels in between: the grid has 81 positions.
        let used = [
            0, 3, 7, 11, 13, 19, 22, 29, 31, 37, 41, 44, 50, 53, 59, 61, 67, 71, 73, 80,
        ];
        let tones: Vec<f64> = used.iter().map(|k| 1000.0 + 1.0 * f64::from(*k)).collect();
        let g = count_grid(&tones, None, None, 0.2).unwrap();
        assert_eq!((g.size, g.occupied), (81, used.len()));
        let g = count_grid_hz(&tones, None, None, 0.05).unwrap();
        assert_eq!(g.size, 81);
    }

    #[test]
    fn jt65a_reads_as_66() {
        // JT65A: sync tone at position 0, 64 data tones at positions 2..=65, spacing
        // 2.6917 Hz. Only some data tones are seen; measured to about 0.1 Hz.
        let spacing = 11025.0 / 4096.0;
        let mut tones = vec![1270.5];
        for (i, k) in [2, 5, 9, 14, 20, 27, 33, 40, 48, 52, 57, 61, 65]
            .iter()
            .enumerate()
        {
            let err = if i % 2 == 0 { 0.08 } else { -0.07 };
            tones.push(1270.5 + spacing * f64::from(*k) + err);
        }
        let g = count_grid_hz(&tones, None, None, 0.3).unwrap();
        assert_eq!(g.size, 66);
        assert!((g.spacing - spacing).abs() < 0.01, "{g:?}");
    }

    #[test]
    fn outer_tone_errors_do_not_hide_the_true_grid() {
        // Reviewer's case: 8 tones on a 10 Hz grid with errors up to 0.2 Hz.
        let errs = [0.2, -0.2, 0.0, 0.1, -0.1, 0.0, 0.2, -0.2];
        let tones: Vec<f64> = errs
            .iter()
            .enumerate()
            .map(|(k, e)| 1000.0 + 10.0 * k as f64 + e)
            .collect();
        let g = count_grid_hz(&tones, None, None, 0.3).unwrap();
        assert_eq!((g.size, g.occupied), (8, 8), "{g:?}");
        assert!((g.spacing - 10.0).abs() < 0.1, "{g:?}");
        // Errors of 0.7 tol_hz on both ends and the opposite sign on an inner tone.
        let tol = 0.3;
        let mut tones: Vec<f64> = (0..8).map(|k| 1000.0 + 10.0 * f64::from(k)).collect();
        for (k, e) in [(0, 0.7), (7, 0.7), (3, -0.7)] {
            if let Some(t) = tones.get_mut(k) {
                *t += e * tol;
            }
        }
        let g = count_grid_hz(&tones, None, None, tol).unwrap();
        assert_eq!((g.size, g.occupied), (8, 8), "{g:?}");
    }

    #[test]
    fn in_tolerance_alternating_errors_keep_the_true_grid() {
        // 8 tones at 100 Hz spacing, errors of 0.9 tol_hz: + at the ends, alternating
        // in the interior. The least-squares refit leaves end residuals above tol_hz;
        // this must not reject the 8-grid and fall through to a spurious finer grid.
        let tol = 5.0;
        for sign in [1.0, -1.0] {
            let tones: Vec<f64> = (0..8)
                .map(|k| {
                    let e = if k == 0 || k == 7 || k % 2 == 1 {
                        0.9 * tol
                    } else {
                        -0.9 * tol
                    };
                    1000.0 + 100.0 * f64::from(k) + sign * e
                })
                .collect();
            let g = count_grid_hz(&tones, None, None, tol).unwrap();
            assert_eq!((g.size, g.occupied), (8, 8), "{g:?}");
            assert!((g.spacing - 100.0).abs() < tol, "{g:?}");
            // Ends +e, every interior tone -e: the worst case for the refit.
            let tones: Vec<f64> = (0..8)
                .map(|k| {
                    let e = if k == 0 || k == 7 { 0.9 } else { -0.9 };
                    1000.0 + 100.0 * f64::from(k) + sign * e * tol
                })
                .collect();
            let g = count_grid_hz(&tones, None, None, tol).unwrap();
            assert_eq!((g.size, g.occupied), (8, 8), "{g:?}");
        }
    }

    #[test]
    fn absolute_tolerance_allows_end_errors_in_the_coarse_check() {
        // Reviewer's case: 10 Hz grid, errors of +-1.4 Hz at k = 0, 1, 6, 7, all within
        // tol_hz = 1.5 (where 0.2 of the spacing, 2 Hz, does not bind).
        let errs = [1.4, -1.4, 0.0, 0.0, 0.0, 0.0, 1.4, -1.4];
        let tones: Vec<f64> = errs
            .iter()
            .enumerate()
            .map(|(k, e)| 1000.0 + 10.0 * k as f64 + e)
            .collect();
        let g = count_grid_hz(&tones, None, None, 1.5).unwrap();
        assert_eq!((g.size, g.occupied), (8, 8), "{g:?}");
        // tol_hz = 0.15 of the spacing, errors of 0.9 tol_hz alternating between the
        // ends and the interior.
        let spacing = 10.0;
        let tol = 0.15 * spacing;
        for sign in [1.0, -1.0] {
            let tones: Vec<f64> = (0..8)
                .map(|k| {
                    let e = if k == 0 || k == 7 { 0.9 } else { -0.9 };
                    1000.0 + spacing * f64::from(k) + sign * e * tol
                })
                .collect();
            let g = count_grid_hz(&tones, None, None, tol).unwrap();
            assert_eq!((g.size, g.occupied), (8, 8), "{g:?}");
            assert!((g.spacing - spacing).abs() < tol, "{g:?}");
        }
        // With only a relative tolerance the same pattern (adjacent tones 0.19 of the
        // spacing off in opposite directions) is ambiguous: a 2-grid would hold
        // markers at 1000 and 1300 Hz with tones at 1100 and 1200 Hz just as well, so
        // the coarse check stays strict there.
        let g = count_grid(&[1100.0, 1200.0], Some(1000.0), Some(1300.0), 0.2).unwrap();
        assert_eq!(g.size, 4, "{g:?}");
    }

    #[test]
    fn absolute_tolerance_rejects_coarse_grids() {
        // A 32-grid with 5 Hz spacing at positions 0, 5, 13, 31. (With 0.5 Hz accuracy
        // a 20-grid of 8.15 Hz also fits every tone within 0.45 Hz, so the tighter
        // accuracy is needed to tell them apart.)
        let g = count_grid_hz(&[0.0, 25.0, 65.0, 155.0], None, None, 0.2).unwrap();
        assert_eq!(g.size, 32);
        // 45 Hz is 5 Hz off a 50 Hz grid; with 0.2 Hz accuracy it cannot be a 3-grid.
        // (With 0.3 Hz a 10-grid of 11.13 Hz holds all three tones within 0.28 Hz.)
        let g = count_grid_hz(&[0.0, 45.0, 100.0], None, None, 0.2).unwrap();
        assert_eq!(g.size, 21);
        let g = count_grid_hz(&[0.0, 45.0, 100.0], None, None, 0.3).unwrap();
        assert_eq!(g.size, 10);
        assert!(count_grid_hz(&[0.0, 1.0], None, None, 0.0).is_err());
    }

    #[test]
    fn markers_override_the_extremes() {
        let g = count_grid(&[1100.0, 1200.0], Some(1000.0), Some(1300.0), 0.2).unwrap();
        assert_eq!((g.size, g.occupied), (4, 2));
        // One tone plus both markers is enough.
        let g = count_grid(&[1100.0], Some(1000.0), Some(1300.0), 0.2).unwrap();
        assert_eq!((g.size, g.occupied), (4, 1));
    }

    #[test]
    fn tones_outside_the_markers_are_ignored() {
        // A 300-position grid defined by the markers; tones two spacings outside it
        // (within 1% of the span) must not count or stretch the grid.
        let used = [0, 1, 5, 17, 40, 99, 150, 233, 298, 299];
        let tones: Vec<f64> = used
            .iter()
            .map(|k| f64::from(*k))
            .chain([-2.0, 301.0])
            .collect();
        let g = count_grid(&tones, Some(0.0), Some(299.0), 0.2).unwrap();
        assert_eq!((g.size, g.occupied), (300, used.len()));
    }

    #[test]
    fn bad_markers_and_tolerances() {
        let t = [1.0, 2.0, 3.5];
        assert!(count_grid(&t, Some(0.0), Some(f64::INFINITY), 0.2).is_err());
        assert!(count_grid(&t, Some(f64::NEG_INFINITY), None, 0.2).is_err());
        assert!(count_grid(&t, Some(f64::NAN), None, 0.2).is_err());
        for tol in [0.6, 0.5, 0.0, -0.1, f64::NAN, f64::INFINITY] {
            let e = count_grid(&[100.0, 101.0, 200.0], None, None, tol).unwrap_err();
            assert!(e.to_string().contains("tolerance"), "{tol}: {e}");
        }
    }
}
