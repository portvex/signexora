//! Window-only screenshots for visual checks (debug builds on macOS).
//!
//! `SA_GUI_SHOT=/path/out.png cargo run -p sn-gui` waits `SA_GUI_SHOT_DELAY_MS`
//! (default 3000), saves a PNG of the app's own window and quits. Only windows
//! owned by this process are captured, never the screen, and no input events
//! are sent anywhere. A process may read its own windows without the Screen
//! Recording permission, which `screencapture -l` would need.
//!
//! `SA_GUI_SCENE` (see `app::Frame::apply_scene`) opens a menu, a message box
//! or the widget gallery first, so each look can be checked without input.

use std::path::{Path, PathBuf};
use std::time::Duration;

use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::geometry::{CGPoint, CGRect, CGSize};
use core_graphics::window::{
    create_description_from_array, create_image, create_window_list, kCGNullWindowID,
    kCGWindowImageBestResolution, kCGWindowImageBoundsIgnoreFraming, kCGWindowListOptionAll,
    kCGWindowListOptionIncludingWindow,
};
use gpui::App;

/// Start the delayed capture if `SA_GUI_SHOT` is set.
pub(crate) fn schedule(cx: &mut App) {
    let Some(path) = std::env::var_os("SA_GUI_SHOT").map(PathBuf::from) else {
        return;
    };
    let delay = std::env::var("SA_GUI_SHOT_DELAY_MS")
        .ok()
        .and_then(|d| d.parse().ok())
        .unwrap_or(3000);
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(Duration::from_millis(delay))
            .await;
        match capture(&path) {
            Ok(n) => eprintln!("sn-gui: saved {n} window(s) to {}", path.display()),
            Err(e) => eprintln!("sn-gui: window capture failed: {e}"),
        }
        cx.update(|cx| cx.quit()).ok();
    })
    .detach();
}

/// A number from a window description.
fn number(
    d: &core_foundation::dictionary::CFDictionary<CFString, core_foundation::base::CFType>,
    key: &'static str,
) -> Option<i64> {
    d.find(CFString::from_static_string(key))?
        .downcast::<CFNumber>()?
        .to_i64()
}

/// The ids of this process's normal (layer 0) windows.
fn own_windows() -> Vec<u32> {
    let pid = i64::from(std::process::id());
    let Some(list) = create_window_list(kCGWindowListOptionAll, kCGNullWindowID) else {
        return Vec::new();
    };
    let Some(desc) = create_description_from_array(list) else {
        return Vec::new();
    };
    desc.iter()
        .filter(|d| {
            number(d, "kCGWindowOwnerPID") == Some(pid) && number(d, "kCGWindowLayer") == Some(0)
        })
        .filter_map(|d| number(&d, "kCGWindowNumber").and_then(|n| u32::try_from(n).ok()))
        .collect()
}

/// Save each own window as a PNG (`out.png`, then `out-2.png`, ...).
fn capture(path: &Path) -> Result<usize, String> {
    let ids = own_windows();
    if ids.is_empty() {
        return Err("no window of this process found".into());
    }
    let null = CGRect::new(
        &CGPoint::new(f64::INFINITY, f64::INFINITY),
        &CGSize::new(0.0, 0.0),
    );
    let mut saved = 0;
    for id in &ids {
        let opts = kCGWindowImageBoundsIgnoreFraming | kCGWindowImageBestResolution;
        // A window without contents (an off-screen helper window) cannot be
        // read; it is skipped.
        let Some(img) = create_image(null, kCGWindowListOptionIncludingWindow, *id, opts) else {
            continue;
        };
        let i = saved;
        let (w, h, stride) = (img.width(), img.height(), img.bytes_per_row());
        let data = img.data();
        let bytes = data.bytes();
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            let row = bytes
                .get(y * stride..y * stride + w * 4)
                .ok_or("short image data")?;
            for &[b, g, r, a] in row.as_chunks::<4>().0 {
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
        let out = if i == 0 {
            path.to_path_buf()
        } else {
            path.with_file_name(format!(
                "{}-{}.png",
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("shot"),
                i + 1
            ))
        };
        image::save_buffer(&out, &rgba, w as u32, h as u32, image::ColorType::Rgba8)
            .map_err(|e| e.to_string())?;
        saved += 1;
    }
    if saved == 0 {
        return Err("no window of this process could be read".into());
    }
    Ok(saved)
}
