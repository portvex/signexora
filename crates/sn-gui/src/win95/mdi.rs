//! The MDI window manager: child windows inside the frame's client area, their
//! z-order and activation, minimize/maximize/restore, and the geometry of
//! moving and sizing. Pure logic, drawn by [`super::mdi_view`].
//!
//! Windows 95 MDI rules kept here:
//! - the active child is on top; clicking a child activates it;
//! - closing the active child activates the topmost remaining one;
//! - while the active child is maximized, every child that becomes active is
//!   maximized too (the frame shows one maximized child at a time);
//! - Ctrl+F6 sends the active child to the bottom and activates the next.

use super::geom::Rect;

/// Width of the sizing frame (`SM_CXFRAME`).
pub(crate) fn frame_w() -> i32 {
    super::style::metrics().frame
}
/// Height of the caption (`SM_CYCAPTION`).
pub(crate) fn caption_h() -> i32 {
    super::style::metrics().caption
}
/// Caption button size (`SM_CXSIZE` - 2, `SM_CYSIZE` - 4).
pub(crate) fn btn_w() -> i32 {
    super::style::metrics().btn.0
}
/// Caption button height.
pub(crate) fn btn_h() -> i32 {
    super::style::metrics().btn.1
}
/// Width of a minimized child (`SM_CXMINIMIZED`): Windows 95's 160 pixels,
/// wider where the caption buttons are, so about as much of the title shows
/// in every style.
pub(crate) fn min_width() -> i32 {
    min_width_of(super::style::metrics())
}

/// Width of a minimized child with metrics `m`.
pub(crate) fn min_width_of(m: super::style::Metrics) -> i32 {
    /// The title width of a minimized Windows 95 child.
    const TITLE: i32 = 79;
    if m.btn_left {
        // macOS: the title is centred between the traffic lights and an
        // equal space on the right.
        return 160.max(2 * m.frame + 2 * title_inset_of(m) + TITLE);
    }
    let buttons = 2 * m.btn.0 + m.close_w + m.btn_gaps.0 + m.btn_gaps.1 + m.btn_margin;
    160.max(2 * m.frame + 21 + buttons + TITLE)
}

/// With the caption buttons at the left (macOS): how far the title keeps
/// from each end of the caption, clear of the traffic lights.
pub(crate) fn title_inset_of(m: super::style::Metrics) -> i32 {
    m.btn_margin + m.close_w + m.btn_gaps.1 + m.btn.0 + m.btn_gaps.0 + m.btn.0 + 6
}

/// Smallest size a child can be dragged to.
pub(crate) fn min_track() -> (i32, i32) {
    min_track_of(super::style::metrics())
}

/// Smallest size of a child with metrics `m`.
pub(crate) fn min_track_of(m: super::style::Metrics) -> (i32, i32) {
    (112, 2 * m.frame + m.caption)
}
/// Offset of a new window from the one it was opened from (SA: 24 px).
pub(crate) const CASCADE: i32 = 24;
/// Size of the corner zones of the sizing frame.
pub(crate) const CORNER: i32 = 16;

/// Window state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum State {
    #[default]
    Normal,
    Minimized,
    Maximized,
}

/// One MDI child window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Child {
    pub(crate) id: u32,
    pub(crate) title: String,
    /// Position and size in the client area (normal state).
    pub(crate) rect: Rect,
    pub(crate) state: State,
    /// Can be resized (and maximized).
    pub(crate) resizable: bool,
    /// Slot of a minimized child along the bottom of the client area.
    icon_slot: usize,
}

/// An edge or corner of the sizing frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

impl Side {
    fn moves(self) -> (bool, bool, bool, bool) {
        // (left, top, right, bottom)
        match self {
            Self::N => (false, true, false, false),
            Self::S => (false, false, false, true),
            Self::E => (false, false, true, false),
            Self::W => (true, false, false, false),
            Self::NE => (false, true, true, false),
            Self::NW => (true, true, false, false),
            Self::SE => (false, false, true, true),
            Self::SW => (true, false, false, true),
        }
    }
}

/// What is under the mouse in a child window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    Close,
    Maximize,
    /// Minimize, or Restore on a minimized child.
    Minimize,
    /// The window icon (system menu).
    Icon,
    Caption,
    Border(Side),
    Client,
}

/// The caption buttons of a child `w` pixels wide: (minimize, maximize, close).
pub(crate) fn caption_buttons(w: i32) -> (Rect, Rect, Rect) {
    caption_buttons_in(w, frame_w())
}

/// Minimize, maximize and close of a window `w` wide with a frame `frame`
/// wide, as the active style places them.
pub(crate) fn caption_buttons_in(w: i32, frame: i32) -> (Rect, Rect, Rect) {
    let m = super::style::metrics();
    let (bw, bh) = m.btn;
    let y = m.btn_top.unwrap_or(frame + (m.caption - bh) / 2);
    if m.btn_left {
        // macOS: close, minimize, zoom from the left.
        let close = Rect::new(frame + m.btn_margin, y, m.close_w, bh);
        let min = Rect::new(close.right() + m.btn_gaps.1, y, bw, bh);
        let max = Rect::new(min.right() + m.btn_gaps.0, y, bw, bh);
        return (min, max, close);
    }
    let close = Rect::new(w - frame - m.btn_margin - m.close_w, y, m.close_w, bh);
    let max = Rect::new(close.x - m.btn_gaps.0 - bw, y, bw, bh);
    let min = Rect::new(max.x - m.btn_gaps.1 - bw, y, bw, bh);
    (min, max, close)
}

/// The span of the traffic lights `(min, max, close)` laid out from the
/// left (macOS): from close to zoom, or close alone when the window has only
/// that. Under the mouse, every light shows its glyph.
pub(crate) fn light_cluster((min, max, close): (Rect, Rect, Rect), resizable: bool) -> Rect {
    let right = if resizable {
        close.right().max(min.right()).max(max.right())
    } else {
        close.right()
    };
    let top = close
        .y
        .min(if resizable { min.y.min(max.y) } else { close.y });
    let bottom = close.bottom().max(if resizable {
        min.bottom().max(max.bottom())
    } else {
        close.bottom()
    });
    Rect::new(close.x, top, right - close.x, bottom - top)
}

/// The caption bar of a child `w` pixels wide.
pub(crate) fn caption_rect(w: i32) -> Rect {
    Rect::new(frame_w(), frame_w(), w - 2 * frame_w(), caption_h())
}

/// The client area of a child of size `w` x `h`, in its own coordinates.
pub(crate) fn client_rect(w: i32, h: i32) -> Rect {
    Rect::new(
        frame_w(),
        frame_w() + caption_h(),
        w - 2 * frame_w(),
        h - 2 * frame_w() - caption_h(),
    )
}

/// The width of the window icon at the left of a caption `cap_h` high,
/// which opens the window menu: none in the macOS looks and Adwaita,
/// Motif's square window-menu button, 18 pixels elsewhere.
pub(crate) fn icon_zone_w(cap_h: i32) -> i32 {
    if super::style::metrics().btn_left {
        return 0;
    }
    super::skin::linux_look().map_or(18, |l| super::skin_linux::icon_width(l, cap_h))
}

/// What is at `(x, y)` (child coordinates) in a child of size `w` x `h`.
pub(crate) fn hit(w: i32, h: i32, state: State, resizable: bool, x: i32, y: i32) -> Option<Hit> {
    if !Rect::new(0, 0, w, h).contains(x, y) {
        return None;
    }
    let (min, max, close) = caption_buttons(w);
    for (r, hit) in [
        (close, Hit::Close),
        (max, Hit::Maximize),
        (min, Hit::Minimize),
    ] {
        if r.contains(x, y) && (resizable || hit == Hit::Close) {
            return Some(hit);
        }
    }
    let cap = caption_rect(w);
    if cap.contains(x, y) {
        // macOS and Adwaita have no window icon: the whole title bar is
        // caption there.
        let icon = x < cap.x + icon_zone_w(cap.h);
        return Some(if icon { Hit::Icon } else { Hit::Caption });
    }
    if resizable
        && state == State::Normal
        && !Rect::new(frame_w(), frame_w(), w - 2 * frame_w(), h - 2 * frame_w()).contains(x, y)
    {
        let (l, t) = (x < CORNER, y < CORNER);
        let (r, b) = (x >= w - CORNER, y >= h - CORNER);
        let side = match (t, b, l, r) {
            (true, _, true, _) => Side::NW,
            (true, _, _, true) => Side::NE,
            (_, true, true, _) => Side::SW,
            (_, true, _, true) => Side::SE,
            _ if y < frame_w() => Side::N,
            _ if y >= h - frame_w() => Side::S,
            _ if x < frame_w() => Side::W,
            _ => Side::E,
        };
        return Some(Hit::Border(side));
    }
    Some(Hit::Client)
}

/// The outer rectangle `r` of a child framed with metrics `old`, framed with
/// `new` instead: the client area is kept (never smaller than `new`'s
/// smallest window), the top left corner stays.
pub(crate) fn restyled(r: Rect, old: super::style::Metrics, new: super::style::Metrics) -> Rect {
    let dw = 2 * (new.frame - old.frame);
    let dh = dw + new.caption - old.caption;
    let (min_w, min_h) = min_track_of(new);
    Rect {
        w: (r.w + dw).max(min_w.min(r.w + dw.max(0))),
        h: (r.h + dh).max(min_h),
        ..r
    }
}

/// Clamp `r` inside a client area of size `cw` x `ch` (keeping its size when
/// it fits; a window larger than the client area is pinned at the top left).
pub(crate) fn clamp_into(r: Rect, cw: i32, ch: i32) -> Rect {
    Rect {
        x: r.x.min(cw.saturating_sub(r.w)).max(0),
        y: r.y.min(ch.saturating_sub(r.h)).max(0),
        ..r
    }
}

/// The rectangle while dragging side `side` of `start` by `(dx, dy)`, limited
/// to the minimum size and to the client area.
pub(crate) fn sized(start: Rect, side: Side, dx: i32, dy: i32, cw: i32, ch: i32) -> Rect {
    let (ml, mt, mr, mb) = side.moves();
    let (mut l, mut t, mut r, mut b) = (start.x, start.y, start.right(), start.bottom());
    // The bounds of every clamp are ordered (`clamp` panics otherwise): a
    // child already narrower than the minimum near the left edge keeps x 0.
    if ml {
        l = (l + dx).clamp(0, (r - min_track().0).max(0));
    }
    if mr {
        r = (r + dx).clamp(l + min_track().0, cw.max(l + min_track().0));
    }
    if mt {
        t = (t + dy).clamp(0, (b - min_track().1).max(0));
    }
    if mb {
        b = (b + dy).clamp(t + min_track().1, ch.max(t + min_track().1));
    }
    Rect::new(l, t, r - l, b - t)
}

/// The MDI client area and its children.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Mdi {
    /// Children from bottom to top.
    children: Vec<Child>,
    active: Option<u32>,
    next_id: u32,
    /// Size of the client area.
    pub(crate) client: (i32, i32),
}

impl Mdi {
    pub(crate) fn new(cw: i32, ch: i32) -> Self {
        Self {
            client: (cw, ch),
            next_id: 1,
            ..Self::default()
        }
    }

    /// Children from bottom to top.
    pub(crate) fn children(&self) -> &[Child] {
        &self.children
    }

    pub(crate) fn active(&self) -> Option<u32> {
        self.active
    }

    pub(crate) fn get(&self, id: u32) -> Option<&Child> {
        self.children.iter().find(|c| c.id == id)
    }

    /// Give child `id` a new title.
    pub(crate) fn set_title(&mut self, id: u32, title: &str) {
        if let Some(c) = self.get_mut(id) {
            title.clone_into(&mut c.title);
        }
    }

    fn get_mut(&mut self, id: u32) -> Option<&mut Child> {
        self.children.iter_mut().find(|c| c.id == id)
    }

    /// Whether the active child is maximized.
    pub(crate) fn maximized(&self) -> Option<&Child> {
        self.active
            .and_then(|id| self.get(id))
            .filter(|c| c.state == State::Maximized)
    }

    /// Open a child of size `w` x `h`, 24 px lower right from `near` (or
    /// cascaded from the top left), and activate it.
    pub(crate) fn add(
        &mut self,
        title: &str,
        w: i32,
        h: i32,
        resizable: bool,
        near: Option<u32>,
    ) -> u32 {
        let (cw, ch) = self.client;
        let base = near.and_then(|n| self.get(n)).map_or_else(
            || {
                let k = (self.children.len() % 8) as i32 * CASCADE;
                (k, k)
            },
            |c| (c.rect.x + CASCADE, c.rect.y + CASCADE),
        );
        let mut rect = Rect::new(base.0, base.1, w, h);
        if rect.right() > cw || rect.bottom() > ch {
            rect.x = 0;
            rect.y = 0;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.children.push(Child {
            id,
            title: title.to_owned(),
            rect,
            state: State::Normal,
            resizable,
            icon_slot: 0,
        });
        self.activate(id);
        id
    }

    /// Bring `id` to the top and make it active.
    pub(crate) fn activate(&mut self, id: u32) {
        let Some(pos) = self.children.iter().position(|c| c.id == id) else {
            return;
        };
        let was_max = self.maximized().map(|c| c.id).filter(|&m| m != id);
        let child = self.children.remove(pos);
        self.children.push(child);
        self.active = Some(id);
        if let Some(m) = was_max {
            self.set_state(m, State::Normal);
            if self.get(id).is_some_and(|c| c.resizable) {
                self.set_state(id, State::Maximized);
            }
        }
    }

    /// Close `id`; the topmost remaining child becomes active.
    pub(crate) fn close(&mut self, id: u32) {
        let was_max = self.maximized().is_some_and(|c| c.id == id);
        self.children.retain(|c| c.id != id);
        if self.active == Some(id) {
            self.active = None;
            if let Some(top) = self.children.last().map(|c| c.id) {
                self.active = Some(top);
                if was_max
                    && self
                        .get(top)
                        .is_some_and(|c| c.resizable && c.state != State::Minimized)
                {
                    self.set_state(top, State::Maximized);
                }
            }
        }
        self.layout_icons();
    }

    /// Close every child (`CloseAllWindows`).
    pub(crate) fn close_all(&mut self) {
        self.children.clear();
        self.active = None;
    }

    /// Ctrl+F6 (or Ctrl+Shift+F6 with `back`): activate the next child.
    pub(crate) fn next(&mut self, back: bool) {
        if self.children.len() < 2 {
            return;
        }
        let target = if back {
            self.children.first().map(|c| c.id)
        } else {
            let top = self.children.pop();
            if let Some(top) = top {
                self.children.insert(0, top);
            }
            self.children.last().map(|c| c.id)
        };
        if let Some(t) = target {
            self.activate(t);
        }
    }

    /// Change the state of `id`.
    pub(crate) fn set_state(&mut self, id: u32, state: State) {
        if let Some(c) = self.get_mut(id) {
            if state == State::Maximized && !c.resizable {
                return;
            }
            c.state = state;
        }
        self.layout_icons();
    }

    /// Minimize, maximize or restore `id`, activating as Windows does.
    pub(crate) fn command(&mut self, id: u32, hit: Hit) {
        let Some(state) = self.get(id).map(|c| c.state) else {
            return;
        };
        match (hit, state) {
            (Hit::Maximize, State::Maximized) | (Hit::Minimize, State::Minimized) => {
                self.set_state(id, State::Normal);
                self.activate(id);
            }
            (Hit::Maximize, _) => {
                self.activate(id);
                self.set_state(id, State::Maximized);
            }
            (Hit::Minimize, _) => {
                self.set_state(id, State::Minimized);
                // Windows activates the next window that is not minimized.
                let next = self
                    .children
                    .iter()
                    .rev()
                    .find(|c| c.id != id && c.state != State::Minimized)
                    .map(|c| c.id);
                match next {
                    Some(n) => self.activate(n),
                    None => self.active = Some(id),
                }
            }
            (Hit::Close, _) => self.close(id),
            _ => {}
        }
    }

    /// Set the normal rectangle of `id` (after sizing).
    pub(crate) fn set_rect(&mut self, id: u32, r: Rect) {
        let (cw, ch) = self.client;
        if let Some(c) = self.get_mut(id) {
            c.rect = clamp_into(r, cw, ch);
        }
    }

    /// After a change of Windows style from frames `old` to `new`: every child
    /// keeps its client area (a fixed form keeps all its controls), its outer
    /// rectangle growing or shrinking with the frame and caption.
    /// A child pushed past the right or bottom edge by growing is moved back
    /// inside the client area.
    pub(crate) fn restyle(&mut self, old: super::style::Metrics, new: super::style::Metrics) {
        let (cw, ch) = self.client;
        for c in &mut self.children {
            c.rect = restyled(c.rect, old, new);
            c.rect = clamp_into(c.rect, cw, ch);
        }
    }

    /// Resize the client area; children are kept inside it.
    pub(crate) fn set_client(&mut self, cw: i32, ch: i32) {
        if self.client == (cw, ch) {
            return;
        }
        self.client = (cw, ch);
        for c in &mut self.children {
            c.rect = clamp_into(c.rect, cw, ch);
        }
    }

    /// Give minimized children their slots along the bottom, in opening order.
    fn layout_icons(&mut self) {
        let mut ids: Vec<u32> = self
            .children
            .iter()
            .filter(|c| c.state == State::Minimized)
            .map(|c| c.id)
            .collect();
        ids.sort_unstable();
        for c in &mut self.children {
            if let Some(slot) = ids.iter().position(|&i| i == c.id) {
                c.icon_slot = slot;
            }
        }
    }

    /// Where `id` is drawn now: its normal rectangle, its minimized bar at the
    /// bottom, or (maximized) the client area with the frame and caption
    /// outside it.
    pub(crate) fn shown_rect(&self, id: u32) -> Option<Rect> {
        let c = self.get(id)?;
        let (cw, ch) = self.client;
        Some(match c.state {
            State::Normal => c.rect,
            State::Minimized => {
                let mw = min_width();
                let per_row = (cw / mw).max(1) as usize;
                let (col, row) = (c.icon_slot % per_row, c.icon_slot / per_row);
                let h = min_track().1;
                Rect::new(col as i32 * mw, ch - h * (row as i32 + 1), mw, h)
            }
            // With the menu in the macOS menu bar there is no bar to take
            // the caption buttons: a maximized child keeps its title bar.
            State::Maximized if super::style::native_menu() => Rect::new(0, 0, cw, ch),
            State::Maximized => Rect::new(
                -frame_w(),
                -frame_w() - caption_h(),
                cw + 2 * frame_w(),
                ch + 2 * frame_w() + caption_h(),
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(m: &Mdi) -> Vec<u32> {
        m.children().iter().map(|c| c.id).collect()
    }

    #[test]
    fn add_cascades_and_activates() {
        let mut m = Mdi::new(800, 600);
        let a = m.add("a", 300, 200, true, None);
        let b = m.add("b", 300, 200, true, Some(a));
        assert_eq!(m.active(), Some(b));
        assert_eq!(m.get(b).map(|c| c.rect), Some(Rect::new(24, 24, 300, 200)));
        let c = m.add("c", 790, 500, true, Some(b));
        assert_eq!(
            m.get(c).map(|c| (c.rect.x, c.rect.y)),
            Some((0, 0)),
            "would leave the client area"
        );
        assert_eq!(ids(&m), vec![a, b, c]);
    }

    #[test]
    fn activation_reorders_and_close_activates_top() {
        let mut m = Mdi::new(800, 600);
        let a = m.add("a", 100, 100, true, None);
        let b = m.add("b", 100, 100, true, None);
        let c = m.add("c", 100, 100, true, None);
        m.activate(a);
        assert_eq!(ids(&m), vec![b, c, a]);
        m.close(a);
        assert_eq!(m.active(), Some(c));
        m.close(b);
        assert_eq!(
            m.active(),
            Some(c),
            "closing an inactive child keeps the active one"
        );
        m.close(c);
        assert_eq!(m.active(), None);
        m.activate(99);
        assert_eq!(m.active(), None);
    }

    #[test]
    fn ctrl_f6_cycles() {
        let mut m = Mdi::new(800, 600);
        let a = m.add("a", 100, 100, true, None);
        let b = m.add("b", 100, 100, true, None);
        let c = m.add("c", 100, 100, true, None);
        m.next(false);
        assert_eq!(m.active(), Some(b));
        assert_eq!(ids(&m), vec![c, a, b]);
        m.next(true);
        assert_eq!(m.active(), Some(c));
    }

    #[test]
    fn maximized_mode_follows_activation() {
        let mut m = Mdi::new(800, 600);
        let a = m.add("a", 100, 100, true, None);
        let b = m.add("b", 100, 100, true, None);
        m.command(b, Hit::Maximize);
        assert_eq!(m.maximized().map(|c| c.id), Some(b));
        assert_eq!(m.shown_rect(b), Some(Rect::new(-4, -22, 808, 626)));
        m.activate(a);
        assert_eq!(m.maximized().map(|c| c.id), Some(a));
        assert_eq!(m.get(b).map(|c| c.state), Some(State::Normal));
        m.close(a);
        assert_eq!(
            m.maximized().map(|c| c.id),
            Some(b),
            "the next child is maximized"
        );
        m.command(b, Hit::Maximize);
        assert_eq!(
            m.get(b).map(|c| c.state),
            Some(State::Normal),
            "maximize again restores"
        );
    }

    #[test]
    fn minimize_stacks_icons_and_activates_next() {
        let mut m = Mdi::new(400, 300);
        let a = m.add("a", 100, 100, true, None);
        let b = m.add("b", 100, 100, true, None);
        m.command(b, Hit::Minimize);
        assert_eq!(m.active(), Some(a));
        m.command(a, Hit::Minimize);
        assert_eq!(m.shown_rect(a), Some(Rect::new(0, 274, 160, 26)));
        assert_eq!(m.shown_rect(b), Some(Rect::new(160, 274, 160, 26)));
        m.command(a, Hit::Minimize);
        assert_eq!(m.get(a).map(|c| c.state), Some(State::Normal), "restore");
        assert_eq!(m.active(), Some(a));
    }

    #[test]
    fn moves_are_clamped_to_the_client_area() {
        let mut m = Mdi::new(400, 300);
        let a = m.add("a", 100, 80, true, None);
        m.set_rect(a, Rect::new(-50, 1000, 100, 80));
        assert_eq!(m.get(a).map(|c| c.rect), Some(Rect::new(0, 220, 100, 80)));
        m.set_client(200, 150);
        assert_eq!(m.get(a).map(|c| c.rect), Some(Rect::new(0, 70, 100, 80)));
        assert_eq!(
            clamp_into(Rect::new(10, 10, 500, 50), 200, 100),
            Rect::new(0, 10, 500, 50)
        );
    }

    #[test]
    fn sizing_respects_minimum_and_client() {
        let start = Rect::new(100, 100, 200, 150);
        assert_eq!(
            sized(start, Side::SE, 30, 20, 800, 600),
            Rect::new(100, 100, 230, 170)
        );
        assert_eq!(
            sized(start, Side::NW, -30, -20, 800, 600),
            Rect::new(70, 80, 230, 170)
        );
        assert_eq!(
            sized(start, Side::W, 500, 0, 800, 600),
            Rect::new(188, 100, 112, 150),
            "minimum width"
        );
        assert_eq!(
            sized(start, Side::E, 900, 0, 800, 600).right(),
            800,
            "client edge"
        );
        assert_eq!(sized(start, Side::N, 0, -500, 800, 600).y, 0);
        assert_eq!(sized(start, Side::S, 0, -500, 800, 600).h, min_track().1);
        // A child smaller than the minimum near the top left must not panic.
        let small = Rect::new(0, 0, 100, 20);
        assert_eq!(sized(small, Side::W, 10, 0, 800, 600).x, 0);
        assert_eq!(sized(small, Side::N, 0, 10, 800, 600).y, 0);
        assert_eq!(sized(small, Side::NW, -5, -5, 800, 600), small);
        assert_eq!(
            clamp_into(Rect::new(5, 5, i32::MAX, i32::MAX), -10, -10),
            Rect::new(0, 0, i32::MAX, i32::MAX)
        );
    }

    #[test]
    fn a_fixed_size_child_cannot_be_maximized() {
        let mut m = Mdi::new(800, 600);
        let a = m.add("a", 200, 100, false, None);
        m.set_state(a, State::Maximized);
        assert_eq!(m.get(a).map(|c| c.state), Some(State::Normal));
        m.set_state(a, State::Minimized);
        assert_eq!(m.get(a).map(|c| c.state), Some(State::Minimized));
    }

    #[test]
    fn restyle_keeps_the_client_area_and_the_children_inside() {
        use super::super::style::Metrics;
        let mut m = Mdi::new(800, 600);
        let a = m.add("a", 300, 200, true, None);
        let b = m.add("b", 300, 200, true, None);
        m.set_rect(b, Rect::new(500, 400, 300, 200));
        let client = |m: &Mdi, id: u32, mt: Metrics| {
            m.get(id).map(|c| {
                (
                    c.rect.w - 2 * mt.frame,
                    c.rect.h - 2 * mt.frame - mt.caption,
                )
            })
        };
        let before = client(&m, a, Metrics::CLASSIC);
        m.restyle(Metrics::CLASSIC, Metrics::AERO);
        assert_eq!(client(&m, a, Metrics::AERO), before, "same client area");
        let r = m.get(b).map(|c| c.rect).unwrap_or_default();
        assert!(
            r.right() <= 800 && r.bottom() <= 600,
            "{r:?} left the client"
        );
        assert_eq!((r.w, r.h), (308, 212));
        m.restyle(Metrics::AERO, Metrics::CLASSIC);
        assert_eq!(client(&m, a, Metrics::CLASSIC), before, "round trip");
        assert_eq!(m.get(b).map(|c| (c.rect.w, c.rect.h)), Some((300, 200)));
        // A child at the minimum size stays at least the new minimum.
        let small = restyled(Rect::new(0, 0, 112, 22), Metrics::CLASSIC, Metrics::AERO);
        assert!(small.h >= min_track_of(Metrics::AERO).1 && small.w >= 112);
    }

    #[test]
    fn minimized_children_show_some_title_in_every_style() {
        use super::super::style::Metrics;
        assert_eq!(min_width_of(Metrics::CLASSIC), 160, "Windows 95's");
        for m in [
            Metrics::XP,
            Metrics::AERO,
            Metrics::FLAT,
            Metrics::ADWAITA,
            Metrics::BREEZE,
            Metrics::CLEARLOOKS,
            Metrics::MOTIF,
        ] {
            let buttons = 2 * m.btn.0 + m.close_w + m.btn_margin;
            assert!(min_width_of(m) - 2 * m.frame - 21 - buttons >= 75, "{m:?}");
        }
        for m in [Metrics::MAC, Metrics::MAC_MODERN] {
            // The lights measured from the left, plus the title's gap.
            let lights = m.btn_margin + m.close_w + m.btn_gaps.1 + m.btn.0 + m.btn_gaps.0 + m.btn.0;
            assert!(title_inset_of(m) > lights, "{m:?}");
            let title = min_width_of(m) - 2 * m.frame - 2 * title_inset_of(m);
            assert!(title >= 75, "{m:?}: {title}");
        }
    }

    #[test]
    fn mac_caption_buttons_sit_at_the_left() {
        use sn_ui_core::theme::ThemeChoice;
        use sn_ui_core::winstyle::{Look, WinStyle};
        for style in [WinStyle::MacAqua, WinStyle::MacModern] {
            super::super::style::set(
                Look {
                    style,
                    ..Look::default()
                },
                ThemeChoice::Classic,
            );
            let m = super::super::style::metrics();
            let (w, h) = (300, 200);
            let (min, max, close) = caption_buttons(w);
            assert_eq!(close.x, frame_w() + m.btn_margin, "{style:?}");
            assert!(close.right() < min.x && min.right() < max.x, "{style:?}");
            assert_eq!(min.x - close.right(), m.btn_gaps.1);
            assert_eq!(max.x - min.right(), m.btn_gaps.0);
            let st = State::Normal;
            let mid = |r: Rect| (r.x + r.w / 2, r.y + r.h / 2);
            let (cx, cy) = mid(close);
            assert_eq!(hit(w, h, st, true, cx, cy), Some(Hit::Close));
            let (nx, ny) = mid(min);
            assert_eq!(hit(w, h, st, true, nx, ny), Some(Hit::Minimize));
            let (xx, xy) = mid(max);
            assert_eq!(hit(w, h, st, true, xx, xy), Some(Hit::Maximize));
            // No invisible window icon around the lights: left of, above and
            // below the close button the title bar is caption.
            let cap = caption_rect(w);
            assert_eq!(hit(w, h, st, true, cap.x + 2, 10), Some(Hit::Caption));
            assert_eq!(hit(w, h, st, true, cx, cap.y), Some(Hit::Caption));
            assert_eq!(
                hit(w, h, st, true, cx, cap.bottom() - 1),
                Some(Hit::Caption)
            );
            assert_eq!(
                hit(w, h, st, false, nx, ny),
                Some(Hit::Caption),
                "fixed windows have only Close"
            );
        }
        super::super::style::set(Look::WIN95, ThemeChoice::Classic);
    }

    #[test]
    fn the_icon_zone_matches_what_the_linux_looks_draw() {
        use sn_ui_core::theme::ThemeChoice;
        use sn_ui_core::winstyle::{Look, WinStyle};
        let (w, h, st) = (300, 200, State::Normal);
        for (style, icon_w) in [
            (WinStyle::LinuxAdwaita, 0),
            (WinStyle::LinuxBreeze, 18),
            (WinStyle::LinuxClearlooks, 18),
            (
                WinStyle::LinuxMotif,
                super::super::style::Metrics::MOTIF.caption,
            ),
        ] {
            super::super::style::set(
                Look {
                    style,
                    ..Look::default()
                },
                ThemeChoice::Classic,
            );
            let cap = caption_rect(w);
            let y = cap.y + cap.h / 2;
            assert_eq!(icon_zone_w(cap.h), icon_w, "{style:?}");
            let at_8 = if icon_w > 8 { Hit::Icon } else { Hit::Caption };
            assert_eq!(hit(w, h, st, true, cap.x + 8, y), Some(at_8), "{style:?}");
            if icon_w > 0 {
                // The whole drawn button opens the window menu, not more.
                let last = cap.x + icon_w - 1;
                assert_eq!(hit(w, h, st, true, last, y), Some(Hit::Icon), "{style:?}");
                assert_eq!(
                    hit(w, h, st, true, last + 1, y),
                    Some(Hit::Caption),
                    "{style:?}"
                );
            }
        }
        super::super::style::set(Look::WIN95, ThemeChoice::Classic);
    }

    #[test]
    fn hit_testing() {
        let (w, h) = (300, 200);
        let (min, max, close) = caption_buttons(w);
        assert_eq!(close, Rect::new(278, 6, 16, 14));
        assert_eq!(max.right() + 2, close.x);
        assert_eq!(min.right(), max.x);
        let st = State::Normal;
        assert_eq!(hit(w, h, st, true, 280, 8), Some(Hit::Close));
        assert_eq!(hit(w, h, st, true, max.x + 1, 8), Some(Hit::Maximize));
        assert_eq!(hit(w, h, st, true, min.x + 1, 8), Some(Hit::Minimize));
        assert_eq!(
            hit(w, h, st, false, min.x + 1, 8),
            Some(Hit::Caption),
            "fixed windows have only Close"
        );
        assert_eq!(hit(w, h, st, true, 8, 10), Some(Hit::Icon));
        assert_eq!(hit(w, h, st, true, 100, 10), Some(Hit::Caption));
        assert_eq!(hit(w, h, st, true, 1, 1), Some(Hit::Border(Side::NW)));
        assert_eq!(hit(w, h, st, true, 150, 1), Some(Hit::Border(Side::N)));
        assert_eq!(hit(w, h, st, true, 299, 100), Some(Hit::Border(Side::E)));
        assert_eq!(hit(w, h, st, true, 298, 198), Some(Hit::Border(Side::SE)));
        assert_eq!(hit(w, h, st, true, 2, 100), Some(Hit::Border(Side::W)));
        assert_eq!(hit(w, h, State::Maximized, true, 2, 100), Some(Hit::Client));
        assert_eq!(hit(w, h, st, false, 1, 100), Some(Hit::Client));
        assert_eq!(hit(w, h, st, true, 150, 100), Some(Hit::Client));
        assert_eq!(hit(w, h, st, true, 300, 100), None);
        assert_eq!(client_rect(w, h), Rect::new(4, 22, 292, 174));
    }
}
