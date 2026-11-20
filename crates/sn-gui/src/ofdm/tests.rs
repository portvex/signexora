//! Behaviour tests of the `SA-OFDM` window on GPUI's test platform, on the
//! OCG test signal of article 371 (`ofdm_lu64_lg20_7000`: LU 64, LG 20,
//! Fd 7000): the mouse and keyboard are simulated inside the test app only.

use std::time::{Duration, Instant};

use gpui::{Entity, Modifiers, MouseButton, Point, TestAppContext, VisualTestContext, point, px};
use serde_json::json;
use sn_core::{MeasurementKind, NodeId, Operation};
use sn_ui_core::ofdm::{KSort, Line};

use super::right::{button_rects, list_rect};
use super::{CLIENT, Layout};
use crate::app::Frame;
use crate::app::model::WinKind;
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

fn redraw(view: &Entity<Frame>, cx: &mut VisualTestContext) {
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
}

/// Draw until the module's job (and the app's jobs) ended.
fn settle(view: &Entity<Frame>, cx: &mut VisualTestContext, node: NodeId) {
    let start = Instant::now();
    loop {
        redraw(view, cx);
        let busy = view.read_with(cx, |f, _| {
            !f.jobs.is_empty() || f.ofdm.nodes.get(&node).is_some_and(|o| o.st.is_running())
        });
        if !busy || start.elapsed() > Duration::from_secs(120) {
            break;
        }
        if view.read_with(cx, |f, _| f.jobs.iter().all(|j| j.handle.is_finished())) {
            view.update(cx, Frame::poll);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    redraw(view, cx);
}

/// Load the OCG preset and open `SA-OFDM` from the sonogram's OFDM button.
fn open(view: &Entity<Frame>, cx: &mut VisualTestContext) -> (NodeId, u32) {
    let node = view.update(cx, |f, cx| {
        let spec = sn_synth::GeneratorSpec::preset("ofdm_lu64_lg20_7000").unwrap();
        let signal = sn_synth::generate(&spec).unwrap().signal;
        let op = Operation::new("load", json!({"path": "/data/ocg.wav"}));
        let id = f.on_loaded("ocg.wav".into(), op, signal, &[]);
        cx.notify();
        id
    });
    settle(view, cx, node);
    let sono = view.read_with(cx, |f, _| f.find_window(node, WinKind::Sonogram).unwrap());
    view.update(cx, |f, cx| {
        f.press_button(sono, sn_ui_core::sono_tools::Button::Ofdm);
        cx.notify();
    });
    settle(view, cx, node);
    let win = view.read_with(cx, |f, _| f.find_window(node, WinKind::Ofdm).unwrap());
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

fn at(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, x: i32, y: i32) -> Px {
    let (_, (ox, oy)) = geometry(view, cx, win);
    point(px((ox + x) as f32), px((oy + y) as f32))
}

fn click(cx: &mut VisualTestContext, p: Px) {
    cx.simulate_click(p, Modifiers::none());
    cx.run_until_parked();
}

/// Click the middle of client rectangle `r`.
fn click_rect(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, r: Rect) {
    let p = at(view, cx, win, r.x + r.w / 2, r.y + r.h / 2);
    click(cx, p);
}

/// Press `ADP`, `Show` / `Break` or `Find CT`.
fn press(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, node: NodeId, name: &str) {
    let (l, _) = geometry(view, cx, win);
    let stoppable = view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.stoppable());
    let r = cx.update(|window, _| {
        let m = |s: &str| crate::win95::text::width(window, s, false);
        button_rects(l.right, stoppable, &m)
            .into_iter()
            .find(|(n, _, _)| *n == name)
            .map(|(_, _, r)| r)
            .unwrap()
    });
    click_rect(view, cx, win, r);
}

/// Right click on list row `row` and return the menu's labels.
fn list_menu(view: &Entity<Frame>, cx: &mut VisualTestContext, win: u32, row: i32) -> Vec<String> {
    view.update(cx, |f, cx| {
        f.mdi.activate(win);
        cx.notify();
    });
    cx.run_until_parked();
    let (l, _) = geometry(view, cx, win);
    // The module of the window under test (another signal may have one).
    let panes = view.read_with(cx, |f, _| {
        f.node_of(win)
            .and_then(|n| f.ofdm.nodes.get(&n))
            .is_some_and(|o| o.st.panes)
    });
    let r = list_rect(l.right, panes);
    let y = r.y + 2 + row * crate::app::demod::ctl::CELL_H + 5;
    let p = at(view, cx, win, r.x + 40, y);
    cx.simulate_mouse_down(p, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(p, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
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

/// Run the open popup menu's item `label`.
fn choose(view: &Entity<Frame>, cx: &mut VisualTestContext, label: &str) {
    view.update(cx, |f, cx| {
        let id = f
            .popup
            .as_ref()
            .and_then(|p| {
                p.items.iter().find_map(|i| match i.kind {
                    Kind::Command(c) if crate::win95::text::mnemonic(&i.label).text == label => {
                        Some(c)
                    }
                    _ => None,
                })
            })
            .unwrap();
        f.popup = None;
        f.ofdm_command(id, cx);
        cx.notify();
    });
    cx.run_until_parked();
}

/// Open the window and run Find CT and ADP with the buttons.
fn after_adp(view: &Entity<Frame>, cx: &mut VisualTestContext) -> (NodeId, u32) {
    let (node, win) = open(view, cx);
    press(view, cx, win, node, "find");
    settle(view, cx, node);
    press(view, cx, win, node, "adp");
    settle(view, cx, node);
    (node, win)
}

#[gpui::test]
fn find_ct_then_adp_on_the_ocg_signal(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open(&view, cx);
    let title = view.read_with(cx, |f, _| f.mdi.get(win).unwrap().title.clone());
    assert_eq!(title, "SA-OFDM");
    assert_eq!(view.read_with(cx, |f, _| f.client_of(win)), CLIENT);
    // SA's start: Survey on, bounds 20 and Fd/20.
    let (survey, bounds) = view.read_with(cx, |f, _| {
        let st = &f.ofdm.nodes[&node].st;
        (st.survey, (st.min_br, st.max_br))
    });
    assert!(survey);
    assert_eq!(bounds, (20, 350));
    // ADP is grey before Find CT: pressing it does nothing (an enabled ADP
    // would answer with an error box).
    press(&view, cx, win, node, "adp");
    assert!(view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.br.is_none()));
    assert!(view.read_with(cx, |f, _| f.msgbox.is_none()));
    press(&view, cx, win, node, "find");
    let running = view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.running().map(|r| r.0));
    assert_eq!(
        running,
        Some(sn_ui_core::ofdm::Work::FindCt),
        "Don't panic! Please wait..."
    );
    settle(&view, cx, node);
    let (ct, survey, last) = view.read_with(cx, |f, _| {
        let st = &f.ofdm.nodes[&node].st;
        let ct = st.ct.clone().unwrap();
        ((ct.ls, ct.lu, ct.lg), st.survey, st.lines.last().cloned())
    });
    assert_eq!(ct, (84, 64, 20));
    assert!(!survey, "Find CT leaves the Survey view");
    assert!(matches!(last, Some(Line::Finish { stopped: false, .. })));
    press(&view, cx, win, node, "adp");
    settle(&view, cx, node);
    let (br_text, info) = view.read_with(cx, |f, _| {
        let o = &f.ofdm.nodes[&node];
        (o.br.text.clone(), o.st.info_lines(7000.0))
    });
    assert!(br_text.starts_with("83"), "{br_text}");
    assert!(
        info.iter().any(|l| l.starts_with("Magic K = 5/16")),
        "{info:?}"
    );
    // ADP records Br for the other modules.
    let br = view.read_with(cx, |f, _| {
        f.measured(&MeasurementKind::BaudRate).unwrap().value
    });
    assert!((br - 7000.0 / 84.0).abs() < 0.05, "{br}");
}

#[gpui::test]
fn list_of_k_sorts_and_resamples_into_an_r_window(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = after_adp(&view, cx);
    // List of K: the button right of Survey, the last of the top bar.
    let (l, _) = geometry(&view, cx, win);
    let (item, x, w) = cx.update(|window, _| {
        let m = |s: &str| crate::win95::text::width(window, s, false);
        *super::top::places(l.top.x + 4, &m).last().unwrap()
    });
    assert_eq!(item, super::top::Item::ListK);
    click_rect(&view, cx, win, Rect::new(x, l.top.y + 4, w, 20));
    settle(&view, cx, node);
    assert!(view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.k_shown));
    let menu = list_menu(&view, cx, win, 0);
    assert_eq!(&menu[..3], ["Sort by SR", "Sort by K", "Sort by AM"]);
    assert!(menu[4].starts_with("Resample to "), "{menu:?}");
    assert_eq!(&menu[5..], ["-", "Close", "-", "Cancel"]);
    choose(&view, cx, "Sort by SR");
    let rates: Vec<f64> = view.read_with(cx, |f, _| {
        let st = &f.ofdm.nodes[&node].st;
        assert_eq!(st.k_sort, KSort::Sr);
        st.k_list().iter().map(|(_, c)| c.sample_rate).collect()
    });
    assert!(rates.windows(2).all(|w| w[0] <= w[1]), "{rates:?}");
    let _ = list_menu(&view, cx, win, 0);
    choose(&view, cx, "Sort by K");
    let ks: Vec<f64> = view.read_with(cx, |f, _| {
        f.ofdm.nodes[&node]
            .st
            .k_list()
            .iter()
            .map(|(_, c)| c.k)
            .collect()
    });
    assert!(ks.windows(2).all(|w| w[0] <= w[1]), "{ks:?}");
    // Resample to the first line's rate: a new `(r_...)` window.
    let menu = list_menu(&view, cx, win, 0);
    let item = menu
        .iter()
        .find(|m| m.starts_with("Resample to "))
        .unwrap()
        .clone();
    choose(&view, cx, &item);
    settle(&view, cx, node);
    let titles: Vec<String> = view.read_with(cx, |f, _| {
        f.signal_windows()
            .into_iter()
            .filter_map(|(id, _, _)| f.mdi.get(id).map(|c| c.title.clone()))
            .collect()
    });
    assert!(
        titles.last().unwrap().starts_with("[TempDir] ocg(0)(r_"),
        "{titles:?}"
    );
    // Close hides the List of K again.
    let _ = list_menu(&view, cx, win, 0);
    choose(&view, cx, "Close");
    assert!(!view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.k_shown));
}

#[gpui::test]
fn show_becomes_break_and_the_arrows_move_the_green_marker(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = after_adp(&view, cx);
    view.update(cx, |f, _| {
        let st = &mut f.ofdm_win(node).st;
        st.sel = 5;
        st.other = 3;
    });
    press(&view, cx, win, node, "show");
    let start = Instant::now();
    while view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.show.is_none())
        && start.elapsed() < Duration::from_secs(60)
    {
        redraw(&view, cx);
        std::thread::sleep(Duration::from_millis(5));
    }
    let (playing, panes) = view.read_with(cx, |f, _| {
        let st = &f.ofdm.nodes[&node].st;
        (st.playing, st.panes)
    });
    assert!(playing && panes, "Show plays and shows the constellations");
    let (l, _) = geometry(&view, cx, win);
    let caption = cx.update(|window, _| {
        let m = |s: &str| crate::win95::text::width(window, s, false);
        button_rects(l.right, true, &m)[1].1.clone()
    });
    assert_eq!(caption, "Break");
    press(&view, cx, win, node, "show");
    assert!(
        !view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.playing),
        "Break stops"
    );
    // The arrow keys move the green channel marker.
    view.update(cx, |f, cx| {
        f.mdi.activate(win);
        f.kit.focus = None;
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("right right left right");
    let sel = view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.sel);
    assert_eq!(sel, 7);
    settle(&view, cx, node);
    let shown = view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.show.clone().unwrap());
    assert_eq!(shown.symbols.bins, vec![7, 3], "Show follows the marker");
}

#[gpui::test]
fn results_list_lines_and_survey_scroll_bar(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open(&view, cx);
    // The cyan scroll bar moves the Survey window by a quarter FFT per arrow.
    let (l, _) = geometry(&view, cx, win);
    let arrow = Rect::new(l.pos.right() - 8, l.pos.y + 4, 4, 4);
    click_rect(&view, cx, win, arrow);
    let pos = view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.survey_pos);
    assert!((pos - 256.0).abs() < 1e-9, "{pos}");
    view.update(cx, |f, cx| {
        f.ofdm_win(node).st.survey_pos = 0.0;
        cx.notify();
    });
    press(&view, cx, win, node, "find");
    settle(&view, cx, node);
    let lines = view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.lines.len());
    assert!(lines >= 2);
    // The results popup.
    let menu = list_menu(&view, cx, win, lines as i32 + 2);
    assert_eq!(menu, ["Clear the results", "-", "Cancel"]);
    choose(&view, cx, "Clear the results");
    assert!(view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.lines.is_empty()));
}

#[test]
fn whole_number_fields_refuse_text_on_enter() {
    let mut o = super::OfdmWin::new();
    let before = o.st.symbols;
    o.symbols.set_text("seven");
    let e = o.apply(super::OField::Symbols, true);
    assert!(e.as_deref().is_some_and(|e| e.contains("seven")), "{e:?}");
    assert_eq!(o.st.symbols, before, "the value in use stays");
    assert_eq!(
        o.symbols.text,
        before.to_string(),
        "the field shows it again"
    );
    o.symbols.set_text("1.5");
    assert_eq!(
        o.apply(super::OField::Symbols, false),
        None,
        "silent on blur"
    );
    // Min at its limit leaves room for Max inside the spin's range.
    o.min.edit.set_text("1000000");
    assert_eq!(o.apply(super::OField::Min, true), None);
    assert!(o.st.max_br <= 1_000_000 && o.st.min_br < o.st.max_br);
}

#[gpui::test]
fn clearing_the_results_stops_a_running_find_ct(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open(&view, cx);
    // The widest Br-LS range, so the search is still running when it is
    // cleared even on a loaded machine (the default range can finish first).
    view.update(cx, |f, _| {
        if let Some(o) = f.ofdm.nodes.get_mut(&node) {
            o.st.min_br = 1;
            o.st.max_br = 1_000_000;
        }
    });
    press(&view, cx, win, node, "find");
    assert!(view.read_with(cx, |f, _| f.ofdm.nodes[&node].st.is_running()));
    view.update(cx, |f, cx| {
        f.ofdm.popup_win = Some(win);
        f.ofdm_command(super::popup::OF_RESET, cx);
    });
    settle(&view, cx, node);
    view.read_with(cx, |f, _| {
        let st = &f.ofdm.nodes[&node].st;
        assert!(st.lines.is_empty(), "{:?}", st.lines);
        assert!(st.ct.is_none());
    });
}

#[gpui::test]
fn the_spectrum_pan_stays_inside_after_a_smaller_scale(cx: &mut TestAppContext) {
    let (view, cx) = frame(cx);
    let (node, win) = open(&view, cx);
    press(&view, cx, win, node, "find");
    settle(&view, cx, node);
    view.update(cx, |f, cx| {
        let o = f.ofdm.nodes.get_mut(&node).unwrap();
        o.st.pan = Some(1e9);
        cx.notify();
    });
    redraw(&view, cx);
    let (l, _) = geometry(&view, cx, win);
    view.read_with(cx, |f, _| {
        let o = &f.ofdm.nodes[&node];
        let signal = &f.graph.get(node).unwrap().signal;
        let (_, spec) = l.split();
        let bins = o.st.grid().map_or(0, |g| g.bins().len());
        let p = sn_ui_core::ofdm::pitch(o.st.scale_fft);
        let pan = super::plots::effective_pan(o.st.pan, signal, bins, p, f64::from(spec.w));
        let max = (bins as f64 * p - f64::from(spec.w)).max(0.0);
        assert!(pan <= max + 1e-9, "{pan} > {max}");
        assert_eq!(o.pan_bar.pos, o.pan_bar.top(), "the thumb shows the end");
    });
}
