//! Drop-down list combo boxes (`CBS_DROPDOWNLIST`): a sunken field with the
//! arrow button; clicking opens a list below it.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, ParentElement, Styled, Window, anchored, canvas, deferred, px,
};

use super::button::{Changed, Ctl};
use super::colors::{Role, Scheme, scheme};
use super::geom::{Edge, Rect};
use super::glyph::{self, Mode};
use super::kit::{Host, KeyHandler, Kit, Lens};
use super::marks::{self, Dir};
use super::paint;
use super::scrollbar::{self, Bar, ScrollState, Wheel};
use super::text::{self, LINE};
use super::widget::{Slot, hot, hover_hitbox, hovered, place, size_of};

/// Height of a list item.
pub(crate) const ITEM: i32 = LINE;
/// Items shown before the list scrolls (Delphi's `DropDownCount`).
pub(crate) const DROP_COUNT: usize = 8;

/// The items and selection of a combo box, and the highlighted item and scroll
/// of its open list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ComboState {
    pub(crate) items: Vec<String>,
    pub(crate) selected: Option<usize>,
    /// The highlighted item of the open list.
    pub(crate) hot: usize,
    /// The first item shown in the open list.
    pub(crate) scroll: ScrollState,
    /// The wheel's part line over the open list.
    pub(crate) wheel: Wheel,
}

/// What a key did to a combo box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComboKey {
    Ignored,
    /// Handled, nothing to report.
    Handled,
    /// The selection changed.
    Changed,
    /// Open the list.
    Open,
    /// Close the list.
    Close,
}

impl ComboState {
    pub(crate) fn new(items: &[&str], selected: Option<usize>) -> Self {
        let items: Vec<String> = items.iter().map(|s| (*s).to_owned()).collect();
        let n = items.len() as i32;
        Self {
            selected: selected.filter(|&i| i < items.len()),
            hot: selected.unwrap_or(0),
            scroll: ScrollState::new(0, n - 1, DROP_COUNT as i32, 0),
            items,
            wheel: Wheel::default(),
        }
    }

    /// The text shown in the field.
    pub(crate) fn text(&self) -> &str {
        self.selected
            .and_then(|i| self.items.get(i))
            .map_or("", String::as_str)
    }

    /// Prepare the list for opening: highlight the selection and scroll to it.
    pub(crate) fn open(&mut self) {
        self.hot = self.selected.unwrap_or(0);
        self.show_hot();
    }

    /// Scroll the list so the highlighted item is visible.
    pub(crate) fn show_hot(&mut self) {
        let hot = self.hot as i32;
        let page = DROP_COUNT as i32;
        if hot < self.scroll.pos {
            self.scroll.set(hot);
        } else if hot >= self.scroll.pos + page {
            self.scroll.set(hot - page + 1);
        }
    }

    /// Select item `i`. Returns whether the selection changed.
    pub(crate) fn select(&mut self, i: usize) -> bool {
        if i >= self.items.len() || self.selected == Some(i) {
            return false;
        }
        self.selected = Some(i);
        self.hot = i;
        true
    }

    /// The first item after the current one starting with `c` (type-ahead).
    fn find_letter(&self, from: usize, c: char) -> Option<usize> {
        let n = self.items.len();
        let c = c.to_lowercase().next()?;
        (1..=n).map(|k| (from + k) % n).find(|&i| {
            self.items
                .get(i)
                .and_then(|s| s.chars().next())
                .and_then(|f| f.to_lowercase().next())
                == Some(c)
        })
    }

    /// Apply a key; `open` tells whether the list is open.
    pub(crate) fn key(
        &mut self,
        key: &str,
        alt: bool,
        open: bool,
        typed: Option<&str>,
    ) -> ComboKey {
        let n = self.items.len();
        if n == 0 {
            return ComboKey::Ignored;
        }
        // Closed with nothing selected: Down, PageDown and Home pick the
        // first item, and type-ahead checks the first item first (Windows).
        let none = !open && self.selected.is_none();
        let cur = if open {
            self.hot
        } else {
            self.selected.unwrap_or(0)
        };
        let target = match key {
            "down" | "up" if alt => {
                return if open {
                    ComboKey::Close
                } else {
                    ComboKey::Open
                };
            }
            "f4" => {
                return if open {
                    ComboKey::Close
                } else {
                    ComboKey::Open
                };
            }
            "enter" if open => {
                let changed = self.select(self.hot);
                return if changed {
                    ComboKey::Changed
                } else {
                    ComboKey::Close
                };
            }
            "escape" if open => return ComboKey::Close,
            "down" | "pagedown" if none => 0,
            "down" => (cur + 1).min(n - 1),
            "up" => cur.saturating_sub(1),
            "home" => 0,
            "end" => n - 1,
            "pagedown" => (cur + DROP_COUNT - 1).min(n - 1),
            "pageup" => cur.saturating_sub(DROP_COUNT - 1),
            _ => match typed
                .and_then(|t| t.chars().next())
                .filter(|c| !c.is_control())
            {
                Some(c) => match self.find_letter(if none { n - 1 } else { cur }, c) {
                    Some(i) => i,
                    None => return ComboKey::Handled,
                },
                None => return ComboKey::Ignored,
            },
        };
        if open {
            self.hot = target;
            self.show_hot();
            ComboKey::Handled
        } else if self.select(target) {
            ComboKey::Changed
        } else {
            ComboKey::Handled
        }
    }
}

/// A drop-down list combo box. `on_change` gets the new selection.
pub(crate) fn combo_box<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    st: &ComboState,
    lens: Lens<V, ComboState>,
    on_change: Option<Changed<V, usize>>,
) -> AnyElement {
    let s = scheme(cx);
    let id = c.id.to_owned();
    let open = kit.popup_is(c.id);
    let focused = kit.has_focus(c.id);
    if c.enabled {
        kit.focusable(
            c.id,
            c.scope,
            Some(combo_keys(id.clone(), lens.clone(), on_change.clone())),
        );
    }
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let shown = st.text().to_owned();
    let enabled = c.enabled;
    let held = kit.is_pressed(c.id);
    let paint = canvas(hover_hitbox, move |b, hitbox, window, cx| {
        paint_slot.set(b);
        let down = held && hovered(window, b);
        let (w, h) = size_of(b);
        let over = hot(window, &hitbox, b.origin, Rect::new(0, 0, w, h));
        let st = (focused && !open, enabled, down, over);
        paint_field(window, cx, b, &shown, st, &s);
    })
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let mut el = place(c.rect)
        .id(gpui::ElementId::Name(id.clone().into()))
        .child(paint);
    if open {
        el = el.child(list(kit, cx, c, st, &lens, on_change, &slot));
    }
    if !c.enabled {
        return el.into_any_element();
    }
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut V, _: &MouseDownEvent, _, cx| {
            let was_open = v.kit().popup_is(&id);
            let kit = v.kit();
            kit.focus = Some(id.clone().into());
            kit.press(&id, None);
            if was_open {
                kit.popup = None;
            } else {
                kit.popup = Some(id.clone().into());
                if let Some(st) = lens(v) {
                    st.open();
                }
            }
            cx.notify();
        }),
    )
    .into_any_element()
}

/// The key handler of a combo box.
fn combo_keys<V: Host>(
    id: String,
    lens: Lens<V, ComboState>,
    on_change: Option<Changed<V, usize>>,
) -> KeyHandler<V> {
    Rc::new(move |v, ev, window, cx| {
        let open = v.kit().popup_is(&id);
        let k = &ev.keystroke;
        if open && k.key == "tab" {
            // The focus leaves: the list closes, as in Windows.
            v.kit().popup = None;
            cx.notify();
            return false;
        }
        let Some(st) = lens(v) else {
            return false;
        };
        let r = st.key(&k.key, k.modifiers.alt, open, k.key_char.as_deref());
        let sel = st.selected;
        match r {
            ComboKey::Ignored => return false,
            ComboKey::Handled => {}
            ComboKey::Open => {
                st.open();
                v.kit().popup = Some(id.clone().into());
            }
            ComboKey::Close => v.kit().popup = None,
            ComboKey::Changed => {
                if open {
                    v.kit().popup = None;
                }
                if let (Some(f), Some(i)) = (&on_change, sel) {
                    f(v, i, window, cx);
                }
            }
        }
        cx.notify();
        true
    })
}

/// Paint the closed combo box; `(focused, enabled, down, hot)`.
fn paint_field(
    window: &mut Window,
    cx: &mut gpui::App,
    b: gpui::Bounds<gpui::Pixels>,
    shown: &str,
    (focused, enabled, down, hot): (bool, bool, bool, bool),
    s: &Scheme,
) {
    let (w, h) = size_of(b);
    let o = b.origin;
    let r = Rect::new(0, 0, w, h);
    paint::fill(
        window,
        o,
        r,
        s.of(if enabled { Role::Window } else { Role::Face }),
    );
    paint::edge(window, o, r, Edge::Sunken, s);
    let btn = Rect::new(w - 2 - scrollbar::THICKNESS, 2, scrollbar::THICKNESS, h - 4);
    if super::skin::skin() == super::skin::Skin::Classic {
        paint_drop_button(window, cx, o, btn, down, enabled, s);
    } else {
        // A themed drop-down button sits one pixel in from the field's border.
        let b2 = Rect::new(w - 1 - scrollbar::THICKNESS, 1, scrollbar::THICKNESS, h - 2);
        let st = super::skin::State {
            hot,
            down,
            focus: false,
            enabled,
        };
        super::skin::drop_button(window, o, b2, st);
    }
    let field = Rect::new(3, 3, btn.x - 4, h - 6);
    let ty = (h - LINE) / 2;
    let mut ink = match (enabled, super::skin::skin()) {
        (true, _) => s.of(Role::Text),
        (false, super::skin::Skin::Classic) => s.of(Role::Shadow),
        // The style's grey, which also reads on Windows 10's dark face.
        (false, _) => super::colors::hsla(super::skin::text_color(false)),
    };
    if focused {
        paint::fill(window, o, field, s.of(Role::Selection));
        paint::focus_rect(window, o, field, s.of(Role::Highlight));
        ink = s.of(Role::SelectionText);
    }
    let clip = paint::bounds(o, field);
    window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
        text::draw(window, cx, paint::at(o, 5, ty), shown, false, ink);
    });
}

/// The classic drop-down button of a combo box.
#[allow(clippy::too_many_arguments)]
fn paint_drop_button(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    btn: Rect,
    down: bool,
    enabled: bool,
    s: &Scheme,
) {
    paint::panel(
        window,
        o,
        btn,
        if down {
            Edge::Flat(Role::Shadow)
        } else {
            Edge::Raised
        },
        s,
    );
    let m = marks::arrow(Dir::Down, 7);
    let d = i32::from(down);
    let mode = if enabled {
        Mode::Normal
    } else {
        Mode::Disabled
    };
    glyph::draw(
        window,
        cx,
        o,
        (btn.x + (btn.w - m.w) / 2 + d, btn.y + (btn.h - m.h) / 2 + d),
        &m,
        mode,
        s,
    );
}

/// What the open list shows.
struct Shown<'a> {
    items: &'a [String],
    hot: usize,
    top: usize,
    rows: i32,
    items_w: i32,
}

/// Paint the open list.
fn paint_list(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    (w, h): (i32, i32),
    l: &Shown<'_>,
    s: &Scheme,
) {
    let all = Rect::new(0, 0, w, h);
    paint::fill(window, o, all, s.of(Role::Window));
    paint::edge(window, o, all, Edge::Flat(Role::DarkShadow), s);
    let visible = l.items.iter().enumerate().skip(l.top).take(l.rows as usize);
    for (row, (i, item)) in (0_i32..).zip(visible) {
        let r = Rect::new(1, 1 + row * ITEM, l.items_w - 1, ITEM);
        let ink = if i == l.hot {
            paint::fill(window, o, r, s.of(Role::Selection));
            s.of(Role::SelectionText)
        } else {
            s.of(Role::Text)
        };
        text::draw(window, cx, paint::at(o, 3, r.y), item, false, ink);
    }
}

/// The scroll bar of a long open list.
fn list_scroll_bar<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    id: &str,
    (w, h): (i32, i32),
    st: &ComboState,
    lens: &Lens<V, ComboState>,
) -> AnyElement {
    let bar = Bar {
        id: &format!("{id}.sb"),
        rect: Rect::new(w - 1 - scrollbar::THICKNESS, 1, scrollbar::THICKNESS, h - 2),
        vertical: true,
    };
    let sl = lens.clone();
    let sb_lens: Lens<V, ScrollState> = Rc::new(move |v| sl(v).map(|s| &mut s.scroll));
    scrollbar::scroll_bar(kit, cx, bar, st.scroll, sb_lens, None)
}

/// The mouse wheel over the open list scrolls it (not what lies under it).
fn list_wheel<V: Host>(
    lens: Lens<V, ComboState>,
) -> impl Fn(&mut V, &gpui::ScrollWheelEvent, &mut Window, &mut Context<'_, V>) + 'static {
    move |v, ev, _, cx| {
        let dy = f32::from(ev.delta.pixel_delta(px(ITEM as f32)).y);
        if let Some(st) = lens(v) {
            st.wheel.scroll(&mut st.scroll, dy, ITEM as f32);
        }
        cx.stop_propagation();
        cx.notify();
    }
}

/// The open list under the combo box.
fn list<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    st: &ComboState,
    lens: &Lens<V, ComboState>,
    on_change: Option<Changed<V, usize>>,
    field: &Slot,
) -> AnyElement {
    let s = scheme(cx);
    let n = st.items.len();
    let rows = n.clamp(1, DROP_COUNT) as i32;
    let scrolls = n > DROP_COUNT;
    let (w, h) = (c.rect.w, rows * ITEM + 2);
    let items_w = if scrolls {
        w - 1 - scrollbar::THICKNESS
    } else {
        w - 1
    };
    let shown: Vec<String> = st.items.clone();
    let (hot, top) = (st.hot, st.scroll.pos.max(0) as usize);
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            paint_slot.set(b);
            let shown = Shown {
                items: &shown,
                hot,
                top,
                rows,
                items_w,
            };
            paint_list(window, cx, b.origin, (w, h), &shown, &s);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let mut body = place(Rect::new(0, 0, w, h))
        .id(gpui::ElementId::Name(format!("{}.list", c.id).into()))
        .occlude()
        .child(paint);
    if scrolls {
        body = body.child(list_scroll_bar(kit, cx, c.id, (w, h), st, lens));
    }
    let row_at = move |slot: &Slot, p: gpui::Point<gpui::Pixels>| -> Option<usize> {
        let (x, y) = slot.local(p)?;
        (x >= 0 && x < items_w && y >= 1 && y < h - 1).then(|| top + ((y - 1) / ITEM) as usize)
    };
    let (l1, l2, s1, s2, f1) = (
        lens.clone(),
        lens.clone(),
        slot.clone(),
        slot,
        field.clone(),
    );
    let id = c.id.to_owned();
    let body = body
        .on_scroll_wheel(cx.listener(list_wheel(lens.clone())))
        .on_mouse_move(cx.listener(move |v: &mut V, ev: &MouseMoveEvent, _, cx| {
            if let (Some(i), Some(st)) = (row_at(&s1, ev.position), l1(v))
                && i < st.items.len()
                && st.hot != i
            {
                st.hot = i;
                cx.notify();
            }
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |v: &mut V, ev: &gpui::MouseUpEvent, window, cx| {
                let Some(i) = row_at(&s2, ev.position) else {
                    return;
                };
                v.kit().popup = None;
                let changed = l2(v).is_some_and(|st| st.select(i));
                if changed && let Some(f) = &on_change {
                    f(v, i, window, cx);
                }
                cx.notify();
            }),
        )
        .on_mouse_down_out(cx.listener(move |v: &mut V, ev: &MouseDownEvent, _, cx| {
            let on_field = f1.bounds().is_some_and(|b| b.contains(&ev.position));
            if !on_field && v.kit().popup_is(&id) {
                v.kit().popup = None;
                cx.notify();
            }
        }));
    let list = deferred(
        anchored().snap_to_window().child(
            gpui::div()
                .relative()
                .w(px(w as f32))
                .h(px(h as f32))
                .child(body),
        ),
    )
    .with_priority(1);
    place(Rect::new(0, c.rect.h, w, 0))
        .child(list)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo() -> ComboState {
        ComboState::new(
            &["Hamming", "Hann", "Kaiser, b=8", "Blackman", "Rect"],
            Some(2),
        )
    }

    #[test]
    fn closed_keys_change_the_selection() {
        let mut c = combo();
        assert_eq!(c.text(), "Kaiser, b=8");
        assert_eq!(c.key("down", false, false, None), ComboKey::Changed);
        assert_eq!(c.selected, Some(3));
        assert_eq!(c.key("end", false, false, None), ComboKey::Changed);
        assert_eq!(
            c.key("down", false, false, None),
            ComboKey::Handled,
            "already last"
        );
        assert_eq!(c.key("home", false, false, None), ComboKey::Changed);
        assert_eq!(c.selected, Some(0));
        assert_eq!(
            c.key("h", false, false, Some("h")),
            ComboKey::Changed,
            "type-ahead finds the next H"
        );
        assert_eq!(c.selected, Some(1));
        assert_eq!(c.key("tab", false, false, None), ComboKey::Ignored);
    }

    #[test]
    fn keys_with_nothing_selected_start_at_the_first_item() {
        let items = ["Hamming", "Hann", "Rect"];
        let mut c = ComboState::new(&items, None);
        assert_eq!(c.key("down", false, false, None), ComboKey::Changed);
        assert_eq!(c.selected, Some(0));
        let mut c = ComboState::new(&items, None);
        assert_eq!(c.key("pagedown", false, false, None), ComboKey::Changed);
        assert_eq!(c.selected, Some(0));
        let mut c = ComboState::new(&items, None);
        assert_eq!(c.key("h", false, false, Some("h")), ComboKey::Changed);
        assert_eq!(
            c.selected,
            Some(0),
            "type-ahead checks the first item first"
        );
        let mut c = ComboState::new(&items, None);
        assert_eq!(c.key("end", false, false, None), ComboKey::Changed);
        assert_eq!(c.selected, Some(2));
    }

    #[test]
    fn open_list_moves_the_highlight_only() {
        let mut c = combo();
        assert_eq!(c.key("down", true, false, None), ComboKey::Open);
        c.open();
        assert_eq!(c.hot, 2);
        assert_eq!(c.key("up", false, true, None), ComboKey::Handled);
        assert_eq!((c.hot, c.selected), (1, Some(2)));
        assert_eq!(c.key("enter", false, true, None), ComboKey::Changed);
        assert_eq!(c.selected, Some(1));
        assert_eq!(c.key("escape", false, true, None), ComboKey::Close);
        assert_eq!(c.key("f4", false, true, None), ComboKey::Close);
    }

    #[test]
    fn long_lists_scroll_to_the_highlight() {
        let names: Vec<String> = (0..20).map(|i| format!("Item {i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut c = ComboState::new(&refs, Some(15));
        c.open();
        assert_eq!(c.scroll.pos, 8, "item 15 is the last visible row");
        c.key("home", false, true, None);
        assert_eq!(c.scroll.pos, 0);
        assert_eq!(
            ComboState::new(&[], None).key("down", false, false, None),
            ComboKey::Ignored
        );
    }
}
