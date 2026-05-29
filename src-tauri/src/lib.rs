pub mod capture;
pub mod config;
pub mod diff;
pub mod history;
pub mod translate;

use capture::AppState;
use history::TranslationHistory;
use std::sync::Arc;
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use translate::{TranslateEngine, TranslateError};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                capture::macos::request_screen_capture_permission();
                if let Some(win) = app.get_webview_window("main") {
                    if let Ok(ns_win) = win.ns_window() {
                        capture::macos::set_all_spaces(ns_win);
                    }
                }
            }

            let dir = app.path().app_data_dir()?;
            let config = config::AppConfig::load_or_default(dir.clone());
            let state = Arc::new(Mutex::new(AppState {
                config,
                is_capturing: true,
                is_stale: false,
                inflight_cancel: None,
            }));
            app.manage(state.clone());

            let hist = Arc::new(Mutex::new(TranslationHistory::load(&dir)));
            app.manage(hist);

            let tray = TrayIconBuilder::with_id("main-tray")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray.png"
                ))?)
                .icon_as_template(true)
                .tooltip("Konjac")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                        let _ = tray.set_visible(false);
                    }
                })
                .build(app)?;
            tray.set_visible(false)?;

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
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                if let Some(tray) = window.app_handle().tray_by_id("main-tray") {
                    let _ = tray.set_visible(true);
                }
            }
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
            get_history,
            delete_history_item,
            clear_history,
            show_tray,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

type ManagedState = Arc<Mutex<AppState>>;
type ManagedHistory = Arc<Mutex<TranslationHistory>>;

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
    history: tauri::State<'_, ManagedHistory>,
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
    let history_arc = history.inner().clone();

    tauri::async_runtime::spawn(async move {
        match engine.translate(&image, &lang, token).await {
            Ok(text) => {
                if let Ok(dir) = app_clone.path().app_data_dir() {
                    history_arc.lock().await.push(text.clone(), &dir);
                }
                let _ = app_clone.emit("translation-updated", text);
            }
            Err(TranslateError::Cancelled) => {}
            Err(e) => { let _ = app_clone.emit("translation-error", e.to_string()); }
        }
    });

    Ok(())
}

#[tauri::command]
async fn get_history(
    history: tauri::State<'_, ManagedHistory>,
) -> Result<Vec<history::HistoryEntry>, String> {
    Ok(history.lock().await.entries.clone())
}

#[tauri::command]
async fn delete_history_item(
    id: u64,
    history: tauri::State<'_, ManagedHistory>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    history.lock().await.remove(id, &dir);
    Ok(())
}

#[tauri::command]
async fn clear_history(
    history: tauri::State<'_, ManagedHistory>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    history.lock().await.clear(&dir);
    Ok(())
}

#[tauri::command]
fn show_tray(app: tauri::AppHandle) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_visible(true);
    }
}
