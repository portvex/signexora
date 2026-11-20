//! The widget gallery (debug builds only): one MDI child showing every widget
//! of the kit, to check the look and behaviour by eye.

use std::rc::Rc;

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};

use super::Frame;
use crate::win95::bars::{Tool, tool_button};
use crate::win95::button::{Ctl, check_box, push_button, radio_button};
use crate::win95::combo::{ComboState, combo_box};
use crate::win95::dialog::{Buttons, MsgBox};
use crate::win95::edit::{EditState, SpinState, edit, spin_edit};
use crate::win95::geom::Rect;
use crate::win95::glyph::{BLACK, Glyph};
use crate::win95::kit::{Lens, action, lens};
use crate::win95::label::{Align, group_box, label};
use crate::win95::marks::Icon;
use crate::win95::scrollbar::{Bar, ScrollState, scroll_bar};

/// Client size of the gallery window.
pub(crate) const SIZE: (i32, i32) = (528, 300);

/// The gallery's widget states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Gallery {
    pub(crate) alc: bool,
    pub(crate) hres: bool,
    pub(crate) radio: usize,
    pub(crate) name: EditState,
    pub(crate) path: EditState,
    pub(crate) size_fft: SpinState,
    pub(crate) win_f: ComboState,
    pub(crate) image: ComboState,
    pub(crate) hbar: ScrollState,
    pub(crate) vbar: ScrollState,
    pub(crate) clicks: u32,
    pub(crate) phase: bool,
}

impl Default for Gallery {
    fn default() -> Self {
        let images: Vec<String> = (0..16).map(|i| format!("- {} db", 20 + 5 * i)).collect();
        let image_refs: Vec<&str> = images.iter().map(String::as_str).collect();
        let mut path = EditState::new("D:\\Signals\\test(0).wav");
        path.read_only = true;
        Self {
            alc: true,
            hres: false,
            radio: 0,
            name: EditState::new("8000"),
            path,
            size_fft: SpinState::new(1024, 64, 65536, 64),
            win_f: ComboState::new(
                &["Rectangle", "Hann", "Hamming", "Blackman", "Kaiser, b=8"],
                Some(4),
            ),
            image: ComboState::new(&image_refs, Some(6)),
            hbar: ScrollState::new(0, 999, 100, 250),
            vbar: ScrollState::new(0, 99, 20, 0),
            clicks: 0,
            phase: false,
        }
    }
}

fn g<S: 'static>(f: impl Fn(&mut Gallery) -> &mut S + 'static) -> Lens<Frame, S> {
    lens(move |v: &mut Frame| Some(f(&mut v.gallery)))
}

/// Show a sample message box.
fn sample_box(v: &mut Frame, icon: Icon) {
    let (title, text, buttons) = match icon {
        Icon::Error => (
            "SigNexora",
            "Cannot open the file:\nthe format is not supported.",
            Buttons::Ok,
        ),
        Icon::Warning => (
            "SigNexora",
            "The selection is shorter than one symbol.\nContinue anyway?",
            Buttons::OkCancel,
        ),
        Icon::Info => ("SigNexora", "Measurement finished.", Buttons::Ok),
        Icon::App => ("SigNexora", "Signal intelligence tool.", Buttons::Ok),
        Icon::Question => ("SigNexora", "Close all windows?", Buttons::YesNo),
    };
    v.show_message(MsgBox::new(title, text, Some(icon), buttons));
}

/// The gallery's client area.
pub(crate) fn view(
    v: &mut Frame,
    window: &mut Window,
    cx: &mut Context<'_, Frame>,
    scope: &str,
) -> AnyElement {
    let _ = window;
    let mut root = div().relative().size_full();
    for el in buttons_group(v, cx, scope)
        .into_iter()
        .chain(toggles_group(v, cx, scope))
        .chain(edits_group(v, cx, scope))
        .chain(bars_group(v, cx))
    {
        root = root.child(el);
    }
    root.into_any_element()
}

fn buttons_group(v: &mut Frame, cx: &mut Context<'_, Frame>, scope: &str) -> Vec<AnyElement> {
    let mut out = vec![group_box(cx, Rect::new(8, 4, 252, 84), "Push buttons")];
    let kit = &mut v.kit;
    let click = action(|v: &mut Frame, _, _| v.gallery.clicks += 1);
    out.push(push_button(
        kit,
        cx,
        Ctl {
            id: "g.ok",
            scope,
            rect: Rect::new(16, 22, 75, 23),
            label: "OK",
            enabled: true,
        },
        true,
        click.clone(),
    ));
    out.push(push_button(
        kit,
        cx,
        Ctl {
            id: "g.cancel",
            scope,
            rect: Rect::new(97, 22, 75, 23),
            label: "Cancel",
            enabled: true,
        },
        false,
        click,
    ));
    out.push(push_button(
        kit,
        cx,
        Ctl {
            id: "g.off",
            scope,
            rect: Rect::new(178, 22, 75, 23),
            label: "&Disabled",
            enabled: false,
        },
        false,
        action(|_, _, _| {}),
    ));
    let boxes = [
        ("g.err", "&Error...", Icon::Error),
        ("g.warn", "&Warning...", Icon::Warning),
        ("g.info", "&Information...", Icon::Info),
        ("g.quest", "&Question...", Icon::Question),
    ];
    for (i, (id, text, icon)) in (0_i32..).zip(boxes) {
        let r = Rect::new(16 + i * 81, 54, 75, 23);
        out.push(push_button(
            kit,
            cx,
            Ctl {
                id,
                scope,
                rect: r,
                label: text,
                enabled: true,
            },
            false,
            action(move |v, _, _| sample_box(v, icon)),
        ));
    }
    out
}

fn toggles_group(v: &mut Frame, cx: &mut Context<'_, Frame>, scope: &str) -> Vec<AnyElement> {
    let gs = v.gallery.clone();
    let kit = &mut v.kit;
    let mut out = vec![group_box(
        cx,
        Rect::new(8, 94, 252, 96),
        "Check boxes and radio buttons",
    )];
    let ctl = |id, x, y, label, enabled| Ctl {
        id,
        scope,
        rect: Rect::new(x, y, 110, 17),
        label,
        enabled,
    };
    out.push(check_box(
        kit,
        cx,
        ctl("g.alc", 16, 112, "&ALC", true),
        gs.alc,
        Rc::new(|v: &mut Frame, on, _, _| v.gallery.alc = on),
    ));
    out.push(check_box(
        kit,
        cx,
        ctl("g.hres", 16, 132, "&HRes", true),
        gs.hres,
        Rc::new(|v: &mut Frame, on, _, _| v.gallery.hres = on),
    ));
    out.push(check_box(
        kit,
        cx,
        ctl("g.lock", 16, 152, "Locked", false),
        true,
        Rc::new(|_: &mut Frame, _, _, _| {}),
    ));
    for (i, (id, text)) in [("g.r0", "&Time"), ("g.r1", "An&gle"), ("g.r2", "&Strobe")]
        .into_iter()
        .enumerate()
    {
        let c = ctl(id, 136, 112 + 20 * i as i32, text, i != 2 || gs.alc);
        out.push(radio_button(
            kit,
            cx,
            c,
            gs.radio == i,
            action(move |v: &mut Frame, _, _| v.gallery.radio = i),
        ));
    }
    out.push(label(
        cx,
        Rect::new(136, 170, 120, 13),
        &format!("Clicks: {}", gs.clicks),
        false,
        true,
        Align::Center,
    ));
    out
}

fn edits_group(v: &mut Frame, cx: &mut Context<'_, Frame>, scope: &str) -> Vec<AnyElement> {
    let mut out = vec![group_box(
        cx,
        Rect::new(268, 4, 252, 186),
        "Edits and lists",
    )];
    let labels = [
        ("&Rate:", false, true, 22),
        ("Path:", false, true, 48),
        ("SizeFFT", true, true, 74),
        ("WinF", true, true, 100),
        ("Image", true, true, 126),
        ("Fl", true, false, 152),
    ];
    for (text, bold, enabled, y) in labels {
        out.push(label(
            cx,
            Rect::new(276, y, 56, 21),
            text,
            bold,
            enabled,
            Align::Right,
        ));
    }
    out.extend(edit_fields(v, cx, scope));
    out
}

fn edit_fields(v: &mut Frame, cx: &mut Context<'_, Frame>, scope: &str) -> Vec<AnyElement> {
    let gs = v.gallery.clone();
    let kit = &mut v.kit;
    let row = |y| Rect::new(340, y, 172, 21);
    let ctl = |id, rect, enabled| Ctl {
        id,
        scope,
        rect,
        label: "",
        enabled,
    };
    let fl = EditState::new("0");
    vec![
        edit(
            kit,
            cx,
            ctl("g.name", row(22), true),
            &gs.name,
            g(|g| &mut g.name),
            None,
        ),
        edit(
            kit,
            cx,
            ctl("g.path", row(48), true),
            &gs.path,
            g(|g| &mut g.path),
            None,
        ),
        spin_edit(
            kit,
            cx,
            ctl("g.fft", Rect::new(340, 74, 90, 21), true),
            &gs.size_fft,
            &g(|g| &mut g.size_fft),
            None,
        ),
        combo_box(
            kit,
            cx,
            ctl("g.winf", row(100), true),
            &gs.win_f,
            g(|g| &mut g.win_f),
            None,
        ),
        combo_box(
            kit,
            cx,
            ctl("g.image", row(126), true),
            &gs.image,
            g(|g| &mut g.image),
            None,
        ),
        edit(
            kit,
            cx,
            ctl("g.fl", Rect::new(340, 152, 90, 21), false),
            &fl,
            g(|g| &mut g.name),
            None,
        ),
    ]
}

fn bars_group(v: &mut Frame, cx: &mut Context<'_, Frame>) -> Vec<AnyElement> {
    let gs = v.gallery.clone();
    let kit = &v.kit;
    let mut out = vec![group_box(
        cx,
        Rect::new(8, 196, 512, 96),
        "Scroll bars and toolbar",
    )];
    let h = Bar {
        id: "g.hbar",
        rect: Rect::new(16, 214, 300, 16),
        vertical: false,
    };
    out.push(scroll_bar(kit, cx, h, gs.hbar, g(|g| &mut g.hbar), None));
    let vb = Bar {
        id: "g.vbar",
        rect: Rect::new(496, 212, 16, 74),
        vertical: true,
    };
    out.push(scroll_bar(kit, cx, vb, gs.vbar, g(|g| &mut g.vbar), None));
    let off = Bar {
        id: "g.offbar",
        rect: Rect::new(16, 236, 300, 16),
        vertical: false,
    };
    out.push(scroll_bar(
        kit,
        cx,
        off,
        ScrollState::new(0, 5, 10, 0),
        g(|g| &mut g.hbar),
        None,
    ));
    let pos = format!("Position: {} / {}", gs.hbar.pos, gs.vbar.pos);
    out.push(label(
        cx,
        Rect::new(324, 214, 160, 16),
        &pos,
        false,
        true,
        Align::Left,
    ));
    let tools: [(&str, Glyph, bool, &str); 5] = [
        ("g.t.open", super::menus::folder_glyph(), true, "Open file"),
        (
            "g.t.save",
            super::menus::floppy_glyph(),
            true,
            "Save session",
        ),
        (
            "g.t.close",
            super::menus::close_all_glyph(),
            true,
            "Close all windows",
        ),
        (
            "g.t.ph",
            Glyph::new(16, 16).text(1, 2, "Ph", true, BLACK),
            true,
            "Phase detector (toggle)",
        ),
        (
            "g.t.off",
            Glyph::new(16, 16).text(4, 2, "R", true, BLACK),
            false,
            "Disabled",
        ),
    ];
    for (i, (id, glyph, enabled, tip)) in (0_i32..).zip(tools.iter()) {
        let checked = *id == "g.t.ph" && gs.phase;
        let t = Tool {
            id,
            at: (16 + i * 22, 262),
            glyph,
            enabled: *enabled,
            checked,
            tip,
        };
        let toggle = *id == "g.t.ph";
        out.push(tool_button(
            kit,
            cx,
            t,
            action(move |v: &mut Frame, _, _| {
                v.gallery.clicks += 1;
                if toggle {
                    v.gallery.phase = !v.gallery.phase;
                }
            }),
        ));
    }
    out
}
