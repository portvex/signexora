//! Modal dialogs and message boxes.
//!
//! A modal dialog is drawn inside the frame window over a layer that takes
//! every mouse click, so the frame and its MDI children cannot be used until
//! the dialog closes. The dialog moves by its caption with the drag outline.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Styled, Window, canvas, div,
};

use super::button::{Ctl, push_button};
use super::colors::scheme;
use super::geom::Rect;
use super::glyph::{self, Mode};
use super::kit::{Action, Capture, Host, Kit, Lens, action};
use super::marks::{self, Icon};
use super::mdi::caption_h;
use super::mdi_view::{caption, caption_button_of, window_frame};
use super::paint;
use super::skin::CapButton;
use super::text::{self, LINE};
use super::widget::{Slot, hovered, place, size_of};

/// Border of a dialog frame (`SM_CXDLGFRAME`).
pub(crate) fn dlg_frame() -> i32 {
    super::style::metrics().dlg_frame
}
/// Push button size in dialogs.
pub(crate) const BUTTON: (i32, i32) = (75, 23);
/// Margin inside message boxes.
pub(crate) const MARGIN: i32 = 12;
/// Gap between buttons.
pub(crate) const BUTTON_GAP: i32 = 6;

/// The client area of a dialog of size `w` x `h`, in its own coordinates.
pub(crate) fn dialog_client(w: i32, h: i32) -> Rect {
    Rect::new(
        dlg_frame(),
        dlg_frame() + caption_h(),
        w - 2 * dlg_frame(),
        h - 2 * dlg_frame() - caption_h(),
    )
}

/// The outer size of a dialog whose client area is `cw` x `ch`.
pub(crate) fn dialog_size(cw: i32, ch: i32) -> (i32, i32) {
    (cw + 2 * dlg_frame(), ch + 2 * dlg_frame() + caption_h())
}

/// The close button of a dialog `w` pixels wide. Motif has none: its
/// window-menu button at the left of the title bar closes the dialog
/// instead (Close is all a dialog's window menu would offer).
fn close_rect(w: i32) -> Rect {
    let close = super::mdi::caption_buttons_in(w, dlg_frame()).2;
    if !close.is_empty() {
        return close;
    }
    let cap = Rect::new(dlg_frame(), dlg_frame(), w - 2 * dlg_frame(), caption_h());
    Rect::new(cap.x, cap.y, super::mdi::icon_zone_w(cap.h), cap.h)
}

/// The width of a dialog's title in caption `cap` with its close button at
/// `close`: up to the close button, or the whole caption when the button is
/// at the left (Motif's window-menu button; macOS centres its own).
fn title_width(cap: Rect, close: Rect) -> i32 {
    if close.x < cap.x + cap.w / 2 {
        cap.w
    } else {
        close.x - cap.x - 2
    }
}

/// The host's state of one dialog.
pub(crate) struct DialogHooks<V> {
    /// The dialog's rectangle (window coordinates).
    pub(crate) rect: Lens<V, Rect>,
    /// The drag outline while the dialog is being moved.
    pub(crate) outline: Lens<V, Option<Rect>>,
    /// The close button (or Escape); `None` greys the close button out (a
    /// Yes/No message box, which Windows does not let close).
    pub(crate) on_close: Option<Action<V>>,
}

impl<V> Clone for DialogHooks<V> {
    fn clone(&self) -> Self {
        Self {
            rect: self.rect.clone(),
            outline: self.outline.clone(),
            on_close: self.on_close.clone(),
        }
    }
}

/// A modal dialog at `r` (window coordinates) over a layer that blocks the
/// frame. `content` is placed in the client area.
#[allow(clippy::too_many_arguments)]
pub(crate) fn modal_dialog<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    id: &str,
    title: &str,
    r: Rect,
    outline: Option<Rect>,
    content: AnyElement,
    hooks: &DialogHooks<V>,
) -> AnyElement {
    let s = scheme(cx);
    let held_close = kit.is_pressed(&format!("{id}.close"));
    let closable = hooks.on_close.is_some();
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let title = title.to_owned();
    let frame = canvas(super::widget::hover_hitbox, move |b, hitbox, window, cx| {
        paint_slot.set(b);
        let (w, h) = size_of(b);
        let o = b.origin;
        window_frame(window, o, w, h, dlg_frame(), true, true, &s);
        let cap = Rect::new(dlg_frame(), dlg_frame(), w - 2 * dlg_frame(), caption_h());
        let close = close_rect(w);
        caption(
            window,
            cx,
            o,
            cap,
            &title,
            (true, true),
            title_width(cap, close),
            &s,
        );
        let over = super::widget::hot(window, &hitbox, o, close);
        let st = super::skin::State {
            hot: over && closable,
            down: held_close && closable && hovered(window, b),
            focus: false,
            enabled: closable,
        };
        caption_button_of(window, cx, o, close, CapButton::Close, st, true, true, &s);
    })
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let body = place(dialog_client(r.w, r.h)).child(content);
    let id2 = id.to_owned();
    let h = hooks.clone();
    let dialog = place(r)
        .id(gpui::ElementId::Name(id.to_owned().into()))
        .occlude()
        .child(frame)
        .child(body)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |v: &mut V, ev: &MouseDownEvent, _, cx| {
                dialog_mouse_down(v, ev, &id2, &slot, &h);
                cx.notify();
            }),
        );
    let mut layer = div()
        .id("modal")
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .occlude()
        .child(dialog);
    if let Some(o) = outline {
        layer = layer.child(outline_layer(o));
    }
    layer.into_any_element()
}

/// The dotted outline of a dialog being moved.
fn outline_layer(o: Rect) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |b, (), window, _| paint::drag_frame(window, b.origin, o, dlg_frame()),
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}

/// Mouse down on a dialog: the close button, or a caption drag.
fn dialog_mouse_down<V: Host>(
    v: &mut V,
    ev: &MouseDownEvent,
    id: &str,
    slot: &Slot,
    hooks: &DialogHooks<V>,
) {
    let (Some((x, y)), Some(b)) = (slot.local(ev.position), slot.bounds()) else {
        return;
    };
    let (w, _) = size_of(b);
    let close = close_rect(w);
    if close.contains(x, y) {
        let Some(on_close) = hooks.on_close.clone() else {
            // Disabled: no pressed look, no action.
            return;
        };
        let slot = slot.clone();
        // Motif's window-menu button closes with a click, and the style's
        // hint teaches a double click: its second click must not land on
        // whatever was under the dialog.
        let menu_button = super::mdi::caption_buttons_in(w, dlg_frame()).2.is_empty();
        v.kit().press(
            &format!("{id}.close"),
            Some(Capture {
                on_move: None,
                on_up: Some(Rc::new(move |v, p, window, cx| {
                    let inside = slot.local(p).is_some_and(|(x, y)| close.contains(x, y));
                    if inside {
                        if menu_button {
                            v.kit().eat_double_click();
                        }
                        on_close(v, window, cx);
                    }
                })),
            }),
        );
        return;
    }
    let cap = Rect::new(dlg_frame(), dlg_frame(), w - 2 * dlg_frame(), caption_h());
    if !cap.contains(x, y) {
        return;
    }
    let Some(start) = (hooks.rect)(v).map(|r| *r) else {
        return;
    };
    let p0 = ev.position;
    let (h1, h2) = (hooks.clone(), hooks.clone());
    let at = move |p: gpui::Point<gpui::Pixels>| {
        let (dx, dy) = super::widget::pixel_delta(p0, p);
        start.offset(dx, dy)
    };
    v.kit().capture(Capture {
        on_move: Some(Rc::new(move |v, p, _, _| {
            if let Some(o) = (h1.outline)(v) {
                *o = Some(at(p));
            }
        })),
        on_up: Some(Rc::new(move |v, _, _, _| {
            let o = (h2.outline)(v).and_then(Option::take);
            if let (Some(o), Some(r)) = (o, (h2.rect)(v)) {
                *r = o;
            }
        })),
    });
}

/// Message box buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Buttons {
    Ok,
    OkCancel,
    YesNo,
    YesNoCancel,
}

/// A message box answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Ok,
    Cancel,
    Yes,
    No,
}

impl Buttons {
    /// The answers, left to right.
    pub(crate) fn answers(self) -> &'static [Answer] {
        match self {
            Self::Ok => &[Answer::Ok],
            Self::OkCancel => &[Answer::Ok, Answer::Cancel],
            Self::YesNo => &[Answer::Yes, Answer::No],
            Self::YesNoCancel => &[Answer::Yes, Answer::No, Answer::Cancel],
        }
    }

    /// What Escape (or the close button) answers, if anything: Cancel, or OK
    /// when OK is the only button. Yes/No boxes cannot be escaped.
    pub(crate) fn escape(self) -> Option<Answer> {
        match self {
            Self::Ok => Some(Answer::Ok),
            Self::OkCancel | Self::YesNoCancel => Some(Answer::Cancel),
            Self::YesNo => None,
        }
    }
}

impl Answer {
    /// The default caption of the answer's button (with its mnemonic).
    pub(crate) fn label(self) -> String {
        match self {
            Self::Ok => sn_ui_core::t!("dialog-ok"),
            Self::Cancel => sn_ui_core::t!("dialog-cancel"),
            Self::Yes => sn_ui_core::t!("gui-button-yes"),
            Self::No => sn_ui_core::t!("gui-button-no"),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Cancel => "cancel",
            Self::Yes => "yes",
            Self::No => "no",
        }
    }
}

/// Widest line of message box text before it wraps.
pub(crate) const MAX_TEXT_W: i32 = 440;

/// `text` broken into lines at `\n` and wrapped at word boundaries so that no
/// line is wider than `max_w` (a single long word stays on its own line).
pub(crate) fn wrap(text: &str, max_w: i32, measure: &dyn Fn(&str) -> i32) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && measure(&candidate) > max_w {
                out.push(std::mem::take(&mut line));
                word.clone_into(&mut line);
            } else {
                line = candidate;
            }
        }
        out.push(line);
    }
    out
}

/// A message box: title, text (lines split at `\n` and wrapped), icon and
/// buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MsgBox {
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) icon: Option<Icon>,
    pub(crate) buttons: Buttons,
    /// Button captions replacing the default ones (one per answer), for
    /// questions such as `Save session first` / `Close all` / `Cancel`.
    pub(crate) labels: Vec<String>,
    /// Where it is (window coordinates); set by [`MsgBox::place`].
    pub(crate) rect: Rect,
    /// The drag outline while it is moved.
    pub(crate) outline: Option<Rect>,
}

/// Layout of a message box's client area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MsgLayout {
    /// Client area size.
    pub(crate) size: (i32, i32),
    pub(crate) icon: Option<(i32, i32)>,
    pub(crate) text: (i32, i32),
    pub(crate) buttons: Vec<Rect>,
}

/// Lay out a message box whose text is `text_w` wide and `lines` lines high,
/// with buttons of the given widths.
pub(crate) fn msg_layout(text_w: i32, lines: i32, icon: bool, widths: &[i32]) -> MsgLayout {
    let n = widths.len() as i32;
    let row_w = widths.iter().sum::<i32>() + (n - 1).max(0) * BUTTON_GAP;
    let text_h = lines.max(1) * LINE;
    let text_x = if icon { MARGIN + 32 + MARGIN } else { MARGIN };
    let top_h = if icon { text_h.max(32) } else { text_h };
    let w = (text_x + text_w + MARGIN).max(row_w + 2 * MARGIN).max(120);
    let text_y = MARGIN + (top_h - text_h) / 2;
    let by = MARGIN + top_h + MARGIN + 4;
    let bx = (w - row_w) / 2;
    MsgLayout {
        size: (w, by + BUTTON.1 + MARGIN),
        icon: icon.then_some((MARGIN, MARGIN)),
        text: (text_x, text_y),
        buttons: widths
            .iter()
            .scan(bx, |x, w| {
                let r = Rect::new(*x, by, *w, BUTTON.1);
                *x += w + BUTTON_GAP;
                Some(r)
            })
            .collect(),
    }
}

impl MsgBox {
    pub(crate) fn new(title: &str, text: &str, icon: Option<Icon>, buttons: Buttons) -> Self {
        Self {
            title: title.to_owned(),
            text: text.to_owned(),
            icon,
            buttons,
            labels: Vec::new(),
            rect: Rect::default(),
            outline: None,
        }
    }

    /// The box with its own button captions (one per answer).
    pub(crate) fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// The caption of answer `a`'s button.
    pub(crate) fn label_of(&self, a: Answer) -> String {
        self.buttons
            .answers()
            .iter()
            .position(|x| *x == a)
            .and_then(|i| self.labels.get(i).cloned())
            .unwrap_or_else(|| a.label())
    }

    /// The text as it is drawn: wrapped lines.
    fn lines(&self, window: &Window) -> Vec<String> {
        wrap(&self.text, MAX_TEXT_W, &|l: &str| {
            text::width(window, l, false)
        })
    }

    fn layout(&self, window: &Window) -> MsgLayout {
        let lines = self.lines(window);
        let tw = lines
            .iter()
            .map(|l| text::width(window, l, false))
            .max()
            .unwrap_or(0);
        let widths: Vec<i32> = self
            .buttons
            .answers()
            .iter()
            .map(|a| {
                let caption = text::mnemonic(&self.label_of(*a)).text;
                (text::width(window, &caption, false) + 16).max(BUTTON.0)
            })
            .collect();
        msg_layout(tw, lines.len() as i32, self.icon.is_some(), &widths)
    }

    /// Centre the box in a `vw` x `vh` window.
    pub(crate) fn place(&mut self, window: &Window, vw: i32, vh: i32) {
        let (cw, ch) = self.layout(window).size;
        let (w, h) = dialog_size(cw, ch);
        self.rect = Rect::new(((vw - w) / 2).max(0), ((vh - h) / 2).max(0), w, h);
    }

    /// The focus id of an answer's button.
    pub(crate) fn button_id(a: Answer) -> String {
        format!("msgbox.{}", a.key())
    }
}

/// Tab scope of message box buttons.
pub(crate) const MSG_SCOPE: &str = "msgbox";

/// A modal message box. `lens` reaches the box in the view; `on_answer`
/// receives the button pressed (the host closes the box).
pub(crate) fn message_box<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    window: &Window,
    mb: &MsgBox,
    lens: Lens<V, MsgBox>,
    on_answer: super::kit::Event<V, Answer>,
) -> AnyElement {
    let lay = mb.layout(window);
    let mut body = div().relative().size_full();
    if let (Some(k), Some((x, y))) = (mb.icon, lay.icon) {
        body = body.child(icon_element(cx, k, x, y));
    }
    let (tx, ty) = lay.text;
    let lines = mb.lines(window);
    let label_r = Rect::new(tx, ty, lay.size.0 - tx, lines.len() as i32 * LINE);
    body = body.child(super::label::label(
        cx,
        label_r,
        &lines.join("\n").replace('&', "&&"),
        false,
        true,
        super::label::Align::Left,
    ));
    let answers = mb.buttons.answers();
    for (a, r) in answers.iter().zip(&lay.buttons) {
        let (a, f) = (*a, on_answer.clone());
        let id = MsgBox::button_id(a);
        let caption = mb.label_of(a);
        let c = Ctl {
            id: &id,
            scope: MSG_SCOPE,
            rect: *r,
            label: &caption,
            enabled: true,
        };
        body = body.child(push_button(
            kit,
            cx,
            c,
            false,
            action(move |v, w, cx| f(v, a, w, cx)),
        ));
    }
    let close: Option<Action<V>> = mb.buttons.escape().map(|a| {
        let f: Action<V> = Rc::new(move |v, w, cx| on_answer(v, a, w, cx));
        f
    });
    let (l1, l2) = (lens.clone(), lens);
    let hooks = DialogHooks {
        rect: Rc::new(move |v| l1(v).map(|m| &mut m.rect)),
        outline: Rc::new(move |v| l2(v).map(|m| &mut m.outline)),
        on_close: close,
    };
    modal_dialog(
        kit,
        cx,
        "msgbox",
        &mb.title,
        mb.rect,
        mb.outline,
        body.into_any_element(),
        &hooks,
    )
}

/// A 32x32 message-box icon at `(x, y)`.
fn icon_element(cx: &App, kind: Icon, x: i32, y: i32) -> AnyElement {
    let s = scheme(cx);
    let g = marks::icon(kind);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            if let Some(g) = &g {
                glyph::draw(window, cx, b.origin, (0, 0), g, Mode::Normal, &s);
            } else {
                crate::brand::paint(window, b.origin, 0, 0, 32);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(Rect::new(x, y, 32, 32))
        .child(paint)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_motif_dialog_closes_from_its_window_menu_button() {
        use sn_ui_core::theme::ThemeChoice;
        use sn_ui_core::winstyle::{Look, WinStyle};
        super::super::style::set(
            Look {
                style: WinStyle::LinuxMotif,
                ..Look::default()
            },
            ThemeChoice::Classic,
        );
        let w = 300;
        let cap = Rect::new(dlg_frame(), dlg_frame(), w - 2 * dlg_frame(), caption_h());
        let close = close_rect(w);
        // The drawn window-menu button, all of it, is the close control.
        assert_eq!(close, Rect::new(cap.x, cap.y, cap.h, cap.h));
        assert!(close.contains(cap.x + cap.h / 2, cap.y + cap.h / 2));
        assert_eq!(title_width(cap, close), cap.w, "the title keeps its room");
        super::super::style::set(Look::WIN95, ThemeChoice::Classic);
        let close = close_rect(w);
        assert!(close.x > w / 2, "Windows' close at the right: {close:?}");
        let cap = Rect::new(dlg_frame(), dlg_frame(), w - 2 * dlg_frame(), caption_h());
        assert_eq!(title_width(cap, close), close.x - cap.x - 2);
    }

    #[test]
    fn message_box_layout() {
        let l = msg_layout(200, 1, true, &[75, 75]);
        assert_eq!(l.icon, Some((12, 12)));
        assert_eq!(l.text, (56, 12 + (32 - 13) / 2));
        assert_eq!(l.size.0, 56 + 200 + 12);
        assert_eq!(l.buttons.len(), 2);
        let row = l.buttons[1].right() - l.buttons[0].x;
        assert_eq!(row, 2 * 75 + 6);
        assert_eq!(l.buttons[0].x, (l.size.0 - row) / 2, "buttons centred");
        assert_eq!(l.size.1, l.buttons[0].bottom() + 12);
        let wide = msg_layout(10, 3, false, &[75, 75, 75]);
        assert_eq!(
            wide.size.0,
            3 * 75 + 2 * 6 + 24,
            "the button row sets the width"
        );
        assert_eq!(wide.text, (12, 12));
        assert_eq!(msg_layout(0, 0, false, &[75]).size.0, 120);
        let sized = msg_layout(10, 1, false, &[120, 75]);
        assert_eq!(sized.buttons[1].x, sized.buttons[0].x + 120 + 6);
    }

    #[test]
    fn answers_and_escape() {
        assert_eq!(
            Buttons::YesNoCancel.answers(),
            &[Answer::Yes, Answer::No, Answer::Cancel]
        );
        assert_eq!(Buttons::Ok.escape(), Some(Answer::Ok));
        assert_eq!(Buttons::YesNo.escape(), None);
        assert_eq!(Buttons::OkCancel.escape(), Some(Answer::Cancel));
        assert_eq!(dialog_size(100, 50), (106, 74));
        assert_eq!(dialog_client(106, 74), Rect::new(3, 21, 100, 50));
        assert_eq!(MsgBox::button_id(Answer::No), "msgbox.no");
    }

    #[test]
    fn text_wraps_at_words() {
        let measure = |s: &str| s.chars().count() as i32 * 10;
        assert_eq!(
            wrap("aaa bbb ccc\ndd", 75, &measure),
            vec!["aaa bbb", "ccc", "dd"]
        );
        assert_eq!(
            wrap("toolongword x", 30, &measure),
            vec!["toolongword", "x"]
        );
        assert_eq!(wrap("", 30, &measure), vec![""]);
    }

    #[test]
    fn custom_labels_replace_the_defaults() {
        let mb = MsgBox::new("t", "x", None, Buttons::YesNoCancel).with_labels(vec![
            "Save".into(),
            "Close all".into(),
            "Cancel".into(),
        ]);
        assert_eq!(mb.label_of(Answer::No), "Close all");
        let plain = MsgBox::new("t", "x", None, Buttons::Ok);
        assert_eq!(plain.label_of(Answer::Ok), "OK");
    }
}
