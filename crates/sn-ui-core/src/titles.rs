//! Window names in Signals Analyzer's style.
//!
//! SA names every child window after its working copy: the opened file gets `(0)`
//! (`(1)`, `(2)` when the same file is opened again), and every window derived from
//! it appends a tag with a running number per operation type: `test(0)(c1).wav`,
//! `test(0)(c1)(phd1).wav`, `test(0)_acf(0).wav`. The title bar shows
//! `[TempDir] <name>.wav`, SA's working-copy folder.
//!
//! Tags SA used are kept as they are; operations SA had no tag for get one in the
//! same style (see `docs/SA_UI_SPEC.md`, "SigNexora additions").

use std::collections::BTreeMap;

use sn_dsp::ops::Edit;

/// How an operation is tagged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tag {
    /// `(<prefix><n>)` with a running number from 1, e.g. `(c1)`.
    Counted(&'static str),
    /// `_<prefix>(<n>)` with a running number from 0, e.g. `_acf(0)`.
    Suffix(&'static str),
    /// A fixed tag, e.g. `(q)`.
    Fixed(&'static str),
    /// `(r_<rate>)`: resampled to this rate.
    Rate(f64),
}

/// The tag of an edit.
pub fn edit_tag(edit: &Edit) -> Tag {
    match edit {
        // SA's own tags.
        Edit::Copy { .. } => Tag::Counted("c"),
        Edit::BandSelect { .. } => Tag::Counted("f"),
        Edit::InvertBand { .. } | Edit::Invert => Tag::Counted("bi"),
        Edit::PhaseDetector => Tag::Counted("phd"),
        Edit::AmDetector => Tag::Counted("u"),
        Edit::ModifiedAm => Tag::Counted("um"),
        Edit::HalfWave => Tag::Counted("du"),
        Edit::Derivative => Tag::Counted("std"),
        Edit::IqToReal => Tag::Counted("sdr"),
        Edit::Power { .. } => Tag::Fixed("(q)"),
        Edit::Resample { rate } => Tag::Rate(*rate),
        Edit::Limiter => Tag::Counted("h"),
        Edit::Acf { .. } => Tag::Suffix("acf"),
        Edit::Ccf { .. } => Tag::Suffix("ccf"),
        // SigNexora additions, in the same style.
        Edit::BandReject { .. } => Tag::Counted("rj"),
        Edit::AreaDelete { .. } => Tag::Counted("ad"),
        Edit::Shift { .. } => Tag::Counted("s"),
        Edit::ShiftToBase {
            inverted: false, ..
        } => Tag::Counted("sb"),
        Edit::ShiftToBase { inverted: true, .. } => Tag::Counted("sbi"),
        Edit::BaudCorrection { .. } => Tag::Counted("brc"),
        Edit::Lowpass { .. } => Tag::Counted("lpf"),
        Edit::Median { .. } => Tag::Counted("md"),
        Edit::Rrc { .. } => Tag::Counted("rrc"),
        Edit::Equalize { .. } => Tag::Counted("isd"),
        Edit::Resynthesize { .. } => Tag::Counted("rs"),
    }
}

/// A rate as SA writes it in a tag: whole numbers plain, fractions with a comma.
fn rate_text(rate: f64) -> String {
    if !rate.is_finite() {
        return "0".to_owned();
    }
    let text = format!("{rate:.2}");
    // 7780.001 rounds to "7780.00": drop the zeros and the separator too.
    let text = text.trim_end_matches('0').trim_end_matches('.');
    let text = if text == "-0" { "0" } else { text };
    text.replace('.', ",")
}

/// The working-copy title SA shows for a window name.
pub fn window_title(name: &str) -> String {
    format!("[TempDir] {name}.wav")
}

/// The name stem of a source label: the file name without folders or extension.
pub fn stem(label: &str) -> String {
    let file = label.rsplit(['/', '\\']).next().unwrap_or(label);
    let base = match file.rsplit_once('.') {
        Some((b, ext)) if !b.is_empty() && !ext.contains(' ') && ext.len() <= 10 => b,
        _ => file,
    };
    if base.trim().is_empty() {
        "signal".to_owned()
    } else {
        base.to_owned()
    }
}

/// The running numbers of a session, per operation type.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Counters(BTreeMap<String, u32>);

impl Counters {
    /// The next number for `key`, starting at `first`.
    fn next(&mut self, key: &str, first: u32) -> u32 {
        let slot = self.0.entry(key.to_owned()).or_insert(first);
        let n = *slot;
        *slot = slot.saturating_add(1);
        n
    }

    /// The name of a newly opened (or generated) source labelled `label`:
    /// `test(0)`, then `test(1)` when the same file is opened again.
    pub fn source(&mut self, label: &str) -> String {
        let stem = stem(label);
        let n = self.next(&format!("load:{stem}"), 0);
        format!("{stem}({n})")
    }

    /// The name of the result of `edit` on the window named `parent`.
    pub fn derived(&mut self, parent: &str, edit: &Edit) -> String {
        format!("{parent}{}", self.tag(edit_tag(edit)))
    }

    /// The text of `tag`, taking its running number.
    pub fn tag(&mut self, tag: Tag) -> String {
        match tag {
            Tag::Counted(p) => format!("({p}{})", self.next(p, 1)),
            Tag::Suffix(p) => format!("_{p}({})", self.next(p, 0)),
            Tag::Fixed(t) => t.to_owned(),
            Tag::Rate(r) => format!("(r_{})", rate_text(r)),
        }
    }

    /// The tag of a new FSK/MFSK demodulator window, `(f1)` (SA shares the `f`
    /// letter with band select, so they share the count).
    pub fn demod(&mut self) -> String {
        self.tag(Tag::Counted("f"))
    }

    /// The tag of a fragment copied in a WF window, `(wfc0)`, `(wfc1)`, ...
    pub fn wf_copy(&mut self) -> String {
        format!("(wfc{})", self.next("wfc", 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_text_never_ends_in_a_comma() {
        assert_eq!(rate_text(7780.001), "7780");
        assert_eq!(rate_text(7777.999), "7778");
        assert_eq!(rate_text(7777.5), "7777,5");
        assert_eq!(rate_text(7777.77), "7777,77");
        assert_eq!(rate_text(8000.0), "8000");
        assert_eq!(rate_text(f64::NAN), "0");
    }

    #[test]
    fn sources_count_repeated_loads() {
        let mut c = Counters::default();
        assert_eq!(c.source("/data/test.wav"), "test(0)");
        assert_eq!(c.source("C:\\x\\test.wav"), "test(1)");
        assert_eq!(c.source("other.flac"), "other(0)");
        assert_eq!(c.source("test.wav"), "test(2)");
        assert_eq!(stem("noext"), "noext");
        assert_eq!(stem(".wav"), ".wav");
        assert_eq!(stem("FSK 100 Bd, 2 tones"), "FSK 100 Bd, 2 tones");
    }

    #[test]
    fn tags_number_per_operation_type() {
        let mut c = Counters::default();
        let copy = Edit::Copy { start: 0, end: 10 };
        assert_eq!(c.derived("test(0)", &copy), "test(0)(c1)");
        assert_eq!(c.derived("test(0)", &copy), "test(0)(c2)");
        assert_eq!(
            c.derived("test(0)(c1)", &Edit::PhaseDetector),
            "test(0)(c1)(phd1)"
        );
        assert_eq!(c.derived("a", &Edit::AmDetector), "a(u1)");
        assert_eq!(c.derived("a", &Edit::ModifiedAm), "a(um1)");
        assert_eq!(c.derived("a", &Edit::HalfWave), "a(du1)");
        assert_eq!(c.derived("a", &Edit::Derivative), "a(std1)");
        assert_eq!(c.derived("a", &Edit::IqToReal), "a(sdr1)");
        assert_eq!(c.derived("a", &Edit::Limiter), "a(h1)");
        assert_eq!(c.derived("a", &Edit::Power { n: 4 }), "a(q)");
        assert_eq!(c.derived("a", &Edit::Invert), "a(bi1)");
        assert_eq!(
            c.derived("a", &Edit::InvertBand { lo: 1.0, hi: 2.0 }),
            "a(bi2)"
        );
        let band = Edit::BandSelect { lo: 1.0, hi: 2.0 };
        assert_eq!(c.derived("a", &band), "a(f1)");
        // The demodulator shares the f count with band select, as in SA.
        assert_eq!(c.demod(), "(f2)");
        assert_eq!(c.derived("a", &band), "a(f3)");
        assert_eq!(c.derived("a", &Edit::Acf { start: 0, end: 1 }), "a_acf(0)");
        assert_eq!(c.derived("a", &Edit::Acf { start: 0, end: 1 }), "a_acf(1)");
        assert_eq!(c.derived("a", &Edit::Ccf { start: 0, end: 1 }), "a_ccf(0)");
        assert_eq!(
            c.derived("a", &Edit::Resample { rate: 13_333.0 }),
            "a(r_13333)"
        );
        assert_eq!(
            c.derived("a", &Edit::Resample { rate: 7777.77 }),
            "a(r_7777,77)"
        );
        assert_eq!(
            c.derived("a", &Edit::Resample { rate: 7777.5 }),
            "a(r_7777,5)"
        );
        assert_eq!(window_title("test(0)(c1)"), "[TempDir] test(0)(c1).wav");
    }

    #[test]
    fn sa_rs_tags_cover_the_other_operations() {
        let mut c = Counters::default();
        let cases: [(Edit, &str); 6] = [
            (Edit::BandReject { lo: 1.0, hi: 2.0 }, "(rj1)"),
            (Edit::Shift { hz: 5.0 }, "(s1)"),
            (
                Edit::ShiftToBase {
                    lo: 1.0,
                    hi: 2.0,
                    inverted: false,
                },
                "(sb1)",
            ),
            (
                Edit::ShiftToBase {
                    lo: 1.0,
                    hi: 2.0,
                    inverted: true,
                },
                "(sbi1)",
            ),
            (
                Edit::BaudCorrection {
                    existing: 1.0,
                    required: 1.1,
                },
                "(brc1)",
            ),
            (Edit::Lowpass { cutoff: 100.0 }, "(lpf1)"),
        ];
        for (edit, tag) in cases {
            assert_eq!(c.derived("", &edit), tag, "{edit:?}");
        }
    }
}
