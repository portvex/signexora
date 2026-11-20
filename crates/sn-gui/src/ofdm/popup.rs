//! The popup menus of `SA-OFDM`: the List of K's (`Sort by SR`, `Sort by K`,
//! `Sort by AM`, SigNexora's `Resample to x Hz` on a line, `Close`, `Cancel`) and
//! the results list's SigNexora items (`Bounds around Br = x`, `Clear the
//! results`, `Cancel`).

use gpui::Context;
use sn_core::{MeasurementKind, NodeId};
use sn_ui_core::ofdm::{KSort, OfdmAction};
use sn_ui_core::{format, t};

use super::Frame;
use crate::win95::menu::{Check, Item};

/// The OFDM module's command ids (`OF_FIRST..=OF_LAST`).
pub(crate) const OF_FIRST: u32 = 1600;
const OF_SORT_SR: u32 = 1600;
const OF_SORT_K: u32 = 1601;
const OF_SORT_AM: u32 = 1602;
const OF_K_CLOSE: u32 = 1603;
const OF_CANCEL: u32 = 1604;
const OF_RESAMPLE: u32 = 1605;
const OF_AROUND: u32 = 1606;
pub(crate) const OF_RESET: u32 = 1607;
pub(crate) const OF_LAST: u32 = 1699;

impl Frame {
    /// The popup menu of the list of window `win` (right click on row `row`).
    pub(crate) fn ofdm_list_menu(
        &mut self,
        win: u32,
        node: NodeId,
        row: Option<usize>,
        at: (i32, i32),
    ) {
        let measured = self.measured(&MeasurementKind::BaudRate).map(|m| m.value);
        let o = self.ofdm_win(node);
        let items = if o.st.k_shown {
            let line = row.and_then(|i| o.st.k_list().get(i).cloned());
            if let Some((rank, _)) = &line {
                o.st.k_selected = Some(*rank);
            }
            o.k_rate = line.map(|(_, c)| c.sample_rate);
            let sort = o.st.k_sort;
            let dot = |s: KSort| if s == sort { Check::Dot } else { Check::None };
            let mut items = vec![
                Item::command(&t!("ofdm-sort-sr"), OF_SORT_SR).check(dot(KSort::Sr)),
                Item::command(&t!("ofdm-sort-k"), OF_SORT_K).check(dot(KSort::K)),
                Item::command(&t!("ofdm-sort-am"), OF_SORT_AM).check(dot(KSort::Am)),
            ];
            if let Some(r) = o.k_rate {
                items.push(Item::separator());
                items.push(Item::command(
                    &t!("ofdm-resample", rate = format::dec(r, 2)),
                    OF_RESAMPLE,
                ));
            }
            items.extend([
                Item::separator(),
                Item::command(&t!("ofdm-k-close"), OF_K_CLOSE),
                Item::separator(),
                Item::command(&t!("ofdm-menu-cancel"), OF_CANCEL),
            ]);
            items
        } else {
            let mut items = Vec::new();
            if let Some(br) = measured {
                items.push(Item::command(
                    &t!("ofdm-menu-around", rate = format::dec(br, 2)),
                    OF_AROUND,
                ));
            }
            items.extend([
                Item::command(&t!("ofdm-menu-reset"), OF_RESET),
                Item::separator(),
                Item::command(&t!("ofdm-menu-cancel"), OF_CANCEL),
            ]);
            items
        };
        self.open_popup(items, at);
        self.ofdm.popup_win = Some(win);
    }

    /// A command of an `SA-OFDM` popup menu.
    pub(crate) fn ofdm_command(&mut self, id: u32, _cx: &mut Context<'_, Self>) {
        let Some(win) = self.ofdm.popup_win.take() else {
            return;
        };
        let Some(node) = self.node_of(win) else {
            return;
        };
        let measured = self.measured(&MeasurementKind::BaudRate).map(|m| m.value);
        let o = self.ofdm_win(node);
        let st = &mut o.st;
        match id {
            OF_SORT_SR => st.k_sort = KSort::Sr,
            OF_SORT_K => st.k_sort = KSort::K,
            OF_SORT_AM => st.k_sort = KSort::Am,
            OF_K_CLOSE => st.k_shown = false,
            OF_RESET => {
                // Stop first: a running Find CT, ADP or List of K would put
                // its result back into the cleared state.
                st.stop();
                st.reset();
            }
            OF_AROUND => {
                if let Some(br) = measured {
                    st.around(br);
                }
            }
            OF_RESAMPLE => {
                if let Some(r) = o.k_rate {
                    self.ofdm_action(win, node, OfdmAction::Resample(r));
                }
            }
            _ => {}
        }
    }
}
