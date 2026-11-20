//! File > Demo...: the workflow guide, SA's analysis method as a checklist of
//! steps with buttons that open the tools (`sa-app`'s `guide.rs`; it replaces
//! SA's demo player). The buttons act on the signal of the topmost signal
//! window, which the guide names.

use gpui::{AnyElement, Context, Window};
use sn_core::{MeasurementKind, NodeId};
use sn_dsp::ops::Edit;
use sn_ui_core::app_glue::{MeasuredTone, guide_step_measure, measured_tone};
use sn_ui_core::edits::MeasuredValue;
use sn_ui_core::shortcuts::Command;
use sn_ui_core::t;

use super::form::{Builder, Tone, measurer};
use super::{Btn, Tool};
use crate::app::Frame;
use crate::app::model::WinKind;
use crate::win95::geom::Rect;

/// What a guide button asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GuideAction {
    /// `SA Open file`: the one button that needs no signal.
    OpenRecording,
    Open(WinKind),
    AmDetector,
    ModifiedAm,
    PhaseDetector,
    Power(u32),
    Adp,
}

/// The steps: the Fluent key of each title; its `.text` attribute explains it.
pub(crate) const STEPS: [&str; 6] = [
    "guide-step-record",
    "guide-step-quality",
    "guide-step-isolate",
    "guide-step-rate",
    "guide-step-family",
    "guide-step-structure",
];

/// The buttons of step `i`: action, caption, tooltip.
pub(crate) fn step_buttons(i: usize) -> Vec<(GuideAction, String, String)> {
    let open = t!("guide-open-view");
    let new = t!("guide-new-signal");
    match i {
        0 => vec![(
            GuideAction::OpenRecording,
            t!("menu-open-file"),
            t!(
                "app-empty-open",
                key = super::help::key_of(Command::OpenRecording)
            ),
        )],
        1 => vec![(
            GuideAction::Open(WinKind::Waveform),
            t!("guide-btn-waveform"),
            open,
        )],
        2 => vec![(
            GuideAction::Open(WinKind::Sonogram),
            t!("guide-btn-sonogram"),
            open,
        )],
        3 => vec![
            (
                GuideAction::AmDetector,
                t!("guide-btn-am-detector"),
                new.clone(),
            ),
            (GuideAction::ModifiedAm, t!("guide-btn-modified-am"), new),
            (
                GuideAction::Adp,
                t!("guide-btn-adp"),
                t!("guide-btn-adp.hover"),
            ),
        ],
        4 => {
            let mut v = vec![
                (
                    GuideAction::PhaseDetector,
                    t!("guide-btn-phase-detector"),
                    new,
                ),
                (
                    GuideAction::Open(WinKind::Demod),
                    t!("guide-btn-demod"),
                    open.clone(),
                ),
            ];
            for n in [2_u32, 4, 8] {
                v.push((
                    GuideAction::Power(n),
                    t!("guide-btn-power", n = n.to_string()),
                    t!("guide-btn-power.hover", n = n.to_string()),
                ));
            }
            v.push((
                GuideAction::Open(WinKind::Phase),
                t!("guide-btn-phase-plane"),
                open,
            ));
            v.push((
                GuideAction::Open(WinKind::Ofdm),
                t!("guide-btn-ofdm"),
                t!("guide-btn-ofdm.hover"),
            ));
            v
        }
        5 => vec![(
            GuideAction::Open(WinKind::Raster),
            t!("guide-btn-raster"),
            open,
        )],
        _ => Vec::new(),
    }
}

/// The line under a step saying what was measured and where, or that nothing
/// was, with its tone (dim when missing, the warning colour when on another
/// axis, as in `sa-app`).
pub(crate) fn measured_line(
    kind: &MeasurementKind,
    value: Option<&MeasuredValue>,
) -> (String, Tone) {
    let tone = match measured_tone(value) {
        MeasuredTone::Missing => Tone::Dim,
        MeasuredTone::OtherAxis => Tone::Warn,
        MeasuredTone::Measured => Tone::Plain,
    };
    (sn_ui_core::app_glue::measured_line(kind, value), tone)
}

impl Frame {
    /// The signal the guide's buttons act on: that of the topmost signal
    /// window (the guide itself is often the active window).
    pub(crate) fn guide_target(&self) -> Option<(u32, NodeId)> {
        self.signal_windows()
            .last()
            .map(|(id, node, _)| (*id, *node))
    }

    /// A guide button was pressed.
    pub(crate) fn guide_action(&mut self, a: GuideAction) {
        if a == GuideAction::OpenRecording {
            self.open_loader(None);
            return;
        }
        let Some((caller, node)) = self.guide_target() else {
            return;
        };
        match a {
            GuideAction::OpenRecording => {}
            GuideAction::Open(kind) => {
                self.open_window(node, kind, Some(caller));
            }
            GuideAction::AmDetector => self.start_edit_on(node, Edit::AmDetector, Some(caller)),
            GuideAction::ModifiedAm => self.start_edit_on(node, Edit::ModifiedAm, Some(caller)),
            GuideAction::PhaseDetector => {
                self.start_edit_on(node, Edit::PhaseDetector, Some(caller));
            }
            GuideAction::Power(n) => self.start_edit_on(node, Edit::Power { n }, Some(caller)),
            GuideAction::Adp => self.start_adp(node),
        }
    }

    /// The measurement of `kind` shown under a step: Sc on the guide
    /// signal's frequency axis when there is one.
    fn guide_measured(
        &self,
        kind: &MeasurementKind,
        target: Option<NodeId>,
    ) -> Option<MeasuredValue> {
        match (kind, target) {
            (MeasurementKind::Carrier, Some(node)) => self.sc_for(node),
            _ => self.measured(kind),
        }
    }

    /// The guide window's client area.
    pub(crate) fn guide_body(
        &mut self,
        win: u32,
        cw: i32,
        ch: i32,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let target = self.guide_target();
        // The measuring keys act on the active window: name them only while
        // that is the guide's signal window.
        let keys_here = target.is_some_and(|(id, _)| self.mdi.active() == Some(id));
        let hint = |c: Command| {
            if keys_here {
                t!("guide-key", key = super::help::key_of(c))
            } else {
                String::new()
            }
        };
        let (br, sc) = (hint(Command::MeasureBr), hint(Command::MeasureSc));
        let measure = measurer(window);
        let mut b = Builder::new(&measure, cw - crate::win95::scrollbar::THICKNESS);
        match target {
            Some((_, node)) => {
                let name = self.name_of(node);
                b.line()
                    .text(&t!("guide-acts-on", signal = name), Tone::Bold);
            }
            None => b.wrapped(&t!("guide-no-signal"), Tone::Plain),
        }
        for (i, step) in STEPS.iter().enumerate() {
            b.line();
            b.line().text(&t!(step), Tone::Bold);
            if let Some(kind) = guide_step_measure(i) {
                let value = self.guide_measured(&kind, target.map(|(_, n)| n));
                let (line, tone) = measured_line(&kind, value.as_ref());
                b.wrapped(&line, tone);
            }
            let text = t!(&format!("{step}.text"), br = br.as_str(), sc = sc.as_str());
            b.wrapped(&text, Tone::Plain);
            let buttons = step_buttons(i);
            if !buttons.is_empty() {
                // Opening a recording needs no signal; every other button
                // acts on one.
                let r = b.flow().when(target.is_some() || i == 0);
                // A disabled button says what it lacks, as `sa-app` does.
                let no_signal = (target.is_none() && i != 0).then(|| t!("guide-no-signal"));
                for (a, caption, tip) in buttons {
                    let hint = no_signal.as_deref().unwrap_or(&tip);
                    r.small_button(Btn::Guide(a), &caption).hint(hint);
                }
            }
        }
        let form = b.finish();
        self.form_view(Tool::Guide, win, &form, Rect::new(0, 0, cw, ch), cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_has_an_english_title_and_text() {
        for step in STEPS {
            assert!(sn_ui_core::i18n::exists_in_english(step), "{step}");
            let text = format!("{step}.text");
            assert!(sn_ui_core::i18n::exists_in_english(&text), "{text}");
        }
        let n: usize = (0..STEPS.len()).map(|i| step_buttons(i).len()).sum();
        assert_eq!(n, 14, "sa-app's fourteen guide buttons");
    }

    #[test]
    fn measured_lines_take_the_tone_of_their_value() {
        let mut br = MeasuredValue {
            value: 75.0,
            from: "W2, adp".into(),
            other_axis: false,
        };
        let value = sn_ui_core::app_glue::with_unit(&MeasurementKind::BaudRate, 75.0);
        assert_eq!(
            measured_line(&MeasurementKind::BaudRate, Some(&br)),
            (format!("Br: {value} (W2, adp)"), Tone::Plain)
        );
        br.other_axis = true;
        assert_eq!(
            measured_line(&MeasurementKind::BaudRate, Some(&br)).1,
            Tone::Warn
        );
        assert_eq!(measured_line(&MeasurementKind::Carrier, None).1, Tone::Dim);
    }
}
