//! The sonogram image of SA's work window without a UI toolkit: the spectrum
//! engine for the toolbar settings, the column cache filled by a background
//! job, the reduction of columns to screen rows, and the RGBA image.
//!
//! The same computation as `sa-app`'s sonogram view (its `SonogramState`); the
//! front end turns [`SonoImage`] into a texture and draws the markers over it.
//!
//! Memory and UI thread: each column is kept reduced to the screen rows (small),
//! and its full spectrum only within [`CACHE_BUDGET_BYTES`], nearest the view
//! first. Spectra are computed and reduced by a background job; the UI thread
//! only reduces inline when the work is small (a frequency zoom at small FFT
//! sizes), so large FFT sizes never stall a frame.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use sn_core::job::{self, JobError, JobHandle};
use sn_core::{NodeId, Signal};
use sn_dsp::spectrum::{ColumnRequest, Combine, SpectrumEngine, sonogram_columns};
use sn_dsp::window::WindowKind;

use crate::color::Rgba8;
use crate::colormap::{Colormap, PENDING};
use crate::jobs::dsp_to_job;
use crate::sono_settings::Settings;
use crate::t;
use crate::view::{Extent, View};

/// Everything that makes cached columns valid.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CacheKey {
    node: NodeId,
    fft_size: usize,
    hop: usize,
    window: WindowKind,
    combine: Combine,
    hres: bool,
    samples_per_px: f64,
}

/// For each screen row (top to bottom), the range of spectrum bins it covers.
type RowMap = Arc<[(usize, usize)]>;

/// One cached display column.
#[derive(Debug, Clone)]
enum Column {
    /// Entirely outside the recording.
    Outside,
    /// Inside the recording. At least one of the two is present.
    Data {
        /// Spectrum in dBFS, one value per bin; dropped beyond the memory budget.
        full: Option<Arc<[f32]>>,
        /// The spectrum reduced to the current screen rows (max per row, NaN for
        /// rows outside the spectrum); `None` until reduced for them.
        reduced: Option<Arc<[f32]>>,
    },
}

/// What a column job hands back: per column, the full spectrum when it may be
/// kept, and the spectrum reduced to the job's rows.
type Done = Vec<(i64, Option<Arc<[f32]>>, Arc<[f32]>)>;

/// The running column job.
struct Running {
    key: CacheKey,
    /// The rows it reduces to.
    rows: RowMap,
    /// A margin (prefetch) job: visible work cancels it.
    prefetch: bool,
    /// The columns it works on.
    columns: Vec<i64>,
    handle: JobHandle<Done>,
}

/// What makes the rendered image current.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RenderKey {
    view: View,
    size: [usize; 2],
    generation: u64,
    range_db: f32,
    alc: bool,
    colormap: Colormap,
    outside: Rgba8,
}

/// A rendered sonogram: `width` x `height` pixels, rows top to bottom.
#[derive(Debug, Clone, PartialEq)]
pub struct SonoImage {
    /// Width in pixels (one column per pixel).
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// The pixels, row by row.
    pub pixels: Vec<Rgba8>,
}

/// Memory the full spectra of the column cache may use (a hard cap). Columns hold
/// one f32 per bin, so at large FFT sizes far fewer full columns fit than at small
/// ones; the reduced columns the image is drawn from are kept besides, within the
/// same budget for their (much smaller) size.
pub const CACHE_BUDGET_BYTES: usize = 256 * 1024 * 1024;

/// How many full spectra of `bins` values fit `budget` bytes.
fn columns_in(budget: usize, bins: usize) -> usize {
    budget / bins.max(1).saturating_mul(size_of::<f32>())
}

/// How many full spectra of `bins` values the cache keeps at most.
pub fn column_budget(bins: usize) -> usize {
    columns_in(CACHE_BUDGET_BYTES, bins)
}

/// Maximum FFT frames combined into one display column.
const MAX_FRAMES_PER_COLUMN: usize = 24;

/// Memory one step of a column job may hold in full spectra at once.
const CHUNK_BYTES: usize = 32 * 1024 * 1024;

/// Bin comparisons the UI thread may spend reducing columns in one frame; more
/// goes to a job.
const INLINE_REDUCE_WORK: usize = 4_000_000;

/// Column `c` lies entirely outside a recording of `len` samples.
fn is_outside(c: i64, samples_per_px: f64, len: f64) -> bool {
    let start = c as f64 * samples_per_px;
    start + samples_per_px <= 0.0 || start >= len
}

/// Sorted columns split into contiguous runs of at most `chunk` columns.
fn runs_of(cols: &[i64], chunk: usize) -> Vec<Range<i64>> {
    let chunk = i64::try_from(chunk.max(1)).unwrap_or(i64::MAX);
    let mut out = Vec::new();
    let mut it = cols.iter().copied();
    let Some(mut lo) = it.next() else {
        return out;
    };
    let mut hi = lo.saturating_add(1);
    for c in it {
        if c == hi && hi - lo < chunk {
            hi = hi.saturating_add(1);
        } else {
            out.push(lo..hi);
            lo = c;
            hi = c.saturating_add(1);
        }
    }
    out.push(lo..hi);
    out
}

/// Distance of column `c` from the view `visible` (0 inside it).
fn distance(c: i64, visible: &Range<i64>) -> i64 {
    if c < visible.start {
        visible.start.saturating_sub(c)
    } else if c >= visible.end {
        c.saturating_sub(visible.end).saturating_add(1)
    } else {
        0
    }
}

/// The sonogram of one window: engine, column cache, running job and image.
pub struct SonoCache {
    engine: Option<(Settings, SpectrumEngine)>,
    key: Option<CacheKey>,
    columns: HashMap<i64, Column>,
    /// The screen rows the reduced columns are for.
    rows: RowMap,
    /// The columns last shown.
    visible: Range<i64>,
    generation: u64,
    job: Option<Running>,
    /// The cache key whose job failed; not retried until the key changes.
    failed: Option<CacheKey>,
    rendered: Option<RenderKey>,
    image: Option<Arc<SonoImage>>,
    /// Counts the images rendered, so a front end knows when to upload again.
    image_count: u64,
    /// The spectrum engine could not be built.
    error: Option<String>,
    /// The last column job failed.
    job_error: Option<String>,
    /// Bytes of full spectra kept at most ([`CACHE_BUDGET_BYTES`]).
    budget: usize,
}

impl Default for SonoCache {
    fn default() -> Self {
        Self {
            engine: None,
            key: None,
            columns: HashMap::new(),
            rows: Arc::from(Vec::new()),
            visible: 0..0,
            generation: 0,
            job: None,
            failed: None,
            rendered: None,
            image: None,
            image_count: 0,
            error: None,
            job_error: None,
            budget: CACHE_BUDGET_BYTES,
        }
    }
}

impl std::fmt::Debug for SonoCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SonoCache")
            .field("columns", &self.columns.len())
            .field("busy", &self.job.is_some())
            .field("image_count", &self.image_count)
            .finish_non_exhaustive()
    }
}

impl SonoCache {
    /// The spectrum engine for `s`, rebuilt when the settings change.
    pub fn engine(&mut self, s: &Settings, signal: &Signal) -> Option<&SpectrumEngine> {
        let needs = match &self.engine {
            Some((old, e)) => {
                old.fft_size != s.fft_size
                    || old.window != s.window
                    || old.hres != s.hres
                    || e.kind() != signal.kind()
            }
            None => true,
        };
        if needs {
            match SpectrumEngine::new(s.fft_size, s.window, signal.kind()) {
                Ok(e) => {
                    self.engine = Some((*s, e.with_reassignment(s.hres)));
                    self.error = None;
                }
                Err(e) => {
                    self.engine = None;
                    self.error = Some(e.to_string());
                }
            }
        }
        self.engine.as_ref().map(|(_, e)| e)
    }

    /// The spectrum extent of `signal` under `s`.
    pub fn extent(&mut self, s: &Settings, signal: &Signal) -> Option<Extent> {
        let fs = signal.sample_rate();
        let len = signal.len() as f64;
        self.engine(s, signal).map(|e| {
            let (f_min, f_max) = e.frequency_range(fs);
            Extent { len, f_min, f_max }
        })
    }

    /// dBFS at plot column `x` and frequency `hz`, if that column is computed: the
    /// bin's level while the full spectrum is kept, else the level of the screen
    /// row drawn at `hz` (the strongest bin of that row).
    pub fn level_at(&self, signal: &Signal, view: &View, x: f64, hz: f64) -> Option<f32> {
        let (_, engine) = self.engine.as_ref()?;
        let col = view.first_column() + x.floor() as i64;
        let Column::Data { full, reduced } = self.columns.get(&col)? else {
            return None;
        };
        if let Some(full) = full {
            let bin = engine.frequency_to_bin(hz, signal.sample_rate()).round();
            if bin < 0.0 {
                return None;
            }
            return full.get(bin as usize).copied();
        }
        let reduced = reduced.as_ref()?;
        let y = view.freq_to_y(hz, self.rows.len() as f64).floor();
        if y < 0.0 {
            return None;
        }
        reduced.get(y as usize).copied().filter(|v| !v.is_nan())
    }

    /// The spectrum engine error, if the engine could not be built. A failed
    /// column job is reported by [`Self::job_error`].
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The last column job's error.
    pub fn job_error(&self) -> Option<&str> {
        self.job_error.as_deref()
    }

    /// True while columns are being computed.
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// The last rendered image.
    pub fn image(&self) -> Option<&Arc<SonoImage>> {
        self.image.as_ref()
    }

    /// How many images were rendered so far (changes with every new image).
    pub fn image_count(&self) -> u64 {
        self.image_count
    }

    /// Drop the cache, the image and the running job.
    pub fn release(&mut self) {
        self.cancel_job();
        *self = Self {
            budget: self.budget,
            ..Self::default()
        };
    }

    /// One step: collect a finished job, ask for the visible columns and render
    /// the image of `size` (columns, rows) if anything visible changed. Columns
    /// outside the recording are drawn in `outside`. Returns whether a new image
    /// was rendered.
    pub fn update(
        &mut self,
        s: &Settings,
        signal: &Signal,
        node: NodeId,
        view: &View,
        size: [usize; 2],
        outside: Rgba8,
    ) -> bool {
        self.collect_job();
        if self.engine(s, signal).is_none() {
            return false;
        }
        self.request_visible(s, signal, node, view, size);
        self.render(s, view, size, outside)
    }

    fn cancel_job(&mut self) {
        if let Some(r) = self.job.take() {
            r.handle.cancel();
        }
    }

    fn bump(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn sync_cache(&mut self, key: CacheKey) {
        if self.key != Some(key) {
            self.key = Some(key);
            self.columns.clear();
            self.failed = None;
            self.job_error = None;
            self.bump();
            self.cancel_job();
        }
    }

    /// New screen rows: every reduced column is stale. Columns that keep their
    /// full spectrum are reduced again; the others are computed again.
    fn sync_rows(&mut self, rows: Vec<(usize, usize)>) {
        if *self.rows == *rows {
            return;
        }
        self.rows = Arc::from(rows);
        self.columns.retain(|_, col| match col {
            Column::Outside => true,
            Column::Data { full, reduced } => {
                *reduced = None;
                full.is_some()
            }
        });
        self.bump();
        self.cancel_job();
    }

    fn collect_job(&mut self) {
        if !self.job.as_ref().is_some_and(|r| r.handle.is_finished()) {
            return;
        }
        let Some(run) = self.job.take() else {
            return;
        };
        match run.handle.join() {
            Ok(done) if Some(run.key) == self.key && Arc::ptr_eq(&run.rows, &self.rows) => {
                for (c, full, reduced) in done {
                    // A job that only reduced, or could not keep the spectrum,
                    // leaves a spectrum already cached in place.
                    let full = full.or_else(|| match self.columns.get(&c) {
                        Some(Column::Data { full, .. }) => full.clone(),
                        _ => None,
                    });
                    self.columns.insert(
                        c,
                        Column::Data {
                            full,
                            reduced: Some(reduced),
                        },
                    );
                }
                self.job_error = None;
                self.bump();
                self.evict(0);
            }
            Ok(_) | Err(JobError::Cancelled) => {}
            Err(e) => {
                // Remember the key so the same failing job is not restarted
                // on every frame.
                self.failed = Some(run.key);
                self.job_error = Some(e.to_string());
            }
        }
    }

    /// Keep the cache within its budget, nearest the view first, leaving room
    /// for `reserve` new full spectra.
    fn evict(&mut self, reserve: usize) {
        let bins = self.engine.as_ref().map_or(1, |(_, e)| e.bins());
        let full_cap = columns_in(self.budget, bins).saturating_sub(reserve);
        let visible = self.visible.clone();
        let mut with_full: Vec<i64> = self
            .columns
            .iter()
            .filter(|(_, col)| matches!(col, Column::Data { full: Some(_), .. }))
            .map(|(c, _)| *c)
            .collect();
        if with_full.len() > full_cap {
            with_full.sort_by_key(|c| distance(*c, &visible));
            for c in with_full.into_iter().skip(full_cap) {
                let drop = match self.columns.get_mut(&c) {
                    Some(Column::Data { full, reduced }) => {
                        *full = None;
                        reduced.is_none()
                    }
                    _ => false,
                };
                if drop {
                    self.columns.remove(&c);
                }
            }
        }
        // The reduced columns: far smaller, but not unbounded either.
        let shown = usize::try_from(visible.end - visible.start).unwrap_or(0);
        let per_reduced = self.rows.len().max(1).saturating_mul(size_of::<f32>());
        let cap = (self.budget / per_reduced).max(shown.saturating_mul(2));
        if self.columns.len() > cap {
            let mut all: Vec<i64> = self.columns.keys().copied().collect();
            all.sort_by_key(|c| distance(*c, &visible));
            for c in all.into_iter().skip(cap) {
                self.columns.remove(&c);
            }
        }
    }

    /// The work columns `cols` need: `(column, full spectrum)` for a column to
    /// reduce, `(column, None)` for one to compute. Columns outside the recording
    /// are marked at once, without a job.
    fn plan(
        &mut self,
        cols: Range<i64>,
        samples_per_px: f64,
        len: f64,
    ) -> Vec<(i64, Option<Arc<[f32]>>)> {
        let mut work = Vec::new();
        for c in cols {
            match self.columns.get(&c) {
                Some(
                    Column::Outside
                    | Column::Data {
                        reduced: Some(_), ..
                    },
                ) => {}
                Some(Column::Data {
                    full: Some(f),
                    reduced: None,
                }) => work.push((c, Some(Arc::clone(f)))),
                Some(Column::Data {
                    full: None,
                    reduced: None,
                })
                | None => {
                    if is_outside(c, samples_per_px, len) {
                        self.columns.insert(c, Column::Outside);
                        self.bump();
                    } else {
                        work.push((c, None));
                    }
                }
            }
        }
        work
    }

    /// Reduce `work` (all with full spectra) on this thread.
    fn reduce_inline(&mut self, work: Vec<(i64, Option<Arc<[f32]>>)>) {
        for (c, full) in work {
            if let (Some(full), Some(Column::Data { reduced, .. })) =
                (full, self.columns.get_mut(&c))
            {
                *reduced = Some(reduce_column(&full, &self.rows).into());
            }
        }
        self.bump();
    }

    /// Start a job for `work`.
    fn spawn(
        &mut self,
        signal: &Signal,
        key: CacheKey,
        work: Vec<(i64, Option<Arc<[f32]>>)>,
        prefetch: bool,
    ) {
        let Some((_, engine)) = self.engine.clone() else {
            return;
        };
        let bins = engine.bins();
        let compute: Vec<i64> = work
            .iter()
            .filter(|(_, f)| f.is_none())
            .map(|(c, _)| *c)
            .collect();
        let full_cap = columns_in(self.budget, bins);
        if !prefetch {
            // Visible columns come first: make room for their spectra.
            self.evict(compute.len().min(full_cap));
        }
        let cached = self
            .columns
            .values()
            .filter(|col| matches!(col, Column::Data { full: Some(_), .. }))
            .count();
        let keep_full = full_cap.saturating_sub(cached);
        let columns: Vec<i64> = work.iter().map(|(c, _)| *c).collect();
        let reduce: Vec<(i64, Arc<[f32]>)> = work
            .into_iter()
            .filter_map(|(c, f)| f.map(|f| (c, f)))
            .collect();
        let chunk = columns_in(CHUNK_BYTES, bins).clamp(8, 256);
        let runs = runs_of(&compute, chunk);
        let rows = Arc::clone(&self.rows);
        let job_rows = Arc::clone(&rows);
        let signal = signal.clone();
        let request = ColumnRequest {
            start: 0.0,
            samples_per_column: key.samples_per_px,
            hop: key.hop,
            max_frames_per_column: MAX_FRAMES_PER_COLUMN,
            combine: key.combine,
        };
        let handle = job::spawn(t!("sono-job-computing"), move |ctx| {
            ctx.set_total(runs.len() as u64 + 1);
            let stop = || ctx.is_cancelled();
            let mut out: Done = Vec::with_capacity(reduce.len() + compute.len());
            for (c, full) in reduce {
                ctx.checkpoint()?;
                let r: Arc<[f32]> = reduce_column(&full, &job_rows).into();
                out.push((c, None, r));
            }
            ctx.advance(1);
            let mut kept = 0;
            for run in runs {
                // One chunk at a time, so the job never holds more than a chunk
                // of spectra beyond those it may keep.
                let cols =
                    sonogram_columns(&engine, &signal, &request, run, &stop).map_err(dsp_to_job)?;
                for (c, d) in cols {
                    let r: Arc<[f32]> = reduce_column(&d, &job_rows).into();
                    let full = if kept < keep_full {
                        kept += 1;
                        Some(Arc::from(d))
                    } else {
                        None
                    };
                    out.push((c, full, r));
                }
                ctx.advance(1);
            }
            Ok(out)
        });
        self.job = Some(Running {
            key,
            rows,
            prefetch,
            columns,
            handle,
        });
    }

    /// Ask for the visible columns (and a margin either side once those are done).
    fn request_visible(
        &mut self,
        s: &Settings,
        signal: &Signal,
        node: NodeId,
        view: &View,
        size: [usize; 2],
    ) {
        let [cols, height] = size;
        let first = view.first_column();
        let key = CacheKey {
            node,
            fft_size: s.fft_size,
            hop: s.hop(),
            window: s.window,
            combine: s.combine,
            hres: s.hres,
            samples_per_px: view.samples_per_px,
        };
        self.sync_cache(key);
        let Some((_, engine)) = &self.engine else {
            return;
        };
        let bins = engine.bins();
        let rows = row_bins(engine, signal.sample_rate(), view, height);
        self.sync_rows(rows);
        let n = i64::try_from(cols).unwrap_or(i64::MAX);
        let visible = first..first.saturating_add(n);
        self.visible = visible.clone();
        if self.failed == Some(key) {
            return;
        }
        let len = signal.len() as f64;
        let spp = key.samples_per_px;
        let work = self.plan(visible, spp, len);
        if !work.is_empty() {
            let visible = &self.visible;
            if self
                .job
                .as_ref()
                .is_some_and(|r| r.prefetch || !r.columns.iter().any(|c| visible.contains(c)))
            {
                // The view comes before the margin, and before columns no
                // longer in view.
                self.cancel_job();
            }
            if self.job.is_some() {
                return;
            }
            let all_reduce = work.iter().all(|(_, f)| f.is_some());
            if all_reduce && work.len().saturating_mul(bins) <= INLINE_REDUCE_WORK {
                self.reduce_inline(work);
            } else {
                self.spawn(signal, key, work, false);
            }
            return;
        }
        if self.job.is_some() {
            return;
        }
        let margin = n / 2;
        if margin > 0 {
            let end = first.saturating_add(n);
            let mut work = self.plan(first.saturating_sub(margin)..first, spp, len);
            work.extend(self.plan(end..end.saturating_add(margin), spp, len));
            if !work.is_empty() {
                self.spawn(signal, key, work, true);
            }
        }
    }

    /// Re-render the image if anything visible changed.
    fn render(&mut self, s: &Settings, view: &View, size: [usize; 2], outside: Rgba8) -> bool {
        let [cols, rows] = size;
        let rkey = RenderKey {
            view: *view,
            size,
            generation: self.generation,
            range_db: s.range_db,
            alc: s.alc,
            colormap: s.colormap,
            outside,
        };
        if self.rendered == Some(rkey) && self.image.is_some() {
            return false;
        }
        let first = view.first_column();
        let pixel_cols: Vec<PixelColumn<'_>> = (0..cols as i64)
            .map(|x| match self.columns.get(&(first + x)) {
                Some(Column::Outside) => PixelColumn::Outside,
                Some(Column::Data {
                    reduced: Some(r), ..
                }) => PixelColumn::Data(r),
                _ => PixelColumn::Pending,
            })
            .collect();
        let image = render_reduced(
            &pixel_cols,
            rows,
            s.range_db,
            s.alc,
            &s.colormap.lut(),
            outside,
        );
        self.image = Some(Arc::new(image));
        self.image_count = self.image_count.wrapping_add(1);
        self.rendered = Some(rkey);
        true
    }
}

/// What one screen column shows.
#[derive(Debug, Clone, Copy)]
pub enum PixelColumn<'a> {
    /// Not computed yet.
    Pending,
    /// Outside the recording.
    Outside,
    /// Spectrum values (one per screen row once reduced).
    Data(&'a [f32]),
}

/// For each screen row (top to bottom), the range of spectrum bins it covers. An
/// empty range means the row lies outside the spectrum.
pub fn row_bins(
    engine: &SpectrumEngine,
    sample_rate: f64,
    view: &View,
    height: usize,
) -> Vec<(usize, usize)> {
    let bins = engine.bins() as f64;
    let h = height as f64;
    (0..height)
        .map(|y| {
            let f_top = view.y_to_freq(y as f64, h);
            let f_bot = view.y_to_freq(y as f64 + 1.0, h);
            let b_top = engine.frequency_to_bin(f_top, sample_rate);
            let b_bot = engine.frequency_to_bin(f_bot, sample_rate);
            let lo = b_top.min(b_bot).round();
            let hi = b_top.max(b_bot).round();
            if hi < -0.5 || lo > bins - 0.5 {
                (0, 0)
            } else {
                let lo = lo.clamp(0.0, bins - 1.0) as usize;
                let hi = hi.clamp(0.0, bins - 1.0) as usize;
                (lo, hi + 1)
            }
        })
        .collect()
}

/// Reduce one spectrum column to screen rows: the strongest bin of each row, NaN
/// for a row outside the spectrum.
pub fn reduce_column(data: &[f32], rows: &[(usize, usize)]) -> Vec<f32> {
    rows.iter()
        .map(|&(lo, hi)| {
            data.get(lo..hi)
                .and_then(|s| s.iter().copied().reduce(f32::max))
                .unwrap_or(f32::NAN)
        })
        .collect()
}

/// Turn columns already reduced to `height` screen rows into an image.
pub fn render_reduced(
    columns: &[PixelColumn<'_>],
    height: usize,
    range_db: f32,
    alc: bool,
    lut: &[Rgba8],
    outside: Rgba8,
) -> SonoImage {
    let width = columns.len();
    let range = range_db.max(1.0);
    let col_max = |d: &[f32]| d.iter().copied().filter(|v| v.is_finite()).reduce(f32::max);
    // Reference level: strongest value in the visible rows (per column with ALC).
    let global_ref = columns
        .iter()
        .filter_map(|c| match c {
            PixelColumn::Data(d) => col_max(d),
            _ => None,
        })
        .reduce(f32::max)
        .unwrap_or(0.0);
    let last = lut.len().saturating_sub(1) as f32;
    let mut pixels = vec![PENDING; width.saturating_mul(height)];
    for (x, col) in columns.iter().enumerate() {
        let col_ref = match col {
            PixelColumn::Data(d) if alc => col_max(d).unwrap_or(0.0),
            _ => global_ref,
        };
        for y in 0..height {
            let color = match col {
                PixelColumn::Pending => PENDING,
                PixelColumn::Outside => outside,
                PixelColumn::Data(d) => match d.get(y).copied().filter(|v| !v.is_nan()) {
                    None => outside,
                    Some(v) => {
                        let t = ((v - (col_ref - range)) / range).clamp(0.0, 1.0);
                        lut.get((t * last).round() as usize)
                            .copied()
                            .unwrap_or(PENDING)
                    }
                },
            };
            if let Some(p) = pixels.get_mut(y * width + x) {
                *p = color;
            }
        }
    }
    SonoImage {
        width,
        height,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use sn_core::SampleKind;

    use super::*;
    use crate::colormap::OUTSIDE;

    #[test]
    fn pending_and_outside_columns() {
        let rows = [(0, 2), (2, 4), (0, 0)];
        let data = [-10.0_f32, -20.0, -60.0, -70.0];
        let reduced = reduce_column(&data, &rows);
        let px = [
            PixelColumn::Pending,
            PixelColumn::Outside,
            PixelColumn::Data(&reduced),
        ];
        let lut = Colormap::Gray.lut();
        let img = render_reduced(&px, rows.len(), 50.0, false, &lut, OUTSIDE);
        assert_eq!(img.pixels[0], PENDING);
        assert_eq!(img.pixels[1], OUTSIDE);
        assert_eq!(img.pixels[2], lut[255]);
        assert_eq!(img.pixels[3 + 2], lut[0]);
        assert_eq!(img.pixels[6 + 2], OUTSIDE);
    }

    #[test]
    fn cache_budget_is_a_hard_cap() {
        assert_eq!(column_budget(131_073), 511);
        for bins in [513, 4097, 131_073, 1 << 20] {
            assert!(column_budget(bins) * bins * 4 <= CACHE_BUDGET_BYTES);
        }
        assert_eq!(
            runs_of(&[-3, -2, 5, 6, 7, 8, 10], 3),
            vec![-3..-1, 5..8, 8..9, 10..11]
        );
    }

    fn tone(len: usize) -> Signal {
        let s: Vec<f32> = (0..len)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 8000.0).sin())
            .collect();
        Signal::real(s, 8000.0).unwrap()
    }

    /// Run `update` until no job is left.
    fn settle(cache: &mut SonoCache, s: &Settings, signal: &Signal, view: &View, size: [usize; 2]) {
        let node = NodeId::from_raw(1);
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(20) {
            cache.update(s, signal, node, view, size, OUTSIDE);
            if !cache.busy() {
                cache.update(s, signal, node, view, size, OUTSIDE);
                if !cache.busy() {
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("the cache never settled");
    }

    /// A view of 100 columns of 64 samples starting at column `first`.
    fn view_at(signal: &Signal, cache: &mut SonoCache, s: &Settings, first: i64) -> View {
        let extent = cache.extent(s, signal).unwrap();
        let mut view = View::fit(extent, 100.0);
        view.samples_per_px = 64.0;
        view.start = first as f64 * 64.0;
        view
    }

    #[test]
    fn margin_job_leaves_the_visible_columns_alone() {
        let signal = tone(64_000);
        let s = Settings::default();
        let mut cache = SonoCache::default();
        let view = view_at(&signal, &mut cache, &s, 300);
        let node = NodeId::from_raw(1);
        // Visible columns first.
        let start = Instant::now();
        while !cache.job.as_ref().is_some_and(|r| r.prefetch) {
            cache.update(&s, &signal, node, &view, [100, 64], OUTSIDE);
            assert!(start.elapsed() < Duration::from_secs(20));
            std::thread::sleep(Duration::from_millis(1));
        }
        let shown: Vec<Arc<[f32]>> = (300..400)
            .map(|c| match cache.columns.get(&c) {
                Some(Column::Data {
                    reduced: Some(r), ..
                }) => Arc::clone(r),
                other => panic!("column {c}: {other:?}"),
            })
            .collect();
        let margin = cache.job.as_ref().expect("margin job");
        assert!(margin.prefetch);
        assert!(
            margin.columns.iter().all(|c| !(300..400).contains(c)),
            "the margin job must not touch the visible columns"
        );
        assert_eq!(margin.columns.len(), 100);
        settle(&mut cache, &s, &signal, &view, [100, 64]);
        for (c, old) in (300..400).zip(&shown) {
            let Some(Column::Data {
                reduced: Some(r), ..
            }) = cache.columns.get(&c)
            else {
                panic!("column {c} lost");
            };
            assert!(Arc::ptr_eq(r, old), "column {c} was reduced again");
        }
        // Panning by 57 columns, 7 past the margin, asks for those 7 only.
        let far = view_at(&signal, &mut cache, &s, 357);
        cache.update(&s, &signal, node, &far, [100, 64], OUTSIDE);
        let job = cache.job.as_ref().expect("visible job");
        assert!(!job.prefetch);
        assert_eq!(job.columns, (450..457).collect::<Vec<_>>());
    }

    #[test]
    fn visible_work_cancels_a_margin_job() {
        let signal = tone(64_000);
        let s = Settings::default();
        let mut cache = SonoCache::default();
        let view = view_at(&signal, &mut cache, &s, 300);
        let node = NodeId::from_raw(1);
        let start = Instant::now();
        while !cache.job.as_ref().is_some_and(|r| r.prefetch) {
            cache.update(&s, &signal, node, &view, [100, 64], OUTSIDE);
            assert!(start.elapsed() < Duration::from_secs(20), "no margin job");
            std::thread::sleep(Duration::from_millis(1));
        }
        let far = view_at(&signal, &mut cache, &s, 800);
        cache.update(&s, &signal, node, &far, [100, 64], OUTSIDE);
        let job = cache.job.as_ref().expect("visible job");
        assert!(!job.prefetch, "the view waits for no margin");
        assert_eq!(job.columns, (800..900).collect::<Vec<_>>());
    }

    #[test]
    fn outside_columns_need_no_job_and_eviction_keeps_the_view() {
        let signal = tone(64_000);
        let s = Settings::default();
        let mut cache = SonoCache {
            // Room for 30 full spectra only.
            budget: 30 * 4 * 513,
            ..SonoCache::default()
        };
        let view = view_at(&signal, &mut cache, &s, -150);
        settle(&mut cache, &s, &signal, &view, [100, 64]);
        assert!(matches!(cache.columns.get(&-150), Some(Column::Outside)));
        let bins = cache.engine.as_ref().unwrap().1.bins();
        assert_eq!(bins, 513);
        let full: Vec<i64> = cache
            .columns
            .iter()
            .filter(|(_, c)| matches!(c, Column::Data { full: Some(_), .. }))
            .map(|(c, _)| *c)
            .collect();
        assert!(full.len() <= 30, "{} full spectra", full.len());
        // Every visible column is still drawn.
        for c in -150..-50 {
            assert!(
                matches!(
                    cache.columns.get(&c),
                    Some(
                        Column::Outside
                            | Column::Data {
                                reduced: Some(_),
                                ..
                            }
                    )
                ),
                "column {c}"
            );
        }
    }

    #[test]
    fn a_new_fft_size_starts_a_new_cache() {
        let signal = tone(32_000);
        let s = Settings::default();
        let mut cache = SonoCache::default();
        let view = view_at(&signal, &mut cache, &s, 0);
        settle(&mut cache, &s, &signal, &view, [100, 64]);
        assert!(!cache.columns.is_empty());
        let generation = cache.generation;
        let bigger = Settings {
            fft_size: s.fft_size * 2,
            ..s
        };
        cache.engine(&bigger, &signal);
        cache.request_visible(&bigger, &signal, NodeId::from_raw(1), &view, [100, 64]);
        assert_ne!(cache.generation, generation);
        assert!(
            cache.columns.values().all(|c| matches!(c, Column::Outside)),
            "no spectrum of the old size survives"
        );
    }

    #[test]
    fn a_tone_lights_its_row_once_the_job_is_done() {
        let fs = 8000.0;
        let s: Vec<f32> = (0..16_000)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 8000.0).sin())
            .collect();
        let signal = Signal::real(s, fs).unwrap();
        assert_eq!(signal.kind(), SampleKind::Real);
        let settings = Settings {
            colormap: Colormap::Gray,
            ..Settings::default()
        };
        let mut cache = SonoCache::default();
        let extent = cache.extent(&settings, &signal).unwrap();
        let view = View::fit(extent, 100.0);
        let node = NodeId::from_raw(1);
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(20) {
            cache.update(&settings, &signal, node, &view, [100, 64], OUTSIDE);
            if !cache.busy() && cache.columns.len() >= 100 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        cache.update(&settings, &signal, node, &view, [100, 64], OUTSIDE);
        let img = cache.image().unwrap();
        assert_eq!((img.width, img.height), (100, 64));
        let y = view.freq_to_y(1000.0, 64.0) as usize;
        let lut = Colormap::Gray.lut();
        let row = |y: usize| img.pixels[y * 100 + 50];
        let brightest = (0..64).max_by_key(|y| row(*y).r).unwrap();
        assert!(brightest.abs_diff(y) <= 1, "{brightest} vs {y}");
        assert_eq!(row(brightest), lut[255]);
        assert!(!cache.update(&settings, &signal, node, &view, [100, 64], OUTSIDE));
    }
}
