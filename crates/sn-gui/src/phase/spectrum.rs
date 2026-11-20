//! The spectrum mode's upper half (M-p14, M-p15): the toolbar `Fmin`, `Fmax`,
//! `Scale`, `FFTSize`, `LowLevel` and `Stretch` in SA's sunken groups, the
//! high-resolution averaged spectrum in cyan on black with the Fl and Fu
//! markers (a click or drag moves the nearer one), and the navigation scroll
//! bar under it.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, StatefulInteractiveElement, Styled, Window, canvas,
};
use sn_core::NodeId;
use sn_ui_core::phase::{FFT_SIZES, SCALES};
use sn_ui_core::phase_plot::{self, FORM};
use sn_ui_core::{format, t};

use super::{PhaseWin, at, scroll_value};
use crate::app::Frame;
use crate::win95::bars::tooltip;
use crate::win95::button::{Changed, Ctl, check_box};
use crate::win95::colors::{Role, hsla, palette, rgb_of, sa_colors, scheme};
use crate::win95::combo::combo_box;
use crate::win95::geom::{Edge, Rect};
use crate::win95::kit::{Capture, action, lens};
use crate::win95::label::{Align, label};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, scroll_bar};
use crate::win95::text;
use crate::win95::trackbar::{TrackState, trackbar};
use crate::win95::widget::{Slot, place, size_of};

/// The spectrum plot.
pub(crate) const PLOT: Rect = at(8, 30, 920, 170);
/// The navigation scroll bar under it.
pub(crate) const NAV: Rect = at(0, 204, FORM.0, 16);
/// Height of a toolbar group.
const GROUP_H: i32 = 26;
/// Height of a control in a group.
const CTL_H: i32 = 21;

/// The controls of the toolbar, in SA's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tool {
    Fmin,
    Fmax,
    Scale,
    Fft,
    LowLevel,
    Stretch,
}

impl Tool {
    pub(crate) const ALL: [Self; 6] = [
        Self::Fmin,
        Self::Fmax,
        Self::Scale,
        Self::Fft,
        Self::LowLevel,
        Self::Stretch,
    ];

    fn caption(self) -> String {
        match self {
            Self::Fmin => t!("pp-fmin"),
            Self::Fmax => t!("pp-fmax"),
            Self::Scale => t!("pp-scale"),
            Self::Fft => t!("pp-fft-size"),
            Self::LowLevel => t!("pp-low-level"),
            Self::Stretch => t!("pp-stretch"),
        }
    }

    fn hint(self) -> String {
        match self {
            Self::Fmin => t!("pp-fmin.hover"),
            Self::Fmax => t!("pp-fmax.hover"),
            Self::Scale => t!("pp-scale.hover"),
            Self::Fft => t!("pp-fft-size.hover"),
            Self::LowLevel => t!("pp-low-level.hover"),
            Self::Stretch => t!("pp-stretch.hover"),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Fmin => "fmin",
            Self::Fmax => "fmax",
            Self::Scale => "scale",
            Self::Fft => "fft",
            Self::LowLevel => "low",
            Self::Stretch => "stretch",
        }
    }

    /// Width of the control.
    fn width(self) -> i32 {
        match self {
            Self::Fmin | Self::Fmax => 60,
            Self::Scale => 56,
            Self::Fft => 74,
            Self::LowLevel => 110,
            Self::Stretch => 14,
        }
    }
}

/// A read-only grey field showing `text` (Fmin, Fmax).
fn readonly_field(cx: &gpui::App, r: Rect, value: String) -> AnyElement {
    let s = scheme(cx);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let o = b.origin;
            paint::fill(window, o, Rect::new(0, 0, w, h), s.of(Role::Face));
            paint::edge(window, o, Rect::new(0, 0, w, h), Edge::Sunken, &s);
            let at = paint::at(o, 4, (h - text::LINE) / 2);
            text::draw(window, cx, at, &value, false, s.of(Role::Text));
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

impl Frame {
    /// The spectrum mode's upper half of window `win`.
    pub(crate) fn spectrum_area(
        &mut self,
        win: u32,
        node: NodeId,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(sig) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return Vec::new();
        };
        let Some(st) = self.phase.states.get(&node) else {
            return Vec::new();
        };
        let f = &st.form;
        let shown =
            phase_plot::spectrum_band(sig.kind(), sig.sample_rate(), st.band, f.zoom, f.spec_pos);
        let rf = sig.center_frequency();
        let mut out = self.spectrum_toolbar(win, shown, rf, window, cx);
        out.push(self.spectrum_plot(win, node, shown, cx));
        let Some(nav) = self.phase.wins.get(&win).map(|w| w.spec_nav) else {
            return out;
        };
        let id = format!("pp{win}.specnav");
        out.push(scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: &id,
                rect: NAV,
                vertical: false,
            },
            nav,
            lens(move |v: &mut Self| v.phase_win(win).map(|w| &mut w.spec_nav)),
            Some(action(move |v: &mut Self, _, _| {
                let pos = v.phase_win(win).map(|w| scroll_value(&w.spec_nav));
                if let (Some(p), Some(st)) = (pos, v.phase_st(win)) {
                    st.form.spec_pos = p;
                }
            })),
        ));
        out
    }

    /// The toolbar of the spectrum mode.
    fn spectrum_toolbar(
        &mut self,
        win: u32,
        shown: (f64, f64),
        rf: f64,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let s = scheme(cx);
        let mut out = Vec::new();
        let mut x = 2;
        for tool in Tool::ALL {
            let caption = tool.caption();
            let cap_w = text::width(window, &caption, true);
            let gw = 4 + cap_w + 4 + tool.width() + 4;
            let g = Rect::new(x, 2 + (24 - GROUP_H) / 2, gw, GROUP_H);
            let panel = canvas(
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
            let cap = label(
                cx,
                Rect::new(4, (GROUP_H - CTL_H) / 2, cap_w, CTL_H),
                &caption,
                true,
                true,
                Align::Left,
            );
            let r = Rect::new(4 + cap_w + 4, (GROUP_H - CTL_H) / 2, tool.width(), CTL_H);
            let control = self.spectrum_control(win, tool, r, shown, rf, cx);
            let el = place(g)
                .id(gpui::ElementId::Name(
                    format!("pp{win}.g.{}", tool.name()).into(),
                ))
                .tooltip(tooltip(&tool.hint()))
                .child(panel)
                .child(cap)
                .child(control);
            out.push(el.into_any_element());
            x += gw + 3;
        }
        out
    }

    /// The control of toolbar group `tool` at `r` (relative to its group).
    fn spectrum_control(
        &mut self,
        win: u32,
        tool: Tool,
        r: Rect,
        shown: (f64, f64),
        rf: f64,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let id = format!("pp{win}.{}", tool.name());
        let scope = format!("mdi{win}");
        let ctl = Ctl {
            id: &id,
            scope: &scope,
            rect: r,
            label: "",
            enabled: true,
        };
        let Some(pw) = self.phase.wins.get(&win) else {
            return gpui::Empty.into_any_element();
        };
        let stretch = self
            .phase
            .states
            .get(&pw.node)
            .is_some_and(|s| s.form.stretch);
        match tool {
            Tool::Fmin => readonly_field(cx, r, format::dec_trim(shown.0 + rf, 0)),
            Tool::Fmax => readonly_field(cx, r, format::dec_trim(shown.1 + rf, 0)),
            Tool::Scale | Tool::Fft => {
                let scale = tool == Tool::Scale;
                let st = if scale {
                    pw.scale.clone()
                } else {
                    pw.fft.clone()
                };
                let l = lens(move |v: &mut Self| {
                    let w = v.phase_win(win)?;
                    Some(if scale { &mut w.scale } else { &mut w.fft })
                });
                let on: Changed<Self, usize> = Rc::new(move |v: &mut Self, i, _, _| {
                    if let Some(st) = v.phase_st(win) {
                        if scale {
                            st.form.zoom = SCALES.get(i).copied().unwrap_or(st.form.zoom);
                        } else {
                            st.form.fft = FFT_SIZES.get(i).copied().unwrap_or(st.form.fft);
                        }
                    }
                });
                combo_box(&mut self.kit, cx, ctl, &st, l, Some(on))
            }
            Tool::LowLevel => {
                let st = pw.low_level;
                trackbar(
                    &mut self.kit,
                    cx,
                    ctl,
                    st,
                    lens(move |v: &mut Self| v.phase_win(win).map(|w| &mut w.low_level)),
                    Some(track_to_form(
                        win,
                        |w| &w.low_level,
                        |f, p| {
                            f.low_level = p as f32;
                        },
                    )),
                )
            }
            Tool::Stretch => check_box(
                &mut self.kit,
                cx,
                ctl,
                stretch,
                Self::form_flag(win, |f| &mut f.stretch),
            ),
        }
    }

    /// The averaged spectrum with its markers.
    fn spectrum_plot(
        &mut self,
        win: u32,
        node: NodeId,
        shown: (f64, f64),
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let pal = palette(cx);
        let (bg, trace, line) = (
            hsla(rgb_of(pal.plot_bg)),
            hsla(rgb_of(pal.trace_i)),
            hsla(rgb_of(pal.marker_free)),
        );
        let green = hsla(sa_colors(cx).green);
        let st = self.phase.states.get(&node);
        let levels = st
            .and_then(|st| {
                let f = &st.form;
                st.hires.as_ref().map(|h| {
                    phase_plot::hires_levels(h, shown, PLOT.w as usize, f.low_level, f.stretch)
                })
            })
            .unwrap_or_default();
        let marks = st.map(|s| s.spec_marks).unwrap_or_default();
        let span = (shown.1 - shown.0).max(1e-9);
        let x_of = move |f: f64| ((f - shown.0) / span * f64::from(PLOT.w)) as i32;
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let (w, h) = size_of(b);
                let o = b.origin;
                paint::fill(window, o, Rect::new(0, 0, w, h), bg);
                for (c, l) in (0_i32..).zip(&levels) {
                    let len = (l * h as f32).round() as i32;
                    if len > 0 {
                        paint::fill(window, o, Rect::new(c, h - len, 1, len), trace);
                    }
                }
                for f in [marks.fl, marks.fu].into_iter().flatten() {
                    let x = x_of(f);
                    paint::fill(window, o, Rect::new(x, 0, 1, h), line);
                    paint::fill(window, o, Rect::new(x - 2, 0, 4, 8), green);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let slot = Slot::default();
        let s2 = slot.clone();
        place(PLOT)
            .id(gpui::ElementId::Name(format!("pp{win}.spec").into()))
            .child(paint)
            .child(
                canvas(move |b, _, _| s2.set(b), |_, (), _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.kit.focus = None;
                    if let Some((x, _)) = slot.local(ev.position) {
                        spec_mark_at(v, win, shown, x);
                        let s3 = slot.clone();
                        v.kit.capture(Capture {
                            on_move: Some(Rc::new(move |v: &mut Self, p, _, _| {
                                if let Some((x, _)) = s3.local(p) {
                                    spec_mark_at(v, win, shown, x);
                                }
                            })),
                            on_up: None,
                        });
                    }
                    cx.notify();
                }),
            )
            .into_any_element()
    }
}

/// Move the nearer frequency marker of window `win` to plot column `x`.
fn spec_mark_at(v: &mut Frame, win: u32, shown: (f64, f64), x: i32) {
    let span = (shown.1 - shown.0).max(1e-9);
    let w = f64::from(PLOT.w);
    let x = f64::from(x.clamp(0, PLOT.w));
    let f = shown.0 + x / w * span;
    let x_of = move |f: f64| ((f - shown.0) / span * w) as f32;
    if let Some(st) = v.phase_st(win) {
        phase_plot::move_spec_mark(&mut st.spec_marks, f, x_of, x as f32);
    }
}

/// A trackbar callback: the position of `get(win)` goes to the form with `set`.
pub(crate) fn track_to_form(
    win: u32,
    get: fn(&PhaseWin) -> &TrackState,
    set: fn(&mut sn_ui_core::phase::Form, i32),
) -> crate::win95::kit::Action<Frame> {
    action(move |v: &mut Frame, _, _| {
        let pos = v.phase.wins.get(&win).map(|w| get(w).pos);
        if let (Some(p), Some(st)) = (pos, v.phase_st(win)) {
            set(&mut st.form, p);
        }
    })
}
