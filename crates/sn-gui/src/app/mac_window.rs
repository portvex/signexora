//! The main window's native macOS title bar in the macOS looks: light or
//! dark as the chosen look, rather than following the system's appearance.
//! The other looks (and every other platform) leave the window as it is.

use gpui::Window;

/// The appearance of the main window's title bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TitleAppearance {
    /// As the system's (the Windows looks).
    System,
    /// Light (a light macOS look).
    Light,
    /// Dark (a dark macOS look).
    Dark,
}

/// Give the main window's title bar `appearance`.
#[cfg_attr(any(not(target_os = "macos"), test), allow(unused_variables))]
pub(crate) fn set_appearance(window: &Window, appearance: TitleAppearance) {
    // Test windows have no platform window behind them.
    #[cfg(all(target_os = "macos", not(test)))]
    mac::set(window, appearance);
}

#[cfg(all(target_os = "macos", not(test)))]
mod mac {
    use gpui::Window;
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
        NSView,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use super::TitleAppearance;

    pub(super) fn set(window: &Window, appearance: TitleAppearance) {
        let dark = match appearance {
            TitleAppearance::System => None,
            TitleAppearance::Light => Some(false),
            TitleAppearance::Dark => Some(true),
        };
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::AppKit(h) = handle.as_raw() else {
            return;
        };
        #[allow(unsafe_code)]
        // SAFETY: GPUI gives the content view of its own live NSWindow; this
        // runs on the main thread (inside a render), as AppKit requires, and
        // the view outlives the call.
        let view: &NSView = unsafe { h.ns_view.cast::<NSView>().as_ref() };
        let Some(ns_window) = view.window() else {
            return;
        };
        let appearance = dark.and_then(|d| {
            #[allow(unsafe_code)]
            // SAFETY: AppKit's appearance names are immutable framework
            // constants, valid for the life of the process.
            let name = unsafe {
                if d {
                    NSAppearanceNameDarkAqua
                } else {
                    NSAppearanceNameAqua
                }
            };
            NSAppearance::appearanceNamed(name)
        });
        ns_window.setAppearance(appearance.as_deref());
    }
}
