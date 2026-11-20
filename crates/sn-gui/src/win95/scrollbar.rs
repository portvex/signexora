//! Scroll bars: arrow buttons, a raised thumb sized to the page, and the
//! dithered track. [`ScrollState`] and the thumb math are pure and tested.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Styled, Window, canvas,
};

use super::colors::{Role, Scheme, scheme};
use super::geom::{Edge, Rect};
use super::glyph::{self, Mode};
use super::kit::{Action, Capture, Host, Kit, Lens, action, start_repeat};
use super::marks::{self, Dir};
use super::paint;
use super::widget::{Slot, hovered, place, size_of};

/// Thickness of a scroll bar and size of its arrow buttons (`SM_CXVSCROLL`).
pub(crate) const THICKNESS: i32 = 16;
/// Smallest thumb length.
pub(crate) const MIN_THUMB: i32 = 8;

/// The range, page and position of a scroll bar (Windows `SCROLLINFO`): the
/// position runs from `min` to `max - page + 1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScrollState {
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) page: i32,
    pub(crate) pos: i32,
}

impl ScrollState {
    pub(crate) fn new(min: i32, max: i32, page: i32, pos: i32) -> Self {
        let mut s = Self {
            min,
            max,
            page,
            pos,
        };
        s.set(pos);
        s
    }

    /// The highest position.
    pub(crate) fn top(&self) -> i32 {
        (self.max - self.page.max(1) + 1).max(self.min)
    }

    /// Whether there is anything to scroll.
    pub(crate) fn enabled(&self) -> bool {
        self.top() > self.min
    }

    /// Set the position (clamped). Returns whether it changed.
    pub(crate) fn set(&mut self, pos: i32) -> bool {
        let p = pos.clamp(self.min, self.top());
        let changed = p != self.pos;
        self.pos = p;
        changed
    }

    /// Scroll by `n` lines.
    pub(crate) fn line(&mut self, n: i32) -> bool {
        self.set(self.pos.saturating_add(n))
    }

    /// Scroll by `n` pages.
    pub(crate) fn page_by(&mut self, n: i32) -> bool {
        self.set(self.pos.saturating_add(n.saturating_mul(self.page.max(1))))
    }
}

/// Parts of a line [`Wheel`] counts in.
const PART: i32 = 1_000_000;

/// What is left of the mouse wheel's and trackpad's scrolling between events,
/// in millionths of a line: a trackpad sends a few pixels per event, which
/// would round to no line at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Wheel {
    rest: i32,
}

impl Wheel {
    /// The whole lines to scroll for a wheel delta of `dy` pixels (positive:
    /// the content moves down, so the view scrolls up) with lines `unit`
    /// pixels high; the fraction is kept for the next event.
    pub(crate) fn lines(&mut self, dy: f32, unit: f32) -> i32 {
        if !dy.is_finite() || unit <= 0.0 {
            return 0;
        }
        let part = (-dy / unit * PART as f32).round().clamp(-1e9, 1e9) as i32;
        let total = self.rest.saturating_add(part);
        self.rest = total % PART;
        total / PART
    }

    /// Scroll `st` by the whole lines of `dy` pixels. Returns whether it moved.
    pub(crate) fn scroll(&mut self, st: &mut ScrollState, dy: f32, unit: f32) -> bool {
        let n = self.lines(dy, unit);
        n != 0 && st.line(n)
    }
}

/// The thumb as `(offset into the track, length)`, or `None` when the bar is
/// disabled or the track is too short for a thumb.
pub(crate) fn thumb(st: &ScrollState, track: i32) -> Option<(i32, i32)> {
    if !st.enabled() || track < MIN_THUMB {
        return None;
    }
    let range = i64::from(st.max) - i64::from(st.min) + 1;
    let len = if st.page <= 0 {
        THICKNESS
    } else {
        (i64::from(track) * i64::from(st.page) / range.max(1)) as i32
    }
    .clamp(MIN_THUMB, track);
    let span = i64::from(track - len);
    let steps = i64::from(st.top() - st.min);
    let off = if steps > 0 {
        (i64::from(st.pos - st.min) * span / steps) as i32
    } else {
        0
    };
    Some((off, len))
}

/// The position for a thumb dragged to `off` in a track of `track` pixels.
pub(crate) fn pos_for_thumb(st: &ScrollState, track: i32, len: i32, off: i32) -> i32 {
    let span = i64::from(track - len);
    if span <= 0 {
        return st.min;
    }
    let steps = i64::from(st.top() - st.min);
    let off = i64::from(off).clamp(0, span);
    st.min + ((off * steps + span / 2) / span) as i32
}

/// A part of a scroll bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// The up or left arrow.
    LineBack,
    /// The down or right arrow.
    LineFwd,
    /// The track before the thumb.
    PageBack,
    /// The track after the thumb.
    PageFwd,
    Thumb,
}

impl Part {
    fn name(self) -> &'static str {
        match self {
            Self::LineBack => "lb",
            Self::LineFwd => "lf",
            Self::PageBack => "pb",
            Self::PageFwd => "pf",
            Self::Thumb => "th",
        }
    }
}

/// Arrow size for a bar `len` pixels long: square, or squeezed when short.
fn arrow_len(len: i32) -> i32 {
    THICKNESS.min(len / 2)
}

/// The part at `at` pixels along a bar `len` pixels long.
pub(crate) fn part_at(st: &ScrollState, len: i32, at: i32) -> Part {
    let a = arrow_len(len);
    if at < a {
        return Part::LineBack;
    }
    if at >= len - a {
        return Part::LineFwd;
    }
    match thumb(st, len - 2 * a) {
        Some((off, _)) if at < a + off => Part::PageBack,
        Some((off, tl)) if at >= a + off + tl => Part::PageFwd,
        Some(_) => Part::Thumb,
        None => Part::PageFwd,
    }
}

/// Whether a held `part` steps again with the pointer at `p` (bar
/// coordinates, along and across a bar `len` long and `across` thick): only
/// while the pointer stays on that part. Paging so stops once the thumb
/// reaches the pointer, and the repeat pauses while the pointer is away from
/// the pressed part, as in Windows 95.
pub(crate) fn keeps_stepping(
    st: &ScrollState,
    len: i32,
    across: i32,
    part: Part,
    p: (i32, i32),
) -> bool {
    let (at, side) = p;
    (0..len).contains(&at) && (0..across).contains(&side) && part_at(st, len, at) == part
}

/// Props of a scroll bar.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bar<'a> {
    pub(crate) id: &'a str,
    pub(crate) rect: Rect,
    pub(crate) vertical: bool,
}

/// A scroll bar. `st` is the state to show, `lens` reaches it in the view, and
/// `on_change` runs after every position change.
pub(crate) fn scroll_bar<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    bar: Bar<'_>,
    st: ScrollState,
    lens: Lens<V, ScrollState>,
    on_change: Option<Action<V>>,
) -> AnyElement {
    tinted_scroll_bar(kit, cx, bar, st, lens, on_change, None)
}

/// A scroll bar whose track is `track` instead of the dither pattern (SA tints
/// some of its scroll bars: the OFDM module's cyan and lavender ones).
pub(crate) fn tinted_scroll_bar<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    bar: Bar<'_>,
    st: ScrollState,
    lens: Lens<V, ScrollState>,
    on_change: Option<Action<V>>,
    track: Option<super::colors::Rgb>,
) -> AnyElement {
    let s = scheme(cx);
    let held = kit
        .pressed
        .as_ref()
        .and_then(|p| p.strip_prefix(bar.id).and_then(|r| r.strip_prefix('.')))
        .map(str::to_owned);
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let vertical = bar.vertical;
    let paint = canvas(super::widget::hover_hitbox, move |b, hitbox, window, cx| {
        paint_slot.set(b);
        let hover = hovered(window, b);
        paint_bar(
            window,
            cx,
            b,
            &st,
            (vertical, hitbox.is_hovered(window)),
            held.as_deref().filter(|_| hover),
            &s,
            track,
        );
    })
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let el = place(bar.rect)
        .id(gpui::ElementId::Name(bar.id.to_owned().into()))
        .child(paint);
    if !st.enabled() {
        return el.into_any_element();
    }
    let id = bar.id.to_owned();
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut V, ev: &MouseDownEvent, window, cx| {
            bar_mouse_down(
                v,
                ev,
                window,
                cx,
                &id,
                vertical,
                &lens,
                &slot,
                on_change.clone(),
            );
            cx.notify();
        }),
    )
    .into_any_element()
}

/// Mouse down on a scroll bar: step and repeat on arrows and track, drag the thumb.
#[allow(clippy::too_many_arguments)]
fn bar_mouse_down<V: Host>(
    v: &mut V,
    ev: &MouseDownEvent,
    window: &mut Window,
    cx: &mut Context<'_, V>,
    id: &str,
    vertical: bool,
    lens: &Lens<V, ScrollState>,
    slot: &Slot,
    on_change: Option<Action<V>>,
) {
    let (Some((x, y)), Some(b)) = (slot.local(ev.position), slot.bounds()) else {
        return;
    };
    let (w, h) = size_of(b);
    let (len, at) = if vertical { (h, y) } else { (w, x) };
    let Some(st) = lens(v).map(|s| *s) else {
        return;
    };
    let part = part_at(&st, len, at);
    let pid = format!("{id}.{}", part.name());
    if part == Part::Thumb {
        let a = arrow_len(len);
        let track = len - 2 * a;
        let Some((off, tl)) = thumb(&st, track) else {
            return;
        };
        let (lens, grab) = (lens.clone(), at - off);
        v.kit().press(
            &pid,
            Some(Capture {
                on_move: Some(Rc::new(move |v, p, window, cx| {
                    let b = b_local(p, b, vertical);
                    let Some(st) = lens(v) else { return };
                    let pos = pos_for_thumb(st, track, tl, b - grab);
                    if st.set(pos)
                        && let Some(f) = &on_change
                    {
                        f(v, window, cx);
                    }
                })),
                on_up: None,
            }),
        );
        return;
    }
    let step: i32 = match part {
        Part::LineBack => -1,
        Part::LineFwd => 1,
        Part::PageBack => -2,
        _ => 2,
    };
    let (lens, slot) = (lens.clone(), slot.clone());
    let across = if vertical { w } else { h };
    let act = action(move |v: &mut V, window, cx| {
        let Some(st) = lens(v) else { return };
        let here = slot
            .local(window.mouse_position())
            .map(|(x, y)| if vertical { (y, x) } else { (x, y) });
        if !here.is_some_and(|p| keeps_stepping(st, len, across, part, p)) {
            return;
        }
        let moved = if step.abs() == 1 {
            st.line(step)
        } else {
            st.page_by(step / 2)
        };
        if moved && let Some(f) = &on_change {
            f(v, window, cx);
        }
    });
    v.kit().press(&pid, None);
    start_repeat(v, &pid, act, window, cx);
}

/// `p` along the bar whose window bounds are `b`.
fn b_local(p: gpui::Point<gpui::Pixels>, b: gpui::Bounds<gpui::Pixels>, vertical: bool) -> i32 {
    let d = if vertical {
        p.y - b.origin.y
    } else {
        p.x - b.origin.x
    };
    f32::from(d).floor() as i32
}

/// Paint a scroll bar. `over` is true while the mouse is over the bar and
/// nothing covers it; `held` names the part held down under the mouse;
/// `tint` replaces the track's dither pattern.
#[allow(clippy::too_many_arguments)]
fn paint_bar(
    window: &mut Window,
    cx: &mut gpui::App,
    b: gpui::Bounds<gpui::Pixels>,
    st: &ScrollState,
    (vertical, over): (bool, bool),
    held: Option<&str>,
    s: &Scheme,
    tint: Option<super::colors::Rgb>,
) {
    let (w, h) = size_of(b);
    let o = b.origin;
    let len = if vertical { h } else { w };
    let a = arrow_len(len);
    let along = |start: i32, n: i32| {
        if vertical {
            Rect::new(0, start, w, n)
        } else {
            Rect::new(start, 0, n, h)
        }
    };
    let track = len - 2 * a;
    if super::skin::skin() != super::skin::Skin::Classic {
        paint_xp_bar(window, o, (w, h), st, (vertical, over), held, tint);
        return;
    }
    match tint {
        Some(c) => paint::fill(window, o, along(a, track), super::colors::hsla(c)),
        None => paint::dither(
            window,
            o,
            along(a, track),
            s.of(Role::Face),
            s.of(Role::Highlight),
        ),
    }
    let enabled = st.enabled();
    let (back, fwd) = if vertical {
        (Dir::Up, Dir::Down)
    } else {
        (Dir::Left, Dir::Right)
    };
    for (r, dir, part) in [
        (along(0, a), back, Part::LineBack),
        (along(len - a, a), fwd, Part::LineFwd),
    ] {
        let down = held == Some(part.name());
        paint::panel(
            window,
            o,
            r,
            if down {
                Edge::Flat(Role::Shadow)
            } else {
                Edge::Raised
            },
            s,
        );
        let m = marks::arrow(dir, 7);
        let d = i32::from(down);
        let mode = if enabled {
            Mode::Normal
        } else {
            Mode::Disabled
        };
        let at = (r.x + (r.w - m.w) / 2 + d, r.y + (r.h - m.h) / 2 + d);
        glyph::draw(window, cx, o, at, &m, mode, s);
    }
    let Some((off, tl)) = thumb(st, track) else {
        return;
    };
    let dark = |window: &mut Window, r: Rect| {
        paint::dither(window, o, r, s.of(Role::DarkShadow), s.of(Role::Shadow));
    };
    match held {
        Some("pb") => dark(window, along(a, off)),
        Some("pf") => dark(window, along(a + off + tl, track - off - tl)),
        _ => {}
    }
    paint::panel(window, o, along(a + off, tl), Edge::Raised, s);
}

/// A themed scroll bar: the style's arrow buttons and thumb on its track.
#[allow(clippy::too_many_arguments)]
fn paint_xp_bar(
    window: &mut Window,
    o: gpui::Point<gpui::Pixels>,
    (w, h): (i32, i32),
    st: &ScrollState,
    (vertical, over): (bool, bool),
    held: Option<&str>,
    tint: Option<super::colors::Rgb>,
) {
    use super::skin::{self, State};
    let len = if vertical { h } else { w };
    let a = arrow_len(len);
    let along = |start: i32, n: i32| {
        if vertical {
            Rect::new(0, start, w, n)
        } else {
            Rect::new(start, 0, n, h)
        }
    };
    let track = len - 2 * a;
    match tint {
        Some(t) => paint::fill(window, o, along(a, track), super::colors::hsla(t)),
        None => skin::scroll_track(window, o, along(a, track), vertical),
    }
    let enabled = st.enabled();
    let pointer = window.mouse_position();
    let hot = |r: Rect| over && paint::bounds(o, r).contains(&pointer);
    let (back, fwd) = if vertical {
        (Dir::Up, Dir::Down)
    } else {
        (Dir::Left, Dir::Right)
    };
    for (r, dir, part) in [
        (along(0, a), back, Part::LineBack),
        (along(len - a, a), fwd, Part::LineFwd),
    ] {
        let s = State {
            hot: hot(r),
            down: held == Some(part.name()),
            focus: false,
            enabled,
        };
        skin::scroll_button(window, o, r, Some(dir), s, vertical);
    }
    let Some((off, tl)) = thumb(st, track) else {
        return;
    };
    let tr = along(a + off, tl);
    let s = State {
        hot: hot(tr),
        down: held == Some("thumb"),
        focus: false,
        enabled,
    };
    skin::scroll_button(window, o, tr, None, s, vertical);
    skin::thumb_grip(window, o, tr, vertical);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_clamps_to_range_minus_page() {
        let mut st = ScrollState::new(0, 99, 10, 500);
        assert_eq!(st.pos, 90);
        assert_eq!(st.top(), 90);
        assert!(st.line(-5));
        assert_eq!(st.pos, 85);
        assert!(st.page_by(-20));
        assert_eq!(st.pos, 0);
        assert!(!st.line(-1));
        assert_eq!(ScrollState::new(0, 9, 10, 0).top(), 0);
        assert!(!ScrollState::new(0, 9, 10, 0).enabled());
    }

    #[test]
    fn thumb_size_and_position() {
        let st = ScrollState::new(0, 99, 25, 0);
        assert_eq!(thumb(&st, 200), Some((0, 50)));
        let st = ScrollState::new(0, 99, 25, 75);
        assert_eq!(
            thumb(&st, 200),
            Some((150, 50)),
            "at the end the thumb touches the far arrow"
        );
        let st = ScrollState::new(0, 9999, 1, 0);
        assert_eq!(thumb(&st, 200).map(|t| t.1), Some(MIN_THUMB));
        let st = ScrollState::new(0, 100, 0, 50);
        assert_eq!(
            thumb(&st, 200).map(|t| t.1),
            Some(THICKNESS),
            "no page: square thumb"
        );
        assert_eq!(thumb(&ScrollState::new(0, 5, 10, 0), 200), None);
        assert_eq!(thumb(&ScrollState::new(0, 99, 10, 0), 4), None);
    }

    #[test]
    fn dragging_the_thumb_maps_back_to_positions() {
        let st = ScrollState::new(0, 99, 25, 0);
        let track = 200;
        let (_, len) = thumb(&st, track).unwrap_or_default();
        assert_eq!(pos_for_thumb(&st, track, len, 0), 0);
        assert_eq!(pos_for_thumb(&st, track, len, 150), 75);
        assert_eq!(pos_for_thumb(&st, track, len, 75), 38);
        assert_eq!(pos_for_thumb(&st, track, len, -30), 0);
        assert_eq!(pos_for_thumb(&st, track, len, 999), 75);
        for pos in 0..=75 {
            let mut s2 = st;
            s2.set(pos);
            let (off, l) = thumb(&s2, track).unwrap_or_default();
            assert_eq!(
                pos_for_thumb(&s2, track, l, off),
                pos,
                "round trip at {pos}"
            );
        }
    }

    #[test]
    fn parts_along_the_bar() {
        let st = ScrollState::new(0, 99, 25, 0);
        // 232 px: arrows 16, track 200, thumb 0..50.
        assert_eq!(part_at(&st, 232, 3), Part::LineBack);
        assert_eq!(part_at(&st, 232, 20), Part::Thumb);
        assert_eq!(part_at(&st, 232, 70), Part::PageFwd);
        assert_eq!(part_at(&st, 232, 220), Part::LineFwd);
        let st = ScrollState::new(0, 99, 25, 75);
        assert_eq!(part_at(&st, 232, 20), Part::PageBack);
        // A short bar squeezes its arrows.
        assert_eq!(part_at(&st, 20, 9), Part::LineBack);
        assert_eq!(part_at(&st, 20, 10), Part::LineFwd);
    }

    #[test]
    fn small_wheel_steps_add_up() {
        let mut w = Wheel::default();
        // A trackpad: 3 px per event on 17 px lines.
        let lines: i32 = (0..17).map(|_| w.lines(-3.0, 17.0)).sum();
        assert_eq!(lines, 3, "51 px are three whole lines");
        let mut w = Wheel::default();
        assert_eq!(w.lines(34.0, 17.0), -2);
        assert_eq!(w.lines(f32::NAN, 17.0), 0);
        assert_eq!(w.lines(5.0, 0.0), 0);
    }

    #[test]
    fn paging_stops_at_the_pointer() {
        // 100 positions, page 10, a 200 px bar (16 px arrows).
        let mut st = ScrollState::new(0, 99, 10, 0);
        let len = 200;
        let at = 150;
        let mut steps = 0;
        while keeps_stepping(&st, len, 16, Part::PageFwd, (at, 5)) && steps < 100 {
            st.page_by(1);
            steps += 1;
        }
        assert!(st.pos < st.top(), "stopped before the end");
        // The thumb reached the pointer (a page step may carry it just past).
        assert_ne!(part_at(&st, len, at), Part::PageFwd);
        assert!(steps > 0);
        // Away from the pressed part (or off the bar), nothing repeats.
        let st = ScrollState::new(0, 99, 10, 0);
        assert!(!keeps_stepping(&st, len, 16, Part::LineFwd, (100, 5)));
        assert!(!keeps_stepping(&st, len, 16, Part::LineFwd, (195, 40)));
        assert!(keeps_stepping(&st, len, 16, Part::LineFwd, (195, 5)));
    }
}
