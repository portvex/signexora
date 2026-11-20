//! Drawing and mouse handling of the MDI client area and its child windows:
//! the sizing frame, the caption with the small icon and the caption buttons,
//! moving and sizing with Windows 95's dotted drag outline.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Context, CursorStyle, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Pixels, Point, StatefulInteractiveElement, Styled, Window,
    canvas,
};
use sn_ui_core::winstyle::WinStyle;

use super::colors::{Role, Scheme, scheme};
use super::geom::{Edge, Rect};
use super::glyph::{self, Glyph, Mode};
use super::kit::{Capture, Host, Kit, Lens};
use super::mdi::{self, Child, Hit, Mdi, Side, State, caption_h, frame_w};
use super::skin::{self, CapButton, Skin, State as State2};
use super::text;
use super::widget::{Slot, hot, hover_hitbox, hovered, place, size_of};
use super::{marks, paint};

/// Something the host must act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MdiEvent {
    /// A child was activated (clicked).
    Activated(u32),
    /// The close button was clicked: close the child (and free its content).
    Close(u32),
    /// The window icon was clicked: show the system menu at this window position.
    SysMenu(u32, (i32, i32)),
}

/// The host's MDI state and event handler.
pub(crate) struct MdiHooks<V> {
    pub(crate) lens: Lens<V, Mdi>,
    /// The drag outline lives in the host so it survives renders.
    pub(crate) outline: Lens<V, Option<Rect>>,
    pub(crate) on_event: super::kit::Event<V, MdiEvent>,
}

impl<V> Clone for MdiHooks<V> {
    fn clone(&self) -> Self {
        Self {
            lens: self.lens.clone(),
            outline: self.outline.clone(),
            on_event: self.on_event.clone(),
        }
    }
}

/// The MDI client area at `r` (with its sunken edge) and the children, bottom
/// to top. `contents` gives each child's client-area element. The active
/// child's caption is drawn active only while `frame_active` (the frame has
/// the focus and no modal dialog is open), as in Windows.
pub(crate) fn mdi_client<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    m: &Mdi,
    r: Rect,
    mut contents: Vec<(u32, AnyElement)>,
    outline: Option<Rect>,
    frame_active: bool,
    hooks: &MdiHooks<V>,
) -> AnyElement {
    let s = scheme(cx);
    let edge = canvas(
        |_, _, _| (),
        move |b, (), window, _| {
            let (w, h) = size_of(b);
            paint::edge(window, b.origin, Rect::new(0, 0, w, h), Edge::Sunken, &s);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let mut area = place(Rect::new(2, 2, r.w - 4, r.h - 4))
        .overflow_hidden()
        .bg(gpui::Hsla::from(gpui::rgb(s.app_workspace)));
    for child in m.children() {
        let content = contents
            .iter()
            .position(|(id, _)| *id == child.id)
            .map(|i| contents.swap_remove(i).1);
        let active = frame_active && m.active() == Some(child.id);
        area = area.child(child_window(kit, cx, m, child, active, content, hooks));
    }
    if let Some(o) = outline {
        area = area.child(drag_outline(o));
    }
    place(r).child(edge).child(area).into_any_element()
}

/// The dotted drag rectangle drawn while moving or sizing.
fn drag_outline(o: Rect) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |b, (), window, _| paint::drag_frame(window, b.origin, o, frame_w()),
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}

/// One child window.
fn child_window<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    m: &Mdi,
    c: &Child,
    active: bool,
    content: Option<AnyElement>,
    hooks: &MdiHooks<V>,
) -> AnyElement {
    let s = scheme(cx);
    let Some(r) = m.shown_rect(c.id) else {
        return gpui::div().into_any_element();
    };
    if c.state == State::Maximized && !super::style::native_menu() {
        // Frame and caption are outside the client area (the caption buttons
        // move to the menu bar), so only the client area is drawn.
        let (cw, ch) = m.client;
        let body = place(Rect::new(0, 0, cw, ch))
            .id(gpui::ElementId::Name(format!("mdi{}", c.id).into()))
            .occlude()
            .overflow_hidden()
            .bg(s.of(Role::Face));
        return match content {
            Some(el) => body.child(el),
            None => body,
        }
        .into_any_element();
    }
    let prefix = format!("mdi{}.", c.id);
    let held = kit
        .pressed
        .as_ref()
        .and_then(|p| p.strip_prefix(prefix.as_str()))
        .map(str::to_owned);
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let (title, state, resizable) = (c.title.clone(), c.state, c.resizable);
    let btns = mdi::caption_buttons(r.w);
    let frame = canvas(hover_hitbox, move |b, hitbox, window, cx| {
        paint_slot.set(b);
        let held = held.as_deref().filter(|_| hovered(window, b));
        let hot = |r: Rect| hot(window, &hitbox, b.origin, r);
        let hot = if super::style::metrics().btn_left {
            // macOS: all the lights show their glyphs while the mouse is over
            // any of them.
            [hot(mdi::light_cluster(btns, resizable)); 3]
        } else {
            [hot(btns.0), hot(btns.1), hot(btns.2)]
        };
        paint_frame(
            window, cx, b, &title, active, state, resizable, held, hot, &s,
        );
    })
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let mut el = place(r)
        .id(gpui::ElementId::Name(format!("mdi{}", c.id).into()))
        .occlude()
        .child(frame);
    el = el.children(caption_tips(
        &format!("mdi{}", c.id),
        &frame_buttons(btns, state, resizable),
    ));
    if state != State::Minimized {
        let cr = mdi::client_rect(r.w, r.h);
        let body = place(cr).overflow_hidden().bg(s.of(Role::Face));
        el = el.child(match content {
            Some(c) => body.child(c),
            None => body,
        });
    }
    if state == State::Normal && resizable {
        for (side, rect, cursor) in border_zones(r.w, r.h) {
            el = el.child(border_zone(cx, c.id, side, rect, cursor, hooks));
        }
    }
    let (id, hooks1, hooks2) = (c.id, hooks.clone(), hooks.clone());
    el.capture_any_mouse_down(
        cx.listener(move |v: &mut V, _: &MouseDownEvent, window, cx| {
            let was = (hooks1.lens)(v).and_then(|m| m.active());
            if let Some(m) = (hooks1.lens)(v) {
                m.activate(id);
            }
            if was != Some(id) {
                (hooks1.on_event)(v, MdiEvent::Activated(id), window, cx);
                cx.notify();
            }
        }),
    )
    .on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut V, ev: &MouseDownEvent, window, cx| {
            frame_mouse_down(v, ev, window, cx, id, &slot, &hooks2);
            cx.notify();
        }),
    )
    .into_any_element()
}

/// The sizing zones of a child of size `w` x `h`: four sides, and each
/// corner as an L (a piece along the top or bottom edge and one along the
/// side), so they cover the same pixels `mdi::hit` reports. The caption
/// buttons are cut out: from Vista on they reach into the top frame, and
/// `mdi::hit` reports them before the frame.
fn border_zones(w: i32, h: i32) -> Vec<(Side, Rect, CursorStyle)> {
    let (f, k) = (frame_w(), mdi::CORNER);
    let (min, max, close) = mdi::caption_buttons(w);
    let zones = [
        (
            Side::N,
            Rect::new(k, 0, w - 2 * k, f),
            CursorStyle::ResizeUpDown,
        ),
        (
            Side::S,
            Rect::new(k, h - f, w - 2 * k, f),
            CursorStyle::ResizeUpDown,
        ),
        (
            Side::W,
            Rect::new(0, k, f, h - 2 * k),
            CursorStyle::ResizeLeftRight,
        ),
        (
            Side::E,
            Rect::new(w - f, k, f, h - 2 * k),
            CursorStyle::ResizeLeftRight,
        ),
        (
            Side::NW,
            Rect::new(0, 0, k, f),
            CursorStyle::ResizeUpLeftDownRight,
        ),
        (
            Side::SE,
            Rect::new(w - k, h - f, k, f),
            CursorStyle::ResizeUpLeftDownRight,
        ),
        (
            Side::NE,
            Rect::new(w - k, 0, k, f),
            CursorStyle::ResizeUpRightDownLeft,
        ),
        (
            Side::SW,
            Rect::new(0, h - f, k, f),
            CursorStyle::ResizeUpRightDownLeft,
        ),
        (
            Side::NW,
            Rect::new(0, f, f, k - f),
            CursorStyle::ResizeUpLeftDownRight,
        ),
        (
            Side::NE,
            Rect::new(w - f, f, f, k - f),
            CursorStyle::ResizeUpRightDownLeft,
        ),
        (
            Side::SW,
            Rect::new(0, h - k, f, k - f),
            CursorStyle::ResizeUpRightDownLeft,
        ),
        (
            Side::SE,
            Rect::new(w - f, h - k, f, k - f),
            CursorStyle::ResizeUpLeftDownRight,
        ),
    ];
    let mut out = Vec::with_capacity(zones.len() + 4);
    for (side, r, cursor) in zones {
        let mut parts = vec![r];
        for cut in [min, max, close] {
            parts = parts.iter().flat_map(|p| p.minus(cut)).collect();
        }
        out.extend(parts.into_iter().map(|p| (side, p, cursor)));
    }
    out
}

/// One sizing zone: dragging it shows the outline, releasing resizes.
fn border_zone<V: Host>(
    cx: &mut Context<'_, V>,
    id: u32,
    side: Side,
    r: Rect,
    cursor: CursorStyle,
    hooks: &MdiHooks<V>,
) -> AnyElement {
    let hooks = hooks.clone();
    place(r)
        .cursor(cursor)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |v: &mut V, ev: &MouseDownEvent, _, cx| {
                let Some((start, client)) =
                    (hooks.lens)(v).and_then(|m| Some((m.get(id)?.rect, m.client)))
                else {
                    return;
                };
                let p0 = ev.position;
                start_drag(v, &hooks, id, move |p| {
                    let (dx, dy) = delta(p0, p);
                    mdi::sized(start, side, dx, dy, client.0, client.1)
                });
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .into_any_element()
}

/// Mouse movement from `a` to `b` in whole pixels.
fn delta(a: Point<Pixels>, b: Point<Pixels>) -> (i32, i32) {
    super::widget::pixel_delta(a, b)
}

/// Capture the mouse for a move or size drag: `rect_at` gives the outline for
/// a mouse position; releasing applies it to child `id`.
fn start_drag<V: Host>(
    v: &mut V,
    hooks: &MdiHooks<V>,
    id: u32,
    rect_at: impl Fn(Point<Pixels>) -> Rect + 'static,
) {
    let rect_at = Rc::new(rect_at);
    let (h1, h2, r2) = (hooks.clone(), hooks.clone(), rect_at.clone());
    v.kit().capture(Capture {
        on_move: Some(Rc::new(move |v, p, _, _| {
            if let Some(o) = (h1.outline)(v) {
                *o = Some(rect_at(p));
            }
        })),
        on_up: Some(Rc::new(move |v, p, _, _| {
            let shown = (h2.outline)(v).and_then(Option::take);
            if shown.is_some()
                && let Some(m) = (h2.lens)(v)
            {
                m.set_rect(id, r2(p));
            }
        })),
    });
}

/// Mouse down on a child's frame or caption.
fn frame_mouse_down<V: Host>(
    v: &mut V,
    ev: &MouseDownEvent,
    window: &mut Window,
    cx: &mut Context<'_, V>,
    id: u32,
    slot: &Slot,
    hooks: &MdiHooks<V>,
) {
    let (Some((x, y)), Some(b)) = (slot.local(ev.position), slot.bounds()) else {
        return;
    };
    let (w, h) = size_of(b);
    let Some(c) = (hooks.lens)(v).and_then(|m| m.get(id).cloned()) else {
        return;
    };
    let Some(hit) = mdi::hit(w, h, c.state, c.resizable, x, y) else {
        return;
    };
    match hit {
        Hit::Close | Hit::Maximize | Hit::Minimize => press_button(v, hooks, id, hit, slot.clone()),
        Hit::Icon if ev.click_count >= 2 => (hooks.on_event)(v, MdiEvent::Close(id), window, cx),
        Hit::Icon => {
            let o = b.origin;
            let at = (
                f32::from(o.x).round() as i32 + frame_w(),
                f32::from(o.y).round() as i32 + frame_w() + caption_h(),
            );
            (hooks.on_event)(v, MdiEvent::SysMenu(id, at), window, cx);
        }
        Hit::Caption if ev.click_count >= 2 && c.resizable => {
            let cmd = if c.state == State::Minimized {
                Hit::Minimize
            } else {
                Hit::Maximize
            };
            if let Some(m) = (hooks.lens)(v) {
                m.command(id, cmd);
            }
        }
        Hit::Caption if c.state != State::Maximized => {
            let Some((start, client)) =
                (hooks.lens)(v).and_then(|m| Some((m.shown_rect(id)?, m.client)))
            else {
                return;
            };
            if c.state == State::Minimized {
                return;
            }
            let p0 = ev.position;
            start_drag(v, hooks, id, move |p| {
                let (dx, dy) = delta(p0, p);
                mdi::clamp_into(start.offset(dx, dy), client.0, client.1)
            });
        }
        _ => {}
    }
}

/// Hold a caption button; releasing over it runs its command.
fn press_button<V: Host>(v: &mut V, hooks: &MdiHooks<V>, id: u32, hit: Hit, slot: Slot) {
    let name = button_name(hit);
    let hooks = hooks.clone();
    v.kit().press(
        &format!("mdi{id}.{name}"),
        Some(Capture {
            on_move: None,
            on_up: Some(Rc::new(move |v, p, window, cx| {
                let (Some((x, y)), Some(b)) = (slot.local(p), slot.bounds()) else {
                    return;
                };
                let (w, h) = size_of(b);
                let Some(c) = (hooks.lens)(v).and_then(|m| m.get(id).cloned()) else {
                    return;
                };
                if mdi::hit(w, h, c.state, c.resizable, x, y) != Some(hit) {
                    return;
                }
                if hit == Hit::Close {
                    (hooks.on_event)(v, MdiEvent::Close(id), window, cx);
                } else if let Some(m) = (hooks.lens)(v) {
                    m.command(id, hit);
                }
            })),
        }),
    );
}

/// The tooltip of a caption button of `kind`: its name, as Windows shows
/// from 98 on (and macOS does). `green` is the maximize slot, which macOS
/// calls Zoom.
pub(crate) fn caption_tip(kind: CapButton, green: bool) -> String {
    if green && super::style::metrics().btn_left {
        return sn_ui_core::t!("mdi-tip-zoom");
    }
    match kind {
        CapButton::Close => sn_ui_core::t!("mdi-tip-close"),
        CapButton::Minimize => sn_ui_core::t!("mdi-tip-minimize"),
        CapButton::Maximize => sn_ui_core::t!("mdi-tip-maximize"),
        CapButton::Restore => sn_ui_core::t!("mdi-tip-restore"),
    }
}

/// Whether the drawn style shows caption button tooltips: every style but
/// Windows 95, which had none.
fn shows_caption_tips() -> bool {
    super::style::active().style != WinStyle::Win95
}

/// Invisible elements over caption buttons that show their tooltips; `id`
/// names the window. Each entry is the button's rectangle, its kind and its
/// name (`min`, `max`, `close`). None in Windows 95, which had no caption
/// button tooltips.
pub(crate) fn caption_tips(id: &str, buttons: &[(Rect, CapButton, &str)]) -> Vec<AnyElement> {
    if !shows_caption_tips() {
        return Vec::new();
    }
    buttons
        .iter()
        .map(|&(r, kind, name)| {
            place(r)
                .id(gpui::ElementId::Name(format!("{id}.tip.{name}").into()))
                .tooltip(super::bars::tooltip(&caption_tip(kind, name == "max")))
                .into_any_element()
        })
        .collect()
}

/// The caption buttons of a child in `state` at `(min, max, close)`: close,
/// and maximize (or restore) and minimize (or restore) when it is resizable.
fn frame_buttons(
    (min, max, close): (Rect, Rect, Rect),
    state: State,
    resizable: bool,
) -> Vec<(Rect, CapButton, &'static str)> {
    let mut buttons = vec![(close, CapButton::Close, "close")];
    if resizable {
        let max_kind = if state == State::Maximized {
            CapButton::Restore
        } else {
            CapButton::Maximize
        };
        let min_kind = if state == State::Minimized {
            CapButton::Restore
        } else {
            CapButton::Minimize
        };
        buttons.push((max, max_kind, "max"));
        buttons.push((min, min_kind, "min"));
    }
    buttons
}

fn button_name(hit: Hit) -> &'static str {
    match hit {
        Hit::Close => "close",
        Hit::Maximize => "max",
        _ => "min",
    }
}

/// Paint a child's frame and caption; `hot` tells which of minimize,
/// maximize and close is under the mouse; in the macOS looks all three are
/// set while the mouse is over any light (which also lights an inactive
/// window's lights).
#[allow(clippy::too_many_arguments)]
fn paint_frame(
    window: &mut Window,
    cx: &mut App,
    b: Bounds<Pixels>,
    title: &str,
    active: bool,
    state: State,
    resizable: bool,
    held: Option<&str>,
    [hot_min, hot_max, hot_close]: [bool; 3],
    s: &Scheme,
) {
    let (w, h) = size_of(b);
    let o = b.origin;
    window_frame(window, o, w, h, frame_w(), active, false, s);
    let cap = mdi::caption_rect(w);
    let (min, max, close) = mdi::caption_buttons(w);
    let title_right = if resizable { min.x } else { close.x };
    caption(
        window,
        cx,
        o,
        cap,
        title,
        (active, false),
        title_right - cap.x - 2,
        s,
    );
    let hots = [hot_close, hot_max, hot_min];
    for ((r, kind, name), hot) in frame_buttons((min, max, close), state, resizable)
        .into_iter()
        .zip(hots)
    {
        let st = State2 {
            hot,
            down: held == Some(name),
            focus: false,
            enabled: true,
        };
        caption_button_of(window, cx, o, r, kind, st, active, false, s);
    }
}

/// The frame and caption background of a window `w` x `h` whose frame is
/// `frame` wide: the raised 3D frame of the classic styles, or the visual
/// style's frame with its caption.
#[allow(clippy::too_many_arguments)]
pub(crate) fn window_frame(
    window: &mut Window,
    o: Point<Pixels>,
    w: i32,
    h: i32,
    frame: i32,
    active: bool,
    top_level: bool,
    s: &Scheme,
) {
    if skin::skin() == Skin::Classic {
        paint::panel(window, o, Rect::new(0, 0, w, h), Edge::Raised, s);
        return;
    }
    // Below the caption only: the rounded corners above stay transparent.
    let top = frame + caption_h();
    paint::fill(window, o, Rect::new(0, top, w, h - top), s.of(Role::Face));
    skin::frame(window, o, (w, h), frame, caption_h(), active, top_level);
}

/// A caption button of `kind` in the active style (`st.enabled` false greys
/// it, as the close button of a Yes/No message box).
#[allow(clippy::too_many_arguments)]
pub(crate) fn caption_button_of(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    r: Rect,
    kind: CapButton,
    st: State2,
    active: bool,
    top_level: bool,
    s: &Scheme,
) {
    match skin::skin() {
        Skin::Xp(_)
        | Skin::Aero { .. }
        | Skin::Flat(_)
        | Skin::Fluent(_)
        | Skin::Mac(_)
        | Skin::Linux(_) => {
            skin::caption_button(window, o, r, kind, st, active && st.enabled, top_level);
        }
        Skin::Classic => {
            let mark = match kind {
                CapButton::Minimize => marks::caption_minimize(),
                CapButton::Maximize => marks::caption_maximize(),
                CapButton::Restore => marks::caption_restore(),
                CapButton::Close => marks::caption_close(),
            };
            let mode = if st.enabled {
                Mode::Normal
            } else {
                Mode::Disabled
            };
            caption_button_in(window, cx, o, r, &mark, st.down, mode, s);
        }
    }
}

/// The caption bar: colour, small icon and bold title (clipped to `title_w`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn caption(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    cap: Rect,
    title: &str,
    (active, top_level): (bool, bool),
    title_w: i32,
    s: &Scheme,
) {
    let (bg, fg) = if active {
        (s.active_caption, s.active_caption_text)
    } else {
        (s.inactive_caption, s.inactive_caption_text)
    };
    if let Some(m) = skin::mac_look() {
        // macOS: no window icon; the title centred over the whole title bar,
        // kept clear of the traffic lights (and as far from the right end).
        let keep = mdi::title_inset_of(super::style::metrics()).min(cap.w / 2);
        let tr = Rect::new(cap.x + keep, cap.y, cap.w - 2 * keep, cap.h);
        let clip = paint::bounds(o, tr);
        window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
            super::skin_mac::title(window, cx, o, (cap, tr), title, active, fg, m);
        });
        return;
    }
    if let Some(l) = skin::linux_look() {
        // The Linux desktops: their own icon place (none in Adwaita, the
        // window-menu button in Motif), then the title in `title_w`.
        if !super::skin_linux::caption_icon(window, o, cap, l) {
            crate::brand::paint(window, o, cap.x + 2, cap.y + (cap.h - 16) / 2, 16);
        }
        let left = if l.desk == super::skin_linux::Desk::Adwaita {
            0
        } else {
            cap.h + 2
        };
        let tr = Rect::new(cap.x + left, cap.y, (title_w - left).max(0), cap.h);
        let clip = paint::bounds(o, tr);
        window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
            super::skin_linux::title(window, cx, o, tr, title, active, l);
        });
        return;
    }
    if skin::skin() != Skin::Classic {
        crate::brand::paint(window, o, cap.x + 2, cap.y + (cap.h - 16) / 2, 16);
        let tr = Rect::new(cap.x + 21, cap.y, title_w - 21, cap.h);
        let clip = paint::bounds(o, tr);
        window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
            skin::title(
                window,
                cx,
                o,
                ((tr.x, cap.y + (cap.h - text::LINE) / 2), tr.w),
                title,
                active,
                top_level,
            );
        });
        return;
    }
    if super::style::active().style == WinStyle::Win98 {
        // Windows 98: the caption fades to its gradient colour on the right.
        let end = super::style::gradient_end(s, active);
        paint::gradient_h(window, o, cap, gpui::rgb(bg).into(), gpui::rgb(end).into());
    } else {
        paint::fill(window, o, cap, gpui::rgb(bg).into());
    }
    crate::brand::paint(window, o, cap.x + 1, cap.y + 1, 16);
    let tr = Rect::new(cap.x + 20, cap.y, title_w - 20, cap.h);
    let clip = paint::bounds(o, tr);
    window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
        let at = paint::at(o, tr.x, cap.y + (cap.h - text::LINE) / 2);
        text::draw_in(
            text::Face::Caption,
            window,
            cx,
            at,
            title,
            true,
            gpui::rgb(fg).into(),
        );
    });
}

/// One caption button with its mark drawn in `mode` (grey: disabled, as the
/// close button of a Yes/No message box).
#[allow(clippy::too_many_arguments)]
pub(crate) fn caption_button_in(
    window: &mut Window,
    cx: &mut App,
    o: Point<Pixels>,
    r: Rect,
    mark: &Glyph,
    down: bool,
    mode: Mode,
    s: &Scheme,
) {
    paint::panel(
        window,
        o,
        r,
        if down { Edge::ButtonDown } else { Edge::Button },
        s,
    );
    let d = i32::from(down);
    let my = if mark.h == 2 {
        r.h - 5
    } else {
        (r.h - mark.h) / 2
    };
    glyph::draw(
        window,
        cx,
        o,
        (r.x + (r.w - mark.w) / 2 + d, r.y + my + d),
        mark,
        mode,
        s,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    use sn_ui_core::theme::ThemeChoice;
    use sn_ui_core::winstyle::{Look, WinStyle};

    #[test]
    fn sizing_zones_match_the_hit_test_in_every_style() {
        let (w, h) = (300, 200);
        for style in WinStyle::ALL {
            super::super::style::set(
                Look {
                    style,
                    ..Look::default()
                },
                ThemeChoice::Classic,
            );
            let zones = border_zones(w, h);
            for y in 0..h {
                for x in 0..w {
                    let hit = mdi::hit(w, h, State::Normal, true, x, y);
                    let zone = zones
                        .iter()
                        .find(|(_, r, _)| r.contains(x, y))
                        .map(|(s, _, _)| *s);
                    match hit {
                        Some(Hit::Border(side)) => {
                            assert_eq!(zone, Some(side), "{style:?} ({x}, {y})");
                        }
                        _ => assert_eq!(zone, None, "{style:?} ({x}, {y}) is not a border"),
                    }
                }
            }
            let (min, max, close) = mdi::caption_buttons(w);
            for (r, want) in [
                (min, Hit::Minimize),
                (max, Hit::Maximize),
                (close, Hit::Close),
            ] {
                // Motif has no close button (its rectangle is empty).
                if r.is_empty() {
                    continue;
                }
                for (x, y) in [
                    (r.x + r.w / 2, r.y + r.h / 2),
                    (r.x, r.y),
                    (r.right() - 1, r.y),
                ] {
                    assert_eq!(
                        mdi::hit(w, h, State::Normal, true, x, y),
                        Some(want),
                        "{style:?} ({x}, {y})"
                    );
                    assert!(
                        zones.iter().all(|(_, z, _)| !z.contains(x, y)),
                        "{style:?}: a sizing zone covers {want:?} at ({x}, {y})"
                    );
                }
            }
        }
        super::super::style::set(Look::WIN95, ThemeChoice::Classic);
    }

    #[test]
    fn caption_tips_name_the_buttons_in_every_style_but_95() {
        use sn_ui_core::t;
        let rects = mdi::caption_buttons(300);
        for style in WinStyle::ALL {
            super::super::style::set(
                Look {
                    style,
                    ..Look::default()
                },
                ThemeChoice::Classic,
            );
            let mac = style.is_mac();
            assert_eq!(shows_caption_tips(), style != WinStyle::Win95, "{style:?}");
            let buttons = frame_buttons(rects, State::Normal, true);
            assert_eq!(
                caption_tips("t", &buttons).is_empty(),
                style == WinStyle::Win95,
                "{style:?}"
            );
            // The green light is Zoom whatever it does; Windows names the
            // maximize slot by its action.
            let zoom = t!("mdi-tip-zoom");
            let green = |want: &str| if mac { zoom.clone() } else { want.to_owned() };
            assert_eq!(
                caption_tip(CapButton::Maximize, true),
                green(&t!("mdi-tip-maximize")),
                "{style:?}"
            );
            assert_eq!(
                caption_tip(CapButton::Restore, true),
                green(&t!("mdi-tip-restore")),
                "{style:?}"
            );
            assert_eq!(
                caption_tip(CapButton::Restore, false),
                t!("mdi-tip-restore")
            );
            assert_eq!(
                caption_tip(CapButton::Minimize, false),
                t!("mdi-tip-minimize")
            );
            assert_eq!(caption_tip(CapButton::Close, false), t!("mdi-tip-close"));
        }
        super::super::style::set(Look::WIN95, ThemeChoice::Classic);
        let (min, max, close) = rects;
        let kinds = |state, resizable| -> Vec<(Rect, CapButton, &str)> {
            frame_buttons(rects, state, resizable)
        };
        assert_eq!(
            kinds(State::Normal, true),
            vec![
                (close, CapButton::Close, "close"),
                (max, CapButton::Maximize, "max"),
                (min, CapButton::Minimize, "min"),
            ]
        );
        assert_eq!(
            kinds(State::Maximized, true),
            vec![
                (close, CapButton::Close, "close"),
                (max, CapButton::Restore, "max"),
                (min, CapButton::Minimize, "min"),
            ]
        );
        assert_eq!(
            kinds(State::Minimized, true),
            vec![
                (close, CapButton::Close, "close"),
                (max, CapButton::Maximize, "max"),
                (min, CapButton::Restore, "min"),
            ]
        );
        for state in [State::Normal, State::Maximized, State::Minimized] {
            assert_eq!(
                kinds(state, false),
                vec![(close, CapButton::Close, "close")],
                "{state:?}"
            );
        }
    }
}
