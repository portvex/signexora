//! What the PhasePlane form draws, independent of the UI toolkit
//! (`SA_UI_SPEC.md` section 6): the form's size, the constellation image, the
//! averaged spectrum's columns, the corrector trace and its lines, the angles
//! histogram's bars, the marker colours, the status bar texts and the mouse
//! rules of those plots. Moved from `sa-app`'s `phase_window.rs` (which keeps
//! its own copy until the egui app is removed); the state is in
//! [`crate::phase`].

use sn_core::{Complex32, SampleKind};

use crate::color::Rgba8;
use crate::format;
use crate::phase::{
    CorrSpec, Form, HiresSpec, IsdMode, MAX_MARKS, Mode, PhaseState, SpecMarks, TRAJ_PER_SYMBOL,
};
use crate::t;
use crate::theme::Palette;

/// Size of the form (the window body below the title bar). SA's form does not
/// scale.
pub const FORM: (i32, i32) = (936, 440);

/// Most symbols drawn per frame; longer spans are thinned evenly.
pub const MAX_DRAWN: usize = 60_000;

/// Readouts of the status bar, from the pointer over a plot.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Hover {
    /// Frequency under the pointer in the corrector plot.
    pub f: Option<f64>,
    /// Direct angle (0 to the pointer) in the angles histogram.
    pub angle: Option<f64>,
}

/// Hits per pixel of the shown symbols (weight 4) and, with Transitions, of the
/// paths between them (weight 1). `scale` is the radius of RMS 1 as a fraction of
/// the half side, `trim` turns everything (degrees).
pub fn density(
    points: &[Complex32],
    path: Option<&[Complex32]>,
    side: usize,
    scale: f32,
    trim: f32,
) -> Vec<u32> {
    let mut hits = vec![0_u32; side * side];
    let half = side as f32 / 2.0;
    let rot = Complex32::from_polar(1.0, trim.to_radians());
    let to = |p: &Complex32| {
        let q = p * rot;
        (half + q.re * half * scale, half - q.im * half * scale)
    };
    let mut hit = |x: f32, y: f32, w: u32| {
        if x >= 0.0
            && y >= 0.0
            && (x as usize) < side
            && (y as usize) < side
            && let Some(h) = hits.get_mut(y as usize * side + x as usize)
        {
            *h = h.saturating_add(w);
        }
    };
    if let Some(path) = path {
        for w in path.windows(2) {
            if let [a, b] = w {
                let (ax, ay) = to(a);
                let (bx, by) = to(b);
                let steps = (bx - ax)
                    .abs()
                    .max((by - ay).abs())
                    .ceil()
                    .clamp(1.0, side as f32) as usize;
                for s in 0..steps {
                    let t = s as f32 / steps as f32;
                    hit(ax + (bx - ax) * t, ay + (by - ay) * t, 1);
                }
            }
        }
    }
    for p in points {
        let (x, y) = to(p);
        hit(x, y, 4);
    }
    hits
}

/// `a` to `b` by `t` (0..1), opaque.
pub fn lerp(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Rgba8::rgb(m(a.r, b.r), m(a.g, b.g), m(a.b, b.b))
}

/// A square raster of `side` x `side` pixels.
struct Raster<'a> {
    px: &'a mut [Rgba8],
    side: usize,
}

impl Raster<'_> {
    /// Draw `c` (premultiplied) over pixel `(x, y)`.
    fn blend(&mut self, x: i32, y: i32, c: Rgba8) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.side || y >= self.side {
            return;
        }
        if let Some(p) = self.px.get_mut(y * self.side + x) {
            *p = c.over(*p);
        }
    }

    /// A line from `a` to `b`, `width` pixels wide.
    fn line(&mut self, a: (f32, f32), b: (f32, f32), width: i32, c: Rgba8) {
        let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as i32;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = (a.0 + (b.0 - a.0) * t).floor() as i32;
            let y = (a.1 + (b.1 - a.1) * t).floor() as i32;
            for dy in 0..width {
                for dx in 0..width {
                    self.blend(x + dx, y + dy, c);
                }
            }
        }
    }
}

/// The constellation square's image (`side` x `side`, row by row): the shown
/// symbols as green points with a white core by density (`Contrast`), the
/// paths between them with Transitions, and the demodulator's positions with
/// `Vectors of dem` (the zero vector light blue). With the eye diagram on
/// (SigNexora extra) the I trace folded at two symbols instead.
pub fn constellation_pixels(st: &PhaseState, side: usize, pal: &Palette) -> Vec<Rgba8> {
    let mut pixels = vec![pal.plot_bg; side * side];
    if st.eye {
        draw_eye(&mut pixels, side, st, pal);
        return pixels;
    }
    if let Some(s) = st.shown() {
        let (a, b) = st.play.shown(st.form.life, st.form.points);
        let stride = (b.saturating_sub(a) / MAX_DRAWN).max(1);
        let pts: Vec<Complex32> =
            s.r.display
                .get(a..b)
                .unwrap_or(&[])
                .iter()
                .step_by(stride)
                .copied()
                .collect();
        let path: Option<Vec<Complex32>> = st.form.transitions.then(|| {
            let traj = &s.r.trajectory;
            if st.form.diff == 0 && b * TRAJ_PER_SYMBOL <= traj.len() {
                // The same budget as the points: the last MAX_DRAWN symbols of
                // the path, so the work per frame does not grow with the file.
                let from = a.max(b.saturating_sub(MAX_DRAWN));
                traj.get(from * TRAJ_PER_SYMBOL..b * TRAJ_PER_SYMBOL)
                    .unwrap_or(&[])
                    .to_vec()
            } else {
                pts.clone()
            }
        });
        let hits = density(
            &pts,
            path.as_deref(),
            side,
            st.form.plane_scale(),
            st.form.trimming as f32,
        );
        let gain = st.form.contrast.max(1) as f32 / 40.0;
        for (px, h) in pixels.iter_mut().zip(&hits) {
            if *h > 0 {
                let level = 1.0 - (-(*h as f32) * gain / 6.0).exp();
                *px = if level < 0.7 {
                    lerp(pal.plot_bg, pal.points, level / 0.7)
                } else {
                    lerp(pal.points, pal.plot_text, (level - 0.7) / 0.3)
                };
            }
        }
    }
    if st.form.vectors {
        draw_vectors(&mut pixels, side, &st.form, pal);
    }
    pixels
}

/// Vectors of dem: the demodulator's positions, the zero vector light blue.
fn draw_vectors(pixels: &mut [Rgba8], side: usize, f: &Form, pal: &Palette) {
    let mut r = Raster { px: pixels, side };
    let c = side as f32 / 2.0;
    let len = c - 4.0;
    let dem = f.dem;
    for k in 0..dem.m {
        let a = (dem.angle(k) as f32).to_radians();
        let tip = (c + a.cos() * len, c - a.sin() * len);
        let color = if k == 0 { pal.trace_i } else { pal.trace_q };
        r.line((c - 1.0, c - 1.0), (tip.0 - 1.0, tip.1 - 1.0), 2, color);
    }
}

/// The eye diagram (SigNexora extra): the I trace folded at two symbols.
fn draw_eye(pixels: &mut [Rgba8], side: usize, st: &PhaseState, pal: &Palette) {
    let Some(s) = st.shown() else {
        return;
    };
    let f = pal.faint;
    let faint = Rgba8::premultiplied(
        (u32::from(f.r) * 60 / 255) as u8,
        (u32::from(f.g) * 60 / 255) as u8,
        (u32::from(f.b) * 60 / 255) as u8,
        (u32::from(f.a) * 60 / 255) as u8,
    );
    let mut r = Raster { px: pixels, side };
    let w = side as f32;
    for tr in &s.eye {
        let n = tr.len().max(2) as f32;
        let pt = |i: usize, v: f32| (i as f32 / (n - 1.0) * w, w / 2.0 - v / 2.0 * w / 2.0);
        for (i, pair) in tr.windows(2).enumerate() {
            if let [a, b] = pair {
                r.line(pt(i, *a), pt(i + 1, *b), 1, faint);
            }
        }
    }
}

/// The strongest bin within `reach` bins of `k`.
pub fn peak_near(db: &[f32], k: usize, reach: usize) -> Option<usize> {
    let a = k.saturating_sub(reach);
    let b = (k + reach + 1).min(db.len());
    db.get(a..b)?
        .iter()
        .enumerate()
        .max_by(|x, y| x.1.total_cmp(y.1))
        .map(|(i, _)| a + i)
}

/// Move the Fl or Fu marker (whichever is nearer to `px`, an unset one first)
/// to `f`.
pub fn move_spec_mark(m: &mut SpecMarks, f: f64, x_of: impl Fn(f64) -> f32, px: f32) {
    match (m.fl, m.fu) {
        (None, _) => m.fl = Some(f),
        (Some(_), None) => m.fu = Some(f),
        (Some(a), Some(b)) => {
            if (x_of(a) - px).abs() <= (x_of(b) - px).abs() {
                m.fl = Some(f);
            } else {
                m.fu = Some(f);
            }
        }
    }
    if let (Some(a), Some(b)) = (m.fl, m.fu)
        && a > b
    {
        m.fl = Some(b);
        m.fu = Some(a);
    }
}

/// Vertical gain of the corrector plot from its scroll bar: the dB shown.
pub fn corr_range_db(scale: f32) -> f32 {
    20.0 + 100.0 * scale.clamp(0.0, 1.0)
}

/// The colours of the ten angle markers.
pub fn mark_colors(pal: &Palette) -> [Rgba8; MAX_MARKS] {
    [
        pal.trace_sum,
        pal.plot_text,
        pal.trace_i,
        pal.trace_extra,
        pal.trace_q,
        pal.trace_alt,
        pal.warn,
        pal.marker_free,
        pal.cursor,
        pal.reference,
    ]
}

/// The band the spectrum mode shows, Hz on the signal's axis: `band` (the
/// caller's H markers) or the whole spectrum, the `1/zoom` part of it at
/// scroll position `pos` (0..1).
pub fn spectrum_band(
    kind: SampleKind,
    fs: f64,
    band: Option<(f64, f64)>,
    zoom: u32,
    pos: f32,
) -> (f64, f64) {
    let full = match kind {
        SampleKind::Real => (0.0, fs / 2.0),
        SampleKind::Complex => (-fs / 2.0, fs / 2.0),
    };
    let (lo, hi) = band.unwrap_or(full);
    let width = (hi - lo) / f64::from(zoom.max(1));
    let start = lo + f64::from(pos.clamp(0.0, 1.0)) * (hi - lo - width);
    (start, start + width)
}

/// The averaged spectrum as one filled column per pixel (the strongest bin),
/// each a level 0..1 above `LowLevel` (`stretch`: up to the strongest column,
/// else up to 0 dB). `cols` columns cover `shown` Hz.
pub fn hires_levels(
    h: &HiresSpec,
    shown: (f64, f64),
    cols: usize,
    low_level: f32,
    stretch: bool,
) -> Vec<f32> {
    let span = (shown.1 - shown.0).max(1e-9);
    let col_db: Vec<Option<f32>> = (0..cols)
        .map(|c| {
            let fa = shown.0 + span * c as f64 / cols as f64;
            let fb = shown.0 + span * (c + 1) as f64 / cols as f64;
            let ka = ((fa - h.f0) / h.df).floor().max(0.0) as usize;
            let kb = (((fb - h.f0) / h.df).ceil().max(0.0) as usize).max(ka + 1);
            h.db.get(ka..kb.min(h.db.len()))
                .and_then(|s| s.iter().copied().reduce(f32::max))
        })
        .collect();
    let top = if stretch {
        col_db
            .iter()
            .flatten()
            .copied()
            .fold(low_level + 1.0, f32::max)
    } else {
        0.0
    };
    let range = (top - low_level).max(1.0);
    col_db
        .iter()
        .map(|v| v.map_or(0.0, |v| ((v - low_level) / range).clamp(0.0, 1.0)))
        .collect()
}

/// What the corrector plot draws, in pixels of a `w` x `h` plot.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CorrPlot {
    /// Cyan lines: the expected sidebands (X^N) or the clock line (AM).
    pub lines: Vec<f32>,
    /// The red line at the PLL frequency (X^N).
    pub pll: Option<f32>,
    /// The yellow trace, one point per bin.
    pub trace: Vec<(f32, f32)>,
}

/// The corrector plot of `c` with the vertical scale `isd_scale`.
pub fn corr_plot(c: &CorrSpec, isd_scale: f32, w: f32, h: f32) -> CorrPlot {
    let span = (c.hi - c.lo).max(1e-9);
    let x_of = |f: f64| ((f - c.lo) / span) as f32 * w;
    let baud = c.key.baud;
    let lines = match c.key.mode {
        IsdMode::Xn => vec![x_of(-baud), x_of(baud)],
        IsdMode::Am => vec![x_of(baud)],
    };
    let pll = (c.key.mode == IsdMode::Xn).then(|| x_of(0.0));
    let top = c.db.iter().copied().fold(f32::MIN, f32::max);
    let range = corr_range_db(isd_scale);
    let trace =
        c.db.iter()
            .enumerate()
            .map(|(k, v)| {
                let y = ((top - v) / range).clamp(0.0, 1.0);
                (x_of(c.f0 + k as f64 * c.df), 4.0 + y * (h - 4.0))
            })
            .collect();
    CorrPlot { lines, pll, trace }
}

/// The frequency at `x` of a corrector plot `w` wide, on the plot's axis.
fn corr_axis_at(c: &CorrSpec, x: f32, w: f32) -> f64 {
    c.lo + f64::from(x / w.max(1.0)) * (c.hi - c.lo).max(1e-9)
}

/// The status bar's `F =` under the pointer at `x`: N times the carrier plus
/// the offset (X^N), or the envelope frequency (AM).
pub fn corr_hover_hz(c: &CorrSpec, x: f32, w: f32) -> f64 {
    let f = corr_axis_at(c, x, w);
    match c.key.mode {
        IsdMode::Xn => f64::from(c.key.n.max(1)) * c.key.carrier + f,
        IsdMode::Am => f,
    }
}

/// A left click at `x` on the corrector plot: the peak near it sets Carrier
/// (X^N) or BR (AM).
pub fn corr_click(form: &mut Form, c: &CorrSpec, x: f32, w: f32) {
    let at = corr_axis_at(c, x, w);
    let near = ((at - c.f0) / c.df.max(1e-12)).round().max(0.0) as usize;
    let f = peak_near(&c.db, near, 3).map_or(at, |k| c.f0 + k as f64 * c.df);
    let n_harm = f64::from(c.key.n.max(1));
    match c.key.mode {
        IsdMode::Xn => form.carrier = format::dec_trim(c.key.carrier + f / n_harm, 2),
        IsdMode::Am if f > 0.0 => form.br = format::dec_trim(f, 2),
        IsdMode::Am => {}
    }
}

/// Guard band at the top and bottom of the angles histogram, where the marker
/// colours show well.
pub const HIST_GUARD: f32 = 8.0;

/// One bar of the angles histogram: its column, its far end, and whether it
/// counts positive angles (red, upwards) or negative ones (blue, downwards).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistBar {
    /// Column, pixels from the left.
    pub x: f32,
    /// The far end of the bar; the near end is the middle line.
    pub y: f32,
    /// Positive angles.
    pub up: bool,
}

/// The x of `deg` degrees in an angles histogram `w` wide: 0 at the right,
/// 180 at the left.
pub fn hist_x(deg: f64, w: f32) -> f32 {
    w - (deg.abs() / 180.0) as f32 * w
}

/// The bars of the angles histogram of `st` in a `w` x `h` plot (R136).
pub fn hist_bars(st: &PhaseState, w: f32, h: f32) -> Vec<HistBar> {
    let mid = h / 2.0;
    let half = h / 2.0 - HIST_GUARD;
    let peak = st.hist.peak().max(1) as f32;
    let gain = 0.25 + 3.75 * (1.0 - st.form.isd_scale.clamp(0.0, 1.0));
    let mut out = Vec::new();
    for (bins, up) in [(&st.hist.pos, true), (&st.hist.neg, false)] {
        for (k, c) in bins.iter().enumerate() {
            if *c == 0 {
                continue;
            }
            let x = hist_x((k as f64 + 0.5) * 0.36, w);
            let len = (*c as f32 / peak * gain).min(1.0) * half;
            let y = if up { mid - len } else { mid + len };
            out.push(HistBar { x, y, up });
        }
    }
    out
}

/// The direct angle (0 to 180 degrees) at `x` of an angles histogram `w` wide.
pub fn hist_angle(x: f32, w: f32) -> f64 {
    f64::from((w - x) / w.max(1.0) * 180.0).clamp(0.0, 180.0)
}

/// The signed angle a click at `(x, y)` drops a marker at: positive above the
/// middle line, negative below.
pub fn hist_mark_at(x: f32, y: f32, w: f32, h: f32) -> f64 {
    let d = hist_angle(x, w);
    if y <= h / 2.0 { d } else { -d }
}

/// The quality number (`0` before a run) and whether an offset mode was
/// detected.
pub fn quality_text(st: &PhaseState) -> (String, bool) {
    let (q, offset) = st.shown().map_or((None, false), |s| (s.quality, s.offset));
    let text = q.map_or_else(|| "0".to_owned(), |q| format::dec(f64::from(q), 4));
    (text, offset)
}

/// What the progress bar under the sliders shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayProgress {
    /// Nothing ran yet.
    Idle,
    /// The run is being computed.
    Busy,
    /// The playback is this far through the file.
    Done(f32),
}

/// The progress bar: first the computation, then the playback.
pub fn play_progress(st: &PhaseState) -> PlayProgress {
    match st.shown() {
        _ if st.computing() => PlayProgress::Busy,
        Some(s) if st.play.running || st.play.pos > 0 => {
            PlayProgress::Done(st.play.pos as f32 / s.len().max(1) as f32)
        }
        _ => PlayProgress::Idle,
    }
}

/// The four panels of SA's status bar: `F = ...` (or `Angle_D` and
/// `Angle_I`) and the loaded file in the phase plane mode; `Fl`, `Fu` and
/// `dF` in the spectrum mode (on the axis of Fmin and Fmax, with the centre
/// frequency `rf`).
pub fn status_texts(st: &PhaseState, hover: Hover, rf: f64) -> [String; 4] {
    match st.mode {
        Mode::Spectrum => {
            let m = st.spec_marks;
            let f = |v: Option<f64>| v.map_or_else(String::new, |v| format::dec(v, 2));
            let df = match (m.fl, m.fu) {
                (Some(a), Some(b)) => Some(b - a),
                _ => None,
            };
            [
                t!("pp-status-fl", value = f(m.fl.map(|v| v + rf))),
                t!("pp-status-fu", value = f(m.fu.map(|v| v + rf))),
                t!("pp-status-df", value = f(df)),
                String::new(),
            ]
        }
        Mode::Phase => {
            let (a, b) = match (hover.angle, hover.f) {
                (Some(d), _) => (
                    t!("pp-status-angle-d", value = format::dec(d, 2)),
                    t!("pp-status-angle-i", value = format::dec(180.0 - d, 2)),
                ),
                (None, Some(f)) => (t!("pp-status-f", value = format::dec(f, 2)), String::new()),
                (None, None) => (String::new(), String::new()),
            };
            [a, b, String::new(), st.loaded_title.clone()]
        }
    }
}

/// The status bar's panels: left edge and width in a form `w` wide.
pub fn status_panels(w: i32) -> [(i32, i32); 4] {
    [(0, 120), (122, 118), (242, 118), (362, w - 362)]
}

#[cfg(test)]
mod tests {
    use sn_core::NodeId;

    use super::*;
    use crate::phase::{CorrKey, HiresKey};
    use crate::theme::{ThemeChoice, palette_for};

    #[test]
    fn density_counts_points_and_paths_inside_the_square() {
        let pts = [
            Complex32::new(0.0, 0.0),
            Complex32::new(1.0, 0.0),
            Complex32::new(9.0, 9.0),
        ];
        let h = density(&pts, None, 10, 0.5, 0.0);
        assert_eq!(h.iter().sum::<u32>(), 8, "the far point is outside");
        assert_eq!(h.get(5 * 10 + 5), Some(&4), "the centre");
        let path = [Complex32::new(-1.0, 0.0), Complex32::new(1.0, 0.0)];
        let h = density(&[], Some(&path), 10, 0.5, 0.0);
        assert!(h.iter().sum::<u32>() >= 4, "the segment is drawn");
        // A trim of 90 degrees turns (1, 0) to the top.
        let h = density(&[Complex32::new(1.0, 0.0)], None, 10, 0.5, 90.0);
        assert_eq!(h.get(2 * 10 + 5), Some(&4));
    }

    #[test]
    fn peaks_are_found_near_the_click_and_marks_move_by_distance() {
        let db = [0.0, 1.0, 5.0, 2.0, 0.0, 9.0];
        assert_eq!(peak_near(&db, 1, 1), Some(2));
        assert_eq!(peak_near(&db, 4, 1), Some(5));
        assert_eq!(peak_near(&[], 0, 3), None);
        let mut m = SpecMarks::default();
        let x = |f: f64| f as f32;
        move_spec_mark(&mut m, 100.0, x, 100.0);
        move_spec_mark(&mut m, 50.0, x, 50.0);
        assert_eq!((m.fl, m.fu), (Some(50.0), Some(100.0)));
        move_spec_mark(&mut m, 90.0, x, 90.0);
        assert_eq!(m.fu, Some(90.0));
        assert!((corr_range_db(0.0) - 20.0).abs() < 1e-6);
    }

    #[test]
    fn vectors_and_points_land_in_the_image() {
        let pal = palette_for(ThemeChoice::Classic, false);
        let mut st = PhaseState::default();
        st.form.vectors = true;
        st.form.dem.m = 4;
        let px = constellation_pixels(&st, 40, pal);
        assert_eq!(px.len(), 1600);
        // The zero vector runs right from the centre.
        assert_eq!(px.get(19 * 40 + 30), Some(&pal.trace_i));
        // The second one runs up.
        assert_eq!(px.get(8 * 40 + 19), Some(&pal.trace_q));
    }

    #[test]
    fn band_status_and_histogram_geometry() {
        let (a, b) = spectrum_band(SampleKind::Real, 8000.0, None, 2, 1.0);
        assert!((a - 2000.0).abs() < 1e-9 && (b - 4000.0).abs() < 1e-9);
        let (a, _) = spectrum_band(SampleKind::Complex, 8000.0, Some((100.0, 300.0)), 1, 0.0);
        assert!((a - 100.0).abs() < 1e-9);
        assert!((hist_angle(0.0, 500.0) - 180.0).abs() < 1e-9);
        assert!((hist_mark_at(500.0, 10.0, 500.0, 100.0)).abs() < 1e-9);
        assert!((hist_mark_at(250.0, 80.0, 500.0, 100.0) + 90.0).abs() < 1e-9);
        let mut st = PhaseState::default();
        st.spec_marks.fl = Some(10.0);
        st.spec_marks.fu = Some(30.0);
        let s = status_texts(&st, Hover::default(), 1000.0);
        let pt = |s: &str| s.replace(',', ".");
        assert_eq!(pt(&s[0]), "Fl = 1010.00");
        assert_eq!(pt(&s[2]), "dF = 20.00");
        st.mode = Mode::Phase;
        let s = status_texts(
            &st,
            Hover {
                f: None,
                angle: Some(113.4),
            },
            0.0,
        );
        assert_eq!(pt(&s[1]), "Angle_I = 66.60");
        assert_eq!(play_progress(&st), PlayProgress::Idle);
        assert_eq!(quality_text(&st), ("0".to_owned(), false));
    }

    #[test]
    fn corrector_clicks_pick_the_peak_and_columns_scale() {
        let key = CorrKey {
            source: NodeId::from_raw(0),
            mode: IsdMode::Xn,
            n: 2,
            carrier: 1000.0,
            baud: 100.0,
            blocks: 10,
            pos: 0,
        };
        let c = CorrSpec {
            key,
            lo: -200.0,
            hi: 200.0,
            f0: -200.0,
            df: 10.0,
            db: (0..41).map(|k| if k == 22 { 0.0 } else { -60.0 }).collect(),
        };
        let p = corr_plot(&c, 0.0, 400.0, 100.0);
        assert_eq!(p.lines.len(), 2);
        assert_eq!(p.pll, Some(200.0));
        assert_eq!(p.trace.len(), 41);
        let mut f = Form::default();
        // A click near 15 Hz finds the peak at 20 Hz: the carrier moves by 10.
        corr_click(&mut f, &c, 215.0, 400.0);
        assert_eq!(f.carrier_hz(), Some(1010.0));
        assert!((corr_hover_hz(&c, 200.0, 400.0) - 2000.0).abs() < 1e-9);
        let h = HiresSpec {
            key: HiresKey {
                node: NodeId::from_raw(0),
                start: 0,
                end: 1,
                fft: 4096,
            },
            f0: 0.0,
            df: 1.0,
            db: vec![-100.0, -50.0, 0.0, -50.0],
        };
        let l = hires_levels(&h, (0.0, 4.0), 4, -100.0, false);
        assert_eq!(l, vec![0.0, 0.5, 1.0, 0.5]);
    }
}
