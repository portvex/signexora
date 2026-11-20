//! The WF window's popup menu (`docs/SA_UI_SPEC.md` 7.2, SA 6.2.5, R258/a2):
//! `Save as` (`WaveFile`, `DataFile`, then SigNexora's float WAV and SigMF),
//! `Count freqs` (only with Sum on) and `Cancel`; then the `SigNexora tools`
//! submenu with the tools SA's WF had no place for, as `sa-app`: Play, Show Q,
//! PhasePlane and OFDM, the Measure items and every signal operation. And the
//! VMW popup (SigNexora extras): the raster start, the period from the last dT and
//! the colour map.

use gpui::Context;
use sn_core::{MeasurementKind, SampleKind};
use sn_ui_core::colormap::Colormap;
use sn_ui_core::edits::SaveFormat;
use sn_ui_core::t;
use sn_ui_core::wf_tools::{self, OpItem, OpRun, OpsCtx, WfPopupItem};

use super::Frame;
use crate::app::menus::cmd;
use crate::app::model::WinKind;
use crate::app::sono::icons::{MenuIcon, menu_glyph};
use crate::win95::menu::{Check, Item};

/// The `Save as` submenu's formats, in order (a separator before the third),
/// as the sonogram window's.
pub(crate) const SAVE_FORMATS: [SaveFormat; 4] = [
    SaveFormat::WavPcm16,
    SaveFormat::RawF32,
    SaveFormat::WavFloat,
    SaveFormat::Sigmf,
];

fn dot(on: bool) -> Check {
    if on { Check::Dot } else { Check::None }
}

/// The `Save as` submenu.
fn save_items() -> Vec<Item> {
    let mut sub = Vec::new();
    for (i, f) in (0_u32..).zip(SAVE_FORMATS) {
        if i == 2 {
            sub.push(Item::separator());
        }
        let label = match f {
            SaveFormat::WavPcm16 => t!("sw-popup-wave-file"),
            SaveFormat::RawF32 => t!("sw-popup-data-file"),
            SaveFormat::WavFloat => t!("sw-popup-wave-file-float"),
            SaveFormat::Sigmf => t!("sw-popup-sigmf"),
        };
        sub.push(Item::command(&label, cmd::WF_SAVE + i).glyph(menu_glyph(MenuIcon::SaveAs)));
    }
    sub
}

impl Frame {
    /// Open the popup menu of WF window `win` at window position `at`.
    pub(crate) fn open_wf_popup(&mut self, win: u32, at: (i32, i32)) {
        let items = self.wf_popup_items(win);
        self.open_popup(items, at);
        self.wf.popup_win = Some(win);
    }

    /// The items of WF window `win`'s popup menu.
    pub(crate) fn wf_popup_items(&mut self, win: u32) -> Vec<Item> {
        let Some(ww) = self.wf.wins.get(&win) else {
            return Vec::new();
        };
        let mut items = Vec::new();
        for it in wf_tools::popup_items(ww.sum) {
            items.push(match it {
                WfPopupItem::SaveAs => Item::submenu(&t!("sw-popup-save-as"), save_items())
                    .glyph(menu_glyph(MenuIcon::SaveAs)),
                // Greyed until the H markers frame the tones (`sa-app`).
                WfPopupItem::CountFreqs => {
                    Item::command(&t!("wf-popup-count-freqs"), cmd::WF_COUNT)
                        .glyph(menu_glyph(MenuIcon::Calculator))
                        .enabled(ww.markers.freq_range().is_some())
                }
                WfPopupItem::Cancel => Item::command(&t!("sw-popup-cancel"), cmd::WF_CANCEL)
                    .glyph(menu_glyph(MenuIcon::Cancel)),
            });
        }
        items.push(Item::separator());
        items.push(Item::submenu(&t!("wf-popup-more"), self.wf_extras(win)));
        items
    }

    /// `SigNexora tools`: Play, Show Q (I/Q signals), Open, Measure, Operations.
    fn wf_extras(&mut self, win: u32) -> Vec<Item> {
        let Some(node) = self.node_of(win) else {
            return Vec::new();
        };
        let play = if self.sono.playback.is_playing() {
            t!("app-play-stop")
        } else {
            t!("app-play")
        };
        let mut out = vec![Item::command(&play, cmd::WF_PLAY)];
        let complex = self
            .graph
            .get(node)
            .is_some_and(|n| n.signal.kind() == SampleKind::Complex);
        if complex {
            let on = self.ui.get(&node).is_none_or(|u| u.show_q);
            out.push(
                Item::command(&t!("app-wave-show-q"), cmd::WF_SHOW_Q).check(if on {
                    Check::Tick
                } else {
                    Check::None
                }),
            );
        }
        out.push(Item::submenu(
            &t!("app-open-in"),
            vec![
                Item::command(&WinKind::Phase.label(), cmd::WF_OPEN_PHASE),
                Item::command(&WinKind::Ofdm.label(), cmd::WF_OPEN_OFDM),
            ],
        ));
        out.push(Item::submenu(
            &t!("app-menu-measure"),
            self.wf_measure_items(node),
        ));
        out.push(Item::submenu(
            &t!("wf-popup-operations"),
            self.wf_ops_items(node),
        ));
        out
    }

    /// The Measure items (on the signal's own markers, as `sa-app`).
    fn wf_measure_items(&self, node: sn_core::NodeId) -> Vec<Item> {
        let m = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let fs = self.graph.get(node).map_or(1.0, |n| n.signal.sample_rate());
        let h = m.freq_range().is_some();
        vec![
            Item::command(&t!("app-measure-br"), cmd::WF_MEASURE_BR)
                .shortcut("B")
                .enabled(h),
            Item::command(&t!("app-measure-sc"), cmd::WF_MEASURE_SC)
                .shortcut("C")
                .enabled(h),
            Item::command(&t!("app-measure-adp"), cmd::WF_ADP),
            Item::command(&t!("raster-dt-menu-item"), cmd::WF_MEASURE_DT)
                .enabled(m.readout(fs).d_t.is_some()),
        ]
    }

    /// The Operations items; their actions are kept for the command.
    fn wf_ops_items(&mut self, node: sn_core::NodeId) -> Vec<Item> {
        let Some(signal) = self.graph.get(node).map(|n| n.signal.clone()) else {
            return Vec::new();
        };
        let markers = self.ui.get(&node).map(|u| u.markers).unwrap_or_default();
        let br = self.measured(&MeasurementKind::BaudRate);
        let sc = self.sc_for(node);
        let ops = wf_tools::signal_ops(&OpsCtx {
            signal: &signal,
            markers: &markers,
            br: br.as_ref(),
            sc: sc.as_ref(),
            resynth: self.wf_resynth_ready(node),
        });
        self.wf.ops.clear();
        let mut items = Vec::new();
        for o in ops {
            match o {
                OpItem::Separator => items.push(Item::separator()),
                OpItem::Section(s) => items.push(Item::command(&s, 0).enabled(false)),
                OpItem::Op { label, run } => {
                    let id = cmd::WF_OP + self.wf.ops.len() as u32;
                    // Ids past the operations' range belong to the VMW popup:
                    // an operation there is shown but cannot run.
                    match run.filter(|_| id < cmd::VMW_START_V1) {
                        Some(r) => {
                            self.wf.ops.push(r);
                            items.push(Item::command(&label, id));
                        }
                        None => items.push(Item::command(&label, 0).enabled(false)),
                    }
                }
            }
        }
        items
    }

    /// Open the VMW popup of window `win` at window position `at`.
    pub(crate) fn open_vmw_popup(&mut self, win: u32, at: (i32, i32)) {
        let Some(ww) = self.wf.wins.get(&win) else {
            return;
        };
        let v1 = ww.markers.v.is_some();
        let own = ww.vmw.own_map;
        let measured = self.measures.latest(&MeasurementKind::Period).is_some();
        let mut colours = vec![
            Item::command(&t!("vmw-popup-sa-colours"), cmd::VMW_SA_COLOURS)
                .check(dot(own.is_none())),
        ];
        for (i, c) in (0_u32..).zip(Colormap::ALL) {
            colours
                .push(Item::command(&c.label(), cmd::VMW_COLOURS + i).check(dot(own == Some(c))));
        }
        let items = vec![
            Item::command(&t!("vmw-popup-start-v1"), cmd::VMW_START_V1).enabled(v1),
            Item::command(&t!("vmw-popup-from-dt"), cmd::VMW_FROM_DT).enabled(measured),
            Item::submenu(&t!("vmw-popup-colours"), colours),
        ];
        self.open_popup(items, at);
        self.wf.popup_win = Some(win);
    }

    /// A command of a WF popup menu, the VMW popup or the LPF preset list.
    pub(crate) fn wf_command(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(win) = self.wf.popup_win.take() else {
            return;
        };
        let Some(node) = self.node_of(win) else {
            return;
        };
        match id {
            i if (cmd::WF_SAVE..cmd::WF_SAVE + SAVE_FORMATS.len() as u32).contains(&i) => {
                if let Some(f) = SAVE_FORMATS.get((i - cmd::WF_SAVE) as usize) {
                    self.choose_save(node, *f, cx);
                }
            }
            cmd::WF_COUNT => self.wf_start_count_freqs(win),
            cmd::WF_PLAY => self.toggle_play(win, node),
            cmd::WF_SHOW_Q => {
                if let Some(u) = self.ui.get_mut(&node) {
                    u.show_q = !u.show_q;
                }
            }
            cmd::WF_OPEN_PHASE => {
                self.open_window(node, WinKind::Phase, Some(win));
            }
            cmd::WF_OPEN_OFDM => {
                self.open_window(node, WinKind::Ofdm, Some(win));
            }
            cmd::WF_MEASURE_BR => self.start_measure(node, MeasurementKind::BaudRate),
            cmd::WF_MEASURE_SC => self.start_measure(node, MeasurementKind::Carrier),
            cmd::WF_ADP => self.start_adp(node),
            cmd::WF_MEASURE_DT => self.wf_record_dt(node),
            i if (cmd::WF_LPF_PRESET..cmd::WF_OP).contains(&i) => {
                let presets = Self::wf_lpf_presets();
                if let (Some(p), Some(ww)) = (
                    presets.get((i - cmd::WF_LPF_PRESET) as usize),
                    self.wf.wins.get_mut(&win),
                ) {
                    ww.lpf.set_text(p);
                }
            }
            i if (cmd::WF_OP..cmd::VMW_START_V1).contains(&i) => {
                if let Some(run) = self.wf.ops.get((i - cmd::WF_OP) as usize).cloned() {
                    self.wf_run_op(win, node, run);
                }
            }
            _ => self.vmw_command(win, id),
        }
    }

    /// Run an operation of the `Operations` submenu.
    fn wf_run_op(&mut self, win: u32, node: sn_core::NodeId, run: OpRun) {
        match run {
            OpRun::Edit(e) => self.start_edit(win, e),
            OpRun::Dialog(kind, notes) => self.open_params(win, kind, notes),
            OpRun::Resynth => self.wf_resynth(win, node),
        }
    }

    /// A command of the VMW popup.
    fn vmw_command(&mut self, win: u32, id: u32) {
        let fs = self
            .node_of(win)
            .and_then(|n| self.graph.get(n))
            .map_or(1.0, |n| n.signal.sample_rate());
        let measured = self
            .measures
            .latest(&MeasurementKind::Period)
            .map(|m| m.value * fs);
        let Some(ww) = self.wf.wins.get_mut(&win) else {
            return;
        };
        match id {
            cmd::VMW_START_V1 => {
                if let Some((a, b)) = ww.markers.v {
                    ww.vmw.start = a.min(b).max(0.0);
                }
            }
            cmd::VMW_FROM_DT => {
                if let Some(p) = measured {
                    ww.vmw.period = p.max(1.0);
                    ww.vmw.as_time = false;
                }
            }
            cmd::VMW_SA_COLOURS => ww.vmw.own_map = None,
            i if i >= cmd::VMW_COLOURS => {
                if let Some(c) = Colormap::ALL.get((i - cmd::VMW_COLOURS) as usize) {
                    ww.vmw.own_map = Some(*c);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use sn_core::{Complex32, Signal};
    use sn_ui_core::edits::MeasuredValue;
    use sn_ui_core::markers::Markers;
    use sn_ui_core::wf_tools::{self, OpItem, OpsCtx};

    use crate::app::menus::cmd;

    #[test]
    fn every_operation_fits_its_id_range() {
        let samples = vec![Complex32::new(0.1, 0.2); 4096];
        let signal = Signal::complex(samples, 8000.0).unwrap();
        let markers = Markers {
            v: Some((100.0, 2000.0)),
            h: Some((100.0, 900.0)),
            ..Markers::default()
        };
        let m = MeasuredValue {
            value: 300.0,
            from: "test".into(),
            other_axis: false,
        };
        let ops = wf_tools::signal_ops(&OpsCtx {
            signal: &signal,
            markers: &markers,
            br: Some(&m),
            sc: Some(&m),
            resynth: true,
        });
        let runnable = ops
            .iter()
            .filter(|o| matches!(o, OpItem::Op { run: Some(_), .. }))
            .count();
        assert!(
            runnable < (cmd::VMW_START_V1 - cmd::WF_OP) as usize,
            "{runnable}"
        );
    }
}
