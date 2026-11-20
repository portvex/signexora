//! Push buttons, check boxes and radio buttons.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled,
    Window, canvas,
};

use super::colors::{Role, Scheme, scheme};
use super::geom::{Edge, Rect, bevel_width};
use super::glyph::{self, Glyph, Mode};
use super::kit::{Action, Host, KeyHandler, Kit};
use super::skin::{self, Skin};
use super::text::{self, Ink, LINE, mnemonic};
use super::widget::{hovered, place, size_of};
use super::{marks, paint};

/// A value-change callback.
pub(crate) type Changed<V, T> = super::kit::Event<V, T>;

/// Common properties of a control.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ctl<'a> {
    /// Unique id (also the focus id).
    pub(crate) id: &'a str,
    /// Tab scope (the window or dialog the control belongs to).
    pub(crate) scope: &'a str,
    /// Position and size inside the parent.
    pub(crate) rect: Rect,
    /// Caption with an `&` mnemonic.
    pub(crate) label: &'a str,
    pub(crate) enabled: bool,
}

/// The ink of a caption: black, or embossed when disabled.
pub(crate) fn caption_ink(s: &Scheme, enabled: bool) -> Ink {
    if enabled {
        Ink::Color(s.of(Role::Text))
    } else {
        Ink::Embossed {
            light: s.of(Role::Highlight),
            dark: s.of(Role::Shadow),
        }
    }
}

/// The ink of a control's caption: as [`caption_ink`] in the classic styles;
/// the themed styles grey disabled text flat instead of embossing it.
pub(crate) fn control_ink(s: &Scheme, enabled: bool) -> Ink {
    if enabled || skin::skin() == Skin::Classic {
        caption_ink(s, enabled)
    } else {
        Ink::Color(super::colors::hsla(skin::text_color(false)))
    }
}

/// Space, or Enter for push buttons, runs `on_click` when the control has focus.
fn click_keys<V: Host>(on_click: Action<V>, enter: bool) -> KeyHandler<V> {
    Rc::new(move |v, ev, window, cx| {
        let k = ev.keystroke.key.as_str();
        if k == "space" || (enter && k == "enter") {
            on_click(v, window, cx);
            return true;
        }
        false
    })
}

/// The mouse handling shared by clickable controls: press on button down,
/// click when released over the control.
fn clickable<V: Host>(
    el: gpui::Stateful<gpui::Div>,
    id: &str,
    enabled: bool,
    on_click: Action<V>,
    cx: &mut Context<'_, V>,
) -> gpui::Stateful<gpui::Div> {
    if !enabled {
        return el;
    }
    let down_id = id.to_owned();
    let up_id = id.to_owned();
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut V, _, _, cx| {
            let kit = v.kit();
            kit.focus = Some(down_id.clone().into());
            kit.press(&down_id, None);
            cx.notify();
        }),
    )
    .on_mouse_up(
        MouseButton::Left,
        cx.listener(move |v: &mut V, _, window, cx| {
            if v.kit().is_pressed(&up_id) {
                on_click(v, window, cx);
                cx.notify();
            }
        }),
    )
}

/// A push button (75x23 in Windows 95 dialogs). `default` marks the dialog's
/// default button. As in Windows, the button that looks like the default
/// (the black frame, or the style's default look) is the focused push
/// button, or the default one while no push button of its dialog has the
/// focus, so a dialog never shows two.
pub(crate) fn push_button<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    default: bool,
    on_click: Action<V>,
) -> AnyElement {
    let s = scheme(cx);
    let held = kit.is_pressed(c.id);
    let focused = kit.has_focus(c.id);
    // Windows' rule: the focused push button is the default one; while the
    // focus is elsewhere, the dialog's default button is. Only one button of
    // a dialog ever looks like the default.
    let defaulted = focused || (default && !kit.push_button_focused_in(c.scope));
    kit.push_button_shown(c.id, c.scope);
    if c.enabled {
        kit.focusable(c.id, c.scope, Some(click_keys(on_click.clone(), true)));
    }
    let label = mnemonic(c.label);
    let enabled = c.enabled;
    let paint = canvas(super::widget::hover_hitbox, move |b, hitbox, window, cx| {
        let (w, h) = size_of(b);
        let down = held && hovered(window, b);
        let r = Rect::new(0, 0, w, h);
        let themed = skin::skin() != Skin::Classic;
        if themed {
            let st = skin::State {
                hot: hitbox.is_hovered(window),
                down,
                focus: defaulted,
                enabled,
            };
            skin::push_button(window, b.origin, r, st);
        } else {
            let edge = match (down, defaulted) {
                (true, _) => Edge::ButtonPressed,
                (false, true) => Edge::ButtonDefault,
                (false, false) => Edge::Button,
            };
            paint::panel(window, b.origin, r, edge, &s);
        }
        // Themed buttons do not move their caption when pressed.
        let d = i32::from(down && !themed);
        let tw = text::width(window, &label.text, false);
        let at = paint::at(b.origin, (w - tw) / 2 + d, (h - LINE) / 2 + d);
        text::draw_label(
            window,
            cx,
            at,
            &label,
            false,
            skin::button_text(skin::State {
                hot: false,
                down,
                focus: defaulted,
                enabled,
            })
            .map_or_else(
                || control_ink(&s, enabled),
                |c| Ink::Color(super::colors::hsla(c)),
            ),
            true,
        );
        if focused {
            let f = r.inset(bevel_width(Edge::ButtonDefault) + 1).offset(d, d);
            paint::focus_rect(window, b.origin, f, s.of(Role::Text));
        }
    })
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let el = place(c.rect)
        .id(gpui::ElementId::Name(c.id.to_owned().into()))
        .child(paint);
    clickable(el, c.id, c.enabled, on_click, cx).into_any_element()
}

/// Check box: a 13x13 sunken white box with a tick, then the caption.
pub(crate) fn check_box<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    checked: bool,
    on_change: Changed<V, bool>,
) -> AnyElement {
    let toggle: Action<V> = Rc::new(move |v, w, cx| on_change(v, !checked, w, cx));
    let tick = marks::check();
    toggle_control(
        kit,
        cx,
        c,
        toggle,
        move |window, cx, origin, y, (down, hot), s, enabled| {
            if skin::skin() != Skin::Classic {
                let st = skin::State {
                    hot,
                    down,
                    focus: false,
                    enabled,
                };
                skin::check(window, origin, (0, y), checked, st);
                return;
            }
            let bx = Rect::new(0, y, 13, 13);
            let inside = if down || !enabled {
                Role::Face
            } else {
                Role::Window
            };
            paint::fill(window, origin, bx, s.of(inside));
            paint::edge(window, origin, bx, Edge::Sunken, s);
            if checked {
                let mode = if enabled {
                    Mode::Normal
                } else {
                    Mode::Mono(s.shadow)
                };
                glyph::draw(window, cx, origin, (3, y + 3), &tick, mode, s);
            }
        },
    )
}

/// Radio button: a 12x12 round button with a dot, then the caption.
pub(crate) fn radio_button<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    selected: bool,
    on_select: Action<V>,
) -> AnyElement {
    let (white, grey, dot) = (radio(Role::Window), radio(Role::Face), radio_dot());
    toggle_control(
        kit,
        cx,
        c,
        on_select,
        move |window, cx, origin, y, (down, hot), s, enabled| {
            if skin::skin() != Skin::Classic {
                let st = skin::State {
                    hot,
                    down,
                    focus: false,
                    enabled,
                };
                skin::radio(window, origin, (0, y), selected, st);
                return;
            }
            let ring = if down || !enabled { &grey } else { &white };
            glyph::draw(window, cx, origin, (0, y), ring, Mode::Normal, s);
            if selected {
                let mode = if enabled {
                    Mode::Normal
                } else {
                    Mode::Mono(s.shadow)
                };
                glyph::draw(window, cx, origin, (4, y + 4), &dot, mode, s);
            }
        },
    )
}

/// A radio button (12x12): grey and black upper left, white and light grey
/// lower right, `inside` in the middle.
fn radio(inside: Role) -> Glyph {
    use glyph::Ink::Role as R;
    Glyph::new(12, 12).charmap(
        0,
        0,
        &[
            "....GGGG....",
            "..GGKKKKGG..",
            ".GKKWWWWKKH.",
            ".GKWWWWWWLH.",
            "GKWWWWWWWWLH",
            "GKWWWWWWWWLH",
            "GKWWWWWWWWLH",
            "GKWWWWWWWWLH",
            ".GKWWWWWWLH.",
            ".HLLWWWWLLH.",
            "..HHLLLLHH..",
            "....HHHH....",
        ],
        &[
            ('G', R(Role::Shadow)),
            ('K', R(Role::DarkShadow)),
            ('H', R(Role::Highlight)),
            ('L', R(Role::Light)),
            ('W', R(inside)),
        ],
    )
}

/// A check box or radio button: the mark drawn by `mark` at the left, the
/// caption 17 px right of it, and a focus rectangle around the caption.
fn toggle_control<V: Host>(
    kit: &mut Kit<V>,
    cx: &mut Context<'_, V>,
    c: Ctl<'_>,
    on_click: Action<V>,
    mark: impl Fn(
        &mut Window,
        &mut gpui::App,
        gpui::Point<gpui::Pixels>,
        i32,
        (bool, bool),
        &Scheme,
        bool,
    ) + 'static,
) -> AnyElement {
    let s = scheme(cx);
    let held = kit.is_pressed(c.id);
    let focused = kit.has_focus(c.id);
    if c.enabled {
        kit.focusable(c.id, c.scope, Some(click_keys(on_click.clone(), false)));
    }
    let label = mnemonic(c.label);
    let enabled = c.enabled;
    let paint = canvas(super::widget::hover_hitbox, move |b, hitbox, window, cx| {
        let (_, h) = size_of(b);
        let down = held && hovered(window, b);
        let y = (h - 13) / 2;
        mark(
            window,
            cx,
            b.origin,
            y,
            (down, hitbox.is_hovered(window)),
            &s,
            enabled,
        );
        let ty = (h - LINE) / 2;
        let tw = text::draw_label(
            window,
            cx,
            paint::at(b.origin, 17, ty),
            &label,
            false,
            control_ink(&s, enabled),
            true,
        );
        if focused {
            let f = Rect::new(15, ty - 1, tw + 4, LINE + 2);
            paint::focus_rect(window, b.origin, f, s.of(Role::Text));
        }
    })
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let el = place(c.rect)
        .id(gpui::ElementId::Name(c.id.to_owned().into()))
        .child(paint);
    clickable(el, c.id, c.enabled, on_click, cx).into_any_element()
}

/// The 4x4 dot of a selected radio button.
fn radio_dot() -> Glyph {
    Glyph::new(4, 4).bitmap(0, 0, &[" ## ", "####", "####", " ## "], glyph::BLACK)
}
