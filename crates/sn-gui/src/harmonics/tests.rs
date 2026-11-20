//! Behaviour tests of the harmonics (`(q)`) window on GPUI's test platform:
//! the mouse is simulated inside the test app only.

use std::time::{Duration, Instant};

use gpui::{Entity, Modifiers, MouseButton, Point, TestAppContext, VisualTestContext, point, px};
use serde_json::json;
use sn_core::{MeasurementKind, NodeId, Operation};
use sn_dsp::ops::Edit;
use sn_ui_core::markers::Markers;
use sn_ui_core::sono_tools::Button;

use super::{Ctrl, places};
use crate::app::Frame;
use crate::app::menus::cmd;
use crate::app::model::WinKind;
use crate::app::sono::Layout;
use crate::win95::bars::TOOL;
use crate::win95::mdi;
use crate::win95::menu_view::bar_h;

type Px = Point<gpui::Pixels>;

fn frame(cx: &mut TestAppContext) -> (Entity<Frame>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(Frame::new);
    cx.run_until_parked();
    (view, cx)
}

/// Wait for every background job, then let the frame collect the results.
fn settle(view: &Entity<Frame>, cx: &mut VisualTestContext) {
    let start = Instant::now();
    loop {
        let done = view.read_with(cx, |f, _| f.jobs.iter().all(|j| j.handle.is_finished()));
        if done {
            view.update(cx, |f, cx| {
                f.poll(cx);
                cx.notify();
            });
            cx.run_until_parked();
        }
        let busy = view.read_with(cx, |f, _| !f.jobs.is_empty());
        if !busy || start.elapsed() > Duration::from_secs(30) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    cx.run_until_parked();
}

/// The QPSK recording with H markers around its carrier, and its `(q)`
/// window made by `x^n`.
fn harmonics(view: &Entity<Frame>, cx: &mut VisualTestContext) -> (NodeId, NodeId, u32) {
    let src = view.update(cx, |f, cx| {
        let op = Operation::new("load", json!({"path": "/data/qpsk.wav"}));
        let signal = crate::app::phase::psk_signal().unwrap();
        let id = f.on_loaded("qpsk.wav".into(), op, signal, &[]);
        f.ui.get_mut(&id).unwrap().markers = Markers {
            v: Some((0.0, 16_000.0)),
            h: Some((1700.0, 1900.0)),
            ..Markers::default()
        };
        cx.notify();
        id
    });
    settle(view, cx);
    let sono = view.read_with(cx, |f, _| f.find_window(src, WinKind::Sonogram).unwrap());
    view.update(cx, |f, _| f.press_button(sono, Button::Power));
    settle(view, cx);
    let (q, win) = view.read_with(cx, |f, _| {
        f.signal_windows()
            .into_iter()
            .find(|(w, _, _)| *w != sono)
            .map(|(w, n, _)| (n, w))
            .unwrap()
    });
    view.update(cx, |f, cx| {
        f.mdi.activate(win);
        cx.notify();
    });
    cx.run_until_parked();
    (src, q, win)
}

/// Window position of the middle of harmonics control `c` of window `win`.
fn ctrl_at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, c: Ctrl) -> Px {
    view.read_with(cx, |f, _| {
        let r = f.mdi.shown_rect(win).unwrap();
        let cr = mdi::client_rect(r.w, r.h);
        let (cw, ch) = f.client_of(win);
        let l = Layout::new(cw, ch);
        let (_, _, _, ctrls) = places(l.bottom.x + 2);
        let (_, x, w) = ctrls.into_iter().find(|(k, _, _)| *k == c).unwrap();
        let y = l.bottom.y + (l.bottom.h - TOOL) / 2 + TOOL / 2;
        let x = if c == Ctrl::Control {
            x + w - 7
        } else {
            x + w / 2
        };
        point(
            px((2 + r.x + cr.x + x) as f32),
            px((bar_h() + 2 + r.y + cr.y + y) as f32),
        )
    })
}

fn click(cx: &mut VisualTestContext, p: Px) {
    cx.simulate_click(p, Modifiers::none());
    cx.run_until_parked();
}

#[gpui::test]
fn x_n_opens_the_q_window_at_power_2_with_the_carrier(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (src, q, win) = harmonics(&view, cx);
    view.read_with(cx, |f, _| {
        assert_eq!(f.mdi.get(win).unwrap().title, "[TempDir] qpsk(0)(q).wav");
        let h = &f.harmonics.states[&q];
        assert_eq!(h.power, 2);
        assert_eq!(h.carrier, "1800", "the middle of the input's H markers");
        assert_eq!(f.graph.get(q).unwrap().inputs, vec![src]);
    });
}

#[gpui::test]
fn carrier_steps_and_the_control_line(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, q, win) = harmonics(&view, cx);
    for c in [Ctrl::Down, Ctrl::DownFast, Ctrl::Up] {
        let p = ctrl_at(&view, cx, win, c);
        click(cx, p);
    }
    assert_eq!(
        view.read_with(cx, |f, _| f.harmonics.states[&q].carrier.clone()),
        "1790"
    );
    assert_eq!(
        view.read_with(cx, |f, _| f.harmonics_line(q)),
        None,
        "no line before Control"
    );
    let p = ctrl_at(&view, cx, win, Ctrl::Control);
    click(cx, p);
    let line = view.read_with(cx, |f, _| f.harmonics_line(q)).unwrap();
    // SigNexora raises the signal itself: the carrier's line is at N times it.
    assert!((line - 3580.0).abs() < 1e-6, "{line}");
    let p = ctrl_at(&view, cx, win, Ctrl::Control);
    click(cx, p);
    assert_eq!(view.read_with(cx, |f, _| f.harmonics_line(q)), None);
}

#[gpui::test]
fn st_n_shows_the_new_power_in_the_same_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (src, q, win) = harmonics(&view, cx);
    for _ in 0..2 {
        let p = ctrl_at(&view, cx, win, Ctrl::PowerUp);
        click(cx, p);
    }
    assert_eq!(view.read_with(cx, |f, _| f.harmonics.states[&q].power), 4);
    let windows = view.read_with(cx, |f, _| f.signal_windows().len());
    let p = ctrl_at(&view, cx, win, Ctrl::Compute);
    click(cx, p);
    view.read_with(cx, |f, _| {
        assert_eq!(f.harmonics.states[&q].pending, Some(4));
        assert!(f.node_progress(q).is_running(), "the window shows the job");
    });
    // A second press while it runs starts nothing.
    let jobs = view.read_with(cx, |f, _| f.jobs.len());
    let p = ctrl_at(&view, cx, win, Ctrl::Compute);
    click(cx, p);
    assert_eq!(view.read_with(cx, |f, _| f.jobs.len()), jobs);
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        let shown = f.node_of(win).unwrap();
        assert_ne!(shown, q, "the window shows the new result");
        assert!(f.discarded.contains(&q), "the old result left the session");
        assert_eq!(f.signal_windows().len(), windows);
        assert!(matches!(
            sn_ui_core::lineage::edit_of(f.graph.get(shown).unwrap()),
            Some(Edit::Power { n: 4 })
        ));
        assert_eq!(f.graph.get(shown).unwrap().inputs, vec![src]);
        let h = &f.harmonics.states[&shown];
        assert_eq!((h.power, h.pending), (4, None));
    });
}

#[gpui::test]
fn measurement_of_freq_records_sc(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (src, q, win) = harmonics(&view, cx);
    // H markers around the carrier's line at twice 1800 Hz.
    view.update(cx, |f, cx| {
        f.ui.get_mut(&q).unwrap().markers = Markers {
            v: Some((0.0, 30_000.0)),
            h: Some((3500.0, 3700.0)),
            ..Markers::default()
        };
        cx.notify();
    });
    cx.run_until_parked();
    let p = view.read_with(cx, |f, _| {
        let r = f.mdi.shown_rect(win).unwrap();
        let c = mdi::client_rect(r.w, r.h);
        point(
            px((2 + r.x + c.x + 300) as f32),
            px((bar_h() + 2 + r.y + c.y + 120) as f32),
        )
    });
    cx.simulate_mouse_down(p, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(p, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |f, _| f.sono.popup_win), Some(win));
    view.update(cx, |f, cx| f.sono_command(cmd::SW_MEASURE, cx));
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        // The line found between the H markers, divided by the power: Sc on
        // the input's axis (QPSK squared has no clean line, so only the band
        // is checked).
        let m = f.measures.latest(&MeasurementKind::Carrier).unwrap();
        assert_eq!(m.source, Some(q));
        assert!((1750.0..=1850.0).contains(&m.value), "{}", m.value);
        // PhasePlane's Carrier is pre-filled from it.
        let sc = f.sc_for(src).unwrap();
        assert!((sc.value - m.value).abs() < 1e-9);
    });
}
