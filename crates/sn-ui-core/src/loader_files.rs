//! The folder listing of SA's `SA Open file` window: entries, the extension filter,
//! the quick search and SA's date and time columns.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::t;

/// Extensions SigNexora can open (the "All supported" filter).
pub const SUPPORTED: [&str; 16] = [
    "wav",
    "rf64",
    "bw64",
    "sigmf-meta",
    "sigmf-data",
    "mp3",
    "flac",
    "ogg",
    "raw",
    "bin",
    "iq",
    "dat",
    "cfile",
    "cu8",
    "cs16",
    "cf32",
];

/// What a listed entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A folder.
    Folder,
    /// A link to a folder (SA: `F-Link`).
    Link,
    /// A file.
    File,
}

/// One line of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// File or folder name.
    pub name: String,
    /// Full path.
    pub path: PathBuf,
    /// What the entry is.
    pub kind: EntryKind,
    /// Size in bytes (files only).
    pub size: u64,
    /// Last modification time.
    pub modified: Option<SystemTime>,
}

impl Entry {
    /// The file name extension, lower case (empty for folders).
    pub fn ext(&self) -> String {
        if self.kind != EntryKind::File {
            return String::new();
        }
        let lower = self.name.to_lowercase();
        // SigMF pairs have a two-part extension.
        for two in ["sigmf-meta", "sigmf-data"] {
            if lower.ends_with(&format!(".{two}")) {
                return two.to_owned();
            }
        }
        Path::new(&lower)
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// SA's `Type/Ext` column: `Folder`, `F-Link` or the extension.
    pub fn type_text(&self) -> String {
        match self.kind {
            EntryKind::Folder => t!("loader-type-folder"),
            EntryKind::Link => t!("loader-type-link"),
            EntryKind::File => self.ext(),
        }
    }

    /// The entry is a folder (or the parent folder).
    pub fn is_folder(&self) -> bool {
        self.kind != EntryKind::File
    }
}

/// Read folder `dir`: folders first, then files, each by name ignoring case. Hidden
/// entries (names starting with a dot) are left out.
pub fn list_dir(dir: &Path) -> std::io::Result<Vec<Entry>> {
    let mut out = Vec::new();
    for item in std::fs::read_dir(dir)? {
        let Ok(item) = item else {
            continue;
        };
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = item.path();
        let link = item.file_type().is_ok_and(|t| t.is_symlink());
        // Follows links: a link to a folder lists as a folder link.
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let kind = match (meta.is_dir(), link) {
            (true, true) => EntryKind::Link,
            (true, false) => EntryKind::Folder,
            (false, _) => EntryKind::File,
        };
        out.push(Entry {
            name,
            path,
            kind,
            size: if kind == EntryKind::File {
                meta.len()
            } else {
                0
            },
            modified: meta.modified().ok(),
        });
    }
    sort_entries(&mut out);
    Ok(out)
}

/// Folders first, then files, each by name ignoring case.
fn sort_entries(entries: &mut [Entry]) {
    // One lowercase key per entry, not two per comparison.
    entries.sort_by_cached_key(|e| (!e.is_folder(), e.name.to_lowercase(), e.name.clone()));
}

/// SA's extension filter list below the file list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtFilter {
    /// `wav & mp3` (SA's default).
    #[default]
    WavMp3,
    /// `wav` files.
    Wav,
    /// `mp3` files.
    Mp3,
    /// Every format SigNexora opens (SigNexora addition).
    Supported,
    /// Every file (SigNexora addition; SA opened any file as data through the options).
    All,
}

impl ExtFilter {
    /// Every filter, in list order.
    pub const ALL: [ExtFilter; 5] = [
        Self::WavMp3,
        Self::Wav,
        Self::Mp3,
        Self::Supported,
        Self::All,
    ];

    /// The text in the filter list.
    pub fn label(self) -> String {
        match self {
            Self::WavMp3 => t!("loader-filter-wav-mp3"),
            Self::Wav => t!("loader-filter-wav"),
            Self::Mp3 => t!("loader-filter-mp3"),
            Self::Supported => t!("loader-filter-supported"),
            Self::All => t!("loader-filter-all"),
        }
    }

    /// Whether a file with extension `ext` (lower case) is listed.
    pub fn matches(self, ext: &str) -> bool {
        match self {
            Self::WavMp3 => matches!(ext, "wav" | "mp3"),
            Self::Wav => ext == "wav",
            Self::Mp3 => ext == "mp3",
            Self::Supported => SUPPORTED.contains(&ext),
            Self::All => true,
        }
    }
}

/// Indexes of the entries the filter lets through (folders always).
pub fn visible(entries: &[Entry], filter: ExtFilter) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.is_folder() || filter.matches(&e.ext()))
        .map(|(i, _)| i)
        .collect()
}

/// SA's quick search: the first listed entry whose name contains `text`, ignoring
/// case. Empty text finds nothing.
pub fn quick_search(entries: &[Entry], shown: &[usize], text: &str) -> Option<usize> {
    let needle = text.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    shown.iter().copied().find(|&i| {
        entries
            .get(i)
            .is_some_and(|e| e.name.to_lowercase().contains(&needle))
    })
}

/// Year, month and day of a day count since 1970-01-01 (proleptic Gregorian).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

/// SA's `Date` and `Time` columns (`22.01.2010`, `4:44:22`), in UTC. The dialog
/// shows local time like SA and the file managers, with [`local_date_time`].
pub fn date_time(t: SystemTime) -> (String, String) {
    date_time_at(t, 0)
}

/// SA's `Date` and `Time` columns of `t` in local time, as the loader dialog
/// shows them (like SA and the file managers).
pub fn local_date_time(t: SystemTime) -> (String, String) {
    date_time_at(t, local_offset_secs(t))
}

/// Seconds the local time zone is ahead of UTC at `t` (daylight saving time
/// included). 0 when `t` is outside the range the time zone data covers.
pub fn local_offset_secs(t: SystemTime) -> i64 {
    let Ok(at) = jiff::Timestamp::from_second(unix_secs(t)) else {
        return 0;
    };
    i64::from(jiff::tz::TimeZone::system().to_offset(at).seconds())
}

/// Whole seconds from the Unix epoch to `t`, negative before it.
fn unix_secs(t: SystemTime) -> i64 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
        Err(e) => -i64::try_from(e.duration().as_secs()).unwrap_or(i64::MAX),
    }
}

/// SA's `Date` and `Time` columns (`22.01.2010`, `4:44:22`) of `t` in the time
/// zone `offset_secs` seconds ahead of UTC (the local offset for the dialog).
pub fn date_time_at(t: SystemTime, offset_secs: i64) -> (String, String) {
    let secs = unix_secs(t).saturating_add(offset_secs);
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    (
        format!("{d:02}.{m:02}.{y}"),
        format!("{}:{:02}:{:02}", rest / 3600, rest % 3600 / 60, rest % 60),
    )
}

/// The folders from the root down to `dir` (the path combo's list).
pub fn ancestors(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = dir.ancestors().map(Path::to_path_buf).collect();
    out.reverse();
    out
}

/// The name the path combo shows for `dir`: its last part, or the whole path for a
/// root.
pub fn folder_name(dir: &Path) -> String {
    dir.file_name().map_or_else(
        || dir.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry {
            name: name.to_owned(),
            path: PathBuf::from(name),
            kind,
            size: 0,
            modified: None,
        }
    }

    #[test]
    fn folders_come_first_then_names_ignoring_case() {
        let mut v = vec![
            entry("b.wav", EntryKind::File),
            entry("Zeta", EntryKind::Folder),
            entry("A.mp3", EntryKind::File),
            entry("alpha", EntryKind::Link),
        ];
        sort_entries(&mut v);
        let names: Vec<&str> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["alpha", "Zeta", "A.mp3", "b.wav"]);
    }

    #[test]
    fn the_filter_keeps_folders_and_matching_files() {
        let v = vec![
            entry("Signals", EntryKind::Folder),
            entry("x.WAV", EntryKind::File),
            entry("y.mp3", EntryKind::File),
            entry("z.txt", EntryKind::File),
            entry("c.sigmf-meta", EntryKind::File),
        ];
        assert_eq!(visible(&v, ExtFilter::WavMp3), [0, 1, 2]);
        assert_eq!(visible(&v, ExtFilter::Wav), [0, 1]);
        assert_eq!(visible(&v, ExtFilter::Mp3), [0, 2]);
        assert_eq!(visible(&v, ExtFilter::Supported), [0, 1, 2, 4]);
        assert_eq!(visible(&v, ExtFilter::All), [0, 1, 2, 3, 4]);
        assert_eq!(v[4].ext(), "sigmf-meta");
        assert_eq!(v[0].ext(), "");
    }

    #[test]
    fn quick_search_finds_the_first_name_containing_the_text() {
        let v = vec![
            entry("fsk2_850.wav", EntryKind::File),
            entry("UNID 2248.wav", EntryKind::File),
            entry("UNID 9268.wav", EntryKind::File),
        ];
        let shown = [0, 1, 2];
        assert_eq!(quick_search(&v, &shown, "uni"), Some(1));
        assert_eq!(quick_search(&v, &shown, "9268"), Some(2));
        assert_eq!(quick_search(&v, &shown, "nothing"), None);
        assert_eq!(quick_search(&v, &shown, "  "), None);
        // Only listed entries are searched.
        assert_eq!(quick_search(&v, &[0, 2], "uni"), Some(2));
    }

    #[test]
    fn dates_and_times_use_sa_format() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(14_631), (2010, 1, 22));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        let t =
            UNIX_EPOCH + std::time::Duration::from_secs(14_631 * 86_400 + 4 * 3600 + 44 * 60 + 22);
        assert_eq!(
            date_time(t),
            ("22.01.2010".to_owned(), "4:44:22".to_owned())
        );
        assert_eq!(date_time_at(t, 0), date_time(t));
        // UTC-5: the evening before; UTC+3: later the same day.
        assert_eq!(
            date_time_at(t, -5 * 3600),
            ("21.01.2010".to_owned(), "23:44:22".to_owned())
        );
        assert_eq!(
            date_time_at(t, 3 * 3600),
            ("22.01.2010".to_owned(), "7:44:22".to_owned())
        );
        // Local time is UTC moved by the zone's offset at that moment, which is
        // within a day of UTC.
        let off = local_offset_secs(t);
        assert!(off.abs() < 86_400, "{off}");
        assert_eq!(local_date_time(t), date_time_at(t, off));
    }

    #[test]
    fn ancestors_run_from_the_root() {
        let a = ancestors(Path::new("/a/b"));
        assert_eq!(
            a,
            [
                PathBuf::from("/"),
                PathBuf::from("/a"),
                PathBuf::from("/a/b")
            ]
        );
        assert_eq!(folder_name(Path::new("/a/b")), "b");
        assert_eq!(folder_name(Path::new("/")), "/");
    }
}
