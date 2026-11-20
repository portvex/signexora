//! The demodulator's popup menus, as `sa-app`'s: the plot's (SigNexora settings
//! SA has no control for: levels at the found tones, the average filter,
//! adaptive levels, Info, reset of the vertical scale, `Cancel`), the text
//! panel's (`Copy`, `Save as...`, `Data as`, `Cancel`), the progress field's
//! (`Stop`) and the cutoff list.

use gpui::{ClipboardItem, Context};
use sn_core::NodeId;
use sn_ui_core::demod::{DataAs, cutoff_list};
use sn_ui_core::{format, t};

use super::Frame;
use crate::app::dialogs::chooser_answer;
use crate::win95::menu::{Check, Item};

/// The demodulator's command ids (`DM_FIRST..=DM_LAST`).
pub(crate) const DM_FIRST: u32 = 1500;
const DM_TONES: u32 = 1500;
const DM_AVERAGE: u32 = 1501;
const DM_ADAPTIVE: u32 = 1502;
const DM_INFO: u32 = 1503;
const DM_RESET_VIEW: u32 = 1504;
const DM_CANCEL: u32 = 1505;
const DM_COPY: u32 = 1506;
const DM_SAVE: u32 = 1507;
const DM_STOP: u32 = 1508;
/// `DM_DATA_AS + i`: `DataAs::ALL[i]`.
const DM_DATA_AS: u32 = 1510;
/// `DM_CUTOFF + i`: item `i` of the cutoff list.
const DM_CUTOFF: u32 = 1530;
pub(crate) const DM_LAST: u32 = 1599;

fn tick(on: bool) -> Check {
    if on { Check::Tick } else { Check::None }
}

impl Frame {
    /// Open a popup menu of demodulator window `win` at `at`.
    fn open_demod_menu(&mut self, win: u32, items: Vec<Item>, at: (i32, i32)) {
        self.open_popup(items, at);
        self.demod.popup_win = Some(win);
    }

    /// The plot's popup menu.
    pub(crate) fn open_demod_plot_menu(&mut self, win: u32, node: NodeId, at: (i32, i32)) {
        let (tones, _) = self.demod_tones(node);
        let st = &self.demod_win(node).st;
        let items = vec![
            Item::command(&t!("demod-menu-tones", count = tones.len()), DM_TONES)
                .enabled(tones.len() >= 2)
                .check(tick(st.from_tones)),
            Item::command(&t!("demod-menu-average"), DM_AVERAGE)
                .enabled(!st.rrc)
                .check(tick(st.average)),
            Item::command(&t!("demod-menu-adaptive"), DM_ADAPTIVE).check(tick(st.adaptive)),
            Item::separator(),
            Item::command(&t!("demod-menu-info"), DM_INFO).check(tick(st.info)),
            Item::command(&t!("demod-menu-reset-view"), DM_RESET_VIEW),
            Item::separator(),
            Item::command(&t!("demod-menu-cancel"), DM_CANCEL),
        ];
        self.open_demod_menu(win, items, at);
    }

    /// The text panel's popup menu.
    pub(crate) fn open_demod_text_menu(&mut self, win: u32, node: NodeId, at: (i32, i32)) {
        let st = &mut self.demod_win(node).st;
        let has = st.data().is_some();
        let form = st.data_as;
        let forms = (0_u32..)
            .zip(DataAs::ALL)
            .map(|(i, f)| {
                let c = if f == form { Check::Dot } else { Check::None };
                Item::command(&f.label(), DM_DATA_AS + i).check(c)
            })
            .collect();
        let items = vec![
            Item::command(&t!("demod-menu-copy"), DM_COPY).enabled(has),
            Item::command(&t!("demod-menu-save"), DM_SAVE).enabled(has),
            Item::separator(),
            Item::submenu(&t!("demod-menu-data-as"), forms),
            Item::separator(),
            Item::command(&t!("demod-menu-cancel"), DM_CANCEL),
        ];
        self.open_demod_menu(win, items, at);
    }

    /// The progress field's popup menu: `Stop`.
    pub(crate) fn open_demod_stop_menu(&mut self, win: u32, node: NodeId, at: (i32, i32)) {
        let running = self.demod_running(node);
        let items = vec![Item::command(&t!("demod-stop"), DM_STOP).enabled(running)];
        self.open_demod_menu(win, items, at);
    }

    /// The cutoff list: Br first, then SA's presets.
    pub(crate) fn open_cutoff_list(&mut self, win: u32, node: NodeId, at: (i32, i32)) {
        let baud = self.demod_win(node).st.baud;
        let items = (0_u32..)
            .zip(cutoff_list(baud))
            .map(|(i, v)| Item::command(&format::dec_trim(v, 2), DM_CUTOFF + i))
            .collect();
        self.open_demod_menu(win, items, at);
    }

    /// A command of a demodulator popup menu.
    pub(crate) fn demod_command(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(win) = self.demod.popup_win.take() else {
            return;
        };
        let Some(node) = self.node_of(win) else {
            return;
        };
        let d = self.demod_win(node);
        let st = &mut d.st;
        match id {
            DM_TONES => st.from_tones = !st.from_tones,
            DM_AVERAGE => st.average = !st.average,
            DM_ADAPTIVE => st.adaptive = !st.adaptive,
            DM_INFO => st.info = !st.info,
            DM_RESET_VIEW => st.view = None,
            DM_STOP => st.stop(),
            DM_COPY => {
                if let Some(text) = st.data() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
                }
            }
            DM_SAVE => self.demod_save(node, cx),
            i if (DM_DATA_AS..DM_DATA_AS + DataAs::ALL.len() as u32).contains(&i) => {
                if let Some(f) = DataAs::ALL.get((i - DM_DATA_AS) as usize) {
                    st.data_as = *f;
                    d.text_bar.pos = 0;
                }
            }
            i if i >= DM_CUTOFF => {
                if let Some(v) = cutoff_list(st.baud).get((i - DM_CUTOFF) as usize) {
                    st.cutoff = format::dec_trim(*v, 2);
                }
            }
            _ => {}
        }
    }

    /// `Save as...`: ask the platform's file chooser, then write the data.
    fn demod_save(&mut self, node: NodeId, cx: &mut Context<'_, Self>) {
        let Some(text) = self.demod_win(node).st.data() else {
            return;
        };
        let dir = self.chooser_dir();
        let name = "demod.txt";
        let path = cx.prompt_for_new_path(&dir, Some(name));
        cx.spawn(async move |this, cx| {
            let message = match chooser_answer(path.await) {
                Ok(None) => return,
                // The file chooser could not be shown: nothing was written.
                Err(e) => t!(
                    "app-chooser-failed",
                    action = t!("sw-popup-save-as"),
                    error = e
                ),
                Ok(Some(p)) => {
                    // Write off the UI thread: a slow drive must not freeze
                    // the frame.
                    let target = p.clone();
                    let written = cx
                        .background_executor()
                        .spawn(async move { std::fs::write(&target, text.as_bytes()) })
                        .await;
                    match written {
                        Ok(()) => return,
                        Err(e) => t!(
                            "demod-save-failed",
                            path = p.display().to_string(),
                            error = e.to_string()
                        ),
                    }
                }
            };
            this.update(cx, |v, cx| {
                v.alert(&message);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
