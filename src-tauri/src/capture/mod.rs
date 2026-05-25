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
mod macos;
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

use crate::{
    config::AppConfig,
    diff,
    translate::{TranslateEngine, TranslateError},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub config: AppConfig,
    pub is_capturing: bool,
}

pub async fn run_capture_loop(app: tauri::AppHandle, state: Arc<Mutex<AppState>>) {
    let mut prev_image: Option<DynamicImage> = None;
    let mut inflight_cancel: Option<CancellationToken> = None;

    loop {
        sleep(Duration::from_millis(500)).await;

        let (is_capturing, config) = {
            let s = state.lock().await;
            (s.is_capturing, s.config.clone())
        };

        if !is_capturing {
            continue;
        }

        let window = match app.get_webview_window("main") {
            Some(w) => w,
            None => continue,
        };

        let pos = match window.outer_position() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let size = match window.outer_size() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let rect = CaptureRect {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
        };

        #[cfg(target_os = "macos")]
        let window_id = {
            use objc::{msg_send, runtime::Object, sel, sel_impl};
            let ns_win = match window.ns_window() {
                Ok(ptr) => ptr as *const Object,
                Err(_) => continue,
            };
            let n: i32 = unsafe { msg_send![ns_win, windowNumber] };
            n as u32
        };

        #[cfg(not(target_os = "macos"))]
        let window_id: u32 = 0;

        let current = match capture_below_window(rect, window_id) {
            Ok(img) => img,
            Err(_) => continue,
        };

        if let Some(ref prev) = prev_image {
            let score = diff::compute_diff_score(prev, &current);
            if !diff::is_changed(score, config.delta_threshold) {
                continue;
            }
        }

        prev_image = Some(current.clone());

        if let Some(token) = inflight_cancel.take() {
            token.cancel();
        }

        let token = CancellationToken::new();
        inflight_cancel = Some(token.clone());

        let app_clone = app.clone();
        let lang = config.target_language.clone();
        let engine = TranslateEngine::new(
            config.gateway_url.clone(),
            config.model.clone(),
            config.api_key.clone(),
        );

        tauri::async_runtime::spawn(async move {
            let _ = app_clone.emit("translation-loading", ());
            match engine.translate(&current, &lang, token).await {
                Ok(text) => {
                    let _ = app_clone.emit("translation-updated", text);
                }
                Err(TranslateError::Cancelled) => {}
                Err(e) => {
                    let _ = app_clone.emit("translation-error", e.to_string());
                }
            }
        });
    }
}
