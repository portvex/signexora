//! The sonogram window's right-click popup menu (`docs/SA_UI_SPEC.md` 4.5, SA
//! 6.2): `Save as` (`WaveFile`, `DataFile`, then SigNexora's float WAV and
//! SigMF), `Wave Form module`, `Measurement of Freq`, `Auto define params`,
//! `Correction of BR`, `Cancel`; with collapsed markers the items that need
//! them are left out. A `(q)` window has the harmonics module's menu. The
//! R/S field's preset list opens as a popup menu too.

use gpui::Context;
use sn_ui_core::edits::SaveFormat;
use sn_ui_core::format;
use sn_ui_core::sono_tools::{self, PopupItem};
use sn_ui_core::t;

use super::Frame;
use super::icons::{MenuIcon, menu_glyph};
use crate::app::menus::cmd;
use crate::app::model::WinKind;
use crate::win95::menu::Item;

/// The `Save as` submenu's formats, in order (a separator before the third).
pub(crate) const SAVE_FORMATS: [SaveFormat; 4] = [
    SaveFormat::WavPcm16,
    SaveFormat::RawF32,
    SaveFormat::WavFloat,
    SaveFormat::Sigmf,
];

/// The menu text of a `Save as` format.
fn format_label(f: SaveFormat) -> String {
    match f {
        SaveFormat::WavPcm16 => t!("sw-popup-wave-file"),
        SaveFormat::RawF32 => t!("sw-popup-data-file"),
        SaveFormat::WavFloat => t!("sw-popup-wave-file-float"),
        SaveFormat::Sigmf => t!("sw-popup-sigmf"),
    }
}

/// The popup menu items for `items` (from [`sono_tools::popup_items`]).
pub(crate) fn menu_items(items: &[PopupItem]) -> Vec<Item> {
    items
        .iter()
        .map(|it| match it {
            PopupItem::Separator => Item::separator(),
            PopupItem::SaveAs => {
                let mut sub = Vec::new();
                for (i, f) in (0_u32..).zip(SAVE_FORMATS) {
                    if i == 2 {
                        sub.push(Item::separator());
                    }
                    sub.push(
                        Item::command(&format_label(f), cmd::SW_SAVE + i)
                            .glyph(menu_glyph(MenuIcon::SaveAs)),
                    );
                }
                Item::submenu(&t!("sw-popup-save-as"), sub).glyph(menu_glyph(MenuIcon::SaveAs))
            }
            PopupItem::WaveForm => Item::command(&t!("sw-popup-waveform"), cmd::SW_WAVEFORM)
                .glyph(menu_glyph(MenuIcon::WaveForm)),
            PopupItem::MeasureFreq => Item::command(&t!("sw-popup-measure"), cmd::SW_MEASURE)
                .glyph(menu_glyph(MenuIcon::Measure)),
            PopupItem::Adp => {
                Item::command(&t!("sw-popup-adp"), cmd::SW_ADP).glyph(menu_glyph(MenuIcon::Adp))
            }
            PopupItem::CorrectBr => Item::command(&t!("sw-popup-brc"), cmd::SW_BRC)
                .glyph(menu_glyph(MenuIcon::Calculator)),
            PopupItem::Cancel => Item::command(&t!("sw-popup-cancel"), cmd::SW_CANCEL)
                .glyph(menu_glyph(MenuIcon::Cancel)),
        })
        .collect()
}

impl Frame {
    /// The popup menu items of window `win` for its markers.
    pub(crate) fn sono_popup_items(&self, win: u32) -> Vec<PopupItem> {
        let Some(node) = self.node_of(win) else {
            return Vec::new();
        };
        let m = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let (v, h) = (m.time_range().is_some(), m.freq_range().is_some());
        if self.is_harmonics(node) {
            sono_tools::harmonics_popup_items(v, h)
        } else {
            sono_tools::popup_items(v, h)
        }
    }

    /// Open the popup menu of window `win` at window position `at`.
    pub(crate) fn open_sono_popup(&mut self, win: u32, at: (i32, i32)) {
        let items = menu_items(&self.sono_popup_items(win));
        self.open_popup(items, at);
        self.sono.popup_win = Some(win);
    }

    /// Open the R/S field's preset list of window `win` at window position `at`.
    pub(crate) fn open_presets(&mut self, win: u32, at: (i32, i32)) {
        let Some(fs) = self
            .node_of(win)
            .and_then(|n| self.graph.get(n))
            .map(|n| n.signal.sample_rate())
        else {
            return;
        };
        let items = (0_u32..)
            .zip(sono_tools::value_presets(fs))
            .map(|(i, p)| Item::command(&format::dec_trim(p, 2), cmd::SW_PRESET + i))
            .collect();
        self.open_popup(items, at);
        self.sono.popup_win = Some(win);
    }

    /// A command of a sonogram window's popup menu or preset list.
    pub(crate) fn sono_command(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(win) = self.sono.popup_win.take() else {
            return;
        };
        let Some(node) = self.node_of(win) else {
            return;
        };
        match id {
            i if (cmd::SW_SAVE..cmd::SW_SAVE + SAVE_FORMATS.len() as u32).contains(&i) => {
                if let Some(f) = SAVE_FORMATS.get((i - cmd::SW_SAVE) as usize) {
                    self.choose_save(node, *f, cx);
                }
            }
            cmd::SW_WAVEFORM => {
                self.open_window(node, WinKind::Waveform, Some(win));
            }
            cmd::SW_MEASURE => self.measure_freq(node),
            cmd::SW_ADP => self.start_adp(node),
            cmd::SW_BRC => self.open_brc(win),
            i if i >= cmd::SW_PRESET => {
                let fs = self.graph.get(node).map_or(0.0, |n| n.signal.sample_rate());
                let presets = sono_tools::value_presets(fs);
                if let (Some(p), Some(sw)) = (
                    presets.get((i - cmd::SW_PRESET) as usize),
                    self.sono.wins.get_mut(&win),
                ) {
                    sw.value.set_text(&format::dec_trim(*p, 2));
                }
            }
            _ => {}
        }
    }
}
