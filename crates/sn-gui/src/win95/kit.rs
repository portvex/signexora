//! The state every view using the widget kit carries: keyboard focus and tab
//! order, the widget held down by the mouse, mouse capture (`SetCapture`) and
//! auto-repeat of held buttons.
//!
//! A view that uses the kit implements [`Host`] and renders [`capture_layer`]
//! as the first child of its root element. Widgets are functions that build
//! GPUI elements from the view's state; their state structs live in the view
//! and are reached through a [`Lens`].

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Context, DispatchPhase, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, SharedString, Styled, Task, Window, canvas,
};

/// Initial delay before a held scroll or spin button repeats (Windows 95
/// keyboard delay setting 1: 500 ms; scroll bars use about 400 ms).
pub(crate) const REPEAT_DELAY: Duration = Duration::from_millis(400);
/// Interval of the repeat.
pub(crate) const REPEAT_INTERVAL: Duration = Duration::from_millis(50);

/// A mouse handler run while the mouse is captured.
pub(crate) type MouseHandler<V> =
    Rc<dyn Fn(&mut V, Point<Pixels>, &mut Window, &mut Context<'_, V>)>;
/// A key handler of the focused widget. Returns whether it used the key.
pub(crate) type KeyHandler<V> =
    Rc<dyn Fn(&mut V, &KeyDownEvent, &mut Window, &mut Context<'_, V>) -> bool>;
/// An action of the view (a click, a menu command).
pub(crate) type Action<V> = Rc<dyn Fn(&mut V, &mut Window, &mut Context<'_, V>)>;
/// A callback with a value (a new selection, a message box answer, a command).
pub(crate) type Event<V, T> = Rc<dyn Fn(&mut V, T, &mut Window, &mut Context<'_, V>)>;
/// Access to a widget's state inside the view.
pub(crate) type Lens<V, S> = Rc<dyn Fn(&mut V) -> Option<&mut S>>;

/// Make a [`Lens`] from a closure.
pub(crate) fn lens<V, S>(f: impl Fn(&mut V) -> Option<&mut S> + 'static) -> Lens<V, S> {
    Rc::new(f)
}

/// Make an [`Action`] from a closure.
pub(crate) fn action<V>(
    f: impl Fn(&mut V, &mut Window, &mut Context<'_, V>) + 'static,
) -> Action<V> {
    Rc::new(f)
}

/// Mouse capture: all mouse moves and the next button release go to these
/// handlers, wherever the pointer is.
pub(crate) struct Capture<V> {
    pub(crate) on_move: Option<MouseHandler<V>>,
    pub(crate) on_up: Option<MouseHandler<V>>,
}

/// A focusable widget registered during the last render.
struct Focusable<V> {
    id: SharedString,
    scope: SharedString,
    keys: Option<KeyHandler<V>>,
}

/// The widget kit's per-view state.
pub(crate) struct Kit<V> {
    /// The widget held down by the left mouse button.
    pub(crate) pressed: Option<SharedString>,
    /// The widget with the keyboard focus.
    pub(crate) focus: Option<SharedString>,
    /// The open drop-down list (combo box) or other popup owned by a widget.
    pub(crate) popup: Option<SharedString>,
    capture: Option<Capture<V>>,
    /// The button whose release ends the capture (Windows keeps the capture
    /// until the button that started it is released).
    capture_button: MouseButton,
    repeat: Option<Task<()>>,
    order: Vec<Focusable<V>>,
    /// The push buttons of this frame and of the last one, `(id, scope)`:
    /// a dialog's default button gives up its default look while another
    /// push button of its scope has the focus, as in Windows.
    push_buttons: Vec<(SharedString, SharedString)>,
    push_buttons_prev: Vec<(SharedString, SharedString)>,
    /// The next mouse down is dropped if it is the second click of a double
    /// click (a click closed what was under the pointer).
    eat_double: bool,
    /// A mouse down was dropped: drop its release too.
    eat_up: bool,
}

impl<V> Default for Kit<V> {
    fn default() -> Self {
        Self {
            pressed: None,
            focus: None,
            popup: None,
            capture: None,
            capture_button: MouseButton::Left,
            repeat: None,
            order: Vec::new(),
            push_buttons: Vec::new(),
            push_buttons_prev: Vec::new(),
            eat_double: false,
            eat_up: false,
        }
    }
}

impl<V> std::fmt::Debug for Kit<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kit")
            .field("pressed", &self.pressed)
            .field("focus", &self.focus)
            .field("popup", &self.popup)
            .finish_non_exhaustive()
    }
}

/// A view that uses the widget kit.
pub(crate) trait Host: Sized + 'static {
    fn kit(&mut self) -> &mut Kit<Self>;
}

impl<V: 'static> Kit<V> {
    /// Start a new frame: forget the focusable widgets of the last render.
    pub(crate) fn begin_frame(&mut self) {
        self.order.clear();
        self.push_buttons_prev = std::mem::take(&mut self.push_buttons);
    }

    /// Note push button `id` of `scope` for this frame.
    pub(crate) fn push_button_shown(&mut self, id: &str, scope: &str) {
        self.push_buttons
            .push((id.to_owned().into(), scope.to_owned().into()));
    }

    /// Whether the focus is on a push button of `scope` (as of the last
    /// frame, so the order the buttons are drawn in does not matter).
    pub(crate) fn push_button_focused_in(&self, scope: &str) -> bool {
        let Some(f) = self.focus.as_ref() else {
            return false;
        };
        self.push_buttons_prev
            .iter()
            .chain(&self.push_buttons)
            .any(|(id, sc)| id == f && sc.as_ref() == scope)
    }

    /// Whether `id` is held down by the mouse.
    pub(crate) fn is_pressed(&self, id: &str) -> bool {
        self.pressed.as_ref().is_some_and(|p| p.as_ref() == id)
    }

    /// Whether `id`'s popup (a combo box list) is open.
    pub(crate) fn popup_is(&self, id: &str) -> bool {
        self.popup.as_ref().is_some_and(|p| p.as_ref() == id)
    }

    /// Whether `id` has the keyboard focus.
    pub(crate) fn has_focus(&self, id: &str) -> bool {
        self.focus.as_ref().is_some_and(|p| p.as_ref() == id)
    }

    /// Register a focusable widget in tab order. `scope` groups the widgets of one
    /// window or dialog; Tab moves within the scope of the focused widget.
    pub(crate) fn focusable(&mut self, id: &str, scope: &str, keys: Option<KeyHandler<V>>) {
        self.order.push(Focusable {
            id: id.to_owned().into(),
            scope: scope.to_owned().into(),
            keys,
        });
    }

    /// Hold `id` down and capture the mouse.
    pub(crate) fn press(&mut self, id: &str, capture: Option<Capture<V>>) {
        self.pressed = Some(id.to_owned().into());
        self.capture = capture;
        self.capture_button = MouseButton::Left;
    }

    /// Capture the mouse without holding a widget (left-button drags).
    pub(crate) fn capture(&mut self, capture: Capture<V>) {
        self.capture_with(MouseButton::Left, capture);
    }

    /// Capture the mouse for a drag with `button` (the middle-button pan).
    pub(crate) fn capture_with(&mut self, button: MouseButton, capture: Capture<V>) {
        self.capture = Some(capture);
        self.capture_button = button;
    }

    /// Release the mouse: forget the held widget, the capture and the repeat.
    pub(crate) fn release(&mut self) -> Option<Capture<V>> {
        self.pressed = None;
        self.repeat = None;
        self.capture.take()
    }

    /// A click just closed a dialog: drop the next mouse down if it is the
    /// second click of a double click, so it does not reach what was under
    /// the dialog.
    pub(crate) fn eat_double_click(&mut self) {
        self.eat_double = true;
    }

    /// Whether a mouse down with `click_count` is dropped (and forget the
    /// request: only the very next mouse down can be the second click).
    pub(crate) fn eats_mouse_down(&mut self, click_count: usize) -> bool {
        self.eat_up = std::mem::take(&mut self.eat_double) && click_count >= 2;
        self.eat_up
    }

    /// Whether a mouse up is the release of a dropped mouse down.
    pub(crate) fn eats_mouse_up(&mut self) -> bool {
        std::mem::take(&mut self.eat_up)
    }

    /// Whether the mouse is captured.
    pub(crate) fn is_capturing(&self) -> bool {
        self.capture.is_some() || self.pressed.is_some()
    }

    /// The ids of the focusable widgets of `scope`, in tab order.
    pub(crate) fn tab_order(&self, scope: &str) -> Vec<SharedString> {
        self.order
            .iter()
            .filter(|f| f.scope.as_ref() == scope)
            .map(|f| f.id.clone())
            .collect()
    }

    /// The scope of the focused widget.
    pub(crate) fn focus_scope(&self) -> Option<SharedString> {
        let focus = self.focus.as_ref()?;
        self.order
            .iter()
            .find(|f| &f.id == focus)
            .map(|f| f.scope.clone())
    }

    /// Move the focus to the next (or previous) widget of the focused widget's
    /// scope, or to the first widget of `scope` when nothing there has it.
    pub(crate) fn tab(&mut self, scope: &str, back: bool) {
        let ids = self.tab_order(scope);
        self.focus = next_in_cycle(&ids, self.focus.as_ref(), back);
    }

    /// The key handler of the focused widget.
    fn focused_keys(&self) -> Option<KeyHandler<V>> {
        let focus = self.focus.as_ref()?;
        self.order
            .iter()
            .find(|f| &f.id == focus)
            .and_then(|f| f.keys.clone())
    }
}

/// The element after `current` in `ids` (or before it with `back`), wrapping
/// around; the first (or last) element when `current` is not in `ids`.
pub(crate) fn next_in_cycle<T: Clone + PartialEq>(
    ids: &[T],
    current: Option<&T>,
    back: bool,
) -> Option<T> {
    let n = ids.len();
    if n == 0 {
        return None;
    }
    let pos = current.and_then(|c| ids.iter().position(|i| i == c));
    let next = match (pos, back) {
        (None, false) => 0,
        (None, true) => n - 1,
        (Some(p), false) => (p + 1) % n,
        (Some(p), true) => (p + n - 1) % n,
    };
    ids.get(next).cloned()
}

/// Give a key to the focused widget. Returns whether it was used.
pub(crate) fn dispatch_key<V: Host>(
    view: &mut V,
    ev: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) -> bool {
    match view.kit().focused_keys() {
        Some(h) => h(view, ev, window, cx),
        None => false,
    }
}

/// An invisible element covering the view that delivers captured mouse events.
/// Render it as the first child of the view's root element.
pub(crate) fn capture_layer<V: Host>(cx: &mut Context<'_, V>) -> impl IntoElement {
    let view = cx.entity().downgrade();
    canvas(
        |_, _, _| (),
        move |_, (), window, _| {
            let v = view.clone();
            window.on_mouse_event(move |ev: &MouseDownEvent, phase, _, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                let eat = v
                    .update(cx, |this, _| this.kit().eats_mouse_down(ev.click_count))
                    .unwrap_or(false);
                if eat {
                    cx.stop_propagation();
                }
            });
            let v = view.clone();
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                v.update(cx, |this, cx| {
                    let kit = this.kit();
                    if !kit.is_capturing() {
                        return;
                    }
                    let h = kit.capture.as_ref().and_then(|c| c.on_move.clone());
                    if let Some(h) = h {
                        h(this, ev.position, window, cx);
                    }
                    cx.notify();
                })
                .ok();
            });
            let v = view.clone();
            window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                let eat = v
                    .update(cx, |this, _| this.kit().eats_mouse_up())
                    .unwrap_or(false);
                if eat {
                    cx.stop_propagation();
                }
            });
            let v = view.clone();
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                v.update(cx, |this, cx| {
                    let kit = this.kit();
                    // Another button's release does not end the drag.
                    if !kit.is_capturing() || ev.button != kit.capture_button {
                        return;
                    }
                    let cap = kit.release();
                    if let Some(h) = cap.and_then(|c| c.on_up) {
                        h(this, ev.position, window, cx);
                    }
                    cx.notify();
                })
                .ok();
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// Run `step` now and, while `id` stays held down, again after
/// [`REPEAT_DELAY`] and then every [`REPEAT_INTERVAL`] (held scroll arrows).
pub(crate) fn start_repeat<V: Host>(
    view: &mut V,
    id: &str,
    step: Action<V>,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) {
    step(view, window, cx);
    let id: SharedString = id.to_owned().into();
    let task = cx.spawn_in(window, async move |this, cx| {
        let mut wait = REPEAT_DELAY;
        loop {
            cx.background_executor().timer(wait).await;
            wait = REPEAT_INTERVAL;
            let go = this.update_in(cx, |v, window, cx| {
                let held = v.kit().pressed.as_ref() == Some(&id);
                if held {
                    step(v, window, cx);
                    cx.notify();
                }
                held
            });
            if !matches!(go, Ok(true)) {
                break;
            }
        }
    });
    view.kit().repeat = Some(task);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_button_yields_to_a_focused_push_button_of_its_dialog() {
        let mut kit: Kit<()> = Kit::default();
        kit.push_button_shown("ok", "dlg");
        kit.push_button_shown("cancel", "dlg");
        kit.push_button_shown("other", "win");
        kit.focus = Some("cancel".into());
        assert!(kit.push_button_focused_in("dlg"));
        assert!(!kit.push_button_focused_in("win"));
        // A button drawn before the focused one in the next frame still sees it.
        kit.begin_frame();
        kit.push_button_shown("ok", "dlg");
        assert!(kit.push_button_focused_in("dlg"));
        // The focus on an edit field (not a push button) leaves the default.
        kit.focus = Some("field".into());
        assert!(!kit.push_button_focused_in("dlg"));
    }

    #[test]
    fn cycle_wraps_both_ways() {
        let ids = ["a", "b", "c"];
        assert_eq!(next_in_cycle(&ids, None, false), Some("a"));
        assert_eq!(next_in_cycle(&ids, None, true), Some("c"));
        assert_eq!(next_in_cycle(&ids, Some(&"a"), false), Some("b"));
        assert_eq!(next_in_cycle(&ids, Some(&"c"), false), Some("a"));
        assert_eq!(next_in_cycle(&ids, Some(&"a"), true), Some("c"));
        assert_eq!(next_in_cycle(&ids, Some(&"x"), false), Some("a"));
        assert_eq!(next_in_cycle::<&str>(&[], None, false), None);
    }

    struct Dummy;

    #[test]
    fn tab_order_by_scope() {
        let mut kit: Kit<Dummy> = Kit::default();
        kit.focusable("ok", "dlg", None);
        kit.focusable("edit", "w1", None);
        kit.focusable("cancel", "dlg", None);
        assert_eq!(
            kit.tab_order("dlg"),
            vec![SharedString::from("ok"), SharedString::from("cancel")]
        );
        kit.tab("dlg", false);
        assert_eq!(kit.focus.as_ref().map(AsRef::as_ref), Some("ok"));
        assert_eq!(kit.focus_scope().as_ref().map(AsRef::as_ref), Some("dlg"));
        kit.tab("dlg", false);
        assert_eq!(kit.focus.as_ref().map(AsRef::as_ref), Some("cancel"));
        kit.tab("dlg", false);
        assert_eq!(kit.focus.as_ref().map(AsRef::as_ref), Some("ok"));
        kit.tab("dlg", true);
        assert_eq!(kit.focus.as_ref().map(AsRef::as_ref), Some("cancel"));
        kit.press("ok", None);
        assert!(kit.is_pressed("ok") && kit.is_capturing());
        assert!(kit.release().is_none());
        assert!(!kit.is_capturing());
        kit.begin_frame();
        assert_eq!(kit.tab_order("dlg"), Vec::<SharedString>::new());
    }

    #[test]
    fn only_the_second_click_after_a_closing_click_is_eaten() {
        let mut kit = Kit::<()>::default();
        assert!(!kit.eats_mouse_down(2), "nothing was closed");
        kit.eat_double_click();
        assert!(kit.eats_mouse_down(2));
        assert!(kit.eats_mouse_up(), "and its release");
        assert!(!kit.eats_mouse_up());
        assert!(!kit.eats_mouse_down(2), "once");
        kit.eat_double_click();
        assert!(!kit.eats_mouse_down(1), "a single click passes");
        assert!(!kit.eats_mouse_up());
        assert!(!kit.eats_mouse_down(2), "and ends the request");
    }
}
