//! The plot of SA's WF window as pixels (`docs/SA_UI_SPEC.md` section 7): the
//! waveform on black without axes (a line through the samples when zoomed in,
//! the min/max envelope per pixel column otherwise), the grey zero line, the
//! Sum histogram from the left edge over a dimmed trace and a CCF drawn over
//! its source, rendered into an RGBA image; the marker hit tests on the value
//! axis; and the Sum histogram built in the background. The same drawing as
//! `sa-app`'s `waveform.rs`, independent of the UI toolkit.

use std::sync::Arc;

use sn_analysis::histogram::Histogram;
use sn_core::job::{self, JobError, JobHandle};
use sn_core::{Complex32, Signal};
use sn_dsp::overview::{MinMax, Overview};

use crate::color::Rgba8;
use crate::jobs::analysis_to_job;
use crate::markers::{self, MarkerId, Markers};
use crate::sono_cache::SonoImage;
use crate::t;
use crate::view::View;

/// Grab distance of a marker line, pixels.
pub const GRAB_PX: f64 = 6.0;
/// Half the size of a marker handle (green triangles, red when locked).
pub const HANDLE: f64 = 6.0;

/// The vertical mapping of a WF plot: it shows `offset - scale..=offset + scale`
/// over `height` pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertical {
    /// Half the vertical range.
    pub scale: f32,
    /// The value at the centre line (SA's DC offset scroll bar).
    pub offset: f64,
    /// Plot height in pixels.
    pub height: f64,
}

impl Vertical {
    /// The value at pixel row `y` (0 at the top).
    pub fn value_at(&self, y: f64) -> f64 {
        let t = (self.height / 2.0 - y) / (self.height / 2.0).max(1e-9);
        self.offset + t * f64::from(self.scale.max(1e-9))
    }

    /// The pixel row of value `v`.
    pub fn y_of(&self, v: f64) -> f64 {
        let t = (v - self.offset) / f64::from(self.scale.max(1e-9));
        self.height / 2.0 - t.clamp(-1e4, 1e4) * self.height / 2.0
    }

    /// Value units per pixel row.
    pub fn per_px(&self) -> f64 {
        2.0 * f64::from(self.scale) / self.height.max(1.0)
    }
}

/// Distance in pixels from `(x, y)` to the line of marker `id` at `v`.
fn distance(id: MarkerId, v: f64, view: &View, vert: &Vertical, (x, y): (f64, f64)) -> f64 {
    if id.is_vertical() {
        (view.sample_to_x(v) - x).abs()
    } else {
        (vert.y_of(v) - y).abs()
    }
}

/// The marker (if any) whose line is within grab distance of `p` (plot
/// coordinates).
pub fn marker_near(m: &Markers, view: &View, vert: &Vertical, p: (f64, f64)) -> Option<MarkerId> {
    markers::nearest(m, GRAB_PX, |id, v| Some(distance(id, v, view, vert, p)))
}

/// The marker whose handle (top edge for V, left edge for H) is under `p`; the
/// nearest when two handles overlap.
pub fn handle_near(m: &Markers, view: &View, vert: &Vertical, p: (f64, f64)) -> Option<MarkerId> {
    let near_top = p.1 <= 2.0 * HANDLE;
    let near_left = p.0 <= 2.0 * HANDLE;
    markers::nearest(m, HANDLE + 2.0, |id, v| {
        let edge = if id.is_vertical() {
            near_top
        } else {
            near_left
        };
        edge.then(|| distance(id, v, view, vert, p))
    })
}

/// A CCF result drawn over its source (SA 6.2.5.4): the CCF of the yellow
/// fragment against the whole signal, aligned with it.
#[derive(Debug, Clone)]
pub struct CcfOverlay {
    /// The CCF, one value (0 to 1) per source sample.
    pub signal: Signal,
    /// Its envelope.
    pub overview: Arc<Overview>,
    /// The reference fragment, samples.
    pub fragment: (usize, usize),
}

/// The colours of the plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WfColors {
    /// Background.
    pub bg: Rgba8,
    /// The zero line.
    pub zero: Rgba8,
    /// A plain trace (light blue).
    pub trace: Rgba8,
    /// A frequency trace after the phase detector (green).
    pub trace_hz: Rgba8,
    /// The Q channel of complex signals, and the source under a CCF (magenta).
    pub trace_q: Rgba8,
    /// The Sum histogram over a plain trace (green).
    pub sum: Rgba8,
    /// The Sum histogram over a frequency trace (yellow).
    pub sum_hz: Rgba8,
    /// The CCF reference fragment (yellow).
    pub fragment: Rgba8,
}

/// What the trace image shows.
#[derive(Debug, Clone, Copy)]
pub struct TraceRequest<'a> {
    /// The signal.
    pub signal: &'a Signal,
    /// Its envelope.
    pub overview: &'a Overview,
    /// The zoom and pan.
    pub view: View,
    /// Image width in pixels.
    pub width: usize,
    /// The vertical mapping (its height is the image height).
    pub vert: Vertical,
    /// Draw the Q channel of a complex signal.
    pub show_q: bool,
    /// The trace is a frequency (green).
    pub hz: bool,
    /// Sum is on: the trace is dimmed and the histogram drawn over it.
    pub sum: Option<&'a Histogram>,
    /// Dim the trace for Sum (also while its histogram is being built).
    pub dim: bool,
    /// A CCF view replaces the trace.
    pub ccf: Option<&'a CcfOverlay>,
    /// The colours.
    pub colors: WfColors,
}

/// `c` at 45% (SA's dimmed trace under Sum, on the dark plot).
fn dim(c: Rgba8) -> Rgba8 {
    let f = |v: u8| (f32::from(v) * 0.45).round() as u8;
    Rgba8::rgb(f(c.r), f(c.g), f(c.b))
}

/// `c` at 80% (the Q channel).
fn q_tint(c: Rgba8) -> Rgba8 {
    let f = |v: u8| (f32::from(v) * 0.8).round() as u8;
    Rgba8::rgb(f(c.r), f(c.g), f(c.b))
}

/// A pixel canvas.
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<Rgba8>,
}

impl Canvas {
    fn new(w: usize, h: usize, bg: Rgba8) -> Self {
        Self {
            w,
            h,
            px: vec![bg; w * h],
        }
    }

    fn set(&mut self, x: i64, y: i64, c: Rgba8) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.w || y >= self.h {
            return;
        }
        if let Some(p) = self.px.get_mut(y * self.w + x) {
            *p = c;
        }
    }

    /// A vertical run at column `x` from row `y0` to `y1` (inclusive, any order).
    fn vline(&mut self, x: i64, y0: f64, y1: f64, c: Rgba8) {
        let (a, b) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        let hmax = self.h as f64;
        let a = a.floor().clamp(-1.0, hmax) as i64;
        let b = b.floor().clamp(-1.0, hmax) as i64;
        for y in a..=b.max(a) {
            self.set(x, y, c);
        }
    }

    /// A horizontal run at row `y` from column 0 to `x1` (exclusive).
    fn hline(&mut self, y: i64, x1: usize, c: Rgba8) {
        for x in 0..x1.min(self.w) {
            self.set(x as i64, y, c);
        }
    }

    /// A one-pixel line (Bresenham), clipped to the canvas.
    fn line(&mut self, (x0, y0): (f64, f64), (x1, y1): (f64, f64), c: Rgba8) {
        let lim = 1e6;
        let (mut x, mut y) = (
            x0.round().clamp(-lim, lim) as i64,
            y0.round().clamp(-lim, lim) as i64,
        );
        let (x1, y1) = (
            x1.round().clamp(-lim, lim) as i64,
            y1.round().clamp(-lim, lim) as i64,
        );
        let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
        let (sx, sy) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        // Lines far outside the canvas are not walked pixel by pixel.
        let mut budget = (self.w + self.h) * 4 + 16;
        loop {
            self.set(x, y, c);
            if (x == x1 && y == y1) || budget == 0 {
                break;
            }
            budget -= 1;
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn image(self) -> SonoImage {
        SonoImage {
            width: self.w,
            height: self.h,
            pixels: self.px,
        }
    }
}

/// The sample span of pixel column `x`.
fn column_span(view: &View, x: usize) -> (usize, usize) {
    let a = view.x_to_sample(x as f64).max(0.0);
    let b = view.x_to_sample(x as f64 + 1.0).max(a + 1.0);
    (a.floor() as usize, b.ceil() as usize)
}

/// Draw one column of envelope `m`.
fn column(cv: &mut Canvas, x: usize, m: MinMax, vert: &Vertical, c: Rgba8) {
    if m.is_empty() {
        return;
    }
    let (y1, y2) = (vert.y_of(f64::from(m.max)), vert.y_of(f64::from(m.min)));
    cv.vline(x as i64, y1, y2.max(y1), c);
}

/// The signal: a line through the samples when zoomed in, the min/max envelope
/// per pixel column otherwise. `q` draws the Q channel too.
fn draw_signal(cv: &mut Canvas, r: &TraceRequest<'_>, color: Rgba8, q: Option<Rgba8>) {
    let view = &r.view;
    if view.samples_per_px <= 1.0 {
        let a = view.x_to_sample(0.0).floor().max(0.0) as usize;
        let b = (view.x_to_sample(r.width as f64).ceil().max(0.0) as usize + 2).min(r.signal.len());
        if b <= a {
            return;
        }
        let mut buf = vec![Complex32::new(0.0, 0.0); b - a];
        let got = r.signal.read_complex(a, &mut buf);
        let pts = |f: &dyn Fn(&Complex32) -> f32| -> Vec<(f64, f64)> {
            buf.iter()
                .take(got)
                .enumerate()
                .map(|(i, v)| {
                    (
                        view.sample_to_x((a + i) as f64),
                        r.vert.y_of(f64::from(f(v))),
                    )
                })
                .collect()
        };
        let mut polyline = |p: Vec<(f64, f64)>, c: Rgba8| {
            for w in p.windows(2) {
                if let [p0, p1] = w {
                    cv.line(*p0, *p1, c);
                }
            }
            if let [only] = p.as_slice() {
                cv.line(*only, *only, c);
            }
        };
        if let Some(qc) = q {
            polyline(pts(&|v| v.im), qc);
        }
        polyline(pts(&|v| v.re), color);
        return;
    }
    for x in 0..r.width {
        let (a, b) = column_span(view, x);
        let env = r.overview.span(r.signal, a, b);
        if let (Some(qc), Some(m)) = (q, env.im) {
            column(cv, x, m, &r.vert, qc);
        }
        column(cv, x, env.re, &r.vert, color);
    }
}

/// The CCF view: the source magenta, the fragment yellow, the CCF (0 to 1) from
/// the centre line up in the trace colour.
fn draw_ccf(cv: &mut Canvas, r: &TraceRequest<'_>, ccf: &CcfOverlay) {
    let c = r.colors;
    draw_signal(cv, r, c.trace_q, None);
    let view = &r.view;
    let (fa, fb) = ccf.fragment;
    let x0 = view.sample_to_x(fa as f64).max(0.0);
    let x1 = view.sample_to_x(fb as f64).min(r.width as f64);
    let mut x = x0.floor();
    while x < x1 {
        let lo = view.x_to_sample(x).max(fa as f64);
        let hi = view.x_to_sample(x + 1.0).min(fb as f64).max(lo + 1.0);
        let env = r
            .overview
            .span(r.signal, lo.floor() as usize, hi.ceil() as usize);
        column(cv, x as usize, env.re, &r.vert, c.fragment);
        x += 1.0;
    }
    let h = r.vert.height;
    let unit = Vertical {
        scale: 1.0,
        offset: 0.0,
        height: h,
    };
    for x in 0..r.width {
        let (a, b) = column_span(view, x);
        let env = ccf.overview.span(&ccf.signal, a, b);
        if env.re.is_empty() {
            continue;
        }
        let top = unit.y_of(f64::from(env.re.max.clamp(-1.0, 1.0)));
        let bottom = unit.y_of(f64::from(env.re.min.clamp(-1.0, 1.0)));
        cv.vline(x as i64, top, bottom.max(top), c.trace);
    }
}

/// SA's Sum: the value histogram of the visible span, one bar per pixel row from
/// the left edge.
fn draw_sum(cv: &mut Canvas, hist: &Histogram, color: Rgba8) {
    let rows = hist.counts.len().max(1);
    let top = hist.counts.iter().copied().max().unwrap_or(1).max(1);
    let (w, h) = (cv.w as f64, cv.h as f64);
    for (i, c) in hist.counts.iter().enumerate() {
        if *c == 0 {
            continue;
        }
        // Bin 0 is the lowest value: draw it at the bottom.
        let y = h - (i as f64 + 0.5) / rows as f64 * h;
        let len = (f64::from(*c) / f64::from(top) * w).round() as usize;
        cv.hline(y.floor() as i64, len.max(1), color);
    }
}

/// Render the trace image: background, zero line, trace (or the CCF view) and
/// the Sum histogram. The markers are drawn over it by the window.
pub fn render(r: &TraceRequest<'_>) -> SonoImage {
    let h = r.vert.height.max(1.0) as usize;
    let mut cv = Canvas::new(r.width.max(1), h, r.colors.bg);
    let zero = r.vert.y_of(0.0);
    if (0.0..r.vert.height).contains(&zero) {
        cv.hline(zero.floor() as i64, r.width, r.colors.zero);
    }
    if let Some(ccf) = r.ccf {
        draw_ccf(&mut cv, r, ccf);
        return cv.image();
    }
    let c = r.colors;
    let mut color = if r.hz { c.trace_hz } else { c.trace };
    if r.dim {
        color = dim(color);
    }
    let q =
        (r.show_q && r.signal.kind() == sn_core::SampleKind::Complex).then(|| q_tint(c.trace_q));
    draw_signal(&mut cv, r, color, q);
    if let Some(hist) = r.sum {
        draw_sum(&mut cv, hist, if r.hz { c.sum_hz } else { c.sum });
    }
    cv.image()
}

/// What a Sum histogram was built for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SumKey {
    /// First sample of the visible span.
    pub start: usize,
    /// End of the visible span.
    pub end: usize,
    /// Bins: one per pixel row.
    pub rows: usize,
    /// Lowest value shown.
    pub lo: f64,
    /// Highest value shown.
    pub hi: f64,
}

/// What the Sum histogram shows now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SumShown<'a> {
    /// The histogram of the current span.
    Ready(&'a Histogram),
    /// It is being built.
    Building,
    /// Building failed.
    Failed(&'a str),
}

/// The Sum histogram, built in the background and rebuilt only when the visible
/// span, the plot height or the vertical range change.
#[derive(Default)]
pub struct SumCache {
    shown: Option<(SumKey, Result<Histogram, String>)>,
    job: Option<(SumKey, JobHandle<Histogram>)>,
    /// Bumped whenever a new histogram arrives.
    count: u64,
}

impl std::fmt::Debug for SumCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SumCache")
            .field("running", &self.job.is_some())
            .field("count", &self.count)
            .finish_non_exhaustive()
    }
}

impl SumCache {
    /// Collect a finished build and start one for `want` if needed.
    pub fn sync(&mut self, signal: &Signal, want: SumKey) {
        if self.job.as_ref().is_some_and(|(_, h)| h.is_finished())
            && let Some((k, h)) = self.job.take()
        {
            match h.join() {
                Ok(hist) => {
                    self.shown = Some((k, Ok(hist)));
                    self.count += 1;
                }
                Err(JobError::Cancelled) => {}
                Err(e) => {
                    self.shown = Some((k, Err(e.to_string())));
                    self.count += 1;
                }
            }
        }
        let have = self.shown.as_ref().map(|(k, _)| *k);
        let pending = self.job.as_ref().map(|(k, _)| *k);
        if have != Some(want) && pending != Some(want) {
            if let Some((_, h)) = self.job.take() {
                h.cancel();
            }
            let s = signal.clone();
            self.job = Some((
                want,
                job::spawn(t!("wave-job-sum"), move |ctx| {
                    ctx.checkpoint()?;
                    let stop = || ctx.is_cancelled();
                    Histogram::build_with_stop(
                        &s,
                        want.start,
                        want.end,
                        want.rows,
                        Some((want.lo, want.hi)),
                        &stop,
                    )
                    .map_err(analysis_to_job)
                }),
            ));
        }
    }

    /// The histogram for `want`, or what stands in for it.
    pub fn shown(&self, want: SumKey) -> SumShown<'_> {
        match &self.shown {
            Some((k, Ok(h))) if *k == want => SumShown::Ready(h),
            Some((k, Err(e))) if *k == want => SumShown::Failed(e),
            _ => SumShown::Building,
        }
    }

    /// A build is running.
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Bumped whenever a histogram (or an error) arrives.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Stop a running build and forget the histogram (the window closed).
    pub fn release(&mut self) {
        if let Some((_, h)) = self.job.take() {
            h.cancel();
        }
        self.shown = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors() -> WfColors {
        WfColors {
            bg: Rgba8::BLACK,
            zero: Rgba8::rgb(128, 128, 128),
            trace: Rgba8::rgb(0, 200, 255),
            trace_hz: Rgba8::rgb(0, 255, 0),
            trace_q: Rgba8::rgb(255, 0, 255),
            sum: Rgba8::rgb(0, 255, 0),
            sum_hz: Rgba8::rgb(255, 255, 0),
            fragment: Rgba8::rgb(255, 255, 0),
        }
    }

    fn sine(n: usize) -> Signal {
        let x: Vec<f32> = (0..n).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        Signal::real(x, 8000.0).unwrap()
    }

    #[test]
    fn vertical_mapping_round_trips() {
        let v = Vertical {
            scale: 2.0,
            offset: 0.5,
            height: 200.0,
        };
        assert!((v.y_of(0.5) - 100.0).abs() < 1e-9);
        assert!((v.y_of(2.5)).abs() < 1e-9);
        for y in [0.0, 37.0, 150.0] {
            assert!((v.y_of(v.value_at(y)) - y).abs() < 1e-9);
        }
        assert!((v.per_px() - 0.02).abs() < 1e-12);
    }

    #[test]
    fn traces_draw_in_their_colours() {
        let s = sine(4000);
        let ov = Overview::build(&s, Overview::DEFAULT_BLOCK, &|| false).unwrap();
        let view = View::fit(
            crate::view::Extent {
                len: 4000.0,
                f_min: 0.0,
                f_max: 1.0,
            },
            200.0,
        );
        let mut r = TraceRequest {
            signal: &s,
            overview: &ov,
            view,
            width: 200,
            vert: Vertical {
                scale: 1.0,
                offset: 0.0,
                height: 100.0,
            },
            show_q: true,
            hz: false,
            sum: None,
            dim: false,
            ccf: None,
            colors: colors(),
        };
        let img = render(&r);
        assert_eq!((img.width, img.height), (200, 100));
        let count = |img: &SonoImage, c: Rgba8| img.pixels.iter().filter(|p| **p == c).count();
        assert!(count(&img, colors().trace) > 200);
        r.hz = true;
        assert!(count(&render(&r), colors().trace_hz) > 200);
        // Zoomed in: a line through the samples.
        r.view = View {
            samples_per_px: 0.25,
            ..view
        };
        assert!(count(&render(&r), colors().trace_hz) > 50);
        // A CCF view: source magenta, fragment yellow, CCF light blue.
        let peaks = Signal::real(vec![0.9; 4000], 8000.0).unwrap();
        let peaks_ov = Overview::build(&peaks, Overview::DEFAULT_BLOCK, &|| false).unwrap();
        let ccf = CcfOverlay {
            signal: peaks,
            overview: Arc::new(peaks_ov),
            fragment: (100, 800),
        };
        r.view = view;
        r.ccf = Some(&ccf);
        let img = render(&r);
        assert!(count(&img, colors().trace_q) > 50);
        assert!(count(&img, colors().fragment) > 10);
        assert!(count(&img, colors().trace) > 10);
    }

    #[test]
    fn sum_bars_start_at_the_left_edge() {
        let hist = Histogram {
            lo: -1.0,
            hi: 1.0,
            counts: vec![0, 10, 5, 0],
        };
        let mut cv = Canvas::new(40, 4, Rgba8::BLACK);
        draw_sum(&mut cv, &hist, Rgba8::WHITE);
        let row = |y: usize| -> usize {
            (0..40)
                .filter(|x| cv.px[y * 40 + x] == Rgba8::WHITE)
                .count()
        };
        // Bin 1 (second lowest) is the second row from the bottom: full width.
        assert_eq!(row(2), 40);
        assert_eq!(row(1), 20);
        assert_eq!(row(0) + row(3), 0);
    }

    #[test]
    fn handles_and_lines_are_hit() {
        let view = View {
            start: 0.0,
            samples_per_px: 10.0,
            f_lo: 0.0,
            f_hi: 1.0,
        };
        let vert = Vertical {
            scale: 1.0,
            offset: 0.0,
            height: 200.0,
        };
        let m = Markers {
            v: Some((1000.0, 2000.0)),
            h: Some((0.5, -0.5)),
            ..Markers::default()
        };
        assert_eq!(
            handle_near(&m, &view, &vert, (101.0, 4.0)),
            Some(MarkerId::V1)
        );
        assert_eq!(handle_near(&m, &view, &vert, (101.0, 60.0)), None);
        assert_eq!(
            handle_near(&m, &view, &vert, (3.0, 52.0)),
            Some(MarkerId::H1)
        );
        assert_eq!(
            marker_near(&m, &view, &vert, (199.0, 90.0)),
            Some(MarkerId::V2)
        );
        assert_eq!(
            marker_near(&m, &view, &vert, (60.0, 149.0)),
            Some(MarkerId::H2)
        );
        assert_eq!(marker_near(&m, &view, &vert, (60.0, 90.0)), None);
        // V1 and V2 four pixels apart: the cursor on V2's handle grabs V2.
        let close = Markers {
            v: Some((1000.0, 1040.0)),
            ..m
        };
        assert_eq!(
            handle_near(&close, &view, &vert, (104.0, 4.0)),
            Some(MarkerId::V2)
        );
        assert_eq!(
            handle_near(&close, &view, &vert, (100.0, 4.0)),
            Some(MarkerId::V1)
        );
    }
}
