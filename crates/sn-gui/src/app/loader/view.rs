//! Drawing and input of `SA Open file` (layout of 6.2.6.4, R278/a1): the path
//! combo and toolbar, the navy file list (list and details views), the info
//! panel, the options group, `Ext. Info`, quick search with the filter, `Ok`
//! and `Cancel`, the preview with its prescaler and the status bar; and the
//! `SA - select channel` question.

use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    AnyElement, ClipboardItem, Context, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, Styled, Window, canvas, deferred, div,
};
use sn_dsp::overview::MinMax;
use sn_ui_core::loader_files::{self, Entry, EntryKind, ExtFilter};
use sn_ui_core::loader_info::StereoChoice;
use sn_ui_core::t;

use super::glyphs;
use super::{Loader, LoaderAction, PRESCALER_MAX_DB};
use crate::app::{Frame, menus::cmd};
use crate::win95::bars::{Tool, tool_button};
use crate::win95::button::{Ctl, check_box, push_button, radio_button};
use crate::win95::colors::{Role, SaColors, Scheme, hsla, sa_colors, scheme};
use crate::win95::combo::combo_box;
use crate::win95::dialog::{BUTTON, DialogHooks, dialog_size, modal_dialog};
use crate::win95::edit::{FieldLook, edit, spin_edit};
use crate::win95::geom::{Edge, Rect};
use crate::win95::glyph::{self, Glyph, Mode};
use crate::win95::kit::{KeyHandler, action, dispatch_key, lens};
use crate::win95::label::group_box;
use crate::win95::marks::{self, Dir};
use crate::win95::menu::{Check, Item};
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, THICKNESS, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::{Slot, place, size_of};

/// Focus id of the file list.
pub(crate) const LIST_ID: &str = "ld.list";
/// Focus id of the path field.
const PATH_ID: &str = "ld.path";
/// Tab scope of the loader.
const SCOPE: &str = "loader";
/// Tab scope of `SA - select channel`.
const PROMPT_SCOPE: &str = "ld.prompt";
/// Size of the window's content (SA's form is about 800 x 540).
const SIZE: (i32, i32) = (784, 492);
/// The file list.
const LIST: Rect = Rect {
    x: 0,
    y: 28,
    w: 616,
    h: 214,
};
/// Height of a list row.
const ROW: i32 = 17;
/// Details view column widths (Type/Ext, Size, Date, Time); Name takes the rest.
const COLUMNS: [i32; 4] = [80, 90, 80, 72];
/// Client size of `SA - select channel`.
const PROMPT_SIZE: (i32, i32) = (250, 127);

/// Text cut to `w` pixels, ending in SA's three dots when cut.
fn fit(window: &Window, s: &str, w: i32, bold: bool) -> String {
    if text::width(window, s, bold) <= w {
        return s.to_owned();
    }
    // Binary search over the char boundaries: the longest prefix that fits
    // with the dots (about log2(n) measurements instead of n).
    let cuts: Vec<usize> = s.char_indices().map(|(i, _)| i).collect();
    let with_dots = |k: usize| {
        format!(
            "{}...",
            s.get(..cuts.get(k).copied().unwrap_or(0)).unwrap_or("")
        )
    };
    let fits = |k: usize| text::width(window, &with_dots(k), bold) <= w;
    if !fits(0) {
        return String::new();
    }
    // `lo` fits; `hi` is one past the last candidate (the whole text is cut).
    let (mut lo, mut hi) = (0, cuts.len());
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    with_dots(lo)
}

/// The inner area of the list (inside its sunken border).
fn list_inner() -> Rect {
    Rect::new(2, 2, LIST.w - 4, LIST.h - 4)
}

/// Rows in a column of the list view (above its horizontal scroll bar).
pub(crate) fn rows_per_column() -> usize {
    ((list_inner().h - THICKNESS) / ROW).max(1) as usize
}

/// Rows the details view shows under its header.
fn detail_rows() -> i32 {
    ((list_inner().h - (ROW + 2)) / ROW).max(1)
}

/// Width of a column of the list view.
fn column_w() -> i32 {
    (list_inner().w / 2).max(120)
}

/// The entry index of each visible cell: `(cell rectangle in list
/// coordinates, entry index)`.
fn cells(l: &Loader) -> Vec<(Rect, usize)> {
    let inner = list_inner();
    let pos = l.w.list.pos.max(0) as usize;
    if l.details {
        let top = inner.y + ROW + 2;
        let w = inner.w - THICKNESS;
        l.shown
            .iter()
            .enumerate()
            .skip(pos)
            .take(detail_rows() as usize)
            .map(|(r, &i)| (Rect::new(inner.x, top + (r - pos) as i32 * ROW, w, ROW), i))
            .collect()
    } else {
        let rpc = rows_per_column();
        let cw = column_w();
        l.shown
            .iter()
            .enumerate()
            .skip(pos * rpc)
            .take(2 * rpc)
            .map(|(r, &i)| {
                let (col, line) = ((r / rpc - pos) as i32, (r % rpc) as i32);
                (
                    Rect::new(inner.x + col * cw, inner.y + line * ROW, cw - 4, ROW),
                    i,
                )
            })
            .collect()
    }
}

impl Loader {
    /// Set the list's scroll range and bring the selection into view.
    fn fit_list(&mut self) {
        let n = self.shown.len() as i32;
        let pos = self.w.list.pos;
        self.w.list = if self.details {
            crate::win95::scrollbar::ScrollState::new(0, (n - 1).max(0), detail_rows(), pos)
        } else {
            let rpc = rows_per_column() as i32;
            let cols = ((n + rpc - 1) / rpc).max(1);
            crate::win95::scrollbar::ScrollState::new(0, cols - 1, 2, pos)
        };
        self.rows_per_column = rows_per_column();
        if !std::mem::take(&mut self.reveal) {
            return;
        }
        let Some(at) = self
            .selected
            .and_then(|s| self.shown.iter().position(|&i| i == s))
        else {
            return;
        };
        let (item, page) = if self.details {
            (at as i32, detail_rows())
        } else {
            ((at / rows_per_column()) as i32, 2)
        };
        let st = &mut self.w.list;
        if item < st.pos {
            st.set(item);
        } else if item >= st.pos + page {
            st.set(item - page + 1);
        }
    }
}

impl Frame {
    /// Open `SA Open file`: on the folder used last, or on `path`'s folder with
    /// `path` selected (a dropped file or one named on the command line).
    pub(crate) fn open_loader(&mut self, path: Option<PathBuf>) {
        let raw = self.prefs.raw;
        let l = match path {
            Some(p) => Loader::new(p, raw, &self.loader_memory),
            None => Loader::browse(raw, &self.loader_memory),
        };
        self.loader = Some(l);
        self.kit.focus = Some(LIST_ID.into());
        self.track.close();
        self.popup = None;
        self.raise_loader_error();
    }

    /// Open the first of the dropped files in the loader. A loader the user is
    /// setting up is not replaced; extra files are named in the status bar.
    pub(crate) fn open_dropped(&mut self, dropped: &[PathBuf]) {
        let Some(path) = dropped.first() else {
            return;
        };
        if self.loader.is_some() {
            self.status = t!("app-drop-loader-open");
            return;
        }
        if dropped.len() > 1 {
            self.status = t!(
                "app-drop-first-only",
                count = dropped.len(),
                name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            );
        }
        self.open_loader(Some(path.clone()));
    }

    /// Carry out what the loader decided.
    pub(crate) fn loader_action(&mut self, a: LoaderAction) {
        match a {
            LoaderAction::None => self.raise_loader_error(),
            LoaderAction::Cancel => self.close_loader(),
            LoaderAction::Load(path, options) => {
                self.close_loader();
                self.start_load(path, options);
            }
        }
    }

    fn close_loader(&mut self) {
        if let Some(l) = self.loader.take() {
            self.loader_memory = l.memory();
        }
        self.kit.focus = None;
        self.kit.popup = None;
    }

    /// Show the loader's error in SA's error box.
    pub(crate) fn raise_loader_error(&mut self) {
        if let Some(e) = self.loader.as_mut().and_then(|l| l.error.take()) {
            self.alert(&e);
        }
    }

    fn with_loader(&mut self, f: impl FnOnce(&mut Loader) -> LoaderAction) {
        if let Some(l) = self.loader.as_mut() {
            let a = f(l);
            self.loader_action(a);
        }
    }

    /// A command of the loader's popup menus.
    pub(crate) fn loader_command(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(l) = self.loader.as_mut() else {
            return;
        };
        match id {
            cmd::LD_UPDATE => l.reload(),
            cmd::LD_AUTO => l.auto_preview = !l.auto_preview,
            cmd::LD_COPY => {
                cx.write_to_clipboard(ClipboardItem::new_string(l.dir.display().to_string()));
            }
            cmd::LD_PASTE => {
                let text = cx
                    .read_from_clipboard()
                    .and_then(|c| c.text())
                    .unwrap_or_default();
                if !text.trim().is_empty() {
                    l.go_text(text.lines().next().unwrap_or(""));
                }
            }
            _ => {}
        }
        self.raise_loader_error();
    }

    /// Keys while the loader is open. Returns whether the key was used.
    pub(crate) fn loader_key(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.loader.as_ref().is_some_and(|l| l.prompt.is_some()) {
            self.prompt_key(ev, window, cx);
            return true;
        }
        let k = ev.keystroke.key.as_str();
        match k {
            "tab" => self.kit.tab(SCOPE, ev.keystroke.modifiers.shift),
            "escape" if self.kit.popup.is_some() => {
                if !dispatch_key(self, ev, window, cx) {
                    self.kit.popup = None;
                }
            }
            "escape" if self.loader.as_ref().is_some_and(|l| l.path_list) => {
                if let Some(l) = self.loader.as_mut() {
                    l.path_list = false;
                }
            }
            "escape" => self.loader_action(LoaderAction::Cancel),
            "enter" if self.kit.popup.is_some() => {
                dispatch_key(self, ev, window, cx);
            }
            "enter" if self.kit.has_focus(PATH_ID) => {
                if let Some(l) = self.loader.as_mut() {
                    let text = l.w.path.text.clone();
                    l.go_text(&text);
                }
                self.raise_loader_error();
            }
            "enter" => self.with_loader(Loader::ok),
            _ => {
                dispatch_key(self, ev, window, cx);
            }
        }
        true
    }

    /// Keys of `SA - select channel`: arrows choose, Enter is `Ok`, Escape is
    /// `Break`.
    fn prompt_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<'_, Self>) {
        let k = ev.keystroke.key.as_str();
        let order = [StereoChoice::First, StereoChoice::Second, StereoChoice::All];
        match k {
            "up" | "down" => {
                if let Some(p) = self.loader.as_mut().and_then(|l| l.prompt.as_mut()) {
                    let i = order.iter().position(|c| *c == p.choice).unwrap_or(2);
                    let n = if k == "up" { (i + 2) % 3 } else { (i + 1) % 3 };
                    p.choice = order.get(n).copied().unwrap_or_default();
                }
            }
            "tab" => self.kit.tab(PROMPT_SCOPE, ev.keystroke.modifiers.shift),
            "escape" => self.prompt_answer(false),
            "enter" if !self.kit.has_focus("ldp.break") => self.prompt_answer(true),
            _ => {
                dispatch_key(self, ev, window, cx);
            }
        }
    }

    /// `Ok` loads with the chosen channel; `Break` returns to the loader.
    fn prompt_answer(&mut self, ok: bool) {
        let Some(l) = self.loader.as_mut() else {
            return;
        };
        let Some(p) = l.prompt.take() else {
            return;
        };
        self.kit.focus = Some(LIST_ID.into());
        if ok {
            let a = l.load(p.choice);
            self.loader_action(a);
        }
    }

    /// The loader and its question over the frame.
    pub(crate) fn loader_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
        vw: i32,
        vh: i32,
    ) -> Option<Vec<AnyElement>> {
        let l = self.loader.as_mut()?;
        if !l.placed {
            let (w, h) = dialog_size(SIZE.0, SIZE.1);
            l.rect = Rect::new(((vw - w) / 2).max(0), ((vh - h) / 2).max(0), w, h);
            l.placed = true;
        }
        l.fit_list();
        let (rect, outline) = (l.rect, l.outline);
        let mut body = div().relative().size_full();
        for el in self.loader_top(cx) {
            body = body.child(el);
        }
        body = body.child(self.file_list(window, cx));
        for el in self.loader_side(cx) {
            body = body.child(el);
        }
        for el in self.loader_bottom(cx) {
            body = body.child(el);
        }
        if let Some(el) = self.path_list(cx) {
            body = body.child(el);
        }
        let hooks = DialogHooks {
            rect: lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.rect)),
            outline: lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.outline)),
            on_close: Some(action(|v: &mut Self, _, _| {
                v.loader_action(LoaderAction::Cancel);
            })),
        };
        let mut out = vec![modal_dialog(
            &self.kit,
            cx,
            "loader",
            &t!("loader-title"),
            rect,
            outline,
            body.into_any_element(),
            &hooks,
        )];
        if let Some(p) = self.prompt_view(cx, vw, vh) {
            out.push(p);
        }
        Some(out)
    }

    /// The path combo, its drop button and the toolbar buttons.
    fn loader_top(&mut self, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(l) = self.loader.as_ref() else {
            return Vec::new();
        };
        let s = scheme(cx);
        let mut path = l.w.path.clone();
        path.look = Some(FieldLook {
            bg: s.window,
            fg: s.text,
            border: false,
        });
        let (has_back, has_up, details) = (!l.back.is_empty(), l.dir.parent().is_some(), l.details);
        let field = Rect::new(0, 0, 616, 22);
        let frame = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let (w, h) = size_of(b);
                let r = Rect::new(0, 0, w, h);
                paint::fill(window, b.origin, r, s.of(Role::Window));
                paint::edge(window, b.origin, r, Edge::Sunken, &s);
                let g = glyphs::folder_icon(false);
                glyph::draw(window, cx, b.origin, (3, 3), &g, Mode::Normal, &s);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let mut out = vec![place(field).child(frame).into_any_element()];
        let path_el = edit(
            &mut self.kit,
            cx,
            Ctl {
                id: PATH_ID,
                scope: SCOPE,
                rect: Rect::new(21, 2, 577, 18),
                label: "",
                enabled: true,
            },
            &path,
            lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.w.path)),
            None,
        );
        let popup_at = cx.listener(|v: &mut Self, ev: &MouseDownEvent, _, cx| {
            let at = (
                f32::from(ev.position.x) as i32,
                f32::from(ev.position.y) as i32,
            );
            v.open_popup(path_popup(), at);
            cx.notify();
        });
        out.push(
            div()
                .id("ld.pathwrap")
                .child(path_el)
                .on_mouse_down(MouseButton::Right, popup_at)
                .into_any_element(),
        );
        out.push(drop_button(cx, Rect::new(599, 2, 15, 18)));
        out.extend(self.loader_tools(cx, has_back, has_up, details));
        out
    }

    /// Back, folder up, view mode and refresh.
    fn loader_tools(
        &mut self,
        cx: &mut Context<'_, Self>,
        has_back: bool,
        has_up: bool,
        details: bool,
    ) -> Vec<AnyElement> {
        let mut out = Vec::new();
        let tools: [(&str, Glyph, bool, String, u8); 4] = [
            (
                "ld.back",
                glyphs::back(),
                has_back,
                t!("loader-back.hover"),
                0,
            ),
            (
                "ld.up",
                glyphs::folder_up(),
                has_up,
                t!("loader-up.hover"),
                1,
            ),
            (
                "ld.view",
                glyphs::view_mode(details),
                true,
                t!("loader-view.hover"),
                2,
            ),
            (
                "ld.refresh",
                glyphs::refresh(),
                true,
                t!("loader-refresh.hover"),
                3,
            ),
        ];
        for (i, (id, g, enabled, tip, which)) in (0_i32..).zip(tools) {
            let t = Tool {
                id,
                at: (624 + i * 27, 0),
                glyph: &g,
                enabled,
                checked: false,
                tip: &tip,
            };
            out.push(tool_button(
                &self.kit,
                cx,
                t,
                action(move |v: &mut Self, _, _| {
                    if let Some(l) = v.loader.as_mut() {
                        match which {
                            0 => l.go_back(),
                            1 => l.go_up(),
                            2 => l.details = !l.details,
                            _ => l.reload(),
                        }
                        l.reveal = true;
                    }
                }),
            ));
        }
        out
    }

    /// The path combo's open list: the folders above this one, then the
    /// folders used.
    fn path_list(&mut self, cx: &mut Context<'_, Self>) -> Option<AnyElement> {
        let l = self.loader.as_ref().filter(|l| l.path_list)?;
        let chain = loader_files::ancestors(&l.dir);
        let mut rows: Vec<(String, Option<PathBuf>)> = chain
            .iter()
            .enumerate()
            .map(|(depth, d)| {
                let name = format!("{}{}", "   ".repeat(depth), loader_files::folder_name(d));
                (name, Some(d.clone()))
            })
            .collect();
        let others: Vec<&PathBuf> = l.history.iter().filter(|h| !chain.contains(h)).collect();
        if !others.is_empty() {
            rows.push((String::new(), None));
            rows.extend(
                others
                    .into_iter()
                    .map(|d| (d.display().to_string(), Some(d.clone()))),
            );
        }
        let s = scheme(cx);
        let h = rows.len() as i32 * (LINE + 2) + 2;
        let r = Rect::new(0, 22, 616, h);
        let texts: Vec<String> = rows.iter().map(|(t, _)| t.clone()).collect();
        let current = chain.iter().position(|d| *d == l.dir);
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let (w, h) = size_of(b);
                let all = Rect::new(0, 0, w, h);
                paint::fill(window, b.origin, all, s.of(Role::Window));
                paint::edge(window, b.origin, all, Edge::Flat(Role::DarkShadow), &s);
                for (i, t) in (0_i32..).zip(&texts) {
                    let row = Rect::new(1, 1 + i * (LINE + 2), w - 2, LINE + 2);
                    let ink = if current == Some(i as usize) {
                        paint::fill(window, b.origin, row, s.of(Role::Selection));
                        s.of(Role::SelectionText)
                    } else {
                        s.of(Role::Text)
                    };
                    text::draw(window, cx, paint::at(b.origin, 4, row.y + 1), t, false, ink);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let targets: Vec<Option<PathBuf>> = rows.into_iter().map(|(_, p)| p).collect();
        let slot = Slot::default();
        let s2 = slot.clone();
        let measure = canvas(move |b, _, _| s2.set(b), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let el = place(r)
            .id("ld.pathlist")
            .occlude()
            .child(paint)
            .child(measure)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    let row = slot.local(ev.position).map(|(_, y)| (y - 1) / (LINE + 2));
                    let target = row
                        .and_then(|i| usize::try_from(i).ok())
                        .and_then(|i| targets.get(i).cloned())
                        .flatten();
                    if let Some(l) = v.loader.as_mut() {
                        l.path_list = false;
                        if let Some(d) = target {
                            l.go(d);
                        }
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            );
        Some(deferred(el).with_priority(1).into_any_element())
    }

    /// The navy file list.
    fn file_list(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> AnyElement {
        let Some(l) = self.loader.as_ref() else {
            return div().into_any_element();
        };
        let (s, sa) = (scheme(cx), sa_colors(cx));
        let rows: Vec<(Rect, Entry, bool)> = cells(l)
            .into_iter()
            .filter_map(|(r, i)| {
                l.entries
                    .get(i)
                    .map(|e| (r, e.clone(), l.selected == Some(i)))
            })
            .collect();
        let details = l.details;
        let error = l.list_error.as_ref().map(|e| {
            t!(
                "loader-list-error",
                path = l.dir.display().to_string(),
                error = e.clone()
            )
        });
        let lines = error.map(|e| {
            crate::win95::dialog::wrap(&e, LIST.w - 40, &|t: &str| text::width(window, t, false))
        });
        let st = l.w.list;
        let focused = self.kit.has_focus(LIST_ID);
        self.kit.focusable(LIST_ID, SCOPE, Some(list_keys()));
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                paint_list(window, cx, b, &rows, details, focused, &s, &sa);
                if let Some(lines) = &lines {
                    for (i, t) in (0_i32..).zip(lines) {
                        let at = paint::at(b.origin, 20, 60 + i * LINE);
                        text::draw(window, cx, at, t, false, hsla(sa.navy_text));
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let inner = list_inner();
        let bar = if details {
            Bar {
                id: "ld.vbar",
                rect: Rect::new(
                    inner.right() - THICKNESS,
                    inner.y + ROW + 2,
                    THICKNESS,
                    inner.h - ROW - 2,
                ),
                vertical: true,
            }
        } else {
            Bar {
                id: "ld.hbar",
                rect: Rect::new(inner.x, inner.bottom() - THICKNESS, inner.w, THICKNESS),
                vertical: false,
            }
        };
        let sb = scroll_bar(
            &self.kit,
            cx,
            bar,
            st,
            lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.w.list)),
            None,
        );
        let slot = Slot::default();
        let s2 = slot.clone();
        let measure = canvas(move |b, _, _| s2.set(b), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let (s_left, s_right) = (slot.clone(), slot);
        place(LIST)
            .id("ld.listbox")
            .child(paint)
            .child(measure)
            .child(sb)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.list_click(&s_left, ev, false);
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |v: &mut Self, ev: &MouseDownEvent, _, cx| {
                    v.list_click(&s_right, ev, true);
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    /// A click in the list: select, open on a double click; the right button
    /// opens the list's popup menu (`Update`, `Auto preview`).
    fn list_click(&mut self, slot: &Slot, ev: &MouseDownEvent, right: bool) {
        let Some((x, y)) = slot.local(ev.position) else {
            return;
        };
        self.kit.focus = Some(LIST_ID.into());
        let Some(l) = self.loader.as_mut() else {
            return;
        };
        l.path_list = false;
        let inner = list_inner();
        let on_bar = if l.details {
            x >= inner.right() - THICKNESS
        } else {
            y >= inner.bottom() - THICKNESS
        };
        if on_bar {
            return;
        }
        let hit = cells(l)
            .into_iter()
            .find(|(r, _)| r.contains(x, y))
            .map(|(_, i)| i);
        if let Some(i) = hit {
            if ev.click_count >= 2 && !right {
                let a = l.activate(i);
                self.loader_action(a);
                return;
            }
            l.select(i);
        }
        if right {
            let auto = l.auto_preview;
            let at = (
                f32::from(ev.position.x) as i32,
                f32::from(ev.position.y) as i32,
            );
            self.open_popup(list_popup(auto), at);
        }
    }

    /// The info panel, the options group and the headerless channel count.
    fn loader_side(&mut self, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(l) = self.loader.as_ref() else {
            return Vec::new();
        };
        let (s, sa) = (scheme(cx), sa_colors(cx));
        let (lines, color) = match l.info_lines() {
            None => (vec![t!("loader-folder-link")], sa.navy_select),
            Some((lines, true)) => (lines, sa.navy_text),
            Some((lines, false)) => (lines, sa.navy_warn),
        };
        let info = Rect::new(622, 28, 162, 92);
        let mut out = vec![navy_panel(info, lines, color, &s, &sa)];
        let (has_file, headerless, use_options) = (l.file.is_some(), l.headerless(), l.use_options);
        let total = l.opts.total.clone();
        if headerless && has_file && use_options {
            let r = Rect::new(626, 94, 96, 22);
            out.push(option_label(r, &t!("loader-in-file"), true, &s, &sa));
            out.push(spin_edit(
                &mut self.kit,
                cx,
                Ctl {
                    id: "ld.total",
                    scope: SCOPE,
                    rect: Rect::new(725, 95, 55, 20),
                    label: "",
                    enabled: true,
                },
                &total,
                &lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.opts.total)),
                Some(Rc::new(|v: &mut Self, _, _, _| {
                    if let Some(l) = v.loader.as_mut() {
                        l.total_changed();
                    }
                })),
            ));
        }
        out.extend(self.loader_options(cx, has_file, use_options));
        out
    }

    /// The options group: `Use options`, `Data start`, `Channels`, `Bits`,
    /// `Freq rate`; navy while used, grey showing the detected values otherwise.
    fn loader_options(
        &mut self,
        cx: &mut Context<'_, Self>,
        has_file: bool,
        use_options: bool,
    ) -> Vec<AnyElement> {
        let (s, sa) = (scheme(cx), sa_colors(cx));
        let l_allowed = self.loader.as_ref().is_some_and(Loader::options_allowed);
        let on = has_file && use_options;
        let group = Rect::new(622, 124, 162, 118);
        let row = |i: i32| Rect::new(group.x, group.y + i * 23, group.w, 21);
        let r0 = row(0);
        let mut out = vec![
            navy_fill(group, &sa),
            option_label(r0, &t!("loader-use-options"), has_file, &s, &sa),
        ];
        out.push(check_box(
            &mut self.kit,
            cx,
            Ctl {
                id: "ld.useopt",
                scope: SCOPE,
                rect: Rect::new(r0.right() - 19, r0.y + 2, 16, 17),
                label: "",
                enabled: l_allowed,
            },
            use_options,
            Rc::new(|v: &mut Self, on, _, _| {
                if let Some(l) = v.loader.as_mut() {
                    l.set_use_options(on);
                }
            }),
        ));
        let labels = [
            t!("loader-data-start"),
            t!("loader-channels"),
            t!("loader-bits"),
            t!("loader-freq-rate"),
        ];
        let look = if on {
            FieldLook {
                bg: sa.navy_bg,
                fg: sa.navy_text,
                border: true,
            }
        } else {
            FieldLook {
                bg: s.face,
                fg: s.text,
                border: true,
            }
        };
        for (k, caption) in (1_i32..).zip(labels) {
            let r = row(k);
            out.push(option_label(
                Rect::new(r.x, r.y, 64, r.h),
                &caption,
                on,
                &s,
                &sa,
            ));
            let field = Rect::new(r.x + 67, r.y, r.w - 67, r.h);
            if let Some(el) = self.option_field(cx, k, field, on, look) {
                out.push(el);
            }
        }
        out
    }

    /// Option field `k` (1 `Data start`, 2 `Channels`, 3 `Bits`, 4 `Freq rate`).
    fn option_field(
        &mut self,
        cx: &mut Context<'_, Self>,
        k: i32,
        field: Rect,
        on: bool,
        look: FieldLook,
    ) -> Option<AnyElement> {
        let l = self.loader.as_ref()?;
        let ctl = |id| Ctl {
            id,
            scope: SCOPE,
            rect: field,
            label: "",
            enabled: on,
        };
        Some(match k {
            1 => {
                let mut st = l.opts.data_start.clone();
                st.look = Some(look);
                let lens = lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.opts.data_start));
                edit(&mut self.kit, cx, ctl("ld.start"), &st, lens, None)
            }
            4 => {
                let mut st = l.opts.rate.clone();
                st.look = Some(look);
                let lens = lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.opts.rate));
                edit(&mut self.kit, cx, ctl("ld.rate"), &st, lens, None)
            }
            2 => {
                let st = l.w.chan.clone();
                let lens = lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.w.chan));
                let changed: crate::win95::button::Changed<Self, usize> =
                    Rc::new(|v: &mut Self, i, _, _| {
                        if let Some(l) = v.loader.as_mut() {
                            l.set_chan(i);
                        }
                    });
                combo_box(&mut self.kit, cx, ctl("ld.chan"), &st, lens, Some(changed))
            }
            _ => {
                let st = l.w.bits.clone();
                let lens = lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.w.bits));
                let changed: crate::win95::button::Changed<Self, usize> =
                    Rc::new(|v: &mut Self, i, _, _| {
                        if let Some(l) = v.loader.as_mut() {
                            l.set_bits(i);
                        }
                    });
                combo_box(&mut self.kit, cx, ctl("ld.bits"), &st, lens, Some(changed))
            }
        })
    }

    /// `Ext. Info`, quick search, the filter, `Ok`, `Cancel`, the preview and
    /// its prescaler, the status bar.
    fn loader_bottom(&mut self, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(l) = self.loader.as_ref() else {
            return Vec::new();
        };
        let (s, sa) = (scheme(cx), sa_colors(cx));
        let mut out = vec![navy_bar(
            Rect::new(0, 248, SIZE.0, 18),
            l.ext_info(),
            &s,
            &sa,
        )];
        let mut search = l.w.search.clone();
        search.look = Some(FieldLook {
            bg: sa.navy_bg,
            fg: sa.navy_text,
            border: true,
        });
        let filter = l.w.filter.clone();
        out.push(edit(
            &mut self.kit,
            cx,
            Ctl {
                id: "ld.search",
                scope: SCOPE,
                rect: Rect::new(0, 272, 516, 22),
                label: "",
                enabled: true,
            },
            &search,
            lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.w.search)),
            Some(action(|v: &mut Self, _, _| {
                if let Some(l) = v.loader.as_mut() {
                    l.search_changed();
                }
            })),
        ));
        out.push(combo_box(
            &mut self.kit,
            cx,
            Ctl {
                id: "ld.filter",
                scope: SCOPE,
                rect: Rect::new(522, 272, 96, 22),
                label: "",
                enabled: true,
            },
            &filter,
            lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.w.filter)),
            Some(Rc::new(|v: &mut Self, i: usize, _, _| {
                if let (Some(l), Some(f)) = (v.loader.as_mut(), ExtFilter::ALL.get(i)) {
                    l.set_filter(*f);
                }
            })),
        ));
        let (ok, cancel) = (t!("loader-ok"), t!("loader-cancel"));
        for (id, caption, x, go) in [("ld.ok", ok, 626, true), ("ld.cancel", cancel, 709, false)] {
            out.push(push_button(
                &mut self.kit,
                cx,
                Ctl {
                    id,
                    scope: SCOPE,
                    rect: Rect::new(x, 271, BUTTON.0, BUTTON.1),
                    label: &caption,
                    enabled: true,
                },
                go,
                action(move |v: &mut Self, _, _| {
                    if go {
                        v.with_loader(Loader::ok);
                    } else {
                        v.loader_action(LoaderAction::Cancel);
                    }
                }),
            ));
        }
        out.extend(self.preview_row(cx));
        out
    }

    /// The preview, its prescaler and the status bar.
    fn preview_row(&mut self, cx: &mut Context<'_, Self>) -> Vec<AnyElement> {
        let Some(l) = self.loader.as_ref() else {
            return Vec::new();
        };
        let (s, sa) = (scheme(cx), sa_colors(cx));
        let preview: Option<Vec<MinMax>> = l.current_preview().map(<[MinMax]>::to_vec);
        let message = l.preview_message();
        let db = if l.use_options { l.prescale_db } else { 0 };
        let gain = 10_f32.powf(db as f32 / 20.0);
        let prescaler = if l.use_options && l.file.is_some() {
            l.w.prescaler
        } else {
            crate::win95::scrollbar::ScrollState::new(0, 0, 1, 0)
        };
        let status = l.status_text();
        let mut out = vec![preview_plot(
            Rect::new(0, 300, SIZE.0 - 18, 168),
            preview,
            gain,
            message,
            &s,
            &sa,
        )];
        out.push(scroll_bar(
            &self.kit,
            cx,
            Bar {
                id: "ld.prescaler",
                rect: Rect::new(SIZE.0 - 16, 300, THICKNESS, 168),
                vertical: true,
            },
            prescaler,
            lens(|v: &mut Self| v.loader.as_mut().map(|l| &mut l.w.prescaler)),
            Some(action(|v: &mut Self, _, _| {
                if let Some(l) = v.loader.as_mut() {
                    l.prescale_db =
                        (PRESCALER_MAX_DB - l.w.prescaler.pos).clamp(0, PRESCALER_MAX_DB);
                }
            })),
        ));
        out.push(sunken_text(Rect::new(0, 474, SIZE.0, 18), status, &s));
        out
    }

    /// `SA - select channel` over the loader.
    fn prompt_view(&mut self, cx: &mut Context<'_, Self>, vw: i32, vh: i32) -> Option<AnyElement> {
        let p = self.loader.as_mut()?.prompt.as_mut()?;
        if !p.placed {
            let (w, h) = dialog_size(PROMPT_SIZE.0, PROMPT_SIZE.1);
            p.rect = Rect::new(((vw - w) / 2).max(0), ((vh - h) / 2).max(0), w, h);
            p.placed = true;
            self.kit.focus = Some("ldp.ok".into());
        }
        let p = *p;
        let mut body = div().relative().size_full().child(group_box(
            cx,
            Rect::new(8, 6, 234, 82),
            &t!("loader-select-channel-group"),
        ));
        let choices = [
            (StereoChoice::First, t!("loader-channel-1")),
            (StereoChoice::Second, t!("loader-channel-2")),
            (StereoChoice::All, t!("loader-all-samples")),
        ];
        for (i, (c, caption)) in (0_i32..).zip(choices) {
            let id = format!("ldp.r{i}");
            body = body.child(radio_button(
                &mut self.kit,
                cx,
                Ctl {
                    id: &id,
                    scope: PROMPT_SCOPE,
                    rect: Rect::new(20, 24 + i * 20, 200, 17),
                    label: &caption,
                    enabled: true,
                },
                p.choice == c,
                action(move |v: &mut Self, _, _| {
                    if let Some(p) = v.loader.as_mut().and_then(|l| l.prompt.as_mut()) {
                        p.choice = c;
                    }
                }),
            ));
        }
        let (ok, brk) = (t!("loader-ok"), t!("loader-break"));
        for (id, caption, x, go) in [("ldp.ok", ok, 8, true), ("ldp.break", brk, 167, false)] {
            body = body.child(push_button(
                &mut self.kit,
                cx,
                Ctl {
                    id,
                    scope: PROMPT_SCOPE,
                    rect: Rect::new(x, 96, BUTTON.0, BUTTON.1),
                    label: &caption,
                    enabled: true,
                },
                go,
                action(move |v: &mut Self, _, _| v.prompt_answer(go)),
            ));
        }
        let hooks = DialogHooks {
            rect: lens(|v: &mut Self| {
                v.loader
                    .as_mut()
                    .and_then(|l| l.prompt.as_mut())
                    .map(|p| &mut p.rect)
            }),
            outline: lens(|v: &mut Self| {
                v.loader
                    .as_mut()
                    .and_then(|l| l.prompt.as_mut())
                    .map(|p| &mut p.outline)
            }),
            on_close: Some(action(|v: &mut Self, _, _| v.prompt_answer(false))),
        };
        Some(modal_dialog(
            &self.kit,
            cx,
            "ldprompt",
            &t!("loader-select-channel-title"),
            p.rect,
            p.outline,
            body.into_any_element(),
            &hooks,
        ))
    }
}

/// The key handler of the file list: arrows move the selection (Left and
/// Right by a column in the list view), Backspace opens the parent folder.
fn list_keys() -> KeyHandler<Frame> {
    Rc::new(|v: &mut Frame, ev, _, cx| {
        let Some(l) = v.loader.as_mut() else {
            return false;
        };
        let page = if l.details {
            0
        } else {
            l.rows_per_column as isize
        };
        match ev.keystroke.key.as_str() {
            "up" => l.step(-1),
            "down" => l.step(1),
            "left" if page > 0 => l.step(-page),
            "right" if page > 0 => l.step(page),
            "home" => l.step(isize::MIN / 2),
            "end" => l.step(isize::MAX / 2),
            "backspace" => l.go_up(),
            _ => return false,
        }
        cx.notify();
        true
    })
}

/// The path combo's popup: `Copy`, `Paste`, `Cancel` (6.2.6.0).
fn path_popup() -> Vec<Item> {
    vec![
        Item::command(&t!("loader-popup-copy"), cmd::LD_COPY),
        Item::command(&t!("loader-popup-paste"), cmd::LD_PASTE),
        Item::command(&t!("loader-popup-cancel"), cmd::LD_CANCEL),
    ]
}

/// The list's popup: `Update`, `Auto preview` (ticked while on).
fn list_popup(auto: bool) -> Vec<Item> {
    vec![
        Item::command(&t!("loader-popup-update"), cmd::LD_UPDATE),
        Item::command(&t!("loader-popup-auto-preview"), cmd::LD_AUTO).check(if auto {
            Check::Tick
        } else {
            Check::None
        }),
    ]
}

/// The combo-style drop button of the path field.
fn drop_button(cx: &mut Context<'_, Frame>, r: Rect) -> AnyElement {
    let s = scheme(cx);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            paint::panel(window, b.origin, Rect::new(0, 0, w, h), Edge::Button, &s);
            let m = marks::arrow(Dir::Down, 7);
            let at = ((w - m.w) / 2, (h - m.h) / 2);
            glyph::draw(window, cx, b.origin, at, &m, Mode::Normal, &s);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r)
        .id("ld.drop")
        .child(paint)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|v: &mut Frame, _: &MouseDownEvent, _, cx| {
                if let Some(l) = v.loader.as_mut() {
                    l.path_list = !l.path_list;
                }
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .into_any_element()
}

/// Paint the list's rows (and the details header).
#[allow(clippy::too_many_arguments)]
fn paint_list(
    window: &mut Window,
    cx: &mut gpui::App,
    b: gpui::Bounds<gpui::Pixels>,
    rows: &[(Rect, Entry, bool)],
    details: bool,
    focused: bool,
    s: &Scheme,
    sa: &SaColors,
) {
    let (w, h) = size_of(b);
    let o = b.origin;
    let all = Rect::new(0, 0, w, h);
    paint::fill(window, o, all, hsla(sa.navy_bg));
    paint::edge(window, o, all, Edge::Sunken, s);
    let inner = list_inner();
    if details {
        paint_header(
            window,
            cx,
            o,
            Rect::new(inner.x, inner.y, inner.w, ROW + 2),
            s,
        );
    }
    for (r, e, selected) in rows {
        let color = if *selected {
            sa.navy_select
        } else {
            sa.navy_text
        };
        if *selected {
            paint::fill(window, o, *r, hsla(sa.navy_select_bg));
            if focused {
                paint::focus_rect(window, o, *r, hsla(sa.navy_text));
            }
        }
        let icon = match e.kind {
            EntryKind::Folder => glyphs::folder_icon(false),
            EntryKind::Link => glyphs::folder_icon(true),
            EntryKind::File => glyphs::file_icon(e.ext() == "wav"),
        };
        glyph::draw(window, cx, o, (r.x + 2, r.y), &icon, Mode::Normal, s);
        let rest: i32 = COLUMNS.iter().sum();
        let name_w = if details { (r.w - rest).max(80) } else { r.w };
        let name = fit(window, &e.name, name_w - 24, true);
        text::draw(
            window,
            cx,
            paint::at(o, r.x + 21, r.y + 2),
            &name,
            true,
            hsla(color),
        );
        if !details {
            continue;
        }
        let (date, time) = e
            .modified
            .map(loader_files::local_date_time)
            .unwrap_or_default();
        let size = if e.kind == EntryKind::File {
            e.size.to_string()
        } else {
            String::new()
        };
        let mut x = r.x + name_w;
        for (t, cw) in [e.type_text(), size, date, time].iter().zip(COLUMNS) {
            let shown = fit(window, t, cw - 4, true);
            let tw = text::width(window, &shown, true);
            let at = paint::at(o, x + (cw - tw) / 2, r.y + 2);
            text::draw(window, cx, at, &shown, true, hsla(color));
            x += cw;
        }
    }
}

/// The details view's column header (raised face cells).
fn paint_header(
    window: &mut Window,
    cx: &mut gpui::App,
    o: gpui::Point<gpui::Pixels>,
    r: Rect,
    s: &Scheme,
) {
    let rest: i32 = COLUMNS.iter().sum();
    let name_w = (r.w - THICKNESS - rest).max(80);
    let titles = [
        (t!("loader-col-name"), name_w),
        (t!("loader-col-type"), COLUMNS[0]),
        (t!("loader-col-size"), COLUMNS[1]),
        (t!("loader-col-date"), COLUMNS[2]),
        (t!("loader-col-time"), COLUMNS[3] + THICKNESS),
    ];
    let mut x = r.x;
    for (i, (title, w)) in titles.iter().enumerate() {
        let cell = Rect::new(x, r.y, *w, r.h);
        paint::panel(window, o, cell, Edge::Button, s);
        let shown = fit(window, title, w - 6, false);
        let tw = text::width(window, &shown, false);
        let tx = if i == 0 {
            cell.x + 6
        } else {
            cell.x + (w - tw) / 2
        };
        text::draw(
            window,
            cx,
            paint::at(o, tx, cell.y + 3),
            &shown,
            false,
            s.of(Role::Text),
        );
        x += w;
    }
}

/// A navy panel with lines of bold text centred from the top (the info panel).
fn navy_panel(r: Rect, lines: Vec<String>, color: u32, s: &Scheme, sa: &SaColors) -> AnyElement {
    let (s, sa) = (*s, *sa);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let all = Rect::new(0, 0, w, h);
            paint::fill(window, b.origin, all, hsla(sa.navy_bg));
            paint::edge(window, b.origin, all, Edge::Sunken, &s);
            for (i, l) in (0_i32..).zip(&lines) {
                let shown = fit(window, l, w - 6, true);
                let tw = text::width(window, &shown, true);
                let at = paint::at(b.origin, (w - tw) / 2, 4 + i * 15);
                text::draw(window, cx, at, &shown, true, hsla(color));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// A plain navy area.
fn navy_fill(r: Rect, sa: &SaColors) -> AnyElement {
    place(r).bg(hsla(sa.navy_bg)).into_any_element()
}

/// A navy bar with one centred bold line (`Ext. Info: ...`).
fn navy_bar(r: Rect, line: String, s: &Scheme, sa: &SaColors) -> AnyElement {
    let lines = if line.is_empty() {
        Vec::new()
    } else {
        vec![line]
    };
    let (s, sa) = (*s, *sa);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let all = Rect::new(0, 0, w, h);
            paint::fill(window, b.origin, all, hsla(sa.navy_bg));
            paint::edge(window, b.origin, all, Edge::Sunken, &s);
            for l in &lines {
                let shown = fit(window, l, w - 8, true);
                let tw = text::width(window, &shown, true);
                let at = paint::at(b.origin, (w - tw) / 2, (h - LINE) / 2);
                text::draw(window, cx, at, &shown, true, hsla(sa.navy_text));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// A label cell of the options group: white bold text in a sunken navy box.
fn option_label(r: Rect, caption: &str, enabled: bool, s: &Scheme, sa: &SaColors) -> AnyElement {
    let (s, sa) = (*s, *sa);
    let caption = caption.to_owned();
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let all = Rect::new(0, 0, w, h);
            paint::fill(window, b.origin, all, hsla(sa.navy_bg));
            paint::edge(window, b.origin, all, Edge::ThinSunken, &s);
            let color = if enabled {
                hsla(sa.navy_text)
            } else {
                s.of(Role::Shadow)
            };
            let shown = fit(window, &caption, w - 26, true);
            text::draw(
                window,
                cx,
                paint::at(b.origin, 4, (h - LINE) / 2),
                &shown,
                true,
                color,
            );
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// A sunken field with one line of text (the loader's status bar).
fn sunken_text(r: Rect, line: String, s: &Scheme) -> AnyElement {
    let s = *s;
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            paint::edge(
                window,
                b.origin,
                Rect::new(0, 0, w, h),
                Edge::ThinSunken,
                &s,
            );
            let shown = fit(window, &line, w - 8, false);
            text::draw(
                window,
                cx,
                paint::at(b.origin, 4, (h - LINE) / 2),
                &shown,
                false,
                s.of(Role::Text),
            );
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

/// The preview: navy, the full-scale limits as red lines, the envelope in
/// yellow after the prescaler (`gain`), clipped at the limits; `message`
/// replaces it.
fn preview_plot(
    r: Rect,
    columns: Option<Vec<MinMax>>,
    gain: f32,
    message: Option<String>,
    s: &Scheme,
    sa: &SaColors,
) -> AnyElement {
    let (s, sa) = (*s, *sa);
    let paint = canvas(
        |_, _, _| (),
        move |b, (), window, cx| {
            let (w, h) = size_of(b);
            let o = b.origin;
            let all = Rect::new(0, 0, w, h);
            paint::fill(window, o, all, hsla(sa.navy_bg));
            paint::edge(window, o, all, Edge::Sunken, &s);
            let mid = h / 2;
            let half = h / 2 - 6;
            if let Some(m) = &message {
                let tw = text::width(window, m, false);
                text::draw(
                    window,
                    cx,
                    paint::at(o, (w - tw) / 2, mid - LINE / 2),
                    m,
                    false,
                    hsla(sa.navy_text),
                );
                return;
            }
            let Some(cols) = &columns else {
                return;
            };
            paint::fill(window, o, Rect::new(2, mid, w - 4, 1), s.of(Role::Shadow));
            let n = cols.len().max(1) as f32;
            let span = (w - 4) as f32;
            for (x, m) in cols.iter().enumerate() {
                if m.is_empty() {
                    continue;
                }
                let lo = (m.min * gain).clamp(-1.0, 1.0);
                let hi = (m.max * gain).clamp(-1.0, 1.0);
                let px = 2 + (x as f32 / n * span) as i32;
                let y1 = mid - (hi * half as f32).round() as i32;
                let y2 = mid - (lo * half as f32).round() as i32;
                paint::fill(
                    window,
                    o,
                    Rect::new(px, y1, 1, (y2 - y1).max(1)),
                    hsla(sa.navy_warn),
                );
            }
            for y in [mid - half, mid + half] {
                paint::fill(window, o, Rect::new(2, y, w - 4, 1), hsla(sa.preview_limit));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    place(r).child(paint).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_geometry() {
        assert_eq!(rows_per_column(), 11);
        assert_eq!(detail_rows(), 11);
        assert_eq!(column_w(), 306);
    }
}
