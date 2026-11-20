//! Drawing and mouse handling of menus: the menu bar, its drop-down menus,
//! submenus and right-click popup menus. The layout is computed from text
//! widths by pure functions.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Styled, Window, canvas, deferred,
};

use super::button::caption_ink;
use super::colors::{Role, Scheme, hsla, scheme};
use super::geom::{Edge, Rect};
use super::glyph::{self, Mode};
use super::kit::{Host, Lens};
use super::menu::{Check, Item, Kind, Menu, Outcome, Source, Tracker};
use super::text::{self, Ink, LINE, mnemonic};
use super::widget::{Slot, place, size_of};
use super::{marks, paint};

/// Height of the menu bar (`SM_CYMENU`).
pub(crate) fn bar_h() -> i32 {
    super::style::metrics().bar_h
}
/// Padding left and right of a menu title.
pub(crate) const TITLE_PAD: i32 = 6;
/// Height of a menu item.
pub(crate) fn item_h() -> i32 {
    super::style::metrics().item_h
}
/// Height of a separator.
pub(crate) const SEP_H: i32 = 9;
/// Border of a drop-down menu (raised edge and one pixel of face).
pub(crate) const BORDER: i32 = 3;
/// Column for check marks and 16x16 item pictures, left of the labels.
pub(crate) const LEFT: i32 = 22;
/// Column for the submenu arrow, right of the labels and shortcuts.
pub(crate) const RIGHT: i32 = 20;
/// Gap between the label and shortcut columns.
pub(crate) const GAP: i32 = 16;

/// Positions of the menu titles in a bar starting at `x0`.
pub(crate) fn bar_layout(menus: &[Menu], x0: i32, measure: &dyn Fn(&str) -> i32) -> Vec<Rect> {
    let mut x = x0;
    menus
        .iter()
        .map(|m| {
            let w = measure(&mnemonic(&m.label).text) + 2 * TITLE_PAD;
            let r = Rect::new(x, 1, w, bar_h() - 2);
            x += w;
            r
        })
        .collect()
}

/// Size and item rows of a drop-down menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopupLayout {
    pub(crate) w: i32,
    pub(crate) h: i32,
    /// Row of each item, relative to the menu's top-left corner.
    pub(crate) rows: Vec<Rect>,
    /// Where the shortcut column starts.
    pub(crate) shortcut_x: i32,
}

/// Lay out a drop-down menu.
pub(crate) fn popup_layout(items: &[Item], measure: &dyn Fn(&str) -> i32) -> PopupLayout {
    let label_w = items
        .iter()
        .map(|i| measure(&mnemonic(&i.label).text))
        .max()
        .unwrap_or(0);
    let key_w = items
        .iter()
        .map(|i| measure(&i.shortcut))
        .max()
        .unwrap_or(0);
    let keys = if key_w > 0 { GAP + key_w } else { 0 };
    let w = 2 * BORDER + LEFT + label_w + keys + RIGHT;
    let mut y = BORDER;
    let rows = items
        .iter()
        .map(|i| {
            let h = if i.is_separator() { SEP_H } else { item_h() };
            let r = Rect::new(BORDER, y, w - 2 * BORDER, h);
            y += h;
            r
        })
        .collect();
    PopupLayout {
        w,
        h: y + BORDER,
        rows,
        shortcut_x: BORDER + LEFT + label_w + GAP,
    }
}

/// Place a menu of size `w` x `h` at `(x, y)` inside a `vw` x `vh` window;
/// when it would leave the window it opens to the other side of `flip_x`
/// (the left edge of the parent menu for submenus) or moves up.
pub(crate) fn fit(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    flip_x: Option<i32>,
    vw: i32,
    vh: i32,
) -> (i32, i32) {
    let x = if x + w > vw {
        flip_x.map_or(vw - w, |fx| fx - w + BORDER)
    } else {
        x
    };
    let y = if y + h > vh { (vh - h).max(0) } else { y };
    (x.max(0), y)
}

/// A click outside the open menus: (window position, click count).
pub(crate) type OutsideClick<V> = super::kit::Event<V, ((i32, i32), usize)>;

/// The host's menu state and command handler.
pub(crate) struct MenuHooks<V> {
    pub(crate) tracker: Lens<V, Tracker>,
    pub(crate) on_invoke: super::kit::Event<V, u32>,
    /// A click outside the menus (window position, click count), before
    /// they close: a double click on a window icon that opened its system
    /// menu lands here.
    pub(crate) on_outside: Option<OutsideClick<V>>,
}

impl<V> Clone for MenuHooks<V> {
    fn clone(&self) -> Self {
        Self {
            tracker: self.tracker.clone(),
            on_invoke: self.on_invoke.clone(),
            on_outside: self.on_outside.clone(),
        }
    }
}

/// Menus shared with event handlers.
#[derive(Clone, Debug)]
pub(crate) enum Menus {
    Bar(Rc<Vec<Menu>>),
    Popup(Rc<Vec<Item>>),
}

impl Menus {
    fn source(&self) -> Source<'_> {
        match self {
            Self::Bar(m) => Source::Bar(m),
            Self::Popup(i) => Source::Popup(i),
        }
    }
}

/// The popup background of flat menus (`COLOR_MENU` of Luna: white).
const MENU_FLAT_BG: super::colors::Rgb = 0xFF_FF_FF;

/// The width of the icon gutter of Vista and 7 popup menus.
const AERO_GUTTER: i32 = LEFT + 4;

use super::skin::{MenuLook, menu_look};

thread_local! {
    /// Whether Alt is held, as the frame last saw it.
    static ALT_HELD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Note whether Alt is held (GNOME and KDE underline mnemonics only then).
/// Returns whether that changes what the menus draw, so the caller redraws.
pub(crate) fn set_alt_held(held: bool) -> bool {
    let was = ALT_HELD.with(|c| c.replace(held));
    was != held && matches!(menu_look(), MenuLook::Linux(_))
}

/// Whether Alt is noted as held (tests).
#[cfg(test)]
pub(crate) fn alt_held() -> bool {
    ALT_HELD.with(std::cell::Cell::get)
}

/// Whether menus in `look` underline their mnemonic letters (macOS never;
/// GNOME and KDE only while Alt is held or in menus entered from the
/// keyboard, `keyboard`).
fn underlines(look: MenuLook, keyboard: bool) -> bool {
    match look {
        MenuLook::Mac(_) => false,
        MenuLook::Linux(l) => {
            super::skin_linux::menu_underlines(l) || keyboard || ALT_HELD.with(std::cell::Cell::get)
        }
        _ => true,
    }
}

/// Width of a label in the UI font.
fn measurer(window: &Window) -> impl Fn(&str) -> i32 + '_ {
    move |s: &str| text::width_in(text::Face::Menu, window, s, false)
}

/// The menu bar at `r`, its titles starting at `x0`.
pub(crate) fn menu_bar<V: Host>(
    cx: &mut Context<'_, V>,
    window: &Window,
    r: Rect,
    x0: i32,
    menus: &Rc<Vec<Menu>>,
    track: &Tracker,
    hooks: &MenuHooks<V>,
) -> AnyElement {
    let s = scheme(cx);
    let titles = bar_layout(menus, x0, &measurer(window));
    let labels: Vec<_> = menus.iter().map(|m| mnemonic(&m.label)).collect();
    let (sel, open, keyboard) = (track.bar, track.is_open(), track.keyboard);
    let t2 = titles.clone();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            paint::fill(window, b.origin, Rect::new(0, 0, w, h), s.of(Role::Face));
            for (i, (t, l)) in t2.iter().zip(&labels).enumerate() {
                let look = menu_look();
                let mac_hot = sel == Some(i) && matches!(look, MenuLook::Mac(_));
                let flat_hot = sel == Some(i) && look == MenuLook::Flat || mac_hot;
                if let (true, MenuLook::Mac(m)) = (mac_hot, look) {
                    super::skin_mac::menu_highlight(window, b.origin, *t, m);
                } else if flat_hot {
                    paint::fill(window, b.origin, *t, s.of(Role::Selection));
                } else if sel == Some(i) && look == MenuLook::Aero {
                    super::skin_aero::menu_highlight(window, b.origin, *t);
                } else if let (true, MenuLook::Linux(lx)) = (sel == Some(i), look) {
                    super::skin_linux::menu_highlight(window, b.origin, *t, lx);
                } else if let (true, MenuLook::Fluent(f)) = (sel == Some(i), look) {
                    super::skin_fluent::menu_highlight(window, b.origin, *t, f);
                } else if let (true, MenuLook::Modern(f)) = (sel == Some(i), look) {
                    super::skin_flat::tool_button(window, b.origin, *t, open, f);
                } else if sel == Some(i) {
                    paint::edge(
                        window,
                        b.origin,
                        *t,
                        if open {
                            Edge::ThinSunken
                        } else {
                            Edge::ThinRaised
                        },
                        &s,
                    );
                }
                let at = paint::at(b.origin, t.x + TITLE_PAD, t.y + (t.h - LINE) / 2);
                let linux_selection_ink = matches!(look, MenuLook::Linux(lx) if !super::skin_linux::menu_keeps_text(lx))
                    && sel == Some(i);
                let ink = if flat_hot || linux_selection_ink {
                    Ink::Color(s.of(Role::SelectionText))
                } else {
                    caption_ink(&s, true)
                };
                let underline = underlines(look, keyboard);
                text::draw_label_in(text::Face::Menu, window, cx, at, l, false, ink, underline);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let measure = canvas(move |b, _, _| paint_slot.set(b), |_, (), _, _| {})
        .absolute()
        .top_0()
        .left_0()
        .size_full();
    let hit = move |slot: &Slot, p| {
        let (x, y) = slot.local(p)?;
        titles.iter().position(|t| t.contains(x, y))
    };
    let (h1, h2, s1, s2) = (hooks.clone(), hooks.clone(), slot.clone(), slot);
    let hit2 = hit.clone();
    place(r)
        .id("menubar")
        .child(paint)
        .child(measure)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |v: &mut V, ev: &MouseDownEvent, _, cx| {
                let Some(t) = (h1.tracker)(v) else { return };
                match hit(&s1, ev.position) {
                    Some(i) if t.bar == Some(i) && t.is_open() => t.close(),
                    Some(i) => t.open_bar(i),
                    None => t.close(),
                }
                cx.notify();
            }),
        )
        .on_mouse_move(cx.listener(move |v: &mut V, ev: &MouseMoveEvent, _, cx| {
            let Some(t) = (h2.tracker)(v) else { return };
            if let Some(i) = hit2(&s2, ev.position)
                && t.is_open()
                && t.bar.is_some()
                && t.bar != Some(i)
            {
                t.open_bar(i);
                cx.notify();
            }
        }))
        .into_any_element()
}

/// The open drop-down menus (or popup menu) of `menus`, the first at `root`
/// (window coordinates), with a click catcher below them that closes the menus.
pub(crate) fn menu_popups<V: Host>(
    cx: &mut Context<'_, V>,
    window: &Window,
    menus: &Menus,
    track: &Tracker,
    root: (i32, i32),
    catch_from_y: i32,
    hooks: &MenuHooks<V>,
) -> Option<AnyElement> {
    if !track.is_open() {
        return None;
    }
    let vp = window.viewport_size();
    let (vw, vh) = (f32::from(vp.width) as i32, f32::from(vp.height) as i32);
    let src = menus.source();
    let measure = measurer(window);
    let mut layer = gpui::div().absolute().top_0().left_0().size_full();
    let h0 = hooks.clone();
    layer = layer.child(
        place(Rect::new(0, catch_from_y, vw, vh - catch_from_y))
            .id("menucatch")
            .occlude()
            .on_any_mouse_down(
                cx.listener(move |v: &mut V, ev: &MouseDownEvent, window, cx| {
                    if let Some(t) = (h0.tracker)(v) {
                        t.close();
                    }
                    if let Some(f) = &h0.on_outside {
                        let at = (
                            f32::from(ev.position.x).round() as i32,
                            f32::from(ev.position.y).round() as i32,
                        );
                        f(v, (at, ev.click_count), window, cx);
                    }
                    cx.notify();
                }),
            ),
    );
    let (mut x, mut y, mut flip) = (root.0, root.1, None);
    for depth in 0..track.levels.len() {
        let Some(items) = track.items_at(src, depth) else {
            break;
        };
        let lay = popup_layout(items, &measure);
        let (px_, py) = fit(x, y, lay.w, lay.h, flip, vw, vh);
        let hot = track.levels.get(depth).copied().flatten();
        let rect = Rect::new(px_, py, lay.w, lay.h);
        let shown = Shown {
            depth,
            hot,
            keyboard: track.keyboard,
        };
        layer = layer.child(popup(cx, menus, items, &lay, rect, shown, hooks));
        let Some(row) = hot.and_then(|h| lay.rows.get(h)) else {
            break;
        };
        x = px_ + lay.w - BORDER;
        y = py + row.y - BORDER;
        flip = Some(px_);
    }
    Some(deferred(layer).with_priority(2).into_any_element())
}

/// The state of one drop-down level.
#[derive(Clone, Copy)]
struct Shown {
    /// The level (0 = the drop-down or popup root).
    depth: usize,
    /// Its highlighted item.
    hot: Option<usize>,
    /// The menus were entered from the keyboard.
    keyboard: bool,
}

/// One drop-down menu at `rect` (window coordinates).
fn popup<V: Host>(
    cx: &mut Context<'_, V>,
    menus: &Menus,
    items: &[Item],
    lay: &PopupLayout,
    rect: Rect,
    st: Shown,
    hooks: &MenuHooks<V>,
) -> AnyElement {
    let s = scheme(cx);
    let Shown {
        depth,
        hot,
        keyboard,
    } = st;
    let (shown, l2) = (items.to_vec(), lay.clone());
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let underline = underlines(menu_look(), keyboard);
            paint_popup(window, cx, b, &shown, &l2, (hot, underline), &s);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let measure = canvas(move |b, _, _| paint_slot.set(b), |_, (), _, _| {})
        .absolute()
        .top_0()
        .left_0()
        .size_full();
    let rows = lay.rows.clone();
    let row_at = move |slot: &Slot, p| -> Option<usize> {
        let (x, y) = slot.local(p)?;
        rows.iter().position(|r| r.contains(x, y))
    };
    let row_at2 = row_at.clone();
    let (h1, h2, m1, m2, s1, s2) = (
        hooks.clone(),
        hooks.clone(),
        menus.clone(),
        menus.clone(),
        slot.clone(),
        slot,
    );
    // Choosing an item on release: the left button, and for a popup menu
    // also the right one (Delphi's TPopupMenu passes TPM_RIGHTBUTTON).
    let choose = Rc::new(
        move |v: &mut V, ev: &MouseUpEvent, window: &mut Window, cx: &mut Context<'_, V>| {
            let Some(i) = row_at2(&s2, ev.position) else {
                return;
            };
            let Some(t) = (h2.tracker)(v) else { return };
            if let Outcome::Invoke(id) = t.choose(m2.source(), depth, i) {
                (h2.on_invoke)(v, id, window, cx);
            }
            cx.notify();
        },
    );
    let is_popup = matches!(menus, Menus::Popup(_));
    let seps: Vec<bool> = items.iter().map(Item::is_separator).collect();
    let choose_left = choose.clone();
    let el = place(rect)
        .id(gpui::ElementId::Name(format!("menu{depth}").into()))
        .occlude()
        .child(paint)
        .child(measure)
        .on_mouse_move(cx.listener(move |v: &mut V, ev: &MouseMoveEvent, _, cx| {
            let i = row_at(&s1, ev.position).filter(|&i| !seps.get(i).copied().unwrap_or(true));
            let Some(t) = (h1.tracker)(v) else { return };
            if t.levels.get(depth).copied().flatten() != i
                || t.levels.len() > depth + 1 && i.is_none()
            {
                t.hover(m1.source(), depth, i);
                cx.notify();
            }
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |v: &mut V, ev: &MouseUpEvent, window, cx| {
                choose_left(v, ev, window, cx);
            }),
        )
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
    if !is_popup {
        return el.into_any_element();
    }
    el.on_mouse_up(
        MouseButton::Right,
        cx.listener(move |v: &mut V, ev: &MouseUpEvent, window, cx| {
            choose(v, ev, window, cx);
        }),
    )
    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
    .into_any_element()
}

/// Paint a drop-down menu.
fn paint_popup(
    window: &mut Window,
    cx: &mut App,
    b: Bounds<Pixels>,
    items: &[Item],
    lay: &PopupLayout,
    (hot, underline): (Option<usize>, bool),
    s: &Scheme,
) {
    let o = b.origin;
    let look = menu_look();
    let flat = look == MenuLook::Flat;
    let all = Rect::new(0, 0, lay.w, lay.h);
    match look {
        // Windows XP's flat menus: the menu colour with a one-pixel border.
        MenuLook::Flat => {
            paint::fill(window, o, all, hsla(popup_bg(look, s)));
            super::skin::outline(window, o, all, s.of(Role::Shadow));
        }
        MenuLook::Aero => super::skin_aero::menu_popup(window, o, all, AERO_GUTTER),
        MenuLook::Fluent(f) => super::skin_fluent::menu_popup(window, o, all, f),
        MenuLook::Mac(m) => super::skin_mac::menu_popup(window, o, all, m),
        MenuLook::Linux(l) => super::skin_linux::menu_popup(window, o, all, l),
        MenuLook::Modern(f) => {
            let border = super::skin_flat::menu_colors(f).1;
            paint::fill(window, o, all, hsla(popup_bg(look, s)));
            super::skin::outline(window, o, all, hsla(border));
        }
        MenuLook::Classic => paint::panel(window, o, all, Edge::Raised, s),
    }
    for (i, (item, row)) in items.iter().zip(&lay.rows).enumerate() {
        if item.is_separator() && look == MenuLook::Aero {
            super::skin_aero::menu_separator(window, o, *row, AERO_GUTTER, row.y + SEP_H / 2 - 1);
            continue;
        }
        if let (true, MenuLook::Linux(l)) = (item.is_separator(), look) {
            super::skin_linux::menu_separator(window, o, *row, row.y + SEP_H / 2 - 1, l);
            continue;
        }
        if let (true, MenuLook::Mac(m)) = (item.is_separator(), look) {
            super::skin_mac::menu_separator(window, o, *row, row.y + SEP_H / 2, m);
            continue;
        }
        if let (true, MenuLook::Fluent(f)) = (item.is_separator(), look) {
            super::skin_fluent::menu_separator(window, o, *row, row.y + SEP_H / 2, f);
            continue;
        }
        if let (true, MenuLook::Modern(f)) = (item.is_separator(), look) {
            let sep = super::skin_flat::menu_colors(f).3;
            let y = row.y + SEP_H / 2;
            paint::fill(window, o, Rect::new(row.x + 2, y, row.w - 4, 1), hsla(sep));
            continue;
        }
        if item.is_separator() && flat {
            let y = row.y + SEP_H / 2;
            paint::fill(
                window,
                o,
                Rect::new(row.x + 1, y, row.w - 2, 1),
                s.of(Role::Shadow),
            );
            continue;
        }
        if item.is_separator() {
            let y = row.y + SEP_H / 2 - 1;
            paint::fill(
                window,
                o,
                Rect::new(row.x + 1, y, row.w - 2, 1),
                s.of(Role::Shadow),
            );
            paint::fill(
                window,
                o,
                Rect::new(row.x + 1, y + 1, row.w - 2, 1),
                s.of(Role::Highlight),
            );
            continue;
        }
        let is_hot = hot == Some(i);
        paint_item(
            window,
            cx,
            o,
            item,
            *row,
            lay.shortcut_x,
            (is_hot, underline),
            s,
        );
    }
}

/// The background of a popup menu drawn in `look`.
fn popup_bg(look: MenuLook, s: &Scheme) -> super::colors::Rgb {
    match look {
        MenuLook::Classic => s.face,
        MenuLook::Flat => MENU_FLAT_BG,
        MenuLook::Aero => super::skin_aero::MENU_BG,
        MenuLook::Modern(f) => super::skin_flat::menu_colors(f).0,
        MenuLook::Fluent(f) => super::skin_fluent::menu_bg(f),
        MenuLook::Mac(m) => super::skin_mac::menu_bg(m),
        MenuLook::Linux(l) => super::skin_linux::menu_bg(l),
    }
}

/// The text colour of a disabled item in a themed menu: the style's grey
/// for disabled text. `None` in classic menus, which emboss it.
fn disabled_ink(look: MenuLook) -> Option<super::colors::Rgb> {
    match look {
        MenuLook::Classic => None,
        MenuLook::Flat => Some(super::skin::XP_DISABLED_TEXT),
        MenuLook::Aero => Some(super::skin_aero::DISABLED_TEXT),
        MenuLook::Modern(f) => Some(super::skin_flat::text_color(f, false)),
        MenuLook::Fluent(f) => Some(super::skin_fluent::text_color(f, false)),
        MenuLook::Mac(m) => Some(super::skin_mac::text_color(m, false)),
        MenuLook::Linux(l) => Some(super::skin_linux::text_color(l, false)),
    }
}

/// The highlight of a hot popup menu item in the style's menu look.
fn paint_highlight(
    window: &mut Window,
    o: gpui::Point<Pixels>,
    row: Rect,
    look: MenuLook,
    s: &Scheme,
) {
    match look {
        MenuLook::Fluent(f) => super::skin_fluent::menu_highlight(window, o, row, f),
        MenuLook::Mac(m) => super::skin_mac::menu_highlight(window, o, row, m),
        MenuLook::Linux(l) => super::skin_linux::menu_highlight(window, o, row, l),
        MenuLook::Modern(f) => {
            let c = super::skin_flat::menu_colors(f).2;
            paint::fill(
                window,
                o,
                Rect::new(row.x + 1, row.y, row.w - 2, row.h),
                hsla(c),
            );
        }
        MenuLook::Aero => {
            super::skin_aero::menu_highlight(
                window,
                o,
                Rect::new(row.x + 1, row.y, row.w - 2, row.h),
            );
        }
        MenuLook::Flat | MenuLook::Classic => paint::fill(window, o, row, s.of(Role::Selection)),
    }
}

/// Paint one menu item in its row.
#[allow(clippy::too_many_arguments)]
fn paint_item(
    window: &mut Window,
    cx: &mut App,
    o: gpui::Point<Pixels>,
    item: &Item,
    row: Rect,
    key_x: i32,
    (hot, underline): (bool, bool),
    s: &Scheme,
) {
    let look = menu_look();
    // Vista and later keep the text colour on their light highlight.
    let aero = match look {
        MenuLook::Aero | MenuLook::Modern(_) | MenuLook::Fluent(_) => true,
        MenuLook::Linux(l) => super::skin_linux::menu_keeps_text(l),
        _ => false,
    };
    if hot {
        paint_highlight(window, o, row, look, s);
    }
    let ink = match (hot, item.enabled, disabled_ink(look)) {
        (true, true, _) if aero => Ink::Color(s.of(Role::Text)),
        (true, true, _) => Ink::Color(s.of(Role::SelectionText)),
        // Themed menus grey disabled items (in their style's grey, which
        // reads on their background) instead of embossing them.
        (_, false, Some(c)) => Ink::Color(hsla(c)),
        (true, false, None) => Ink::Color(s.of(Role::Shadow)),
        (false, e, _) => caption_ink(s, e),
    };
    let mark_mode = match (hot, item.enabled) {
        (true, true) if aero => Mode::Normal,
        (true, true) => Mode::Mono(s.selection_text),
        (_, false) => Mode::Disabled,
        (false, true) => Mode::Normal,
    };
    let mid = row.y + row.h / 2;
    let mark = match item.check {
        Check::Tick => Some(marks::check()),
        Check::Dot => Some(marks::menu_dot()),
        Check::None => None,
    };
    if let Some(m) = mark {
        glyph::draw(
            window,
            cx,
            o,
            (row.x + (LEFT - m.w) / 2, mid - m.h / 2),
            &m,
            mark_mode,
            s,
        );
    } else if let Some(g) = &item.glyph {
        let mode = if item.enabled {
            Mode::Normal
        } else {
            Mode::Disabled
        };
        glyph::draw(
            window,
            cx,
            o,
            (row.x + (LEFT - g.w) / 2, mid - g.h / 2),
            g,
            mode,
            s,
        );
    }
    let ty = row.y + (row.h - LINE) / 2;
    let label = mnemonic(&item.label);
    text::draw_label_in(
        text::Face::Menu,
        window,
        cx,
        paint::at(o, row.x + LEFT, ty),
        &label,
        false,
        ink,
        underline,
    );
    if !item.shortcut.is_empty() {
        let key = mnemonic(&item.shortcut.replace('&', "&&"));
        text::draw_label_in(
            text::Face::Menu,
            window,
            cx,
            paint::at(o, key_x, ty),
            &key,
            false,
            ink,
            false,
        );
    }
    if matches!(item.kind, Kind::Submenu(_)) {
        let a = marks::submenu_arrow();
        glyph::draw(
            window,
            cx,
            o,
            (row.right() - 4 - a.w - 4, mid - a.h / 2),
            &a,
            mark_mode,
            s,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &str) -> i32 {
        s.chars().count() as i32 * 6
    }

    #[test]
    fn gnome_and_kde_underline_mnemonics_while_alt_is_held() {
        use super::super::skin_linux::Lin;
        use sn_ui_core::winstyle::{Look, WinStyle};
        let lin = |style| {
            Lin::of(Look {
                style,
                ..Look::default()
            })
        };
        let (Some(adwaita), Some(motif)) = (lin(WinStyle::LinuxAdwaita), lin(WinStyle::LinuxMotif))
        else {
            panic!("Linux looks");
        };
        ALT_HELD.with(|c| c.set(false));
        assert!(!underlines(MenuLook::Linux(adwaita), false));
        assert!(underlines(MenuLook::Linux(motif), false), "Motif always");
        ALT_HELD.with(|c| c.set(true));
        assert!(underlines(MenuLook::Linux(adwaita), false));
        ALT_HELD.with(|c| c.set(false));
        // Alt tapped and released: the bar is in keyboard mode, Alt is up.
        let m = vec![Menu {
            label: "&File".into(),
            items: vec![Item::command("&Open", 1)],
        }];
        let mut t = Tracker::default();
        t.key(Source::Bar(&m), super::super::menu::Key::Alt);
        assert!(underlines(MenuLook::Linux(adwaita), t.keyboard));
        t.open_bar(0);
        assert!(!underlines(MenuLook::Linux(adwaita), t.keyboard), "mouse");
    }

    #[test]
    fn disabled_items_read_on_every_menu_background() {
        use super::super::colors::contrast;
        use super::super::skin_flat::Flat;
        let s = Scheme::STANDARD;
        let mut looks = vec![MenuLook::Flat, MenuLook::Aero];
        for (win10, dark) in [(false, false), (true, false), (true, true)] {
            looks.push(MenuLook::Modern(Flat {
                win10,
                dark,
                accent: 0x00_78_D7,
            }));
        }
        for look in looks {
            let ink = disabled_ink(look).unwrap_or(0);
            let bg = popup_bg(look, &s);
            assert!(contrast(ink, bg) >= 2.5, "{look:?}: {ink:06X} on {bg:06X}");
        }
        assert_eq!(disabled_ink(MenuLook::Classic), None, "classic embosses");
        assert_eq!(popup_bg(MenuLook::Classic, &s), s.face);
    }

    #[test]
    fn bar_titles_are_padded_and_adjacent() {
        let menus = vec![
            Menu {
                label: "&File".into(),
                items: vec![],
            },
            Menu {
                label: "&Windows".into(),
                items: vec![],
            },
        ];
        let t = bar_layout(&menus, 0, &w);
        assert_eq!(t, vec![Rect::new(0, 1, 36, 16), Rect::new(36, 1, 54, 16)]);
    }

    #[test]
    fn popup_rows_and_width() {
        let items = vec![
            Item::command("&Open file...", 1).shortcut("Ctrl+O"),
            Item::separator(),
            Item::command("E&xit", 2),
        ];
        let lay = popup_layout(&items, &w);
        // label 12 chars = 72, shortcut 6 chars = 36.
        assert_eq!(lay.w, 2 * BORDER + LEFT + 72 + GAP + 36 + RIGHT);
        assert_eq!(lay.rows[0], Rect::new(3, 3, lay.w - 6, item_h()));
        assert_eq!(lay.rows[1].h, SEP_H);
        assert_eq!(lay.rows[2].y, 3 + item_h() + SEP_H);
        assert_eq!(lay.h, 3 + 2 * item_h() + SEP_H + 3);
        assert_eq!(lay.shortcut_x, BORDER + LEFT + 72 + GAP);
        let plain = popup_layout(&[Item::command("Ab", 1)], &w);
        assert_eq!(
            plain.w,
            2 * BORDER + LEFT + 12 + RIGHT,
            "no shortcut column"
        );
    }

    #[test]
    fn menus_stay_in_the_window() {
        assert_eq!(fit(10, 20, 100, 50, None, 800, 600), (10, 20));
        assert_eq!(fit(750, 20, 100, 50, None, 800, 600), (700, 20));
        assert_eq!(
            fit(750, 20, 100, 50, Some(650), 800, 600),
            (553, 20),
            "submenus open to the left"
        );
        assert_eq!(fit(10, 580, 100, 50, None, 800, 600), (10, 550));
        assert_eq!(fit(10, 10, 100, 900, None, 800, 600), (10, 0));
    }
}
