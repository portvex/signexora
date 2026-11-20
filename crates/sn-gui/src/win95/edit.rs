//! Single-line edit fields and spin edits (an edit with an up-down control).
//!
//! [`EditState`] holds the text, caret and selection and implements the
//! Windows 95 editing keys without any drawing, so it is tested directly.

use std::rc::Rc;

use gpui::{
    AnyElement, ClipboardItem, ContentMask, Context, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Styled, Window, canvas, px,
};

use super::button::{Changed, Ctl};
use super::colors::{Role, Scheme, scheme};
use super::geom::{Edge, Rect};
use super::glyph::{self, Mode};
use super::kit::{Action, Capture, Host, KeyHandler, Kit, Lens, action, start_repeat};
use super::marks::{self, Dir};
use super::paint;
use super::text::{self, LINE};
use super::widget::{Slot, hovered, place, size_of};

/// Text left margin inside the sunken border.
const MARGIN: i32 = 4;

/// The text, caret and selection of an edit field. Positions are byte indices
/// on character boundaries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EditState {
    pub(crate) text: String,
    /// Where the caret is (the moving end of the selection).
    pub(crate) caret: usize,
    /// The fixed end of the selection.
    pub(crate) anchor: usize,
    /// Horizontal scroll of the text, in pixels.
    pub(crate) scroll: i32,
    /// Most characters allowed, if limited.
    pub(crate) max_chars: Option<usize>,
    pub(crate) read_only: bool,
    /// Colours an application set itself (SA's navy fields), instead of the
    /// scheme's window colours.
    pub(crate) look: Option<FieldLook>,
    /// Pixels at the right that something else covers (a spin edit's
    /// buttons): the text and caret scroll to stay left of them.
    pub(crate) pad_right: i32,
}

/// An edit field's own colours and whether it draws its sunken border.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldLook {
    pub(crate) bg: super::colors::Rgb,
    pub(crate) fg: super::colors::Rgb,
    /// Draw the sunken border (off when the field sits inside another frame).
    pub(crate) border: bool,
}

/// What a key did to an edit field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditKey {
    /// Not an editing key.
    Ignored,
    /// The caret or selection moved.
    Moved,
    /// The text changed.
    Changed,
    /// Copy this text to the clipboard.
    Copy(String),
    /// Copy this text to the clipboard; the selection was deleted.
    Cut(String),
    /// Paste from the clipboard.
    Paste,
}

impl EditState {
    /// A field showing `text` with the caret at the end.
    pub(crate) fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            caret: text.len(),
            anchor: text.len(),
            ..Self::default()
        }
    }

    /// The selection as `(start, end)`.
    pub(crate) fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    pub(crate) fn has_selection(&self) -> bool {
        self.caret != self.anchor
    }

    /// The selected text.
    pub(crate) fn selected(&self) -> &str {
        let (a, b) = self.selection();
        self.text.get(a..b).unwrap_or("")
    }

    /// Replace the text and put the caret at the end; the field's limits,
    /// look and padding stay.
    pub(crate) fn set_text(&mut self, text: &str) {
        *self = Self {
            max_chars: self.max_chars,
            read_only: self.read_only,
            look: self.look,
            pad_right: self.pad_right,
            ..Self::new(text)
        };
    }

    pub(crate) fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.text.len();
    }

    /// Move the caret to `pos`, extending the selection with `extend`.
    pub(crate) fn move_to(&mut self, pos: usize, extend: bool) {
        self.caret = pos.min(self.text.len());
        if !self.text.is_char_boundary(self.caret) {
            self.caret = self.prev(self.caret);
        }
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// The character boundary before `i`.
    fn prev(&self, i: usize) -> usize {
        self.text
            .get(..i)
            .and_then(|s| s.char_indices().next_back())
            .map_or(0, |(j, _)| j)
    }

    /// The character boundary after `i`.
    fn next(&self, i: usize) -> usize {
        self.text
            .get(i..)
            .and_then(|s| s.chars().next())
            .map_or(self.text.len(), |c| i + c.len_utf8())
    }

    /// The start of the word before `i` (Ctrl+Left, Ctrl+Backspace): back
    /// over spaces and punctuation, then over the word.
    pub(crate) fn prev_word(&self, i: usize) -> usize {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let before = self.text.get(..i).unwrap_or("");
        let mut it = before.char_indices().rev().peekable();
        while it.next_if(|&(_, c)| !is_word(c)).is_some() {}
        let mut start = it.peek().map_or(0, |&(j, _)| j);
        for (j, c) in it {
            if !is_word(c) {
                break;
            }
            start = j;
        }
        start.min(i)
    }

    /// The start of the next word after `i` (Ctrl+Right, Ctrl+Delete): over
    /// the rest of this word, then over spaces and punctuation.
    pub(crate) fn next_word(&self, i: usize) -> usize {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let after = self.text.get(i..).unwrap_or("");
        let mut it = after.char_indices().peekable();
        while it.next_if(|&(_, c)| is_word(c)).is_some() {}
        while it.next_if(|&(_, c)| !is_word(c)).is_some() {}
        it.peek().map_or(self.text.len(), |&(j, _)| i + j)
    }

    /// Delete from the caret to `to` (unless there is a selection, which goes
    /// instead).
    fn delete_to(&mut self, to: usize) -> bool {
        if self.read_only {
            return false;
        }
        if !self.has_selection() {
            if to == self.caret {
                return false;
            }
            self.anchor = to;
        }
        self.insert("") || self.has_selection()
    }

    /// The word around `i` (letters, digits and `_`), for double clicks.
    pub(crate) fn word_at(&self, i: usize) -> (usize, usize) {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let before = self.text.get(..i).unwrap_or("");
        let after = self.text.get(i..).unwrap_or("");
        let start = before
            .char_indices()
            .rev()
            .take_while(|&(_, c)| is_word(c))
            .last()
            .map_or(i, |(j, _)| j);
        let end = after
            .char_indices()
            .find(|&(_, c)| !is_word(c))
            .map_or(self.text.len(), |(j, _)| i + j);
        (start, end)
    }

    /// Insert `s` in place of the selection. Returns whether the text changed.
    pub(crate) fn insert(&mut self, s: &str) -> bool {
        if self.read_only {
            return false;
        }
        let s: String = s.chars().filter(|c| !c.is_control()).collect();
        let (a, b) = self.selection();
        let kept = self.text.chars().count() - self.text.get(a..b).map_or(0, |t| t.chars().count());
        let room = self
            .max_chars
            .map_or(usize::MAX, |m| m.saturating_sub(kept));
        let s: String = s.chars().take(room).collect();
        if s.is_empty() && a == b {
            return false;
        }
        let head = self.text.get(..a).unwrap_or("");
        let tail = self.text.get(b..).unwrap_or("");
        self.text = format!("{head}{s}{tail}");
        self.move_to(a + s.len(), false);
        true
    }

    /// Delete the selection, or the character before (`back`) or after the caret.
    pub(crate) fn delete(&mut self, back: bool) -> bool {
        if self.read_only {
            return false;
        }
        if !self.has_selection() {
            let c = self.caret;
            let other = if back { self.prev(c) } else { self.next(c) };
            if other == c {
                return false;
            }
            self.anchor = other;
        }
        self.insert("") || self.has_selection()
    }

    /// Apply a key. `key` is GPUI's key name, `ctrl` is Ctrl or Cmd, `typed` the
    /// character the key types.
    pub(crate) fn key(
        &mut self,
        key: &str,
        ctrl: bool,
        shift: bool,
        typed: Option<&str>,
    ) -> EditKey {
        let moved = |this: &mut Self, pos: usize| {
            this.move_to(pos, shift);
            EditKey::Moved
        };
        match (key, ctrl) {
            // Ctrl: by words, as a Windows 95 edit control.
            ("left", true) => moved(self, self.prev_word(self.caret)),
            ("right", true) => moved(self, self.next_word(self.caret)),
            ("backspace", true) => changed(self.delete_to(self.prev_word(self.caret))),
            ("delete", true) if !shift => changed(self.delete_to(self.next_word(self.caret))),
            ("left", _) if self.has_selection() && !shift => {
                let (a, _) = self.selection();
                moved(self, a)
            }
            ("right", _) if self.has_selection() && !shift => {
                let (_, b) = self.selection();
                moved(self, b)
            }
            ("left", _) => moved(self, self.prev(self.caret)),
            ("right", _) => moved(self, self.next(self.caret)),
            ("home", _) => moved(self, 0),
            ("end", _) => moved(self, self.text.len()),
            ("backspace", _) => changed(self.delete(true)),
            ("delete", _) if shift => self.cut(),
            ("delete", _) => changed(self.delete(false)),
            ("insert" | "c", true) => EditKey::Copy(self.selected().to_owned()),
            ("insert", false) if shift => EditKey::Paste,
            ("v", true) => EditKey::Paste,
            ("x", true) => self.cut(),
            ("a", true) => {
                self.select_all();
                EditKey::Moved
            }
            (_, true) | ("enter" | "escape" | "tab" | "up" | "down", _) => EditKey::Ignored,
            _ => match typed {
                Some(t) if !t.is_empty() && !t.chars().any(char::is_control) => {
                    changed(self.insert(t))
                }
                _ => EditKey::Ignored,
            },
        }
    }

    fn cut(&mut self) -> EditKey {
        let s = self.selected().to_owned();
        if self.read_only || s.is_empty() {
            return EditKey::Copy(s);
        }
        self.insert("");
        EditKey::Cut(s)
    }

    /// Scroll so that the caret at `caret_x` (pixels from the text start) is
    /// visible in a view `view_w` pixels wide.
    pub(crate) fn scroll_to_caret(&mut self, caret_x: i32, view_w: i32) {
        let view_w = view_w.max(1);
        if caret_x < self.scroll {
            self.scroll = (caret_x - view_w / 4).max(0);
        } else if caret_x >= self.scroll + view_w {
            self.scroll = caret_x - view_w * 3 / 4;
        }
    }
}

fn changed(yes: bool) -> EditKey {
    if yes {
        EditKey::Changed
    } else {
        EditKey::Moved
    }
}

/// The caret position in pixels from the start of the text.
fn caret_x(window: &Window, st: &EditState) -> i32 {
    let line = text::shape(window, &st.text, false, gpui::black());
    f32::from(line.x_for_index(st.caret)).round() as i32
}

/// The character position under the local x coordinate `x`.
fn index_at(window: &Window, st: &EditState, x: i32) -> usize {
    let line = text::shape(window, &st.text, false, gpui::black());
    line.closest_index_for_x(px((x - MARGIN + st.scroll) as f32))
}

/// Keep the caret visible after a change.
fn fix_scroll(window: &Window, st: &mut EditState, slot: &Slot) {
    let w = slot.width().unwrap_or(100) - 2 * MARGIN - st.pad_right;
    let x = caret_x(window, st);
    st.scroll_to_caret(x, w);
}

/// The key handler of an edit field.
fn edit_keys<V: Host>(
    lens: Lens<V, EditState>,
    slot: Slot,
    on_change: Option<Action<V>>,
) -> KeyHandler<V> {
    Rc::new(move |v, ev: &KeyDownEvent, window, cx| {
        let k = &ev.keystroke;
        let ctrl = k.modifiers.control || k.modifiers.platform;
        let Some(st) = lens(v) else {
            return false;
        };
        let r = st.key(&k.key, ctrl, k.modifiers.shift, k.key_char.as_deref());
        let changed = match &r {
            EditKey::Ignored => return false,
            EditKey::Moved => false,
            EditKey::Changed => true,
            EditKey::Copy(s) => {
                cx.write_to_clipboard(ClipboardItem::new_string(s.clone()));
                false
            }
            EditKey::Cut(s) => {
                cx.write_to_clipboard(ClipboardItem::new_string(s.clone()));
                true
            }
            // Only a paste reads the clipboard (another process's data).
            EditKey::Paste => {
                let paste = cx
                    .read_from_clipboard()
                    .and_then(|c| c.text())
                    .unwrap_or_default();
                st.insert(paste.lines().next().unwrap_or(""))
            }
        };
        fix_scroll(window, st, &slot);
        if changed && let Some(f) = &on_change {
            f(v, window, cx);
        }
        cx.notify();
        true
    })
}

/// Paint an edit field's border, background, text, selection and caret.
fn paint_edit(
    window: &mut Window,
    cx: &mut gpui::App,
    b: gpui::Bounds<gpui::Pixels>,
    st: &EditState,
    focused: bool,
    enabled: bool,
    s: &Scheme,
) {
    let (w, h) = size_of(b);
    let r = Rect::new(0, 0, w, h);
    let bg = if enabled && !st.read_only {
        Role::Window
    } else {
        Role::Face
    };
    let (bg, mut ink, border) = match st.look {
        Some(l) => (
            super::colors::hsla(l.bg),
            super::colors::hsla(l.fg),
            l.border,
        ),
        None => (s.of(bg), s.of(Role::Text), true),
    };
    paint::fill(window, b.origin, r, bg);
    if border {
        paint::edge(window, b.origin, r, Edge::Sunken, s);
    }
    let text_area = r.inset(2);
    let inner = paint::bounds(
        b.origin,
        Rect::new(
            text_area.x,
            text_area.y,
            (text_area.w - st.pad_right).max(0),
            text_area.h,
        ),
    );
    let ty = (h - LINE) / 2;
    let tx = MARGIN - st.scroll;
    if !enabled {
        ink = s.of(Role::Shadow);
    }
    window.with_content_mask(Some(ContentMask { bounds: inner }), |window| {
        let line = text::shape(window, &st.text, false, ink);
        line.paint(paint::at(b.origin, tx, ty), px(LINE as f32), window, cx)
            .ok();
        if !focused {
            return;
        }
        let x_of = |i: usize| f32::from(line.x_for_index(i)).round() as i32;
        if st.has_selection() {
            let (a, e) = st.selection();
            let sel = Rect::new(tx + x_of(a), ty, x_of(e) - x_of(a), LINE);
            paint::fill(window, b.origin, sel, s.of(Role::Selection));
            let mask = paint::bounds(b.origin, sel).intersect(&inner);
            window.with_content_mask(Some(ContentMask { bounds: mask }), |window| {
                let white = text::shape(window, &st.text, false, s.of(Role::SelectionText));
                white
                    .paint(paint::at(b.origin, tx, ty), px(LINE as f32), window, cx)
                    .ok();
            });
        }
        let caret = Rect::new(tx + x_of(st.caret), ty, 1, LINE);
        paint::fill(window, b.origin, caret, ink);
    });
}

/// A single-line edit field. `st` is the state to show, `lens` reaches the same
/// state in the view.
pub(crate) fn edit<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    st: &EditState,
    lens: Lens<V, EditState>,
    on_change: Option<Action<V>>,
) -> AnyElement {
    edit_with_keys(kit, cx, c, st, lens, on_change, None)
}

/// An edit field whose key handler first offers keys to `pre`.
pub(crate) fn edit_with_keys<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    st: &EditState,
    lens: Lens<V, EditState>,
    on_change: Option<Action<V>>,
    pre: Option<KeyHandler<V>>,
) -> AnyElement {
    let s = scheme(cx);
    let focused = kit.has_focus(c.id);
    let slot = Slot::default();
    if c.enabled {
        let keys = edit_keys(lens.clone(), slot.clone(), on_change);
        let keys: KeyHandler<V> = match pre {
            Some(pre) => Rc::new(move |v, ev, w, cx| pre(v, ev, w, cx) || keys(v, ev, w, cx)),
            None => keys,
        };
        kit.focusable(c.id, c.scope, Some(keys));
    }
    let shown = st.clone();
    let enabled = c.enabled;
    let paint_slot = slot.clone();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            paint_slot.set(b);
            paint_edit(window, cx, b, &shown, focused, enabled, &s);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let el = place(c.rect)
        .id(gpui::ElementId::Name(c.id.to_owned().into()))
        .cursor_text()
        .child(paint);
    if !c.enabled {
        return el.into_any_element();
    }
    let id = c.id.to_owned();
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut V, ev: &MouseDownEvent, window, cx| {
            edit_mouse_down(v, ev, window, &id, &lens, &slot);
            cx.notify();
        }),
    )
    .into_any_element()
}

/// Mouse down in an edit field: focus, place the caret, start a drag selection.
fn edit_mouse_down<V: Host>(
    v: &mut V,
    ev: &MouseDownEvent,
    window: &mut Window,
    id: &str,
    lens: &Lens<V, EditState>,
    slot: &Slot,
) {
    v.kit().focus = Some(id.to_owned().into());
    let Some((x, _)) = slot.local(ev.position) else {
        return;
    };
    let Some(st) = lens(v) else {
        return;
    };
    let i = index_at(window, st, x);
    if ev.click_count >= 2 {
        let (a, e) = st.word_at(i);
        st.anchor = a;
        st.caret = e;
    } else {
        st.move_to(i, ev.modifiers.shift);
    }
    fix_scroll(window, st, slot);
    let (lens, slot) = (lens.clone(), slot.clone());
    v.kit().capture(Capture {
        on_move: Some(Rc::new(move |v, p, window, _| {
            if let (Some((x, _)), Some(st)) = (slot.local(p), lens(v)) {
                let i = index_at(window, st, x);
                st.move_to(i, true);
                fix_scroll(window, st, &slot);
            }
        })),
        on_up: None,
    });
}

/// A number with limits, shown in an edit with up and down buttons.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpinState {
    pub(crate) edit: EditState,
    pub(crate) value: i64,
    pub(crate) min: i64,
    pub(crate) max: i64,
    pub(crate) step: i64,
}

impl SpinState {
    pub(crate) fn new(value: i64, min: i64, max: i64, step: i64) -> Self {
        let mut s = Self {
            edit: EditState {
                pad_right: SPIN_BUTTONS_W,
                ..EditState::default()
            },
            value,
            min,
            max,
            step,
        };
        s.set(value);
        s
    }

    /// Set the value (clamped) and show it.
    pub(crate) fn set(&mut self, v: i64) {
        self.value = v.clamp(self.min, self.max.max(self.min));
        self.edit.set_text(&self.value.to_string());
        self.edit.select_all();
    }

    /// Step up (`n > 0`) or down by `n` steps. Returns whether the value
    /// changed (not at a limit).
    pub(crate) fn step_by(&mut self, n: i64) -> bool {
        let before = self.value;
        self.commit();
        self.set(self.value.saturating_add(n.saturating_mul(self.step)));
        self.value != before
    }

    /// The value the field holds now: the typed number (clamped) when it is
    /// one, else the last value. Readers use it so that a number typed
    /// without Enter counts, as Delphi's spin edit takes the text when the
    /// focus leaves it.
    pub(crate) fn current(&self) -> i64 {
        self.edit
            .text
            .trim()
            .parse::<i64>()
            .map_or(self.value, |v| v.clamp(self.min, self.max.max(self.min)))
    }

    /// Take the typed number, if it is one; otherwise keep the old value (the
    /// field shows it again, all selected). Returns whether the value changed.
    pub(crate) fn commit(&mut self) -> bool {
        let before = self.value;
        self.set(self.current());
        self.value != before
    }
}

/// Width a spin edit's buttons cover at the right of its field (with the
/// gap to the border).
const SPIN_BUTTONS_W: i32 = 18;

/// A spin edit: an edit field with up and down buttons at its right.
pub(crate) fn spin_edit<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    st: &SpinState,
    lens: &Lens<V, SpinState>,
    on_change: Option<Changed<V, i64>>,
) -> AnyElement {
    let s = scheme(cx);
    let notify =
        move |v: &mut V, w: &mut Window, cx: &mut Context<'_, V>, l: &Lens<V, SpinState>| {
            if let (Some(f), Some(val)) = (&on_change, l(v).map(|st| st.value)) {
                f(v, val, w, cx);
            }
        };
    let notify = Rc::new(notify);
    let stepper = |n: i64| -> Action<V> {
        let (lens, notify) = (lens.clone(), notify.clone());
        action(move |v, w, cx| {
            // At a limit a held button repeats without a change: no notice.
            if lens(v).is_some_and(|st| st.step_by(n)) {
                notify(v, w, cx, &lens);
            }
        })
    };
    let (up, down) = (stepper(1), stepper(-1));
    let pre: KeyHandler<V> = {
        let (up, down, lens, notify) = (up.clone(), down.clone(), lens.clone(), notify.clone());
        Rc::new(move |v, ev, w, cx| match ev.keystroke.key.as_str() {
            "up" => {
                up(v, w, cx);
                true
            }
            "down" => {
                down(v, w, cx);
                true
            }
            "enter" => {
                if lens(v).is_some_and(SpinState::commit) {
                    notify(v, w, cx, &lens);
                }
                false
            }
            _ => false,
        })
    };
    let edit_lens: Lens<V, EditState> = {
        let lens = lens.clone();
        Rc::new(move |v| lens(v).map(|s| &mut s.edit))
    };
    let field_rect = Rect::new(0, 0, c.rect.w, c.rect.h);
    let field = edit_with_keys(
        kit,
        cx,
        Ctl {
            rect: field_rect,
            ..c
        },
        &st.edit,
        edit_lens,
        None,
        Some(pre),
    );
    let bw = SPIN_BUTTONS_W - 2;
    let half = (c.rect.h - 4) / 2;
    let up_r = Rect::new(c.rect.w - 2 - bw, 2, bw, half);
    let down_r = Rect::new(c.rect.w - 2 - bw, 2 + half, bw, c.rect.h - 4 - half);
    let up_id = format!("{}.up", c.id);
    let down_id = format!("{}.down", c.id);
    let ub = spin_button(kit, cx, &up_id, up_r, Dir::Up, c.enabled, up, &s);
    let db = spin_button(kit, cx, &down_id, down_r, Dir::Down, c.enabled, down, &s);
    place(c.rect)
        .child(field)
        .child(ub)
        .child(db)
        .into_any_element()
}

/// One half of an up-down control: held down, it repeats.
fn spin_button<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    id: &str,
    r: Rect,
    dir: Dir,
    enabled: bool,
    step: Action<V>,
    s: &Scheme,
) -> AnyElement {
    let held = kit.is_pressed(id);
    let s = *s;
    let mark = marks::arrow(dir, 5);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let down = held && hovered(window, b);
            let r = Rect::new(0, 0, w, h);
            let e = if down { Edge::ThinSunken } else { Edge::Button };
            paint::panel(window, b.origin, r, e, &s);
            let d = i32::from(down);
            let mode = if enabled {
                Mode::Normal
            } else {
                Mode::Disabled
            };
            glyph::draw(
                window,
                cx,
                b.origin,
                ((w - mark.w) / 2 + d, (h - mark.h) / 2 + d),
                &mark,
                mode,
                &s,
            );
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let el = place(r)
        .id(gpui::ElementId::Name(id.to_owned().into()))
        .child(paint);
    if !enabled {
        return el.into_any_element();
    }
    let id = id.to_owned();
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut V, _, window, cx| {
            v.kit().press(&id, None);
            start_repeat(v, &id, step.clone(), window, cx);
            cx.notify();
        }),
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(st: &mut EditState, s: &str) {
        for c in s.chars() {
            let t = c.to_string();
            st.key(&t, false, false, Some(&t));
        }
    }

    #[test]
    fn typing_and_deleting() {
        let mut st = EditState::default();
        typed(&mut st, "hello");
        assert_eq!(st.text, "hello");
        assert_eq!(st.caret, 5);
        assert_eq!(st.key("backspace", false, false, None), EditKey::Changed);
        assert_eq!(st.text, "hell");
        st.key("home", false, false, None);
        assert_eq!(st.key("delete", false, false, None), EditKey::Changed);
        assert_eq!(st.text, "ell");
        assert_eq!(
            st.key("backspace", false, false, None),
            EditKey::Moved,
            "nothing before the caret"
        );
    }

    #[test]
    fn selection_with_shift_and_replace() {
        let mut st = EditState::new("abcdef");
        st.key("home", false, false, None);
        st.key("right", false, true, None);
        st.key("right", false, true, None);
        assert_eq!(st.selected(), "ab");
        typed(&mut st, "X");
        assert_eq!(st.text, "Xcdef");
        st.key("a", true, false, Some("a"));
        assert_eq!(st.selected(), "Xcdef");
        st.key("left", false, false, None);
        assert_eq!((st.caret, st.anchor), (0, 0), "Left collapses to the start");
    }

    #[test]
    fn clipboard_keys() {
        let mut st = EditState::new("copy me");
        st.select_all();
        assert_eq!(
            st.key("c", true, false, Some("c")),
            EditKey::Copy("copy me".into())
        );
        assert_eq!(
            st.key("x", true, false, Some("x")),
            EditKey::Cut("copy me".into())
        );
        assert_eq!(st.text, "");
        assert_eq!(st.key("v", true, false, Some("v")), EditKey::Paste);
        assert_eq!(st.key("enter", false, false, None), EditKey::Ignored);
        assert_eq!(st.key("q", true, false, Some("q")), EditKey::Ignored);
    }

    #[test]
    fn multibyte_text_keeps_char_boundaries() {
        let mut st = EditState::new("\u{430}\u{431}\u{432}");
        assert_eq!(st.caret, 6);
        st.key("left", false, false, None);
        assert_eq!(st.caret, 4);
        st.key("backspace", false, false, None);
        assert_eq!(st.text, "\u{430}\u{432}");
        st.move_to(1, false);
        assert_eq!(st.caret, 0, "inside a character snaps back");
    }

    #[test]
    fn limits_and_read_only() {
        let mut st = EditState {
            max_chars: Some(3),
            ..EditState::default()
        };
        typed(&mut st, "abcdef");
        assert_eq!(st.text, "abc");
        st.read_only = true;
        assert!(!st.insert("z"));
        assert!(!st.delete(true));
        assert_eq!(
            st.key("x", true, false, Some("x")),
            EditKey::Copy(String::new())
        );
    }

    #[test]
    fn words_and_scrolling() {
        let st = EditState::new("one two_3 four");
        assert_eq!(st.word_at(5), (4, 9));
        assert_eq!(st.word_at(0), (0, 3));
        assert_eq!(st.word_at(14), (10, 14));
        let mut st = EditState::new("x");
        st.scroll_to_caret(150, 100);
        assert_eq!(st.scroll, 75);
        st.scroll_to_caret(80, 100);
        assert_eq!(st.scroll, 75, "visible: no change");
        st.scroll_to_caret(10, 100);
        assert_eq!(st.scroll, 0);
    }

    #[test]
    fn spin_steps_and_clamps() {
        let mut sp = SpinState::new(5, 0, 10, 2);
        assert_eq!(sp.edit.text, "5");
        sp.step_by(1);
        assert_eq!(sp.value, 7);
        sp.step_by(5);
        assert_eq!(sp.value, 10);
        sp.edit.set_text("-4");
        sp.commit();
        assert_eq!(sp.value, 0);
        sp.edit.set_text("abc");
        assert!(!sp.commit());
        assert_eq!((sp.value, sp.edit.text.as_str()), (0, "0"));
        assert!(!sp.step_by(-1), "at the limit nothing changes");
        assert!(sp.step_by(1));
    }

    #[test]
    fn typed_spin_text_counts_without_enter() {
        let mut sp = SpinState::new(2, 1, 64, 1);
        sp.edit.set_text("12");
        assert_eq!(sp.current(), 12, "focus left without Enter");
        sp.edit.set_text("999");
        assert_eq!(sp.current(), 64, "clamped");
        sp.edit.set_text("x");
        assert_eq!(sp.current(), 2, "not a number: the last value");
        assert!(sp.commit() || sp.value == 2);
        assert_eq!(sp.edit.selection(), (0, 1), "shown again, all selected");
    }

    #[test]
    fn set_text_keeps_the_fields_settings() {
        let look = FieldLook {
            bg: 0x00_0080,
            fg: 0xFF_FFFF,
            border: false,
        };
        let mut st = EditState {
            max_chars: Some(4),
            read_only: true,
            look: Some(look),
            pad_right: 18,
            ..EditState::new("a")
        };
        st.set_text("bc");
        assert_eq!(st.text, "bc");
        assert_eq!(
            (st.max_chars, st.read_only, st.look, st.pad_right),
            (Some(4), true, Some(look), 18)
        );
    }

    #[test]
    fn ctrl_moves_and_deletes_by_words() {
        let mut st = EditState::new("one two, three");
        st.key("left", true, false, None);
        assert_eq!(st.caret, 9, "start of three");
        st.key("left", true, false, None);
        assert_eq!(st.caret, 4, "start of two");
        st.key("right", true, false, None);
        assert_eq!(st.caret, 9, "start of the next word");
        st.key("end", false, false, None);
        assert_eq!(st.key("backspace", true, false, None), EditKey::Changed);
        assert_eq!(st.text, "one two, ");
        st.key("home", false, false, None);
        assert_eq!(st.key("delete", true, false, None), EditKey::Changed);
        assert_eq!(st.text, "two, ");
        assert_eq!(st.prev_word(0), 0);
        assert_eq!(st.next_word(st.text.len()), st.text.len());
    }
}
