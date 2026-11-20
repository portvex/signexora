//! Behaviour tests of the PhasePlane window on GPUI's test platform: the mouse
//! and keyboard are simulated inside the test app only.

use std::time::{Duration, Instant};

use gpui::{
    Entity, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Point, TestAppContext,
    VisualTestContext, point, px,
};
use serde_json::json;
use sn_core::{Measurement, MeasurementKind, NodeId, Operation};
use sn_ui_core::markers::Markers;
use sn_ui_core::phase::Mode;
use sn_ui_core::phase_plot::FORM;

use super::corrector::{AUTO, PANEL, PLOT, RESET, SAVE_AS};
use super::load::{LIST, buttons};
use super::viewer::{ANGLES, CLEAR, CONSTELLATION, LIFE, LOAD_FILE, NONSTOP, OUTPUT_DEM, START};
use crate::app::Frame;
use crate::app::model::WinKind;
use crate::win95::mdi;
use crate::win95::menu_view::bar_h;

type Px = Point<gpui::Pixels>;

fn frame(cx: &mut TestAppContext) -> (Entity<Frame>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(Frame::new);
    cx.run_until_parked();
    (view, cx)
}

/// Draw frames until `done` holds (jobs run on other threads), at most 30 s.
fn wait_for(
    view: &Entity<Frame>,
    cx: &mut VisualTestContext,
    done: impl Fn(&Frame) -> bool,
) -> bool {
    let start = Instant::now();
    loop {
        view.update(cx, |f, cx| {
            f.poll(cx);
            cx.notify();
        });
        cx.run_until_parked();
        if view.read_with(cx, |f, _| done(f)) {
            return true;
        }
        if start.elapsed() > Duration::from_secs(30) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Load the QPSK test recording with V and H markers and Sc and Br measured;
/// returns its node and sonogram window.
fn load(view: &Entity<Frame>, cx: &mut VisualTestContext) -> (NodeId, u32) {
    let node = view.update(cx, |f, cx| {
        let op = Operation::new("load", json!({"path": "/data/qpsk.wav"}));
        let id = f.on_loaded("qpsk.wav".into(), op, super::psk_signal().unwrap(), &[]);
        f.ui.get_mut(&id).unwrap().markers = Markers {
            v: Some((0.0, 16_000.0)),
            h: Some((1000.0, 2600.0)),
            ..Markers::default()
        };
        for (kind, value) in [
            (MeasurementKind::Carrier, 1800.25),
            (MeasurementKind::BaudRate, 600.0),
        ] {
            let m = Measurement {
                value,
                uncertainty: None,
                source: Some(id),
                tool: "test".into(),
            };
            f.measures.record(kind, m).unwrap();
        }
        cx.notify();
        id
    });
    cx.run_until_parked();
    wait_for(view, cx, |f| f.jobs.is_empty());
    let win = view.read_with(cx, |f, _| f.find_window(node, WinKind::Sonogram).unwrap());
    (node, win)
}

/// Open the PhasePlane window of `node` from window `caller`.
fn open_phase(view: &Entity<Frame>, cx: &mut VisualTestContext, node: NodeId, caller: u32) -> u32 {
    let win = view.update(cx, |f, cx| {
        let w = f.open_window(node, WinKind::Phase, Some(caller)).unwrap();
        cx.notify();
        w
    });
    cx.run_until_parked();
    win
}

/// Window position of form point `(x, y)` of window `win`.
fn form_at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) -> Px {
    view.read_with(cx, |f, _| {
        let r = f.mdi.shown_rect(win).unwrap();
        let c = mdi::client_rect(r.w, r.h);
        point(
            px((2 + r.x + c.x + x) as f32),
            px((bar_h() + 2 + r.y + c.y + y) as f32),
        )
    })
}

fn click(cx: &mut VisualTestContext, p: Px) {
    cx.simulate_click(p, Modifiers::none());
    cx.run_until_parked();
}

fn click_rect(
    view: &Entity<Frame>,
    cx: &mut VisualTestContext,
    win: u32,
    r: crate::win95::geom::Rect,
) {
    let p = form_at(view, cx, win, r.x + r.w / 2, r.y + r.h / 2);
    click(cx, p);
}

/// A check box is clicked on its box.
fn click_check(
    view: &Entity<Frame>,
    cx: &mut VisualTestContext,
    win: u32,
    r: crate::win95::geom::Rect,
) {
    let p = form_at(view, cx, win, r.x + 6, r.y + r.h / 2);
    click(cx, p);
}

fn right_click(cx: &mut VisualTestContext, p: Px) {
    cx.simulate_mouse_down(p, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(p, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
}

fn double_click(cx: &mut VisualTestContext, p: Px) {
    click(cx, p);
    cx.simulate_event(MouseDownEvent {
        position: p,
        modifiers: Modifiers::none(),
        button: MouseButton::Left,
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_event(MouseUpEvent {
        position: p,
        modifiers: Modifiers::none(),
        button: MouseButton::Left,
        click_count: 2,
    });
    cx.run_until_parked();
}

/// Window position of point `(x, y)` of the `LoadFile` dialog's client area.
fn dialog_at(view: &Entity<Frame>, cx: &mut VisualTestContext, x: i32, y: i32) -> Px {
    view.read_with(cx, |f, _| {
        let d = f.phase.load.as_ref().unwrap().rect;
        let c = crate::win95::dialog::dialog_client(d.w, d.h);
        point(px((d.x + c.x + x) as f32), px((d.y + c.y + y) as f32))
    })
}

/// Pick the first working file in the `LoadFile` dialog of window `win`.
fn load_file(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32) {
    click_rect(view, cx, win, LOAD_FILE);
    assert!(
        view.read_with(cx, |f, _| f.phase.load.is_some()),
        "the dialog opened"
    );
    let p = dialog_at(view, cx, LIST.x + 20, LIST.y + 6);
    click(cx, p);
    let (ok, _) = buttons();
    let p = dialog_at(view, cx, ok.x + ok.w / 2, ok.y + ok.h / 2);
    click(cx, p);
}

fn mode(view: &Entity<Frame>, cx: &mut VisualTestContext, node: NodeId) -> Mode {
    view.read_with(cx, |f, _| f.phase.states[&node].mode)
}

#[gpui::test]
fn the_form_opens_fixed_size_in_spectrum_mode_pre_filled(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    view.read_with(cx, |f, _| {
        let c = f.mdi.get(win).unwrap();
        assert!(!c.resizable, "SA's form does not scale");
        assert_eq!(
            c.title, "[TempDir] qpsk(0).wav",
            "titled as the working file"
        );
        assert_eq!(f.client_of(win), FORM);
        let st = &f.phase.states[&node];
        assert_eq!(st.mode, Mode::Spectrum);
        let pt = |s: &str| s.replace(',', ".");
        assert_eq!(pt(&st.form.carrier), "1800.25", "Carrier from the last Sc");
        assert_eq!(st.form.br, "600", "BR from the last Br");
        // Start needs the phase plane mode.
        assert!(!st.play.running);
    });
    click_rect(&view, cx, win, START);
    assert!(!view.read_with(cx, |f, _| f.phase.states[&node].play.running));
    // The spectrum of the V span arrives.
    assert!(wait_for(&view, cx, |f| f.phase.states[&node]
        .hires
        .is_some()));
}

#[gpui::test]
fn controls_follow_sa_order(cx: &mut TestAppContext) {
    // Upper half (6.1, M-p16 numbering): plot, scale bar, Mode, N, (B),
    // Corr-OFF/ON, SaveAs, Blocks, Auto, Break, Reset, text panel.
    assert!(PLOT.right() < SAVE_AS.x && SAVE_AS.right() < AUTO.x && AUTO.right() < PANEL.x);
    assert!(AUTO.y < RESET.y && PANEL.right() <= FORM.0);
    // Lower half (6.2, R278/a2): constellation, the etched column, PLL and the
    // check boxes, the quality and Angles column, the sliders, the buttons.
    assert!(CONSTELLATION.right() < NONSTOP.x && NONSTOP.right() < LIFE.right());
    assert!(LIFE.right() < ANGLES.x && ANGLES.y < OUTPUT_DEM.y);
    assert!(START.y < CLEAR.y && CLEAR.y < LOAD_FILE.y && START.x > OUTPUT_DEM.right());
    assert!(PLOT.bottom() < CONSTELLATION.y);
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    load_file(&view, cx, win);
    assert_eq!(mode(&view, cx, node), Mode::Phase);
    view.read_with(cx, |f, _| {
        let st = &f.phase.states[&node];
        assert_eq!(st.loaded, Some(node));
        assert_eq!(st.loaded_title, "[TempDir] qpsk(0).wav");
        assert!(f.phase.load.is_none(), "the dialog closed");
    });
}

#[gpui::test]
fn start_runs_and_break_stops(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    load_file(&view, cx, win);
    view.update(cx, |f, _| {
        let st = f.phase.states.get_mut(&node).unwrap();
        st.form.order = 4;
    });
    // NonStop off: the run stops at the end of the file.
    click_check(&view, cx, win, NONSTOP);
    assert!(!view.read_with(cx, |f, _| f.phase.states[&node].form.nonstop));
    click_rect(&view, cx, win, START);
    assert!(view.read_with(cx, |f, _| f.phase.states[&node].play.running));
    assert!(
        view.read_with(cx, |f, _| !f.jobs_stopped_by_closing(win).is_empty()),
        "closing now would stop the run"
    );
    assert!(wait_for(&view, cx, |f| f.phase.states[&node]
        .shown()
        .is_some()));
    // LoadFile is disabled while running; the button reads Break.
    click_rect(&view, cx, win, LOAD_FILE);
    assert!(view.read_with(cx, |f, _| f.phase.load.is_none()));
    click_rect(&view, cx, win, START);
    view.read_with(cx, |f, _| {
        let st = &f.phase.states[&node];
        assert!(!st.play.running, "Break stops");
        let q = st.shown().and_then(|s| s.quality).unwrap_or(0.0);
        assert!(q > 5.0, "a clean QPSK constellation: {q}");
    });
    // Start again plays from the beginning to the end of the file.
    click_rect(&view, cx, win, START);
    assert!(wait_for(&view, cx, |f| !f.phase.states[&node].play.running));
    let (pos, len) = view.read_with(cx, |f, _| {
        let st = &f.phase.states[&node];
        (
            st.play.pos,
            st.shown().map_or(0, sn_ui_core::phase::Shown::len),
        )
    });
    assert_eq!(pos, len);
}

#[gpui::test]
fn angle_markers_drop_up_to_ten_and_a_right_click_clears(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    load_file(&view, cx, win);
    click_check(&view, cx, win, ANGLES);
    assert!(view.read_with(cx, |f, _| f.phase.states[&node].form.angles));
    // A click right of the middle line's top half: a positive angle near 45.
    let p = form_at(&view, cx, win, PLOT.x + PLOT.w * 3 / 4, PLOT.y + 20);
    click(cx, p);
    let marks = view.read_with(cx, |f, _| f.phase.states[&node].marks.clone());
    assert_eq!(marks.len(), 1);
    let first = marks.first().copied().unwrap();
    assert!((first - 45.0).abs() < 1.0, "{first}");
    // Below the middle line the angle is negative.
    let p = form_at(&view, cx, win, PLOT.x + PLOT.w / 2, PLOT.y + PLOT.h - 20);
    click(cx, p);
    let second = view.read_with(cx, |f, _| f.phase.states[&node].marks[1]);
    assert!((second + 90.0).abs() < 1.0, "{second}");
    for k in 0..12 {
        let p = form_at(&view, cx, win, PLOT.x + 10 + 30 * k, PLOT.y + 30);
        click(cx, p);
    }
    let lines = view.read_with(cx, |f, _| {
        let st = &f.phase.states[&node];
        assert_eq!(st.marks.len(), 10, "at most ten markers");
        st.panel_lines()
    });
    let pt = |s: &str| s.replace(',', ".");
    assert_eq!(lines.len(), 10);
    assert!(
        pt(&lines[1].1).starts_with("1 -90.00 1-0="),
        "{:?}",
        lines[1]
    );
    let p = form_at(&view, cx, win, PLOT.x + 100, PLOT.y + 100);
    right_click(cx, p);
    assert!(view.read_with(cx, |f, _| f.phase.states[&node].marks.is_empty()));
}

#[gpui::test]
fn isd_auto_makes_a_corrected_signal_without_a_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    load_file(&view, cx, win);
    let windows = view.read_with(cx, |f, _| f.signal_windows().len());
    click_rect(&view, cx, win, AUTO);
    assert!(view.read_with(cx, |f, _| f.phase.states[&node].isd_job.is_some()));
    assert!(wait_for(&view, cx, |f| f.phase.states[&node]
        .corrected
        .is_some()));
    let corrected = view.read_with(cx, |f, _| {
        let c = f.phase.states[&node].corrected.unwrap();
        assert_eq!(f.name_of(c), "qpsk(0)(isd1)");
        assert_eq!(f.signal_windows().len(), windows, "SA opens no window");
        assert_eq!(f.phase.states[&node].source(), Some(c), "Corr-ON");
        c
    });
    // Reset switches the corrector off; the corrected signal leaves.
    click_rect(&view, cx, win, RESET);
    view.read_with(cx, |f, _| {
        assert_eq!(f.phase.states[&node].corrected, None);
        assert!(f.discarded.contains(&corrected));
    });
}

#[gpui::test]
fn the_load_dialog_takes_a_double_click_and_escape(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    click_rect(&view, cx, win, LOAD_FILE);
    cx.simulate_keystrokes("escape");
    assert!(view.read_with(cx, |f, _| f.phase.load.is_none()));
    assert_eq!(mode(&view, cx, node), Mode::Spectrum);
    click_rect(&view, cx, win, LOAD_FILE);
    let p = dialog_at(&view, cx, LIST.x + 20, LIST.y + 6);
    double_click(cx, p);
    assert_eq!(mode(&view, cx, node), Mode::Phase);
}

#[gpui::test]
fn output_dem_writes_the_code_table(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    load_file(&view, cx, win);
    click_check(&view, cx, win, OUTPUT_DEM);
    click_rect(&view, cx, win, START);
    assert!(wait_for(&view, cx, |f| f.phase.states[&node].play.pos > 100));
    let lines = view.read_with(cx, |f, _| f.phase.states[&node].panel_lines());
    let texts: Vec<&str> = lines.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(texts.first(), Some(&"View mode: diff 0"));
    assert!(texts.contains(&"The coding positions of angles:"));
    assert!(texts.contains(&"Data:"));
    // Closing the window stops the run and lets go of the loaded file.
    view.update(cx, |f, _| f.close_window_now(win));
    view.read_with(cx, |f, _| {
        assert!(f.phase.wins.is_empty());
        assert!(!f.phase.states[&node].play.running);
        assert_eq!(f.phase.states[&node].loaded, None);
    });
}

#[gpui::test]
fn ok_after_the_selected_file_closed_says_so(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    click_rect(&view, cx, win, LOAD_FILE);
    let p = dialog_at(&view, cx, LIST.x + 20, LIST.y + 6);
    click(cx, p);
    view.update(cx, |f, cx| {
        assert_eq!(f.phase.load.as_ref().and_then(|d| d.sel), Some(node));
        // Its sonogram window closes while the dialog is open.
        f.close_window_now(sono);
        f.phase_load_ok();
        assert_eq!(f.phase.load.as_ref().and_then(|d| d.sel), None);
        assert_eq!(
            f.phase.states[&node].error.as_deref(),
            Some(sn_ui_core::t!("pp-load-gone").as_str())
        );
        cx.notify();
    });
    assert_eq!(mode(&view, cx, node), Mode::Spectrum);
}

#[gpui::test]
fn the_load_dialog_takes_the_arrow_keys(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    click_rect(&view, cx, win, LOAD_FILE);
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    assert!(view.read_with(cx, |f, _| f.phase.load.is_none()));
    assert_eq!(mode(&view, cx, node), Mode::Phase);
}

#[gpui::test]
fn the_state_of_a_signal_that_left_the_session_is_freed(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, sono) = load(&view, cx);
    let win = open_phase(&view, cx, node, sono);
    assert!(view.read_with(cx, |f, _| f.phase.states.contains_key(&node)));
    view.update(cx, |f, cx| {
        f.close_window_now(win);
        f.close_window_now(sono);
        assert!(f.discarded.contains(&node));
        cx.notify();
    });
    cx.run_until_parked();
    assert!(view.read_with(cx, |f, _| !f.phase.states.contains_key(&node)));
}
