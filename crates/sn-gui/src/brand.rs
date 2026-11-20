//! The shared SigNexora mark in window captions, About and the macOS Dock.

use std::sync::{Arc, OnceLock};

use gpui::{Bounds, Corners, Pixels, Point, RenderImage, Window, point, px, size};

const ICON: &[u8] = include_bytes!("../assets/branding/signexora-64.png");

fn render_image() -> Option<Arc<RenderImage>> {
    static IMAGE: OnceLock<Option<Arc<RenderImage>>> = OnceLock::new();
    IMAGE
        .get_or_init(|| {
            let mut pixels = match image::load_from_memory(ICON) {
                Ok(image) => image.to_rgba8(),
                Err(error) => {
                    eprintln!("signexora: cannot decode the application icon: {error}");
                    return None;
                }
            };
            // GPUI's RenderImage stores BGRA, while PNG decodes to RGBA.
            for pixel in pixels.pixels_mut() {
                pixel.0.swap(0, 2);
            }
            Some(Arc::new(RenderImage::new(vec![image::Frame::new(pixels)])))
        })
        .clone()
}

/// Draw the embedded mark at a logical pixel size, preserving its alpha.
pub(crate) fn paint(window: &mut Window, origin: Point<Pixels>, x: i32, y: i32, side: i32) {
    let Some(image) = render_image() else {
        return;
    };
    let bounds = Bounds::new(
        origin + point(px(x as f32), px(y as f32)),
        size(px(side as f32), px(side as f32)),
    );
    if let Err(error) = window.paint_image(bounds, Corners::default(), image, 0, false) {
        eprintln!("signexora: cannot draw the application icon: {error}");
    }
}

/// Set the Dock icon even when launched directly through `cargo run`.
pub(crate) fn install_native_icon() {
    // GPUI's simulated test application must never change the desktop's icon.
    #[cfg(all(target_os = "macos", not(test)))]
    {
        use objc2::{AnyThread, MainThreadMarker};
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::NSData;

        let Some(main_thread) = MainThreadMarker::new() else {
            return;
        };
        let data = NSData::with_bytes(include_bytes!("../assets/branding/signexora.icns"));
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
            eprintln!("signexora: cannot decode the Dock icon");
            return;
        };
        let app = NSApplication::sharedApplication(main_thread);
        #[allow(unsafe_code)]
        // SAFETY: AppKit is called on the main thread, with a non-null, live
        // NSImage. NSApplication retains the image for subsequent Dock draws.
        unsafe {
            app.setApplicationIconImage(Some(&image));
        }
    }
}
