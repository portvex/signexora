//! Menus: the model (items, submenus, check marks) and the Windows 95 menu
//! tracking logic (which menu is open, which item is highlighted, and what the
//! keyboard does), independent of drawing so it can be tested.
//!
//! Keyboard rules (Windows 95):
//! - Alt (pressed and released alone) or F10 selects the first menu title
//!   without opening it; Alt again or Escape leaves the menu bar.
//! - Alt+letter opens the menu whose title has that mnemonic.
//! - On the bar: Left/Right move between titles, Down or Enter opens.
//! - In a menu: Up/Down move (skipping separators, wrapping), Right opens a
//!   submenu or moves to the next title, Left closes a submenu or moves to the
//!   previous title, Enter chooses, a mnemonic letter chooses its item, Escape
//!   closes one level.

use super::glyph::Glyph;
use super::text::mnemonic;

/// Check mark of a menu item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Check {
    #[default]
    None,
    /// A tick (check box style).
    Tick,
    /// A dot (one of a group).
    Dot,
}

/// What a menu item does.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind {
    /// Runs the command with this id.
    Command(u32),
    /// Opens a submenu.
    Submenu(Vec<Item>),
    /// A separator line.
    Separator,
}

/// One menu item.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Item {
    /// The label with an `&` mnemonic.
    pub(crate) label: String,
    /// Shortcut text shown at the right (`Ctrl+F4`).
    pub(crate) shortcut: String,
    pub(crate) kind: Kind,
    pub(crate) enabled: bool,
    pub(crate) check: Check,
    /// A 16x16 picture left of the label (Delphi menu images).
    pub(crate) glyph: Option<Glyph>,
    /// The prompt the status bar shows while the item is highlighted (the
    /// Windows 95 menu help line).
    pub(crate) hint: String,
}

impl Item {
    /// A command item.
    pub(crate) fn command(label: &str, id: u32) -> Self {
        Self {
            label: label.to_owned(),
            shortcut: String::new(),
            kind: Kind::Command(id),
            enabled: true,
            check: Check::None,
            glyph: None,
            hint: String::new(),
        }
    }

    /// A submenu item.
    pub(crate) fn submenu(label: &str, items: Vec<Item>) -> Self {
        Self {
            kind: Kind::Submenu(items),
            ..Self::command(label, 0)
        }
    }

    /// A separator.
    pub(crate) fn separator() -> Self {
        Self {
            kind: Kind::Separator,
            ..Self::command("", 0)
        }
    }

    #[must_use]
    pub(crate) fn shortcut(mut self, s: &str) -> Self {
        s.clone_into(&mut self.shortcut);
        self
    }

    #[must_use]
    pub(crate) fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }

    #[must_use]
    pub(crate) fn check(mut self, c: Check) -> Self {
        self.check = c;
        self
    }

    #[must_use]
    pub(crate) fn hint(mut self, s: &str) -> Self {
        s.clone_into(&mut self.hint);
        self
    }

    #[must_use]
    pub(crate) fn glyph(mut self, g: Glyph) -> Self {
        self.glyph = Some(g);
        self
    }

    pub(crate) fn is_separator(&self) -> bool {
        matches!(self.kind, Kind::Separator)
    }

    fn children(&self) -> Option<&[Item]> {
        match &self.kind {
            Kind::Submenu(items) => Some(items),
            _ => None,
        }
    }
}

/// A menu of the menu bar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Menu {
    /// The title with an `&` mnemonic.
    pub(crate) label: String,
    pub(crate) items: Vec<Item>,
}

/// A navigation key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    /// Alt (or F10) pressed and released alone.
    Alt,
    /// A letter: a mnemonic.
    Char(char),
    /// Alt+letter.
    AltChar(char),
}

/// What a key or click did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The key was not for the menus.
    Ignored,
    /// The menus changed (or the key was swallowed).
    Handled,
    /// A command was chosen; the menus are closed.
    Invoke(u32),
}

/// Menu tracking state: the selected bar title and the open menus.
///
/// `levels[0]` is the open drop-down of the selected title (or the root of a
/// popup menu); each further level is an open submenu. Each level holds its
/// highlighted item.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tracker {
    /// The selected title of the menu bar (highlighted or open).
    pub(crate) bar: Option<usize>,
    /// The highlighted item of each open level.
    pub(crate) levels: Vec<Option<usize>>,
    /// The menus were entered from the keyboard (Alt, F10, Alt+letter):
    /// GNOME and KDE then underline the mnemonics until they close.
    pub(crate) keyboard: bool,
}

/// Where the root items come from.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Source<'a> {
    Bar(&'a [Menu]),
    Popup(&'a [Item]),
}

impl Tracker {
    /// Whether any menu is shown (or the bar is keyboard-selected).
    pub(crate) fn is_active(&self) -> bool {
        self.bar.is_some() || !self.levels.is_empty()
    }

    /// Whether a drop-down or popup is open.
    pub(crate) fn is_open(&self) -> bool {
        !self.levels.is_empty()
    }

    /// Close everything.
    pub(crate) fn close(&mut self) {
        self.bar = None;
        self.levels.clear();
        self.keyboard = false;
    }

    /// Open bar menu `i` with nothing highlighted (mouse).
    pub(crate) fn open_bar(&mut self, i: usize) {
        self.bar = Some(i);
        self.levels = vec![None];
        self.keyboard = false;
    }

    /// Open a popup menu with nothing highlighted.
    pub(crate) fn open_popup(&mut self) {
        self.bar = None;
        self.levels = vec![None];
        self.keyboard = false;
    }

    /// The items shown at `depth` (0 = the drop-down or popup root).
    pub(crate) fn items_at<'a>(&self, src: Source<'a>, depth: usize) -> Option<&'a [Item]> {
        let mut items: &[Item] = match src {
            Source::Bar(menus) => &menus.get(self.bar?)?.items,
            Source::Popup(items) => items,
        };
        for d in 0..depth {
            let hot = (*self.levels.get(d)?)?;
            items = items.get(hot)?.children()?;
        }
        Some(items)
    }

    /// The highlighted item of the deepest open level.
    pub(crate) fn hot_item<'a>(&self, src: Source<'a>) -> Option<&'a Item> {
        let depth = self.levels.len().checked_sub(1)?;
        let hot = (*self.levels.get(depth)?)?;
        self.items_at(src, depth)?.get(hot)
    }

    /// Highlight item `i` of level `depth` with the mouse, closing deeper levels
    /// and opening `i`'s submenu if it has one.
    pub(crate) fn hover(&mut self, src: Source<'_>, depth: usize, i: Option<usize>) {
        if depth >= self.levels.len() {
            return;
        }
        self.levels.truncate(depth + 1);
        if let Some(slot) = self.levels.get_mut(depth) {
            *slot = i;
        }
        let opens = i
            .and_then(|i| self.items_at(src, depth)?.get(i))
            .is_some_and(|it| it.enabled && it.children().is_some());
        if opens {
            self.levels.push(None);
        }
    }

    /// Choose item `i` of level `depth` (mouse click or Enter).
    pub(crate) fn choose(&mut self, src: Source<'_>, depth: usize, i: usize) -> Outcome {
        let Some(item) = self.items_at(src, depth).and_then(|it| it.get(i)) else {
            return Outcome::Handled;
        };
        if !item.enabled || item.is_separator() {
            return Outcome::Handled;
        }
        match &item.kind {
            Kind::Command(id) => {
                let id = *id;
                self.close();
                Outcome::Invoke(id)
            }
            Kind::Submenu(children) => {
                self.levels.truncate(depth + 1);
                if let Some(slot) = self.levels.get_mut(depth) {
                    *slot = Some(i);
                }
                self.levels.push(first_selectable(children));
                Outcome::Handled
            }
            Kind::Separator => Outcome::Handled,
        }
    }

    /// Handle a navigation key.
    pub(crate) fn key(&mut self, src: Source<'_>, key: Key) -> Outcome {
        let was_active = self.is_active();
        let out = self.key_inner(src, key);
        // Entering the menus from the keyboard (or Alt+letter to another
        // title) marks them keyboard menus; a mouse click clears it.
        if self.is_active() && (!was_active || matches!(key, Key::AltChar(_))) {
            self.keyboard = true;
        }
        out
    }

    fn key_inner(&mut self, src: Source<'_>, key: Key) -> Outcome {
        let bar_len = match src {
            Source::Bar(m) => m.len(),
            Source::Popup(_) => 0,
        };
        if let (Key::AltChar(c), Source::Bar(menus)) = (key, src) {
            return match find_mnemonic(menus.iter().map(|m| m.label.as_str()), c) {
                Some(i) => {
                    self.open_bar_keyboard(src, i);
                    Outcome::Handled
                }
                None => Outcome::Ignored,
            };
        }
        if !self.is_active() {
            if key == Key::Alt && bar_len > 0 {
                self.bar = Some(0);
                return Outcome::Handled;
            }
            return Outcome::Ignored;
        }
        if self.levels.is_empty() {
            return self.bar_key(src, key, bar_len);
        }
        self.menu_key(src, key, bar_len)
    }

    /// A key while a bar title is selected but no menu is open.
    fn bar_key(&mut self, src: Source<'_>, key: Key, n: usize) -> Outcome {
        let Some(cur) = self.bar else {
            return Outcome::Ignored;
        };
        match key {
            Key::Left => self.bar = Some((cur + n.max(1) - 1) % n.max(1)),
            Key::Right => self.bar = Some((cur + 1) % n.max(1)),
            Key::Down | Key::Up | Key::Enter => self.open_bar_keyboard(src, cur),
            Key::Escape | Key::Alt => self.close(),
            Key::Char(c) | Key::AltChar(c) => {
                if let Source::Bar(menus) = src
                    && let Some(i) = find_mnemonic(menus.iter().map(|m| m.label.as_str()), c)
                {
                    self.open_bar_keyboard(src, i);
                }
            }
        }
        Outcome::Handled
    }

    /// Open bar menu `i` with its first item highlighted (keyboard).
    fn open_bar_keyboard(&mut self, src: Source<'_>, i: usize) {
        self.bar = Some(i);
        self.levels = vec![None];
        let first = self.items_at(src, 0).and_then(first_selectable);
        self.levels = vec![first];
    }

    /// A key while a menu is open.
    fn menu_key(&mut self, src: Source<'_>, key: Key, bar_len: usize) -> Outcome {
        let depth = self.levels.len() - 1;
        let Some(items) = self.items_at(src, depth) else {
            self.close();
            return Outcome::Handled;
        };
        let hot = self.levels.last().copied().flatten();
        match key {
            Key::Up | Key::Down => {
                let next = step(items, hot, key == Key::Up);
                if let Some(slot) = self.levels.last_mut() {
                    *slot = next;
                }
            }
            Key::Right => {
                let sub = hot.filter(|&h| {
                    items
                        .get(h)
                        .is_some_and(|it| it.enabled && it.children().is_some())
                });
                match sub {
                    Some(h) => return self.choose(src, depth, h),
                    None if bar_len > 0 => {
                        let cur = self.bar.unwrap_or(0);
                        self.open_bar_keyboard(src, (cur + 1) % bar_len);
                    }
                    None => {}
                }
            }
            Key::Left => {
                if depth > 0 {
                    self.levels.pop();
                } else if bar_len > 0 {
                    let cur = self.bar.unwrap_or(0);
                    self.open_bar_keyboard(src, (cur + bar_len - 1) % bar_len);
                }
            }
            Key::Enter => {
                if let Some(h) = hot {
                    return self.choose(src, depth, h);
                }
            }
            Key::Escape => {
                self.levels.pop();
                if self.levels.is_empty() && bar_len == 0 {
                    self.close();
                }
            }
            Key::Alt => self.close(),
            Key::Char(c) | Key::AltChar(c) => {
                if let Some(i) = find_mnemonic(items.iter().map(|it| it.label.as_str()), c) {
                    if let Some(slot) = self.levels.last_mut() {
                        *slot = Some(i);
                    }
                    return self.choose(src, depth, i);
                }
            }
        }
        Outcome::Handled
    }
}

/// The first item that can be highlighted (not a separator).
fn first_selectable(items: &[Item]) -> Option<usize> {
    items.iter().position(|it| !it.is_separator())
}

/// The next highlightable item after `from` (or before it with `up`), wrapping.
/// Windows 95 highlights disabled items too; only separators are skipped.
fn step(items: &[Item], from: Option<usize>, up: bool) -> Option<usize> {
    let n = items.len();
    if n == 0 {
        return None;
    }
    let mut i = match (from, up) {
        (Some(f), _) => f,
        (None, false) => n - 1,
        (None, true) => 0,
    };
    for _ in 0..n {
        i = if up { (i + n - 1) % n } else { (i + 1) % n };
        if items.get(i).is_some_and(|it| !it.is_separator()) {
            return Some(i);
        }
    }
    None
}

/// The index of the label whose mnemonic is `c`.
pub(crate) fn find_mnemonic<'a>(labels: impl Iterator<Item = &'a str>, c: char) -> Option<usize> {
    let c = c.to_lowercase().next()?;
    labels.into_iter().position(|l| mnemonic(l).key == Some(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menus() -> Vec<Menu> {
        vec![
            Menu {
                label: "&File".into(),
                items: vec![
                    Item::command("&Open file...", 1),
                    Item::command("&Demo...", 2).enabled(false),
                    Item::separator(),
                    Item::command("E&xit", 3),
                ],
            },
            Menu {
                label: "&Windows".into(),
                items: vec![Item::command("&CloseAllWindows", 10)],
            },
            Menu {
                label: "&Options".into(),
                items: vec![
                    Item::submenu(
                        "&Info by default",
                        vec![Item::command("O&n", 20), Item::command("O&ff", 21)],
                    ),
                    Item::command("&Theme", 22),
                ],
            },
        ]
    }

    #[test]
    fn keyboard_entry_is_noted_until_the_menus_close() {
        let m = menus();
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        t.key(src, Key::Alt);
        assert!(t.keyboard, "Alt (or F10) tapped");
        t.key(src, Key::Down);
        assert!(t.keyboard, "kept while navigating");
        t.key(src, Key::Escape);
        t.key(src, Key::Escape);
        assert!(!t.keyboard, "cleared when the menus close");
        t.key(src, Key::AltChar('f'));
        assert!(t.keyboard, "Alt+letter");
        t.open_bar(1);
        assert!(!t.keyboard, "a click opens a mouse menu");
        t.open_popup();
        assert!(!t.keyboard);
        t.close();
        assert_eq!(t.key(src, Key::Down), Outcome::Ignored);
        assert!(!t.keyboard, "an ignored key enters nothing");
    }

    #[test]
    fn alt_selects_bar_and_arrows_move() {
        let m = menus();
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        assert_eq!(t.key(src, Key::Down), Outcome::Ignored);
        assert_eq!(t.key(src, Key::Alt), Outcome::Handled);
        assert_eq!(t.bar, Some(0));
        assert!(!t.is_open());
        t.key(src, Key::Left);
        assert_eq!(t.bar, Some(2), "Left wraps to the last title");
        t.key(src, Key::Right);
        assert_eq!(t.bar, Some(0));
        t.key(src, Key::Down);
        assert_eq!(t.levels, vec![Some(0)], "Down opens with the first item");
        t.key(src, Key::Escape);
        assert!(!t.is_open());
        assert_eq!(t.bar, Some(0), "Escape closes the menu but keeps the bar");
        t.key(src, Key::Escape);
        assert!(!t.is_active());
    }

    #[test]
    fn up_down_skip_separators_and_wrap() {
        let m = menus();
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        t.key(src, Key::AltChar('f'));
        assert_eq!(t.levels, vec![Some(0)]);
        t.key(src, Key::Down);
        assert_eq!(t.levels, vec![Some(1)], "disabled items are highlighted");
        t.key(src, Key::Down);
        assert_eq!(t.levels, vec![Some(3)], "separator skipped");
        t.key(src, Key::Down);
        assert_eq!(t.levels, vec![Some(0)], "wraps");
        t.key(src, Key::Up);
        assert_eq!(t.levels, vec![Some(3)]);
        assert_eq!(t.key(src, Key::Enter), Outcome::Invoke(3));
        assert!(!t.is_active());
    }

    #[test]
    fn disabled_items_do_not_run() {
        let m = menus();
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        t.open_bar(0);
        assert_eq!(t.choose(src, 0, 1), Outcome::Handled);
        assert!(t.is_open());
        assert_eq!(t.key(src, Key::Char('d')), Outcome::Handled);
        assert_eq!(t.choose(src, 0, 2), Outcome::Handled, "separator");
    }

    #[test]
    fn submenus_open_with_right_and_close_with_left() {
        let m = menus();
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        t.key(src, Key::AltChar('o'));
        assert_eq!(t.bar, Some(2));
        t.key(src, Key::Right);
        assert_eq!(t.levels, vec![Some(0), Some(0)]);
        assert_eq!(t.items_at(src, 1).map(<[Item]>::len), Some(2));
        t.key(src, Key::Down);
        assert_eq!(t.levels, vec![Some(0), Some(1)]);
        t.key(src, Key::Left);
        assert_eq!(t.levels, vec![Some(0)]);
        t.key(src, Key::Right);
        assert_eq!(t.key(src, Key::Char('f')), Outcome::Invoke(21));
    }

    #[test]
    fn right_and_left_move_between_menus() {
        let m = menus();
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        t.key(src, Key::AltChar('f'));
        t.key(src, Key::Right);
        assert_eq!((t.bar, t.levels.clone()), (Some(1), vec![Some(0)]));
        t.key(src, Key::Left);
        assert_eq!(t.bar, Some(0));
        t.key(src, Key::Left);
        assert_eq!(t.bar, Some(2), "wraps to the last menu");
    }

    #[test]
    fn mnemonics_choose_items() {
        let m = menus();
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        t.key(src, Key::AltChar('w'));
        assert_eq!(t.key(src, Key::Char('C')), Outcome::Invoke(10));
        assert_eq!(t.key(src, Key::AltChar('z')), Outcome::Ignored);
    }

    #[test]
    fn mouse_hover_opens_submenus() {
        let m = menus();
        let src = Source::Bar(&m);
        let mut t = Tracker::default();
        t.open_bar(2);
        t.hover(src, 0, Some(0));
        assert_eq!(t.levels, vec![Some(0), None]);
        t.hover(src, 0, Some(1));
        assert_eq!(
            t.levels,
            vec![Some(1)],
            "leaving the submenu item closes it"
        );
        t.hover(src, 3, Some(0));
        assert_eq!(
            t.levels,
            vec![Some(1)],
            "levels that are not open are ignored"
        );
    }

    #[test]
    fn popup_menus_close_on_escape() {
        let items = vec![Item::command("&Copy", 5), Item::command("&Paste", 6)];
        let src = Source::Popup(&items);
        let mut t = Tracker::default();
        t.open_popup();
        t.key(src, Key::Down);
        assert_eq!(t.levels, vec![Some(0)]);
        t.key(src, Key::Right);
        assert_eq!(t.levels, vec![Some(0)], "no bar to move to");
        t.key(src, Key::Escape);
        assert!(!t.is_active());
        t.open_popup();
        assert_eq!(t.key(src, Key::Char('p')), Outcome::Invoke(6));
    }
}
