//! `LoadFile`'s dialog `SA - select file for PhasePlane` (M-p15): a list of
//! the open working files (by window title), `Ok` and `Cancel`; a double click
//! picks a file. The chosen file switches the form to its phase plane mode.

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, Styled, canvas, div,
};
use sn_core::NodeId;
use sn_ui_core::t;

use crate::app::Frame;
use crate::app::model::WinKind;
use crate::win95::button::{Ctl, push_button};
use crate::win95::colors::{Role, scheme};
use crate::win95::dialog::{BUTTON, DialogHooks, MARGIN, dialog_size, modal_dialog};
use crate::win95::geom::{Edge, Rect};
use crate::win95::kit::{action, lens};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, place, size_of};

/// The dialog's client size.
pub(crate) const CLIENT: (i32, i32) = (420, 250);
/// The list of files.
pub(crate) const LIST: Rect = Rect::new(MARGIN, MARGIN, CLIENT.0 - 2 * MARGIN, 200);
/// Height of a row of the list.
const ROW: i32 = LINE + 3;
/// Rows the list shows at once.
fn visible_rows() -> i32 {
    ((LIST.h - 4) / ROW).max(1)
}

/// The tab scope of the dialog.
const SCOPE: &str = "ppload";

/// The open `LoadFile` dialog.
#[derive(Clone, Debug)]
pub(crate) struct LoadDialog {
    /// The PhasePlane window it loads into.
    pub(crate) win: u32,
    /// The file selected in the list.
    pub(crate) sel: Option<NodeId>,
    /// The list's scroll bar (the first row shown).
    pub(crate) scroll: ScrollState,
    pub(crate) rect: Rect,
    placed: bool,
    pub(crate) outline: Option<Rect>,
}

/// Where `Ok` and `Cancel` go in the client area.
pub(crate) fn buttons() -> (Rect, Rect) {
    let y = CLIENT.1 - MARGIN - BUTTON.1 + 4;
    (
        Rect::new(MARGIN, y, BUTTON.0, BUTTON.1),
        Rect::new(CLIENT.0 - MARGIN - BUTTON.0, y, BUTTON.0, BUTTON.1),
    )
}

impl Frame {
    /// The working files `LoadFile` lists: the signal of every open sonogram
    /// window, with its title.
    pub(crate) fn working_files(&self) -> Vec<(NodeId, String)> {
        let mut files: Vec<(NodeId, String)> = Vec::new();
        for (w, n, k) in self.signal_windows() {
            if k == WinKind::Sonogram && !files.iter().any(|(f, _)| *f == n) {
                let title = self.mdi.get(w).map(|c| c.title.clone()).unwrap_or_default();
                files.push((n, title));
            }
        }
        files
    }

    /// `LoadFile` of window `win`: open the dialog with the loaded file
    /// selected.
    pub(crate) fn open_phase_load(&mut self, win: u32) {
        let sel = self.phase_st(win).and_then(|st| st.loaded);
        self.phase.load = Some(LoadDialog {
            win,
            sel,
            scroll: ScrollState::default(),
            rect: Rect::default(),
            placed: false,
            outline: None,
        });
        self.kit.focus = None;
    }

    /// The dialog is open.
    pub(crate) fn phase_load_open(&self) -> bool {
        self.phase.load.is_some()
    }

    /// `Cancel`, Escape or the close button.
    pub(crate) fn phase_load_cancel(&mut self) {
        self.phase.load = None;
        self.kit.focus = None;
    }

    /// `Ok`, Enter or a double click: load the selected file. A file whose
    /// window closed while the dialog was open is no longer a working file.
    pub(crate) fn phase_load_ok(&mut self) {
        let Some(d) = self.phase.load.clone() else {
            return;
        };
        let Some(id) = d.sel else {
            return;
        };
        let Some(title) = self
            .working_files()
            .into_iter()
            .find(|(n, _)| *n == id)
            .map(|(_, t)| t)
        else {
            if let Some(l) = self.phase.load.as_mut() {
                l.sel = None;
            }
            if let Some(st) = self.phase_st(d.win) {
                st.error = Some(t!("pp-load-gone"));
            }
            return;
        };
        self.phase_load_cancel();
        self.phase_load_file(d.win, id, title);
    }

    /// Keys of the dialog: Escape cancels, Enter loads, Tab moves between the
    /// buttons, the arrows move the selection.
    pub(crate) fn phase_load_key(&mut self, ev: &KeyDownEvent) {
        match ev.keystroke.key.as_str() {
            "escape" => self.phase_load_cancel(),
            "enter" if self.kit.has_focus("ppload.cancel") => self.phase_load_cancel(),
            "enter" => self.phase_load_ok(),
            "tab" => self.kit.tab(SCOPE, ev.keystroke.modifiers.shift),
            k @ ("up" | "down") => {
                let files = self.working_files();
                let Some(d) = self.phase.load.as_mut() else {
                    return;
                };
                let i = d.sel.and_then(|s| files.iter().position(|(n, _)| *n == s));
                let next = match (i, k) {
                    (None, _) => 0,
                    (Some(i), "up") => i.saturating_sub(1),
                    (Some(i), _) => (i + 1).min(files.len().saturating_sub(1)),
                };
                d.sel = files.get(next).map(|(n, _)| *n);
                // Keep the selection in view, as a list box does.
                let next = next as i32;
                let page = visible_rows();
                if next < d.scroll.pos {
                    d.scroll.set(next);
                } else if next >= d.scroll.pos + page {
                    d.scroll.set(next - page + 1);
                }
            }
            _ => {}
        }
    }

    /// The dialog over the frame.
    pub(crate) fn phase_load_view(
        &mut self,
        cx: &mut Context<'_, Self>,
        vw: i32,
        vh: i32,
    ) -> Option<AnyElement> {
        let d = self.phase.load.as_mut()?;
        if !d.placed {
            let (w, h) = dialog_size(CLIENT.0, CLIENT.1);
            d.rect = Rect::new(((vw - w) / 2).max(0), ((vh - h) / 2).max(0), w, h);
            d.placed = true;
        }
        let d = d.clone();
        let files = self.working_files();
        let n = files.len() as i32;
        let scroll = ScrollState::new(0, (n - 1).max(0), visible_rows(), d.scroll.pos);
        if let Some(l) = self.phase.load.as_mut() {
            l.scroll = scroll;
        }
        let mut body = div().relative().size_full();
        body = body.child(self.load_list(&files, d.sel, scroll, cx));
        let (ok, cancel) = buttons();
        body = body.child(push_button(
            &mut self.kit,
            cx,
            Ctl {
                id: "ppload.ok",
                scope: SCOPE,
                rect: ok,
                label: &t!("pp-ok"),
                enabled: d.sel.is_some(),
            },
            true,
            action(|v: &mut Self, _, _| v.phase_load_ok()),
        ));
        body = body.child(push_button(
            &mut self.kit,
            cx,
            Ctl {
                id: "ppload.cancel",
                scope: SCOPE,
                rect: cancel,
                label: &t!("pp-cancel"),
                enabled: true,
            },
            false,
            action(|v: &mut Self, _, _| v.phase_load_cancel()),
        ));
        let hooks = DialogHooks {
            rect: lens(|v: &mut Self| v.phase.load.as_mut().map(|d| &mut d.rect)),
            outline: lens(|v: &mut Self| v.phase.load.as_mut().map(|d| &mut d.outline)),
            on_close: Some(action(|v: &mut Self, _, _| v.phase_load_cancel())),
        };
        Some(modal_dialog(
            &self.kit,
            cx,
            "ppload",
            &t!("pp-load-title"),
            d.rect,
            d.outline,
            body.into_any_element(),
            &hooks,
        ))
    }

    /// The white list box of working files, from row `scroll.pos`, with a
    /// scroll bar when they do not all fit.
    fn load_list(
        &self,
        files: &[(NodeId, String)],
        sel: Option<NodeId>,
        scroll: ScrollState,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let s = scheme(cx);
        let top = scroll.pos.max(0) as usize;
        let long = scroll.enabled();
        let rows_w = if long { LIST.w - THICKNESS - 2 } else { LIST.w };
        let rows: Vec<(String, bool)> = files
            .iter()
            .skip(top)
            .map(|(n, t)| (t.clone(), Some(*n) == sel))
            .collect();
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let (w, h) = size_of(b);
                let o = b.origin;
                paint::fill(window, o, Rect::new(0, 0, w, h), s.of(Role::Window));
                paint::edge(window, o, Rect::new(0, 0, w, h), Edge::Sunken, &s);
                let w = rows_w.min(w);
                let clip = paint::bounds(o, Rect::new(2, 2, w - 4, h - 4));
                window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
                    for (i, (title, on)) in (0_i32..).zip(&rows) {
                        let y = 2 + i * ROW;
                        let ink = if *on {
                            paint::fill(
                                window,
                                o,
                                Rect::new(2, y, w - 4, ROW),
                                s.of(Role::Selection),
                            );
                            s.of(Role::SelectionText)
                        } else {
                            s.of(Role::Text)
                        };
                        text::draw(window, cx, paint::at(o, 4, y + 1), title, false, ink);
                    }
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let ids: Vec<NodeId> = files.iter().map(|(n, _)| *n).collect();
        let slot = Slot::default();
        let s2 = slot.clone();
        let bar = long.then(|| {
            scroll_bar(
                &self.kit,
                cx,
                Bar {
                    id: "ppload.sb",
                    rect: Rect::new(LIST.w - 2 - THICKNESS, 2, THICKNESS, LIST.h - 4),
                    vertical: true,
                },
                scroll,
                lens(|v: &mut Self| v.phase.load.as_mut().map(|d| &mut d.scroll)),
                None,
            )
        });
        place(LIST)
            .id("ppload.list")
            .child(paint)
            .children(bar)
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
                    if let Some((x, y)) = slot.local(ev.position)
                        && x < rows_w - 2
                    {
                        let i = usize::try_from((y - 2) / ROW).ok().map(|i| i + top);
                        if let (Some(n), Some(d)) =
                            (i.and_then(|i| ids.get(i)), v.phase.load.as_mut())
                        {
                            d.sel = Some(*n);
                            if ev.click_count >= 2 {
                                v.phase_load_ok();
                            }
                        }
                    }
                    cx.notify();
                }),
            )
            .into_any_element()
    }
}
