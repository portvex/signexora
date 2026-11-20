//! Behaviour tests of the demodulator window on GPUI's test platform: the
//! mouse and keyboard are simulated inside the test app only.

use std::time::{Duration, Instant};

use gpui::{Entity, Modifiers, MouseButton, Point, TestAppContext, VisualTestContext, point, px};
use serde_json::json;
use sn_core::{Measurement, MeasurementKind, NodeId, Operation};
use sn_ui_core::demod::{DataAs, boundaries};

use super::bottom::{Btn, Slot, places};
use super::top::{order, order_x};
use super::{CLIENT, Field, Layout};
use crate::app::Frame;
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

/// Draw the frame again (the window body polls its jobs while drawn).
fn redraw(view: &Entity<Frame>, cx: &mut VisualTestContext) {
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
}

/// Draw until the demodulator of `node` has no work left.
fn settle(view: &Entity<Frame>, cx: &mut VisualTestContext, node: NodeId) {
    let start = Instant::now();
    loop {
        redraw(view, cx);
        let busy = view.read_with(cx, |f, _| {
            !f.jobs.is_empty() || f.demod.nodes.get(&node).is_some_and(|d| d.st.is_running())
        });
        if !busy || start.elapsed() > Duration::from_secs(60) {
            break;
        }
        if view.read_with(cx, |f, _| f.jobs.iter().all(|j| j.handle.is_finished())) {
            view.update(cx, Frame::poll);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    redraw(view, cx);
}

/// Load the generated MFSK-8 recording with a measured Br of 75 Bd and the
/// eight tones WF found; open its demodulator. Returns the node and window.
fn open(view: &Entity<Frame>, cx: &mut VisualTestContext) -> (NodeId, u32) {
    let node = view.update(cx, |f, cx| {
        let op = Operation::new("load", json!({"path": "/data/mfsk8.wav"}));
        let signal = crate::app::sono::test_signal().unwrap();
        let id = f.on_loaded("mfsk8.wav".into(), op, signal, &[]);
        let m = Measurement {
            value: 75.0,
            uncertainty: None,
            source: Some(id),
            tool: "test".into(),
        };
        f.measures.record(MeasurementKind::BaudRate, m).unwrap();
        let tones: Vec<f64> = (0..8).map(|i| 1062.5 + 125.0 * f64::from(i)).collect();
        let u = f.ui.get_mut(&id).unwrap();
        u.kept.insert(sn_session::UI_KEY_TONES.into(), json!(tones));
        u.kept
            .insert(sn_session::UI_KEY_TONE_GRID.into(), json!({"size": 8}));
        cx.notify();
        id
    });
    cx.run_until_parked();
    let win = view.update(cx, |f, cx| {
        let caller = f.find_window(node, WinKind::Sonogram);
        let w = f.open_demod(node, None, caller).unwrap();
        cx.notify();
        w
    });
    settle(view, cx, node);
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

/// Window position of client point `(x, y)` of window `win`.
fn at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) -> Px {
    let (_, (ox, oy)) = geometry(view, cx, win);
    point(px((ox + x) as f32), px((oy + y) as f32))
}

/// The middle of bottom bar button `b`.
fn button(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, b: Btn) -> Px {
    let (l, _) = geometry(view, cx, win);
    let x = places(l.bottom.x + 2)
        .into_iter()
        .find(|(s, _, _)| *s == Slot::Button(b))
        .map(|(_, x, _)| x)
        .unwrap();
    let y = l.bottom.y + (l.bottom.h - TOOL) / 2;
    at(view, cx, win, x + TOOL / 2, y + TOOL / 2)
}

fn press(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, b: Btn) {
    let p = button(view, cx, win, b);
    click(cx, p);
}

fn right_click_at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) {
    let p = at(view, cx, win, x, y);
    right_click(cx, p);
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

/// The command id of the open popup menu's item `label`.
fn popup_command(view: &Entity<Frame>, cx: &mut VisualTestContext, label: &str) -> u32 {
    view.read_with(cx, |f, _| {
        f.popup
            .as_ref()
            .and_then(|p| {
                p.items.iter().find_map(|i| match i.kind {
                    Kind::Command(c) if crate::win95::text::mnemonic(&i.label).text == label => {
                        Some(c)
                    }
                    _ => None,
                })
            })
            .unwrap()
    })
}

fn run_command(view: &Entity<Frame>, cx: &mut VisualTestContext, id: u32) {
    view.update(cx, |f, cx| {
        f.popup = None;
        f.demod_command(id, cx);
        cx.notify();
    });
    cx.run_until_parked();
}

#[gpui::test]
fn the_top_bar_follows_sa_6_2_3_8_and_the_window_is_titled_f1(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (_, win) = open(&view, cx);
    let title = view.read_with(cx, |f, _| f.mdi.get(win).unwrap().title.clone());
    assert_eq!(title, "[TempDir] mfsk8(0)(f1).wav");
    assert_eq!(view.read_with(cx, |f, _| f.client_of(win)), CLIENT);
    // Left to right: RRC, cutoff / alpha, Br and its seven pairs, A / Exactly
    // / Roughly, Levels / Raw; every group starts right of the one before.
    let (ids, xs) = (order(win), order_x());
    assert_eq!(ids.len(), xs.len());
    let br = ids.iter().position(|i| *i == Field::Br.id(win)).unwrap();
    let auto = ids
        .iter()
        .position(|i| *i == format!("dm{win}.auto"))
        .unwrap();
    let levels = ids
        .iter()
        .position(|i| *i == Field::Levels.id(win))
        .unwrap();
    assert!(xs[br] > xs[0] + 100 && xs[auto] > xs[br + 7] && xs[levels] > xs[auto] + 200);
    assert!(xs[br + 1..br + 8].windows(2).all(|w| w[1] > w[0]));
    // The Levels group ends inside the plot's width (the text panel follows).
    let (l, _) = geometry(&view, cx, win);
    assert!(
        xs[levels] + 60 <= l.text.right(),
        "{} {}",
        xs[levels],
        l.text.x
    );
    // Tab moves through the controls in the same order.
    // Tab moves through the controls in the same order; the alpha field is
    // grey (skipped) until RRC is checked.
    let tab = |view: &Entity<Frame>, cx: &mut VisualTestContext| -> Vec<String> {
        view.read_with(cx, |f, _| {
            f.kit
                .tab_order(&format!("mdi{win}"))
                .into_iter()
                .map(|s| s.to_string())
                .filter(|s| s.starts_with(&format!("dm{win}.")))
                .collect()
        })
    };
    let names = |n: &[&str]| -> Vec<String> { n.iter().map(|n| format!("dm{win}.{n}")).collect() };
    assert_eq!(
        tab(&view, cx),
        names(&["rrc", "cutoff", "br", "auto", "levels", "raw", "page"])
    );
    let rrc = at(&view, cx, win, order_x()[0] + 4, 3 + 10);
    click(cx, rrc);
    assert!(view.read_with(cx, |f, _| f.demod.nodes.values().all(|d| d.st.rrc)));
    assert_eq!(
        tab(&view, cx),
        names(&[
            "rrc", "cutoff", "alpha", "br", "auto", "levels", "raw", "page"
        ])
    );
}

#[gpui::test]
fn prefill_levels_markers_and_the_text_panel(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open(&view, cx);
    // Pre-filled from the measured Br, the tone grid and the tones.
    let (baud, levels, cutoff, markers, zones) = view.read_with(cx, |f, _| {
        let st = &f.demod.nodes[&node].st;
        (
            st.baud,
            st.levels,
            st.cutoff.clone(),
            st.markers.unwrap(),
            st.result().map(|d| d.layout.order()),
        )
    });
    assert!((baud - 75.0).abs() < 1e-9);
    assert_eq!((levels, cutoff.as_str(), zones), (8, "75", Some(8)));
    assert!((markers.0 - 1000.0).abs() < 1e-6 && (markers.1 - 2000.0).abs() < 1e-6);
    // The fields show the state.
    let (br_text, levels_text) = view.read_with(cx, |f, _| {
        let d = &f.demod.nodes[&node];
        (d.br.text.clone(), d.levels.edit.text.clone())
    });
    assert_eq!(br_text, sn_ui_core::format::dec(75.0, 5));
    assert_eq!(levels_text, "8");
    // D puts the symbols into the text panel in SA's format.
    press(&view, cx, win, Btn::D);
    let text = view.update(cx, |f, _| {
        sn_ui_core::demod::panel_text(&mut f.demod_win(node).st, 8000.0)
    });
    let head = format!(
        "SA - MFSK dem\nLevels: 8\nFd: 8000\nBr: {}\nData as: Default map\nData:\n",
        sn_ui_core::format::dec(75.0, 1)
    );
    assert!(text.starts_with(&head), "{text}");
    let data = text.lines().nth(6).unwrap();
    assert!(
        data.len() > 300 && data.chars().all(|c| ('0'..='7').contains(&c)),
        "{data}"
    );
    // Raw: hex bytes each followed by x.
    let raw = at(&view, cx, win, order_x()[order(win).len() - 1] + 4, 37);
    click(cx, raw);
    let (form, data) = view.update(cx, |f, _| {
        let st = &mut f.demod_win(node).st;
        (st.data_as, st.data().unwrap().to_string())
    });
    assert_eq!(form, DataAs::Raw);
    assert!(
        data.starts_with('0') && data.contains("x0") && data.ends_with('x'),
        "{data}"
    );
    // The Levels spin edit's up button adds a level and the decisions follow.
    let (l, _) = geometry(&view, cx, win);
    let up = at(
        &view,
        cx,
        win,
        order_x()[order(win).len() - 2] + 56 - 8,
        l.top.y + 3 + 5,
    );
    click(cx, up);
    settle(&view, cx, node);
    let (n, zones) = view.read_with(cx, |f, _| {
        let st = &f.demod.nodes[&node].st;
        (st.levels, st.result().map(|d| d.layout.order()))
    });
    assert_eq!((n, zones), (9, Some(9)));
}

#[gpui::test]
fn dragging_a_main_marker_and_the_plot_menu(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open(&view, cx);
    let (l, _) = geometry(&view, cx, win);
    // The top main marker's line, then a drag 20 px up.
    let (y_top, lines) = view.read_with(cx, |f, _| {
        let st = &f.demod.nodes[&node].st;
        let range = st.shown_range();
        let (_, t) = st.markers.unwrap();
        let h = l.plot.h as f32;
        let y = h - ((t - range.0) / (range.1 - range.0)) as f32 * h;
        (
            y.round() as i32,
            boundaries(st.result().unwrap().layout.centres()).len(),
        )
    });
    assert_eq!(lines, 9, "8 zones: their outer edges and 7 thresholds");
    let before = view.read_with(cx, |f, _| f.demod.nodes[&node].st.markers.unwrap());
    let range = view.read_with(cx, |f, _| f.demod.nodes[&node].st.shown_range());
    let from = at(&view, cx, win, l.plot.x + 200, l.plot.y + y_top);
    let none = Modifiers::none();
    cx.simulate_mouse_down(from, MouseButton::Left, none);
    // Several small moves: the marker must follow the pointer, not run away.
    for k in 1..=4 {
        let p = at(&view, cx, win, l.plot.x + 200, l.plot.y + y_top - 5 * k);
        cx.simulate_mouse_move(p, MouseButton::Left, none);
    }
    let to = at(&view, cx, win, l.plot.x + 200, l.plot.y + y_top - 20);
    cx.simulate_mouse_up(to, MouseButton::Left, none);
    cx.run_until_parked();
    let after = view.read_with(cx, |f, _| f.demod.nodes[&node].st.markers.unwrap());
    assert!((after.0 - before.0).abs() < 1e-9, "the bottom one stays");
    assert!(after.1 > before.1 + 10.0, "{before:?} {after:?}");
    let px_value = (range.1 - range.0) / f64::from(l.plot.h);
    let expected = before.1 + 20.0 * px_value;
    assert!(
        (after.1 - expected).abs() <= 2.0 * px_value,
        "under the pointer: {expected} vs {after:?}"
    );
    settle(&view, cx, node);
    // The plot's popup menu: SigNexora's settings, then Cancel.
    right_click_at(&view, cx, win, l.plot.x + 100, l.plot.y + 100);
    assert_eq!(
        popup_labels(&view, cx),
        [
            "Levels at the found tones (8)",
            "Average filter",
            "Adaptive levels",
            "-",
            "Info",
            "Reset the vertical scale",
            "-",
            "Cancel"
        ]
    );
    let id = popup_command(&view, cx, "Levels at the found tones (8)");
    run_command(&view, cx, id);
    settle(&view, cx, node);
    let from_tones = view.read_with(cx, |f, _| f.demod.nodes[&node].st.from_tones);
    assert!(from_tones);
    // The text panel's menu.
    right_click_at(&view, cx, win, l.text.x + 50, l.text.y + 50);
    assert_eq!(
        popup_labels(&view, cx),
        ["Copy", "Save as...", "-", "Data as", "-", "Cancel"]
    );
}

#[gpui::test]
fn navigation_and_scale_buttons(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open(&view, cx);
    let pages = view.read_with(cx, |f, _| f.demod.nodes[&node].st.page_count());
    assert!(pages >= 4, "{pages}");
    press(&view, cx, win, Btn::Forward);
    press(&view, cx, win, Btn::ForwardFast);
    redraw(&view, cx);
    let (page, text) = view.read_with(cx, |f, _| {
        let d = &f.demod.nodes[&node];
        (d.st.page, d.page.text.clone())
    });
    assert_eq!(page, pages - 1, "forward 10 stops at the last page");
    assert_eq!(text, pages.to_string());
    press(&view, cx, win, Btn::BackFast);
    assert_eq!(view.read_with(cx, |f, _| f.demod.nodes[&node].st.page), 0);
    press(&view, cx, win, Btn::ScaleUp);
    let (base, shown) = view.read_with(cx, |f, _| {
        let st = &f.demod.nodes[&node].st;
        (st.base_range(), st.shown_range())
    });
    assert!(shown.1 - shown.0 < base.1 - base.0, "zoomed in");
    press(&view, cx, win, Btn::Filter);
    assert!(view.read_with(cx, |f, _| f.demod.nodes[&node].st.rrc));
    press(&view, cx, win, Btn::Revert);
    assert!(!view.read_with(cx, |f, _| f.demod.nodes[&node].st.rrc));
}

#[gpui::test]
fn the_br_field_applies_on_enter_and_when_left(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open(&view, cx);
    let ids = order(win);
    let br = ids.iter().position(|i| *i == Field::Br.id(win)).unwrap();
    let field = at(&view, cx, win, order_x()[br] + 20, 25);
    click(cx, field);
    cx.simulate_keystrokes("cmd-a 1 2 0 0 tab");
    redraw(&view, cx);
    let baud = view.read_with(cx, |f, _| f.demod.nodes[&node].st.baud);
    assert!(
        (baud - 1200.0).abs() < 1e-9,
        "leaving the field applies: {baud}"
    );
    // A refused Br on Enter: SA's error box, the Br in use kept.
    click(cx, field);
    cx.simulate_keystrokes("cmd-a x enter");
    let (baud, refused, boxed) = view.read_with(cx, |f, _| {
        let st = &f.demod.nodes[&node].st;
        (st.baud, st.br_refused.is_some(), f.msgbox.is_some())
    });
    assert!((baud - 1200.0).abs() < 1e-9);
    assert!(refused && boxed);
    // A spin pair steps the digit it stands for (the fourth: 1 Bd).
    view.update(cx, |f, cx| {
        f.msgbox = None;
        cx.notify();
    });
    cx.run_until_parked();
    // Typed into the field, not applied yet, then a spin pair.
    click(cx, field);
    cx.simulate_keystrokes("cmd-a 1 2 0 0");
    let pair = at(&view, cx, win, order_x()[br + 4] + 7, 15 + 5);
    click(cx, pair);
    let baud = view.read_with(cx, |f, _| f.demod.nodes[&node].st.baud);
    assert!((baud - 1201.0).abs() < 1e-9, "{baud}");
    // The field still has the focus; leaving it must not put back its old
    // text.
    redraw(&view, cx);
    let focused = view.read_with(cx, |f, _| f.kit.has_focus(&Field::Br.id(win)));
    assert!(focused, "the pair does not take the focus");
    cx.simulate_keystrokes("tab");
    redraw(&view, cx);
    let (baud, boxed) = view.read_with(cx, |f, _| {
        (f.demod.nodes[&node].st.baud, f.msgbox.is_some())
    });
    assert!((baud - 1201.0).abs() < 1e-9, "{baud}");
    assert!(!boxed, "no new refusal");
}

#[gpui::test]
fn wf_opens_the_demodulator_on_its_fragment(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, _) = open(&view, cx);
    // The diamond button's API: the fragment becomes the V markers, and the
    // window that is open comes to the front (no second one).
    let (v, wins) = view.update(cx, |f, cx| {
        f.open_demod(node, Some((800, 24_000)), None);
        cx.notify();
        let wins = f
            .signal_windows()
            .into_iter()
            .filter(|(_, _, k)| *k == WinKind::Demod)
            .count();
        (f.ui[&node].markers.v, wins)
    });
    assert_eq!(v, Some((800.0, 24_000.0)));
    assert_eq!(wins, 1);
    settle(&view, cx, node);
    // Closing the window frees the work; the settings stay for the next one.
    let win = view.read_with(cx, |f, _| f.find_window(node, WinKind::Demod).unwrap());
    view.update(cx, |f, cx| {
        f.close_window_now(win);
        cx.notify();
    });
    redraw(&view, cx);
    let (levels, track) = view.read_with(cx, |f, _| {
        let st = &f.demod.nodes[&node].st;
        (st.levels, st.track().is_some())
    });
    assert_eq!((levels, track), (8, false));
}
