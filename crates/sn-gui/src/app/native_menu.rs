//! The menu in the macOS menu bar, for the macOS looks on a Mac: SA's menus
//! (`File`, `Windows`, `Options`, `Help`) after the application menu, built
//! from the same items as the menu the window draws in the other looks.
//!
//! Each command item runs [`MenuCommand`] with its command id. GPUI's native
//! menus have no check marks, so a ticked or chosen item shows `* ` before
//! its name; a disabled item runs [`MenuDisabled`], which nothing handles, so
//! macOS greys it out.

use std::fmt::Write as _;

use gpui::{Action, App, Menu as NativeMenu, MenuItem};

use super::menus::cmd;
use crate::win95::menu::{Check, Item, Kind, Menu};
use crate::win95::text::mnemonic;
use sn_ui_core::t;

/// A menu command chosen in the macOS menu bar.
#[derive(Clone, Debug, PartialEq, Eq, Action)]
#[action(namespace = sa_rs, no_json)]
pub(crate) struct MenuCommand(pub(crate) u32);

/// A disabled item of the macOS menu bar (no handler, so macOS greys it).
#[derive(Clone, Debug, PartialEq, Eq, Action)]
#[action(namespace = sa_rs, no_json)]
pub(crate) struct MenuDisabled;

/// The text of an item: its label without mnemonic markers, `* ` before a
/// ticked or chosen one.
fn name(item: &Item) -> String {
    let label = mnemonic(&item.label).text;
    match item.check {
        Check::None => label,
        Check::Tick | Check::Dot => format!("* {label}"),
    }
}

fn convert(items: &[Item]) -> Vec<MenuItem> {
    items
        .iter()
        .map(|item| match &item.kind {
            Kind::Separator => MenuItem::separator(),
            Kind::Submenu(sub) => MenuItem::submenu(NativeMenu {
                name: name(item).into(),
                items: convert(sub),
            }),
            Kind::Command(id) if item.enabled && *id != 0 => {
                MenuItem::action(name(item), MenuCommand(*id))
            }
            Kind::Command(_) => MenuItem::action(name(item), MenuDisabled),
        })
        .collect()
}

/// The application menu (About, Quit) and SA's menus.
pub(crate) fn build(menus: &[Menu]) -> Vec<NativeMenu> {
    let mut out = vec![app_menu()];
    out.extend(menus.iter().map(|m| NativeMenu {
        name: mnemonic(&m.label).text.into(),
        items: convert(&m.items),
    }));
    out
}

/// The application menu alone (the other looks keep SA's menu in the window).
pub(crate) fn app_menu() -> NativeMenu {
    NativeMenu {
        name: "SigNexora".into(),
        items: vec![
            MenuItem::action(mnemonic(&t!("menu-about")).text, MenuCommand(cmd::ABOUT)),
            MenuItem::separator(),
            MenuItem::action(mnemonic(&t!("menu-exit")).text, MenuCommand(cmd::EXIT)),
        ],
    }
}

/// A short description of `menus` that changes whenever an item's text,
/// command, state or enabling does, so the menu bar is only rebuilt then.
pub(crate) fn signature(menus: &[Menu]) -> String {
    fn walk(items: &[Item], out: &mut String) {
        for item in items {
            match &item.kind {
                Kind::Separator => out.push('|'),
                Kind::Command(id) => {
                    let _ = write!(
                        out,
                        "{}:{id}:{}:{:?};",
                        item.label, item.enabled, item.check
                    );
                }
                Kind::Submenu(sub) => {
                    let _ = write!(out, "{}:{}[", item.label, item.enabled);
                    walk(sub, out);
                    out.push(']');
                }
            }
        }
    }
    let mut out = String::new();
    for m in menus {
        out.push_str(&m.label);
        out.push('{');
        walk(&m.items, &mut out);
        out.push('}');
    }
    out
}

/// Put `menus` in the macOS menu bar.
pub(crate) fn install(cx: &App, menus: &[Menu]) {
    cx.set_menus(build(menus));
}

/// Leave only the application menu in the macOS menu bar.
pub(crate) fn uninstall(cx: &App) {
    cx.set_menus(vec![app_menu()]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Menu> {
        vec![Menu {
            label: "&File".into(),
            items: vec![
                Item::command("&Open file...", cmd::OPEN_FILE),
                Item::separator(),
                Item::command("&Save", cmd::SAVE_SESSION).enabled(false),
                Item::submenu(
                    "&Info",
                    vec![
                        Item::command("On", cmd::INFO_ON).check(Check::Tick),
                        Item::command("Off", cmd::INFO_OFF),
                    ],
                ),
            ],
        }]
    }

    #[test]
    fn items_lose_their_mnemonics_and_ticks_become_stars() {
        let built = build(&sample());
        assert_eq!(built.len(), 2, "the application menu, then File");
        let file = built.get(1).map(|m| m.name.to_string());
        assert_eq!(file.as_deref(), Some("File"));
        let Some(MenuItem::Submenu(info)) = built.get(1).and_then(|m| m.items.get(3)) else {
            panic!("Info is a submenu");
        };
        let on = info.items.first();
        assert!(matches!(on, Some(MenuItem::Action { name, .. }) if name.as_ref() == "* On"));
    }

    #[test]
    fn disabled_items_run_the_unhandled_action() {
        let built = build(&sample());
        let save = built.get(1).and_then(|m| m.items.get(2));
        let Some(MenuItem::Action { action, .. }) = save else {
            panic!("Save is an action");
        };
        assert!(action.as_any().downcast_ref::<MenuDisabled>().is_some());
    }

    #[test]
    fn the_signature_follows_state_changes() {
        let a = sample();
        let mut b = sample();
        if let Some(m) = b.first_mut()
            && let Some(item) = m.items.get_mut(2)
        {
            item.enabled = true;
        }
        assert_eq!(signature(&a), signature(&sample()));
        assert_ne!(signature(&a), signature(&b));
    }
}
