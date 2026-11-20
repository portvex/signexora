//! The phase viewer in the lower half (`SA_UI_SPEC.md` 6.2, the 6.2.6.4
//! layout of R278/a2) and SA's status bar: the constellation square with its
//! scale scroll bar; the etched panel with `Carrier`, `BR`, `n-Ary`,
//! `Synch BR`, `NonStop`, `RRC filter` and alpha, group `PLL`, `AutoReset`,
//! `Transitions`, `Life` and `Diff`; the quality number and `Offset mode
//! detected`; `Angles`, `Vectors of dem`, `Output Dem as`; the sliders
//! `Trimming`, `Delay`, `Points...`, `Contrast` and the progress bar; `Start`
//! (`Break` while running), `Clear` and `LoadFile`.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, Corners, InteractiveElement, IntoElement, MouseButton, MouseUpEvent,
    ParentElement, StatefulInteractiveElement, Styled, Window, canvas,
};
use sn_ui_core::phase::{DemMode, Form, Mode, PhaseState, PllSpeed};
use sn_ui_core::phase_plot::{self, FORM, PlayProgress};
use sn_ui_core::{format, t};

use super::spectrum::track_to_form;
use super::{PhaseWin, at, scroll_value, tip};
use crate::app::Frame;
use crate::app::dialogs::colored_label;
use crate::app::menus::cmd;
use crate::win95::bars::tooltip;
use crate::win95::button::{Changed, Ctl, check_box, push_button, radio_button};
use crate::win95::colors::{Role, hsla, palette, rgb_of, scheme};
use crate::win95::combo::{ComboState, combo_box};
use crate::win95::edit::edit;
use crate::win95::geom::{Edge, Rect};
use crate::win95::kit::{Lens, action, lens};
use crate::win95::label::{Align, group_box, label};
use crate::win95::menu::{Check, Item};
use crate::win95::paint;
use crate::win95::progress::{Progress, progress_field};
use crate::win95::scrollbar::{Bar, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::trackbar::{TrackState, trackbar};
use crate::win95::widget::{place, size_of};

/// The constellation square.
pub(crate) const CONSTELLATION: Rect = at(8, 230, 176, 176);
/// `Start` (`Break` while running), `Clear` and `LoadFile`.
pub(crate) const START: Rect = at(858, 290, 70, 22);
pub(crate) const CLEAR: Rect = at(858, 318, 70, 22);
pub(crate) const LOAD_FILE: Rect = at(858, 382, 70, 22);
/// The check boxes of the lower half.
pub(crate) const NONSTOP: Rect = at(220, 334, 130, 18);
pub(crate) const LIFE: Rect = at(366, 358, 100, 18);
pub(crate) const ANGLES: Rect = at(482, 248, 76, 18);
pub(crate) const OUTPUT_DEM: Rect = at(482, 270, 120, 18);
/// The first slider (`Trimming`); the others follow 20 px lower.
pub(crate) const TRIMMING: Rect = at(482, 294, 184, 18);
/// SA's blue `Offset mode detected` (grey until detected).
const OFFSET_BLUE: u32 = 0x00_00_FF;

/// A check box of the lower half: name, place, caption, hint, enabled and
/// its flag in the form.
type FlagRow = (
    &'static str,
    Rect,
    String,
    String,
    bool,
    fn(&mut Form) -> &mut bool,
);

/// The four sliders: name, the slider's state in the window, and where its
/// position goes in the form.
type Slider = (
    &'static str,
    fn(&PhaseWin) -> &TrackState,
    fn(&mut Form, i32),
);

const SLIDERS: [Slider; 4] = [
    ("trim", |w| &w.trimming, |f, p| f.trimming = p),
    (
        "delay",
        |w| &w.delay,
        |f, p| f.delay = f64::from(p) / 1000.0,
    ),
    (
        "points",
        |w| &w.points,
        |f, p| {
            f.points = usize::try_from(p).unwrap_or(10);
        },
    ),
    (
        "contrast",
        |w| &w.contrast,
        |f, p| {
            f.contrast = u32::try_from(p).unwrap_or(1);
        },
    ),
];

/// The lens of slider `k` of a window.
fn slider_lens(win: u32, k: usize) -> Lens<Frame, TrackState> {
    lens(move |v: &mut Frame| {
        let w = v.phase_win(win)?;
        Some(match k {
            0 => &mut w.trimming,
            1 => &mut w.delay,
            2 => &mut w.points,
            _ => &mut w.contrast,
        })
    })
}

impl Frame {
    /// The lower half and the status bar of window `win`.
    pub(crate) fn viewer_area(
        &mut self,
        win: u32,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(node) = self.phase.wins.get(&win).map(|w| w.node) else {
            return Vec::new();
        };
        let Some(st) = self.phase.states.get(&node) else {
            return Vec::new();
        };
        let running = st.play.running;
        let mut out = self.constellation(win, cx);
        out.push(group_box(cx, at(212, 219, 262, 191), ""));
        out.extend(self.fields(win, running, cx));
        out.extend(self.column_checks(win, running, cx));
        out.extend(self.pll_group(win, cx));
        out.extend(self.right_part(win, cx));
        out.extend(self.sliders(win, cx));
        out.extend(self.viewer_buttons(win, running, cx));
        out.push(self.phase_status_bar(win, node, cx));
        let mut note_y = 8;
        if let Some(e) = self.phase.states.get(&node).and_then(|s| s.error.clone()) {
            let w = text::width(window, &e, false);
            let err = hsla(rgb_of(palette(cx).error));
            out.push(colored_label(at(12, note_y, w + 4, LINE), &e, err, false));
            note_y += LINE;
        }
        // A limited run is a note, not an error: drawn in the normal text colour.
        if let Some(n) = self.phase.states.get(&node).and_then(PhaseState::notice) {
            let w = text::width(window, &n, false);
            let color = scheme(cx).of(Role::Text);
            out.push(colored_label(at(12, note_y, w + 4, LINE), &n, color, false));
        }
        out
    }

    /// The constellation square (or the eye diagram) with its scale scroll
    /// bar. A right click offers the eye diagram (SigNexora extra); the hint shows
    /// the rings and offsets of the run.
    fn constellation(&mut self, win: u32, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let img = self.constellation_image(win, CONSTELLATION.w as usize, cx);
        let s = scheme(cx);
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let (w, h) = size_of(b);
                if let Some(img) = &img {
                    window
                        .paint_image(b, Corners::default(), img.clone(), 0, false)
                        .ok();
                }
                paint::edge(window, b.origin, Rect::new(0, 0, w, h), Edge::Sunken, &s);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let details = self
            .phase_st(win)
            .and_then(|st| st.shown().map(|s| s.details.clone()));
        let mut el = place(CONSTELLATION)
            .id(gpui::ElementId::Name(format!("pp{win}.const").into()))
            .child(paint)
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseUpEvent, _, cx| {
                    let at = (
                        f32::from(ev.position.x).round() as i32,
                        f32::from(ev.position.y).round() as i32,
                    );
                    let eye = v.phase_st(win).is_some_and(|st| st.eye);
                    let item = Item::command(&t!("pp-eye"), cmd::PP_EYE).check(if eye {
                        Check::Tick
                    } else {
                        Check::None
                    });
                    v.open_popup(vec![item], at);
                    v.phase.popup_win = Some(win);
                    cx.notify();
                }),
            );
        if let Some(d) = details {
            el = el.tooltip(tooltip(&d));
        }
        let mut out = vec![el.into_any_element()];
        if let Some(vs) = self.phase.wins.get(&win).map(|w| w.const_vs) {
            let id = format!("pp{win}.constvs");
            out.push(scroll_bar(
                &self.kit,
                cx,
                Bar {
                    id: &id,
                    rect: at(188, 228, 16, 180),
                    vertical: true,
                },
                vs,
                lens(move |v: &mut Self| v.phase_win(win).map(|w| &mut w.const_vs)),
                Some(action(move |v: &mut Self, _, _| {
                    let p = v.phase_win(win).map(|w| scroll_value(&w.const_vs));
                    if let (Some(p), Some(st)) = (p, v.phase_st(win)) {
                        st.form.scale = p;
                    }
                })),
            ));
        }
        out
    }

    /// Carrier, BR and n-Ary.
    fn fields(&mut self, win: u32, running: bool, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(pw) = self.phase.wins.get(&win) else {
            return Vec::new();
        };
        let (carrier, br, nary) = (pw.carrier.clone(), pw.br.clone(), pw.nary.clone());
        let locked = running && self.phase_st(win).is_some_and(|st| st.form.output_dem);
        let scope = format!("mdi{win}");
        let mut out = Vec::new();
        for (k, st) in [carrier, br].into_iter().enumerate() {
            let carrier = k == 0;
            let r = at(220, 236 + 26 * k as i32, 84, 21);
            let (name, cap, hint) = if carrier {
                ("carrier", t!("pp-carrier"), t!("pp-carrier.hover"))
            } else {
                ("br", t!("pp-br"), t!("pp-br.hover"))
            };
            let id = format!("pp{win}.{name}");
            out.push(tip(format!("pp{win}.t.{name}"), r, &hint));
            out.push(edit(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: r,
                    label: "",
                    enabled: true,
                },
                &st,
                lens(move |v: &mut Self| {
                    let w = v.phase_win(win)?;
                    Some(if carrier { &mut w.carrier } else { &mut w.br })
                }),
                Some(action(move |v: &mut Self, _, _| {
                    v.phase_field_typed(win, carrier);
                })),
            ));
            out.push(label(
                cx,
                at(310, r.y, 54, 21),
                &cap,
                false,
                true,
                Align::Left,
            ));
        }
        out.push(tip(
            format!("pp{win}.t.nary"),
            at(220, 288, 74, 21),
            &t!("pp-nary.hover"),
        ));
        out.push(self.list(win, "nary", at(220, 288, 74, 21), !locked, &nary, cx));
        out.push(label(
            cx,
            at(300, 288, 60, 21),
            &t!("pp-nary"),
            false,
            true,
            Align::Left,
        ));
        out
    }

    /// A combo box of the lower half (`nary`, `diff`, `dem`).
    fn list(
        &mut self,
        win: u32,
        name: &'static str,
        r: Rect,
        enabled: bool,
        st: &ComboState,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let id = format!("pp{win}.{name}");
        let scope = format!("mdi{win}");
        let l = lens(move |v: &mut Self| {
            let w = v.phase_win(win)?;
            Some(match name {
                "nary" => &mut w.nary,
                "diff" => &mut w.diff,
                _ => &mut w.dem,
            })
        });
        let on: Changed<Self, usize> = Rc::new(move |v: &mut Self, i, _, _| {
            if let Some(st) = v.phase_st(win) {
                let f = &mut st.form;
                match name {
                    "nary" => f.order = i as u32 + 1,
                    "diff" => f.diff = u8::try_from(i).unwrap_or(0).min(3),
                    _ => f.dem = DemMode::all().get(i).copied().unwrap_or(f.dem),
                }
            }
        });
        combo_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: r,
                label: "",
                enabled,
            },
            st,
            l,
            Some(on),
        )
    }

    /// A typed Carrier or BR goes to the form.
    fn phase_field_typed(&mut self, win: u32, carrier: bool) {
        let Some(text) = self.phase_win(win).map(|w| {
            if carrier {
                w.carrier.text.clone()
            } else {
                w.br.text.clone()
            }
        }) else {
            return;
        };
        if let Some(st) = self.phase_st(win) {
            if carrier {
                st.form.carrier = text;
            } else {
                st.form.br = text;
            }
        }
    }

    /// A check box of the lower half.
    #[allow(clippy::too_many_arguments)]
    fn flag(
        &mut self,
        win: u32,
        name: &str,
        r: Rect,
        caption: &str,
        hint: &str,
        enabled: bool,
        get: fn(&mut Form) -> &mut bool,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let on = self.phase_st(win).is_some_and(|st| *get(&mut st.form));
        let id = format!("pp{win}.{name}");
        let scope = format!("mdi{win}");
        vec![
            tip(format!("pp{win}.t.{name}"), r, hint),
            check_box(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: r,
                    label: caption,
                    enabled,
                },
                on,
                Self::form_flag(win, get),
            ),
        ]
    }

    /// `Synch BR`, `NonStop`, `RRC filter` with alpha, `AutoReset`,
    /// `Transitions`, `Life` and `Diff`.
    fn column_checks(
        &mut self,
        win: u32,
        running: bool,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let (locked, rrc, alpha) = self.phase_st(win).map_or((false, false, 0.5), |st| {
            (running && st.form.output_dem, st.form.rrc, st.form.alpha)
        });
        let mut out = Vec::new();
        let flags: [FlagRow; 6] = [
            (
                "synch",
                at(220, 314, 130, 18),
                t!("pp-synch-br"),
                t!("pp-synch-br.hover"),
                !locked,
                |f| &mut f.synch,
            ),
            (
                "nonstop",
                NONSTOP,
                t!("pp-nonstop"),
                t!("pp-nonstop.hover"),
                true,
                |f| &mut f.nonstop,
            ),
            (
                "rrc",
                at(220, 354, 130, 18),
                t!("pp-rrc"),
                t!("pp-rrc.hover"),
                true,
                |f| &mut f.rrc,
            ),
            (
                "autoreset",
                at(366, 322, 100, 18),
                t!("pp-auto-reset"),
                t!("pp-auto-reset.hover"),
                !running,
                |f| &mut f.auto_reset,
            ),
            (
                "transitions",
                at(366, 340, 100, 18),
                t!("pp-transitions"),
                t!("pp-transitions.hover"),
                true,
                |f| &mut f.transitions,
            ),
            (
                "life",
                LIFE,
                t!("pp-life"),
                t!("pp-life.hover"),
                true,
                |f| &mut f.life,
            ),
        ];
        for (name, r, cap, hint, enabled, get) in flags {
            out.extend(self.flag(win, name, r, &cap, &hint, enabled, get, cx));
        }
        out.extend(self.alpha(win, rrc, alpha, cx));
        if let Some(diff) = self.phase.wins.get(&win).map(|w| w.diff.clone()) {
            let r = at(366, 380, 56, 21);
            out.push(tip(format!("pp{win}.t.diff"), r, &t!("pp-diff.hover")));
            out.push(self.list(win, "diff", r, !locked, &diff, cx));
        }
        out.push(label(
            cx,
            at(426, 380, 40, 21),
            &t!("pp-diff"),
            false,
            true,
            Align::Left,
        ));
        out
    }

    /// The alpha slider of the RRC filter and its value.
    fn alpha(
        &mut self,
        win: u32,
        rrc: bool,
        alpha: f64,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::new();
        if let Some(st) = self.phase.wins.get(&win).map(|w| w.alpha) {
            let id = format!("pp{win}.alpha");
            let scope = format!("mdi{win}");
            let r = at(220, 382, 110, 18);
            out.push(tip(format!("pp{win}.t.alpha"), r, &t!("pp-alpha.hover")));
            out.push(trackbar(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: r,
                    label: "",
                    enabled: rrc,
                },
                st,
                lens(move |v: &mut Self| v.phase_win(win).map(|w| &mut w.alpha)),
                Some(track_to_form(
                    win,
                    |w| &w.alpha,
                    |f, p| {
                        f.alpha = f64::from(p) / 20.0;
                    },
                )),
            ));
        }
        out.push(label(
            cx,
            at(336, 382, 30, 18),
            &format::dec_trim(alpha, 2),
            false,
            rrc,
            Align::Left,
        ));
        out
    }

    /// The PLL group: Slow, Medium, Quick, Hard (SA 6.2.6).
    fn pll_group(&mut self, win: u32, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let pll = self.phase_st(win).map(|st| st.form.pll);
        let scope = format!("mdi{win}");
        let mut out = vec![
            tip(
                format!("pp{win}.t.pll"),
                at(366, 228, 100, 14),
                &t!("pp-pll.hover"),
            ),
            group_box(cx, at(366, 228, 100, 88), &t!("pp-pll")),
        ];
        let speeds = [
            (PllSpeed::Slow, t!("pp-pll-slow")),
            (PllSpeed::Medium, t!("pp-pll-medium")),
            (PllSpeed::Quick, t!("pp-pll-quick")),
            (PllSpeed::Hard, t!("pp-pll-hard")),
        ];
        for (k, (speed, caption)) in speeds.into_iter().enumerate() {
            let id = format!("pp{win}.pll{k}");
            out.push(radio_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: at(374, 242 + 17 * k as i32, 88, 16),
                    label: &caption,
                    enabled: true,
                },
                pll == Some(speed),
                Self::on_st(win, move |st| st.form.pll = speed),
            ));
        }
        out
    }

    /// The quality number, `Offset mode detected`, Angles, Vectors of dem and
    /// Output Dem as with its list.
    fn right_part(&mut self, win: u32, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some((q, offset)) = self.phase_st(win).map(|st| phase_plot::quality_text(st)) else {
            return Vec::new();
        };
        let s = scheme(cx);
        let offset_color = if offset {
            hsla(OFFSET_BLUE)
        } else {
            s.of(Role::Shadow)
        };
        let qr = at(482, 230, 72, LINE);
        let mut out = vec![
            tip(format!("pp{win}.t.q"), qr, &t!("pp-quality.hover")),
            colored_label(qr, &q, s.of(Role::Text), false),
            colored_label(
                at(560, 230, 160, LINE),
                &t!("pp-offset-detected"),
                offset_color,
                false,
            ),
        ];
        out.extend(self.flag(
            win,
            "angles",
            ANGLES,
            &t!("pp-angles"),
            &t!("pp-angles.hover"),
            true,
            |f| &mut f.angles,
            cx,
        ));
        out.extend(self.flag(
            win,
            "vectors",
            at(560, 248, 130, 18),
            &t!("pp-vectors"),
            &t!("pp-vectors.hover"),
            true,
            |f| &mut f.vectors,
            cx,
        ));
        let on = self.phase_st(win).is_some_and(|st| st.form.output_dem);
        let id = format!("pp{win}.outdem");
        let scope = format!("mdi{win}");
        out.push(tip(
            format!("pp{win}.t.outdem"),
            OUTPUT_DEM,
            &t!("pp-output-dem.hover"),
        ));
        out.push(check_box(
            &mut self.kit,
            cx,
            Ctl {
                id: &id,
                scope: &scope,
                rect: OUTPUT_DEM,
                label: &t!("pp-output-dem"),
                enabled: true,
            },
            on,
            Rc::new(move |v: &mut Self, on, _, _| {
                if let Some(st) = v.phase_st(win) {
                    st.form.output_dem = on;
                    st.play.dem_from = st.play.pos;
                }
            }),
        ));
        if let Some(dem) = self.phase.wins.get(&win).map(|w| w.dem.clone()) {
            out.push(self.list(win, "dem", at(604, 268, 66, 21), true, &dem, cx));
        }
        out
    }

    /// Trimming, Delay, Points... and Contrast with their values, and the
    /// progress bar.
    fn sliders(&mut self, win: u32, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(st) = self.phase_st(win) else {
            return Vec::new();
        };
        let f = st.form.clone();
        let progress = match phase_plot::play_progress(st) {
            PlayProgress::Idle => Progress::Idle,
            PlayProgress::Busy => Progress::Busy(self.tick),
            PlayProgress::Done(p) => Progress::Done(p),
        };
        let scope = format!("mdi{win}");
        let hints = [
            t!("pp-trimming.hover"),
            t!("pp-delay.hover"),
            t!("pp-points.hover"),
            t!("pp-contrast.hover"),
        ];
        let texts = [
            t!("pp-trimming", value = f.trimming.to_string()),
            t!("pp-delay", value = format::dec(f.delay, 3)),
            t!("pp-points", value = f.points.to_string()),
            t!("pp-contrast", value = f.contrast.to_string()),
        ];
        let mut out = Vec::new();
        for (k, ((name, get, set), (hint, text))) in SLIDERS
            .into_iter()
            .zip(hints.into_iter().zip(texts))
            .enumerate()
        {
            let y = TRIMMING.y + 20 * k as i32;
            let r = Rect::new(TRIMMING.x, y, TRIMMING.w, TRIMMING.h);
            let Some(state) = self.phase.wins.get(&win).map(|w| *get(w)) else {
                continue;
            };
            let id = format!("pp{win}.{name}");
            out.push(tip(format!("pp{win}.t.{name}"), r, &hint));
            out.push(trackbar(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: r,
                    label: "",
                    enabled: true,
                },
                state,
                slider_lens(win, k),
                Some(track_to_form(win, get, set)),
            ));
            out.push(label(
                cx,
                at(672, y, 150, 18),
                &text,
                false,
                true,
                Align::Left,
            ));
        }
        out.push(progress_field(cx, at(482, 380, 184, 18), progress));
        out
    }

    /// `Start` (`Break` while running), `Clear` and `LoadFile`.
    fn viewer_buttons(
        &mut self,
        win: u32,
        running: bool,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let Some(st) = self.phase_st(win) else {
            return Vec::new();
        };
        let phase = st.mode == Mode::Phase;
        // Break must always be available while running, even when Carrier or
        // BR was edited to something that is not a number meanwhile.
        let can_start = running || (phase && st.form.config().is_some());
        let start = if running {
            t!("pp-break")
        } else {
            t!("pp-start")
        };
        let scope = format!("mdi{win}");
        let buttons = [
            (START, "start", start, t!("pp-start.hover"), can_start),
            (CLEAR, "clear", t!("pp-clear"), t!("pp-clear.hover"), true),
            (
                LOAD_FILE,
                "load",
                t!("pp-load-file"),
                t!("pp-load-file.hover"),
                !running,
            ),
        ];
        let mut out = Vec::new();
        for (r, name, text, hint, enabled) in buttons {
            let id = format!("pp{win}.{name}");
            let on = match name {
                "start" => Self::on_st(win, PhaseState::start_or_break),
                "clear" => Self::on_st(win, PhaseState::clear),
                _ => action(move |v: &mut Self, _, _| v.open_phase_load(win)),
            };
            out.push(tip(format!("pp{win}.t.{name}"), r, &hint));
            out.push(push_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: &scope,
                    rect: r,
                    label: &text,
                    enabled,
                },
                false,
                on,
            ));
        }
        out
    }

    /// SA's status bar at the bottom of the form.
    fn phase_status_bar(
        &mut self,
        win: u32,
        node: sn_core::NodeId,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let rf = self
            .graph
            .get(node)
            .map_or(0.0, |n| n.signal.center_frequency());
        let hover = self.phase_win(win).map(|w| w.hover).unwrap_or_default();
        let texts = self
            .phase_st(win)
            .map(|st| phase_plot::status_texts(st, hover, rf))
            .unwrap_or_default();
        let s = scheme(cx);
        let panels = phase_plot::status_panels(FORM.0);
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let o = b.origin;
                for ((x, w), t) in panels.iter().zip(&texts) {
                    let r = Rect::new(*x, 0, *w, 20);
                    paint::edge(window, o, r, Edge::ThinSunken, &s);
                    let clip = paint::bounds(o, r.inset(1));
                    window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
                        let at = paint::at(o, x + 4, (20 - LINE) / 2);
                        text::draw(window, cx, at, t, false, s.of(Role::Text));
                    });
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        place(at(0, FORM.1 - 22, FORM.0, 20))
            .child(paint)
            .into_any_element()
    }
}
