//! Behaviour tests of the frame on GPUI's test platform: keyboard and mouse
//! input is simulated inside the test app only.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui::{Modifiers, MouseButton, Point, TestAppContext, VisualTestContext, point, px};
use serde_json::json;
use sn_core::{Complex32, NodeId, Operation, Signal};
use sn_dsp::ops::Edit;
use sn_ui_core::edits::ParamKind;
use sn_ui_core::theme::ThemeChoice;

use super::model::WinKind;
use super::{Frame, MsgAction, Setup, menus::cmd};
use crate::win95::dialog::Answer;
use crate::win95::mdi::{CASCADE, State, frame_w};
use crate::win95::menu_view::bar_h;

fn p(x: i32, y: i32) -> Point<gpui::Pixels> {
    point(px(x as f32), px(y as f32))
}

/// Window position of a point in the MDI client area.
fn client(x: i32, y: i32) -> Point<gpui::Pixels> {
    p(x + 2, y + bar_h() + 2)
}

fn frame(cx: &mut TestAppContext) -> (gpui::Entity<Frame>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(Frame::new);
    cx.run_until_parked();
    (view, cx)
}

fn drag(cx: &mut VisualTestContext, from: Point<gpui::Pixels>, to: Point<gpui::Pixels>) {
    let none = Modifiers::none();
    cx.simulate_mouse_down(from, MouseButton::Left, none);
    cx.simulate_mouse_move(to, MouseButton::Left, none);
    cx.simulate_mouse_up(to, MouseButton::Left, none);
    cx.run_until_parked();
}

/// A real test signal: a tone at `rate` Hz, `n` samples.
fn tone(rate: f64, n: usize) -> Signal {
    let s: Vec<f32> = (0..n).map(|i| (i as f32 * 0.3).sin() * 0.5).collect();
    Signal::real(s, rate).unwrap()
}

/// Add a loaded recording as if `SA Open file` had read it.
fn add_signal(
    view: &gpui::Entity<Frame>,
    cx: &mut VisualTestContext,
    name: &str,
    rate: f64,
) -> NodeId {
    let id = view.update(cx, |f, cx| {
        let op = Operation::new("load", json!({"path": format!("/data/{name}")}));
        let id = f.on_loaded(name.to_owned(), op, tone(rate, 16_000), &[]);
        cx.notify();
        id
    });
    cx.run_until_parked();
    settle(view, cx);
    id
}

/// A WAV file of `signal` in a new temporary folder.
fn wav(dir: &Path, name: &str, signal: &Signal) -> PathBuf {
    let path = dir.join(name);
    sn_io::write_wav(&path, signal, sn_io::WavFormat::Pcm16).unwrap();
    path
}

/// Wait for every background job, then let the frame collect the results.
fn settle(view: &gpui::Entity<Frame>, cx: &mut VisualTestContext) {
    let start = Instant::now();
    loop {
        let busy = view.read_with(cx, |f, _| !f.jobs.is_empty() || !f.pending.is_empty());
        if !busy {
            break;
        }
        if start.elapsed() > Duration::from_secs(20) {
            let names = view.read_with(cx, |f, _| {
                f.jobs
                    .iter()
                    .map(|j| j.handle.name().to_owned())
                    .collect::<Vec<_>>()
            });
            panic!("jobs still running after 20 s: {names:?}");
        }
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

/// The titles of the signal windows, bottom to top.
fn titles(view: &gpui::Entity<Frame>, cx: &mut VisualTestContext) -> Vec<String> {
    view.read_with(cx, |f, _| {
        f.signal_windows()
            .into_iter()
            .filter_map(|(id, _, _)| f.mdi.get(id).map(|c| c.title.clone()))
            .collect()
    })
}

fn active(view: &gpui::Entity<Frame>, cx: &mut VisualTestContext) -> u32 {
    view.read_with(cx, |f, _| f.mdi.active().unwrap())
}

#[gpui::test]
fn starts_empty_with_sa_status_bar(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, _| {
        assert_eq!(f.mdi.children(), []);
        let [rate, path, msg] = f.status_texts();
        assert_eq!(rate, "SamplingRate = ");
        assert_eq!(path, "");
        assert_eq!(msg, "Ready");
    });
}

#[gpui::test]
fn open_file_loads_a_wav_into_a_window(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = wav(dir.path(), "test.wav", &tone(8000.0, 16_000));
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_loader(Some(path.clone()));
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        let l = f.loader.as_ref().unwrap();
        assert_eq!(
            l.selected_entry().map(|e| e.path.clone()),
            Some(path.clone())
        );
        let (lines, standard) = l.info_lines().unwrap();
        assert!(standard, "16-bit mono PCM is SA's standard file");
        assert!(lines.contains(&"Freq rate = 8000".to_owned()), "{lines:?}");
        assert_eq!(l.ext_info(), "Ext. Info: PCM");
    });
    cx.simulate_keystrokes("enter");
    assert!(
        view.read_with(cx, |f, _| f.loader.is_none()),
        "Ok closes the loader"
    );
    settle(&view, cx);
    assert_eq!(titles(&view, cx), ["[TempDir] test(0).wav"]);
    view.update(cx, |f, _| {
        let win = f.mdi.active().unwrap();
        assert_eq!(f.binding(win).map(|(_, k)| k), Some(WinKind::Sonogram));
        let [rate, shown, msg] = f.status_texts();
        assert_eq!(rate, "SamplingRate = 8000");
        assert_eq!(shown, path.display().to_string());
        assert!(msg.starts_with("Loaded test.wav"), "{msg}");
        assert!(f.overviews.contains_key(&f.active_node().unwrap()));
    });
    // The same file again is `test(1)`, 24 px lower right is not needed for a
    // load: it opens at the cascade position.
    view.update(cx, |f, cx| {
        f.open_dropped(std::slice::from_ref(&path));
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    settle(&view, cx);
    assert_eq!(
        titles(&view, cx),
        ["[TempDir] test(0).wav", "[TempDir] test(1).wav"]
    );
}

#[gpui::test]
fn stereo_files_ask_which_channel(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let iq: Vec<Complex32> = (0..8000)
        .map(|i| Complex32::new((i as f32 * 0.1).sin(), 0.25))
        .collect();
    let path = wav(dir.path(), "st.wav", &Signal::complex(iq, 8000.0).unwrap());
    let (view, cx) = frame(cx);
    view.update(cx, |f, _| f.open_loader(Some(path.clone())));
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    assert!(view.read_with(cx, |f, _| f.loader.as_ref().unwrap().prompt.is_some()));
    // Break returns to the loader.
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |f, _| {
        let l = f.loader.as_ref().unwrap();
        assert!(l.prompt.is_none());
    });
    // Channel - 1 (Up from the default All samples twice).
    cx.simulate_keystrokes("enter up up enter");
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert!(f.loader.is_none());
        let n = f.active_node().unwrap();
        assert_eq!(
            f.graph.get(n).unwrap().signal.kind(),
            sn_core::SampleKind::Real
        );
    });
}

#[gpui::test]
fn derived_windows_take_sa_tags_and_cascade(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "test.wav", 8000.0);
    let first = active(&view, cx);
    view.update(cx, |f, _| f.start_edit(first, Edit::Derivative));
    settle(&view, cx);
    let second = active(&view, cx);
    view.update(cx, |f, _| {
        let (a, b) = (
            f.mdi.get(first).unwrap().rect,
            f.mdi.get(second).unwrap().rect,
        );
        assert_eq!(
            (b.x, b.y),
            (a.x + CASCADE, a.y + CASCADE),
            "lower right of the caller"
        );
        f.start_edit(second, Edit::Invert);
    });
    settle(&view, cx);
    let third = active(&view, cx);
    view.update(cx, |f, _| {
        f.start_edit(third, Edit::Resample { rate: 4000.0 });
    });
    settle(&view, cx);
    assert_eq!(
        titles(&view, cx),
        [
            "[TempDir] test(0).wav",
            "[TempDir] test(0)(std1).wav",
            "[TempDir] test(0)(std1)(bi1).wav",
            "[TempDir] test(0)(std1)(bi1)(r_4000).wav",
        ]
    );
    view.read_with(cx, |f, _| {
        assert!(f.status.contains("(r_4000)"), "{}", f.status);
        let n = f.active_node().unwrap();
        assert!((f.graph.get(n).unwrap().signal.sample_rate() - 4000.0).abs() < 1e-9);
    });
}

#[gpui::test]
fn acf_asks_first_and_opens_a_wf_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "test.wav", 8000.0);
    let win = active(&view, cx);
    view.update(cx, |f, cx| {
        f.start_edit(
            win,
            Edit::Acf {
                start: 0,
                end: 1000,
            },
        );
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        let mb = f.msgbox.as_ref().expect("SA's question");
        assert!(mb.text.starts_with("Expected running time"), "{}", mb.text);
        assert!(matches!(f.msg_action, MsgAction::Correlation { .. }));
        assert!(f.jobs.is_empty(), "nothing runs before OK");
    });
    cx.simulate_keystrokes("enter");
    settle(&view, cx);
    let t = titles(&view, cx);
    assert_eq!(
        t.last().map(String::as_str),
        Some("[TempDir] test(0)_acf(0).wav")
    );
    view.read_with(cx, |f, _| {
        assert_eq!(f.active_kind(), Some(WinKind::Waveform));
    });
}

#[gpui::test]
fn closing_a_window_is_the_undo(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let root = add_signal(&view, cx, "test.wav", 8000.0);
    let w0 = active(&view, cx);
    view.update(cx, |f, _| f.start_edit(w0, Edit::Derivative));
    settle(&view, cx);
    let w1 = active(&view, cx);
    view.update(cx, |f, _| f.start_edit(w1, Edit::Invert));
    settle(&view, cx);
    let w2 = active(&view, cx);
    let (mid, leaf) = view.read_with(cx, |f, _| (f.node_of(w1).unwrap(), f.node_of(w2).unwrap()));
    // The middle result has an open result: it stays, hidden.
    view.update(cx, |f, _| f.close_child(w1));
    view.read_with(cx, |f, _| {
        assert_eq!(f.live_nodes(), [root, mid, leaf]);
        assert!(!f.shows(mid));
    });
    // Closing the last result drops it and the hidden signal it came from.
    cx.simulate_keystrokes("cmd-w");
    view.read_with(cx, |f, _| {
        assert_eq!(f.live_nodes(), [root]);
        assert!(
            f.graph.get(leaf).unwrap().signal.is_empty(),
            "samples freed"
        );
        assert_eq!(f.mdi.active(), Some(w0));
    });
    // Closing the recording's window empties the session; the status bar
    // forgets the source.
    cx.simulate_keystrokes("cmd-w");
    view.update(cx, |f, _| {
        assert!(!f.has_signals());
        assert_eq!(f.status_texts()[0], "SamplingRate = ");
    });
}

#[gpui::test]
fn close_all_windows_asks_about_unsaved_work(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "a.wav", 8000.0);
    add_signal(&view, cx, "b.wav", 11025.0);
    cx.simulate_keystrokes("alt-w c");
    view.read_with(cx, |f, _| {
        let mb = f.msgbox.as_ref().expect("save first?");
        assert_eq!(mb.labels.len(), 3, "{:?}", mb.labels);
        assert_eq!(mb.labels[1], "Close all");
    });
    view.update(cx, |f, cx| f.answer(Answer::Cancel, cx));
    assert_eq!(titles(&view, cx).len(), 2, "Cancel keeps everything");
    cx.simulate_keystrokes("alt-w c");
    view.update(cx, |f, cx| f.answer(Answer::No, cx));
    view.update(cx, |f, _| {
        assert_eq!(f.mdi.children(), []);
        assert!(!f.has_signals());
        assert!(!f.unsaved());
    });
    // With nothing to lose it does not ask.
    add_signal(&view, cx, "c.wav", 8000.0);
    view.update(cx, |f, _| f.dirty = false);
    cx.simulate_keystrokes("alt-w c");
    view.read_with(cx, |f, _| {
        assert!(f.msgbox.is_none());
        assert_eq!(f.mdi.children(), []);
        assert_eq!(titles_of(f).len(), 0);
    });
    // Names start again after a new start.
    add_signal(&view, cx, "a.wav", 8000.0);
    assert_eq!(titles(&view, cx), ["[TempDir] a(0).wav"]);
}

fn titles_of(f: &Frame) -> Vec<String> {
    f.mdi.children().iter().map(|c| c.title.clone()).collect()
}

#[gpui::test]
fn status_bar_follows_the_pointer_then_the_active_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "a.wav", 8000.0);
    let first = active(&view, cx);
    add_signal(&view, cx, "b.wav", 11025.0);
    let r = view.read_with(cx, |f, _| f.mdi.get(first).unwrap().rect);
    // Over the visible caption part of the first (lower) window.
    cx.simulate_mouse_move(
        client(r.x + 6, r.y + frame_w() + 5),
        None,
        Modifiers::none(),
    );
    view.update(cx, |f, _| {
        let [rate, path, _] = f.status_texts();
        assert_eq!(rate, "SamplingRate = 8000");
        assert_eq!(path, "/data/a.wav");
    });
    // Off the windows: the active one.
    cx.simulate_mouse_move(p(2, bar_h() + 400), None, Modifiers::none());
    view.update(cx, |f, _| {
        assert_eq!(f.status_texts()[0], "SamplingRate = 11025");
    });
}

#[gpui::test]
fn parameter_dialog_checks_its_values(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "test.wav", 8000.0);
    let win = active(&view, cx);
    view.update(cx, |f, cx| {
        f.open_params(win, ParamKind::Power { n: 2 }, Vec::new());
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        let d = f.params.as_ref().unwrap();
        assert_eq!(d.texts(), ["2"]);
        assert_eq!(f.kit.focus.as_ref().map(AsRef::as_ref), Some("pd.f0"));
    });
    cx.simulate_keystrokes("9 9 enter");
    view.read_with(cx, |f, _| {
        let mb = f.msgbox.as_ref().expect("Please check settings!");
        assert!(mb.text.starts_with("Please check settings!"), "{}", mb.text);
        assert!(mb.text.contains("\"99\""), "{}", mb.text);
        assert!(mb.text.contains("From 2 to 32"), "{}", mb.text);
        assert!(f.params.is_some(), "the dialog stays");
    });
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |f, _| {
        assert!(f.msgbox.is_none());
        assert_eq!(
            f.kit.focus.as_ref().map(AsRef::as_ref),
            Some("pd.f0"),
            "back to the field"
        );
    });
    cx.simulate_keystrokes("3 enter");
    view.read_with(cx, |f, _| assert!(f.params.is_none()));
    settle(&view, cx);
    assert_eq!(
        titles(&view, cx).last().map(String::as_str),
        Some("[TempDir] test(0)(q).wav")
    );
    // Escape cancels.
    view.update(cx, |f, cx| {
        f.open_params(win, ParamKind::Median { window: 5 }, Vec::new());
        cx.notify();
    });
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |f, _| assert!(f.params.is_none() && f.jobs.is_empty()));
}

#[gpui::test]
fn prefs_are_remembered_in_the_settings_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sa-rs").join("prefs.json");
    let setup = Setup {
        store: Some(path.clone()),
        ..Setup::default()
    };
    let (view, cx) = cx.add_window_view(move |w, cx| Frame::with_setup(setup.clone(), w, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("alt-o down right enter");
    let stored = crate::store::Stored::load(&path);
    assert!(stored.prefs.info_default, "Options > Info by default > On");
    let hc = ThemeChoice::ALL
        .iter()
        .position(|t| *t == ThemeChoice::HighContrastDark)
        .unwrap();
    view.update_in(cx, |f, window, cx| {
        f.command(cmd::THEME + hc as u32, window, cx);
    });
    let stored = crate::store::Stored::load(&path);
    assert_eq!(stored.prefs.theme, ThemeChoice::HighContrastDark);
    cx.update(|_, cx| {
        assert_eq!(
            crate::win95::colors::scheme(cx),
            crate::win95::colors::Scheme::HIGH_CONTRAST_BLACK
        );
    });
    view.update_in(cx, |f, window, cx| {
        f.command(cmd::START_SIZE + 1, window, cx);
    });
    let stored = crate::store::Stored::load(&path);
    assert_eq!(
        stored.prefs.start_size,
        sn_ui_core::prefs::StartSize::S800x600
    );
    assert!(
        stored.frame.is_some(),
        "the frame size for As when last closed"
    );
    // A new frame starts from the file.
    let again = crate::store::Stored::load(&path);
    let setup = Setup {
        stored: again,
        ..Setup::default()
    };
    let (view2, cx2) = cx.add_window_view(move |w, cx| Frame::with_setup(setup.clone(), w, cx));
    view2.read_with(cx2, |f, _| {
        assert!(f.prefs.info_default);
        assert_eq!(f.prefs.theme, ThemeChoice::HighContrastDark);
    });
}

#[gpui::test]
fn sessions_restore_the_windows(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let wav_path = wav(dir.path(), "test.wav", &tone(8000.0, 16_000));
    let session = dir.path().join("s.json");
    let (view, cx) = frame(cx);
    view.update(cx, |f, _| f.open_loader(Some(wav_path)));
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    settle(&view, cx);
    let w0 = active(&view, cx);
    view.update(cx, |f, _| f.start_edit(w0, Edit::Derivative));
    settle(&view, cx);
    let before = titles(&view, cx);
    view.update(cx, |f, _| assert!(f.write_session_to(session.clone())));
    settle(&view, cx);
    view.update(cx, |f, cx| {
        assert!(!f.unsaved());
        f.close_all_windows();
        f.open_session_from(session.clone());
        cx.notify();
    });
    settle(&view, cx);
    assert_eq!(titles(&view, cx), before);
    view.read_with(cx, |f, _| {
        assert_eq!(f.session_path.as_deref(), Some(session.as_path()));
        assert_eq!(f.status, "Session restored");
    });
}

#[gpui::test]
fn alt_arrows_bring_back_signals_and_cmd_digits_open_views(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let a = add_signal(&view, cx, "a.wav", 8000.0);
    let b = add_signal(&view, cx, "b.wav", 8000.0);
    cx.simulate_keystrokes("alt-up");
    assert_eq!(view.read_with(cx, |f, _| f.active_node()), Some(a));
    cx.simulate_keystrokes("alt-down");
    assert_eq!(view.read_with(cx, |f, _| f.active_node()), Some(b));
    cx.simulate_keystrokes("cmd-2");
    view.read_with(cx, |f, _| {
        assert_eq!(f.active_kind(), Some(WinKind::Waveform));
        assert_eq!(f.active_node(), Some(b));
    });
    cx.simulate_keystrokes("ctrl-tab");
    assert_ne!(
        view.read_with(cx, |f, _| f.active_kind()),
        Some(WinKind::Waveform)
    );
}

#[gpui::test]
fn alt_letter_opens_a_menu_and_escape_closes_it(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_keystrokes("alt-o");
    view.read_with(cx, |f, _| {
        assert_eq!(f.track.bar, Some(2));
        assert!(f.track.is_open());
    });
    cx.simulate_keystrokes("down right");
    view.read_with(cx, |f, _| {
        assert_eq!(f.track.levels.len(), 2, "Info by default submenu");
    });
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |f, _| {
        assert!(f.prefs.info_default);
        assert!(!f.track.is_active());
    });
    cx.simulate_keystrokes("alt-f escape escape");
    view.read_with(cx, |f, _| assert!(!f.track.is_active()));
}

#[gpui::test]
fn leaving_the_window_forgets_that_alt_is_held(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    cx.simulate_modifiers_change(Modifiers::alt());
    assert!(crate::win95::menu_view::alt_held());
    view.read_with(cx, |f, _| assert!(f.alt_alone));
    // Alt+Tab: the release goes to the other application.
    cx.deactivate_window();
    assert!(
        !crate::win95::menu_view::alt_held(),
        "GNOME and KDE menus drop their underlines"
    );
    view.read_with(cx, |f, _| {
        assert!(!f.alt_alone);
        assert!(!f.track.is_active());
    });
    // Back in the window, the first modifier change is not Alt tapped alone.
    cx.simulate_modifiers_change(Modifiers::none());
    view.read_with(cx, |f, _| assert!(!f.track.is_active()));
}

#[gpui::test]
fn clicking_a_child_activates_it_and_dragging_the_caption_moves_it(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "a.wav", 8000.0);
    add_signal(&view, cx, "b.wav", 8000.0);
    let (first, r) = view.read_with(cx, |f, _| {
        let c = f.mdi.children().first().unwrap();
        (c.id, c.rect)
    });
    // The first child is partly covered; click its caption's visible part.
    let grab = client(r.x + 10 + 18, r.y + frame_w() + 5);
    cx.simulate_click(grab, Modifiers::none());
    view.read_with(cx, |f, _| assert_eq!(f.mdi.active(), Some(first)));
    let to = point(grab.x + px(30.0), grab.y + px(20.0));
    drag(cx, grab, to);
    view.read_with(cx, |f, _| {
        let moved = f.mdi.get(first).map(|c| (c.rect.x, c.rect.y));
        assert_eq!(moved, Some((r.x + 30, r.y + 20)));
        assert_eq!(f.outline, None, "the drag outline is gone after the drop");
    });
}

#[gpui::test]
fn caption_buttons_maximize_and_close(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "a.wav", 8000.0);
    add_signal(&view, cx, "b.wav", 8000.0);
    let (id, r) = view.read_with(cx, |f, _| {
        let c = f.mdi.children().last().unwrap();
        (c.id, c.rect)
    });
    let (_, max, _) = crate::win95::mdi::caption_buttons(r.w);
    cx.simulate_click(client(r.x + max.x + 4, r.y + max.y + 4), Modifiers::none());
    view.read_with(cx, |f, _| {
        assert_eq!(f.mdi.get(id).map(|c| c.state), Some(State::Maximized));
    });
    cx.simulate_keystrokes("ctrl-f4");
    view.read_with(cx, |f, _| {
        assert_eq!(f.mdi.get(id), None);
        assert_eq!(
            f.mdi.maximized().map(|c| c.state),
            Some(State::Maximized),
            "the next child takes over"
        );
    });
    // A maximized child's buttons are at the right of the menu bar.
    let vw = cx.update(|window, _| f32::from(window.viewport_size().width) as i32);
    let close = super::buttons_at(vw).2;
    assert!(
        close.y >= 0 && close.bottom() <= bar_h(),
        "{close:?} fits the bar"
    );
    let at = p(close.x + close.w / 2, close.y + close.h / 2);
    cx.simulate_click(at, Modifiers::none());
    view.read_with(cx, |f, _| {
        assert_eq!(f.mdi.children(), []);
        assert!(!f.has_signals(), "closing is the undo");
    });
}

#[gpui::test]
fn the_message_box_is_modal_and_answers_keys(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_keystrokes("alt-h a");
    view.read_with(cx, |f, _| {
        let mb = f.msgbox.as_ref().expect("Help > About opens a box");
        assert!(mb.text.contains("W95FA"));
        assert!(mb.text.contains("Noto"));
    });
    cx.simulate_keystrokes("alt-f");
    view.read_with(cx, |f, _| {
        assert!(!f.track.is_active(), "menus are blocked");
    });
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |f, _| assert!(f.msgbox.is_none()));
}

#[gpui::test]
fn open_file_from_the_menu_shows_sa_open_file(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_keystrokes("alt-f o");
    view.read_with(cx, |f, _| {
        assert!(f.loader.is_some());
        assert!(f.msgbox.is_none());
    });
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |f, _| assert!(f.loader.is_none()));
    cx.simulate_keystrokes("cmd-o");
    view.read_with(cx, |f, _| assert!(f.loader.is_some()));
    cx.simulate_keystrokes("escape");
    // The generator opens as a tool window.
    cx.simulate_keystrokes("cmd-shift-n");
    view.read_with(cx, |f, _| {
        assert!(f.msgbox.is_none());
        assert!(f.tool_window(super::tools::Tool::Generator).is_some());
    });
}

#[gpui::test]
fn mouse_opens_menus_and_chooses_items(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_click(p(3, 9), Modifiers::none());
    view.read_with(cx, |f, _| {
        assert_eq!((f.track.bar, f.track.is_open()), (Some(0), true));
    });
    // Hover the first item, then release on it (Windows chooses on button up).
    let item = p(30, bar_h() + 3 + 8);
    cx.simulate_mouse_move(item, None, Modifiers::none());
    view.read_with(cx, |f, _| assert_eq!(f.track.levels, vec![Some(0)]));
    cx.simulate_click(item, Modifiers::none());
    view.read_with(cx, |f, _| {
        assert!(!f.track.is_active());
        assert!(f.loader.is_some(), "Open file... opens SA Open file");
    });
    cx.simulate_keystrokes("escape");
    // A click outside an open menu closes it without doing anything else.
    cx.simulate_click(p(3, 9), Modifiers::none());
    cx.simulate_click(p(600, 400), Modifiers::none());
    view.read_with(cx, |f, _| {
        assert!(!f.track.is_active());
        assert!(f.msgbox.is_none() && f.loader.is_none());
    });
}

#[cfg(debug_assertions)]
#[gpui::test]
fn tab_moves_focus_between_controls_of_the_active_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_gallery();
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("tab");
    let first = view.read_with(cx, |f, _| f.kit.focus.clone());
    assert_eq!(first.as_ref().map(AsRef::as_ref), Some("g.ok"));
    cx.simulate_keystrokes("tab tab");
    let third = view.read_with(cx, |f, _| f.kit.focus.clone());
    assert_eq!(
        third.as_ref().map(AsRef::as_ref),
        Some("g.err"),
        "disabled buttons are skipped"
    );
    cx.simulate_keystrokes("shift-tab");
    let back = view.read_with(cx, |f, _| f.kit.focus.clone());
    assert_eq!(back.as_ref().map(AsRef::as_ref), Some("g.cancel"));
    cx.simulate_keystrokes("space");
    view.read_with(cx, |f, _| assert_eq!(f.gallery.clicks, 1));
}

#[cfg(debug_assertions)]
#[gpui::test]
fn typing_into_an_edit_field(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_gallery();
        f.kit.focus = Some("g.name".into());
        f.gallery.name.select_all();
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("1 2 5 0 backspace");
    view.read_with(cx, |f, _| assert_eq!(f.gallery.name.text, "125"));
    cx.simulate_keystrokes("shift-home 9");
    view.read_with(cx, |f, _| assert_eq!(f.gallery.name.text, "9"));
    cx.simulate_keystrokes("tab");
    view.read_with(cx, |f, _| {
        assert_eq!(f.kit.focus.as_ref().map(AsRef::as_ref), Some("g.path"));
    });
}

#[gpui::test]
fn script_results_count_as_unsaved_work(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    wav(dir.path(), "t.wav", &tone(8000.0, 4000));
    let script = dir.path().join("s.rhai");
    std::fs::write(&script, "let s = open(\"t.wav\");\nprint(\"done\");\n").unwrap();
    let (view, cx) = frame(cx);
    view.update(cx, |f, _| f.run_script_from(script));
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert!(f.has_signals(), "{}", f.status);
        assert!(
            f.must_ask(sn_ui_core::app_glue::Confirm::Quit),
            "the script's signals are in no session file"
        );
    });
}

#[gpui::test]
fn a_failed_settings_save_is_shown_before_quitting(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    // A file where the settings folder should be: the save fails.
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "x").unwrap();
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.store = Some(blocker.join("prefs.json"));
        assert!(!f.quit_now(cx), "the app waits for the answer");
        assert!(f.msgbox.is_some());
        assert_eq!(f.msg_action, MsgAction::Quit);
    });
}

#[gpui::test]
fn work_done_while_a_script_runs_is_not_replaced_without_asking(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    wav(dir.path(), "t.wav", &tone(8000.0, 4000));
    let script = dir.path().join("s.rhai");
    std::fs::write(&script, "let s = open(\"t.wav\");\n").unwrap();
    let (view, cx) = frame(cx);
    view.update(cx, |f, _| f.run_script_from(script));
    // A file loaded while the script runs.
    add_signal(&view, cx, "newer.wav", 8000.0);
    settle(&view, cx);
    view.update(cx, |f, cx| {
        assert_eq!(f.msg_action, MsgAction::ApplyHeld, "the user is asked");
        f.answer(Answer::Cancel, cx);
    });
    let names = titles(&view, cx);
    assert!(names.iter().any(|t| t.contains("newer")), "{names:?}");
}

#[gpui::test]
fn a_measurement_recorded_while_a_script_runs_is_not_replaced_without_asking(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    wav(dir.path(), "t.wav", &tone(8000.0, 4000));
    let script = dir.path().join("s.rhai");
    std::fs::write(&script, "let s = open(\"t.wav\");\n").unwrap();
    let (view, cx) = frame(cx);
    let node = add_signal(&view, cx, "older.wav", 8000.0);
    view.update(cx, |f, _| {
        f.run_script_from(script);
        // Work that adds no signal: a measurement.
        f.record_with(
            &sn_core::MeasurementKind::BaudRate,
            50.0,
            None,
            node,
            "test",
        );
    });
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert_eq!(f.msg_action, MsgAction::ApplyHeld, "the user is asked");
    });
}

#[gpui::test]
fn every_held_script_result_gets_its_own_question(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    wav(dir.path(), "t.wav", &tone(8000.0, 4000));
    let script = dir.path().join("s.rhai");
    std::fs::write(&script, "let s = open(\"t.wav\");\n").unwrap();
    let (view, cx) = frame(cx);
    view.update(cx, |f, _| {
        f.run_script_from(script.clone());
        f.run_script_from(script);
    });
    add_signal(&view, cx, "newer.wav", 8000.0);
    settle(&view, cx);
    view.update(cx, |f, cx| {
        assert_eq!(f.held.len(), 2, "no result is dropped");
        assert_eq!(f.msg_action, MsgAction::ApplyHeld);
        f.answer(Answer::Cancel, cx);
        assert_eq!(f.msg_action, MsgAction::ApplyHeld, "the second question");
        f.answer(Answer::Cancel, cx);
        assert!(f.held.is_empty());
    });
    let names = titles(&view, cx);
    assert!(names.iter().any(|t| t.contains("newer")), "{names:?}");
}

#[gpui::test]
fn double_clicking_the_window_icon_closes_it(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "icon.wav", 8000.0);
    let (id, r) = view.read_with(cx, |f, _| {
        let c = f.mdi.children().last().unwrap();
        (c.id, f.mdi.shown_rect(c.id).unwrap())
    });
    let p = client(r.x + frame_w() + 6, r.y + frame_w() + 6);
    let none = Modifiers::none();
    cx.simulate_mouse_down(p, MouseButton::Left, none);
    cx.simulate_mouse_up(p, MouseButton::Left, none);
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |f, _| f.popup.is_some()),
        "the system menu"
    );
    cx.simulate_event(gpui::MouseDownEvent {
        position: p,
        modifiers: none,
        button: MouseButton::Left,
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_event(gpui::MouseUpEvent {
        position: p,
        modifiers: none,
        button: MouseButton::Left,
        click_count: 2,
    });
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        assert!(f.mdi.get(id).is_none(), "closed");
        assert!(f.popup.is_none());
    });
}

#[gpui::test]
fn double_clicking_a_maximized_motif_childs_menu_button_closes_it(cx: &mut TestAppContext) {
    use crate::win95::style;
    use sn_ui_core::winstyle::{Look, WinStyle};
    let (view, cx) = frame(cx);
    let motif = Look {
        style: WinStyle::LinuxMotif,
        ..Look::default()
    };
    view.update(cx, |f, cx| {
        f.look_override = Some(motif);
        cx.notify();
    });
    style::set(motif, ThemeChoice::Classic);
    add_signal(&view, cx, "max.wav", 8000.0);
    let id = view.update(cx, |f, cx| {
        let id = f.mdi.children().last().map(|c| c.id);
        if let Some(id) = id {
            f.mdi.command(id, crate::win95::mdi::Hit::Maximize);
        }
        cx.notify();
        id
    });
    cx.run_until_parked();
    assert!(view.read_with(cx, |f, _| f.mdi.maximized().is_some()));
    // The window-menu button at the left of the menu bar.
    let at = p(6, bar_h() / 2);
    let none = Modifiers::none();
    cx.simulate_mouse_down(at, MouseButton::Left, none);
    cx.simulate_mouse_up(at, MouseButton::Left, none);
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |f, _| f.popup.is_some()),
        "one click: the window menu"
    );
    cx.simulate_event(gpui::MouseDownEvent {
        position: at,
        modifiers: none,
        button: MouseButton::Left,
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_event(gpui::MouseUpEvent {
        position: at,
        modifiers: none,
        button: MouseButton::Left,
        click_count: 2,
    });
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        assert!(id.is_some_and(|id| f.mdi.get(id).is_none()), "closed");
        assert!(f.popup.is_none());
    });
    style::set(Look::WIN95, ThemeChoice::Classic);
}

#[gpui::test]
fn escape_closing_a_dropdown_keeps_the_focus_and_the_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_tool(super::tools::Tool::Generator);
        cx.notify();
    });
    cx.run_until_parked();
    // Focus the kind combo and open its list.
    cx.simulate_keystrokes("tab");
    cx.simulate_keystrokes("alt-down");
    let open = view.read_with(cx, |f, _| f.kit.popup.is_some());
    assert!(open, "the list is open");
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |f, _| {
        assert!(f.kit.popup.is_none(), "the list closed");
        assert!(f.kit.focus.is_some(), "the combo keeps the focus");
        assert!(
            f.tool_window(super::tools::Tool::Generator).is_some(),
            "the window stays open"
        );
    });
}

#[gpui::test]
fn a_session_chooser_that_could_not_be_shown_stops_the_pending_quit(cx: &mut TestAppContext) {
    use super::sessions::Chooser;
    use sn_ui_core::app_glue::Confirm;
    let (view, cx) = frame(cx);
    view.update(cx, |f, _| {
        f.after_save = Some(Confirm::Quit);
        f.chosen(Chooser::SaveSessionAs, Err("portal missing".into()));
        assert_eq!(f.after_save, None, "the quit does not go ahead");
        let mb = f.msgbox.as_ref().expect("the user is told");
        assert!(mb.text.contains("portal missing"), "{}", mb.text);
        assert_eq!(f.msg_action, MsgAction::None);
    });
}

/// Window position of `(x, y)` in the SA Open file window's client area.
fn loader_at(
    view: &gpui::Entity<Frame>,
    cx: &mut VisualTestContext,
    x: i32,
    y: i32,
) -> Point<gpui::Pixels> {
    let r = view.read_with(cx, |f, _| f.loader.as_ref().unwrap().rect);
    let c = crate::win95::dialog::dialog_client(r.w, r.h);
    p(r.x + c.x + x, r.y + c.y + y)
}

#[gpui::test]
fn the_open_file_window_answers_clicks(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let a = wav(dir.path(), "a.wav", &tone(8000.0, 1000));
    let b = wav(dir.path(), "b.wav", &tone(8000.0, 1000));
    let (view, cx) = frame(cx);
    view.update(cx, |f, cx| {
        f.open_loader(Some(a.clone()));
        cx.notify();
    });
    cx.run_until_parked();
    // The second row of the list (list view: rows of 17 px from the top).
    let (lx, ly) = (0, 28);
    let at = loader_at(&view, cx, lx + 30, ly + 2 + 17 + 8);
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        let l = f.loader.as_ref().unwrap();
        assert_eq!(
            l.selected_entry().map(|e| e.path.clone()),
            Some(b.clone()),
            "a click selects the row"
        );
    });
    let at = loader_at(&view, cx, 709 + 20, 272 + 10);
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |f, _| f.loader.is_none()),
        "Cancel closes the window"
    );
}

#[gpui::test]
fn the_open_file_window_opened_from_the_menu_answers_clicks(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    cx.simulate_click(p(3, 9), Modifiers::none());
    let item = p(30, bar_h() + 3 + 8);
    cx.simulate_mouse_move(item, None, Modifiers::none());
    cx.simulate_click(item, Modifiers::none());
    cx.run_until_parked();
    assert!(view.read_with(cx, |f, _| f.loader.is_some()));
    let at = loader_at(&view, cx, 709 + 20, 272 + 10);
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |f, _| f.loader.is_none()),
        "Cancel closes the window"
    );
}

#[gpui::test]
fn the_open_file_window_of_a_start_file_answers_clicks(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let a = wav(dir.path(), "a.wav", &tone(8000.0, 1000));
    let b = wav(dir.path(), "b.wav", &tone(8000.0, 1000));
    let setup = Setup {
        stored: crate::store::Stored::default(),
        store: None,
        file: Some(a.clone()),
    };
    let (view, cx) = cx.add_window_view(move |w, cx| Frame::with_setup(setup, w, cx));
    cx.run_until_parked();
    let at = loader_at(&view, cx, 30, 28 + 2 + 17 + 8);
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        let l = f.loader.as_ref().unwrap();
        assert_eq!(
            l.selected_entry().map(|e| e.path.clone()),
            Some(b.clone()),
            "a click selects the row"
        );
    });
    let at = loader_at(&view, cx, 709 + 20, 272 + 10);
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |f, _| f.loader.is_none()),
        "Cancel closes the window"
    );
}

#[gpui::test]
fn a_windows_style_change_keeps_every_client_area(cx: &mut TestAppContext) {
    use sn_ui_core::winstyle::WinStyle;
    let (view, cx) = frame(cx);
    add_signal(&view, cx, "a.wav", 8000.0);
    let client_of = |f: &Frame| {
        let c = f.mdi.children().last().unwrap();
        let r = crate::win95::mdi::client_rect(c.rect.w, c.rect.h);
        (r.w, r.h)
    };
    let before = view.read_with(cx, |f, _| client_of(f));
    let vista = WinStyle::ALL
        .iter()
        .position(|s| *s == WinStyle::Vista)
        .unwrap() as u32;
    view.update_in(cx, |f, window, cx| {
        f.command(cmd::STYLE + vista, window, cx);
    });
    view.read_with(cx, |f, _| {
        assert_eq!(f.prefs.look.style, WinStyle::Vista);
        assert_eq!(crate::win95::style::active().style, WinStyle::Vista);
        assert_eq!(client_of(f), before, "the form keeps its size");
        let (cw, ch) = f.mdi.client;
        for c in f.mdi.children() {
            assert!(c.rect.right() <= cw && c.rect.bottom() <= ch, "{c:?}");
        }
    });
    view.update_in(cx, |f, window, cx| f.command(cmd::STYLE, window, cx));
    view.read_with(cx, |f, _| {
        assert_eq!(f.prefs.look.style, WinStyle::Win95);
        assert_eq!(client_of(f), before, "and back");
    });
}

#[cfg(debug_assertions)]
#[test]
fn the_debug_look_rejects_unknown_parts() {
    use sn_ui_core::winstyle::{Look, WinStyle, XpColor};
    let base = Look::default();
    let l = super::debug_look("xp-olive", base);
    assert_eq!(
        l.map(|l| (l.style, l.xp)),
        Some((WinStyle::Xp, XpColor::Olive))
    );
    assert_eq!(
        super::debug_look("10-dark-#FF8C00", base).map(|l| (l.dark, l.accent)),
        Some((true, 0xFF_8C_00))
    );
    // The accent goes to the field the look draws, whatever the order.
    for v in ["gnome-#123456-yaru", "gnome-yaru-#123456"] {
        assert_eq!(
            super::debug_look(v, base).map(|l| (l.yaru_accent, l.linux_accent)),
            Some((0x12_34_56, base.linux_accent)),
            "{v}"
        );
    }
    assert_eq!(
        super::debug_look("kde-#654321", base).map(|l| (l.breeze_accent, l.linux_accent)),
        Some((0x65_43_21, base.linux_accent))
    );
    assert_eq!(super::debug_look("xp-olvie", base), None, "a typo");
    assert_eq!(super::debug_look("10-#GG0000", base), None, "bad hex");
    assert_eq!(super::debug_look("12", base), None, "no such style");
    assert_eq!(
        super::debug_look("11-dark", base).map(|l| (l.style, l.dark)),
        Some((WinStyle::Win11, true))
    );
}

#[test]
fn a_maximized_mac_child_keeps_the_lights_at_the_left_in_mac_order() {
    use crate::win95::style;
    use sn_ui_core::winstyle::{Look, WinStyle};
    for st in [WinStyle::MacAqua, WinStyle::MacModern] {
        style::set(
            Look {
                style: st,
                ..Look::default()
            },
            ThemeChoice::Classic,
        );
        let m = style::metrics();
        let (min, max, close) = super::buttons_at(800);
        assert_eq!(close.x, m.btn_margin, "{st:?}");
        assert_eq!(min.x - close.right(), m.btn_gaps.1, "{st:?}");
        assert_eq!(max.x - min.right(), m.btn_gaps.0, "{st:?}");
        assert!(
            super::maximized_bar_x0() > max.right(),
            "menus after the lights"
        );
        assert!(close.bottom() <= bar_h());
    }
    // Motif has no close button in the menu bar either: its window menu
    // (the icon at the left) closes.
    style::set(
        Look {
            style: WinStyle::LinuxMotif,
            ..Look::default()
        },
        ThemeChoice::Classic,
    );
    let (_, max, close) = super::buttons_at(800);
    assert!(close.is_empty(), "{close:?}");
    assert_eq!(max.right(), 800 - 2);
    style::set(Look::WIN95, ThemeChoice::Classic);
    let (min, max, close) = super::buttons_at(800);
    assert!(min.x < max.x && max.x < close.x, "Windows keeps its order");
    assert_eq!(super::maximized_bar_x0(), 20);
}
