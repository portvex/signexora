//! SA's VMW mode of the WF window as plain data (`docs/SA_UI_SPEC.md` 7.3):
//! the `TS` line period field (a left click on a digit adds 1, a right click
//! subtracts 1, with carry), the strobe range, the logarithmic line period
//! slider, the status line, the Info overlay, the blue, black, red and orange
//! colour map, the period correction of a right drag, and the raster built in
//! the background with its image. The same rules as `sa-app`'s
//! `vmw_window.rs` and `raster_view.rs`, independent of the UI toolkit.

use sn_analysis::raster::{Raster, RasterRequest, build_with_stop};
use sn_core::Signal;
use sn_core::job::{self, JobError, JobHandle};

use crate::color::Rgba8;
use crate::colormap::Colormap;
use crate::jobs::analysis_to_job;
use crate::sono_cache::SonoImage;
use crate::{format, t};

/// Strobe range of SA 6.2.5 (R261/a4).
pub const STROBE_MIN: usize = 1;
/// Largest strobe.
pub const STROBE_MAX: usize = 1000;
/// `Y scale` range: each raster line is this many pixels tall.
pub const Y_SCALE_MAX: usize = 10;
/// SA's VMW colours, lowest to highest level: blue, black, red, orange.
pub const SA_MAP: [Rgba8; 4] = [
    Rgba8::rgb(80, 60, 255),
    Rgba8::rgb(0, 0, 0),
    Rgba8::rgb(230, 20, 0),
    Rgba8::rgb(255, 170, 30),
];
/// The default line period, samples.
pub const DEFAULT_PERIOD: f64 = 1000.0;

/// The line period as the `TS` field shows it: `SS:mmm.uuu.nnn` (seconds,
/// milliseconds, microseconds, nanoseconds).
pub fn ts_text(seconds: f64) -> String {
    let (s, ms, us, ns) = split_ns(seconds);
    format!("{s:02}:{ms:03}.{us:03}.{ns:03}")
}

/// A span as the status bar shows it: `S:mmm.uuu.nnn`.
pub fn dt_text(seconds: f64) -> String {
    let (s, ms, us, ns) = split_ns(seconds);
    format!("{s}:{ms:03}.{us:03}.{ns:03}")
}

fn split_ns(seconds: f64) -> (u64, u64, u64, u64) {
    let total = if seconds.is_finite() {
        (seconds.max(0.0) * 1e9).round() as u64
    } else {
        0
    };
    (
        total / 1_000_000_000,
        (total / 1_000_000) % 1000,
        (total / 1000) % 1000,
        total % 1000,
    )
}

/// The value in seconds of the digit at character `index` of [`ts_text`]; `None` for
/// the separators.
pub fn ts_digit_value(index: usize) -> Option<f64> {
    let exp = match index {
        0 => 1,
        1 => 0,
        3..=5 => 2 - index as i32,
        7..=9 => 3 - index as i32,
        11..=13 => 4 - index as i32,
        _ => return None,
    };
    Some(10_f64.powi(exp))
}

/// The line period after a click on digit `index` of the `TS` field: a left click
/// (`up`) adds 1 to the digit, a right click subtracts 1 (with carry, as numbers
/// do). The period stays between `min` and 99 s.
pub fn ts_click(seconds: f64, index: usize, up: bool, min: f64) -> f64 {
    let Some(step) = ts_digit_value(index) else {
        return seconds;
    };
    let next = if up { seconds + step } else { seconds - step };
    // Round to whole nanoseconds so repeated clicks do not drift. `min` (one
    // sample) can exceed the field's largest value at a sample rate below
    // 0.01 Hz; `f64::clamp` would panic on the inverted range.
    let hi = 99.999_999_999_f64;
    ((next * 1e9).round() / 1e9).max(min.min(hi)).min(hi)
}

/// Shortest and longest line period of the slider, in samples.
pub fn period_limits(len: usize) -> (f64, f64) {
    let max = (len as f64 / 2.0).max(4.0);
    (2.0_f64.min(max), max)
}

/// Slider position (0 to 1) of a line period: logarithmic, SA's "highly
/// non-linear" line period slider.
pub fn period_to_slider(period: f64, (lo, hi): (f64, f64)) -> f64 {
    if hi <= lo || period <= 0.0 {
        return 0.0;
    }
    ((period.ln() - lo.ln()) / (hi.ln() - lo.ln())).clamp(0.0, 1.0)
}

/// The line period at slider position `v`.
pub fn slider_to_period(v: f64, (lo, hi): (f64, f64)) -> f64 {
    (lo.ln() + v.clamp(0.0, 1.0) * (hi.ln() - lo.ln())).exp()
}

/// Strobe value typed into the spin edit, if valid.
pub fn parse_strobe(text: &str) -> Option<usize> {
    text.trim()
        .parse::<usize>()
        .ok()
        .filter(|s| (STROBE_MIN..=STROBE_MAX).contains(s))
}

/// The `Y scale - N` text of the list.
pub fn y_scale_text(n: usize) -> String {
    format!("Y scale - {n}")
}

/// The status line under the raster: the span between the V markers and its rate.
pub fn status_text(span_s: f64) -> String {
    let speed = if span_s > 0.0 {
        format::dec_trim(1.0 / span_s, 2)
    } else {
        "0".to_owned()
    };
    format!("dT = {} | Speed = {speed}", dt_text(span_s))
}

/// The VMW Info overlay: `dT` between the V markers, `S` its rate, `TSs` the line
/// rate.
pub fn info_lines(span_s: f64, period_s: f64) -> Vec<String> {
    let rate = |s: f64| {
        if s > 0.0 {
            format::dec_trim(1.0 / s, 2)
        } else {
            "0".to_owned()
        }
    };
    vec![
        format!("dT = {}", ts_text(span_s)),
        format!("S = {}", rate(span_s)),
        format!(
            "TSs = {}",
            format::dec(if period_s > 0.0 { 1.0 / period_s } else { 0.0 }, 2)
        ),
    ]
}

/// SA's VMW colour of level `t` (0 to 1): blue for the lowest values, black in the
/// middle, red, then orange for the highest.
pub fn vmw_color(t: f32, map: &[Rgba8; 4]) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    let [blue, black, red, orange] = *map;
    let (a, b, f) = if t < 0.5 {
        (blue, black, t / 0.5)
    } else if t < 0.75 {
        (black, red, (t - 0.5) / 0.25)
    } else {
        (red, orange, (t - 0.75) / 0.25)
    };
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * f).round() as u8;
    Rgba8::rgb(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b))
}

/// The level (0 to 1) of value `v` in a raster spanning `lo..hi`: `offset` (-1 to 1)
/// moves the signal against zero, `bright` (0 to 1) is a non-linear gain.
pub fn level(v: f32, (lo, hi): (f32, f32), bright: f64, offset: f64) -> f32 {
    let span = (hi - lo).max(f32::EPSILON);
    let mid = f32::midpoint(lo, hi);
    let gain = 2_f64.powf((bright - 0.5) * 6.0) as f32;
    let x = (v - mid) / span * 2.0 + offset as f32;
    (0.5 + 0.5 * x * gain).clamp(0.0, 1.0)
}

/// Period correction from a drag of `(dx, dy)` screen points over a plot of `size`
/// points showing a raster of `cols` x `rows`: a feature drifting dx columns over dy
/// lines means the period is off by `dx_cols / cols * period / dy_rows` samples per
/// shown line.
pub fn drag_drift(
    (dx, dy): (f64, f64),
    size: (f64, f64),
    (cols, rows): (usize, usize),
    period: f64,
) -> Option<f64> {
    let (w, h) = size;
    if w <= 0.0 || h <= 0.0 || cols == 0 || rows == 0 {
        return None;
    }
    let dx_cols = dx / w * cols as f64;
    let dy_rows = dy / h * rows as f64;
    (dy_rows.abs() >= 1.0).then(|| dx_cols / cols as f64 * period / dy_rows)
}

/// The navigation bar's thumb position (0..1 of the thumb's travel) for a view
/// starting at `start` and showing `shown` of `len` samples, and the scrollable
/// range it maps to: the thumb reaches the right end when the last part of the
/// file is shown.
pub fn nav_position(start: f64, shown: f64, len: f64) -> (f64, f64) {
    let range = (len - shown).max(1.0);
    ((start / range).clamp(0.0, 1.0), range)
}

/// The text for a raster job that failed with `err`.
///
/// Analysis errors are lower-case clauses; many already say what to change after a
/// `;`, so the generic advice is added only when the error has none.
pub fn raster_error_text(err: &str) -> String {
    let err = err.trim().trim_end_matches('.');
    let mut chars = err.chars();
    let err = chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().chain(chars).collect::<String>()
    });
    if err.contains(';') {
        t!("raster-not-built", error = err)
    } else {
        t!("raster-not-built-advice", error = err)
    }
}

/// The raster request for a plot of `w` x `h` pixels.
pub fn request(
    start: f64,
    period: f64,
    strobe: usize,
    y_scale: usize,
    (w, h): (i32, i32),
) -> RasterRequest {
    let y_scale = y_scale.clamp(1, Y_SCALE_MAX);
    RasterRequest {
        start,
        period,
        strobe,
        rows: ((f64::from(h) / y_scale as f64).round() as usize).clamp(4, 4096),
        cols: (w.max(0) as usize).clamp(8, 4096),
    }
}

/// `a` and `b` ask for the same raster, whatever the plot size.
pub fn same_settings(a: RasterRequest, b: RasterRequest) -> bool {
    let sized = |r: RasterRequest| RasterRequest {
        rows: 0,
        cols: 0,
        ..r
    };
    sized(a) == sized(b)
}

/// The colours a raster image is drawn with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterLook {
    /// `Bright`, 0 to 1.
    pub bright: f64,
    /// The offset scroll bar, -1 to 1.
    pub offset: f64,
    /// SA's colours.
    pub map: [Rgba8; 4],
    /// SigNexora extra: another colour map instead of SA's.
    pub own: Option<Colormap>,
}

/// The image of raster `r` whose values span `range`.
pub fn raster_image(r: &Raster, range: (f32, f32), look: &RasterLook) -> SonoImage {
    let lut = look.own.map(Colormap::lut);
    let last = lut.as_ref().map_or(0, |l| l.len().saturating_sub(1)) as f32;
    let pixels = r
        .values
        .iter()
        .map(|v| {
            if !v.is_finite() {
                return Rgba8::BLACK;
            }
            let t = level(*v, range, look.bright, look.offset);
            match &lut {
                Some(l) => l
                    .get((t * last).round() as usize)
                    .copied()
                    .unwrap_or(Rgba8::BLACK),
                None => vmw_color(t, &look.map),
            }
        })
        .collect();
    SonoImage {
        width: r.cols,
        height: r.rows,
        pixels,
    }
}

/// A raster with its value range (computed in the job).
type RangedRaster = (Raster, (f32, f32));

/// The raster of one VMW window: built in the background for the line period,
/// start and strobe, and the image made from it.
#[derive(Default)]
pub struct RasterCache {
    job: Option<(RasterRequest, JobHandle<RangedRaster>)>,
    /// The raster on screen and the request it was built for, with its range.
    shown: Option<(RasterRequest, Raster, (f32, f32))>,
    /// The request whose job failed; not retried until the settings change.
    failed: Option<RasterRequest>,
    error: Option<String>,
    /// Bumped whenever a raster arrives.
    count: u64,
}

impl std::fmt::Debug for RasterCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RasterCache")
            .field("running", &self.job.is_some())
            .field("count", &self.count)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl RasterCache {
    /// Collect a finished raster job and start a new one when the request changed.
    pub fn sync(&mut self, signal: &Signal, req: RasterRequest) {
        if self.job.as_ref().is_some_and(|(_, h)| h.is_finished())
            && let Some((r, h)) = self.job.take()
        {
            match h.join() {
                Ok((raster, range)) => {
                    self.shown = Some((r, raster, range));
                    self.failed = None;
                    self.error = None;
                    self.count += 1;
                }
                Err(JobError::Cancelled) => {}
                Err(e) => {
                    self.failed = Some(r);
                    self.error = Some(raster_error_text(&e.to_string()));
                    self.count += 1;
                }
            }
        }
        if self.failed.is_some_and(|f| !same_settings(f, req)) {
            self.failed = None;
            self.error = None;
        }
        if self.needs_job(req) {
            if let Some((_, h)) = self.job.take() {
                h.cancel();
            }
            let s = signal.clone();
            self.job = Some((
                req,
                job::spawn(t!("raster-job"), move |ctx| {
                    ctx.checkpoint()?;
                    let stop = || ctx.is_cancelled();
                    let raster = build_with_stop(&s, &req, &stop).map_err(analysis_to_job)?;
                    let range = raster.auto_range().unwrap_or((0.0, 1.0));
                    Ok((raster, range))
                }),
            ));
        }
    }

    /// True when `req` should be computed: it is neither shown, running nor failed.
    pub fn needs_job(&self, req: RasterRequest) -> bool {
        let shown = self.shown.as_ref().map(|(r, _, _)| *r);
        let pending = self.job.as_ref().map(|(r, _)| *r);
        shown != Some(req)
            && pending != Some(req)
            && !self.failed.is_some_and(|f| same_settings(f, req))
    }

    /// The request of the raster on screen.
    pub fn shown_request(&self) -> Option<RasterRequest> {
        self.shown.as_ref().map(|(r, _, _)| *r)
    }

    /// The image of the raster on screen.
    pub fn image(&self, look: &RasterLook) -> Option<SonoImage> {
        let (_, r, range) = self.shown.as_ref()?;
        Some(raster_image(r, *range, look))
    }

    /// Why the last raster was not built.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// A build is running.
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Bumped whenever a raster (or an error) arrives.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Stop a running build and drop the raster (the window closed).
    pub fn release(&mut self) {
        if let Some((_, h)) = self.job.take() {
            h.cancel();
        }
        self.shown = None;
    }

    /// Pretend that `req` failed with `error` (tests).
    #[doc(hidden)]
    pub fn set_failed(&mut self, req: RasterRequest, error: &str) {
        self.failed = Some(req);
        self.error = Some(error.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ts_click_survives_a_minimum_above_the_field_range() {
        let v = ts_click(1.0, 0, true, 200.0);
        assert!(v.is_finite() && v <= 99.999_999_999, "{v}");
    }

    #[test]
    fn line_period_reads_as_sa() {
        assert_eq!(ts_text(0.170_089_234), "00:170.089.234");
        assert_eq!(ts_text(4.000_627), "04:000.627.000");
        assert_eq!(dt_text(0.0), "0:000.000.000");
        assert_eq!(dt_text(0.125_666_666), "0:125.666.666");
        assert_eq!(status_text(0.0), "dT = 0:000.000.000 | Speed = 0");
        assert_eq!(status_text(0.01), "dT = 0:010.000.000 | Speed = 100");
        assert_eq!(y_scale_text(2), "Y scale - 2");
    }

    #[test]
    fn ts_digit_clicks_add_and_subtract_one() {
        let ts = 0.170_089_234;
        assert_eq!(ts_text(ts_click(ts, 3, true, 1e-9)), "00:270.089.234");
        assert_eq!(ts_text(ts_click(ts, 3, false, 1e-9)), "00:070.089.234");
        assert_eq!(ts_text(ts_click(ts, 13, true, 1e-9)), "00:170.089.235");
        assert_eq!(ts_text(ts_click(ts, 9, false, 1e-9)), "00:170.088.234");
        assert_eq!(ts_text(ts_click(ts, 1, true, 1e-9)), "01:170.089.234");
        assert_eq!(ts_text(ts_click(0.009, 5, true, 1e-9)), "00:010.000.000");
        assert!((ts_click(ts, 2, true, 1e-9) - ts).abs() < 1e-15);
        assert!((ts_click(ts, 6, false, 1e-9) - ts).abs() < 1e-15);
        assert!((ts_click(0.001, 3, false, 0.000_125) - 0.000_125).abs() < 1e-15);
        for (i, c) in "00:170.089.234".chars().enumerate() {
            assert_eq!(ts_digit_value(i).is_some(), c.is_ascii_digit(), "{i}");
        }
    }

    #[test]
    fn strobe_range_is_1_to_1000() {
        assert_eq!(parse_strobe("1"), Some(1));
        assert_eq!(parse_strobe(" 1000 "), Some(1000));
        assert_eq!(parse_strobe("0"), None);
        assert_eq!(parse_strobe("1001"), None);
        assert_eq!(parse_strobe("x"), None);
    }

    #[test]
    fn period_slider_is_logarithmic_and_round_trips() {
        let limits = period_limits(1_000_000);
        assert_eq!(limits, (2.0, 500_000.0));
        assert!(period_to_slider(2.0, limits).abs() < 1e-12);
        assert!((period_to_slider(500_000.0, limits) - 1.0).abs() < 1e-12);
        for p in [3.0, 100.0, 1333.33, 250_000.0] {
            let v = period_to_slider(p, limits);
            assert!((slider_to_period(v, limits) - p).abs() / p < 1e-9);
        }
        assert!((slider_to_period(0.5, limits) - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn colour_map_runs_blue_black_red_orange() {
        let map = SA_MAP;
        assert_eq!(vmw_color(0.0, &map), map[0]);
        assert_eq!(vmw_color(0.5, &map), map[1]);
        assert_eq!(vmw_color(0.75, &map), map[2]);
        assert_eq!(vmw_color(1.0, &map), map[3]);
        assert!((level(0.0, (-1.0, 1.0), 0.5, 0.0) - 0.5).abs() < 1e-6);
        assert!(level(0.5, (-1.0, 1.0), 1.0, 0.0) > level(0.5, (-1.0, 1.0), 0.5, 0.0));
        assert!(level(0.0, (-1.0, 1.0), 0.5, 0.5) > 0.5);
        let lines = info_lines(0.0, 0.17);
        assert_eq!(lines[0], "dT = 00:000.000.000");
        assert_eq!(lines[1], "S = 0");
        assert!(lines[2].starts_with("TSs = 5"));
    }

    #[test]
    fn drag_uses_the_shown_raster_size() {
        let d = drag_drift((2000.0, 1000.0), (2000.0, 1000.0), (4096, 1000), 500.0).unwrap();
        assert!((d - 500.0 / 1000.0).abs() < 1e-9);
        assert!(drag_drift((10.0, 0.1), (400.0, 200.0), (400, 200), 1000.0).is_none());
    }

    #[test]
    fn nav_thumb_spans_the_scrollable_range() {
        let (pos, range) = nav_position(0.0, 800.0, 1000.0);
        assert_eq!((pos, range), (0.0, 200.0));
        let (pos, _) = nav_position(200.0, 800.0, 1000.0);
        assert!((pos - 1.0).abs() < 1e-12);
        assert_eq!(nav_position(5.0, 2000.0, 1000.0).0, 1.0);
    }

    #[test]
    fn raster_error_is_capitalised_and_advises_once() {
        assert_eq!(
            raster_error_text("the line period exceeds the signal length; shorten it"),
            "Raster not built. The line period exceeds the signal length; shorten it."
        );
        assert_eq!(
            raster_error_text("not enough samples."),
            "Raster not built. Not enough samples. Change TS or the strobe."
        );
    }

    #[test]
    fn failed_request_is_not_restarted_on_resize() {
        let req = request(0.0, 1e9, 1, 2, (8, 8));
        let mut c = RasterCache::default();
        c.set_failed(req, "x");
        assert!(!c.needs_job(req));
        assert!(!c.needs_job(RasterRequest {
            rows: 20,
            cols: 30,
            ..req
        }));
        assert!(c.needs_job(RasterRequest {
            period: 100.0,
            ..req
        }));
    }

    #[test]
    fn rasters_are_built_and_drawn() {
        let x: Vec<f32> = (0..20_000).map(|i| (i as f32 * 0.01).sin()).collect();
        let s = Signal::real(x, 8000.0).unwrap();
        let req = request(0.0, 100.0, 1, 2, (100, 80));
        let mut c = RasterCache::default();
        for _ in 0..2000 {
            c.sync(&s, req);
            if !c.busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(c.shown_request(), Some(req));
        let look = RasterLook {
            bright: 0.5,
            offset: 0.0,
            map: SA_MAP,
            own: None,
        };
        let img = c.image(&look).unwrap();
        assert_eq!((img.width, img.height), (100, 40));
        assert_eq!(img.pixels.len(), 4000);
    }
}
