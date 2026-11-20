//! The sonogram window's top toolbar (`docs/SA_UI_SPEC.md` 4.1): `SizeFFT`,
//! `Shear`, `Image`, `WinF`, `Fl`, `Fu`, `ALC`, `HRes` and `Info`, each a bold
//! caption and its control in a sunken group, all on one row. Changing any of
//! them redraws the same window.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, canvas,
};
use sn_core::NodeId;
use sn_dsp::window::WindowKind;
use sn_ui_core::sono_settings::{FFT_SIZES, Settings, Shear};
use sn_ui_core::sono_tools::{self, IMAGE_RANGES};
use sn_ui_core::t;

use super::Frame;
use crate::win95::bars::tooltip;
use crate::win95::button::{Changed, Ctl, check_box};
use crate::win95::colors::{Role, SaColors, sa_colors, scheme};
use crate::win95::combo::{ComboState, combo_box};
use crate::win95::edit::{EditState, FieldLook, edit};
use crate::win95::geom::{Edge, Rect};
use crate::win95::kit::lens;
use crate::win95::label::{Align, label};
use crate::win95::paint;
use crate::win95::scrollbar::THICKNESS;
use crate::win95::text;
use crate::win95::widget::place;

/// Height of a group panel.
const GROUP_H: i32 = 26;
/// Height of a control in a group.
const CTL_H: i32 = 21;
/// Width of the Fl and Fu fields.
const FIELD_W: i32 = 48;
/// Gap between groups.
const GROUP_GAP: i32 = 3;

/// Which control a group holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ctrl {
    Fft,
    Shear,
    Image,
    WinF,
    Fl,
    Fu,
    Alc,
    HRes,
    Info,
}

impl Ctrl {
    const ALL: [Self; 9] = [
        Self::Fft,
        Self::Shear,
        Self::Image,
        Self::WinF,
        Self::Fl,
        Self::Fu,
        Self::Alc,
        Self::HRes,
        Self::Info,
    ];

    fn caption(self) -> String {
        match self {
            Self::Fft => t!("app-sono-fft"),
            Self::Shear => t!("app-sono-shear"),
            Self::Image => t!("app-sono-image"),
            Self::WinF => t!("app-sono-window"),
            Self::Fl => t!("sw-fl"),
            Self::Fu => t!("sw-fu"),
            Self::Alc => t!("sw-alc"),
            Self::HRes => t!("sw-hres"),
            Self::Info => t!("sw-info"),
        }
    }

    fn hint(self) -> String {
        match self {
            Self::Fft => t!("app-sono-fft.hover"),
            Self::Shear => t!("app-sono-shear.hover"),
            Self::Image => t!("app-sono-image.hover"),
            Self::WinF => t!("app-sono-window.hover"),
            Self::Fl => t!("sw-fl.hover"),
            Self::Fu => t!("sw-fu.hover"),
            Self::Alc => t!("app-sono-alc-hover"),
            Self::HRes => t!("app-sono-hres-hover"),
            Self::Info => t!("sw-info.hover"),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Fft => "fft",
            Self::Shear => "shear",
            Self::Image => "image",
            Self::WinF => "winf",
            Self::Fl => "fl",
            Self::Fu => "fu",
            Self::Alc => "alc",
            Self::HRes => "hres",
            Self::Info => "info",
        }
    }
}

/// The items and selection of the four lists for `s`.
pub(crate) fn list_items(s: &Settings) -> [(Vec<String>, Option<usize>); 4] {
    let fft: Vec<String> = FFT_SIZES.iter().map(ToString::to_string).collect();
    let fft_sel = FFT_SIZES.iter().position(|n| *n == s.fft_size);
    let shear: Vec<String> = Shear::ALL.iter().map(|x| x.label()).collect();
    let shear_sel = Shear::ALL.iter().position(|x| *x == s.shear);
    let ranges = image_ranges(s);
    let image_sel = ranges.iter().position(|r| (r - s.range_db).abs() < 1e-3);
    let image: Vec<String> = ranges.iter().map(|r| sono_tools::image_text(*r)).collect();
    let winf: Vec<String> = WindowKind::ALL.iter().map(ToString::to_string).collect();
    let winf_sel = WindowKind::ALL.iter().position(|w| *w == s.window);
    [
        (fft, fft_sel),
        (shear, shear_sel),
        (image, image_sel),
        (winf, winf_sel),
    ]
}

/// The Image list's ranges in dB: SA's, and a range from the settings that
/// SA's list lacks (it stays selectable).
pub(crate) fn image_ranges(s: &Settings) -> Vec<f32> {
    let mut ranges: Vec<f32> = IMAGE_RANGES.to_vec();
    if !ranges.iter().any(|r| (r - s.range_db).abs() < 1e-3) {
        ranges.push(s.range_db);
        ranges.sort_by(f32::total_cmp);
    }
    ranges
}

/// Bring a list's items and selection up to date, keeping its open state.
fn sync_combo(c: &mut ComboState, (items, sel): (Vec<String>, Option<usize>)) {
    if c.items == items {
        c.selected = sel;
    } else {
        *c = ComboState::new(&items.iter().map(String::as_str).collect::<Vec<_>>(), sel);
    }
}

/// Bring a read-only field's text up to date.
fn sync_field(f: &mut EditState, text: &str, look: FieldLook) {
    if f.text != text {
        f.set_text(text);
    }
    f.read_only = true;
    f.look = Some(look);
}

impl Frame {
    /// Bring the top toolbar's controls of window `win` up to date.
    fn sync_top(&mut self, win: u32, node: NodeId, cx: &gpui::App) {
        let Some(s) = self.ui.get(&node).map(|u| u.settings) else {
            return;
        };
        let rf = self
            .graph
            .get(node)
            .map_or(0.0, |n| n.signal.center_frequency());
        let sch = scheme(cx);
        let Some(sw) = self.sono.wins.get_mut(&win) else {
            return;
        };
        let [a, b, c, d] = list_items(&s);
        sync_combo(&mut sw.fft, a);
        sync_combo(&mut sw.shear, b);
        sync_combo(&mut sw.image, c);
        sync_combo(&mut sw.winf, d);
        let (lo, hi, zoomed) = sono_tools::shown_band(sw.extent, sw.view, rf);
        // Grey while the whole band shows; blue with white text while zoomed.
        let look = match (zoomed, sa_colors(cx) == SaColors::SA) {
            (true, true) => FieldLook {
                bg: 0x00_00_A0,
                fg: 0xFF_FF_FF,
                border: true,
            },
            (true, false) => FieldLook {
                bg: sch.rgb(Role::Selection),
                fg: sch.rgb(Role::SelectionText),
                border: true,
            },
            (false, _) => FieldLook {
                bg: sch.rgb(Role::Face),
                fg: sch.rgb(Role::Text),
                border: true,
            },
        };
        sync_field(&mut sw.fl, &lo, look);
        sync_field(&mut sw.fu, &hi, look);
    }

    /// The width of the control of `c`.
    fn control_w(&self, win: u32, c: Ctrl, window: &Window) -> i32 {
        let list_w = |st: Option<&ComboState>| {
            st.map_or(0, |st| {
                st.items
                    .iter()
                    .map(|i| text::width(window, i, false))
                    .max()
                    .unwrap_or(0)
            }) + THICKNESS
                + 10
        };
        let sw = self.sono.wins.get(&win);
        match c {
            Ctrl::Fft => list_w(sw.map(|w| &w.fft)),
            Ctrl::Shear => list_w(sw.map(|w| &w.shear)),
            Ctrl::Image => list_w(sw.map(|w| &w.image)),
            Ctrl::WinF => list_w(sw.map(|w| &w.winf)),
            Ctrl::Fl | Ctrl::Fu => FIELD_W,
            Ctrl::Alc | Ctrl::HRes | Ctrl::Info => 14,
        }
    }

    /// The top toolbar of window `win` in `r` (client coordinates).
    pub(crate) fn top_toolbar(
        &mut self,
        win: u32,
        node: NodeId,
        r: Rect,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        self.sync_top(win, node, cx);
        let s = scheme(cx);
        let mut out = Vec::new();
        let mut x = r.x + 2;
        for c in Ctrl::ALL {
            let caption = c.caption();
            let cap_w = text::width(window, &caption, true);
            let ctl_w = self.control_w(win, c, window);
            let gw = 4 + cap_w + 4 + ctl_w + 4;
            let g = Rect::new(x, r.y + (r.h - GROUP_H) / 2, gw, GROUP_H);
            let panel = canvas(
                |_, _, _| (),
                move |b, (), window, _| {
                    let (w, h) = crate::win95::widget::size_of(b);
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
            let at = Rect::new(4 + cap_w + 4, (GROUP_H - CTL_H) / 2, ctl_w, CTL_H);
            let control = self.top_control(win, node, c, at, cx);
            let el = place(g)
                .id(gpui::ElementId::Name(
                    format!("sw{win}.g.{}", c.name()).into(),
                ))
                .tooltip(tooltip(&c.hint()))
                .child(panel)
                .child(cap)
                .child(control);
            out.push(el.into_any_element());
            x += gw + GROUP_GAP;
        }
        out
    }

    /// The control of group `c` at `at` (relative to its group).
    fn top_control(
        &mut self,
        win: u32,
        node: NodeId,
        c: Ctrl,
        at: Rect,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let id = format!("sw{win}.{}", c.name());
        let scope = format!("mdi{win}");
        let ctl = Ctl {
            id: &id,
            scope: &scope,
            rect: at,
            label: "",
            enabled: true,
        };
        let Some(sw) = self.sono.wins.get(&win) else {
            return gpui::Empty.into_any_element();
        };
        let settings = self.ui.get(&node).map(|u| u.settings).unwrap_or_default();
        let info = self.ui.get(&node).is_some_and(|u| u.info);
        match c {
            Ctrl::Fft | Ctrl::Shear | Ctrl::Image | Ctrl::WinF => {
                let st = match c {
                    Ctrl::Fft => sw.fft.clone(),
                    Ctrl::Shear => sw.shear.clone(),
                    Ctrl::Image => sw.image.clone(),
                    _ => sw.winf.clone(),
                };
                let l = lens(move |v: &mut Self| {
                    let sw = v.sono.wins.get_mut(&win)?;
                    Some(match c {
                        Ctrl::Fft => &mut sw.fft,
                        Ctrl::Shear => &mut sw.shear,
                        Ctrl::Image => &mut sw.image,
                        _ => &mut sw.winf,
                    })
                });
                let on: Changed<Self, usize> = Rc::new(move |v: &mut Self, i, _, _| {
                    v.list_chosen(node, c, i);
                });
                combo_box(&mut self.kit, cx, ctl, &st, l, Some(on))
            }
            Ctrl::Fl | Ctrl::Fu => {
                let st = if c == Ctrl::Fl {
                    sw.fl.clone()
                } else {
                    sw.fu.clone()
                };
                let l = lens(move |v: &mut Self| {
                    let sw = v.sono.wins.get_mut(&win)?;
                    Some(if c == Ctrl::Fl {
                        &mut sw.fl
                    } else {
                        &mut sw.fu
                    })
                });
                edit(&mut self.kit, cx, ctl, &st, l, None)
            }
            Ctrl::Alc | Ctrl::HRes | Ctrl::Info => {
                let checked = match c {
                    Ctrl::Alc => settings.alc,
                    Ctrl::HRes => settings.hres,
                    _ => info,
                };
                let on: Changed<Self, bool> = Rc::new(move |v: &mut Self, on, _, _| {
                    if let Some(u) = v.ui.get_mut(&node) {
                        match c {
                            Ctrl::Alc => u.settings.alc = on,
                            Ctrl::HRes => u.settings.hres = on,
                            _ => u.info = on,
                        }
                    }
                });
                check_box(&mut self.kit, cx, ctl, checked, on)
            }
        }
    }

    /// Item `i` of list `c` was chosen: change the display settings.
    pub(crate) fn list_chosen(&mut self, node: NodeId, c: Ctrl, i: usize) {
        let Some(u) = self.ui.get_mut(&node) else {
            return;
        };
        let s = &mut u.settings;
        match c {
            Ctrl::Fft => {
                if let Some(n) = FFT_SIZES.get(i) {
                    s.fft_size = *n;
                }
            }
            Ctrl::Shear => {
                if let Some(x) = Shear::ALL.get(i) {
                    s.shear = *x;
                }
            }
            // By index into the same ranges the list shows (the label is
            // rounded, so it is never parsed back).
            Ctrl::Image => {
                if let Some(db) = image_ranges(s).get(i) {
                    s.range_db = *db;
                }
            }
            Ctrl::WinF => {
                if let Some(w) = WindowKind::ALL.get(i) {
                    s.window = *w;
                }
            }
            _ => {}
        }
    }

    /// The width the top toolbar needs (for the default window size check).
    #[cfg(test)]
    pub(crate) fn top_toolbar_width(&mut self, win: u32, window: &Window) -> i32 {
        let mut x = 2;
        for c in Ctrl::ALL {
            let cap_w = text::width(window, &c.caption(), true);
            x += 4 + cap_w + 4 + self.control_w(win, c, window) + 4 + GROUP_GAP;
        }
        x
    }
}
