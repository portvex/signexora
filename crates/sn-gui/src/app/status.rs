//! The frame's status bar: SA's `SamplingRate = 8000` of the window under the
//! pointer or the active one, the full path of its original source file (SA
//! 6.0.7 and later), SigNexora's last message, and, while work not tied to a
//! window runs (loading, saving, a session or script), its progress and Stop.

use gpui::{AnyElement, Context, IntoElement, Styled};
use sn_ui_core::{format, t};

use super::Frame;
use crate::win95::bars::{STATUS_H, TOOL, Tool, status_bar, tool_button};
use crate::win95::geom::Rect;
use crate::win95::kit::action;
use crate::win95::progress::{Progress, abort_glyph, progress_field};

/// Width of SA's `SamplingRate = ...` panel.
pub(crate) const RATE_W: i32 = 150;
/// Width of the pointer readout panel (time, frequency and level under the
/// pointer in a plot), as `sa-app`'s: wide enough for an RF-centred readout.
pub(crate) const READOUT_W: i32 = 260;
/// Width of the progress field of frame-level work.
const WORK_W: i32 = 120;

/// The rate as SA's status bar shows it: `SamplingRate = 8000`.
pub(crate) fn rate_text(rate: Option<f64>) -> String {
    let value = rate.map_or_else(String::new, |r| format::dec_trim(r, 3));
    t!("status-sampling-rate", rate = value)
}

impl Frame {
    /// The status bar's panel texts: rate, source path, message.
    pub(crate) fn status_texts(&mut self) -> [String; 3] {
        self.update_source();
        let (rate, path) = self
            .last_source
            .clone()
            .map_or((None, String::new()), |(r, p)| (Some(r), p));
        let message = if let Some(hint) = self.menu_prompt() {
            hint
        } else if self.status.is_empty() {
            t!("app-ready")
        } else {
            self.status.clone()
        };
        [rate_text(rate), path, message]
    }

    /// The prompt of the highlighted menu item (Windows 95 shows it in the
    /// status bar while a menu is open).
    fn menu_prompt(&self) -> Option<String> {
        use crate::win95::menu::Source;
        let item = if let Some(p) = self.popup.as_ref() {
            self.popup_track.hot_item(Source::Popup(&p.items))
        } else {
            self.track.hot_item(Source::Bar(&self.menus))
        };
        item.map(|i| i.hint.clone()).filter(|h| !h.is_empty())
    }

    /// The status bar along the bottom of a `vw` x `vh` frame.
    pub(crate) fn status_bar(
        &mut self,
        cx: &mut Context<'_, Self>,
        vw: i32,
        vh: i32,
    ) -> Vec<AnyElement> {
        let [rate, path, message] = self.status_texts();
        let work = self
            .frame_job()
            .and_then(|i| self.jobs.get(i))
            .map(|j| (j.progress(self.tick), j.handle.name().to_owned()));
        let work_w = if work.is_some() { WORK_W + TOOL + 8 } else { 0 };
        let bar_w = (vw - work_w).max(0);
        let path_w = ((bar_w - RATE_W - READOUT_W) / 2).max(80);
        // Always there (empty off the plots), so the panels after it stay put.
        let readout = self
            .sono_readout()
            .or_else(|| self.wf_readout())
            .unwrap_or_default();
        let panels = [
            (rate, RATE_W),
            (readout, READOUT_W),
            (path, path_w),
            (message, 0),
        ];
        let y = vh - STATUS_H;
        let mut out = vec![status_bar(
            cx,
            Rect::new(0, y, bar_w, STATUS_H),
            &panels,
            work.is_none(),
        )];
        if let Some((p, name)) = work {
            let face = crate::win95::colors::scheme(cx).of(crate::win95::colors::Role::Face);
            let back = crate::win95::widget::place(Rect::new(bar_w, y, work_w, STATUS_H)).bg(face);
            out.push(back.into_any_element());
            out.push(progress_field(
                cx,
                Rect::new(bar_w + 2, y + 3, WORK_W, STATUS_H - 5),
                p,
            ));
            let glyph = abort_glyph();
            out.push(tool_button(
                &self.kit,
                cx,
                Tool {
                    id: "status.stop",
                    at: (bar_w + WORK_W + 6, y - 1),
                    glyph: &glyph,
                    enabled: p != Progress::Idle,
                    checked: false,
                    tip: &name,
                },
                action(|v: &mut Self, _, _| {
                    if let Some(j) = v.frame_job().and_then(|i| v.jobs.get_mut(i)) {
                        j.cancel();
                    }
                }),
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_reads_like_sa() {
        assert_eq!(rate_text(Some(8000.0)), "SamplingRate = 8000");
        assert_eq!(rate_text(None), "SamplingRate = ");
    }
}
