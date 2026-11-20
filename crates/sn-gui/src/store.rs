//! The settings remembered across runs, in the platform's configuration folder
//! (`<config>/sa-rs/prefs.json`; eframe kept them in its own storage, GPUI has
//! none).
//!
//! The file holds the shared [`Prefs`] (read leniently by `sn-ui-core`, so a
//! damaged or newer file never stops the app), the frame size for `Size of
//! start SA = As when last closed`, and the signal generator's last spec (read
//! leniently by `sn-ui-core::prefs::generator_from_json`). It is written
//! atomically: a temporary file in the same folder, then a rename.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use sn_ui_core::prefs::Prefs;
use sn_ui_core::theme::ThemeChoice;
use sn_ui_core::winstyle::WinStyle;

/// Key of the shared settings in the file.
const PREFS_KEY: &str = "prefs";
/// Key of the frame size (`[width, height]`).
const FRAME_KEY: &str = "frame";
/// Key of the generator's spec.
const GENERATOR_KEY: &str = "generator";

/// Smallest frame size that is remembered (a smaller one is ignored).
const MIN_FRAME: (f32, f32) = (320.0, 200.0);
/// Largest frame size that is remembered (a damaged or hand-edited file must
/// not open the frame far beyond any screen).
const MAX_FRAME: f32 = 16384.0;

/// What the settings file holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Stored {
    pub(crate) prefs: Prefs,
    /// The frame's size when the app was last closed.
    pub(crate) frame: Option<(f32, f32)>,
    /// The generator's spec, as read.
    pub(crate) generator: Option<Value>,
}

/// The settings file of this user, if the platform has a configuration folder.
pub(crate) fn default_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("sa-rs").join("prefs.json"))
}

impl Stored {
    /// Read `text`; anything missing or unreadable takes its default.
    pub(crate) fn from_json(text: Option<&str>) -> Self {
        let v = text
            .and_then(|t| serde_json::from_str::<Value>(t).ok())
            .unwrap_or(Value::Null);
        let prefs_text = v.get(PREFS_KEY).map(Value::to_string);
        let frame = v
            .get(FRAME_KEY)
            .and_then(|f| serde_json::from_value::<(f32, f32)>(f.clone()).ok())
            .filter(|(w, h)| {
                w.is_finite()
                    && h.is_finite()
                    && (MIN_FRAME.0..=MAX_FRAME).contains(w)
                    && (MIN_FRAME.1..=MAX_FRAME).contains(h)
            });
        let mut prefs = Prefs::from_json(prefs_text.as_deref());
        // Settings from before Options > Windows style: a user who chose a
        // theme other than Windows Standard had it colour the windows, which
        // only the classic styles do. Keep that look rather than switch to
        // XP's (whose windows ignore the theme).
        let stored_look = v
            .get(PREFS_KEY)
            .is_some_and(|p| p.is_object() && p.get("look").is_some());
        if v.get(PREFS_KEY).is_some_and(Value::is_object)
            && !stored_look
            && prefs.theme != ThemeChoice::Classic
        {
            prefs.look.style = WinStyle::Win95;
        }
        Self {
            prefs,
            frame,
            generator: v.get(GENERATOR_KEY).cloned(),
        }
    }

    /// The file's text, or [`SaveError::Encode`] when the settings cannot be
    /// converted to JSON (the old file is then kept rather than replaced with
    /// an empty one).
    pub(crate) fn to_json(&self) -> Result<String, SaveError> {
        let mut m = Map::new();
        let text = self.prefs.to_json().ok_or(SaveError::Encode)?;
        let prefs = serde_json::from_str::<Value>(&text).map_err(|_| SaveError::Encode)?;
        m.insert(PREFS_KEY.into(), prefs);
        if let Some((w, h)) = self.frame {
            m.insert(FRAME_KEY.into(), json!([w, h]));
        }
        if let Some(g) = &self.generator {
            m.insert(GENERATOR_KEY.into(), g.clone());
        }
        serde_json::to_string_pretty(&Value::Object(m)).map_err(|_| SaveError::Encode)
    }

    /// Read the file at `path` (defaults when it does not exist or cannot be read).
    pub(crate) fn load(path: &Path) -> Self {
        Self::from_json(std::fs::read_to_string(path).ok().as_deref())
    }

    /// Write the file at `path` atomically, creating its folder: the data is
    /// synced to disk before the rename, so a crash leaves the old or the new
    /// file, never an empty one.
    pub(crate) fn save(&self, path: &Path) -> Result<(), SaveError> {
        let text = self.to_json()?;
        Self::write_text(path, &text).map_err(SaveError::Io)
    }

    /// Write `text` to `path` atomically (see [`Stored::save`]).
    fn write_text(path: &Path, text: &str) -> std::io::Result<()> {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(dir)?;
        let name = path
            .file_name()
            .map_or_else(|| "prefs.json".into(), |n| n.to_string_lossy().into_owned());
        let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
        let written =
            write_synced(&tmp, text.as_bytes()).and_then(|()| std::fs::rename(&tmp, path));
        if written.is_err() {
            // Best effort: do not leave the temporary file behind.
            let _ = std::fs::remove_file(&tmp);
        }
        written?;
        sync_dir(dir);
        Ok(())
    }
}

/// Why the settings file could not be written.
#[derive(Debug)]
pub(crate) enum SaveError {
    /// The settings could not be converted to JSON (a program error, not
    /// something the user can fix by choosing another folder).
    Encode,
    /// The file or its folder could not be written.
    Io(std::io::Error),
}

/// Write `bytes` to a new file at `path` and flush them to the disk.
fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Flush the rename to the disk (Unix; best effort, the file is complete
/// either way).
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = std::fs::File::open(dir) {
        let _ = d.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

#[cfg(test)]
mod tests {
    use super::*;
    use sn_ui_core::i18n::Lang;
    use sn_ui_core::prefs::StartSize;

    #[test]
    fn round_trip_through_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("prefs.json");
        let mut s = Stored::default();
        s.prefs.info_default = true;
        s.prefs.language = Some(Lang::De);
        s.prefs.theme = ThemeChoice::HighContrastDark;
        s.prefs.start_size = StartSize::S1024x768;
        s.frame = Some((1000.0, 700.0));
        s.generator = Some(json!({"kind": "Linear"}));
        s.save(&path).unwrap();
        assert_eq!(Stored::load(&path), s);
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn a_folder_that_cannot_be_made_is_an_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        let e = Stored::default().save(&blocker.join("prefs.json"));
        assert!(matches!(e, Err(SaveError::Io(_))), "{e:?}");
    }

    #[test]
    fn damaged_files_give_defaults_value_by_value() {
        assert_eq!(Stored::from_json(None), Stored::default());
        assert_eq!(Stored::from_json(Some("not json")), Stored::default());
        let s = Stored::from_json(Some(
            r#"{"prefs": {"version": 1, "info_default": true, "theme": "NoSuchTheme"},
                "frame": [10, 10]}"#,
        ));
        assert!(s.prefs.info_default);
        assert_eq!(s.prefs.theme, Prefs::default().theme);
        assert_eq!(s.frame, None, "too small to be a frame size");
        let s = Stored::from_json(Some(r#"{"frame": [100000, 100000]}"#));
        assert_eq!(s.frame, None, "too large to be a frame size");
        let s = Stored::from_json(Some(r#"{"frame": [1600, 900]}"#));
        assert_eq!(s.frame, Some((1600.0, 900.0)));
    }

    #[test]
    fn settings_from_before_windows_style_keep_their_theme_on_the_windows() {
        let s = Stored::from_json(Some(r#"{"prefs": {"version": 1, "theme": "NightVision"}}"#));
        assert_eq!(s.prefs.theme, ThemeChoice::NightVision);
        assert_eq!(
            s.prefs.look.style,
            WinStyle::Win95,
            "the theme still colours"
        );
        let s = Stored::from_json(Some(r#"{"prefs": {"version": 1, "theme": "Classic"}}"#));
        assert_eq!(
            s.prefs.look.style,
            WinStyle::Xp,
            "Windows Standard users get XP"
        );
        let s = Stored::from_json(Some(
            r#"{"prefs": {"version": 1, "theme": "Dark", "look": {"style": "Win10"}}}"#,
        ));
        assert_eq!(
            s.prefs.look.style,
            WinStyle::Win10,
            "a chosen style is kept"
        );
        assert_eq!(
            Stored::from_json(None).prefs.look.style,
            WinStyle::Xp,
            "new users"
        );
    }
}
