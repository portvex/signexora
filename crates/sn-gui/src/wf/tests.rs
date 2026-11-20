//! Behaviour tests of the WF window and its VMW mode on GPUI's test platform:
//! the mouse is simulated inside the test app only.

use std::time::{Duration, Instant};

use gpui::{
    Entity, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Point, TestAppContext,
    VisualTestContext, point, px,
};
use serde_json::json;
use sn_core::{NodeId, Operation};
use sn_ui_core::markers::Markers;
use sn_ui_core::vmw_tools::ts_text;
use sn_ui_core::wf_tools::{self, WF_BUTTONS, WfButton, WfSlot};

use super::bottom::slot_places;
use super::vmw::big_line;
use crate::app::Frame;
use crate::app::menus::cmd;
use crate::app::model::WinKind;
use crate::win95::bars::TOOL;
use crate::win95::geom::Rect;
use crate::win95::mdi;
use crate::win95::menu::Kind;
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
        view.update(cx, |f, cx| {
            f.poll(cx);
            cx.notify();
        });
        cx.run_until_parked();
        let busy = view.read_with(cx, |f, _| !f.jobs.is_empty() || f.wf_busy());
        if !busy {
            break;
        }
        assert!(
            start.elapsed() <= Duration::from_secs(30),
            "jobs still running after 30 s"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    cx.run_until_parked();
}

/// Load the generated MFSK-8 test recording and open its WF window from the
/// sonogram popup's `Wave Form module`; returns the node and the WF window.
fn open_wf(view: &Entity<Frame>, cx: &mut VisualTestContext) -> (NodeId, u32) {
    let node = view.update(cx, |f, cx| {
        let op = Operation::new("load", json!({"path": "/data/mfsk8.wav"}));
        let id = f.on_loaded(
            "mfsk8.wav".into(),
            op,
            crate::app::sono::test_signal().unwrap(),
            &[],
        );
        cx.notify();
        id
    });
    settle(view, cx);
    view.update_in(cx, |f, window, cx| {
        let sono = f.find_window(node, WinKind::Sonogram).unwrap();
        f.open_sono_popup(sono, (300, 200));
        f.command(cmd::SW_WAVEFORM, window, cx);
        close_popup(f);
    });
    settle(view, cx);
    let win = view.read_with(cx, |f, _| f.find_window(node, WinKind::Waveform).unwrap());
    (node, win)
}

/// The client origin of window `win` in window coordinates.
fn origin(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32) -> (i32, i32) {
    view.read_with(cx, |f, _| {
        let r = f.mdi.shown_rect(win).unwrap();
        let c = mdi::client_rect(r.w, r.h);
        (2 + r.x + c.x, bar_h() + 2 + r.y + c.y)
    })
}

/// Window position of client point `(x, y)` of window `win`.
fn at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) -> Px {
    let (ox, oy) = origin(view, cx, win);
    point(px((ox + x) as f32), px((oy + y) as f32))
}

/// Window position of plot point `(x, y)` of WF window `win`.
fn plot_at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) -> Px {
    let plot = view.read_with(cx, |f, _| f.wf.wins[&win].plot);
    at(view, cx, win, plot.x + x, plot.y + y)
}

/// The middle of control `name` of window `win`.
fn hot(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, name: &str) -> Rect {
    view.read_with(cx, |f, _| f.wf.wins[&win].hot[name])
}

/// Window position of the middle of bottom toolbar button `b` of window `win`.
fn button_at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, b: WfButton) -> Px {
    let (cw, ch) = view.read_with(cx, |f, _| f.client_of(win));
    let bottom = Rect::new(0, ch - super::BOT_H, cw, super::BOT_H);
    let x = slot_places(bottom.x + 2)
        .into_iter()
        .find(|(s, _, _)| *s == WfSlot::Button(b))
        .map(|(_, x, _)| x)
        .unwrap();
    let y = bottom.y + (bottom.h - TOOL) / 2;
    at(view, cx, win, x + TOOL / 2, y + TOOL / 2)
}

/// Close the popup menu a test opened (its command ran directly).
fn close_popup(f: &mut Frame) {
    f.popup = None;
    f.popup_track.close();
}

fn click(cx: &mut VisualTestContext, p: Px) {
    cx.simulate_click(p, Modifiers::none());
    cx.run_until_parked();
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

fn drag(cx: &mut VisualTestContext, from: Px, to: Px) {
    let none = Modifiers::none();
    cx.simulate_mouse_down(from, MouseButton::Left, none);
    let mid = point((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
    cx.simulate_mouse_move(mid, MouseButton::Left, none);
    cx.simulate_mouse_move(to, MouseButton::Left, none);
    cx.simulate_mouse_up(to, MouseButton::Left, none);
    cx.run_until_parked();
}

/// Bring window `win` to the front (and draw the frame again).
fn activate(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32) {
    view.update(cx, |f, cx| {
        f.mdi.activate(win);
        cx.notify();
    });
    cx.run_until_parked();
}

fn click_button(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, b: WfButton) {
    let p = button_at(view, cx, win, b);
    click(cx, p);
}

fn markers(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32) -> Markers {
    view.read_with(cx, |f, _| f.wf.wins[&win].markers)
}

/// The titles of the signal windows, sorted.
fn titles(view: &Entity<Frame>, cx: &mut VisualTestContext) -> Vec<(String, WinKind)> {
    let mut t: Vec<(String, WinKind)> = view.read_with(cx, |f, _| {
        f.signal_windows()
            .into_iter()
            .filter_map(|(id, _, k)| f.mdi.get(id).map(|c| (c.title.clone(), k)))
            .collect()
    });
    t.sort_by(|a, b| a.0.cmp(&b.0));
    t
}

/// The labels of the open popup menu (separators as `-`).
fn popup_labels(view: &Entity<Frame>, cx: &mut VisualTestContext) -> Vec<String> {
    view.read_with(cx, |f, _| {
        f.popup
            .as_ref()
            .map(|p| {
                p.items
                    .iter()
                    .map(|i| match i.kind {
                        Kind::Separator => "-".to_owned(),
                        _ => crate::win95::text::mnemonic(&i.label).text,
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Sweep a V/H selection in the WF plot.
fn sweep(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32) {
    let a = plot_at(view, cx, win, 200, 40);
    let b = plot_at(view, cx, win, 400, 120);
    drag(cx, a, b);
}

#[gpui::test]
fn the_wave_form_module_opens_a_wf_window_with_sa_markers(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open_wf(&view, cx);
    let title = view.read_with(cx, |f, _| f.mdi.get(win).unwrap().title.clone());
    assert_eq!(title, "[TempDir] mfsk8(0).wav", "the caller's title");
    let len = view.read_with(cx, |f, _| f.graph.get(node).unwrap().signal.len());
    let m = markers(&view, cx, win);
    // SA: the V pair collapsed in the middle, the H pair at the edges.
    assert_eq!(m.v, Some((len as f64 / 2.0, len as f64 / 2.0)));
    assert!(m.time_range().is_none() && m.freq_range().is_some());
    view.read_with(cx, |f, _| {
        let ww = &f.wf.wins[&win];
        assert!(
            ww.view.is_some() && ww.shown.is_some(),
            "the trace is drawn"
        );
        assert_eq!(ww.lpf.text, "33", "LPF starts at SA's 33 without a Br");
    });
}

#[gpui::test]
fn toolbar_buttons_follow_sa_order_and_enable_rules(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open_wf(&view, cx);
    let order: Vec<WfButton> = slot_places(2)
        .into_iter()
        .filter_map(|(s, _, _)| match s {
            WfSlot::Button(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(order, WF_BUTTONS.to_vec());
    let (_, x, w) = *slot_places(2).last().unwrap();
    assert!(x + w <= view.read_with(cx, |f, _| f.client_of(win).0));
    // The loaded `(0)` file: no zoom step, nothing to revert, no tones, no Save.
    let tools = view.read_with(cx, |f, _| f.wf_tools(win, node));
    let disabled: Vec<WfButton> = WF_BUTTONS
        .into_iter()
        .filter(|b| wf_tools::missing(*b, tools).is_some())
        .collect();
    assert_eq!(
        disabled,
        [
            WfButton::ZoomMinus,
            WfButton::Revert,
            WfButton::Synth,
            WfButton::Save
        ]
    );
    // A disabled button does nothing (Z- would go back in the history).
    let state = |view: &Entity<Frame>, cx: &mut VisualTestContext| {
        view.read_with(cx, |f, _| {
            let ww = &f.wf.wins[&win];
            (ww.view, ww.history.len())
        })
    };
    let before = state(&view, cx);
    click_button(&view, cx, win, WfButton::ZoomMinus);
    assert_eq!(state(&view, cx), before);
    assert!(view.read_with(cx, |f, _| f.msgbox.is_none()));
    // With collapsed markers the fragment tools answer SA's error box.
    click_button(&view, cx, win, WfButton::Copy);
    let text = view.read_with(cx, |f, _| f.msgbox.as_ref().map(|m| m.text.clone()));
    assert_eq!(text.as_deref(), Some("Please check settings!"));
    view.update(cx, |f, cx| {
        f.msgbox = None;
        cx.notify();
    });
    cx.run_until_parked();
    // Z+ zooms to the swept V span; Z- is then enabled and goes back.
    sweep(&view, cx, win);
    let full = view.read_with(cx, |f, _| f.wf.wins[&win].view.unwrap());
    click_button(&view, cx, win, WfButton::ZoomPlus);
    let zoomed = view.read_with(cx, |f, _| f.wf.wins[&win].view.unwrap());
    assert!(zoomed.samples_per_px < full.samples_per_px);
    assert!(view.read_with(cx, |f, _| f.wf_tools(win, node).history));
    click_button(&view, cx, win, WfButton::ZoomMinus);
    assert_eq!(
        view.read_with(cx, |f, _| f.wf.wins[&win].view.unwrap()),
        full
    );
    // The scale buttons halve and double the vertical half range.
    let s0 = view.read_with(cx, |f, _| f.ui[&node].wave_scale);
    click_button(&view, cx, win, WfButton::ScaleUp);
    let s1 = view.read_with(cx, |f, _| f.ui[&node].wave_scale);
    assert!((s1 - s0 / 2.0).abs() < 1e-6);
}

#[gpui::test]
fn copy_opens_wfc_windows_and_acf_a_wf_acf_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = open_wf(&view, cx);
    sweep(&view, cx, win);
    let m = markers(&view, cx, win);
    assert!(m.time_range().is_some());
    click_button(&view, cx, win, WfButton::Copy);
    settle(&view, cx);
    activate(&view, cx, win);
    click_button(&view, cx, win, WfButton::Copy);
    settle(&view, cx);
    activate(&view, cx, win);
    click_button(&view, cx, win, WfButton::Acf);
    settle(&view, cx);
    let t = titles(&view, cx);
    let names: Vec<&str> = t.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "[TempDir] mfsk8(0)(wf_acf).wav",
            "[TempDir] mfsk8(0)(wfc0).wav",
            "[TempDir] mfsk8(0)(wfc1).wav",
            "[TempDir] mfsk8(0).wav",
            "[TempDir] mfsk8(0).wav",
        ]
    );
    let wf_windows = t.iter().filter(|(_, k)| *k == WinKind::Waveform).count();
    assert_eq!(wf_windows, 4, "results open as WF windows");
}

#[gpui::test]
fn the_filter_redraws_the_window_and_revert_brings_back_the_original(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open_wf(&view, cx);
    let before = view.read_with(cx, |f, _| f.signal_windows().len());
    view.update(cx, |f, _| {
        f.wf.wins.get_mut(&win).unwrap().lpf.set_text("300");
    });
    click_button(&view, cx, win, WfButton::Lpf);
    settle(&view, cx);
    let shown = view.read_with(cx, |f, _| f.node_of(win).unwrap());
    assert_ne!(shown, node, "the window shows the filtered signal");
    assert_eq!(view.read_with(cx, |f, _| f.signal_windows().len()), before);
    view.read_with(cx, |f, _| {
        assert_eq!(f.name_of(shown), "mfsk8(0)", "under the same name");
        assert_eq!(f.wf.wins[&win].original, Some(node));
        assert!(f.wf_tools(win, shown).revert && f.wf_tools(win, shown).derived);
    });
    click_button(&view, cx, win, WfButton::Revert);
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert_eq!(f.node_of(win), Some(node));
        assert!(
            f.discarded.contains(&shown),
            "the filtered signal is dropped"
        );
        assert!(!f.wf_tools(win, node).revert);
    });
    // The CCF is drawn over the source and Revert clears it.
    sweep(&view, cx, win);
    click_button(&view, cx, win, WfButton::Ccf);
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert_eq!(f.signal_windows().len(), before, "no window opens");
        assert!(f.wf.wins[&win].ccf.is_some());
    });
    click_button(&view, cx, win, WfButton::Revert);
    assert!(view.read_with(cx, |f, _| f.wf.wins[&win].ccf.is_none()));
}

#[gpui::test]
fn count_freqs_needs_sum_and_shows_the_grid_box(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open_wf(&view, cx);
    // Without Sum the popup has no Count freqs.
    let p = plot_at(&view, cx, win, 300, 150);
    right_click(cx, p);
    assert_eq!(
        popup_labels(&view, cx),
        ["Save as", "Cancel", "-", "SigNexora tools"]
    );
    view.update(cx, |f, _| close_popup(f));
    // A frequency trace: the phase detector of the MFSK-8 signal.
    view.update(cx, |f, _| {
        f.start_edit(win, sn_dsp::ops::Edit::PhaseDetector);
    });
    settle(&view, cx);
    let phd = view.read_with(cx, |f, _| {
        f.graph
            .nodes()
            .find(|n| n.inputs.first() == Some(&node))
            .map(|n| n.id)
            .unwrap()
    });
    let pw = view.update(cx, |f, _| {
        let sono = f.find_window(phd, WinKind::Sonogram);
        f.open_window(phd, WinKind::Waveform, sono).unwrap()
    });
    settle(&view, cx);
    view.update(cx, |f, _| {
        let ww = f.wf.wins.get_mut(&pw).unwrap();
        ww.sum = true;
        ww.markers.h = Some((1000.0, 2000.0));
        // The trace zoomed vertically onto the tones, as an analyst does.
        let u = f.ui.get_mut(&phd).unwrap();
        u.wave_scale = 500.0;
        u.wave_offset = 1500.0;
    });
    settle(&view, cx);
    let p = plot_at(&view, cx, pw, 300, 150);
    right_click(cx, p);
    assert_eq!(
        popup_labels(&view, cx),
        ["Save as", "Count freqs", "Cancel", "-", "SigNexora tools"]
    );
    view.update_in(cx, |f, window, cx| {
        f.command(cmd::WF_COUNT, window, cx);
        close_popup(f);
    });
    settle(&view, cx);
    view.update(cx, |_, cx| cx.notify());
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        let ww = &f.wf.wins[&pw];
        assert!(ww.count_pending.is_none());
        assert!(!f.wf_tones(phd).is_empty(), "the tones are kept");
        assert_eq!(
            ww.labels.len(),
            1,
            "{:?}",
            f.msgbox.as_ref().map(|m| &m.text)
        );
    });
    // The grid fitted to known tones (MFSK-8 at 125 Hz) between the H markers.
    view.update(cx, |f, cx| {
        let ww = f.wf.wins.get_mut(&pw).unwrap();
        // SA: the H markers on the extreme channels.
        ww.count_pending = Some((1062.5, 1937.5));
        ww.markers.h = Some((1062.5, 1937.5));
        let tones: Vec<f64> = (0..8).map(|k| 1062.5 + 125.0 * f64::from(k)).collect();
        f.wf_on_tones(phd, tones, 5.0);
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        let lines = &f.wf.wins[&pw].labels.first().unwrap().lines;
        assert_eq!(lines, &["Freq grid: 8", "Delta: 125.00 Hz"]);
    });
    // A right click on the box removes it, without a popup menu.
    let (bx, by) = view.read_with(cx, |f, _| f.wf.wins[&pw].labels[0].offset.unwrap());
    let p = plot_at(&view, cx, pw, bx + 4, by + 4);
    right_click(cx, p);
    view.read_with(cx, |f, _| {
        assert_eq!(f.wf.wins[&pw].labels.len(), 0);
        assert!(f.popup.is_none());
    });
}

#[gpui::test]
fn markers_lock_and_collapse_like_sa(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = open_wf(&view, cx);
    sweep(&view, cx, win);
    let p = plot_at(&view, cx, win, 200, 3);
    right_click(cx, p);
    assert_eq!(
        markers(&view, cx, win).locked,
        Some(sn_ui_core::markers::MarkerId::V1)
    );
    assert!(view.read_with(cx, |f, _| f.popup.is_none()));
    let p = plot_at(&view, cx, win, 300, 90);
    double_click(cx, p);
    let m = markers(&view, cx, win);
    assert!(m.time_range().is_none() && m.locked.is_none());
}

#[gpui::test]
fn the_mode_buttons_switch_between_wf_and_vmw(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open_wf(&view, cx);
    let r = hot(&view, cx, win, "vmw");
    let p = at(&view, cx, win, r.x + r.w / 2, r.y + r.h / 2);
    click(cx, p);
    settle(&view, cx);
    assert_eq!(
        view.read_with(cx, |f, _| f.binding(win)),
        Some((node, WinKind::Raster))
    );
    view.read_with(cx, |f, _| {
        let v = &f.wf.wins[&win].vmw;
        assert!(v.raster.shown_request().is_some(), "the raster is drawn");
        assert_eq!(v.strobe(), 1);
        assert_eq!(v.y_scale(), 2, "SA's Y scale - 2");
    });
    let r = hot(&view, cx, win, "waveform");
    let p = at(&view, cx, win, r.x + r.w / 2, r.y + r.h / 2);
    click(cx, p);
    assert_eq!(
        view.read_with(cx, |f, _| f.binding(win)),
        Some((node, WinKind::Waveform))
    );
}

#[gpui::test]
fn ts_digits_take_left_and_right_clicks(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = open_wf(&view, cx);
    view.update(cx, |f, cx| {
        f.wf_set_window_kind(win, WinKind::Raster);
        cx.notify();
    });
    settle(&view, cx);
    let fs = 8000.0;
    let period = view.read_with(cx, |f, _| f.wf.wins[&win].vmw.period);
    let text = ts_text(period / fs);
    assert_eq!(text, "00:125.000.000", "the default line period");
    let r = hot(&view, cx, win, "ts");
    // The 100 ms digit is character 3.
    let x = view.update_in(cx, |_, window, _| {
        let line = big_line(window, &text, gpui::black());
        let a = f32::from(line.x_for_index(3));
        let b = f32::from(line.x_for_index(4));
        f32::midpoint(a, b).round() as i32
    });
    let p = at(&view, cx, win, r.x + x, r.y + r.h / 2);
    click(cx, p);
    let ts = view.read_with(cx, |f, _| ts_text(f.wf.wins[&win].vmw.period / fs));
    assert_eq!(ts, "00:225.000.000");
    right_click(cx, p);
    right_click(cx, p);
    let ts = view.read_with(cx, |f, _| ts_text(f.wf.wins[&win].vmw.period / fs));
    assert_eq!(ts, "00:025.000.000");
    assert!(
        view.read_with(cx, |f, _| f.popup.is_none()),
        "no popup menu"
    );
}

#[gpui::test]
fn a_second_filter_press_does_not_open_a_window_under_the_same_name(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open_wf(&view, cx);
    let before = titles(&view, cx).len();
    view.update(cx, |f, _| {
        f.wf_filter(win, node);
        f.wf_filter(win, node);
    });
    settle(&view, cx);
    let after = titles(&view, cx);
    assert_eq!(after.len(), before, "{after:?}");
    view.read_with(cx, |f, _| {
        assert_ne!(f.node_of(win), Some(node), "the filter result is shown");
    });
}

#[gpui::test]
fn a_filter_stopped_by_revert_does_not_replace_the_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open_wf(&view, cx);
    view.update(cx, |f, _| f.wf_filter(win, node));
    // Let the filter finish before Revert stops it, without collecting it.
    let start = Instant::now();
    while !view.read_with(cx, |f, _| f.jobs.iter().all(|j| j.handle.is_finished())) {
        assert!(
            start.elapsed() <= Duration::from_secs(30),
            "filter still running"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    view.update(cx, |f, _| f.wf_revert(win, node));
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert_eq!(f.node_of(win), Some(node), "Revert is not undone");
        assert_eq!(f.wf.wins[&win].original, None);
        assert_eq!(f.graph.children(node).count(), 0, "no filtered node kept");
        assert!(f.status.contains("cancelled"), "{}", f.status);
    });
}

/// Wait until every job has finished, without collecting the results.
fn wait_finished(view: &Entity<Frame>, cx: &mut VisualTestContext) {
    let start = Instant::now();
    while !view.read_with(cx, |f, _| f.jobs.iter().all(|j| j.handle.is_finished())) {
        assert!(
            start.elapsed() <= Duration::from_secs(30),
            "jobs still running"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[gpui::test]
fn a_finished_filter_stopped_by_a_newer_press_does_not_replace_the_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open_wf(&view, cx);
    let nodes_before = view.read_with(cx, |f, _| f.graph.len());
    view.update(cx, |f, _| {
        f.wf.wins.get_mut(&win).unwrap().lpf.set_text("300");
        f.wf_filter(win, node);
    });
    // The first filter finishes before the second press stops it, and both
    // results are collected in the same poll.
    wait_finished(&view, cx);
    view.update(cx, |f, _| {
        f.wf.wins.get_mut(&win).unwrap().lpf.set_text("500");
        f.wf_filter(win, node);
    });
    wait_finished(&view, cx);
    view.update(cx, |f, _| f.collect_finished_jobs());
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        let shown = f.node_of(win).unwrap();
        assert_ne!(shown, node, "the newest filter result is shown");
        let children: Vec<NodeId> = f.graph.children(node).map(|n| n.id).collect();
        assert_eq!(children, vec![shown], "no node from the first filter");
        assert_eq!(f.graph.len(), nodes_before + 1);
        let params = f.graph.get(shown).unwrap().operation.params.to_string();
        assert!(params.contains("500"), "{params}");
        assert!(!params.contains("300"), "{params}");
    });
}

#[gpui::test]
fn count_freqs_settles_while_the_window_shows_the_raster(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = open_wf(&view, cx);
    view.update(cx, |f, _| {
        f.wf.wins.get_mut(&win).unwrap().markers.h = Some((1000.0, 2000.0));
        f.wf_start_count_freqs(win);
        assert!(f.wf.wins[&win].count_pending.is_some());
        f.wf_set_window_kind(win, WinKind::Raster);
    });
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert!(f.wf.wins[&win].count_pending.is_none());
        assert!(!f.wf_busy(), "no endless poll loop");
    });
}

#[gpui::test]
fn a_cancelled_find_tones_ends_count_freqs(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = open_wf(&view, cx);
    view.update(cx, |f, _| {
        f.wf.wins.get_mut(&win).unwrap().markers.h = Some((1000.0, 2000.0));
        f.wf_start_count_freqs(win);
        for j in &mut f.jobs {
            j.cancel();
        }
    });
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert!(f.wf.wins[&win].count_pending.is_none());
        assert_eq!(f.wf.wins[&win].labels.len(), 0);
        assert!(!f.wf_busy());
    });
}

#[gpui::test]
fn straightening_divides_the_drift_by_the_strobe(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = open_wf(&view, cx);
    view.update(cx, |f, _| {
        f.wf.wins.get_mut(&win).unwrap().vmw.strobe.set(3);
        f.wf_set_window_kind(win, WinKind::Raster);
    });
    let start = Instant::now();
    while view.read_with(cx, |f, _| {
        f.wf.wins[&win].vmw.raster.shown_request().is_none()
    }) {
        assert!(start.elapsed() < Duration::from_secs(30), "no raster");
        settle(&view, cx);
        std::thread::sleep(Duration::from_millis(5));
    }
    view.update(cx, |f, _| {
        let v = &f.wf.wins[&win].vmw;
        let shown = v.raster.shown_request().unwrap();
        assert_eq!(shown.strobe, 3);
        let (w, h) = (f64::from(v.plot.w), f64::from(v.plot.h));
        let drift = sn_ui_core::vmw_tools::drag_drift(
            (20.0, h / 2.0),
            (w, h),
            (shown.cols, shown.rows),
            shown.period,
        )
        .unwrap();
        f.vmw_straighten(win, (20.0, h / 2.0));
        let period = f.wf.wins[&win].vmw.period;
        assert!(
            (period - (shown.period + drift / 3.0)).abs() < 1e-9,
            "{period} vs {} + {drift} / 3",
            shown.period
        );
        // The navigation bar at its end: the start stays inside the file.
        let ww = f.wf.wins.get_mut(&win).unwrap();
        ww.vmw.nav.pos = ww.vmw.nav.top();
        f.vmw_nav_moved(win);
        let len = f.graph.get(f.node_of(win).unwrap()).unwrap().signal.len() as f64;
        let v = &f.wf.wins[&win].vmw;
        assert!(
            v.start >= 0.0 && v.start <= (len - v.period).max(0.0),
            "{}",
            v.start
        );
    });
}
