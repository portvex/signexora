//! Text windows: a script's output and the problems met restoring a session,
//! as MDI children with a read-only list (SigNexora additions; `sa-app` shows them
//! as floating windows).

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, canvas, div};

use super::{ChildKind, Frame};
use crate::win95::colors::{Role, scheme};
use crate::win95::geom::{Edge, Rect};
use crate::win95::kit::lens;
use crate::win95::mdi;
use crate::win95::paint;
use crate::win95::scrollbar::{Bar, ScrollState, THICKNESS, scroll_bar};
use crate::win95::text::{self, LINE};
use crate::win95::widget::size_of;

/// Client size of a text window.
const SIZE: (i32, i32) = (520, 300);

/// The lines of a text window and its scroll position (in lines).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextWin {
    pub(crate) lines: Vec<String>,
    pub(crate) scroll: ScrollState,
    /// The script output window (the next script's output replaces it).
    pub(crate) script: bool,
}

impl Frame {
    /// Open a text window titled `title` showing `lines`.
    pub(crate) fn open_text(&mut self, title: &str, lines: Vec<String>) -> u32 {
        let id = self.open_child(
            title,
            SIZE.0,
            SIZE.1,
            true,
            self.mdi.active(),
            ChildKind::Text,
        );
        let n = lines.len() as i32;
        self.texts.insert(
            id,
            TextWin {
                lines,
                scroll: ScrollState::new(0, (n - 1).max(0), 1, 0),
                script: false,
            },
        );
        id
    }

    /// Show a script's output in the script output window: the open one is
    /// reused (`sa-app` keeps one), else a new one opens.
    pub(crate) fn show_script_output(&mut self, title: &str, lines: Vec<String>) -> u32 {
        let open = self
            .texts
            .iter()
            .find(|(id, t)| t.script && self.mdi.get(**id).is_some())
            .map(|(id, _)| *id);
        let id = match open {
            Some(id) => {
                self.mdi.set_title(id, title);
                self.mdi.activate(id);
                if let Some(t) = self.texts.get_mut(&id) {
                    let n = lines.len() as i32;
                    t.lines = lines;
                    t.scroll = ScrollState::new(0, (n - 1).max(0), 1, 0);
                }
                id
            }
            None => self.open_text(title, lines),
        };
        if let Some(t) = self.texts.get_mut(&id) {
            t.script = true;
        }
        id
    }

    /// The client area of text window `win`.
    pub(crate) fn text_body(&mut self, win: u32, cx: &mut Context<'_, Self>) -> AnyElement {
        let (cw, ch) = self.mdi.shown_rect(win).map_or(SIZE, |r| {
            let c = mdi::client_rect(r.w, r.h);
            (c.w, c.h)
        });
        let Some(t) = self.texts.get_mut(&win) else {
            return div().into_any_element();
        };
        let visible = ((ch - 4) / LINE).max(1);
        let pos = t.scroll.pos;
        t.scroll = ScrollState::new(0, (t.lines.len() as i32 - 1).max(0), visible, pos);
        let st = t.scroll;
        let lines: Vec<String> = t
            .lines
            .iter()
            .skip(st.pos.max(0) as usize)
            .take(visible as usize + 1)
            .cloned()
            .collect();
        let s = scheme(cx);
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, cx| {
                let (w, h) = size_of(b);
                let all = Rect::new(0, 0, w, h);
                paint::fill(window, b.origin, all, s.of(Role::Window));
                paint::edge(window, b.origin, all, Edge::Sunken, &s);
                let clip = paint::bounds(b.origin, all.inset(2));
                window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
                    for (i, l) in (0_i32..).zip(&lines) {
                        let at = paint::at(b.origin, 4, 2 + i * LINE);
                        text::draw(window, cx, at, l, false, s.of(Role::Text));
                    }
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let bar = Bar {
            id: &format!("txt{win}"),
            rect: Rect::new(cw - 2 - THICKNESS, 2, THICKNESS, ch - 4),
            vertical: true,
        };
        let sb = scroll_bar(
            &self.kit,
            cx,
            bar,
            st,
            lens(move |v: &mut Self| v.texts.get_mut(&win).map(|t| &mut t.scroll)),
            None,
        );
        div()
            .relative()
            .size_full()
            .child(paint)
            .child(sb)
            .into_any_element()
    }
}
