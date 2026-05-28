pub mod capture;
pub mod config;
pub mod diff;
pub mod translate;

use capture::AppState;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use translate::{TranslateEngine, TranslateError};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            capture::macos::request_screen_capture_permission();

            let dir = app.path().app_data_dir()?;
            let config = config::AppConfig::load_or_default(dir);
            let state = Arc::new(Mutex::new(AppState {
                config,
                is_capturing: true,
                is_stale: false,
                inflight_cancel: None,
            }));
            app.manage(state.clone());

            #[cfg(target_os = "windows")]
            if let Some(win) = app.get_webview_window("main") {
                if let Ok(hwnd) = win.hwnd() {
                    capture::windows::init_window_exclusion(hwnd.0 as isize);
                }
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                capture::run_capture_loop(app_handle, state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_target_language,
            set_delta_threshold,
            pause_capture,
            resume_capture,
            get_config,
            save_config,
            translate_now,
            get_stale,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

type ManagedState = Arc<Mutex<AppState>>;

#[tauri::command]
async fn get_config(state: tauri::State<'_, ManagedState>) -> Result<config::AppConfig, String> {
    Ok(state.lock().await.config.clone())
}

#[tauri::command]
async fn get_stale(state: tauri::State<'_, ManagedState>) -> Result<bool, String> {
    Ok(state.lock().await.is_stale)
}

#[tauri::command]
async fn set_target_language(
    language: String,
    state: tauri::State<'_, ManagedState>,
) -> Result<(), String> {
    state.lock().await.config.target_language = language;
    Ok(())
}

#[tauri::command]
async fn set_delta_threshold(
    threshold: f32,
    state: tauri::State<'_, ManagedState>,
) -> Result<(), String> {
    state.lock().await.config.delta_threshold = threshold;
    Ok(())
}

#[tauri::command]
async fn pause_capture(state: tauri::State<'_, ManagedState>) -> Result<(), String> {
    state.lock().await.is_capturing = false;
    Ok(())
}

#[tauri::command]
async fn resume_capture(state: tauri::State<'_, ManagedState>) -> Result<(), String> {
    state.lock().await.is_capturing = true;
    Ok(())
}

#[tauri::command]
async fn save_config(
    config: config::AppConfig,
    state: tauri::State<'_, ManagedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    config.save(dir).map_err(|e| e.to_string())?;
    let mut s = state.lock().await;
    s.config = config;
    let should_notify = !s.is_stale;
    if should_notify {
        s.is_stale = true;
    }
    drop(s);
    if should_notify {
        let _ = app.emit("capture-stale", ());
    }
    Ok(())
}

#[tauri::command]
async fn translate_now(
    state: tauri::State<'_, ManagedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let config = {
        let s = state.lock().await;
        if !s.is_capturing {
            return Ok(());
        }
        s.config.clone()
    };

    if config.gateway_url.is_empty() || config.api_key.is_empty() {
        let _ = app.emit("translation-error", "Open Settings to configure the API gateway URL and key.");
        return Ok(());
    }

    // Cancel any in-flight translation.
    {
        let mut s = state.lock().await;
        if let Some(old) = s.inflight_cancel.take() {
            old.cancel();
        }
        s.is_stale = false;
    }

    let _ = app.emit("translation-loading", ());

    let window = app.get_webview_window("main").ok_or("no window")?;
    let image = match capture::capture_window_region(&window) {
        Ok(img) => img,
        Err(e) => {
            let _ = app.emit("translation-error", e.to_string());
            return Ok(());
        }
    };

    let token = CancellationToken::new();
    state.lock().await.inflight_cancel = Some(token.clone());

    let engine = TranslateEngine::new(config.gateway_url, config.model, config.api_key);
    let lang = config.target_language;
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        match engine.translate(&image, &lang, token).await {
            Ok(text) => { let _ = app_clone.emit("translation-updated", text); }
            Err(TranslateError::Cancelled) => {}
            Err(e) => { let _ = app_clone.emit("translation-error", e.to_string()); }
        }
    });

    Ok(())
}
