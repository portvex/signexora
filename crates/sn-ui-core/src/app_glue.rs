//! Pure helpers of the application glue: the questions asked before work is
//! thrown away, lists of names, measurement values with units, and the
//! choices made when results arrive.

use sn_core::{Measurement, MeasurementContext, MeasurementKind, NodeId, Signal};
use sn_dsp::ops::Edit;
use sn_dsp::overview::Overview;

use crate::format;
use crate::t;

/// `names` as a list in the interface language: "A", "A and B", "A, B and C".
pub fn join_names<S: AsRef<str>>(names: &[S]) -> String {
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.as_ref().to_owned(),
        Some((last, rest)) => {
            let rest: Vec<&str> = rest.iter().map(AsRef::as_ref).collect();
            t!(
                "app-list-and",
                rest = rest.join(&t!("app-list-separator")),
                last = last.as_ref()
            )
        }
    }
}

/// A question waiting for the user before work would be thrown away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirm {
    /// Open a session file.
    OpenSession,
    /// Run a script.
    RunScript,
    /// Windows > `CloseAllWindows`: empties the session.
    CloseAll,
    /// Quit the app.
    Quit,
    /// Closing this child window would stop its tool job. The window id is kept:
    /// windows can open, close or be replaced (a session load) while it waits.
    CloseWindow(u64),
}

/// Title, question and go-button text of the "save first?" question for `c`;
/// `None` for a question asked by its own window.
pub fn confirm_texts(c: Confirm, unsaved: bool) -> Option<(String, String, String)> {
    let texts = match c {
        Confirm::OpenSession => (
            t!("app-confirm-open-title"),
            t!("app-confirm-open-question"),
            t!("app-confirm-replace"),
        ),
        Confirm::RunScript => (
            t!("app-confirm-script-title"),
            t!("app-confirm-script-question"),
            t!("app-confirm-replace"),
        ),
        Confirm::CloseAll => (
            t!("app-confirm-close-all-title"),
            t!("app-confirm-close-all-question"),
            t!("app-confirm-close-all"),
        ),
        Confirm::Quit if unsaved => (
            t!("app-confirm-quit-title"),
            t!("app-confirm-quit-unsaved"),
            t!("app-confirm-quit-without-saving"),
        ),
        Confirm::Quit => (
            t!("app-confirm-quit-title"),
            t!("app-confirm-quit-writing"),
            t!("app-confirm-quit-anyway"),
        ),
        // Asked by the window itself (the front end's `confirm_close_window`).
        Confirm::CloseWindow(_) => return None,
    };
    Some(texts)
}

/// Waveform offset and scale that show the whole of `signal`, from its overview.
pub fn fit_range(signal: &Signal, ov: &Overview) -> Option<(f64, f32)> {
    let env = ov.span(signal, 0, signal.len());
    let mut m = env.re;
    if let Some(q) = env.im {
        m = m.merge(q);
    }
    if m.is_empty() {
        return None;
    }
    let (lo, hi) = (f64::from(m.min), f64::from(m.max));
    Some((
        f64::midpoint(lo, hi),
        (((hi - lo) / 2.0) * 1.05).max(1e-6) as f32,
    ))
}

/// Stable ids of the tools that record measurements, stored in the session.
pub use sn_session::tool_id;

/// The shown name of the tool that recorded a measurement. Sessions written
/// before the ids were introduced hold a free text name, shown as it is.
pub fn tool_label(tool: &str) -> String {
    let key = format!("measure-tool-{tool}");
    let text = crate::i18n::tr(&key);
    if text == key { tool.to_owned() } else { text }
}

/// The uncertainty and tool id to record an ADP candidate with (shared with
/// scripts, so both record the same).
pub use sn_session::tool_id::adp_quality;

/// Record a measurement of `kind` made on `node` (shown as `node_name`) by the
/// tool with id `tool`. Returns the status line that says so, or the message
/// for the alert box when the value cannot be recorded (not finite, say).
pub fn record_measurement(
    measures: &mut MeasurementContext,
    kind: &MeasurementKind,
    value: f64,
    uncertainty: Option<f64>,
    node: NodeId,
    node_name: &str,
    tool: &str,
) -> Result<String, String> {
    let m = Measurement {
        value,
        uncertainty,
        source: Some(node),
        tool: tool.to_owned(),
    };
    match measures.record(kind.clone(), m) {
        Ok(()) => {
            let value = with_unit(kind, value);
            Ok(if let Some(u) = uncertainty {
                t!(
                    "app-recorded-uncertain",
                    kind = kind.to_string(),
                    value = value,
                    uncertainty = with_unit(kind, u),
                    node = node_name,
                    tool = tool_label(tool)
                )
            } else {
                t!(
                    "app-recorded",
                    kind = kind.to_string(),
                    value = value,
                    node = node_name,
                    tool = tool_label(tool)
                )
            })
        }
        Err(e) => Err(t!(
            "app-not-recorded",
            kind = kind.to_string(),
            error = e.to_string()
        )),
    }
}

/// Edits whose output keeps the input's amplitude, so the parent's waveform scale and
/// offset still fit the child.
pub fn keeps_amplitude(edit: &Edit) -> bool {
    matches!(
        edit,
        Edit::Copy { .. }
            | Edit::Shift { .. }
            | Edit::ShiftToBase { .. }
            | Edit::Invert
            | Edit::InvertBand { .. }
            | Edit::Resample { .. }
            | Edit::BaudCorrection { .. }
            | Edit::BandSelect { .. }
            | Edit::BandReject { .. }
            | Edit::AreaDelete { .. }
            | Edit::Lowpass { .. }
            | Edit::Median { .. }
            | Edit::Rrc { .. }
    )
}

/// Unit suffix for a measurement value in messages.
pub fn with_unit(kind: &MeasurementKind, value: f64) -> String {
    match kind {
        MeasurementKind::BaudRate => format!("{} Bd", format::rate(value)),
        MeasurementKind::Carrier | MeasurementKind::Spacing => {
            format!("{} Hz", format::rate(value))
        }
        MeasurementKind::Period => format::time_long(value),
        MeasurementKind::OfdmK | MeasurementKind::Custom(_) => format::rate(value),
    }
}

/// The guide's line under a step saying what was measured and where, or that
/// nothing was. Both front ends show it; they pick the colour themselves.
pub fn measured_line(
    kind: &MeasurementKind,
    value: Option<&crate::edits::MeasuredValue>,
) -> String {
    // Br, Sc and dT stay untranslated, so the line needs no message of its own.
    value.map_or_else(
        || t!("guide-not-measured", kind = kind.to_string()),
        |v| format!("{kind}: {} ({})", with_unit(kind, v.value), v.source()),
    )
}

/// The measurement the guide's step `i` (counted from 0) relies on: Br under
/// step 4, Sc under step 5 and dT under step 6.
pub fn guide_step_measure(i: usize) -> Option<MeasurementKind> {
    match i {
        3 => Some(MeasurementKind::BaudRate),
        4 => Some(MeasurementKind::Carrier),
        5 => Some(MeasurementKind::Period),
        _ => None,
    }
}

/// How the guide's line under a step is drawn. Each front end maps it to its
/// own colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasuredTone {
    /// Nothing measured yet: dim, a secondary note.
    Missing,
    /// A value to rely on: the normal text colour.
    Measured,
    /// A value from another frequency axis: the warning colour, as the
    /// parameter dialogs show it.
    OtherAxis,
}

/// The tone of the guide's line for `value` (see [`measured_line`]).
pub fn measured_tone(value: Option<&crate::edits::MeasuredValue>) -> MeasuredTone {
    match value {
        None => MeasuredTone::Missing,
        Some(v) if v.other_axis => MeasuredTone::OtherAxis,
        Some(_) => MeasuredTone::Measured,
    }
}

/// Accuracy of tones read from a histogram: twice its bin width or the smoothing
/// width, whichever is larger (both in Hz).
pub fn tone_tolerance(bin_width: f64, smooth: f64) -> f64 {
    (2.0 * bin_width).max(smooth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_listed_in_the_interface_language() {
        assert_eq!(join_names::<&str>(&[]), "");
        assert_eq!(join_names(&["OFDM"]), "OFDM");
        assert_eq!(join_names(&["Demodulator", "OFDM"]), "Demodulator and OFDM");
        assert_eq!(
            join_names(&["Demodulator", "PhasePlane", "OFDM"]),
            "Demodulator, PhasePlane and OFDM"
        );
    }

    #[test]
    fn guide_steps_show_their_measurements_in_the_right_tone() {
        assert_eq!(guide_step_measure(3), Some(MeasurementKind::BaudRate));
        assert_eq!(guide_step_measure(4), Some(MeasurementKind::Carrier));
        assert_eq!(guide_step_measure(5), Some(MeasurementKind::Period));
        assert_eq!(guide_step_measure(0), None);
        assert_eq!(guide_step_measure(6), None);
        let mut v = crate::edits::MeasuredValue {
            value: 1500.0,
            from: "W1, measure line".into(),
            other_axis: false,
        };
        assert_eq!(measured_tone(None), MeasuredTone::Missing);
        assert_eq!(measured_tone(Some(&v)), MeasuredTone::Measured);
        v.other_axis = true;
        assert_eq!(measured_tone(Some(&v)), MeasuredTone::OtherAxis);
    }

    #[test]
    fn measurements_record_a_tool_id_and_show_its_name() {
        // A session from before the ids holds free text: shown as it is.
        assert_eq!(tool_label("ADP, coarse"), "ADP, coarse");
        // Ids never show a raw Fluent key.
        for id in tool_id::ALL {
            assert!(!tool_label(id).starts_with("measure-tool-"), "{id}");
        }
    }

    #[test]
    fn recording_says_what_was_recorded_or_why_not() {
        let mut measures = MeasurementContext::default();
        let node = NodeId::from_raw(3);
        let kind = MeasurementKind::BaudRate;
        let ok =
            record_measurement(&mut measures, &kind, 75.0, None, node, "n3", tool_id::ADP).unwrap();
        assert!(
            ok.contains("n3") && ok.contains(&tool_label(tool_id::ADP)),
            "{ok}"
        );
        assert_eq!(
            measures.latest(&kind).map(|m| m.tool.as_str()),
            Some(tool_id::ADP)
        );
        let bad = record_measurement(&mut measures, &kind, f64::NAN, None, node, "n3", "x");
        assert!(bad.is_err());
    }
}
