//! SA's dialogs and message boxes on the Windows 95 kit: the error box, the
//! question before an ACF or CCF, the "save first?" question, About, and the
//! parameter dialogs of the operations that need values (`sn-ui-core::edits`).

use std::collections::VecDeque;

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use sn_core::NodeId;
use sn_dsp::ops::Edit;
use sn_ui_core::app_glue::Confirm;
use sn_ui_core::edits::{LPF_PRESETS, Note, ParamKind};
use sn_ui_core::{format, t};

use super::{Frame, frame_title};
use crate::win95::button::{Ctl, push_button};
use crate::win95::colors::{hsla, palette, rgb_of, sa_colors};
use crate::win95::combo::{ComboState, combo_box};
use crate::win95::dialog::{
    Answer, BUTTON, BUTTON_GAP, Buttons, DialogHooks, MARGIN, MsgBox, dialog_size, modal_dialog,
};
use crate::win95::edit::{EditState, edit};
use crate::win95::geom::Rect;
use crate::win95::kit::{action, lens};
use crate::win95::label::{Align, label};
use crate::win95::marks::Icon;
use crate::win95::text::{self, LINE};
use crate::win95::widget::place;

/// The program name in message box title bars (SA: `Signals Analyzer`).
pub(crate) const BOX_TITLE: &str = "SigNexora";

/// What answering the open message box does.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum MsgAction {
    /// Nothing (an error box, About).
    #[default]
    None,
    /// SA's question before an ACF or CCF: `OK` runs it.
    Correlation {
        node: NodeId,
        edit: Edit,
        caller: Option<u32>,
    },
    /// The "save first?" question.
    Confirm(Confirm),
    /// Closing this window stops its tool job: `OK` closes it.
    CloseWindow(u32),
    /// A parameter dialog's value was refused: the field gets the focus back.
    ParamField(usize),
    /// The settings could not be saved on the way out: quit once answered.
    Quit,
    /// A replay or script finished after newer work was done: `OK` replaces
    /// the session with the held result.
    ApplyHeld,
}

/// The answer of a platform file chooser: `Ok(Some(path))` chosen,
/// `Ok(None)` cancelled by the user, `Err` the chooser could not be shown
/// (its channel closed, or the platform failed), with the reason.
pub(crate) fn chooser_answer<T, E: std::fmt::Display, C: std::fmt::Display>(
    r: Result<Result<Option<T>, E>, C>,
) -> Result<Option<T>, String> {
    match r {
        Ok(Ok(p)) => Ok(p),
        Ok(Err(e)) => Err(e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Message boxes waiting behind the open one.
pub(crate) type MsgQueue = VecDeque<(MsgBox, MsgAction)>;

/// Tab scope of the parameter dialog.
pub(crate) const PARAMS_SCOPE: &str = "params";
/// Height of a field row.
const ROW: i32 = 27;
/// Width of an edit field.
const FIELD_W: i32 = 96;

/// An operation waiting for its values (SA's small modal dialog).
#[derive(Clone, Debug)]
pub(crate) struct ParamDialog {
    pub(crate) node: NodeId,
    /// The window it was opened from (its result opens offset from it).
    pub(crate) caller: Option<u32>,
    /// The window name, for the title.
    label: String,
    pub(crate) kind: ParamKind,
    /// Where pre-filled values came from.
    notes: Vec<Note>,
    /// The signal's sample rate, for the ranges.
    fs: f64,
    pub(crate) fields: Vec<EditState>,
    /// The low-pass cutoff's preset list.
    presets: ComboState,
    pub(crate) rect: Rect,
    pub(crate) outline: Option<Rect>,
    /// Centred on its first frame.
    placed: bool,
}

impl ParamDialog {
    fn new(
        node: NodeId,
        caller: Option<u32>,
        label: String,
        kind: ParamKind,
        notes: Vec<Note>,
        fs: f64,
    ) -> Self {
        let fields = kind.texts().iter().map(|t| EditState::new(t)).collect();
        let presets: Vec<String> = LPF_PRESETS
            .into_iter()
            .filter(|p| *p < fs / 2.0)
            .map(|p| format::dec_trim(p, 0))
            .collect();
        let refs: Vec<&str> = presets.iter().map(String::as_str).collect();
        Self {
            node,
            caller,
            label,
            kind,
            notes,
            fs,
            fields,
            presets: ComboState::new(&refs, None),
            rect: Rect::default(),
            outline: None,
            placed: false,
        }
    }

    /// The typed texts.
    pub(crate) fn texts(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.text.clone()).collect()
    }

    /// The dialog's title: `SA - <operation>: <window>`.
    fn title(&self) -> String {
        t!(
            "edits-dialog-title",
            operation = self.kind.title(),
            signal = self.label.as_str()
        )
    }

    /// The width of the third column: the green `Max delta change` of
    /// Correction of BR, the preset list of the low-pass cutoff.
    fn extra_w(&self, window: &Window) -> i32 {
        match self.kind {
            ParamKind::Baud { .. } => text::width(window, &t!("sw-brc-max-delta"), true) + 8,
            ParamKind::Lowpass { .. } => 80 + 8,
            _ => 0,
        }
    }

    /// Client size and the label column's width.
    fn layout(&self, window: &Window) -> ((i32, i32), i32) {
        let label_w = self
            .kind
            .labels()
            .iter()
            .map(|l| text::width(window, l, false))
            .max()
            .unwrap_or(0)
            + 8;
        let notes_w = self
            .notes
            .iter()
            .map(|n| text::width(window, &n.text, false))
            .max()
            .unwrap_or(0);
        let w = (MARGIN + label_w + FIELD_W + 8 + self.extra_w(window) + MARGIN)
            .max(notes_w + 2 * MARGIN)
            .max(320);
        let rows = self.fields.len() as i32;
        let h = MARGIN + rows * ROW + self.notes.len() as i32 * (LINE + 2) + 8 + BUTTON.1 + MARGIN;
        ((w, h), label_w)
    }
}

impl Frame {
    /// Show `mb`; `action` says what its answer does. A box already open keeps
    /// its place and this one waits behind it.
    pub(crate) fn ask(&mut self, mb: MsgBox, action: MsgAction) {
        if self.msgbox.is_some() {
            self.msg_queue.push_back((mb, action));
            return;
        }
        self.msg_action = action;
        self.show_message(mb);
    }

    /// SA's error box: the program name, a red X, the text and `OK`.
    pub(crate) fn alert(&mut self, text: &str) {
        let mb = MsgBox::new(BOX_TITLE, text, Some(Icon::Error), Buttons::Ok);
        self.ask(mb, MsgAction::None);
    }

    /// SA 6.2.4.2's warning question before an ACF or CCF.
    pub(crate) fn ask_correlation(
        &mut self,
        node: NodeId,
        edit: Edit,
        caller: Option<u32>,
        text: &str,
    ) {
        let mb = MsgBox::new(BOX_TITLE, text, Some(Icon::Warning), Buttons::OkCancel);
        self.ask(mb, MsgAction::Correlation { node, edit, caller });
    }

    /// Forget questions about signals of a replaced session.
    pub(crate) fn drop_node_questions(&mut self) {
        let about_node =
            |a: &MsgAction| matches!(a, MsgAction::Correlation { .. } | MsgAction::CloseWindow(_));
        self.msg_queue.retain(|(_, a)| !about_node(a));
        if about_node(&self.msg_action) {
            self.msgbox = None;
            self.msg_action = MsgAction::None;
            self.next_message();
        }
    }

    /// The open box was answered with `a`.
    pub(crate) fn answer(&mut self, a: Answer, cx: &mut Context<'_, Self>) {
        self.msgbox = None;
        self.kit.focus = None;
        let act = std::mem::take(&mut self.msg_action);
        match act {
            MsgAction::None => {}
            MsgAction::Correlation { node, edit, caller } => {
                let live = self.graph.get(node).is_some() && !self.discarded.contains(&node);
                if a == Answer::Ok && live {
                    self.run_edit(node, edit, caller);
                }
            }
            MsgAction::Confirm(c) => self.confirm_answer(c, a, cx),
            MsgAction::CloseWindow(w) => {
                if a == Answer::Ok {
                    self.close_window_now(w);
                }
            }
            MsgAction::ApplyHeld => self.held_answer(a == Answer::Ok),
            MsgAction::Quit => {
                cx.quit();
                return;
            }
            MsgAction::ParamField(i) => {
                if let Some(f) = self.params.as_mut().and_then(|p| p.fields.get_mut(i)) {
                    f.select_all();
                    self.kit.focus = Some(param_field_id(i).into());
                }
            }
        }
        if self.loader.is_some() && self.kit.focus.is_none() {
            self.kit.focus = Some(super::loader::view::LIST_ID.into());
        }
        self.next_message();
    }

    /// A "save first?" question with nothing at stake any more (the save or
    /// the file write finished meanwhile) closes and its action goes ahead, as
    /// `sa-app`'s question re-checks itself every frame.
    pub(crate) fn recheck_confirm(&mut self, cx: &mut Context<'_, Self>) {
        let MsgAction::Confirm(c) = self.msg_action else {
            return;
        };
        if self.msgbox.is_none() || matches!(c, Confirm::CloseWindow(_)) || self.must_ask(c) {
            return;
        }
        self.msgbox = None;
        self.msg_action = MsgAction::None;
        self.kit.focus = None;
        self.do_confirmed(c, cx);
        self.next_message();
    }

    /// Show the next waiting box, if any.
    fn next_message(&mut self) {
        if self.msgbox.is_none()
            && let Some((mb, a)) = self.msg_queue.pop_front()
        {
            self.msg_action = a;
            self.show_message(mb);
        }
    }

    /// Help > About: the version, licence and credits.
    pub(crate) fn about(&mut self) {
        let mut lines = vec![
            frame_title(),
            String::new(),
            t!("about-text"),
            String::new(),
            t!("about-licence", licence = env!("CARGO_PKG_LICENSE")),
            String::new(),
            t!("about-credits"),
        ];
        lines.extend([
            t!("about-credit-sa"),
            t!("gui-about-credit-gpui"),
            t!("gui-about-credit-w95fa"),
            t!("gui-about-credit-selawik"),
            t!("gui-about-credit-linux-fonts"),
            t!("about-credit-noto"),
            t!("about-credit-crates"),
        ]);
        let mb = MsgBox::new(
            &t!("about-title"),
            &lines.join("\n"),
            Some(Icon::App),
            Buttons::Ok,
        );
        self.ask(mb, MsgAction::None);
    }

    /// Open the parameter dialog of `kind` for the signal of window `win`
    /// (the API the analysis windows use for operations that need values).
    pub(crate) fn open_params(&mut self, win: u32, kind: ParamKind, notes: Vec<Note>) {
        let Some(node) = self.node_of(win) else {
            return;
        };
        let Some(fs) = self.graph.get(node).map(|n| n.signal.sample_rate()) else {
            return;
        };
        let label = self.name_of(node);
        self.params = Some(ParamDialog::new(node, Some(win), label, kind, notes, fs));
        self.kit.focus = Some(param_field_id(0).into());
        if let Some(f) = self.params.as_mut().and_then(|p| p.fields.first_mut()) {
            f.select_all();
        }
    }

    /// `Ok` (or Enter) in the parameter dialog: run the operation, or SA's
    /// `Please check settings!` box naming the field and its range.
    pub(crate) fn params_ok(&mut self) {
        let Some(d) = self.params.as_ref() else {
            return;
        };
        match d.kind.parse(&d.texts(), d.fs) {
            Ok(kind) => {
                let (node, caller) = (d.node, d.caller);
                self.params = None;
                self.kit.focus = None;
                let caller = caller.filter(|c| self.mdi.get(*c).is_some());
                self.start_edit_on(node, kind.edit(), caller);
            }
            Err((field, why)) => {
                let check = t!("sw-check-settings");
                let text = if why.starts_with(&check) {
                    why
                } else {
                    format!("{check}\n{why}")
                };
                let mb = MsgBox::new(BOX_TITLE, &text, Some(Icon::Error), Buttons::Ok);
                self.ask(mb, MsgAction::ParamField(field));
            }
        }
    }

    /// `Cancel`, Escape or the close button of the parameter dialog.
    pub(crate) fn params_cancel(&mut self) {
        self.params = None;
        self.kit.focus = None;
    }

    /// The parameter dialog over the frame.
    pub(crate) fn params_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
        vw: i32,
        vh: i32,
    ) -> Option<AnyElement> {
        // A dialog over a signal that no longer exists is closed.
        if self
            .params
            .as_ref()
            .is_some_and(|d| self.graph.get(d.node).is_none() || self.discarded.contains(&d.node))
        {
            self.params = None;
        }
        let d = self.params.as_mut()?;
        let ((cw, ch), label_w) = d.layout(window);
        if !d.placed {
            let (w, h) = dialog_size(cw, ch);
            d.rect = Rect::new(((vw - w) / 2).max(0), ((vh - h) / 2).max(0), w, h);
            d.placed = true;
        }
        let d = d.clone();
        let mut body = div().relative().size_full();
        for el in self.param_rows(&d, label_w, cx) {
            body = body.child(el);
        }
        let notes_y = MARGIN + d.fields.len() as i32 * ROW;
        let warn = hsla(rgb_of(palette(cx).warn));
        for (i, n) in (0_i32..).zip(&d.notes) {
            let r = Rect::new(MARGIN, notes_y + i * (LINE + 2), cw - 2 * MARGIN, LINE);
            let el = if n.warn {
                colored_label(r, &n.text, warn, false)
            } else {
                label(cx, r, &n.text.replace('&', "&&"), false, true, Align::Left)
            };
            body = body.child(el);
        }
        for el in self.param_buttons(&d, cw, ch, cx) {
            body = body.child(el);
        }
        let hooks = DialogHooks {
            rect: lens(|v: &mut Self| v.params.as_mut().map(|p| &mut p.rect)),
            outline: lens(|v: &mut Self| v.params.as_mut().map(|p| &mut p.outline)),
            on_close: Some(action(|v: &mut Self, _, _| v.params_cancel())),
        };
        Some(modal_dialog(
            &self.kit,
            cx,
            "params",
            &d.title(),
            d.rect,
            d.outline,
            body.into_any_element(),
            &hooks,
        ))
    }

    fn param_rows(
        &mut self,
        d: &ParamDialog,
        label_w: i32,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::new();
        let labels = d.kind.labels();
        let x_field = MARGIN + label_w;
        for (i, st) in d.fields.iter().enumerate() {
            let y = MARGIN + i as i32 * ROW;
            let caption = labels.get(i).cloned().unwrap_or_default();
            out.push(label(
                cx,
                Rect::new(MARGIN, y, label_w - 4, 21),
                &caption.replace('&', "&&"),
                false,
                true,
                Align::Left,
            ));
            let id = param_field_id(i);
            out.push(edit(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: PARAMS_SCOPE,
                    rect: Rect::new(x_field, y, FIELD_W, 21),
                    label: "",
                    enabled: true,
                },
                st,
                lens(move |v: &mut Self| v.params.as_mut().and_then(|p| p.fields.get_mut(i))),
                None,
            ));
            let x_extra = x_field + FIELD_W + 8;
            match (&d.kind, i) {
                (ParamKind::Baud { .. }, 0) => {
                    let green = hsla(sa_colors(cx).green);
                    let r = Rect::new(x_extra, y + 4, 240, LINE);
                    out.push(colored_label(r, &t!("sw-brc-max-delta"), green, true));
                }
                (ParamKind::Lowpass { .. }, 0) if !d.presets.items.is_empty() => {
                    out.push(combo_box(
                        &mut self.kit,
                        cx,
                        Ctl {
                            id: "pd.presets",
                            scope: PARAMS_SCOPE,
                            rect: Rect::new(x_extra, y, 80, 21),
                            label: "",
                            enabled: true,
                        },
                        &d.presets,
                        lens(|v: &mut Self| v.params.as_mut().map(|p| &mut p.presets)),
                        Some(std::rc::Rc::new(|v: &mut Self, i: usize, _, _| {
                            if let Some(p) = v.params.as_mut() {
                                let text = p.presets.items.get(i).cloned().unwrap_or_default();
                                if let Some(f) = p.fields.first_mut() {
                                    f.set_text(&text);
                                }
                            }
                        })),
                    ));
                }
                _ => {}
            }
        }
        out
    }

    fn param_buttons(
        &mut self,
        d: &ParamDialog,
        cw: i32,
        ch: i32,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let ok = if matches!(d.kind, ParamKind::Baud { .. }) {
            t!("sw-brc-yes")
        } else {
            t!("edits-ok")
        };
        let cancel = t!("edits-cancel");
        let y = ch - MARGIN - BUTTON.1;
        let x_cancel = cw - MARGIN - BUTTON.0;
        let x_ok = x_cancel - BUTTON_GAP - BUTTON.0;
        vec![
            push_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: "pd.ok",
                    scope: PARAMS_SCOPE,
                    rect: Rect::new(x_ok, y, BUTTON.0, BUTTON.1),
                    label: &ok,
                    enabled: true,
                },
                true,
                action(|v: &mut Self, _, _| v.params_ok()),
            ),
            push_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: "pd.cancel",
                    scope: PARAMS_SCOPE,
                    rect: Rect::new(x_cancel, y, BUTTON.0, BUTTON.1),
                    label: &cancel,
                    enabled: true,
                },
                false,
                action(|v: &mut Self, _, _| v.params_cancel()),
            ),
        ]
    }
}

/// The focus id of the parameter dialog's field `i`.
pub(crate) fn param_field_id(i: usize) -> String {
    format!("pd.f{i}")
}

/// A line of text in one colour (SA's green and warning texts).
pub(crate) fn colored_label(r: Rect, s: &str, color: gpui::Hsla, bold: bool) -> AnyElement {
    let s = s.to_owned();
    let paint = gpui::canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            text::draw(window, cx, b.origin, &s, bold, color);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_starts_with_the_kind_texts_and_lpf_presets_below_nyquist() {
        let d = ParamDialog::new(
            NodeId::from_raw(1),
            None,
            "a(0)".into(),
            ParamKind::Lowpass { cutoff: 300.0 },
            Vec::new(),
            1000.0,
        );
        assert_eq!(d.texts().len(), 1);
        assert!(
            d.presets
                .items
                .iter()
                .all(|p| p.parse::<f64>().unwrap() < 500.0)
        );
        assert!(d.title().contains("a(0)"));
    }
}
