//! SA's own file loader, the `SA Open file` window (6.1.1.3 to 6.2.6.7,
//! `docs/SA_UI_SPEC.md` section 9): a folder browser with SA's info panel, the
//! `Use options` raw-data settings that replaced OpenAsData, the whole-file
//! preview with its prescaler and the `SA - select channel` question for
//! stereo files. This is the state and its rules, as in `sa-app`'s loader;
//! `view.rs` draws it.

pub(crate) mod glyphs;
pub(crate) mod view;

use std::path::{Path, PathBuf};
use std::time::Instant;

use sn_core::job::{JobError, JobHandle};
use sn_dsp::overview::MinMax;
use sn_io::{LoadOptions, RawFormat};
pub(crate) use sn_ui_core::loader::{
    Detected, PRESCALER_MAX_DB, PREVIEW_DEBOUNCE, Preview, Selected,
};
use sn_ui_core::loader::{FieldsIn, OptionFields, OptionsIn, spawn_preview};
use sn_ui_core::loader_files::{self, Entry, ExtFilter};
use sn_ui_core::loader_info::{self, Bits, Chan, DEFAULT_RAW, StereoChoice};
use sn_ui_core::{format, t};

use crate::win95::combo::ComboState;
use crate::win95::edit::{EditState, SpinState};
use crate::win95::geom::Rect;
use crate::win95::scrollbar::ScrollState;

/// Entries kept in the path combo's history.
const HISTORY: usize = 20;

/// The option fields (`Data start`, `Channels`, `Bits`, `Freq rate`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Options {
    pub(crate) data_start: EditState,
    pub(crate) chan: Chan,
    pub(crate) bits: Bits,
    pub(crate) rate: EditState,
    /// Interleaved channels of headerless data (SigNexora addition).
    pub(crate) total: SpinState,
}

impl Options {
    fn new(f: &OptionFields) -> Self {
        Self {
            data_start: EditState::new(&f.data_start),
            chan: f.chan,
            bits: f.bits,
            rate: EditState::new(&f.rate),
            total: SpinState::new(i64::from(f.total.max(1)), 1, 64, 1),
        }
    }

    /// The channel count typed for headerless data.
    pub(crate) fn total(&self) -> u16 {
        u16::try_from(self.total.current().clamp(1, 64)).unwrap_or(1)
    }
}

/// What the loader remembers while the program runs.
#[derive(Debug, Clone, Default)]
pub(crate) struct Memory {
    pub(crate) dir: Option<PathBuf>,
    pub(crate) history: Vec<PathBuf>,
    pub(crate) details: bool,
    pub(crate) filter: ExtFilter,
    /// `Auto preview`; `None` until first set (on by default).
    pub(crate) auto_preview: Option<bool>,
}

/// What the user decided.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum LoaderAction {
    /// Keep the window open.
    #[default]
    None,
    /// Close without loading.
    Cancel,
    /// Load the file with these options.
    Load(PathBuf, LoadOptions),
}

/// The widgets' states.
#[derive(Debug, Clone)]
pub(crate) struct Widgets {
    pub(crate) path: EditState,
    pub(crate) search: EditState,
    pub(crate) chan: ComboState,
    pub(crate) bits: ComboState,
    pub(crate) filter: ComboState,
    /// The list: columns (list view) or rows (details view).
    pub(crate) list: ScrollState,
    pub(crate) prescaler: ScrollState,
}

/// The `SA - select channel` question.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Prompt {
    pub(crate) choice: StereoChoice,
    pub(crate) rect: Rect,
    pub(crate) outline: Option<Rect>,
    pub(crate) placed: bool,
}

/// The window's state.
// The flags are independent view and option settings.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub(crate) struct Loader {
    pub(crate) dir: PathBuf,
    pub(crate) entries: Vec<Entry>,
    pub(crate) shown: Vec<usize>,
    pub(crate) list_error: Option<String>,
    pub(crate) selected: Option<usize>,
    select_path: Option<PathBuf>,
    pub(crate) back: Vec<PathBuf>,
    pub(crate) history: Vec<PathBuf>,
    pub(crate) details: bool,
    pub(crate) auto_preview: bool,
    /// Scroll the selected entry into view on the next frame.
    pub(crate) reveal: bool,
    pub(crate) filter: ExtFilter,
    /// The filter the user chose, remembered for the next opening (a dropped
    /// file the filter hides widens `filter` for this opening only).
    remembered_filter: ExtFilter,
    started: bool,
    pub(crate) file: Option<Selected>,
    raw: RawFormat,
    pub(crate) use_options: bool,
    pub(crate) opts: Options,
    pub(crate) prescale_db: i32,
    pub(crate) prompt: Option<Prompt>,
    /// An error to show in SA's error box.
    pub(crate) error: Option<String>,
    pub(crate) preview: Option<Preview>,
    pub(crate) preview_for: Option<(PathBuf, LoadOptions)>,
    preview_job: Option<((PathBuf, LoadOptions), JobHandle<Preview>)>,
    pub(crate) preview_error: Option<String>,
    preview_wanted: Option<((PathBuf, LoadOptions), Instant)>,
    pub(crate) w: Widgets,
    /// Rows in one column of the list view (from the last layout).
    pub(crate) rows_per_column: usize,
    /// Where the dialog is (window coordinates).
    pub(crate) rect: Rect,
    pub(crate) outline: Option<Rect>,
    pub(crate) placed: bool,
    /// The path list under the path combo is open.
    pub(crate) path_list: bool,
}

impl Drop for Loader {
    fn drop(&mut self) {
        // A closed window must not keep reading the file.
        if let Some((_, h)) = self.preview_job.take() {
            h.cancel();
        }
    }
}

fn start_dir() -> PathBuf {
    std::env::current_dir()
        .ok()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn combo(items: &[String], selected: Option<usize>) -> ComboState {
    let refs: Vec<&str> = items.iter().map(String::as_str).collect();
    ComboState::new(&refs, selected)
}

impl Loader {
    /// `File > Open file...`: the window on the folder used last.
    pub(crate) fn browse(raw: Option<RawFormat>, mem: &Memory) -> Self {
        Self::at(None, raw, mem)
    }

    /// The window on `path`'s folder with `path` selected (a dropped file or
    /// one named on the command line).
    pub(crate) fn new(path: PathBuf, raw: Option<RawFormat>, mem: &Memory) -> Self {
        Self::at(Some(path), raw, mem)
    }

    fn at(path: Option<PathBuf>, raw: Option<RawFormat>, mem: &Memory) -> Self {
        let raw = raw.unwrap_or(DEFAULT_RAW);
        let path = path.map(|p| std::path::absolute(&p).unwrap_or(p));
        let missing = path.as_ref().filter(|p| !p.exists()).cloned();
        let dir = path
            .as_ref()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
            .filter(|d| d.is_dir());
        let explicit = dir.is_some();
        let filters: Vec<String> = ExtFilter::ALL.iter().map(|f| f.label()).collect();
        let bits: Vec<String> = Bits::ALL.iter().map(|b| b.label().to_owned()).collect();
        let opts = Options::new(&OptionFields::raw(&raw));
        let mut l = Self {
            dir: dir.unwrap_or_else(start_dir),
            entries: Vec::new(),
            shown: Vec::new(),
            list_error: None,
            selected: None,
            select_path: path,
            back: Vec::new(),
            history: mem.history.clone(),
            details: mem.details,
            auto_preview: mem.auto_preview.unwrap_or(true),
            reveal: true,
            filter: mem.filter,
            remembered_filter: mem.filter,
            started: explicit,
            file: None,
            raw,
            use_options: false,
            w: Widgets {
                path: EditState::new(""),
                search: EditState::new(""),
                chan: combo(&[], None),
                bits: combo(&bits, None),
                filter: combo(
                    &filters,
                    ExtFilter::ALL.iter().position(|f| *f == mem.filter),
                ),
                list: ScrollState::new(0, 0, 1, 0),
                prescaler: ScrollState::new(0, PRESCALER_MAX_DB, 1, PRESCALER_MAX_DB),
            },
            opts,
            prescale_db: 0,
            prompt: None,
            error: None,
            preview: None,
            preview_for: None,
            preview_job: None,
            preview_error: None,
            preview_wanted: None,
            rows_per_column: 11,
            rect: Rect::default(),
            outline: None,
            placed: false,
            path_list: false,
        };
        if !explicit && let Some(d) = mem.dir.clone().filter(|d| d.is_dir()) {
            l.dir = d;
        }
        l.reload();
        if l.selected.is_some_and(|i| !l.shown.contains(&i)) {
            // A dropped file the filter would hide: show every file, this
            // time only.
            l.set_filter(ExtFilter::All);
            l.remembered_filter = mem.filter;
        }
        l.remember_dir();
        l.started = true;
        if let Some(p) = missing {
            l.error = Some(t!("loader-path-not-found", path = p.display().to_string()));
        }
        l.sync_widgets();
        l
    }

    /// What to remember for the next time the window opens.
    pub(crate) fn memory(&self) -> Memory {
        Memory {
            dir: Some(self.dir.clone()),
            history: self.history.clone(),
            details: self.details,
            filter: self.remembered_filter,
            auto_preview: Some(self.auto_preview),
        }
    }

    /// Bring the widgets in line with the state (after a new file or folder).
    pub(crate) fn sync_widgets(&mut self) {
        let chans: Vec<String> = Chan::list(self.channel_total())
            .iter()
            .map(|c| c.label())
            .collect();
        let sel = Chan::list(self.channel_total())
            .iter()
            .position(|c| *c == self.opts.chan);
        self.w.chan = combo(&chans, sel);
        self.w.bits.selected = Bits::ALL.iter().position(|b| *b == self.opts.bits);
        self.w.path.set_text(&loader_files::folder_name(&self.dir));
        self.w.prescaler.pos = PRESCALER_MAX_DB - self.prescale_db;
    }

    fn remember_dir(&mut self) {
        self.history.retain(|d| *d != self.dir);
        self.history.insert(0, self.dir.clone());
        self.history.truncate(HISTORY);
    }

    /// Show only the entries `f` lets through.
    pub(crate) fn set_filter(&mut self, f: ExtFilter) {
        self.filter = f;
        self.remembered_filter = f;
        self.shown = loader_files::visible(&self.entries, f);
        self.w.filter.selected = ExtFilter::ALL.iter().position(|x| *x == f);
    }

    /// Re-read the folder (SA's `Update`), keeping the selection when it is
    /// still there. A selected file that changed on disk (size or modification
    /// time) is probed again.
    pub(crate) fn reload(&mut self) {
        let keep = self
            .select_path
            .take()
            .or_else(|| self.selected_entry().map(|e| e.path.clone()));
        match loader_files::list_dir(&self.dir) {
            Ok(v) => {
                self.entries = v;
                self.list_error = None;
            }
            Err(e) => {
                self.entries = Vec::new();
                self.list_error = Some(e.to_string());
            }
        }
        self.shown = loader_files::visible(&self.entries, self.filter);
        let found = keep.and_then(|p| self.entries.iter().position(|e| e.path == p));
        let same_file = found
            .and_then(|i| self.entries.get(i))
            .zip(self.file.as_ref())
            .is_some_and(|(e, f)| e.path == f.path && e.size == f.size && e.modified == f.modified);
        self.selected = None;
        if let Some(i) = found
            && same_file
        {
            self.selected = Some(i);
        } else if let Some(i) = found {
            self.select(i);
        } else {
            self.file = None;
            self.use_options = false;
        }
        self.reveal = true;
        if self.started {
            self.w.path.set_text(&loader_files::folder_name(&self.dir));
        }
    }

    pub(crate) fn selected_entry(&self) -> Option<&Entry> {
        self.selected.and_then(|i| self.entries.get(i))
    }

    /// Open folder `dir` (back remembers where we were).
    pub(crate) fn go(&mut self, dir: PathBuf) {
        if dir == self.dir {
            return;
        }
        self.back.push(std::mem::replace(&mut self.dir, dir));
        self.w.search.set_text("");
        self.w.list.pos = 0;
        self.reload();
        self.remember_dir();
    }

    pub(crate) fn go_back(&mut self) {
        if let Some(d) = self.back.pop() {
            self.dir = d;
            self.w.list.pos = 0;
            self.reload();
            self.remember_dir();
        }
    }

    pub(crate) fn go_up(&mut self) {
        if let Some(parent) = self.dir.parent().map(Path::to_path_buf) {
            let child = self.dir.clone();
            self.go(parent);
            self.select_path_now(&child);
        }
    }

    fn select_path_now(&mut self, path: &Path) {
        if let Some(i) = self.entries.iter().position(|e| e.path == path) {
            self.select(i);
            self.reveal = true;
        }
    }

    /// A path typed or pasted into the path combo: a folder opens, a file
    /// opens its folder and is selected.
    pub(crate) fn go_text(&mut self, text: &str) {
        let typed = PathBuf::from(text.trim());
        let path = if typed.is_relative() {
            self.dir.join(&typed)
        } else {
            typed
        };
        if path.is_dir() {
            self.go(path);
        } else if path.is_file()
            && let Some(parent) = path.parent().map(Path::to_path_buf)
        {
            self.go(parent);
            self.select_path_now(&path);
        } else {
            self.error = Some(t!("loader-path-not-found", path = text.trim()));
        }
        self.w.path.set_text(&loader_files::folder_name(&self.dir));
    }

    /// Select entry `i`: a file is probed and its options reset (SA resets `Use
    /// options` and the prescaler for every new file).
    pub(crate) fn select(&mut self, i: usize) {
        if self.selected == Some(i) {
            return;
        }
        self.selected = Some(i);
        self.prescale_db = 0;
        self.use_options = false;
        let Some(e) = self.entries.get(i) else {
            return;
        };
        if e.is_folder() {
            self.file = None;
            self.sync_widgets();
            return;
        }
        let file = Selected::probe(e.path.clone(), e.size, e.modified);
        self.opts = Options::new(&file.initial_fields(&self.raw));
        // Headerless data can only be read through the options.
        self.use_options = !matches!(file.detected, Detected::Known(_));
        self.file = Some(file);
        self.sync_widgets();
    }

    /// Activate entry `i` (double click or Enter): open a folder, load a file.
    pub(crate) fn activate(&mut self, i: usize) -> LoaderAction {
        let Some(e) = self.entries.get(i) else {
            return LoaderAction::None;
        };
        if e.is_folder() {
            let p = e.path.clone();
            self.go(p);
            return LoaderAction::None;
        }
        self.select(i);
        self.ok()
    }

    /// Move the selection by `step` entries of the shown list.
    pub(crate) fn step(&mut self, step: isize) {
        let pos = self
            .selected
            .and_then(|s| self.shown.iter().position(|&i| i == s));
        let Some(last) = self.shown.len().checked_sub(1) else {
            return;
        };
        let next = match pos {
            Some(p) => p.saturating_add_signed(step).min(last),
            None => 0,
        };
        if let Some(&i) = self.shown.get(next) {
            self.select(i);
            self.reveal = true;
        }
    }

    /// The loader's state that decides the load options.
    fn options_in(&self) -> OptionsIn<'_> {
        OptionsIn {
            file: self.file.as_ref(),
            use_options: self.use_options,
            fields: FieldsIn {
                data_start: &self.opts.data_start.text,
                chan: self.opts.chan,
                bits: self.opts.bits,
                rate: &self.opts.rate.text,
                total: self.opts.total(),
            },
            prescale_db: self.prescale_db as f32,
        }
    }

    /// The channel count the options refer to.
    pub(crate) fn channel_total(&self) -> u16 {
        self.options_in().channel_total()
    }

    /// The selected file has no header SigNexora knows.
    pub(crate) fn headerless(&self) -> bool {
        self.file.as_ref().is_some_and(Selected::headerless)
    }

    /// `Use options` can be switched: the file has an uncompressed layout the
    /// options can describe (a compressed file read as raw PCM is noise).
    pub(crate) fn options_allowed(&self) -> bool {
        self.file.as_ref().is_some_and(Selected::options_allowed)
    }

    /// `Use options` was ticked or cleared.
    pub(crate) fn set_use_options(&mut self, on: bool) {
        if self.options_allowed() {
            self.use_options = on;
            self.set_prescale(0);
        }
    }

    /// A new channel or format was chosen: SA resets the prescaler.
    pub(crate) fn set_chan(&mut self, i: usize) {
        if let Some(c) = Chan::list(self.channel_total()).get(i).copied() {
            self.opts.chan = c;
            self.set_prescale(0);
        }
    }

    /// A new sample format was chosen.
    pub(crate) fn set_bits(&mut self, i: usize) {
        if let Some(b) = Bits::ALL.get(i).copied() {
            self.opts.bits = b;
            self.set_prescale(0);
        }
    }

    /// The channels of headerless data changed.
    pub(crate) fn total_changed(&mut self) {
        self.set_prescale(0);
        if !Chan::list(self.opts.total()).contains(&self.opts.chan) {
            self.opts.chan = Chan::All;
        }
        self.sync_widgets();
    }

    /// Set the prescaler (0 to 60 dB).
    pub(crate) fn set_prescale(&mut self, db: i32) {
        self.prescale_db = db.clamp(0, PRESCALER_MAX_DB);
        self.w.prescaler.pos = PRESCALER_MAX_DB - self.prescale_db;
    }

    /// The prescaler as load scaling: gain, then clipping at full scale.
    #[cfg(test)]
    pub(crate) fn scaling(&self) -> sn_io::Scaling {
        sn_ui_core::loader::scaling(self.use_options, self.prescale_db as f32)
    }

    /// `Ok`: load the selected file, asking first which channel of a stereo
    /// file.
    pub(crate) fn ok(&mut self) -> LoaderAction {
        let Some(file) = self.file.as_ref() else {
            if let Some(i) = self.selected
                && self.entries.get(i).is_some_and(Entry::is_folder)
            {
                return self.activate(i);
            }
            return LoaderAction::None;
        };
        if let (false, Detected::Failed(e)) = (self.use_options, &file.detected) {
            self.error = Some(t!("loader-cannot-read", error = e.as_str()));
            return LoaderAction::None;
        }
        let stereo = !self.use_options && file.is_stereo();
        if stereo {
            self.prompt = Some(Prompt {
                choice: StereoChoice::default(),
                rect: Rect::default(),
                outline: None,
                placed: false,
            });
            return LoaderAction::None;
        }
        self.load(StereoChoice::default())
    }

    /// Load with the stereo answer `stereo`.
    pub(crate) fn load(&mut self, stereo: StereoChoice) -> LoaderAction {
        match self.options_in().load(stereo) {
            Ok((path, o)) => LoaderAction::Load(path, o),
            Err(why) => {
                self.error = Some(format!("{}\n{why}", t!("loader-check-settings")));
                LoaderAction::None
            }
        }
    }

    /// The preview to show: with the options, or (Auto preview) as the file
    /// loads.
    pub(crate) fn preview_key(&self) -> Option<(PathBuf, LoadOptions)> {
        self.options_in().preview_key(self.auto_preview)
    }

    /// Collect a finished preview and start the one now wanted. Returns
    /// whether a preview is still being built or waited for.
    pub(crate) fn poll_preview(&mut self) -> bool {
        let done = self
            .preview_job
            .as_ref()
            .is_some_and(|(_, h)| h.is_finished());
        if done && let Some((key, h)) = self.preview_job.take() {
            match h.join() {
                Ok(p) => {
                    self.preview = Some(p);
                    self.preview_for = Some(key);
                    self.preview_error = None;
                }
                Err(JobError::Cancelled) => {}
                Err(e) => {
                    self.preview = None;
                    self.preview_for = Some(key);
                    self.preview_error = Some(e.to_string());
                }
            }
        }
        let Some(key) = self.preview_key() else {
            self.preview_wanted = None;
            if let Some((_, h)) = self.preview_job.take() {
                h.cancel();
            }
            return false;
        };
        let stale = self.preview_for.as_ref() != Some(&key)
            && self.preview_job.as_ref().is_none_or(|(k, _)| *k != key);
        if !stale {
            self.preview_wanted = None;
            return self.preview_job.is_some();
        }
        let now = Instant::now();
        let since = match &self.preview_wanted {
            Some((k, t)) if *k == key => *t,
            _ => {
                self.preview_wanted = Some((key.clone(), now));
                now
            }
        };
        let new_file = self.preview_for.as_ref().is_none_or(|(p, _)| *p != key.0);
        if new_file || now.duration_since(since) >= PREVIEW_DEBOUNCE {
            self.preview_wanted = None;
            if let Some((_, h)) = self.preview_job.take() {
                h.cancel();
            }
            self.preview_job = Some((key.clone(), spawn_preview(key)));
        }
        true
    }

    /// A preview is being built, or one is wanted that is not built yet.
    pub(crate) fn preview_pending(&self) -> bool {
        self.preview_job.is_some()
            || self.preview_wanted.is_some()
            || self
                .preview_key()
                .is_some_and(|k| self.preview_for.as_ref() != Some(&k))
    }

    /// The preview columns for the selected file, if they are the current ones.
    pub(crate) fn current_preview(&self) -> Option<&[MinMax]> {
        let key = self.preview_key()?;
        if self.preview_for.as_ref() != Some(&key) || self.preview_error.is_some() {
            return None;
        }
        self.preview.as_ref().map(|p| p.columns.as_slice())
    }

    /// The text replacing the preview: its error, or "Building preview...".
    pub(crate) fn preview_message(&self) -> Option<String> {
        let key = self.preview_key();
        let building = key.is_some() && self.current_preview().is_none();
        match (&self.preview_error, building) {
            (Some(e), _) if key.is_some() && self.preview_for == key => Some(e.clone()),
            (_, true) => Some(t!("loader-preview-building")),
            _ => None,
        }
    }

    /// The info panel's lines and whether the file is SA's standard one (16-bit
    /// mono PCM, white) or not (yellow). `None` for a folder.
    pub(crate) fn info_lines(&self) -> Option<(Vec<String>, bool)> {
        if self.selected_entry().is_some_and(Entry::is_folder) {
            return None;
        }
        let Some(file) = &self.file else {
            return Some((Vec::new(), true));
        };
        Some(match (&file.wav, &file.detected) {
            (Some(h), _) => (
                vec![
                    t!("loader-info-tag", value = h.tag.to_string()),
                    t!("loader-info-channels", value = h.channels.to_string()),
                    t!("loader-info-rate", value = h.rate.to_string()),
                    t!("loader-info-bits", value = h.bits.to_string()),
                    t!("loader-info-size", value = h.data_size.to_string()),
                ],
                loader_info::is_standard(h),
            ),
            (None, Detected::Known(i)) => {
                let mut v = vec![
                    t!("loader-info-channels", value = i.channels.to_string()),
                    t!(
                        "loader-info-rate",
                        value = format::dec_trim(i.sample_rate, 2)
                    ),
                ];
                if let Some(f) = i.sample_format {
                    v.push(t!("loader-info-bits", value = (f.bytes() * 8).to_string()));
                }
                v.push(t!(
                    "loader-info-size",
                    value = file.data_bytes().to_string()
                ));
                (v, false)
            }
            (None, Detected::NeedsRaw) => (vec![t!("loader-info-no-header")], false),
            (None, Detected::Failed(_)) => (vec![t!("loader-info-unreadable")], false),
        })
    }

    /// `Ext. Info: <codec>` (6.2.6.4).
    pub(crate) fn ext_info(&self) -> String {
        let Some(file) = &self.file else {
            return String::new();
        };
        let codec = match (&file.wav, &file.detected) {
            (Some(h), _) => loader_info::codec_name(h.tag, h.sub_tag),
            (None, Detected::Known(i)) => i.codec.clone(),
            (None, _) => return String::new(),
        };
        t!("loader-ext-info", codec = codec)
    }

    /// Quick search: select the first shown name containing the typed text.
    pub(crate) fn search_changed(&mut self) {
        let text = self.w.search.text.clone();
        if let Some(i) = loader_files::quick_search(&self.entries, &self.shown, &text) {
            self.select(i);
            self.reveal = true;
        }
    }

    /// The status bar text: the full path of the selection or the folder.
    pub(crate) fn status_text(&self) -> String {
        self.selected_entry().map_or_else(
            || self.dir.display().to_string(),
            |e| e.path.display().to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sn_io::{ChannelSelection, SampleFormat, Scaling};
    use sn_ui_core::loader::PREVIEW_COLUMNS;
    use std::time::Duration;

    fn mem() -> Memory {
        Memory::default()
    }

    #[test]
    fn prescaler_is_gain_then_full_scale_clipping_and_only_with_options() {
        let mut l = Loader::browse(None, &mem());
        l.set_prescale(20);
        assert_eq!(l.scaling(), Scaling::default(), "options off");
        l.use_options = true;
        let s = l.scaling();
        assert!((s.gain - 1.0).abs() < 1e-6);
        assert!((s.clip.unwrap() - 0.1).abs() < 1e-6);
        assert_eq!(l.w.prescaler.pos, PRESCALER_MAX_DB - 20);
        l.set_prescale(99);
        assert_eq!(l.prescale_db, PRESCALER_MAX_DB);
    }

    #[test]
    fn options_build_raw_formats_and_reject_bad_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.bin");
        std::fs::write(&path, vec![0_u8; 8192]).unwrap();
        let mut l = Loader::new(path.clone(), None, &mem());
        assert!(l.headerless() && l.use_options, "headerless needs options");
        l.opts.rate.set_text("48000");
        l.set_bits(Bits::ALL.iter().position(|b| *b == Bits::F32).unwrap());
        match l.ok() {
            LoaderAction::Load(p, o) => {
                assert_eq!(p, path);
                let raw = o.raw.unwrap();
                assert_eq!(raw.format, SampleFormat::F32);
                assert!((raw.sample_rate - 48000.0).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
        l.opts.rate.set_text("fast");
        assert_eq!(l.ok(), LoaderAction::None);
        assert!(l.error.as_deref().unwrap().contains("fast"));
        l.opts.total.set(4);
        l.total_changed();
        assert_eq!(l.channel_total(), 4);
        assert_eq!(
            l.opts.chan.selection(4),
            ChannelSelection::IQ { i: 0, q: 1 },
            "All - I/Q"
        );
    }

    #[test]
    fn the_preview_is_built_in_the_background() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.wav");
        let s: Vec<f32> = (0..20_000).map(|i| (i as f32 * 0.2).sin()).collect();
        let signal = sn_core::Signal::real(s, 8000.0).unwrap();
        sn_io::write_wav(&path, &signal, sn_io::WavFormat::Pcm16).unwrap();
        let mut l = Loader::new(path, None, &mem());
        assert!(l.preview_pending());
        assert_eq!(l.preview_message().as_deref(), Some("Building preview..."));
        let start = Instant::now();
        while l.poll_preview() && start.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(5));
        }
        let cols = l.current_preview().unwrap();
        assert_eq!(cols.len(), PREVIEW_COLUMNS);
        assert!(cols.iter().all(|c| !c.is_empty()), "no blank column");
        assert!(cols.iter().any(|c| c.max > 0.9), "a full-scale tone");
        assert!(!l.preview_pending());
        assert_eq!(l.preview_message(), None);
    }

    #[test]
    fn short_files_fill_the_whole_preview() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.wav");
        // Just over the 4 KB preview minimum: 2100 frames give two frames
        // per column (the old block mapping left 60 columns blank).
        let s: Vec<f32> = (0..2100).map(|i| (i as f32 * 0.3).sin()).collect();
        let signal = sn_core::Signal::real(s, 8000.0).unwrap();
        sn_io::write_wav(&path, &signal, sn_io::WavFormat::Pcm16).unwrap();
        let mut l = Loader::new(path, None, &mem());
        let start = Instant::now();
        while l.poll_preview() && start.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(5));
        }
        let cols = l.current_preview().unwrap();
        assert!(cols.iter().all(|c| !c.is_empty()), "no blank column");
    }

    /// Silent MPEG-1 layer III frames (128 kbit/s, 44.1 kHz).
    fn mp3_bytes() -> Vec<u8> {
        let mut v = Vec::new();
        for _ in 0..40 {
            let mut frame = vec![0_u8; 417];
            frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
            v.extend(frame);
        }
        v
    }

    #[test]
    fn use_options_stays_off_for_files_without_a_raw_layout() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.mp3");
        std::fs::write(&path, mp3_bytes()).unwrap();
        let mut l = Loader::new(path, None, &mem());
        if let Some(Selected {
            detected: Detected::Known(info),
            ..
        }) = &l.file
        {
            assert!(info.layout().is_none(), "compressed");
        }
        assert!(!l.options_allowed());
        l.set_use_options(true);
        assert!(
            !l.use_options || l.headerless(),
            "compressed data is not raw PCM"
        );
    }

    #[test]
    fn a_wav_allows_options_and_reports_its_data_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.wav");
        let signal = sn_core::Signal::real(vec![0.0; 3000], 8000.0).unwrap();
        sn_io::write_wav(&path, &signal, sn_io::WavFormat::Pcm16).unwrap();
        let mut l = Loader::new(path, None, &mem());
        assert!(l.options_allowed());
        l.set_use_options(true);
        assert!(l.use_options);
        let (lines, _) = l.info_lines().unwrap();
        assert!(lines.iter().any(|s| s.contains("6000")), "{lines:?}");
    }

    #[test]
    fn update_reprobes_a_file_that_changed_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.wav");
        let mono = sn_core::Signal::real(vec![0.0; 3000], 8000.0).unwrap();
        sn_io::write_wav(&path, &mono, sn_io::WavFormat::Pcm16).unwrap();
        let mut l = Loader::new(path.clone(), None, &mem());
        assert_eq!(l.channel_total(), 1);
        let stereo = sn_core::Signal::real(vec![0.0; 9000], 8000.0).unwrap();
        sn_io::write_wav(&path, &stereo, sn_io::WavFormat::Pcm16).unwrap();
        l.reload();
        let (lines, _) = l.info_lines().unwrap();
        assert!(lines.iter().any(|s| s.contains("18000")), "{lines:?}");
    }

    #[test]
    fn a_hidden_dropped_file_does_not_change_the_remembered_filter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.xyz");
        std::fs::write(&path, vec![0_u8; 100]).unwrap();
        let m = Memory {
            filter: ExtFilter::Wav,
            ..mem()
        };
        let mut l = Loader::new(path, None, &m);
        assert_eq!(l.filter, ExtFilter::All, "the dropped file is shown");
        assert_eq!(l.memory().filter, ExtFilter::Wav, "this opening only");
        l.set_filter(ExtFilter::All);
        assert_eq!(l.memory().filter, ExtFilter::All, "the user's choice");
    }

    #[test]
    fn a_missing_path_is_reported_and_folders_open_on_ok() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let l = Loader::new(dir.path().join("nope.wav"), None, &mem());
        assert!(l.error.is_some());
        let mut l = Loader::new(dir.path().join("sub"), None, &mem());
        assert!(l.selected_entry().is_some_and(Entry::is_folder));
        assert_eq!(l.info_lines(), None);
        assert_eq!(l.ok(), LoaderAction::None);
        assert!(l.dir.ends_with("sub"), "Ok opens the folder");
        l.go_up();
        assert!(l.selected_entry().is_some_and(|e| e.path.ends_with("sub")));
        l.go_text("sub");
        assert!(l.dir.ends_with("sub"), "relative to the folder shown");
        l.go_back();
        assert!(!l.dir.ends_with("sub"));
        assert!(l.memory().history.len() >= 2);
    }
}
