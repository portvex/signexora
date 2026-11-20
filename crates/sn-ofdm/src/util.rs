//! Small numeric helpers shared by the OFDM tools.

use num_complex::Complex64;
use sn_core::Complex32;

pub(crate) fn c64(v: Complex32) -> Complex64 {
    Complex64::new(f64::from(v.re), f64::from(v.im))
}

/// `p[n] = x[n] * conj(x[n + lag])` for `n` in `from..to` (clipped to the data).
pub(crate) fn lag_product(x: &[Complex32], lag: usize, from: usize, to: usize) -> Vec<Complex32> {
    let a = x.get(from..).unwrap_or(&[]);
    let b = x.get(from.saturating_add(lag)..).unwrap_or(&[]);
    a.iter()
        .zip(b)
        .take(to.saturating_sub(from))
        .map(|(u, v)| u * v.conj())
        .collect()
}

/// Cyclic sums of `lg` consecutive values: `out[m] = sum_{i<lg} f[(m+i) mod len]`.
pub(crate) fn window_sums(f: &[Complex64], lg: usize) -> Vec<Complex64> {
    let n = f.len();
    if n == 0 {
        return Vec::new();
    }
    let at = |i: usize| f.get(i % n).copied().unwrap_or_default();
    let mut s: Complex64 = (0..lg).map(at).sum();
    let mut out = Vec::with_capacity(n);
    for m in 0..n {
        out.push(s);
        s += at(m + lg) - at(m);
    }
    out
}

/// Index and value of the largest `key`.
pub(crate) fn argmax<T>(v: &[T], key: impl Fn(&T) -> f64) -> Option<(usize, f64)> {
    v.iter()
        .enumerate()
        .map(|(i, x)| (i, key(x)))
        .filter(|(_, k)| k.is_finite())
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// Maximise a unimodal `f` on `[a, b]` by golden-section search.
pub(crate) fn golden_max(mut f: impl FnMut(f64) -> f64, a: f64, b: f64, tol: f64) -> f64 {
    let g = (5_f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (a.min(b), a.max(b));
    let mut c = b - g * (b - a);
    let mut d = a + g * (b - a);
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..200 {
        if (b - a).abs() <= tol {
            break;
        }
        if fc >= fd {
            b = d;
            d = c;
            fd = fc;
            c = b - g * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + g * (b - a);
            fd = f(d);
        }
    }
    f64::midpoint(a, b)
}

/// Offset in `-0.5..=0.5` of the apex of a triangle (two straight slopes of equal
/// steepness) through three equally spaced points, `y0` being the largest. A parabola
/// fitted to a triangle is biased (it gives `d / (2 (1 - d))` for an apex at `d`);
/// this is exact for a triangle. Returns 0 when the points do not form a peak.
pub(crate) fn triangle_vertex(ym: f64, y0: f64, yp: f64) -> f64 {
    if !(ym.is_finite() && y0.is_finite() && yp.is_finite()) || y0 < ym || y0 < yp {
        return 0.0;
    }
    let low = ym.min(yp);
    let den = 2.0 * (y0 - low);
    if den <= 0.0 {
        return 0.0;
    }
    ((yp - ym) / den).clamp(-0.5, 0.5)
}

/// Sums of `lg` consecutive values (not cyclic): `out[m] = sum_{i<lg} f[m + i]` for
/// `m` in `0..=f.len() - lg`, by a running sum. Empty when `lg` is 0 or too long.
pub(crate) fn sliding_sums<T>(f: &[T], lg: usize) -> Vec<T>
where
    T: Copy + Default + std::ops::AddAssign + std::ops::SubAssign,
{
    if lg == 0 || lg > f.len() {
        return Vec::new();
    }
    let mut s = T::default();
    for v in f.iter().take(lg) {
        s += *v;
    }
    let mut out = Vec::with_capacity(f.len() - lg + 1);
    out.push(s);
    for (head, tail) in f.iter().skip(lg).zip(f) {
        s += *head;
        s -= *tail;
        out.push(s);
    }
    out
}

/// Half the length of the interpolation kernel.
pub(crate) const TAP_HALF: usize = 32;
/// Number of interpolation taps.
pub(crate) const TAPS: usize = 2 * TAP_HALF;

/// Taps of the band-limited interpolator (Blackman-windowed sinc, 64 taps: accurate up
/// to about 0.45 of the sample rate) for the fractional position `frac` in `0..1`.
/// The value at `base + frac` is `sum_j taps[j] * x[base + j + 1 - TAP_HALF]`. Every
/// position with the same fractional part shares these taps, so build them once.
pub(crate) fn taps(frac: f64) -> [f64; TAPS] {
    let h = TAP_HALF as f64;
    let mut out = [0.0; TAPS];
    for (t, j) in out.iter_mut().zip(1 - TAP_HALF as i64..=TAP_HALF as i64) {
        let d = j as f64 - frac;
        let sinc = if d.abs() < 1e-12 {
            1.0
        } else {
            (std::f64::consts::PI * d).sin() / (std::f64::consts::PI * d)
        };
        let u = (d + h) / (2.0 * h);
        let w = if (0.0..=1.0).contains(&u) {
            let a = std::f64::consts::TAU * u;
            0.42 - 0.5 * a.cos() + 0.08 * (2.0 * a).cos()
        } else {
            0.0
        };
        *t = sinc * w;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_finds_the_vertex() {
        let x = golden_max(|x| -(x - 0.3).powi(2), -1.0, 1.0, 1e-9);
        assert!((x - 0.3).abs() < 1e-6);
    }

    #[test]
    fn interp_reproduces_a_tone() {
        let x: Vec<Complex32> = (0..200)
            .map(|n| {
                let ph = 0.7 * n as f32;
                Complex32::new(ph.cos(), ph.sin())
            })
            .collect();
        let h = taps(0.37);
        let v: Complex64 = h
            .iter()
            .zip(x.iter().skip(100 + 1 - TAP_HALF))
            .map(|(t, v)| c64(*v) * *t)
            .sum();
        let want = Complex64::from_polar(1.0, 0.7 * 100.37);
        assert!((v - want).norm() < 2e-3, "{v} vs {want}");
    }

    #[test]
    fn triangle_vertex_is_exact_on_a_triangle() {
        for apex in [-0.4, -0.1, 0.0, 0.2, 0.45] {
            let y = |x: f64| 10.0 - 3.0 * (x - apex).abs();
            let d = triangle_vertex(y(-1.0), y(0.0), y(1.0));
            assert!((d - apex).abs() < 1e-12, "{d} vs {apex}");
        }
    }

    #[test]
    fn sliding_sums_match_direct_sums() {
        let f: Vec<f64> = (0..7).map(f64::from).collect();
        assert_eq!(sliding_sums(&f, 3), vec![3.0, 6.0, 9.0, 12.0, 15.0]);
        assert_eq!(sliding_sums(&f, 8), Vec::<f64>::new());
    }

    #[test]
    fn window_sums_are_cyclic() {
        let f: Vec<Complex64> = (0..5).map(|i| Complex64::new(f64::from(i), 0.0)).collect();
        let w = window_sums(&f, 2);
        let re: Vec<f64> = w.iter().map(|c| c.re).collect();
        assert_eq!(re, vec![1.0, 3.0, 5.0, 7.0, 4.0]);
    }
}
