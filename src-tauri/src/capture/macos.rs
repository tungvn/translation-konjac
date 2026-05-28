use super::{CaptureError, CaptureRect};
use image::DynamicImage;
use std::process::Command;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// Check without prompting. Safe to call from any thread.
pub fn has_screen_capture_permission() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// Prompt once. Must be called from the main thread.
pub fn request_screen_capture_permission() {
    unsafe { CGRequestScreenCaptureAccess() };
}

pub fn capture_below_window(rect: CaptureRect, _window_id: u32) -> Result<DynamicImage, CaptureError> {
    if rect.width == 0 || rect.height == 0 {
        return Err(CaptureError::Platform("zero-dimension rect".to_string()));
    }

    if !has_screen_capture_permission() {
        return Err(CaptureError::Platform(
            "Screen Recording permission not granted — open System Settings > Privacy & Security > Screen Recording".to_string(),
        ));
    }

    let tmp = "/tmp/konjac_capture.jpg";
    let region = format!("{},{},{},{}", rect.x, rect.y, rect.width, rect.height);

    let out = Command::new("screencapture")
        .args(["-x", "-R", &region, "-t", "jpeg", tmp])
        .output()
        .map_err(|e| CaptureError::Platform(e.to_string()))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(CaptureError::Platform(format!(
            "screencapture failed — check Screen Recording permission in System Settings ({})",
            stderr.trim()
        )));
    }

    image::open(tmp).map_err(|_| CaptureError::ConversionError)
}
