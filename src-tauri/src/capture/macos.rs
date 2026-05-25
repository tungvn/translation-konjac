use super::{CaptureError, CaptureRect};
use image::{DynamicImage, RgbaImage};
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint { x: f64, y: f64 }

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize { width: f64, height: f64 }

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect { origin: CGPoint, size: CGSize }

// kCGWindowListOptionOnScreenBelowWindow = 1 << 4
const CG_WINDOW_LIST_OPTION_ON_SCREEN_BELOW_WINDOW: u32 = 1 << 4;
const CG_WINDOW_IMAGE_DEFAULT: u32 = 0;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCreateImage(
        screen_bounds: CGRect,
        list_option: u32,
        window_id: u32,
        image_option: u32,
    ) -> *mut c_void;

    fn CGImageGetWidth(image: *const c_void) -> usize;
    fn CGImageGetHeight(image: *const c_void) -> usize;
    fn CGImageGetDataProvider(image: *const c_void) -> *mut c_void;
    fn CGDataProviderCopyData(provider: *const c_void) -> *const c_void;
    fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
    fn CFDataGetLength(data: *const c_void) -> isize;
    fn CFRelease(cf: *const c_void);
    fn CGImageRelease(image: *mut c_void);
}

pub fn capture_below_window(rect: CaptureRect, window_id: u32) -> Result<DynamicImage, CaptureError> {
    unsafe {
        let cg_rect = CGRect {
            origin: CGPoint { x: rect.x as f64, y: rect.y as f64 },
            size: CGSize { width: rect.width as f64, height: rect.height as f64 },
        };

        let image_ref = CGWindowListCreateImage(
            cg_rect,
            CG_WINDOW_LIST_OPTION_ON_SCREEN_BELOW_WINDOW,
            window_id,
            CG_WINDOW_IMAGE_DEFAULT,
        );

        if image_ref.is_null() {
            return Err(CaptureError::NullImage);
        }

        let width = CGImageGetWidth(image_ref) as u32;
        let height = CGImageGetHeight(image_ref) as u32;
        let provider = CGImageGetDataProvider(image_ref);
        let data_ref = CGDataProviderCopyData(provider);

        if data_ref.is_null() {
            CGImageRelease(image_ref);
            return Err(CaptureError::ConversionError);
        }

        let len = CFDataGetLength(data_ref) as usize;
        let ptr = CFDataGetBytePtr(data_ref);
        let bytes = std::slice::from_raw_parts(ptr, len).to_vec();

        CFRelease(data_ref);
        CGImageRelease(image_ref);

        // CoreGraphics returns BGRA; swap to RGBA
        let rgba_bytes: Vec<u8> = bytes
            .chunks(4)
            .flat_map(|px| [px[2], px[1], px[0], px[3]])
            .collect();

        RgbaImage::from_raw(width, height, rgba_bytes)
            .map(DynamicImage::ImageRgba8)
            .ok_or(CaptureError::ConversionError)
    }
}
