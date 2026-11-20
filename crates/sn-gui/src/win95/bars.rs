//! Status bars, flat toolbar buttons and tooltips.

use gpui::{
    AnyElement, App, AppContext, Context, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div,
    px,
};

use super::colors::{Role, scheme};
use super::geom::{Edge, Rect};
use super::glyph::{self, Glyph, Mode};
use super::kit::{Action, Host, Kit};
use super::text::{self, LINE};
use super::widget::{place, size_of};
use super::{marks, paint};

/// Height of a status bar.
pub(crate) const STATUS_H: i32 = 20;
/// Gap between status bar panels.
pub(crate) const PANEL_GAP: i32 = 2;
/// Size of a flat toolbar button.
pub(crate) const TOOL: i32 = 22;

/// The panels of a status bar `w` pixels wide: each width in `widths` is fixed,
/// and a width of 0 takes the rest (only the last such panel gets it; other
/// panels of width 0 stay empty).
pub(crate) fn status_layout(w: i32, h: i32, widths: &[i32]) -> Vec<Rect> {
    let fixed: i32 = widths.iter().sum::<i32>() + PANEL_GAP * (widths.len() as i32 - 1).max(0);
    let rest = (w - fixed).max(0);
    let last_fill = widths.iter().rposition(|&pw| pw == 0);
    let mut x = 0;
    widths
        .iter()
        .enumerate()
        .map(|(i, &pw)| {
            let pw = match (pw, Some(i) == last_fill) {
                (0, true) => rest,
                _ => pw,
            };
            let r = Rect::new(x, 2, pw, h - 2);
            x += pw + PANEL_GAP;
            r
        })
        .collect()
}

/// A status bar at `r` with sunken panels (text, width; width 0 fills) and a
/// size grip at the right.
pub(crate) fn status_bar(cx: &App, r: Rect, panels: &[(String, i32)], grip: bool) -> AnyElement {
    let s = scheme(cx);
    let texts: Vec<String> = panels.iter().map(|p| p.0.clone()).collect();
    let widths: Vec<i32> = panels.iter().map(|p| p.1).collect();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let o = b.origin;
            paint::fill(window, o, Rect::new(0, 0, w, h), s.of(Role::Face));
            let themed = super::skin::skin() != super::skin::Skin::Classic;
            let layout = status_layout(w, h, &widths);
            let last = layout.len().saturating_sub(1);
            for (i, (pr, t)) in layout.into_iter().zip(&texts).enumerate() {
                if !themed {
                    paint::edge(window, o, pr, Edge::ThinSunken, &s);
                } else if i < last {
                    // Themed status bars separate their panels with a line.
                    let x = pr.right() + 1;
                    paint::fill(
                        window,
                        o,
                        Rect::new(x, pr.y + 2, 1, pr.h - 4),
                        s.of(Role::Shadow),
                    );
                    paint::fill(
                        window,
                        o,
                        Rect::new(x + 1, pr.y + 2, 1, pr.h - 4),
                        s.of(Role::Highlight),
                    );
                }
                let clip = paint::bounds(o, pr.inset(1));
                window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
                    let at = paint::at(o, pr.x + 3, pr.y + (pr.h - LINE) / 2);
                    text::draw(window, cx, at, t, false, s.of(Role::Text));
                });
            }
            if grip {
                let g = marks::size_grip();
                glyph::draw(
                    window,
                    cx,
                    o,
                    (w - g.w - 1, h - g.h - 1),
                    &g,
                    Mode::Normal,
                    &s,
                );
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// A tooltip in Windows 95 style: pale yellow, black border, black text.
#[derive(Debug)]
pub(crate) struct Tooltip {
    text: SharedString,
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let s = scheme(cx);
        div()
            .font_family(text::font(false).family)
            .text_size(px(text::content_size()))
            .line_height(px(LINE as f32))
            .bg(s.of(Role::Info))
            .text_color(s.of(Role::InfoText))
            .border_1()
            .border_color(s.of(Role::DarkShadow))
            .px(px(2.0))
            .py(px(1.0))
            .child(self.text.clone())
    }
}

/// Build a tooltip view (GPUI shows it after 500 ms, Windows 95's delay).
pub(crate) fn tooltip(text: &str) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static {
    let text: SharedString = text.to_owned().into();
    move |_, cx| cx.new(|_| Tooltip { text: text.clone() }).into()
}

/// Props of a flat toolbar button.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tool<'a> {
    pub(crate) id: &'a str,
    /// Top-left corner; the button is [`TOOL`] pixels square.
    pub(crate) at: (i32, i32),
    pub(crate) glyph: &'a Glyph,
    pub(crate) enabled: bool,
    /// Latched down (a toggle that is on).
    pub(crate) checked: bool,
    /// Tooltip text (empty for none).
    pub(crate) tip: &'a str,
}

/// A flat toolbar button: no border until the mouse is over it, then a thin
/// raised one; sunken while held or checked.
pub(crate) fn tool_button<V: Host>(
    kit: &Kit<V>,
    cx: &mut Context<'_, V>,
    t: Tool<'_>,
    on_click: Action<V>,
) -> AnyElement {
    let s = scheme(cx);
    let held = kit.is_pressed(t.id);
    let (g, enabled, checked) = (t.glyph.clone(), t.enabled, t.checked);
    let paint = canvas(
        // A hitbox, so the hover respects what lies on top (a modal box, an
        // open menu): Windows shows no hover there.
        |b, window, _| window.insert_hitbox(b, gpui::HitboxBehavior::Normal),
        move |b, hitbox, window, cx| {
            let (w, h) = size_of(b);
            let o = b.origin;
            let over = hitbox.is_hovered(window) && enabled;
            let down = (held && over) || checked;
            let r = Rect::new(0, 0, w, h);
            let themed = super::skin::skin() != super::skin::Skin::Classic;
            if themed && (down || over) {
                super::skin::tool_button(window, o, r, down);
            } else if themed {
            } else if checked && !(held && over) {
                paint::dither(
                    window,
                    o,
                    r.inset(1),
                    s.of(Role::Face),
                    s.of(Role::Highlight),
                );
            }
            if themed {
            } else if down {
                paint::edge(window, o, r, Edge::ThinSunken, &s);
            } else if over {
                paint::edge(window, o, r, Edge::ThinRaised, &s);
            }
            // Themed toolbar glyphs do not move when pressed.
            let d = i32::from(down && !themed);
            let mode = if enabled {
                Mode::Normal
            } else {
                Mode::Disabled
            };
            glyph::draw(
                window,
                cx,
                o,
                ((w - g.w) / 2 + d, (h - g.h) / 2 + d),
                &g,
                mode,
                &s,
            );
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let mut el = place(Rect::new(t.at.0, t.at.1, TOOL, TOOL))
        .id(gpui::ElementId::Name(t.id.to_owned().into()))
        .child(paint)
        .on_hover(cx.listener(|_, _: &bool, _, cx| cx.notify()));
    if !t.tip.is_empty() {
        el = el.tooltip(tooltip(t.tip));
    }
    if !t.enabled {
        return el.into_any_element();
    }
    let (id1, id2) = (t.id.to_owned(), t.id.to_owned());
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v: &mut V, _, _, cx| {
            v.kit().press(&id1, None);
            cx.notify();
        }),
    )
    .on_mouse_up(
        MouseButton::Left,
        cx.listener(move |v: &mut V, _, window, cx| {
            if v.kit().is_pressed(&id2) {
                on_click(v, window, cx);
                cx.notify();
            }
        }),
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_panels_fill_the_rest() {
        let p = status_layout(500, 20, &[150, 0]);
        assert_eq!(
            p,
            vec![Rect::new(0, 2, 150, 18), Rect::new(152, 2, 348, 18)]
        );
        let p = status_layout(100, 20, &[150, 0]);
        assert_eq!(p[1].w, 0, "never negative");
        assert_eq!(status_layout(100, 20, &[]), Vec::<Rect>::new());
        // Only the last fill panel takes the rest: the bar never overflows.
        let p = status_layout(500, 20, &[0, 100, 0]);
        assert_eq!(p[0].w, 0);
        assert_eq!(p[2].w, 500 - 100 - 2 * PANEL_GAP);
        assert_eq!(p[2].right(), 500);
    }
}
