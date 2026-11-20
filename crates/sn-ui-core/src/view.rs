//! Mapping between screen pixels and signal time/frequency, with zoom and pan.
//!
//! Time is measured in samples. The visible start is kept on a multiple of
//! `samples_per_px`, so sonogram columns stay aligned to a fixed grid while panning
//! and previously computed columns can be reused.

/// Visible time and frequency ranges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// First visible sample (multiple of `samples_per_px`).
    pub start: f64,
    /// Samples per horizontal pixel.
    pub samples_per_px: f64,
    /// Lowest visible frequency in Hz (bottom edge).
    pub f_lo: f64,
    /// Highest visible frequency in Hz (top edge).
    pub f_hi: f64,
}

/// The signal's full extent, used to clamp the view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    /// Number of samples.
    pub len: f64,
    /// Lowest frequency the spectrum can show.
    pub f_min: f64,
    /// Highest frequency the spectrum can show.
    pub f_max: f64,
}

/// Smallest zoom: 1/16 sample per pixel (each sample 16 px wide).
pub const MIN_SAMPLES_PER_PX: f64 = 1.0 / 16.0;
/// Smallest visible frequency span in Hz.
const MIN_FREQ_SPAN: f64 = 1e-3;

impl View {
    /// Whole signal and spectrum, `width` pixels wide.
    pub fn fit(extent: Extent, width: f64) -> Self {
        let spp = (extent.len / width.max(1.0)).max(MIN_SAMPLES_PER_PX);
        Self {
            start: 0.0,
            samples_per_px: spp,
            f_lo: extent.f_min,
            f_hi: extent.f_max,
        }
    }

    /// Sample at horizontal pixel `x`.
    pub fn x_to_sample(&self, x: f64) -> f64 {
        self.start + x * self.samples_per_px
    }

    /// Horizontal pixel of sample `s`.
    pub fn sample_to_x(&self, s: f64) -> f64 {
        (s - self.start) / self.samples_per_px
    }

    /// Frequency at vertical pixel `y` (0 at the top) in a view `height` pixels tall.
    pub fn y_to_freq(&self, y: f64, height: f64) -> f64 {
        self.f_hi - y / height.max(1.0) * (self.f_hi - self.f_lo)
    }

    /// Vertical pixel of frequency `f`.
    pub fn freq_to_y(&self, f: f64, height: f64) -> f64 {
        (self.f_hi - f) / (self.f_hi - self.f_lo).max(MIN_FREQ_SPAN) * height
    }

    /// Index of the first sonogram column on screen: column `c` covers samples
    /// `c * samples_per_px .. (c + 1) * samples_per_px`.
    pub fn first_column(&self) -> i64 {
        (self.start / self.samples_per_px).round() as i64
    }

    fn snap(&mut self) {
        self.start = (self.start / self.samples_per_px).round() * self.samples_per_px;
    }

    fn clamp_time(&mut self, extent: Extent, width: f64) {
        let max_spp = (extent.len / width.max(1.0)).max(MIN_SAMPLES_PER_PX);
        self.samples_per_px = self.samples_per_px.clamp(MIN_SAMPLES_PER_PX, max_spp);
        let visible = width * self.samples_per_px;
        let max_start = (extent.len - visible).max(0.0);
        self.start = self.start.clamp(0.0, max_start);
        self.snap();
        if self.start > max_start {
            self.start -= self.samples_per_px;
        }
        self.start = self.start.max(0.0);
    }

    fn clamp_freq(&mut self, extent: Extent) {
        let full = extent.f_max - extent.f_min;
        let span = (self.f_hi - self.f_lo).clamp(MIN_FREQ_SPAN, full.max(MIN_FREQ_SPAN));
        let lo = self
            .f_lo
            .clamp(extent.f_min, (extent.f_max - span).max(extent.f_min));
        self.f_lo = lo;
        self.f_hi = lo + span;
    }

    /// Zoom time by `factor` (<1 zooms in) keeping the sample under pixel `anchor_x`
    /// fixed.
    pub fn zoom_time(&mut self, factor: f64, anchor_x: f64, extent: Extent, width: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let anchor = self.x_to_sample(anchor_x);
        self.samples_per_px *= factor;
        self.clamp_time(extent, width);
        self.start = anchor - anchor_x * self.samples_per_px;
        self.clamp_time(extent, width);
    }

    /// Pan by `dx` pixels (positive moves later in time).
    pub fn pan_time(&mut self, dx: f64, extent: Extent, width: f64) {
        if dx.is_finite() {
            self.start += dx * self.samples_per_px;
            self.clamp_time(extent, width);
        }
    }

    /// Zoom frequency by `factor` keeping the frequency under pixel `anchor_y` fixed.
    pub fn zoom_freq(&mut self, factor: f64, anchor_y: f64, height: f64, extent: Extent) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let anchor = self.y_to_freq(anchor_y, height);
        let frac = (anchor_y / height.max(1.0)).clamp(0.0, 1.0);
        let span = (self.f_hi - self.f_lo) * factor;
        self.f_hi = anchor + frac * span;
        self.f_lo = self.f_hi - span;
        self.clamp_freq(extent);
    }

    /// Show samples `a..b` and frequencies `f1..f2` (either may be `None` to keep the
    /// current range).
    pub fn show(
        &mut self,
        time: Option<(f64, f64)>,
        freq: Option<(f64, f64)>,
        extent: Extent,
        width: f64,
    ) {
        if let Some((a, b)) = time {
            let (a, b) = (a.min(b), a.max(b));
            if b > a {
                self.samples_per_px = (b - a) / width.max(1.0);
                self.start = a;
                self.clamp_time(extent, width);
            }
        }
        if let Some((f1, f2)) = freq {
            let (lo, hi) = (f1.min(f2), f1.max(f2));
            if hi > lo {
                self.f_lo = lo;
                self.f_hi = hi;
                self.clamp_freq(extent);
            }
        }
    }

    /// Re-apply limits after the widget was resized.
    pub fn resized(&mut self, extent: Extent, width: f64) {
        self.clamp_time(extent, width);
        self.clamp_freq(extent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXT: Extent = Extent {
        len: 480_000.0,
        f_min: 0.0,
        f_max: 24_000.0,
    };

    #[test]
    fn fit_shows_everything() {
        let v = View::fit(EXT, 1000.0);
        assert!((v.samples_per_px - 480.0).abs() < 1e-9);
        assert!((v.x_to_sample(1000.0) - 480_000.0).abs() < 1e-9);
        assert!((v.y_to_freq(0.0, 600.0) - 24_000.0).abs() < 1e-9);
        assert!((v.y_to_freq(600.0, 600.0)).abs() < 1e-9);
    }

    #[test]
    fn zoom_keeps_anchor_and_stays_on_grid() {
        let mut v = View::fit(EXT, 1000.0);
        let before = v.x_to_sample(250.0);
        v.zoom_time(0.1, 250.0, EXT, 1000.0);
        assert!((v.samples_per_px - 48.0).abs() < 1e-9);
        assert!((v.x_to_sample(250.0) - before).abs() <= v.samples_per_px);
        assert!((v.start / v.samples_per_px).fract().abs() < 1e-9);
        // Cannot zoom out past the whole file.
        v.zoom_time(1000.0, 0.0, EXT, 1000.0);
        assert!((v.samples_per_px - 480.0).abs() < 1e-9);
        assert!(v.start.abs() < 1e-9);
    }

    #[test]
    fn pan_is_clamped() {
        let mut v = View::fit(EXT, 1000.0);
        v.zoom_time(0.5, 0.0, EXT, 1000.0);
        v.pan_time(1e9, EXT, 1000.0);
        assert!(v.x_to_sample(1000.0) <= EXT.len + 1e-6);
        v.pan_time(-1e9, EXT, 1000.0);
        assert!(v.start.abs() < 1e-9);
    }

    #[test]
    fn show_selection_and_freq_zoom() {
        let mut v = View::fit(EXT, 1000.0);
        v.show(
            Some((48_000.0, 96_000.0)),
            Some((1500.0, 2100.0)),
            EXT,
            1000.0,
        );
        assert!((v.samples_per_px - 48.0).abs() < 1e-9);
        assert!((v.start - 48_000.0).abs() < 1e-9);
        assert!((v.f_lo - 1500.0).abs() < 1e-9 && (v.f_hi - 2100.0).abs() < 1e-9);
        let f_mid = v.y_to_freq(300.0, 600.0);
        v.zoom_freq(0.5, 300.0, 600.0, EXT);
        assert!((v.y_to_freq(300.0, 600.0) - f_mid).abs() < 1e-6);
        assert!((v.f_hi - v.f_lo - 300.0).abs() < 1e-6);
        v.zoom_freq(1e6, 300.0, 600.0, EXT);
        assert!((v.f_lo - EXT.f_min).abs() < 1e-9 && (v.f_hi - EXT.f_max).abs() < 1e-9);
    }

    #[test]
    fn y_and_freq_round_trip() {
        let v = View::fit(EXT, 800.0);
        let y = v.freq_to_y(6000.0, 400.0);
        assert!((v.y_to_freq(y, 400.0) - 6000.0).abs() < 1e-9);
        assert!((v.sample_to_x(v.x_to_sample(123.0)) - 123.0).abs() < 1e-9);
    }
}
