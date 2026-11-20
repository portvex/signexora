//! The WF window's top toolbar (`docs/SA_UI_SPEC.md` 7.1, SA 6.2.5.4): bold
//! `LPF` caption with its editable combo, `<==`, the `RRC` caption with its
//! roll-off field and slider, the short-interval frequency field `0-0`, the
//! light blue `- VMW -` button, and the `Sum` and `Info` check boxes. The
//! groups wrap to a second row when the window is narrow (as `sa-app`'s
//! wrapped toolbar).
//!
//! The small widgets SA's WF and VMW bars share live here too: sunken caption
//! groups, the mode button, the bold text button, the slider and the field
//! with a drop button.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, StatefulInteractiveElement, Styled, Window, canvas,
};

use sn_ui_core::{format, t, wf_tools};

use super::{Frame, ROW_H, ink};
use crate::app::menus::cmd;
use crate::win95::bars::tooltip;
use crate::win95::button::{Changed, Ctl, check_box};
use crate::win95::colors::{Role, scheme};
use crate::win95::edit::{FieldLook, edit};
use crate::win95::geom::{Edge, Rect};
use crate::win95::glyph::{self, Mode};
use crate::win95::kit::{Action, Lens, action, lens};
use crate::win95::label::{Align, label};
use crate::win95::marks::{self, Dir};
use crate::win95::menu::Item;
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, hovered, place, size_of};

/// Height of a group panel.
pub(crate) const GROUP_H: i32 = 26;
/// Height of a control in a group.
pub(crate) const CTL_H: i32 = 21;
/// Gap between groups.
pub(crate) const GAP: i32 = 3;
/// Width of a field's drop button.
const DROP_W: i32 = 16;
/// Width of the LPF field with its drop button.
const LPF_W: i32 = 78;
/// Width of the RRC roll-off field and of its slider.
const ALPHA_W: i32 = 36;
const SLIDER_W: i32 = 120;
/// Width of the frequency field.
const FREQ_W: i32 = 150;
/// Width of the `- VMW -` button.
const VMW_W: i32 = 90;

/// Builds a toolbar item in its row slot.
pub(crate) type BuildFn<'a> =
    Box<dyn FnOnce(&mut Frame, Rect, &mut Context<'_, Frame>) -> AnyElement + 'a>;
/// Opens a list at a window position.
pub(crate) type OpenAt = Rc<dyn Fn(&mut Frame, (i32, i32))>;

/// One item of a toolbar row: its width and how it is built at `x, y`.
pub(crate) struct BarItem<'a> {
    pub(crate) w: i32,
    /// Extra space before it.
    pub(crate) before: i32,
    pub(crate) build: BuildFn<'a>,
}

/// Place `items` left to right from `x0`, wrapping to a new row when one does
/// not fit in `cw`. Returns the elements and the number of rows.
pub(crate) fn flow(
    f: &mut Frame,
    items: Vec<BarItem<'_>>,
    cw: i32,
    cx: &mut Context<'_, Frame>,
) -> (Vec<AnyElement>, i32) {
    let (mut x, mut row) = (2, 0);
    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let mut at = x + it.before;
        if at + it.w > cw - 2 && x > 2 {
            row += 1;
            at = 2;
        }
        let y = row * ROW_H;
        let r = Rect::new(at, y, it.w, ROW_H);
        out.push((it.build)(f, r, cx));
        x = at + it.w + GAP;
    }
    (out, row + 1)
}

/// The sunken frame of a caption group at `r` (the group's full rectangle).
pub(crate) fn group_panel(cx: &gpui::App, r: Rect) -> AnyElement {
    let s = scheme(cx);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, _| {
            let (w, h) = size_of(b);
            paint::edge(
                window,
                b.origin,
                Rect::new(0, 0, w, h),
                Edge::ThinSunken,
                &s,
            );
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// A row slot's group rectangle: `GROUP_H` high, centred in the row.
pub(crate) fn group_rect(r: Rect) -> Rect {
    Rect::new(r.x, r.y + (ROW_H - GROUP_H) / 2, r.w, GROUP_H)
}

/// A control's rectangle `w` wide at `x` in a row slot.
pub(crate) fn ctl_rect(r: Rect, x: i32, w: i32) -> Rect {
    Rect::new(x, r.y + (ROW_H - CTL_H) / 2, w, CTL_H)
}

/// Make `el` a button of the kit: pressed on button down, `on_click` when
/// released over it.
pub(crate) fn clickable(
    el: gpui::Stateful<gpui::Div>,
    id: &str,
    on_click: Action<Frame>,
    cx: &mut Context<'_, Frame>,
) -> gpui::Stateful<gpui::Div> {
    let (down_id, up_id) = (id.to_owned(), id.to_owned());
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut Frame, _, _, cx| {
            v.kit.focus = None;
            v.kit.press(&down_id, None);
            cx.notify();
        }),
    )
    .on_mouse_up(
        MouseButton::Left,
        cx.listener(move |v: &mut Frame, _, window, cx| {
            if v.kit.is_pressed(&up_id) {
                on_click(v, window, cx);
                cx.notify();
            }
        }),
    )
}

/// SA's light blue mode button (`- VMW -`, `WaveForm`) with bold dark blue text.
pub(crate) fn mode_button(
    f: &Frame,
    cx: &mut Context<'_, Frame>,
    id: &str,
    r: Rect,
    caption: &str,
    tip: &str,
    on_click: Action<Frame>,
) -> AnyElement {
    let s = scheme(cx);
    let wink = f.wf_ink(cx);
    let (face, fg) = (ink(wink.mode_face), ink(wink.mode_text));
    let held = f.kit.is_pressed(id);
    let caption = caption.to_owned();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let down = held && hovered(window, b);
            let rr = Rect::new(0, 0, w, h);
            paint::fill(window, b.origin, rr, face);
            let edge = if down {
                Edge::ButtonPressed
            } else {
                Edge::Button
            };
            paint::edge(window, b.origin, rr, edge, &s);
            let tw = text::width(window, &caption, true);
            let d = i32::from(down);
            let at = paint::at(b.origin, (w - tw) / 2 + d, (h - LINE) / 2 + d);
            text::draw(window, cx, at, &caption, true, fg);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let el = place(r)
        .id(gpui::ElementId::Name(id.to_owned().into()))
        .tooltip(tooltip(tip))
        .child(paint);
    clickable(el, id, on_click, cx).into_any_element()
}

/// A flat bold text button (SA's `<==`): grey text while disabled.
pub(crate) fn text_button(
    f: &Frame,
    cx: &mut Context<'_, Frame>,
    id: &str,
    r: Rect,
    caption: &str,
    enabled: bool,
    tip: &str,
    on_click: Action<Frame>,
) -> AnyElement {
    let s = scheme(cx);
    let held = f.kit.is_pressed(id);
    let caption = caption.to_owned();
    let paint = canvas(
        crate::win95::widget::hover_hitbox,
        move |b, hitbox, window, cx| {
            let (w, h) = size_of(b);
            let over = hitbox.is_hovered(window) && enabled;
            let down = held && over;
            let rr = Rect::new(0, 0, w, h);
            if down {
                paint::edge(window, b.origin, rr, Edge::ThinSunken, &s);
            } else if over {
                paint::edge(window, b.origin, rr, Edge::ThinRaised, &s);
            }
            let tw = text::width(window, &caption, true);
            let d = i32::from(down);
            let at = paint::at(b.origin, (w - tw) / 2 + d, (h - LINE) / 2 + d);
            let ink = crate::win95::button::caption_ink(&s, enabled);
            let label = text::mnemonic(&caption.replace('&', "&&"));
            text::draw_label(window, cx, at, &label, true, ink, false);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let el = place(r)
        .id(gpui::ElementId::Name(id.to_owned().into()))
        .tooltip(tooltip(tip))
        .child(paint)
        .on_hover(cx.listener(|_, _: &bool, _, cx| cx.notify()));
    if enabled {
        clickable(el, id, on_click, cx).into_any_element()
    } else {
        el.into_any_element()
    }
}

/// A horizontal slider drawn as SA's scroll bar.
pub(crate) fn slider(
    f: &Frame,
    cx: &mut Context<'_, Frame>,
    id: &str,
    r: Rect,
    st: ScrollState,
    l: Lens<Frame, ScrollState>,
    on_change: Action<Frame>,
) -> AnyElement {
    scroll_bar(
        &f.kit,
        cx,
        Bar {
            id,
            rect: r,
            vertical: false,
        },
        st,
        l,
        Some(on_change),
    )
}

/// The drop button at the right end of field `r` (the LPF presets).
pub(crate) fn drop_button(
    f: &Frame,
    cx: &mut Context<'_, Frame>,
    id: &str,
    r: Rect,
    tip: &str,
    open: OpenAt,
) -> AnyElement {
    let s = scheme(cx);
    let held = f.kit.is_pressed(id);
    let slot = Slot::default();
    let paint_slot = slot.clone();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            paint_slot.set(b);
            let (w, h) = size_of(b);
            let down = held && hovered(window, b);
            let edge = if down {
                Edge::Flat(Role::Shadow)
            } else {
                Edge::Raised
            };
            paint::panel(window, b.origin, Rect::new(0, 0, w, h), edge, &s);
            let m = marks::arrow(Dir::Down, 7);
            let d = i32::from(down);
            let at = ((w - m.w) / 2 + d, (h - m.h) / 2 + d);
            glyph::draw(window, cx, b.origin, at, &m, Mode::Normal, &s);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let drop = Rect::new(r.right() - 2 - DROP_W, r.y + 2, DROP_W, r.h - 4);
    let width = r.w;
    let press_id = id.to_owned();
    place(drop)
        .id(gpui::ElementId::Name(id.to_owned().into()))
        .tooltip(tooltip(tip))
        .child(paint)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |v: &mut Frame, _: &MouseDownEvent, _, cx| {
                v.kit.press(&press_id, None);
                if let Some(b) = slot.bounds() {
                    let at = (
                        f32::from(b.origin.x).round() as i32 - (width - DROP_W - 2),
                        f32::from(b.origin.y + b.size.height).round() as i32 + 2,
                    );
                    open(v, at);
                }
                cx.notify();
            }),
        )
        .into_any_element()
}

/// A bold caption with a check box in a sunken group.
pub(crate) fn check_group(
    f: &mut Frame,
    cx: &mut Context<'_, Frame>,
    win: u32,
    name: &str,
    caption: &str,
    tip: &str,
    r: Rect,
    checked: bool,
    on: Changed<Frame, bool>,
) -> AnyElement {
    let cap_w = r.w - 4 - 4 - 14 - 4;
    let id = format!("wf{win}.{name}");
    let scope = format!("mdi{win}");
    let cap = label(
        cx,
        ctl_rect(r, r.x + 4, cap_w),
        caption,
        true,
        true,
        Align::Left,
    );
    let ctl = Ctl {
        id: &id,
        scope: &scope,
        rect: ctl_rect(r, r.x + 4 + cap_w + 4, 14),
        label: "",
        enabled: true,
    };
    let check = check_box(&mut f.kit, cx, ctl, checked, on);
    let tip_el = place(group_rect(r))
        .id(gpui::ElementId::Name(format!("wf{win}.{name}.tip").into()))
        .tooltip(tooltip(tip));
    gpui::div()
        .absolute()
        .size_full()
        .child(group_panel(cx, group_rect(r)))
        .child(tip_el)
        .child(cap)
        .child(check)
        .into_any_element()
}

/// The width of a check group with `caption`.
pub(crate) fn check_group_w(window: &Window, caption: &str) -> i32 {
    4 + text::width(window, caption, true) + 4 + 14 + 4
}

/// A grey read-only field look (SA's indicator fields).
pub(crate) fn grey_look(cx: &gpui::App) -> FieldLook {
    let s = scheme(cx);
    FieldLook {
        bg: s.rgb(Role::Face),
        fg: s.rgb(Role::Text),
        border: true,
    }
}

/// A sunken read-only text field that runs `on_click` when clicked.
pub(crate) fn click_field(
    cx: &mut Context<'_, Frame>,
    id: &str,
    r: Rect,
    value: &str,
    tip: &str,
    on_click: Action<Frame>,
) -> AnyElement {
    let s = scheme(cx);
    let value = value.to_owned();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let rr = Rect::new(0, 0, w, h);
            paint::fill(window, b.origin, rr, s.of(Role::Face));
            paint::edge(window, b.origin, rr, Edge::Sunken, &s);
            let clip = paint::bounds(b.origin, rr.inset(2));
            window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
                let at = paint::at(b.origin, 4, (h - LINE) / 2);
                text::draw(window, cx, at, &value, false, s.of(Role::Text));
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r)
        .id(gpui::ElementId::Name(id.to_owned().into()))
        .tooltip(tooltip(tip))
        .child(paint)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |v: &mut Frame, _: &MouseDownEvent, window, cx| {
                v.kit.focus = None;
                on_click(v, window, cx);
                cx.notify();
            }),
        )
        .into_any_element()
}

impl Frame {
    /// The latest measured Br.
    pub(crate) fn wf_br(&self) -> Option<f64> {
        self.measured(&sn_core::MeasurementKind::BaudRate)
            .map(|m| m.value)
    }

    /// Bring the RRC controls of window `win` up to date.
    fn wf_sync_rrc(&mut self, win: u32, cx: &gpui::App) {
        let look = grey_look(cx);
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        let alpha = wf_tools::rrc_alpha(ww.alpha);
        let text = format::dec(alpha, 2);
        if ww.alpha_field.text != text {
            ww.alpha_field.set_text(&text);
        }
        ww.alpha_field.read_only = true;
        ww.alpha_field.look = Some(look);
        let pos = (wf_tools::slider_of_alpha(alpha) * 95.0).round() as i32;
        ww.alpha_bar = ScrollState::new(0, 95 + 10 - 1, 10, pos);
    }

    /// The RRC slider moved.
    fn wf_alpha_moved(&mut self, win: u32) {
        if let Some(ww) = self.wf.wins.get_mut(&win) {
            ww.alpha = wf_tools::alpha_of_slider(f64::from(ww.alpha_bar.pos) / 95.0);
        }
    }

    /// The top toolbar of WF window `win`, `cw` wide. Returns its elements
    /// and its number of rows.
    pub(crate) fn wf_top_toolbar(
        &mut self,
        win: u32,
        cw: i32,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> (Vec<AnyElement>, i32) {
        self.wf_sync_rrc(win, cx);
        let br = self.wf_br();
        let lpf_cap = t!("wf-lpf");
        let lpf_w = 4 + text::width(window, &lpf_cap, true) + 4 + LPF_W + 4;
        let rrc_cap = t!("wf-rrc");
        let rrc_cap_w = text::width(window, &rrc_cap, true);
        let rrc_w = 4 + rrc_cap_w + 4 + ALPHA_W + 4 + SLIDER_W + 4;
        let copy_w = text::width(window, &t!("wf-copy-br"), true) + 10;
        let sum_cap = t!("wf-sum");
        let info_cap = t!("sw-info");
        let items = vec![
            BarItem {
                w: lpf_w,
                before: 0,
                build: Box::new(move |f, r, cx| f.wf_lpf_group(win, r, &lpf_cap, cx)),
            },
            BarItem {
                w: copy_w,
                before: 0,
                build: Box::new(move |f, r, cx| f.wf_copy_br_button(win, r, br, cx)),
            },
            BarItem {
                w: rrc_w,
                before: 0,
                build: Box::new(move |f, r, cx| f.wf_rrc_group(win, r, &rrc_cap, rrc_cap_w, cx)),
            },
            BarItem {
                w: FREQ_W,
                before: 0,
                build: Box::new(move |f, r, cx| f.wf_freq_field(win, r, cx)),
            },
            BarItem {
                w: VMW_W,
                before: 6,
                build: Box::new(move |f, r, cx| {
                    let id = format!("wf{win}.vmw");
                    let go = action(move |v: &mut Frame, _, _| {
                        v.wf_set_window_kind(win, crate::app::model::WinKind::Raster);
                    });
                    let caption = format!("- {} -", t!("wf-vmw"));
                    f.wf_hot(win, "vmw", ctl_rect(r, r.x, r.w));
                    mode_button(
                        f,
                        cx,
                        &id,
                        ctl_rect(r, r.x, r.w),
                        &caption,
                        &t!("wf-vmw.hover"),
                        go,
                    )
                }),
            },
            BarItem {
                w: check_group_w(window, &sum_cap),
                before: 6,
                build: Box::new(move |f, r, cx| {
                    let on = f.wf.wins.get(&win).is_some_and(|w| w.sum);
                    let set: Changed<Frame, bool> = Rc::new(move |v: &mut Frame, on, _, _| {
                        if let Some(w) = v.wf.wins.get_mut(&win) {
                            w.sum = on;
                        }
                    });
                    check_group(f, cx, win, "sum", &sum_cap, &t!("wf-sum.hover"), r, on, set)
                }),
            },
            BarItem {
                w: check_group_w(window, &info_cap),
                before: 0,
                build: Box::new(move |f, r, cx| {
                    let on = f.wf.wins.get(&win).is_some_and(|w| w.info);
                    let set: Changed<Frame, bool> = Rc::new(move |v: &mut Frame, on, _, _| {
                        if let Some(w) = v.wf.wins.get_mut(&win) {
                            w.info = on;
                        }
                    });
                    check_group(
                        f,
                        cx,
                        win,
                        "info",
                        &info_cap,
                        &t!("wf-info.hover"),
                        r,
                        on,
                        set,
                    )
                }),
            },
        ];
        flow(self, items, cw, cx)
    }

    /// `LPF` and its editable combo.
    fn wf_lpf_group(
        &mut self,
        win: u32,
        r: Rect,
        caption: &str,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let cap_w = r.w - 4 - 4 - LPF_W - 4;
        let st = self
            .wf
            .wins
            .get(&win)
            .map(|w| w.lpf.clone())
            .unwrap_or_default();
        let id = format!("wf{win}.lpf");
        let scope = format!("mdi{win}");
        let field_r = ctl_rect(r, r.x + 4 + cap_w + 4, LPF_W);
        let field = edit(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: field_r,
                label: "",
                enabled: true,
            },
            &st,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.lpf)),
            None,
        );
        let open: OpenAt = Rc::new(move |v: &mut Frame, at| v.wf_open_lpf_presets(win, at));
        let drop = drop_button(
            self,
            cx,
            &format!("wf{win}.lpf.drop"),
            field_r,
            &t!("edits-presets"),
            open,
        );
        let tip_el = place(Rect::new(
            field_r.x,
            field_r.y,
            field_r.w - DROP_W - 2,
            field_r.h,
        ))
        .id(gpui::ElementId::Name(format!("wf{win}.lpf.tip").into()))
        .tooltip(tooltip(&t!("wf-lpf.hover")));
        gpui::div()
            .absolute()
            .size_full()
            .child(group_panel(cx, group_rect(r)))
            .child(label(
                cx,
                ctl_rect(r, r.x + 4, cap_w),
                caption,
                true,
                true,
                Align::Left,
            ))
            .child(tip_el)
            .child(field)
            .child(drop)
            .into_any_element()
    }

    /// Open the LPF preset list of window `win` at window position `at`.
    pub(crate) fn wf_open_lpf_presets(&mut self, win: u32, at: (i32, i32)) {
        let items = (0_u32..)
            .zip(Self::wf_lpf_presets())
            .map(|(i, p)| Item::command(&p, cmd::WF_LPF_PRESET + i))
            .collect();
        self.open_popup(items, at);
        self.wf.popup_win = Some(win);
    }

    /// `<==`: copy the measured Br into the LPF field (only with a Br).
    fn wf_copy_br_button(
        &mut self,
        win: u32,
        r: Rect,
        br: Option<f64>,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let tip = br.map_or_else(
            || t!("wf-copy-br-missing"),
            |b| t!("wf-copy-br-value", br = format::dec(b, 2)),
        );
        let go = action(move |v: &mut Frame, _, _| {
            if let Some(w) = v.wf.wins.get_mut(&win)
                && br.is_some()
            {
                w.lpf.set_text(&wf_tools::lpf_default(br));
            }
        });
        let id = format!("wf{win}.copybr");
        text_button(
            self,
            cx,
            &id,
            ctl_rect(r, r.x, r.w),
            &t!("wf-copy-br"),
            br.is_some(),
            &tip,
            go,
        )
    }

    /// `RRC` (greyed unless on; a click switches it), its roll-off and slider.
    fn wf_rrc_group(
        &mut self,
        win: u32,
        r: Rect,
        caption: &str,
        cap_w: i32,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let Some((on, field, bar)) = self
            .wf
            .wins
            .get(&win)
            .map(|w| (w.rrc, w.alpha_field.clone(), w.alpha_bar))
        else {
            return gpui::Empty.into_any_element();
        };
        let cap_r = ctl_rect(r, r.x + 4, cap_w);
        let toggle = action(move |v: &mut Frame, _, _| {
            if let Some(w) = v.wf.wins.get_mut(&win) {
                w.rrc = !w.rrc;
            }
        });
        let cap_id = format!("wf{win}.rrc");
        let cap = clickable(
            place(cap_r)
                .id(gpui::ElementId::Name(cap_id.clone().into()))
                .tooltip(tooltip(&t!("wf-rrc.hover")))
                .child(label(
                    cx,
                    Rect::new(0, 0, cap_w, CTL_H),
                    caption,
                    true,
                    on,
                    Align::Left,
                )),
            &cap_id,
            toggle,
            cx,
        );
        let id = format!("wf{win}.alpha");
        let scope = format!("mdi{win}");
        let field_r = ctl_rect(r, cap_r.right() + 4, ALPHA_W);
        let field_el = edit(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: field_r,
                label: "",
                enabled: true,
            },
            &field,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.alpha_field)),
            None,
        );
        let bar_r = Rect::new(
            field_r.right() + 4,
            r.y + (ROW_H - THICKNESS) / 2,
            SLIDER_W,
            THICKNESS,
        );
        let bar_el = slider(
            self,
            cx,
            &format!("wf{win}.alphabar"),
            bar_r,
            bar,
            lens(move |v: &mut Self| v.wf.wins.get_mut(&win).map(|w| &mut w.alpha_bar)),
            action(move |v: &mut Self, _, _| v.wf_alpha_moved(win)),
        );
        let tip_el = place(field_r)
            .id(gpui::ElementId::Name(format!("wf{win}.alpha.tip").into()))
            .tooltip(tooltip(&t!("wf-rrc-alpha.hover")));
        gpui::div()
            .absolute()
            .size_full()
            .child(group_panel(cx, group_rect(r)))
            .child(cap)
            .child(field_el)
            .child(tip_el)
            .child(bar_el)
            .into_any_element()
    }

    /// The short-interval frequency field: a click measures the V fragment.
    fn wf_freq_field(&mut self, win: u32, r: Rect, cx: &mut Context<'_, Self>) -> AnyElement {
        let value = self
            .wf
            .wins
            .get(&win)
            .and_then(|w| w.freq.clone())
            .unwrap_or_else(|| t!("wf-freq"));
        let id = format!("wf{win}.freq");
        let go = action(move |v: &mut Frame, _, _| v.wf_measure_freq(win));
        click_field(
            cx,
            &id,
            ctl_rect(r, r.x, r.w),
            &value,
            &t!("wf-freq.hover"),
            go,
        )
    }

    /// A click on the frequency field: `<frequency>-<periods>` of the V
    /// fragment, or SA's `Please check settings!`.
    pub(crate) fn wf_measure_freq(&mut self, win: u32) {
        let Some(ww) = self.wf.wins.get(&win) else {
            return;
        };
        let markers = ww.markers;
        let Some(signal) = self.graph.get(ww.node).map(|n| n.signal.clone()) else {
            return;
        };
        match wf_tools::short_freq_text(&signal, &markers) {
            Some(text) => {
                if let Some(w) = self.wf.wins.get_mut(&win) {
                    w.freq = Some(text);
                }
            }
            None => self.alert(&t!("sw-check-settings")),
        }
    }
}
