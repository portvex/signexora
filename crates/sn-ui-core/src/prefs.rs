//! Display and loader settings remembered across runs (stored by the front end).
//!
//! The stored text is JSON. Anything missing, unreadable or out of range falls back
//! to the default, so a damaged or older settings file never stops the app. Each
//! value is read on its own: an unknown language or theme (say, from a newer
//! version) resets only that value, not the rest. The same holds inside the
//! nested sonogram settings and the remembered raw format, whose unreadable
//! fields take the loader's defaults.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sn_dsp::window::WindowKind;
use sn_io::RawFormat;

use crate::colormap::Colormap;
use crate::i18n::Lang;
use crate::sono_settings::{FFT_SIZES, Settings};
use crate::theme::ThemeChoice;
use crate::winstyle::Look;

/// Smallest and largest user interface scale.
pub const UI_SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.5..=3.0;

/// Storage key of the remembered settings.
pub const STORAGE_KEY: &str = "sa-rs-prefs";

/// Storage key of the signal generator's last spec. It is kept apart from
/// [`Prefs`] (which is small and `Copy`), so a spec that cannot be read resets
/// only the generator.
pub const GENERATOR_KEY: &str = "sa-rs-generator";

/// The signal generator's spec stored as `text`, read leniently: a spec from an
/// older or newer version keeps every field that still reads (over the defaults of
/// its kind), and anything unreadable gives the generator's default spec. The
/// other settings are never affected.
pub fn generator_from_json(text: Option<&str>) -> sn_synth::GeneratorSpec {
    let fallback = || sn_synth::GeneratorSpec::default_for(sn_synth::SignalKind::Linear);
    let Some(v) = text.and_then(|t| serde_json::from_str::<Value>(t).ok()) else {
        return fallback();
    };
    if let Ok(spec) = serde_json::from_value::<sn_synth::GeneratorSpec>(v.clone()) {
        return spec;
    }
    let kind = v
        .get("kind")
        .and_then(|k| serde_json::from_value::<sn_synth::SignalKind>(k.clone()).ok());
    kind.map_or_else(fallback, |k| {
        lenient(sn_synth::GeneratorSpec::default_for(k), &v)
    })
}

/// The generator's spec as JSON for storage.
pub fn generator_to_json(spec: &sn_synth::GeneratorSpec) -> Option<String> {
    serde_json::to_string(spec).ok()
}

/// Options > Size of start SA: the size of the main window at the next start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum StartSize {
    /// The size the window had when the app was closed.
    #[default]
    Last,
    /// 800 x 600.
    S800x600,
    /// 1024 x 768.
    S1024x768,
    /// 1280 x 800.
    S1280x800,
    /// 1280 x 1024.
    S1280x1024,
    /// 1600 x 900.
    S1600x900,
    /// 1920 x 1080.
    S1920x1080,
    /// The whole screen.
    Maximized,
}

impl StartSize {
    /// Every choice, in menu order.
    pub const ALL: [Self; 8] = [
        Self::Last,
        Self::S800x600,
        Self::S1024x768,
        Self::S1280x800,
        Self::S1280x1024,
        Self::S1600x900,
        Self::S1920x1080,
        Self::Maximized,
    ];

    /// The inner size in points, for the fixed sizes.
    pub const fn size(self) -> Option<[f32; 2]> {
        match self {
            Self::Last | Self::Maximized => None,
            Self::S800x600 => Some([800.0, 600.0]),
            Self::S1024x768 => Some([1024.0, 768.0]),
            Self::S1280x800 => Some([1280.0, 800.0]),
            Self::S1280x1024 => Some([1280.0, 1024.0]),
            Self::S1600x900 => Some([1600.0, 900.0]),
            Self::S1920x1080 => Some([1920.0, 1080.0]),
        }
    }

    /// The menu text.
    pub fn label(self) -> String {
        match self {
            Self::Last => crate::t!("menu-start-size-last"),
            Self::Maximized => crate::t!("menu-start-size-max"),
            _ => self
                .size()
                .map(|[w, h]| format!("{w:.0} x {h:.0}"))
                .unwrap_or_default(),
        }
    }
}

/// Settings a new recording and the loader start from.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// Sonogram toolbar settings.
    pub sonogram: Settings,
    /// Waveform half range.
    pub wave_scale: f32,
    /// Waveform: draw the Q channel.
    pub show_q: bool,
    /// Raster colour map.
    pub raster_colormap: Colormap,
    /// Raw sample format last used to open a headerless file.
    pub raw: Option<RawFormat>,
    /// User interface language; `None` follows the system.
    pub language: Option<Lang>,
    /// Colour theme (the appearance scheme of the classic Windows styles).
    pub theme: ThemeChoice,
    /// Options > Windows style: the Windows version drawn and its variant.
    pub look: Look,
    /// User interface scale (zoom factor).
    pub ui_scale: f32,
    /// Options > Info by default: new sonogram windows show the Info overlay.
    pub info_default: bool,
    /// Options > Size of start SA.
    pub start_size: StartSize,
    /// Version of the stored settings (see [`PREFS_VERSION`]).
    pub version: u32,
}

/// Version of the stored settings. Version 1 introduced the SA-identical
/// interface: settings stored before it switch to the Classic (SA) theme once,
/// since their theme was chosen for the earlier, modern interface.
pub const PREFS_VERSION: u32 = 1;

impl Default for Prefs {
    fn default() -> Self {
        Self {
            sonogram: Settings::default(),
            wave_scale: 1.0,
            show_q: true,
            raster_colormap: crate::colormap::RASTER_DEFAULT,
            raw: None,
            language: None,
            // SA's own look for new users; a stored choice is kept.
            theme: ThemeChoice::default(),
            look: Look::default(),
            ui_scale: 1.0,
            info_default: false,
            start_size: StartSize::Last,
            version: PREFS_VERSION,
        }
    }
}

impl Prefs {
    /// Read stored settings; `None` or unreadable text gives the defaults. A value
    /// that cannot be read (an unknown enum variant, a wrong type) takes its default
    /// and the other values are kept.
    pub fn from_json(text: Option<&str>) -> Self {
        text.and_then(|t| serde_json::from_str::<Value>(t).ok())
            .map(|v| {
                // `raw` defaults to null, which has no fields to merge into: start
                // from the loader's format when one is stored, so one unreadable
                // field (a sample format from a newer version) resets only itself.
                let stored_raw = v.get("raw").is_some_and(Value::is_object);
                let seed = Self {
                    raw: stored_raw.then_some(crate::loader_info::DEFAULT_RAW),
                    ..Self::default()
                };
                let mut p = lenient(seed, &v);
                // Settings from before the SA-identical interface: show SA's look.
                let stored_version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
                if stored_version < u64::from(PREFS_VERSION) {
                    p.theme = ThemeChoice::Classic;
                    p.version = PREFS_VERSION;
                }
                p
            })
            .unwrap_or_default()
            .sanitized()
    }

    /// The settings as JSON for storage.
    pub fn to_json(self) -> Option<String> {
        serde_json::to_string(&self).ok()
    }

    /// Replace values the UI could not have produced with their defaults.
    fn sanitized(mut self) -> Self {
        let d = Self::default();
        let s = &mut self.sonogram;
        if !FFT_SIZES.contains(&s.fft_size) {
            s.fft_size = d.sonogram.fft_size;
        }
        if !(s.range_db.is_finite() && (10.0..=160.0).contains(&s.range_db)) {
            s.range_db = d.sonogram.range_db;
        }
        if !WindowKind::ALL.contains(&s.window) {
            s.window = d.sonogram.window;
        }
        self.look = self.look.sanitized();
        if !(self.wave_scale.is_finite() && (1e-6..=1e9).contains(&self.wave_scale)) {
            self.wave_scale = d.wave_scale;
        }
        if self
            .raw
            .is_some_and(|r| !(crate::loader_info::rate_ok(r.sample_rate) && r.channels > 0))
        {
            self.raw = None;
        }
        if !(self.ui_scale.is_finite() && UI_SCALE_RANGE.contains(&self.ui_scale)) {
            self.ui_scale = d.ui_scale;
        }
        self
    }

    /// The language to show: the stored choice, or the system's.
    pub fn effective_language(&self) -> Lang {
        self.language.unwrap_or_else(Lang::system)
    }
}

/// `input` read over `default` value by value: each field (and each field of a
/// nested struct) of `input` is taken only if the whole still deserializes, so one
/// unreadable value does not discard the others. Merging field by field needs an
/// object in `default` to merge into: a field that is null there (an empty
/// `Option`) is taken whole or not at all, so seed it when that matters.
fn lenient<T: Serialize + DeserializeOwned>(default: T, input: &Value) -> T {
    let Ok(mut acc) = serde_json::to_value(&default) else {
        return default;
    };
    let valid = |v: &Value| serde_json::from_value::<T>(v.clone()).is_ok();
    overlay(&mut acc, &mut Vec::new(), input, &valid);
    serde_json::from_value(acc).unwrap_or(default)
}

/// The value at `path` inside `root`.
fn at<'a>(root: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
    path.iter().try_fold(root, |v, k| v.get_mut(k))
}

/// Copy the fields of `input` into the object at `path` in `root`, one by one,
/// keeping each only while `valid(root)` holds. A rejected object is merged field
/// by field instead.
fn overlay(
    root: &mut Value,
    path: &mut Vec<String>,
    input: &Value,
    valid: &dyn Fn(&Value) -> bool,
) {
    let Value::Object(fields) = input else {
        return;
    };
    for (key, value) in fields {
        let Some(Value::Object(target)) = at(root, path) else {
            return;
        };
        // Unknown fields are ignored by serde; skipping them keeps the result small.
        let Some(old) = target.get(key).cloned() else {
            continue;
        };
        target.insert(key.clone(), value.clone());
        if valid(root) {
            continue;
        }
        if let Some(Value::Object(target)) = at(root, path) {
            target.insert(key.clone(), old.clone());
        }
        if old.is_object() && value.is_object() {
            path.push(key.clone());
            overlay(root, path, value, valid);
            path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winstyle::{WinStyle, XpColor};

    #[test]
    fn settings_without_a_look_get_the_default_look() {
        let p = Prefs::from_json(Some(r#"{"version":1,"theme":"Classic"}"#));
        assert_eq!(p.look, Look::default());
    }

    #[test]
    fn an_unknown_windows_style_resets_only_the_style() {
        // 15208739 is 0xE81123, the red accent.
        let text = r#"{"version":1,"look":{"style":"Win12","xp":"Silver","aero":false,"accent":15208739,"dark":true}}"#;
        let l = Prefs::from_json(Some(text)).look;
        assert_eq!(l.style, WinStyle::default());
        assert_eq!(
            (l.xp, l.aero, l.accent, l.dark),
            (XpColor::Silver, false, 0xE8_11_23, true)
        );
    }

    #[test]
    fn a_look_round_trips_through_the_settings() {
        let look = Look {
            style: WinStyle::Win10,
            xp: XpColor::Olive,
            aero: false,
            accent: 0xFF_8C_00,
            dark: true,
            aqua: crate::winstyle::AquaColor::Graphite,
            mac_accent: 0xF7_4F_9E,
            yaru: true,
            linux_accent: 0x91_41_AC,
            yaru_accent: 0x00_35_84_E4,
            breeze_accent: 0x00_E6_2D_42,
        };
        let p = Prefs {
            look,
            ..Prefs::default()
        };
        assert_eq!(Prefs::from_json(p.to_json().as_deref()).look, look);
    }

    #[test]
    fn a_stored_linux_accent_outside_the_swatches_resets_to_the_default() {
        // 1193046 is 0x123456, no swatch.
        let text = r#"{"version":1,"look":{"style":"LinuxAdwaita","linux_accent":1193046,"yaru_accent":1193046,"breeze_accent":1193046}}"#;
        let l = Prefs::from_json(Some(text)).look;
        assert_eq!(l.style, WinStyle::LinuxAdwaita);
        assert_eq!(
            (l.linux_accent, l.yaru_accent, l.breeze_accent),
            (
                crate::winstyle::DEFAULT_LINUX_ACCENT,
                crate::winstyle::YARU_ORANGE,
                crate::winstyle::BREEZE_BLUE
            )
        );
    }

    #[test]
    fn a_stored_accent_outside_the_swatches_resets_to_the_default() {
        let text = r#"{"version":1,"look":{"style":"Win8","accent":13959168}}"#;
        let l = Prefs::from_json(Some(text)).look;
        assert_eq!(l.style, WinStyle::Win8);
        assert_eq!(l.accent, crate::winstyle::DEFAULT_ACCENT);
    }

    #[test]
    fn an_unreadable_mac_value_resets_only_that_value() {
        // 1193046 is 0x123456, not a macOS swatch.
        let text = r#"{"version":1,"look":{"style":"MacModern","aqua":"Pink","mac_accent":1193046,"dark":true}}"#;
        let l = Prefs::from_json(Some(text)).look;
        assert_eq!(l.style, WinStyle::MacModern);
        assert_eq!(l.aqua, crate::winstyle::AquaColor::Blue);
        assert_eq!(l.mac_accent, crate::winstyle::DEFAULT_MAC_ACCENT);
        assert_eq!(l.accent_rgb(), crate::winstyle::DEFAULT_MAC_ACCENT);
        assert!(l.dark);
        assert_eq!(l.accent, crate::winstyle::DEFAULT_ACCENT);
        // 16207774 is 0xF74F9E, the pink swatch, which is kept.
        let text = r#"{"version":1,"look":{"style":"MacYosemite","aqua":"Graphite","mac_accent":16207774}}"#;
        let l = Prefs::from_json(Some(text)).look;
        assert_eq!(l.style, WinStyle::MacYosemite);
        assert_eq!(l.aqua, crate::winstyle::AquaColor::Graphite);
        assert_eq!(l.mac_accent, 0xF7_4F_9E);
        let l = Prefs::from_json(Some(
            r#"{"version":1,"look":{"style":"MacModern","mac_accent":123}}"#,
        ))
        .look;
        assert_eq!(l.mac_accent, crate::winstyle::DEFAULT_MAC_ACCENT);
        assert_eq!(l.accent_rgb(), crate::winstyle::DEFAULT_MAC_ACCENT);
    }

    #[test]
    fn settings_from_before_the_sa_interface_switch_to_classic_once() {
        // Stored by an earlier version (no version field) with the dark theme.
        let old = Prefs::from_json(Some(r#"{"theme": "Dark", "wave_scale": 2.0}"#));
        assert_eq!(old.theme, ThemeChoice::Classic);
        assert_eq!(old.version, PREFS_VERSION);
        assert_eq!(old.wave_scale, 2.0);
        // A choice made after the switch is kept.
        let Some(text) = Prefs {
            theme: ThemeChoice::Dark,
            ..old
        }
        .to_json() else {
            panic!("json");
        };
        assert_eq!(Prefs::from_json(Some(&text)).theme, ThemeChoice::Dark);
    }

    #[test]
    fn round_trips_and_survives_bad_input() {
        let mut p = Prefs::default();
        p.sonogram.fft_size = 4096;
        p.sonogram.colormap = Colormap::Inferno;
        p.wave_scale = 250.0;
        p.raster_colormap = Colormap::Viridis;
        p.raw = Some(RawFormat {
            format: sn_io::SampleFormat::F32,
            endian: sn_io::Endian::Big,
            channels: 2,
            sample_rate: 96_000.0,
            data_offset: 16,
        });
        p.language = Some(Lang::Ja);
        p.theme = ThemeChoice::HighContrastDark;
        p.ui_scale = 1.5;
        p.info_default = true;
        p.start_size = StartSize::S1024x768;
        let text = p.to_json().unwrap();
        assert_eq!(Prefs::from_json(Some(&text)), p);

        assert_eq!(Prefs::from_json(None), Prefs::default());
        assert_eq!(Prefs::from_json(Some("not json")), Prefs::default());
        // Missing fields take their defaults; impossible values are replaced.
        let partial = Prefs::from_json(Some(
            r#"{"wave_scale": -3.0, "ui_scale": 40.0, "sonogram": {"fft_size": 1000, "range_db": 1e9}}"#,
        ));
        assert_eq!(partial, Prefs::default());
        // An unknown language or theme name falls back to the defaults.
        assert_eq!(
            Prefs::from_json(Some(r#"{"language": "Klingon", "theme": "Neon"}"#)),
            Prefs::default()
        );
        // ... and only that value: the readable values next to it are kept,
        // also inside the nested sonogram settings.
        let mixed = Prefs::from_json(Some(
            r#"{"language": "Zh", "theme": "Neon", "wave_scale": 2.0, "ui_scale": 1.5,
                "raster_colormap": "NoSuchMap", "show_q": "yes",
                "sonogram": {"fft_size": 4096, "colormap": "Plasma9", "window": "Nope"},
                "raw": {"format": "F32", "endian": "Big", "channels": 2,
                        "sample_rate": 96000.0, "data_offset": 16}}"#,
        ));
        let mut want = Prefs {
            wave_scale: 2.0,
            ui_scale: 1.5,
            raw: p.raw,
            ..Prefs::default()
        };
        want.sonogram.fft_size = 4096;
        assert_eq!(mixed, want);
        // A known language next to a bad theme is kept.
        let lang = Prefs::from_json(Some(r#"{"language": "De", "theme": 7}"#));
        assert_eq!(lang.language, Some(Lang::De));
        assert_eq!(lang.theme, ThemeChoice::Classic);
        // New users get the Classic (SA) theme; a choice stored at the current
        // version is kept.
        assert_eq!(Prefs::default().theme, ThemeChoice::Classic);
        let stored = Prefs::from_json(Some(r#"{"theme": "System", "version": 1}"#));
        assert_eq!(stored.theme, ThemeChoice::System);
        // One unreadable field of the raw format (a sample format from a newer
        // version) takes the loader's default; the other fields are kept.
        let raw = Prefs::from_json(Some(
            r#"{"raw": {"format": "F128", "endian": "Big", "channels": 2,
                        "sample_rate": 96000.0, "data_offset": 16}}"#,
        ));
        assert_eq!(
            raw.raw,
            Some(RawFormat {
                format: crate::loader_info::DEFAULT_RAW.format,
                ..p.raw.unwrap_or(crate::loader_info::DEFAULT_RAW)
            })
        );
        // No stored raw format stays none.
        assert_eq!(Prefs::from_json(Some("{}")).raw, None);
        // A rate the loader would refuse is not offered as the starting format.
        let huge = Prefs::from_json(Some(
            r#"{"raw": {"format": "I16", "endian": "Little", "channels": 1,
                        "sample_rate": 1e300, "data_offset": 0}}"#,
        ));
        assert_eq!(huge.raw, None);
    }

    #[test]
    fn generator_spec_survives_bad_input() {
        use sn_synth::{GeneratorSpec, SignalKind};
        let linear = GeneratorSpec::default_for(SignalKind::Linear);
        let mut ofdm = GeneratorSpec::preset("cis12_like").unwrap();
        ofdm.common_mut().seed = 77;
        let text = generator_to_json(&ofdm).unwrap();
        assert_eq!(generator_from_json(Some(&text)), ofdm);
        assert_eq!(generator_from_json(None), linear);
        assert_eq!(generator_from_json(Some("not json")), linear);
        assert_eq!(generator_from_json(Some(r#"{"kind": "warp"}"#)), linear);
        // One unreadable field resets only itself; the rest of the spec is kept.
        let mut v: Value = serde_json::from_str(&text).unwrap();
        v["modulation"] = Value::from("qam4096");
        v["future_field"] = Value::from(1);
        let back = generator_from_json(Some(&v.to_string()));
        let (GeneratorSpec::Ofdm(got), GeneratorSpec::Ofdm(want)) = (&back, &ofdm) else {
            panic!("not OFDM: {back:?}")
        };
        assert_eq!(got.lu, want.lu);
        assert_eq!(got.common.seed, 77);
        assert_eq!(
            got.modulation,
            match GeneratorSpec::default_for(SignalKind::Ofdm) {
                GeneratorSpec::Ofdm(d) => d.modulation,
                _ => panic!("default"),
            }
        );
        // A damaged generator spec leaves the other settings alone.
        assert_eq!(Prefs::from_json(Some("{}")), Prefs::default());
    }
}
