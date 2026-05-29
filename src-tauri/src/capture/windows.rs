use super::{CaptureError, CaptureRect};
use image::DynamicImage;

pub fn capture_below_window(rect: CaptureRect, _window_id: u32) -> Result<DynamicImage, CaptureError> {
    use screenshots::Screen;

    let screens = Screen::all().map_err(|e| CaptureError::Platform(e.to_string()))?;
    let screen = screens
        .first()
        .ok_or_else(|| CaptureError::Platform("no screens found".to_string()))?;

    let capture = screen
        .capture_area(rect.x, rect.y, rect.width, rect.height)
        .map_err(|e| CaptureError::Platform(e.to_string()))?;

    let (w, h) = (capture.width(), capture.height());
    let rgba = image::RgbaImage::from_raw(w, h, capture.into_raw())
        .ok_or(CaptureError::ConversionError)?;

    Ok(DynamicImage::ImageRgba8(rgba))
}

/// Call once at startup to exclude our overlay window from all screen captures.
pub fn init_window_exclusion(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};
    unsafe {
        let _ = SetWindowDisplayAffinity(HWND(hwnd as *mut _), WDA_EXCLUDEFROMCAPTURE);
    }
}
