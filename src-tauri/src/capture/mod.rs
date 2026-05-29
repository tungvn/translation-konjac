use image::DynamicImage;
use tauri::{Emitter, Manager};
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct CaptureRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture returned null image")]
    NullImage,
    #[error("image conversion failed")]
    ConversionError,
    #[error("platform error: {0}")]
    Platform(String),
}

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub fn capture_below_window(rect: CaptureRect, window_id: u32) -> Result<DynamicImage, CaptureError> {
    #[cfg(target_os = "macos")]
    return macos::capture_below_window(rect, window_id);

    #[cfg(target_os = "windows")]
    return windows::capture_below_window(rect, window_id);

    #[allow(unreachable_code)]
    Err(CaptureError::Platform("unsupported platform".to_string()))
}

/// Captures the region beneath the overlay window, excluding the toolbar.
pub fn capture_window_region(window: &tauri::WebviewWindow) -> Result<DynamicImage, CaptureError> {
    let scale = window.scale_factor().unwrap_or(1.0);
    let pos = window
        .outer_position()
        .map_err(|e| CaptureError::Platform(e.to_string()))?;
    let size = window
        .outer_size()
        .map_err(|e| CaptureError::Platform(e.to_string()))?;

    let lx = (pos.x as f64 / scale) as i32;
    let ly = (pos.y as f64 / scale) as i32;
    let lw = (size.width as f64 / scale) as u32;
    let lh = (size.height as f64 / scale) as u32;

    let toolbar_pt = 32_u32;
    let rect = CaptureRect {
        x: lx,
        y: ly + toolbar_pt as i32,
        width: lw,
        height: lh.saturating_sub(toolbar_pt),
    };

    capture_below_window(rect, 0)
}

use crate::{config::AppConfig, diff};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub config: AppConfig,
    pub is_capturing: bool,
    pub is_stale: bool,
    pub inflight_cancel: Option<CancellationToken>,
}

/// Monitors for capture changes and emits `capture-stale` when the content
/// under the window differs from the previous check. Does NOT auto-translate.
pub async fn run_capture_loop(app: tauri::AppHandle, state: Arc<Mutex<AppState>>) {
    let mut prev_image: Option<DynamicImage> = None;
    let mut prev_rect: Option<(i32, i32, u32, u32)> = None;

    loop {
        sleep(Duration::from_millis(600)).await;

        let (is_capturing, config) = {
            let s = state.lock().await;
            (s.is_capturing, s.config.clone())
        };

        if !is_capturing {
            prev_image = None;
            prev_rect = None;
            continue;
        }

        let window = match app.get_webview_window("main") {
            Some(w) => w,
            None => continue,
        };

        let scale = window.scale_factor().unwrap_or(1.0);
        let pos = match window.outer_position() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let size = match window.outer_size() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let rect_key = (pos.x, pos.y, size.width, size.height);
        let rect_changed = prev_rect != Some(rect_key);
        prev_rect = Some(rect_key);

        let toolbar_pt = 32_u32;
        let lx = (pos.x as f64 / scale) as i32;
        let ly = (pos.y as f64 / scale) as i32;
        let lw = (size.width as f64 / scale) as u32;
        let lh = (size.height as f64 / scale) as u32;
        let rect = CaptureRect {
            x: lx,
            y: ly + toolbar_pt as i32,
            width: lw,
            height: lh.saturating_sub(toolbar_pt),
        };

        let current = match capture_below_window(rect, 0) {
            Ok(img) => img,
            Err(_) => {
                prev_image = None;
                continue;
            }
        };

        let image_changed = match &prev_image {
            Some(prev) => {
                let score = diff::compute_diff_score(prev, &current);
                diff::is_changed(score, config.delta_threshold)
            }
            None => true,
        };

        prev_image = Some(current);

        if rect_changed || image_changed {
            let should_notify = {
                let mut s = state.lock().await;
                if !s.is_stale {
                    s.is_stale = true;
                    true
                } else {
                    false
                }
            };
            if should_notify {
                let _ = app.emit("capture-stale", ());
            }
        }
    }
}
