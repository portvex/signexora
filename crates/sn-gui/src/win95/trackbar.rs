//! The Windows 95 trackbar (`msctls_trackbar32`, Delphi's `TTrackBar`): a
//! sunken channel with a raised thumb that slides along it. A click on the
//! channel moves the thumb a page towards the pointer (repeating while held),
//! the thumb drags, and the arrow keys, Page Up/Down, Home and End move it when
//! it has the focus.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Styled, canvas,
};

use super::button::Ctl;
use super::colors::{Role, scheme};
use super::geom::{Edge, Rect};
use super::kit::{Action, Capture, Host, KeyHandler, Kit, Lens, action, start_repeat};
use super::paint;
use super::widget::{Slot, size_of};

/// Width of the thumb.
pub(crate) const THUMB_W: i32 = 11;
/// Height of the thumb (the trackbar is drawn centred in its rectangle).
pub(crate) const THUMB_H: i32 = 18;

/// A trackbar's range and position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TrackState {
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) pos: i32,
    /// Step of a click on the channel and of Page Up/Down.
    pub(crate) page: i32,
}

impl TrackState {
    pub(crate) fn new(min: i32, max: i32, pos: i32, page: i32) -> Self {
        let mut s = Self {
            min,
            max: max.max(min),
            pos: min,
            page: page.max(1),
        };
        s.set(pos);
        s
    }

    /// Move to `pos` (clamped). Returns whether it moved.
    pub(crate) fn set(&mut self, pos: i32) -> bool {
        let p = pos.clamp(self.min, self.max);
        let moved = p != self.pos;
        self.pos = p;
        moved
    }

    /// Move by `n`. Returns whether it moved.
    pub(crate) fn step(&mut self, n: i32) -> bool {
        self.set(self.pos.saturating_add(n))
    }
}

/// Left edge of the thumb in a trackbar `w` wide.
pub(crate) fn thumb_x(st: &TrackState, w: i32) -> i32 {
    let room = (w - THUMB_W).max(0);
    let span = (st.max - st.min).max(1);
    ((i64::from(st.pos - st.min) * i64::from(room)) / i64::from(span)) as i32
}

/// The position whose thumb starts at `x` in a trackbar `w` wide.
pub(crate) fn pos_at(st: &TrackState, w: i32, x: i32) -> i32 {
    let room = (w - THUMB_W).max(1);
    let span = st.max - st.min;
    let x = x.clamp(0, room);
    let p = (i64::from(x) * i64::from(span) + i64::from(room) / 2) / i64::from(room);
    st.min + p as i32
}

/// Keys of a focused trackbar.
fn track_keys<V: Host>(lens: Lens<V, TrackState>, on_change: Option<Action<V>>) -> KeyHandler<V> {
    Rc::new(move |v, ev, window, cx| {
        let Some(st) = lens(v) else {
            return false;
        };
        let moved = match ev.keystroke.key.as_str() {
            "left" | "up" => st.step(-1),
            "right" | "down" => st.step(1),
            "pageup" => st.step(-st.page),
            "pagedown" => st.step(st.page),
            "home" => st.set(st.min),
            "end" => st.set(st.max),
            _ => return false,
        };
        if moved && let Some(f) = &on_change {
            f(v, window, cx);
        }
        true
    })
}

/// A horizontal trackbar at `c.rect`. `on_change` runs after every move.
pub(crate) fn trackbar<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    st: TrackState,
    lens: Lens<V, TrackState>,
    on_change: Option<Action<V>>,
) -> AnyElement {
    let s = scheme(cx);
    let focused = kit.has_focus(c.id);
    if c.enabled {
        kit.focusable(
            c.id,
            c.scope,
            Some(track_keys(lens.clone(), on_change.clone())),
        );
    }
    let enabled = c.enabled;
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let paint = canvas(super::widget::hover_hitbox, move |b, hitbox, window, _| {
        paint_slot.set(b);
        let (w, h) = size_of(b);
        let o = b.origin;
        let tx = thumb_x(&st, w);
        let over = super::widget::hot(window, &hitbox, o, thumb_rect(tx, h));
        paint_track(window, o, (w, h), tx, (focused, enabled, over), &s);
    })
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let el = super::widget::place(c.rect)
        .id(gpui::ElementId::Name(c.id.to_owned().into()))
        .child(paint);
    if !enabled {
        return el.into_any_element();
    }
    let id = c.id.to_owned();
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut V, ev: &MouseDownEvent, window, cx| {
            v.kit().focus = Some(id.clone().into());
            let (Some((x, _)), Some(b)) = (slot.local(ev.position), slot.bounds()) else {
                return;
            };
            let (w, _) = size_of(b);
            let Some(now) = lens(v).map(|s| *s) else {
                return;
            };
            let tx = thumb_x(&now, w);
            if (tx..tx + THUMB_W).contains(&x) {
                let (lens, on_change, grab) = (lens.clone(), on_change.clone(), x - tx);
                let origin = b.origin.x;
                v.kit().press(
                    &id,
                    Some(Capture {
                        on_move: Some(Rc::new(move |v, p, window, cx| {
                            let px = f32::from(p.x - origin).floor() as i32;
                            let Some(st) = lens(v) else { return };
                            if st.set(pos_at(st, w, px - grab))
                                && let Some(f) = &on_change
                            {
                                f(v, window, cx);
                            }
                        })),
                        on_up: None,
                    }),
                );
            } else {
                let dir = if x < tx { -1 } else { 1 };
                let (lens2, on_change) = (lens.clone(), on_change.clone());
                let step = action(move |v: &mut V, window, cx| {
                    let Some(st) = lens2(v) else { return };
                    // Stop once the thumb reaches the pointer.
                    let tx = thumb_x(st, w);
                    let past = if dir < 0 { tx <= x } else { tx + THUMB_W > x };
                    if !past
                        && st.step(dir * st.page)
                        && let Some(f) = &on_change
                    {
                        f(v, window, cx);
                    }
                });
                v.kit().press(&id, None);
                start_repeat(v, &id, step, window, cx);
            }
            cx.notify();
        }),
    )
    .into_any_element()
}

/// The thumb at `tx` of a track bar `h` high.
fn thumb_rect(tx: i32, h: i32) -> Rect {
    Rect::new(tx, h / 2 - THUMB_H / 2, THUMB_W, THUMB_H)
}

/// Paint a track bar `w` x `h` with its thumb at `tx`: the classic sunken
/// channel and raised thumb, or the themed style's; `(focused, enabled,
/// hot)`, hot with the mouse over the thumb.
fn paint_track(
    window: &mut gpui::Window,
    o: gpui::Point<gpui::Pixels>,
    (w, h): (i32, i32),
    tx: i32,
    (focused, enabled, hot): (bool, bool, bool),
    s: &super::colors::Scheme,
) {
    let mid = h / 2;
    let channel = Rect::new(2, mid - 2, (w - 4).max(0), 4);
    let thumb = thumb_rect(tx, h);
    if super::skin::skin() == super::skin::Skin::Classic {
        paint::edge(window, o, channel, Edge::Sunken, s);
        paint::panel(window, o, thumb, Edge::Raised, s);
        if !enabled {
            // A disabled thumb is dithered like Windows draws it.
            paint::dither(
                window,
                o,
                thumb.inset(2),
                s.of(Role::Face),
                s.of(Role::Highlight),
            );
        }
    } else {
        let st = super::skin::State {
            hot,
            down: false,
            focus: focused,
            enabled,
        };
        super::skin::trackbar(window, o, channel, thumb, st);
    }
    if focused {
        paint::focus_rect(window, o, Rect::new(0, 0, w, h), s.of(Role::Text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_map_to_the_thumb_and_back() {
        let mut st = TrackState::new(-45, 45, 0, 5);
        assert_eq!(thumb_x(&st, 111), 50);
        assert_eq!(pos_at(&st, 111, 50), 0);
        assert_eq!(pos_at(&st, 111, -20), -45);
        assert_eq!(pos_at(&st, 111, 500), 45);
        assert!(st.step(100));
        assert_eq!(st.pos, 45);
        assert!(!st.step(1));
        assert_eq!(TrackState::new(1, 255, 999, 10).pos, 255);
    }
}
