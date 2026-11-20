//! User interface translations (Fluent).
//!
//! Every user-visible string of the app lives in `locales/<language>/<module>.ftl`.
//! The build script embeds all of them. A message missing in the chosen language
//! falls back to English, and a key missing in English shows the key itself, so a
//! gap never stops the app.
//!
//! Use the [`t!`](crate::t) macro: `t!("menu-file")` or
//! `t!("status-saved", path = path.display().to_string())`.
//!
//! Rules for arguments:
//! - Pass counts that choose a plural form as integers (`count = n`).
//! - Pass every other number as a string formatted the way the code did before
//!   (`freq = format!("{f:.3}")`). Fluent's own number formatting is not used.
//! - Text from lower crates (error messages, file names) is passed as an
//!   argument and is not translated.

use std::sync::RwLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/locales.rs"));
}

/// A user interface language.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Lang {
    /// English (the source language and the fallback).
    #[default]
    En,
    /// French.
    Fr,
    /// Spanish.
    Es,
    /// German.
    De,
    /// Portuguese.
    Pt,
    /// Ukrainian.
    Uk,
    /// Russian.
    Ru,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
}

impl Lang {
    /// Every supported language, in menu order.
    pub const ALL: [Self; 9] = [
        Self::En,
        Self::Fr,
        Self::Es,
        Self::De,
        Self::Pt,
        Self::Uk,
        Self::Ru,
        Self::Ja,
        Self::Ko,
    ];

    /// The BCP 47 language code, also the `locales/` directory name.
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
            Self::Es => "es",
            Self::De => "de",
            Self::Pt => "pt",
            Self::Uk => "uk",
            Self::Ru => "ru",
            Self::Ja => "ja",
            Self::Ko => "ko",
        }
    }

    /// The language's name in that language, for the language menu.
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Fr => "Français",
            Self::Es => "Español",
            Self::De => "Deutsch",
            Self::Pt => "Português",
            Self::Uk => "Українська",
            Self::Ru => "Русский",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
        }
    }

    /// The language of a locale tag such as `pt-BR`, `uk_UA.UTF-8` or `ja`.
    pub fn from_tag(tag: &str) -> Option<Self> {
        let primary = tag
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        Self::ALL.into_iter().find(|l| l.code() == primary)
    }

    /// The first supported language among the system's preferred locales, or
    /// English.
    pub fn system() -> Self {
        sys_locale::get_locales()
            .find_map(|tag| Self::from_tag(&tag))
            .unwrap_or_default()
    }
}

/// The bundles used to format messages: the chosen language, then English.
struct Catalog {
    lang: Lang,
    bundles: Vec<FluentBundle<FluentResource>>,
}

impl std::fmt::Debug for Catalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Catalog")
            .field("lang", &self.lang)
            .finish_non_exhaustive()
    }
}

impl Catalog {
    fn new(lang: Lang) -> Self {
        let mut bundles = vec![bundle(lang)];
        if lang != Lang::En {
            bundles.push(bundle(Lang::En));
        }
        Self { lang, bundles }
    }

    /// The message (or `message.attribute`) for `key`, or `None` when no bundle
    /// has it. Formatting errors (a missing argument, an unknown reference) are
    /// reported once per key on standard error; the text is still shown.
    fn format(&self, key: &str, args: Option<&FluentArgs<'_>>) -> Option<String> {
        let (text, errors) = self.format_checked(key, args)?;
        if !errors.is_empty() {
            report_errors(self.lang, key, &errors);
        }
        Some(text)
    }

    /// [`Self::format`] with the resolver errors returned instead of reported.
    fn format_checked(
        &self,
        key: &str,
        args: Option<&FluentArgs<'_>>,
    ) -> Option<(String, Vec<fluent_bundle::FluentError>)> {
        let (id, attribute) = match key.split_once('.') {
            Some((id, attr)) => (id, Some(attr)),
            None => (key, None),
        };
        self.bundles.iter().find_map(|b| {
            let message = b.get_message(id)?;
            let pattern = match attribute {
                Some(a) => message.get_attribute(a)?.value(),
                None => message.value()?,
            };
            let mut errors = Vec::new();
            let text = b.format_pattern(pattern, args, &mut errors).into_owned();
            Some((text, errors))
        })
    }
}

/// Say on standard error that `key` did not format cleanly (for example a call
/// site that forgot an argument, which then shows as `{$name}`). Each key is
/// reported once per language, so a message drawn every frame does not flood
/// the log.
fn report_errors(lang: Lang, key: &str, errors: &[fluent_bundle::FluentError]) {
    static REPORTED: std::sync::Mutex<std::collections::BTreeSet<(Lang, String)>> =
        std::sync::Mutex::new(std::collections::BTreeSet::new());
    let first = REPORTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert((lang, key.to_owned()));
    if first {
        eprintln!(
            "signexora: translation {key} ({}) did not format cleanly: {errors:?}",
            lang.code()
        );
    }
}

/// One language's bundle. Files that fail to parse still contribute the
/// messages that did parse; the tests make sure none fail.
fn bundle(lang: Lang) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = lang.code().parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // Unicode isolation marks around arguments render as boxes in egui.
    bundle.set_use_isolating(false);
    for (code, _, text) in embedded::FILES {
        if *code != lang.code() {
            continue;
        }
        let resource = FluentResource::try_new((*text).to_owned()).unwrap_or_else(|(r, _)| r);
        // A duplicate key keeps the first definition.
        let _ = bundle.add_resource(resource);
    }
    bundle
}

static CATALOG: RwLock<Option<Catalog>> = RwLock::new(None);

/// The decimal separator of the interface language, cached by
/// [`set_language`] so number formatting in every frame needs no Fluent lookup.
/// Zero until a language is set.
static SEPARATOR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

impl Catalog {
    /// The single-character decimal separator of this language (`.` when the
    /// translation is missing or longer than one character).
    fn decimal_separator(&self) -> char {
        let text = self
            .format("format-decimal-separator", None)
            .unwrap_or_default();
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => c,
            _ => '.',
        }
    }
}

/// The decimal separator `lang` uses.
#[cfg(test)]
pub fn decimal_separator_of(lang: Lang) -> char {
    Catalog::new(lang).decimal_separator()
}

/// The decimal separator of the interface language.
pub fn decimal_separator() -> char {
    let cached = SEPARATOR.load(std::sync::atomic::Ordering::Relaxed);
    if let Some(c) = char::from_u32(cached).filter(|_| cached != 0) {
        return c;
    }
    ensure_catalog();
    // The separator is stored while the catalog it comes from is held, so a
    // language switch (which stores under the write lock) cannot interleave.
    let guard = CATALOG
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let c = guard.as_ref().map_or('.', Catalog::decimal_separator);
    SEPARATOR.store(u32::from(c), std::sync::atomic::Ordering::Relaxed);
    c
}

/// Load English when no language was set yet (leaving one set meanwhile alone).
fn ensure_catalog() {
    let mut guard = CATALOG
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.is_none() {
        let catalog = Catalog::new(Lang::En);
        SEPARATOR.store(
            u32::from(catalog.decimal_separator()),
            std::sync::atomic::Ordering::Relaxed,
        );
        *guard = Some(catalog);
    }
}

/// Switch the user interface language.
pub fn set_language(lang: Lang) {
    let catalog = Catalog::new(lang);
    let separator = catalog.decimal_separator();
    // Catalog and cached separator change together under the write lock.
    let mut guard = CATALOG
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    SEPARATOR.store(u32::from(separator), std::sync::atomic::Ordering::Relaxed);
    *guard = Some(catalog);
}

/// The translated text for `key` with `args`. See the module docs.
pub fn tr_args(key: &str, args: Option<&FluentArgs<'_>>) -> String {
    {
        let guard = CATALOG
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(c) = guard.as_ref() {
            return c.format(key, args).unwrap_or_else(|| key.to_owned());
        }
    }
    ensure_catalog();
    tr_args(key, args)
}

/// Whether English has a message (or `message.attribute`) for `key`, for front
/// ends that check keys they choose at run time.
pub fn exists_in_english(key: &str) -> bool {
    Catalog::new(Lang::En).format(key, None).is_some()
}

/// The translated text for `key`.
pub fn tr(key: &str) -> String {
    tr_args(key, None)
}

/// A toolbar button's tooltip: the hint of `hint_key`, and for a disabled
/// button also what it lacks.
pub fn tooltip(hint_key: &str, why_disabled: Option<&str>) -> String {
    let hint = tr(hint_key);
    match why_disabled {
        Some(why) => format!("{hint}\n{why}"),
        None => hint,
    }
}

/// Translate a message: `t!("key")` or `t!("key", name = value, ...)`.
///
/// `key` may name an attribute as `message.attribute`. Values are anything
/// that converts into a Fluent value: strings, or integers for plurals.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::tr($key)
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = $crate::fluent_bundle::FluentArgs::new();
        $( args.set(stringify!($name), $crate::fluent_bundle::FluentValue::from($value)); )+
        $crate::i18n::tr_args($key, Some(&args))
    }};
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    /// Plural categories and other variant keys a translation may choose freely.
    fn is_plural_variant(name: &str) -> bool {
        matches!(name, "zero" | "one" | "two" | "few" | "many" | "other")
            || name.parse::<f64>().is_ok()
    }

    /// `message id -> variant keys` of the string selectors (`[markers]`,
    /// `[grid]`) in one Fluent source; plural categories are left out, since
    /// languages use different ones.
    fn variants(text: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut current: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            if !line.starts_with(' ')
                && let Some((id, _)) = line.split_once('=')
            {
                current = Some(id.trim().to_owned());
                continue;
            }
            let Some(c) = &current else { continue };
            let Some(rest) = trimmed.trim_start_matches('*').strip_prefix('[') else {
                continue;
            };
            let Some((name, _)) = rest.split_once(']') else {
                continue;
            };
            let name = name.trim();
            if !is_plural_variant(name) {
                out.entry(c.clone()).or_default().insert(name.to_owned());
            }
        }
        out
    }

    /// `message id -> variables used` for one Fluent source. Attributes are
    /// listed as `id.attr`.
    fn messages(text: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut current: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            let indented = line.starts_with(' ');
            if !indented && let Some((id, _)) = line.split_once('=') {
                let id = id.trim().to_owned();
                current = Some(id.clone());
                out.entry(id).or_default();
            } else if indented
                && trimmed.starts_with('.')
                && let Some((attr, _)) = trimmed.split_once('=')
                && let Some(c) = &current
            {
                out.entry(format!("{c}.{}", attr.trim().trim_start_matches('.')))
                    .or_default();
            }
            if let Some(c) = &current {
                let vars = out.entry(c.clone()).or_default();
                let mut rest = line;
                while let Some(i) = rest.find('$') {
                    rest = rest.get(i + 1..).unwrap_or_default();
                    let name: String = rest
                        .chars()
                        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
                        .collect();
                    if !name.is_empty() {
                        vars.insert(name);
                    }
                }
            }
        }
        out
    }

    fn files_of(lang: Lang) -> BTreeMap<&'static str, &'static str> {
        embedded::FILES
            .iter()
            .filter(|(code, _, _)| *code == lang.code())
            .map(|(_, name, text)| (*name, *text))
            .collect()
    }

    #[test]
    fn every_file_parses() {
        for (code, name, text) in embedded::FILES {
            if let Err((_, errors)) = FluentResource::try_new((*text).to_owned()) {
                panic!("{code}/{name} does not parse: {errors:?}");
            }
        }
    }

    #[test]
    fn every_language_has_the_same_files_messages_and_variables() {
        // `SA_I18N_LANG=fr cargo test -p sn-ui-core i18n` checks one language only.
        let only = std::env::var("SA_I18N_LANG").ok();
        let english = files_of(Lang::En);
        assert!(!english.is_empty(), "no English files are embedded");
        let mut problems = Vec::new();
        for lang in Lang::ALL {
            if only.as_deref().is_some_and(|o| o != lang.code()) {
                continue;
            }
            let code = lang.code();
            let files = files_of(lang);
            for name in english.keys().filter(|n| !files.contains_key(*n)) {
                problems.push(format!("{code}/{name}: missing file"));
            }
            for name in files.keys().filter(|n| !english.contains_key(*n)) {
                problems.push(format!("{code}/{name}: no English file of that name"));
            }
            for (name, text) in &english {
                let Some(other) = files.get(name) else {
                    continue;
                };
                let want = messages(text);
                let got = messages(other);
                for id in want.keys().filter(|k| !got.contains_key(*k)) {
                    problems.push(format!("{code}/{name}: missing {id}"));
                }
                for id in got.keys().filter(|k| !want.contains_key(*k)) {
                    problems.push(format!("{code}/{name}: {id} is not in English"));
                }
                // A string selector must offer the same choices, or a dropped
                // or misspelt one silently shows the default sentence.
                let (want_v, got_v) = (variants(text), variants(other));
                for (id, names) in &want_v {
                    let have = got_v.get(id).cloned().unwrap_or_default();
                    if &have != names {
                        problems.push(format!(
                            "{code}/{name}: {id} has variants {have:?}, English has {names:?}"
                        ));
                    }
                }
                for id in got_v.keys().filter(|k| !want_v.contains_key(*k)) {
                    problems.push(format!("{code}/{name}: {id} has variants English lacks"));
                }
                for (id, vars) in &want {
                    // A translation may drop a variable (e.g. a count spelled out
                    // in a plural form) but must not invent one.
                    if let Some(used) = got.get(id)
                        && !used.is_subset(vars)
                    {
                        problems.push(format!(
                            "{code}/{name}: {id} uses {used:?}, English has {vars:?}"
                        ));
                    }
                }
            }
        }
        assert!(
            problems.is_empty(),
            "{} problems:\n{}",
            problems.len(),
            problems.join("\n")
        );
    }

    #[test]
    fn no_message_id_is_defined_twice() {
        for lang in Lang::ALL {
            let mut seen = BTreeSet::new();
            for text in files_of(lang).values() {
                for line in text.lines() {
                    if line.starts_with([' ', '#']) || line.trim().is_empty() {
                        continue;
                    }
                    if let Some((id, _)) = line.split_once('=') {
                        let id = id.trim();
                        assert!(
                            seen.insert(id.to_owned()),
                            "{}: {id} is defined twice",
                            lang.code()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn translations_avoid_decorative_unicode() {
        // Project rule: no em or en dashes, arrows, bullets, check marks or stars.
        let banned = |c: char| {
            matches!(
                c,
                '\u{2013}'
                    | '\u{2014}'
                    | '\u{2022}'
                    | '\u{2713}'
                    | '\u{2714}'
                    | '\u{2605}'
                    | '\u{2606}'
            ) || ('\u{2190}'..='\u{21FF}').contains(&c)
        };
        for (code, name, text) in embedded::FILES {
            for (n, line) in text.lines().enumerate() {
                assert!(
                    !line.chars().any(banned),
                    "{code}/{name}:{}: decorative Unicode character",
                    n + 1
                );
            }
        }
    }

    /// The source directories, relative to `crates/`, whose `t!` keys must
    /// exist in English: this crate and the front ends.
    const SOURCE_DIRS: [&str; 3] = ["sn-ui-core/src", "sa-app/src", "sn-gui/src"];

    #[test]
    fn every_key_used_in_the_source_exists_in_english() {
        let english = Catalog::new(Lang::En);
        // The sources of this crate and of every front end that uses it. A front
        // end that is not in the workspace (yet, or any more) is skipped.
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut paths = Vec::new();
        for name in SOURCE_DIRS {
            let dir = crates.join(name);
            if !dir.is_dir() {
                continue;
            }
            // Front ends keep modules in sub-folders (`sn-gui/src/app/`).
            let mut dirs = vec![dir];
            while let Some(d) = dirs.pop() {
                for entry in std::fs::read_dir(&d).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        dirs.push(path);
                    } else {
                        paths.push(path);
                    }
                }
            }
        }
        assert!(
            paths.iter().any(|p| p.ends_with("sn-ui-core/src/lib.rs")),
            "this crate's sources were not scanned"
        );
        paths.sort();
        let mut missing = Vec::new();
        for path in paths {
            // This module's docs and tests use example keys.
            if path.extension().is_none_or(|x| x != "rs")
                || path.ends_with("sn-ui-core/src/i18n.rs")
            {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for marker in ["t!(", "tr("] {
                let mut rest = text.as_str();
                while let Some(i) = rest.find(marker) {
                    // `format!("` also ends in `t!("`: require a word boundary.
                    let boundary = rest
                        .get(..i)
                        .unwrap_or_default()
                        .chars()
                        .next_back()
                        .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                    rest = rest.get(i + marker.len()..).unwrap_or_default();
                    if !boundary {
                        continue;
                    }
                    // rustfmt puts the key of a long call on the next line.
                    let Some(quoted) = rest.trim_start().strip_prefix('"') else {
                        continue;
                    };
                    let key: String = quoted.chars().take_while(|c| *c != '"').collect();
                    // Skip the examples in this module's docs and tests.
                    if key.is_empty() || key == "key" || key.starts_with("no-such") {
                        continue;
                    }
                    if english.format(&key, None).is_none() {
                        missing.push(format!("{}: {key}", path.display()));
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "keys missing from locales/en: {missing:#?}"
        );
    }

    #[test]
    fn keys_chosen_at_run_time_exist_in_english() {
        let english = Catalog::new(Lang::En);
        let mut keys: Vec<String> = crate::theme::ThemeChoice::ALL
            .iter()
            .map(|c| c.key().to_owned())
            .collect();
        keys.extend(
            [
                "playback-playing-first-markers",
                "playback-playing-first-view",
                // The front ends build these as `measure-tool-{id}` from the
                // tool id stored in the session.
                "measure-tool-adp",
                "measure-tool-adp-coarse",
                "measure-tool-measure-line",
                "measure-tool-v-markers",
                "measure-tool-ofdm-adp",
                "measure-tool-script-record",
            ]
            .map(str::to_owned),
        );
        for key in keys {
            assert!(
                english.format(&key, None).is_some(),
                "{key} is not in English"
            );
        }
    }

    #[test]
    fn every_english_message_formats_without_errors() {
        // Each message gets every variable it uses; a reference to an unknown
        // message or term, or a malformed placeable, shows as an error here.
        let english = Catalog::new(Lang::En);
        let mut problems = Vec::new();
        for text in files_of(Lang::En).values() {
            let found = messages(text);
            for id in found.keys() {
                // The parser lists an attribute's variables under its message.
                let base = id.split('.').next().unwrap_or_default();
                let vars: BTreeSet<&String> = found
                    .get(id)
                    .into_iter()
                    .chain(found.get(base))
                    .flatten()
                    .collect();
                let mut args = FluentArgs::new();
                for v in vars {
                    args.set(v.clone(), fluent_bundle::FluentValue::from(1));
                }
                match english.format_checked(id, Some(&args)) {
                    Some((_, errors)) if errors.is_empty() => {}
                    Some((_, errors)) => problems.push(format!("{id}: {errors:?}")),
                    None => problems.push(format!("{id}: not found")),
                }
            }
        }
        assert!(problems.is_empty(), "{problems:#?}");
        // A forgotten argument is an error (reported at run time, not hidden).
        let (_, errors) = english
            .format_checked("app-saved", None)
            .unwrap_or_default();
        assert_ne!(errors.len(), 0, "a missing argument is not reported");
    }

    #[test]
    fn unknown_keys_show_the_key_and_tags_map_to_languages() {
        let c = Catalog::new(Lang::Fr);
        assert_eq!(c.format("no-such-key", None), None);
        assert_eq!(tr("no-such-key-either"), "no-such-key-either");
        assert_eq!(Lang::from_tag("pt-BR"), Some(Lang::Pt));
        assert_eq!(Lang::from_tag("uk_UA.UTF-8"), Some(Lang::Uk));
        assert_eq!(Lang::from_tag("JA"), Some(Lang::Ja));
        assert_eq!(Lang::from_tag("zh-Hans"), None);
        assert_eq!(Lang::from_tag(""), None);
    }
}
