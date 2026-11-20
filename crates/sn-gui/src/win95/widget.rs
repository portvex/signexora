//! Helpers shared by the widgets: absolute placement (Windows dialogs place
//! every control at fixed pixel coordinates, like Delphi forms) and the bounds
//! a widget had when it was last painted, for mapping mouse positions.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{App, Bounds, Div, Hitbox, HitboxBehavior, Pixels, Point, Styled, Window, div, px};

use super::geom::Rect;

/// A `div` placed at `r` inside a `relative` parent.
pub(crate) fn place(r: Rect) -> Div {
    div()
        .absolute()
        .left(px(r.x as f32))
        .top(px(r.y as f32))
        .w(px(r.w as f32))
        .h(px(r.h as f32))
}

/// Where a widget was painted last, filled in by its paint callback.
#[derive(Clone, Debug, Default)]
pub(crate) struct Slot(Rc<Cell<Option<Bounds<Pixels>>>>);

impl Slot {
    pub(crate) fn set(&self, b: Bounds<Pixels>) {
        self.0.set(Some(b));
    }

    /// The painted width in whole pixels.
    pub(crate) fn width(&self) -> Option<i32> {
        self.0.get().map(|b| size_of(b).0)
    }

    /// The painted bounds.
    pub(crate) fn bounds(&self) -> Option<Bounds<Pixels>> {
        self.0.get()
    }

    /// `p` (window coordinates) relative to the widget's top-left corner.
    pub(crate) fn local(&self, p: Point<Pixels>) -> Option<(i32, i32)> {
        let b = self.0.get()?;
        Some((
            f32::from(p.x - b.origin.x).floor() as i32,
            f32::from(p.y - b.origin.y).floor() as i32,
        ))
    }
}

/// A canvas prepaint that inserts a hitbox over the canvas, so its paint can
/// tell with [`hot`] whether the mouse is over it with what lies on top (another
/// window, an open menu, a modal box) taken into account: Windows shows no
/// hover there.
pub(crate) fn hover_hitbox(b: Bounds<Pixels>, window: &mut Window, _: &mut App) -> Hitbox {
    window.insert_hitbox(b, HitboxBehavior::Normal)
}

/// Whether the mouse is over the part `r` (relative to `o`) of a canvas whose
/// hitbox is `hitbox`, and nothing covers it there.
pub(crate) fn hot(window: &Window, hitbox: &Hitbox, o: Point<Pixels>, r: Rect) -> bool {
    hitbox.is_hovered(window) && super::paint::bounds(o, r).contains(&window.mouse_position())
}

/// Whether the mouse is inside `b`.
pub(crate) fn hovered(window: &Window, b: Bounds<Pixels>) -> bool {
    b.contains(&window.mouse_position())
}

/// Size of painted bounds in whole pixels.
pub(crate) fn size_of(b: Bounds<Pixels>) -> (i32, i32) {
    (
        f32::from(b.size.width).round() as i32,
        f32::from(b.size.height).round() as i32,
    )
}

/// The whole-pixel move from `a` to `b` (drags).
pub(crate) fn pixel_delta(a: Point<Pixels>, b: Point<Pixels>) -> (i32, i32) {
    (
        f32::from(b.x - a.x).round() as i32,
        f32::from(b.y - a.y).round() as i32,
    )
}
