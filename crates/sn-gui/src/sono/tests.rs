//! Behaviour tests of the sonogram work window on GPUI's test platform: the
//! mouse and keyboard are simulated inside the test app only.

use std::time::{Duration, Instant};

use gpui::{
    Entity, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Point, TestAppContext,
    VisualTestContext, point, px,
};
use serde_json::json;
use sn_core::{NodeId, Operation};
use sn_ui_core::markers::{MarkerId, Markers};
use sn_ui_core::sono_tools::{self, BUTTONS, Button, Slot};

use super::bottom::slot_places;
use super::{CLIENT, Layout};
use crate::app::Frame;
use crate::app::menus::cmd;
use crate::app::model::WinKind;
use crate::win95::bars::TOOL;
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
    cx.run_until_parked();
}

/// Load the generated MFSK-8 test recording; returns its node and window.
fn load(view: &Entity<Frame>, cx: &mut VisualTestContext) -> (NodeId, u32) {
    let node = view.update(cx, |f, cx| {
        let op = Operation::new("load", json!({"path": "/data/mfsk8.wav"}));
        let id = f.on_loaded("mfsk8.wav".into(), op, super::test_signal().unwrap(), &[]);
        cx.notify();
        id
    });
    cx.run_until_parked();
    settle(view, cx);
    let win = view.read_with(cx, |f, _| f.find_window(node, WinKind::Sonogram).unwrap());
    (node, win)
}

/// The layout of window `win` and its client origin in window coordinates.
fn geometry(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32) -> (Layout, (i32, i32)) {
    view.read_with(cx, |f, _| {
        let r = f.mdi.shown_rect(win).unwrap();
        let c = mdi::client_rect(r.w, r.h);
        let (cw, ch) = f.client_of(win);
        (
            Layout::new(cw, ch),
            (2 + r.x + c.x, bar_h() + 2 + r.y + c.y),
        )
    })
}

/// Window position of plot point `(x, y)` of window `win`.
fn plot_at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) -> Px {
    let (l, (ox, oy)) = geometry(view, cx, win);
    point(
        px((ox + l.plot.x + x) as f32),
        px((oy + l.plot.y + y) as f32),
    )
}

/// Window position of the middle of bottom toolbar button `b` of window `win`.
fn button_at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, b: Button) -> Px {
    let (l, (ox, oy)) = geometry(view, cx, win);
    let x = slot_places(l.bottom.x + 2)
        .into_iter()
        .find(|(s, _, _)| *s == Slot::Button(b))
        .map(|(_, x, _)| x)
        .unwrap();
    let y = l.bottom.y + (l.bottom.h - TOOL) / 2;
    point(
        px((ox + x + TOOL / 2) as f32),
        px((oy + y + TOOL / 2) as f32),
    )
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

fn click_button(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, b: Button) {
    let p = button_at(view, cx, win, b);
    click(cx, p);
}

fn right_click_plot(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) {
    let p = plot_at(view, cx, win, x, y);
    right_click(cx, p);
}

fn double_click_plot(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) {
    let p = plot_at(view, cx, win, x, y);
    double_click(cx, p);
}

fn drag_plot(
    view: &Entity<Frame>,
    cx: &mut VisualTestContext,
    win: u32,
    from: (i32, i32),
    to: (i32, i32),
) {
    let a = plot_at(view, cx, win, from.0, from.1);
    let b = plot_at(view, cx, win, to.0, to.1);
    drag(cx, a, b);
}

fn markers(view: &Entity<Frame>, cx: &mut VisualTestContext, node: NodeId) -> Markers {
    view.read_with(cx, |f, _| f.ui[&node].markers)
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

#[gpui::test]
fn both_toolbars_fit_the_new_window_in_one_row(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = load(&view, cx);
    let (cw, ch) = view.read_with(cx, |f, _| f.client_of(win));
    assert_eq!((cw, ch), CLIENT);
    // The top toolbar's width depends on the font; the test platform's text
    // system has fixed-width glyphs, so it is checked on window captures
    // (it ends near x = 800 with the W95 font).
    let (_, x, w) = *slot_places(2).last().unwrap();
    assert!(x + w <= cw, "bottom toolbar {} px in {cw}", x + w);
    // The plot got a view and its first image is being computed or shown.
    view.read_with(cx, |f, _| {
        let sw = &f.sono.wins[&win];
        assert!(sw.view.is_some() && sw.extent.is_some());
        assert_eq!(sw.value.text, "8000", "R/S starts at the sample rate");
    });
}

#[gpui::test]
fn toolbar_buttons_follow_sa_order_and_enable_rules(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    // SA's order, left to right, with the progress field after the zoom group.
    let order: Vec<Button> = slot_places(2)
        .into_iter()
        .filter_map(|(s, _, _)| match s {
            Slot::Button(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(order, BUTTONS.to_vec());
    // Collapse the markers with a double click: the selection tools grey out.
    double_click_plot(&view, cx, win, 200, 100);
    let tools = view.read_with(cx, |f, _| f.tool_state(win, node));
    assert!(!tools.v && !tools.h);
    let enabled: Vec<Button> = BUTTONS
        .into_iter()
        .filter(|b| sono_tools::missing(*b, tools).is_none())
        .collect();
    assert_eq!(
        enabled,
        [
            Button::ZoomReset,
            Button::Play,
            Button::AmDetector,
            Button::ModifiedAm,
            Button::HalfWave,
            Button::Derivative,
            Button::Limiter,
            Button::Inverse,
            Button::Resample,
            Button::Shift,
        ]
    );
    // A disabled button does nothing; an enabled one runs its operation.
    let before = view.read_with(cx, |f, _| f.signal_windows().len());
    click_button(&view, cx, win, Button::Copy);
    settle(&view, cx);
    assert_eq!(view.read_with(cx, |f, _| f.signal_windows().len()), before);
    click_button(&view, cx, win, Button::AmDetector);
    settle(&view, cx);
    let titles: Vec<String> = view.read_with(cx, |f, _| {
        f.signal_windows()
            .into_iter()
            .filter_map(|(id, _, _)| f.mdi.get(id).map(|c| c.title.clone()))
            .collect()
    });
    assert_eq!(titles.last().unwrap(), "[TempDir] mfsk8(0)(u1).wav");
}

#[gpui::test]
fn sweeping_locking_and_collapsing_the_markers(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    let (sv, h) = view.read_with(cx, |f, _| {
        let sw = &f.sono.wins[&win];
        (sw.view.unwrap(), f64::from(sw.plot.h))
    });
    // Left drag sweeps the V and H pairs as a rectangle.
    drag_plot(&view, cx, win, (100, 40), (300, 140));
    let m = markers(&view, cx, node);
    let (t0, t1) = m.time_range().unwrap();
    let (f0, f1) = m.freq_range().unwrap();
    assert!((t0 - sv.x_to_sample(100.0)).abs() < 1e-6);
    assert!((t1 - sv.x_to_sample(300.0)).abs() < 1e-6);
    assert!((f0 - sv.y_to_freq(140.0, h)).abs() < 1e-6);
    assert!((f1 - sv.y_to_freq(40.0, h)).abs() < 1e-6);
    // Right click on V1's handle locks it (red), no popup menu.
    right_click_plot(&view, cx, win, 100, 3);
    assert_eq!(markers(&view, cx, node).locked, Some(MarkerId::V1));
    assert!(view.read_with(cx, |f, _| f.popup.is_none()));
    // Only one marker is locked at a time.
    let x2 = sv.sample_to_x(t1).round() as i32;
    right_click_plot(&view, cx, win, x2, 3);
    assert_eq!(markers(&view, cx, node).locked, Some(MarkerId::V2));
    right_click_plot(&view, cx, win, x2, 3);
    assert_eq!(markers(&view, cx, node).locked, None);
    // A locked marker stays put when a new sweep starts.
    right_click_plot(&view, cx, win, 100, 3);
    drag_plot(&view, cx, win, (400, 60), (500, 120));
    let m = markers(&view, cx, node);
    assert!((m.get(MarkerId::V1).unwrap() - t0).abs() < 1e-6, "{m:?}");
    // Dragging a marker by its line moves it.
    let y_h1 = sv.freq_to_y(m.get(MarkerId::H1).unwrap(), h).round() as i32;
    drag_plot(&view, cx, win, (600, y_h1), (600, y_h1 + 30));
    let moved = markers(&view, cx, node).get(MarkerId::H1).unwrap();
    assert!((moved - sv.y_to_freq(f64::from(y_h1 + 30), h)).abs() < 1e-6);
    // Double click collapses both pairs, unlocks, and disables the selection.
    double_click_plot(&view, cx, win, 250, 90);
    let m = markers(&view, cx, node);
    assert_eq!(m.locked, None);
    assert!(m.time_range().is_none() && m.freq_range().is_none() && m.v.is_some());
    view.update(cx, |f, _| f.ui.get_mut(&node).unwrap().info = true);
    let lines = view.read_with(cx, |f, _| f.info_lines(node).unwrap());
    assert_eq!(lines[3], "dF = 0");
    assert_eq!(lines[6], "dT = 00:00.000.000");
}

#[gpui::test]
fn the_popup_menu_follows_sa_6_2(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    drag_plot(&view, cx, win, (100, 40), (300, 140));
    right_click_plot(&view, cx, win, 400, 200);
    assert_eq!(
        popup_labels(&view, cx),
        [
            "Save as",
            "-",
            "Wave Form module",
            "Measurement of Freq",
            "Auto define params",
            "-",
            "Correction of BR",
            "Cancel",
        ]
    );
    // Wave Form module opens the WF window on this signal.
    view.update_in(cx, |f, window, cx| f.command(cmd::SW_WAVEFORM, window, cx));
    cx.run_until_parked();
    assert!(view.read_with(cx, |f, _| f.find_window(node, WinKind::Waveform).is_some()));
    // With collapsed markers the items that need them are left out.
    activate(&view, cx, win);
    double_click_plot(&view, cx, win, 250, 90);
    right_click_plot(&view, cx, win, 400, 200);
    assert_eq!(
        popup_labels(&view, cx),
        [
            "Save as",
            "-",
            "Wave Form module",
            "-",
            "Correction of BR",
            "Cancel"
        ]
    );
    // Correction of BR opens SA's panel inside the plot.
    view.update_in(cx, |f, window, cx| f.command(cmd::SW_BRC, window, cx));
    cx.run_until_parked();
    view.read_with(cx, |f, _| {
        let brc = f.sono.wins[&win].brc.as_ref().unwrap();
        assert!(!brc.measured && brc.existing.text.is_empty());
        assert_eq!(
            f.kit.focus.as_ref().map(AsRef::<str>::as_ref),
            Some(super::brc::field_id(win, 0).as_str()),
            "without a measured Br the existing rate is typed first"
        );
    });
}

#[gpui::test]
fn the_info_box_shows_sa_lines_in_sa_order(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    assert!(view.read_with(cx, |f, _| f.info_lines(node).is_none()));
    // Wide enough for the test platform's fixed-width text.
    view.update(cx, |f, cx| {
        let r = f.mdi.get(win).unwrap().rect;
        f.mdi
            .set_rect(win, crate::win95::geom::Rect::new(r.x, r.y, 1200, r.h));
        cx.notify();
    });
    cx.run_until_parked();
    // The Info check box of the top toolbar.
    let (l, (ox, oy)) = geometry(&view, cx, win);
    let info_x = view.update_in(cx, |f, window, _| f.top_toolbar_width(win, window)) - 3 - 4 - 7;
    let at = point(
        px((ox + info_x) as f32),
        px((oy + l.top.y + l.top.h / 2) as f32),
    );
    click(cx, at);
    view.update(cx, |f, _| {
        f.ui.get_mut(&node).unwrap().markers = Markers {
            v: Some((0.0, 461_312.0)),
            h: Some((1516.0, 6411.0)),
            ..Markers::default()
        };
    });
    let lines = view
        .read_with(cx, |f, _| f.info_lines(node))
        .expect("Info checked");
    assert_eq!(
        lines,
        [
            "Fu = 6411",
            "Fc = 3963.5",
            "Fl = 1516",
            "dF = 4895",
            "t1 = 00:00.000",
            "t2 = 00:57.664",
            "dT = 00:57.664.000",
            "S = 0.02",
            "Br = 33.33",
            "Sc = 3963.5",
        ]
    );
}

#[gpui::test]
fn toolbar_tools_open_tagged_windows(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = load(&view, cx);
    drag_plot(&view, cx, win, (100, 40), (300, 140));
    click_button(&view, cx, win, Button::Copy);
    settle(&view, cx);
    activate(&view, cx, win);
    // R resamples to the rate in the value field.
    view.update(cx, |f, _| {
        f.sono.wins.get_mut(&win).unwrap().value.set_text("4000");
    });
    click_button(&view, cx, win, Button::Resample);
    settle(&view, cx);
    let titles: Vec<String> = view.read_with(cx, |f, _| {
        f.signal_windows()
            .into_iter()
            .filter_map(|(id, _, _)| f.mdi.get(id).map(|c| c.title.clone()))
            .collect()
    });
    let mut titles = titles;
    titles.sort();
    assert_eq!(
        titles,
        [
            "[TempDir] mfsk8(0)(c1).wav",
            "[TempDir] mfsk8(0)(r_4000).wav",
            "[TempDir] mfsk8(0).wav",
        ]
    );
    // A bad R/S value gets SA's error box instead.
    activate(&view, cx, win);
    view.update(cx, |f, _| {
        f.sono.wins.get_mut(&win).unwrap().value.set_text("abc");
    });
    click_button(&view, cx, win, Button::Resample);
    assert!(view.read_with(cx, |f, _| f.msgbox.is_some()));
}

#[gpui::test]
fn result_boxes_move_with_the_left_button_and_go_with_a_right_click(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    // Measurement of Freq on one tone of the MFSK grid.
    view.update(cx, |f, _| {
        f.ui.get_mut(&node).unwrap().markers = Markers {
            v: Some((0.0, 40_000.0)),
            h: Some((1380.0, 1620.0)),
            ..Markers::default()
        };
        f.measure_freq(node);
    });
    settle(&view, cx);
    let label = view.read_with(cx, |f, _| f.sono.wins[&win].labels.clone());
    assert_eq!(label.len(), 1);
    assert!(label[0].text.starts_with("Freq: "), "{}", label[0].text);
    let (x, y) = label[0].offset.unwrap();
    // Left drag moves it.
    drag_plot(&view, cx, win, (x + 5, y + 5), (x - 35, y + 25));
    let moved = view.read_with(cx, |f, _| f.sono.wins[&win].labels[0].offset);
    assert_eq!(moved, Some((x - 40, y + 20)));
    // The markers stayed where they were: the drag was the box's.
    assert_eq!(markers(&view, cx, node).h, Some((1380.0, 1620.0)));
    // A right click removes it, without a popup menu.
    right_click_plot(&view, cx, win, x - 35, y + 25);
    view.read_with(cx, |f, _| {
        assert_eq!(f.sono.wins[&win].labels, []);
        assert!(f.popup.is_none());
    });
}

#[gpui::test]
fn plot_keys_zoom_restore_and_nudge(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    drag_plot(&view, cx, win, (100, 40), (300, 140));
    let swept = markers(&view, cx, node);
    let full = view.read_with(cx, |f, _| f.sono.wins[&win].view.unwrap());
    // Z zooms to the selection, Backspace goes back, F shows everything.
    cx.simulate_keystrokes("z");
    cx.run_until_parked();
    let zoomed = view.read_with(cx, |f, _| f.sono.wins[&win].view.unwrap());
    assert!(zoomed.samples_per_px < full.samples_per_px);
    assert!(view.read_with(cx, |f, _| f.tool_state(win, node).history));
    cx.simulate_keystrokes("backspace");
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |f, _| f.sono.wins[&win].view.unwrap()),
        full
    );
    // The arrow keys move the marker moved last (the sweep sets V2, then H2).
    let h = view.read_with(cx, |f, _| f64::from(f.sono.wins[&win].plot.h));
    cx.simulate_keystrokes("up");
    cx.run_until_parked();
    let m = markers(&view, cx, node);
    let d = m.get(MarkerId::H2).unwrap() - swept.get(MarkerId::H2).unwrap();
    assert!((d - (full.f_hi - full.f_lo) / h).abs() < 1e-6, "{d}");
    // A double click collapses; Cmd+Z brings the swept markers back.
    double_click_plot(&view, cx, win, 250, 90);
    assert!(markers(&view, cx, node).time_range().is_none());
    cx.simulate_keystrokes("cmd-z");
    cx.run_until_parked();
    assert_eq!(markers(&view, cx, node).time_range(), m.time_range());
}

#[gpui::test]
fn the_brc_panel_takes_clicks_and_checks_its_values(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    drag_plot(&view, cx, win, (100, 120), (300, 200));
    let before = markers(&view, cx, node);
    view.update(cx, |f, cx| {
        f.open_brc(win);
        cx.notify();
    });
    cx.run_until_parked();
    // A click in the panel's field is the panel's, not a new sweep.
    drag_plot(&view, cx, win, (130, 20), (170, 30));
    assert_eq!(markers(&view, cx, node), before);
    // Values more than 30% apart are refused with SA's error box.
    view.update(cx, |f, _| {
        let b = f.sono.wins.get_mut(&win).unwrap().brc.as_mut().unwrap();
        b.existing.set_text("100");
        b.required.set_text("200");
        f.brc_yes(win);
    });
    assert!(view.read_with(cx, |f, _| f.msgbox.is_some()));
    assert!(view.read_with(cx, |f, _| f.sono.wins[&win].brc.is_some()));
    // A valid correction closes the panel and opens a tagged window.
    view.update(cx, |f, _| {
        f.msgbox = None;
        let b = f.sono.wins.get_mut(&win).unwrap().brc.as_mut().unwrap();
        b.required.set_text("110");
        f.brc_yes(win);
    });
    settle(&view, cx);
    view.read_with(cx, |f, _| {
        assert!(f.sono.wins[&win].brc.is_none());
        let top = f.mdi.active().unwrap();
        assert_eq!(
            f.mdi.get(top).unwrap().title,
            "[TempDir] mfsk8(0)(brc1).wav"
        );
    });
}

#[gpui::test]
fn save_as_writes_every_format(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (view, cx) = frame(cx);
    let (node, _) = load(&view, cx);
    for (i, format) in super::popup::SAVE_FORMATS.into_iter().enumerate() {
        let path = dir.path().join(format!("out{i}.{}", format.extension()));
        view.update(cx, |f, _| f.start_save(node, format, path.clone()));
        settle(&view, cx);
        view.read_with(cx, |f, _| {
            assert!(f.status.contains(&format!("out{i}")), "{}", f.status);
        });
        let written = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&format!("out{i}"))
            });
        assert!(written, "{format:?}");
    }
}

#[gpui::test]
fn closing_the_window_keeps_a_running_save(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kept.wav");
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    view.update(cx, |f, _| {
        f.start_save(node, sn_ui_core::edits::SaveFormat::WavPcm16, path.clone());
        f.close_window_now(win);
        assert!(f.discarded.contains(&node), "the signal left the session");
    });
    settle(&view, cx);
    assert!(path.exists(), "the file the user asked for is written");
}

#[gpui::test]
fn save_as_after_the_signal_closed_is_refused(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("late.wav");
    let (view, cx) = frame(cx);
    let (node, win) = load(&view, cx);
    view.update(cx, |f, _| {
        let generation = f.generation;
        f.close_window_now(win);
        f.save_chosen(
            node,
            sn_ui_core::edits::SaveFormat::WavPcm16,
            generation,
            Ok(Some(path.clone())),
        );
        assert!(f.jobs.iter().all(|j| !j.writes_file()), "no empty file");
        assert!(f.msgbox.is_some(), "the user is told");
    });
    settle(&view, cx);
    assert!(!path.exists());
}

#[gpui::test]
fn a_chooser_that_could_not_be_shown_is_reported(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, _) = load(&view, cx);
    view.update(cx, |f, _| {
        let generation = f.generation;
        f.save_chosen(
            node,
            sn_ui_core::edits::SaveFormat::WavPcm16,
            generation,
            Err("portal missing".to_owned()),
        );
        assert!(
            f.jobs.iter().all(|j| !j.writes_file()),
            "nothing is written"
        );
        assert!(f.msgbox.is_some(), "the user is told");
    });
}

#[gpui::test]
fn the_image_list_picks_by_index_without_rounding(cx: &mut TestAppContext) {
    use super::top::{Ctrl, image_ranges};
    let (view, cx) = frame(cx);
    let (node, _) = load(&view, cx);
    view.update(cx, |f, _| {
        // A range SA's list lacks (from a session): shown as "- 58 db".
        f.ui.get_mut(&node).unwrap().settings.range_db = 57.5;
        let ranges = image_ranges(&f.ui[&node].settings);
        let i = ranges
            .iter()
            .position(|r| (*r - 57.5).abs() < 1e-6)
            .unwrap();
        f.list_chosen(node, Ctrl::Image, i);
        assert!((f.ui[&node].settings.range_db - 57.5).abs() < 1e-6, "kept");
        // Every list choice lands in the settings.
        f.list_chosen(node, Ctrl::Image, 0);
        assert!((f.ui[&node].settings.range_db - ranges[0]).abs() < 1e-6);
        f.list_chosen(node, Ctrl::Fft, 0);
        assert_eq!(
            f.ui[&node].settings.fft_size,
            sn_ui_core::sono_settings::FFT_SIZES[0]
        );
    });
}
