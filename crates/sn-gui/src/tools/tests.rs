//! Behaviour tests of the tool windows on GPUI's test platform: keyboard and
//! mouse input is simulated inside the test app only.

use std::time::{Duration, Instant};

use gpui::{Entity, TestAppContext, VisualTestContext};
use serde_json::json;
use sn_core::Operation;
use sn_synth::{GeneratorSpec, SignalKind};

use super::fields::{CalcNum, Num};
use super::{Btn, GuideAction, OC_PRESET, TableId, Tool};
use crate::app::Frame;
use crate::app::model::WinKind;
use crate::win95::edit::EditState;

fn frame(cx: &mut TestAppContext) -> (Entity<Frame>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(Frame::new);
    cx.run_until_parked();
    (view, cx)
}

/// Wait for every background job, then let the frame collect the results.
fn settle(view: &Entity<Frame>, cx: &mut VisualTestContext) {
    let start = Instant::now();
    loop {
        let busy = view.read_with(cx, |f, _| !f.jobs.is_empty());
        if !busy {
            break;
        }
        assert!(
            start.elapsed() <= Duration::from_secs(30),
            "jobs still running after 30 s"
        );
        let done = view.read_with(cx, |f, _| f.jobs.iter().all(|j| j.handle.is_finished()));
        if done {
            view.update(cx, |f, cx| {
                f.poll(cx);
                cx.notify();
            });
            cx.run_until_parked();
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

/// The titles of all windows, bottom to top.
fn titles(view: &Entity<Frame>, cx: &mut VisualTestContext) -> Vec<String> {
    view.read_with(cx, |f, _| {
        f.mdi.children().iter().map(|c| c.title.clone()).collect()
    })
}

fn active_tool(view: &Entity<Frame>, cx: &mut VisualTestContext) -> Option<Tool> {
    view.read_with(cx, |f, _| f.mdi.active().and_then(|w| f.tool_of(w)))
}

fn focus(view: &Entity<Frame>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |f, _| {
        f.kit
            .focus
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default()
    })
}

fn press(view: &Entity<Frame>, cx: &mut VisualTestContext, b: Btn) {
    view.update(cx, |f, cx| {
        f.tool_button(b, None, cx);
        cx.notify();
    });
    cx.run_until_parked();
}

#[gpui::test]
fn generator_makes_ocg_windows_recorded_in_the_session(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_keystrokes("cmd-shift-n");
    assert_eq!(active_tool(&view, cx), Some(Tool::Generator));
    assert_eq!(titles(&view, cx), ["Signal generator (OCG)"]);
    // Alt+P opens the Preset menu: Defaults, a separator and the presets.
    cx.simulate_keystrokes("alt-p");
    let n = view.read_with(cx, |f, _| f.popup.as_ref().map(|p| p.items.len()));
    assert_eq!(n, Some(2 + sn_synth::PRESET_NAMES.len()));
    cx.simulate_keystrokes("escape");
    let qam = sn_synth::PRESET_NAMES
        .iter()
        .position(|p| *p == "qam16_rrc_1200")
        .unwrap() as u32;
    view.update(cx, |f, cx| {
        f.tool_command(OC_PRESET + qam);
        cx.notify();
    });
    cx.run_until_parked();
    let preset = GeneratorSpec::preset("qam16_rrc_1200").unwrap();
    view.update(cx, |f, _| {
        assert_eq!(f.tools.ocg.spec, preset);
        assert!(f.tools.problem().is_none());
    });
    // Alt+G is Generate: the signal opens as ocg(0) in a sonogram window.
    cx.simulate_keystrokes("alt-g");
    assert!(view.read_with(cx, |f, _| f.generating()));
    settle(&view, cx);
    let t = titles(&view, cx);
    assert!(t.iter().any(|s| s.contains("ocg(0)")), "{t:?}");
    view.read_with(cx, |f, _| {
        let n = f.active_node().unwrap();
        let node = f.graph.get(n).unwrap();
        assert_eq!(node.operation.name, sn_session::OP_GENERATE);
        assert_eq!(
            f.binding(f.mdi.active().unwrap()).unwrap().1,
            WinKind::Sonogram
        );
        assert!(f.unsaved());
        // The spec is remembered for the next run.
        let stored = f.stored.generator.as_ref().map(ToString::to_string);
        assert_eq!(
            sn_ui_core::prefs::generator_from_json(stored.as_deref()),
            preset
        );
    });
    // A second one is ocg(1), from Enter in a generator field.
    let gen_win = view.read_with(cx, |f, _| f.tool_window(Tool::Generator).unwrap());
    view.update(cx, |f, cx| {
        f.mdi.activate(gen_win);
        f.kit.focus = Some(Num::Baud.id().into());
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    settle(&view, cx);
    let t = titles(&view, cx);
    assert!(t.iter().any(|s| s.contains("ocg(1)")), "{t:?}");
    // The session records both, so they replay as ocg windows.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.sars");
    assert!(view.update(cx, |f, _| f.write_session_to(path.clone())));
    settle(&view, cx);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(sn_session::OP_GENERATE), "{text}");
    view.update(cx, |f, _| f.open_session_from(path.clone()));
    settle(&view, cx);
    let t = titles(&view, cx);
    assert_eq!(t.iter().filter(|s| s.contains("ocg(")).count(), 2, "{t:?}");
}

#[gpui::test]
fn a_bad_field_blocks_generate_and_says_why(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_tool(Tool::Generator);
        f.tools.nums.insert(Num::SampleRate, EditState::new("abc"));
        f.tool_num_changed(Num::SampleRate);
        cx.notify();
    });
    cx.run_until_parked();
    let problem = view.update(cx, |f, _| f.tools.problem()).unwrap();
    assert!(problem.starts_with("Sample rate:"), "{problem}");
    cx.simulate_keystrokes("alt-g");
    assert!(
        !view.read_with(cx, |f, _| f.generating()),
        "Generate is disabled"
    );
    // Fixing the text clears the problem.
    view.update(cx, |f, _| {
        f.tools.nums.insert(Num::SampleRate, EditState::new("9600"));
        f.tool_num_changed(Num::SampleRate);
        assert!(f.tools.problem().is_none());
        assert!((f.tools.ocg.spec.common().sample_rate - 9600.0).abs() < 1e-9);
    });
}

#[gpui::test]
fn calculators_fill_the_ofdm_editor(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_tool(Tool::Generator);
        cx.notify();
    });
    cx.run_until_parked();
    // Alt+C is Calculators...
    cx.simulate_keystrokes("alt-c");
    assert_eq!(active_tool(&view, cx), Some(Tool::Calculators));
    view.update(cx, |f, _| {
        f.tools
            .nums
            .insert(Num::Calc(CalcNum::Fd), EditState::new("8000"));
        f.tool_num_changed(Num::Calc(CalcNum::Fd));
    });
    press(&view, cx, Btn::UseCp);
    view.read_with(cx, |f, _| {
        let GeneratorSpec::Ofdm(o) = &f.tools.ocg.spec else {
            panic!("the OFDM editor is shown");
        };
        assert_eq!((o.lu, o.lg, o.common.sample_rate), (64, 20, 8000.0));
        assert!(f.tool_window(Tool::Generator).is_some());
    });
    assert_eq!(active_tool(&view, cx), Some(Tool::Calculators));
    // List of K: the first row with the keyboard, Enter uses it.
    let expected = view.update(cx, |f, cx| {
        f.kit.focus = Some(format!("ocg.{:?}", TableId::KList).into());
        cx.notify();
        let fr = f.tools.calc.use_k(0).unwrap();
        sn_ui_core::generator::fraction_lengths(64, fr).unwrap()
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("down enter");
    view.read_with(cx, |f, _| {
        assert_eq!(f.table_selected(TableId::KList), Some(0));
        let GeneratorSpec::Ofdm(o) = &f.tools.ocg.spec else {
            panic!("OFDM");
        };
        assert_eq!((o.lu, o.lg), expected);
    });
    // The generator shows the new LU in its field.
    let lu = view.update(cx, |f, _| f.tools.num_state(Num::Lu).text);
    assert_eq!(lu, expected.0.to_string());
}

#[gpui::test]
fn tab_walks_the_generator_and_scrolls_to_the_focus(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.tools
            .ocg
            .set_spec(GeneratorSpec::default_for(SignalKind::Linear));
        f.tools.sync();
        f.open_tool(Tool::Generator);
        cx.notify();
    });
    cx.run_until_parked();
    let mut order = Vec::new();
    for _ in 0..4 {
        cx.simulate_keystrokes("tab");
        order.push(focus(&view, cx));
    }
    assert_eq!(
        order,
        [
            "ocg.Kind",
            "ocg.Preset",
            "ocg.Constellation",
            Num::Baud.id().as_str()
        ]
    );
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(focus(&view, cx), "ocg.Constellation");
    // Down to the preview's radio buttons: the form scrolls to them.
    for _ in 0..60 {
        cx.simulate_keystrokes("tab");
        if focus(&view, cx) == "ocg.Preview.0" {
            break;
        }
    }
    assert_eq!(focus(&view, cx), "ocg.Preview.0");
    let pos = view.read_with(cx, |f, _| f.tools.wins[&Tool::Generator].scroll.pos);
    assert!(pos > 0, "scrolled to the focused control");
    // Preview.0 is the spectrum; Tab to Preview.1 and Space shows the waveform.
    cx.simulate_keystrokes("tab space");
    assert!(view.read_with(cx, |f, _| f.tools.ocg.show_wave));
    // Escape leaves the control.
    cx.simulate_keystrokes("escape");
    assert_eq!(focus(&view, cx), "");
    // The kind combo box: Down chooses the next kind and its editor.
    view.update(cx, |f, cx| {
        f.kit.focus = Some("ocg.Kind".into());
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("down");
    assert_eq!(
        view.read_with(cx, |f, _| f.tools.ocg.spec.kind()),
        SignalKind::C4fm
    );
}

#[gpui::test]
fn guide_buttons_act_on_the_top_signal_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_keystrokes("f1");
    assert_eq!(active_tool(&view, cx), Some(Tool::Guide));
    assert!(view.read_with(cx, |f, _| f.guide_target().is_none()));
    // Without a signal the guide still opens a recording.
    press(&view, cx, Btn::Guide(GuideAction::OpenRecording));
    assert!(
        view.read_with(cx, |f, _| f.loader.is_some()),
        "SA Open file"
    );
    view.update(cx, |f, _| f.loader = None);
    let node = view.update(cx, |f, cx| {
        let s: Vec<f32> = (0..16_000).map(|i| (i as f32 * 0.3).sin() * 0.5).collect();
        let signal = sn_core::Signal::real(s, 8000.0).unwrap();
        let op = Operation::new("load", json!({"path": "/data/a.wav"}));
        let id = f.on_loaded("a.wav".into(), op, signal, &[]);
        cx.notify();
        id
    });
    settle(&view, cx);
    let guide = view.read_with(cx, |f, _| f.tool_window(Tool::Guide).unwrap());
    view.update(cx, |f, _| f.mdi.activate(guide));
    assert_eq!(
        view.read_with(cx, |f, _| f.guide_target().map(|t| t.1)),
        Some(node)
    );
    press(&view, cx, Btn::Guide(GuideAction::Open(WinKind::Waveform)));
    view.read_with(cx, |f, _| {
        assert!(f.find_window(node, WinKind::Waveform).is_some());
    });
    // A detector makes a new signal from the guide's signal.
    let before = view.read_with(cx, |f, _| f.live_nodes().len());
    press(&view, cx, Btn::Guide(GuideAction::AmDetector));
    settle(&view, cx);
    assert_eq!(view.read_with(cx, |f, _| f.live_nodes().len()), before + 1);
    // F1 shows or hides it; Escape closes it when it is active.
    view.update(cx, |f, _| f.mdi.activate(guide));
    cx.simulate_keystrokes("escape");
    assert!(view.read_with(cx, |f, _| f.tool_window(Tool::Guide).is_none()));
    cx.simulate_keystrokes("f1");
    assert_eq!(active_tool(&view, cx), Some(Tool::Guide));
    cx.simulate_keystrokes("f1");
    assert!(view.read_with(cx, |f, _| f.tool_window(Tool::Guide).is_none()));
}

#[gpui::test]
fn help_lists_the_keys_and_opens_the_guide(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_keystrokes("cmd-/");
    assert_eq!(active_tool(&view, cx), Some(Tool::Help));
    // Tab reaches the guide button first; Space presses it.
    cx.simulate_keystrokes("tab");
    assert_eq!(focus(&view, cx), "ocg.OpenGuide");
    cx.simulate_keystrokes("space");
    assert_eq!(active_tool(&view, cx), Some(Tool::Guide));
    // Help > Use help... brings the list back to the front; Ctrl+/ hides it.
    cx.simulate_keystrokes("alt-h u");
    assert_eq!(active_tool(&view, cx), Some(Tool::Help));
    cx.simulate_keystrokes("ctrl-/");
    assert!(view.read_with(cx, |f, _| f.tool_window(Tool::Help).is_none()));
    // File > Demo... opens the guide from the menu.
    cx.simulate_keystrokes("alt-f d");
    assert_eq!(active_tool(&view, cx), Some(Tool::Guide));
}

#[gpui::test]
fn shortcuts_open_and_close_the_tool_windows(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_keystrokes("ctrl-shift-n");
    assert_eq!(active_tool(&view, cx), Some(Tool::Generator));
    // Alt+F still opens the File menu over the generator.
    cx.simulate_keystrokes("alt-f");
    assert!(view.read_with(cx, |f, _| f.track.is_active()));
    cx.simulate_keystrokes("escape escape");
    cx.simulate_keystrokes("cmd-shift-n");
    assert!(view.read_with(cx, |f, _| f.tool_window(Tool::Generator).is_none()));
    // CloseAllWindows closes the tool windows too.
    cx.simulate_keystrokes("f1 cmd-shift-n");
    assert_eq!(titles(&view, cx).len(), 2);
    view.update(cx, |f, _| f.close_all_windows());
    assert!(view.read_with(cx, |f, _| f.tools.wins.is_empty()));
}

#[gpui::test]
fn the_status_bar_shows_the_highlighted_menu_items_prompt(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_keystrokes("alt-f down");
    let msg = view.update(cx, |f, _| f.status_texts()[2].clone());
    assert_eq!(msg, sn_ui_core::t!("menu-demo.hover"));
    cx.simulate_keystrokes("escape escape");
    let msg = view.update(cx, |f, _| f.status_texts()[2].clone());
    assert_eq!(msg, "Ready");
}

#[gpui::test]
fn a_question_with_nothing_at_stake_any_more_goes_ahead(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        let s = sn_core::Signal::real(vec![0.0; 800], 8000.0).unwrap();
        let op = Operation::new("load", json!({"path": "/data/a.wav"}));
        f.on_loaded("a.wav".into(), op, s, &[]);
        cx.notify();
    });
    settle(&view, cx);
    cx.simulate_keystrokes("alt-w c");
    assert!(view.read_with(cx, |f, _| f.msgbox.is_some()));
    // The work was saved meanwhile (as a running save would finish).
    view.update(cx, |f, cx| {
        f.dirty = false;
        f.poll(cx);
    });
    view.read_with(cx, |f, _| {
        assert!(f.msgbox.is_none());
        assert_eq!(f.mdi.children(), [], "CloseAllWindows went ahead");
    });
}

#[gpui::test]
fn the_next_script_output_replaces_the_last(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let first = view.update(cx, |f, _| f.show_script_output("a", vec!["1".into()]));
    let second = view.update(cx, |f, _| f.show_script_output("b", vec!["2".into()]));
    assert_eq!(first, second);
    view.read_with(cx, |f, _| {
        assert_eq!(f.mdi.get(first).map(|c| c.title.as_str()), Some("b"));
        assert_eq!(f.texts[&first].lines, ["2"]);
    });
}

#[gpui::test]
fn a_bad_calculator_field_does_not_block_the_generator(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_tool(Tool::Calculators);
        f.tools
            .nums
            .insert(Num::Calc(CalcNum::Fd), EditState::new("abc"));
        f.tool_num_changed(Num::Calc(CalcNum::Fd));
        assert!(f.tools.errors.contains_key(&Num::Calc(CalcNum::Fd)));
        assert_eq!(f.tools.problem(), None, "the generator is not blocked");
        cx.notify();
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_local_change_keeps_refused_text(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.tools
            .ocg
            .set_spec(GeneratorSpec::default_for(SignalKind::Ofdm));
        f.tools.sync();
        f.open_tool(Tool::Generator);
        // A refused list and a refused number.
        f.tools
            .texts
            .insert(super::TextField::Unused, EditState::new("3, seven"));
        f.tools.nums.insert(Num::SampleRate, EditState::new("fast"));
        f.tool_num_changed(Num::SampleRate);
        let before = f.tools.problem();
        assert!(before.is_some());
        f.tool_toggle(super::Toggle::Complex, true);
        f.tool_pick(super::Pick::Preview, 1);
        assert_eq!(f.tools.texts[&super::TextField::Unused].text, "3, seven");
        assert_eq!(f.tools.num_state(Num::SampleRate).text, "fast");
        assert!(f.tools.problem().is_some(), "the message stays");
        cx.notify();
    });
    cx.run_until_parked();
}

#[gpui::test]
fn closing_the_generator_stops_its_preview(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_tool(Tool::Generator);
        cx.notify();
    });
    cx.run_until_parked();
    let start = Instant::now();
    while view.read_with(cx, |f, _| f.tools.ocg.preview.is_none()) {
        assert!(start.elapsed() < Duration::from_secs(30), "no preview");
        view.update(cx, |f, cx| {
            f.poll(cx);
            cx.notify();
        });
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    view.update(cx, |f, _| {
        let id = f.tool_window(Tool::Generator).unwrap();
        f.tool_closed(id);
        assert!(!f.tools_busy(), "no preview work left");
    });
}
