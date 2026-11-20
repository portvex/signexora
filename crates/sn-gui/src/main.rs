//! SigNexora (`signexora`) with the look and behaviour of Signals Analyzer on
//! Windows 95, built on GPUI: the MDI frame with the session, SA's loader,
//! dialogs and message boxes on the Windows 95 widget kit. It replaced the
//! earlier egui front end (`sa-app`, removed in Phase 10 step 8); comments
//! saying "as `sa-app`" refer to that app's behaviour, kept here.
//!
//! `signexora <file>` opens `SA Open file` on that file.

mod app;
mod brand;
#[cfg(all(debug_assertions, target_os = "macos"))]
mod shot;
mod store;
// The kit is ahead of the app: until the SA windows of the next steps use
// them, some widgets are only used by the debug-build widget gallery.
#[cfg_attr(not(debug_assertions), allow(dead_code))]
mod win95;

use std::path::PathBuf;

use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, point, px, size};
use sn_ui_core::prefs::StartSize;

/// The frame's size when nothing else says (and the first start): the
/// 1024 x 768 screen SA was made for; the PhasePlane form (936 px) fits.
const DEFAULT_SIZE: (f32, f32) = (1024.0, 768.0);

/// Where the frame opens: `Size of start SA`.
fn window_bounds(stored: &store::Stored, display: Option<(f32, f32)>) -> WindowBounds {
    let (mut w, mut h) = match (stored.prefs.start_size, stored.frame) {
        (StartSize::Last, Some(f)) => f,
        (s, _) => s.size().map_or(DEFAULT_SIZE, |[w, h]| (w, h)),
    };
    // Keep the title bar and the controls on screen: never larger than the
    // display (less the 60 px offset), never smaller than the minimum size.
    if let Some((dw, dh)) = display.filter(|(dw, dh)| *dw > 0.0 && *dh > 0.0) {
        w = w.min((dw - 60.0).max(320.0));
        h = h.min((dh - 60.0).max(200.0));
    }
    let bounds = Bounds::new(point(px(60.0), px(60.0)), size(px(w), px(h)));
    if stored.prefs.start_size == StartSize::Maximized {
        WindowBounds::Maximized(bounds)
    } else {
        WindowBounds::Windowed(bounds)
    }
}

fn main() {
    let file = std::env::args_os().nth(1).map(PathBuf::from);
    // Window captures for visual checks must not touch the user's settings.
    let capture = cfg!(debug_assertions) && std::env::var_os("SA_GUI_SHOT").is_some();
    let path = if capture { None } else { store::default_path() };
    let stored = path.as_deref().map(store::Stored::load).unwrap_or_default();
    sn_ui_core::i18n::set_language(stored.prefs.effective_language());
    Application::new().run(move |cx: &mut App| {
        brand::install_native_icon();
        if let Err(e) = win95::text::load_fonts(cx) {
            eprintln!("sn-gui: could not load the user-interface fonts: {e}");
        }
        let display = cx.primary_display().map(|d| {
            let b = d.bounds();
            (f32::from(b.size.width), f32::from(b.size.height))
        });
        let options = WindowOptions {
            window_bounds: Some(window_bounds(&stored, display)),
            window_min_size: Some(size(px(320.0), px(200.0))),
            ..WindowOptions::default()
        };
        let setup = app::Setup {
            stored: stored.clone(),
            store: path.clone(),
            file: file.clone(),
        };
        let opened = cx.open_window(options, |window, cx| {
            window.set_window_title(&app::frame_title());
            let view = cx.new(|cx| app::Frame::with_setup(setup, window, cx));
            let v = view.downgrade();
            window.on_window_should_close(cx, move |_, cx| {
                v.update(cx, app::Frame::should_close).unwrap_or(true)
            });
            view
        });
        if let Err(e) = opened {
            eprintln!("sn-gui: could not open the main window: {e}");
            cx.quit();
            return;
        }
        #[cfg(all(debug_assertions, target_os = "macos"))]
        shot::schedule(cx);
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_size_picks_the_frame_bounds() {
        let mut s = store::Stored::default();
        let size_of = |b: WindowBounds| {
            let r = b.get_bounds();
            (f32::from(r.size.width), f32::from(r.size.height))
        };
        assert_eq!(size_of(window_bounds(&s, None)), DEFAULT_SIZE);
        s.frame = Some((1000.0, 700.0));
        assert_eq!(size_of(window_bounds(&s, None)), (1000.0, 700.0));
        s.prefs.start_size = StartSize::S800x600;
        assert_eq!(size_of(window_bounds(&s, None)), (800.0, 600.0));
        s.prefs.start_size = StartSize::Maximized;
        assert!(matches!(
            window_bounds(&s, None),
            WindowBounds::Maximized(_)
        ));
        s.prefs.start_size = StartSize::Last;
        s.frame = Some((5000.0, 4000.0));
        assert_eq!(
            size_of(window_bounds(&s, Some((1920.0, 1080.0)))),
            (1860.0, 1020.0),
            "fits on the display"
        );
    }
}
