//! The tool windows of the frame, MDI children on the Windows 95 kit:
//!
//! - File > Signal generator (OCG) (`gen_view.rs`, `generator.rs`) and its
//!   calculators (`calc.rs`), as `sa-app`'s `generator.rs`;
//! - File > Demo...: the workflow guide (`guide.rs`, `sa-app`'s `guide.rs`);
//! - Help > Use help...: the keyboard shortcuts (`help.rs`, `sa-app`'s
//!   `shortcuts::window`).
//!
//! Each is a single window built as a scrolling form (`form.rs`) whose
//! controls report their changes here. The module is a child of `app`
//! (declared there with a path), so it extends [`Frame`] directly.

pub(crate) mod calc;
pub(crate) mod fields;
pub(crate) mod form;
mod gen_view;
mod guide;
mod help;
mod preview;
mod table;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};

use gpui::{AnyElement, Context, Window};
use sn_core::NodeId;
use sn_core::job;
use sn_synth::{
    C4fmPulse, Constellation, FskShaping, GeneratorSpec, Length, NoiseLevel, OfdmModel,
    PRESET_NAMES, PilotPattern, PilotReference, Shaping, SignalKind,
};
use sn_ui_core::edits::SaveFormat;
use sn_ui_core::generator::{OFDM_MODULATIONS, kind_label, new_seed};
use sn_ui_core::jobs::synth_to_job;
use sn_ui_core::t;

pub(crate) use guide::GuideAction;

use super::model::{AppJob, JobKind, Output};
use super::{ChildKind, Frame};
use crate::win95::combo::ComboState;
use crate::win95::edit::EditState;
use crate::win95::mdi;
use crate::win95::menu::Item;
use crate::win95::scrollbar::ScrollState;
use calc::CalcInputs;
use fields::Num;
use sn_ui_core::generator::{GenState, ListTexts, pick_noise, set_complex};
use table::TableState;

/// The popup menus of the generator: presets and the save formats.
pub(crate) const OC_FIRST: u32 = 1700;
/// `Preset` > `Defaults`.
const OC_DEFAULTS: u32 = 1700;
/// `OC_PRESET + i`: preset `PRESET_NAMES[i]`.
pub(crate) const OC_PRESET: u32 = 1701;
/// `OC_SAVE + i`: `Generate and save as` in `SaveFormat::ALL[i]`.
pub(crate) const OC_SAVE: u32 = 1730;
pub(crate) const OC_LAST: u32 = 1749;

/// A tool window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Tool {
    Generator,
    Calculators,
    Guide,
    Help,
}

impl Tool {
    /// The window's title.
    fn title(self) -> String {
        match self {
            Self::Generator => t!("gen-title"),
            Self::Calculators => t!("gui-gen-calc-title"),
            Self::Guide => t!("guide-title"),
            Self::Help => t!("keys-title"),
        }
    }

    /// Client size of a new window.
    fn size(self) -> (i32, i32) {
        match self {
            Self::Generator => (580, 500),
            Self::Calculators => (540, 460),
            Self::Guide => (480, 440),
            Self::Help => (560, 440),
        }
    }
}

/// A choice made with a combo box or a group of radio buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Pick {
    Kind,
    Constellation,
    Shaping,
    Noise,
    Modulation,
    Length,
    FskShaping,
    C4fmPulse,
    Model,
    Pilots,
    Reference,
    Preview,
    KFrom,
}

/// A check box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Toggle {
    Complex,
    Differential,
    Shaped,
    Clip,
    Bandwidth,
}

/// A text field that is not a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TextField {
    Label,
    Unused,
    Overrides,
    Pilots,
}

/// A list box of the calculators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TableId {
    Candidates,
    KList,
}

/// A push button of a tool window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Btn {
    Preset,
    NewSeed,
    AddTone,
    RemoveTone(usize),
    AddTap,
    RemoveTap(usize),
    Msk,
    Gmsk,
    Generate,
    GenerateSave,
    Calculators,
    UseCp,
    UseInverse,
    UseCandidate,
    UseK,
    Guide(GuideAction),
    OpenGuide,
}

/// An open tool window.
#[derive(Clone, Debug, Default)]
pub(crate) struct ToolWin {
    pub(crate) id: u32,
    pub(crate) scroll: ScrollState,
    /// The wheel's part unit over the form.
    pub(crate) wheel: crate::win95::scrollbar::Wheel,
    /// The focused control at the last render (a newly focused control is
    /// scrolled into view).
    pub(crate) last_focus: Option<String>,
}

/// The tool windows' state.
#[derive(Debug)]
pub(crate) struct ToolsHub {
    pub(crate) wins: HashMap<Tool, ToolWin>,
    pub(crate) ocg: GenState,
    pub(crate) calc: CalcInputs,
    /// The number fields' texts.
    pub(crate) nums: HashMap<Num, EditState>,
    /// Number fields whose text is not a valid value, with the message.
    pub(crate) errors: BTreeMap<Num, String>,
    pub(crate) texts: HashMap<TextField, EditState>,
    pub(crate) combos: HashMap<Pick, ComboState>,
    pub(crate) tables: HashMap<TableId, TableState>,
    /// A generated signal to save once its window is open.
    pub(crate) save_after: Option<(NodeId, SaveFormat)>,
    /// The buttons with a mnemonic at the last render: window, letter, button
    /// (Alt+letter presses them when no menu title has the letter).
    pub(crate) keys: Vec<(u32, char, Btn)>,
}

impl Default for ToolsHub {
    fn default() -> Self {
        Self::new(GeneratorSpec::default_for(SignalKind::Linear))
    }
}

impl ToolsHub {
    /// The generator starts with `spec` (remembered from the last run).
    pub(crate) fn new(spec: GeneratorSpec) -> Self {
        let mut hub = Self {
            wins: HashMap::new(),
            ocg: GenState::new(spec),
            calc: CalcInputs::default(),
            nums: HashMap::new(),
            errors: BTreeMap::new(),
            texts: HashMap::new(),
            combos: HashMap::new(),
            tables: HashMap::new(),
            save_after: None,
            keys: Vec::new(),
        };
        hub.sync();
        hub
    }

    /// Show the spec in every field again (after it changed as a whole).
    pub(crate) fn sync(&mut self) {
        self.nums.clear();
        self.errors.clear();
        let lists = self.ocg.list_texts();
        let mut label = EditState::new(&self.ocg.spec.common().label);
        label.max_chars = Some(sn_synth::spec::MAX_LABEL);
        for (f, text) in [
            (TextField::Unused, lists.unused),
            (TextField::Overrides, lists.overrides),
            (TextField::Pilots, lists.pilots),
        ] {
            self.texts.insert(f, EditState::new(&text));
        }
        self.texts.insert(TextField::Label, label);
    }

    /// Show the spec again after a local change (a radio button, a check box,
    /// a kind's button): the fields the spec now gives are rebuilt, but text
    /// the user typed that was refused stays, with its message, and so do
    /// the list texts while they are refused.
    pub(crate) fn refresh(&mut self) {
        let errors = &self.errors;
        self.nums.retain(|n, _| errors.contains_key(n));
        let current = self.list_texts();
        if self.ocg.apply_texts(&current).is_err() {
            return;
        }
        let lists = self.ocg.list_texts();
        for (f, text) in [
            (TextField::Unused, lists.unused),
            (TextField::Overrides, lists.overrides),
            (TextField::Pilots, lists.pilots),
        ] {
            if self.texts.get(&f).is_none_or(|e| e.text != text) {
                self.texts.insert(f, EditState::new(&text));
            }
        }
    }

    /// The edit state of a number field, made from the spec when new.
    pub(crate) fn num_state(&mut self, n: Num) -> EditState {
        if let Some(s) = self.nums.get(&n) {
            return s.clone();
        }
        let text = n.text(&self.ocg.spec, &self.calc).unwrap_or_default();
        let st = EditState::new(&text);
        self.nums.insert(n, st.clone());
        st
    }

    /// The texts of the OFDM list fields.
    fn list_texts(&self) -> ListTexts {
        let get = |f| {
            self.texts
                .get(&f)
                .map(|e: &EditState| e.text.clone())
                .unwrap_or_default()
        };
        ListTexts {
            unused: get(TextField::Unused),
            overrides: get(TextField::Overrides),
            pilots: get(TextField::Pilots),
        }
    }

    /// The first problem of the generator's parameters: a number field, a
    /// list field, or the generator's own check. The calculators' fields are
    /// not the generator's: their errors show in the calculators window.
    pub(crate) fn problem(&mut self) -> Option<String> {
        if let Some((_, e)) = self.errors.iter().find(|(n, _)| !matches!(n, Num::Calc(_))) {
            return Some(e.clone());
        }
        let texts = self.list_texts();
        if let Err(e) = self.ocg.apply_texts(&texts) {
            return Some(e);
        }
        self.ocg
            .spec
            .plan()
            .err()
            .map(|e| t!("gen-invalid", error = e.to_string()))
    }

    /// The items of a combo box and the selected one.
    fn combo_items(p: Pick) -> Vec<String> {
        match p {
            Pick::Kind => SignalKind::ALL.iter().map(|k| kind_label(*k)).collect(),
            Pick::Constellation => Constellation::ALL
                .iter()
                .map(|c| c.name().to_owned())
                .collect(),
            Pick::Shaping => vec![t!("gen-lin-rect"), t!("gen-lin-rc"), t!("gen-lin-rrc")],
            Pick::Noise => vec![
                t!("gen-ch-noise-none"),
                t!("gen-ch-noise-full"),
                t!("gen-ch-noise-inband"),
                t!("gen-ch-noise-esn0"),
            ],
            Pick::Modulation => OFDM_MODULATIONS
                .iter()
                .map(|m| m.name().to_owned())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The combo box state of `p` with its current items and selection.
    pub(crate) fn combo_state(&mut self, p: Pick) -> ComboState {
        let items = Self::combo_items(p);
        let selected = Some(self.pick_index(p));
        let st = self
            .combos
            .entry(p)
            .or_insert_with(|| ComboState::new(&[], None));
        if st.items == items {
            st.selected = selected.filter(|i| *i < items.len());
        } else {
            let refs: Vec<&str> = items.iter().map(String::as_str).collect();
            *st = ComboState::new(&refs, selected);
        }
        st.clone()
    }

    /// Which item of `p` is chosen.
    pub(crate) fn pick_index(&self, p: Pick) -> usize {
        let spec = &self.ocg.spec;
        match (p, spec) {
            (Pick::Kind, _) => SignalKind::ALL
                .iter()
                .position(|k| *k == spec.kind())
                .unwrap_or(0),
            (Pick::Constellation, GeneratorSpec::Linear(s)) => Constellation::ALL
                .iter()
                .position(|c| *c == s.constellation)
                .unwrap_or(0),
            (Pick::Shaping, GeneratorSpec::Linear(s)) => match s.shaping {
                Shaping::Rectangular => 0,
                Shaping::RaisedCosine { .. } => 1,
                Shaping::RootRaisedCosine { .. } => 2,
            },
            (Pick::Noise, _) => match spec.common().channel.noise {
                None => 0,
                Some(NoiseLevel::SnrFullBand { .. }) => 1,
                Some(NoiseLevel::Snr { .. }) => 2,
                Some(NoiseLevel::EsN0 { .. }) => 3,
            },
            (Pick::Modulation, GeneratorSpec::Ofdm(o)) => OFDM_MODULATIONS
                .iter()
                .position(|m| *m == o.modulation)
                .unwrap_or(0),
            (Pick::Length, _) => usize::from(matches!(spec.common().length, Length::Symbols(_))),
            (Pick::FskShaping, GeneratorSpec::Fsk(s)) => {
                usize::from(matches!(s.shaping, FskShaping::Gaussian { .. }))
            }
            (Pick::C4fmPulse, GeneratorSpec::C4fm(s)) => usize::from(s.pulse == C4fmPulse::Apco25),
            (Pick::Model, GeneratorSpec::Ofdm(o)) => match o.model {
                OfdmModel::A => 0,
                OfdmModel::B => 1,
                OfdmModel::C => 2,
            },
            (Pick::Pilots, GeneratorSpec::Ofdm(o)) => match o.pilots {
                PilotPattern::None => 0,
                PilotPattern::Positions(_) => 1,
                PilotPattern::Every { .. } => 2,
            },
            (Pick::Reference, GeneratorSpec::Ofdm(o)) => {
                usize::from(o.pilot_reference == PilotReference::Differential)
            }
            (Pick::Preview, _) => usize::from(self.ocg.show_wave),
            (Pick::KFrom, _) => usize::from(!self.calc.k_from_rates),
            _ => 0,
        }
    }

    /// Whether a check box is ticked.
    pub(crate) fn toggled(&self, t: Toggle) -> bool {
        let spec = &self.ocg.spec;
        let ch = &spec.common().channel;
        match (t, spec) {
            (Toggle::Complex, _) => sn_ui_core::generator::spec_is_complex(spec),
            (Toggle::Differential, GeneratorSpec::Linear(s)) => s.differential,
            (Toggle::Shaped, GeneratorSpec::C4fm(s)) => s.shaped,
            (Toggle::Clip, _) => ch.clip.is_some(),
            (Toggle::Bandwidth, _) => matches!(
                ch.noise,
                Some(NoiseLevel::Snr {
                    bandwidth: Some(_),
                    ..
                })
            ),
            _ => false,
        }
    }

    /// Choose item `i` of `p`.
    fn pick(&mut self, p: Pick, i: usize) {
        let rate = self.ocg.spec.plan().ok().and_then(|p| p.symbol_rate);
        match p {
            Pick::Kind => {
                if let Some(k) = SignalKind::ALL.get(i) {
                    self.ocg.switch_kind(*k);
                }
                // A new kind: every field shows the new spec.
                self.sync();
                return;
            }
            // Neither changes the spec: the fields keep what was typed.
            Pick::Preview => {
                self.ocg.show_wave = i == 1;
                return;
            }
            Pick::KFrom => {
                self.calc.k_from_rates = i == 0;
                return;
            }
            Pick::Length => {
                let c = self.ocg.spec.common_mut();
                c.length = match (i, c.length, rate) {
                    (0, Length::Symbols(n), Some(r)) if r > 0.0 => Length::Seconds(n as f64 / r),
                    (0, Length::Symbols(_), _) => Length::Seconds(10.0),
                    (1, Length::Seconds(s), Some(r)) => {
                        Length::Symbols((s * r).floor().max(1.0) as usize)
                    }
                    (1, Length::Seconds(_), None) => Length::Symbols(1000),
                    (_, l, _) => l,
                };
            }
            Pick::Noise => pick_noise(&mut self.ocg.spec.common_mut().channel.noise, i),
            _ => pick_kind_value(&mut self.ocg.spec, p, i),
        }
        self.refresh();
    }

    /// Tick or clear a check box.
    fn toggle(&mut self, t: Toggle, on: bool) {
        let spec = &mut self.ocg.spec;
        match (t, &mut *spec) {
            (Toggle::Complex, s) => set_complex(s, on),
            (Toggle::Differential, GeneratorSpec::Linear(s)) => s.differential = on,
            (Toggle::Shaped, GeneratorSpec::C4fm(s)) => s.shaped = on,
            (Toggle::Clip, s) => s.common_mut().channel.clip = on.then_some(1.0),
            (Toggle::Bandwidth, s) => {
                if let Some(NoiseLevel::Snr { bandwidth, .. }) = &mut s.common_mut().channel.noise {
                    *bandwidth = on.then_some(2400.0);
                }
            }
            _ => {}
        }
        self.refresh();
    }
}

/// The choices of the kinds' own editors.
fn pick_kind_value(spec: &mut GeneratorSpec, p: Pick, i: usize) {
    match (p, spec) {
        (Pick::Constellation, GeneratorSpec::Linear(s)) => {
            if let Some(c) = Constellation::ALL.get(i) {
                s.constellation = *c;
            }
            // QAM carries amplitude too, so it is never differential.
            if !s.constellation.is_psk() {
                s.differential = false;
            }
        }
        (Pick::Shaping, GeneratorSpec::Linear(s)) => {
            let alpha = s.shaping.alpha().unwrap_or(0.35);
            s.shaping = match i {
                1 => Shaping::RaisedCosine { alpha },
                2 => Shaping::RootRaisedCosine { alpha },
                _ => Shaping::Rectangular,
            };
        }
        (Pick::Modulation, GeneratorSpec::Ofdm(o)) => {
            if let Some(m) = OFDM_MODULATIONS.get(i) {
                o.modulation = *m;
            }
        }
        (Pick::FskShaping, GeneratorSpec::Fsk(s)) => {
            let gaussian = matches!(s.shaping, FskShaping::Gaussian { .. });
            if i == 0 {
                s.shaping = FskShaping::None;
            } else if !gaussian {
                s.shaping = FskShaping::Gaussian { bt: 0.3 };
            }
        }
        (Pick::C4fmPulse, GeneratorSpec::C4fm(s)) => {
            s.pulse = if i == 1 {
                C4fmPulse::Apco25
            } else {
                C4fmPulse::RaisedCosine
            };
        }
        (Pick::Model, GeneratorSpec::Ofdm(o)) => {
            // A bad index keeps the model, as the other picks do.
            if let Some(m) = [OfdmModel::A, OfdmModel::B, OfdmModel::C].get(i) {
                o.model = *m;
            }
        }
        (Pick::Pilots, GeneratorSpec::Ofdm(o)) => {
            let kind = match o.pilots {
                PilotPattern::None => 0,
                PilotPattern::Positions(_) => 1,
                PilotPattern::Every { .. } => 2,
            };
            if kind != i {
                o.pilots = match i {
                    1 => PilotPattern::Positions(Vec::new()),
                    2 => PilotPattern::Every { step: 4, offset: 0 },
                    _ => PilotPattern::None,
                };
            }
        }
        (Pick::Reference, GeneratorSpec::Ofdm(o)) => {
            o.pilot_reference = if i == 1 {
                PilotReference::Differential
            } else {
                PilotReference::Constant
            };
        }
        _ => {}
    }
}

impl Frame {
    /// Open tool window `tool`, or bring it to the front.
    pub(crate) fn open_tool(&mut self, tool: Tool) -> u32 {
        if let Some(w) = self.tools.wins.get(&tool) {
            let id = w.id;
            self.mdi.activate(id);
            self.kit.focus = None;
            return id;
        }
        let (w, h) = tool.size();
        let id = self.open_child(
            &tool.title(),
            w,
            h,
            true,
            self.mdi.active(),
            ChildKind::Tool(tool),
        );
        self.tools.wins.insert(
            tool,
            ToolWin {
                id,
                ..ToolWin::default()
            },
        );
        id
    }

    /// The keyboard's show-or-hide: the active tool window closes, another
    /// one comes to the front.
    pub(crate) fn toggle_tool(&mut self, tool: Tool) {
        match self.tools.wins.get(&tool).map(|w| w.id) {
            Some(id) if self.mdi.active() == Some(id) => self.close_child(id),
            _ => {
                self.open_tool(tool);
            }
        }
    }

    /// Tool window `id` was closed.
    pub(crate) fn tool_closed(&mut self, id: u32) {
        let gone: Vec<Tool> = self
            .tools
            .wins
            .iter()
            .filter(|(_, w)| w.id == id)
            .map(|(t, _)| *t)
            .collect();
        for t in gone {
            self.tools.wins.remove(&t);
            if t == Tool::Generator {
                self.tools.ocg.stop_preview();
                self.remember_generator();
            }
        }
    }

    /// Every window closed (`CloseAllWindows`).
    pub(crate) fn tools_all_closed(&mut self) {
        let ids: Vec<u32> = self.tools.wins.values().map(|w| w.id).collect();
        for id in ids {
            self.tool_closed(id);
        }
    }

    /// Keep the generator's spec for the next run.
    pub(crate) fn remember_generator(&mut self) {
        self.stored.generator = sn_ui_core::prefs::generator_to_json(&self.tools.ocg.spec)
            .and_then(|t| serde_json::from_str(&t).ok());
    }

    /// The window of `tool`, if open.
    pub(crate) fn tool_window(&self, tool: Tool) -> Option<u32> {
        self.tools.wins.get(&tool).map(|w| w.id)
    }

    /// Background work of the tool windows (the preview).
    pub(crate) fn tools_busy(&self) -> bool {
        self.tools.ocg.preview_busy() || self.tools.save_after.is_some()
    }

    /// One poll step of the tool windows: the preview, a save after Generate.
    pub(crate) fn tools_poll(&mut self, cx: &mut Context<'_, Self>) {
        if self.tools.wins.contains_key(&Tool::Generator) {
            let ok = self.tools.problem().is_none();
            let plan = self.tools.ocg.spec.plan().ok().filter(|_| ok);
            self.tools.ocg.poll_preview(plan.as_ref());
        } else {
            self.tools.ocg.stop_preview();
        }
        if let Some((node, format)) = self.tools.save_after.take() {
            self.choose_save(node, format, cx);
        }
    }

    /// Alt+`c` in the active window: press the tool button with that
    /// mnemonic. Returns whether there was one.
    pub(crate) fn tool_mnemonic(&mut self, c: char, cx: &mut Context<'_, Self>) -> bool {
        let Some(win) = self.mdi.active() else {
            return false;
        };
        let c = c.to_ascii_lowercase();
        let hit = self
            .tools
            .keys
            .iter()
            .find(|(w, k, _)| *w == win && *k == c)
            .map(|(_, _, b)| *b);
        match hit {
            Some(b) => {
                self.tool_button(b, None, cx);
                true
            }
            None => false,
        }
    }

    /// The tool shown in window `win`, if it is a tool window.
    pub(crate) fn tool_of(&self, win: u32) -> Option<Tool> {
        self.tools
            .wins
            .iter()
            .find(|(_, w)| w.id == win)
            .map(|(t, _)| *t)
    }

    /// Enter or Escape in the active tool window that no control used:
    /// Enter in a generator field generates, Escape closes the guide and the
    /// shortcuts list (as `sa-app`'s) and leaves a control elsewhere.
    pub(crate) fn tool_key(&mut self, key: &str) -> bool {
        let Some(win) = self.mdi.active() else {
            return false;
        };
        let Some(tool) = self.tool_of(win) else {
            return false;
        };
        let typing = self.kit.focus.as_ref().is_some_and(|f| {
            self.tools.nums.keys().any(|n| n.id() == f.as_ref())
                || f.as_ref().starts_with("ocg.Label")
                || f.as_ref().starts_with("ocg.Unused")
                || f.as_ref().starts_with("ocg.Overrides")
                || f.as_ref().starts_with("ocg.Pilots")
        });
        match (key, tool) {
            ("enter", Tool::Generator) if typing => {
                self.generate(None);
                true
            }
            ("escape", Tool::Guide | Tool::Help) => {
                self.close_child(win);
                true
            }
            ("escape", _) if self.kit.focus.is_some() => {
                self.kit.focus = None;
                true
            }
            _ => false,
        }
    }

    /// The client area of tool window `win`.
    pub(crate) fn tool_body(
        &mut self,
        win: u32,
        tool: Tool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let (cw, ch) = self.mdi.shown_rect(win).map_or(tool.size(), |r| {
            let c = mdi::client_rect(r.w, r.h);
            (c.w, c.h)
        });
        match tool {
            Tool::Generator => self.generator_body(win, cw, ch, window, cx),
            Tool::Calculators => self.calc_body(win, cw, ch, window, cx),
            Tool::Guide => self.guide_body(win, cw, ch, window, cx),
            Tool::Help => self.help_body(win, cw, ch, window, cx),
        }
    }

    /// A number field's text changed: read it.
    pub(crate) fn tool_num_changed(&mut self, n: Num) {
        let Some(text) = self.tools.nums.get(&n).map(|e| e.text.clone()) else {
            return;
        };
        let hub = &mut self.tools;
        match n.apply(&text, &mut hub.ocg.spec, &mut hub.calc) {
            Ok(()) => {
                hub.errors.remove(&n);
            }
            Err(e) => {
                hub.errors.insert(n, e);
            }
        }
    }

    /// A text field changed.
    pub(crate) fn tool_text_changed(&mut self, f: TextField) {
        if f == TextField::Label
            && let Some(text) = self.tools.texts.get(&f).map(|e| e.text.clone())
        {
            self.tools.ocg.spec.common_mut().label = text;
        }
    }

    pub(crate) fn tool_pick(&mut self, p: Pick, i: usize) {
        self.tools.pick(p, i);
    }

    pub(crate) fn tool_toggle(&mut self, t: Toggle, on: bool) {
        self.tools.toggle(t, on);
    }

    /// A tool window's button was pressed; `at` is the window position under
    /// it (where its menu opens).
    pub(crate) fn tool_button(
        &mut self,
        b: Btn,
        at: Option<(i32, i32)>,
        cx: &mut Context<'_, Self>,
    ) {
        let at = at.or(self.pointer).unwrap_or((0, 0));
        match b {
            Btn::Preset => self.open_popup(preset_items(), at),
            Btn::GenerateSave => self.open_popup(save_items(), at),
            Btn::Generate => self.generate(None),
            Btn::Calculators => {
                self.open_tool(Tool::Calculators);
            }
            Btn::OpenGuide => {
                self.open_tool(Tool::Guide);
            }
            Btn::Guide(a) => self.guide_action(a),
            Btn::UseCp | Btn::UseInverse | Btn::UseCandidate | Btn::UseK => self.use_calc(b),
            _ => self.edit_button(b),
        }
        cx.notify();
    }

    /// The buttons that change the spec.
    fn edit_button(&mut self, b: Btn) {
        let spec = &mut self.tools.ocg.spec;
        match (b, &mut *spec) {
            (Btn::NewSeed, s) => {
                let c = s.common_mut();
                c.seed = new_seed(c.seed);
            }
            (Btn::AddTone, GeneratorSpec::Tones(s))
                if s.tones.len() < sn_synth::spec::MAX_TONES =>
            {
                let next = s.tones.last().map_or(1000.0, |t| t.frequency + 500.0);
                s.tones.push(sn_synth::ToneSpec {
                    frequency: next,
                    amplitude: 0.25,
                    phase: 0.0,
                });
            }
            (Btn::RemoveTone(i), GeneratorSpec::Tones(s))
                if s.tones.len() > 1 && i < s.tones.len() =>
            {
                s.tones.remove(i);
            }
            (Btn::AddTap, s) => {
                let taps = &mut s.common_mut().channel.multipath;
                if taps.len() < sn_synth::MAX_TAPS {
                    // The first tap is the direct path.
                    let tap = match taps.last() {
                        None => sn_synth::Tap {
                            delay: 0.0,
                            gain: 1.0,
                            phase: 0.0,
                        },
                        Some(t) => sn_synth::Tap {
                            delay: t.delay + 1e-3,
                            gain: 0.5,
                            phase: 0.0,
                        },
                    };
                    taps.push(tap);
                }
            }
            (Btn::RemoveTap(i), s) => {
                let taps = &mut s.common_mut().channel.multipath;
                if i < taps.len() {
                    taps.remove(i);
                }
            }
            (Btn::Msk | Btn::Gmsk, GeneratorSpec::Fsk(s)) => {
                s.order = 2;
                s.spacing = s.baud / 2.0;
                s.shaping = if b == Btn::Gmsk {
                    FskShaping::Gaussian { bt: 0.3 }
                } else {
                    FskShaping::None
                };
            }
            _ => return,
        }
        self.tools.refresh();
    }

    /// `Use in OFDM` of a calculator: the generator shows the OFDM editor
    /// with the values.
    fn use_calc(&mut self, b: Btn) {
        let hub = &mut self.tools;
        let row = |t| hub.tables.get(&t).and_then(|s: &TableState| s.selected);
        let (cand, k) = (row(TableId::Candidates), row(TableId::KList));
        match b {
            Btn::UseCp => {
                if let Some(u) = hub.calc.use_cp() {
                    hub.ocg.use_lengths(u);
                }
            }
            Btn::UseInverse => {
                if let Some(u) = hub.calc.use_inverse() {
                    hub.ocg.use_lengths(u);
                }
            }
            Btn::UseCandidate => {
                if let Some(u) = cand.and_then(|i| hub.calc.use_candidate(i)) {
                    hub.ocg.use_lengths(u);
                }
            }
            Btn::UseK => {
                if let Some(f) = k.and_then(|i| hub.calc.use_k(i)) {
                    hub.ocg.use_fraction(f);
                }
            }
            _ => return,
        }
        hub.sync();
        self.open_tool(Tool::Generator);
        if let Some(id) = self.tool_window(Tool::Calculators) {
            // The calculators stay in front of the generator they fill.
            self.mdi.activate(id);
        }
    }

    /// A command of the generator's popup menus.
    pub(crate) fn tool_command(&mut self, id: u32) {
        if id == OC_DEFAULTS {
            let kind = self.tools.ocg.spec.kind();
            self.tools.ocg.set_spec(GeneratorSpec::default_for(kind));
            self.tools.sync();
        } else if let Some(name) = id
            .checked_sub(OC_PRESET)
            .and_then(|i| PRESET_NAMES.get(i as usize))
            .filter(|_| id < OC_SAVE)
        {
            if let Some(s) = GeneratorSpec::preset(name) {
                self.tools.ocg.set_spec(s);
                self.tools.sync();
            }
        } else if let Some(f) = id
            .checked_sub(OC_SAVE)
            .and_then(|i| SaveFormat::ALL.get(i as usize))
        {
            self.generate(Some(*f));
        }
    }

    /// A generate job is running.
    pub(crate) fn generating(&self) -> bool {
        self.jobs
            .iter()
            .any(|j| j.kind == JobKind::Generate && !j.cancelled)
    }

    /// Generate the spec as a new root signal (`ocg(n)`), then save it in
    /// `save` format if set. The session records the whole spec, so it
    /// replays exactly.
    pub(crate) fn generate(&mut self, save: Option<SaveFormat>) {
        if self.generating() {
            return;
        }
        if let Some(e) = self.tools.problem() {
            self.alert(&e);
            return;
        }
        let spec = self.tools.ocg.spec.clone();
        self.remember_generator();
        // A failure is in the status bar; generating goes ahead.
        let _ = self.save_prefs();
        let name = if spec.common().label.is_empty() {
            kind_label(spec.kind())
        } else {
            spec.common().label.clone()
        };
        let handle = job::spawn(t!("gen-job", name = name), move |ctx| {
            ctx.set_total(1);
            ctx.checkpoint()?;
            let stop = || ctx.is_cancelled();
            let made = sn_synth::generate_with_stop(&spec, &stop).map_err(|e| synth_to_job(&e))?;
            ctx.checkpoint()?;
            ctx.advance(1);
            let label = if spec.common().label.is_empty() {
                made.description.clone()
            } else {
                spec.common().label.clone()
            };
            let operation = sn_session::generate_operation(&spec, &made.description)
                .map_err(|e| job::JobError::Failed(e.to_string()))?;
            Ok(Output::Generated {
                label,
                operation,
                signal: made.signal,
                save,
            })
        });
        let mut job = AppJob::new(self.generation, false, handle);
        job.kind = JobKind::Generate;
        self.jobs.push(job);
    }
}

/// `Preset` menu: `Defaults`, then the presets.
fn preset_items() -> Vec<Item> {
    let mut items = vec![
        Item::command(&t!("gen-preset-defaults"), OC_DEFAULTS),
        Item::separator(),
    ];
    for (i, name) in (0_u32..).zip(PRESET_NAMES) {
        items.push(Item::command(name, OC_PRESET + i));
    }
    super::menus::add_mnemonics(&mut items);
    items
}

/// `Generate and save as` menu: the save formats.
fn save_items() -> Vec<Item> {
    let mut items: Vec<Item> = (0_u32..)
        .zip(SaveFormat::ALL)
        .map(|(i, f)| Item::command(&f.label(), OC_SAVE + i))
        .collect();
    super::menus::add_mnemonics(&mut items);
    items
}
