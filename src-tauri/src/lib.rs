pub mod capture;
pub mod config;
pub mod diff;
pub mod translate;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use capture::{run_capture_loop, AppState};
    use config::AppConfig;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let config = AppConfig::load_or_default(dir);
            let state = Arc::new(Mutex::new(AppState {
                config,
                is_capturing: true,
            }));
            app.manage(state.clone());

            #[cfg(target_os = "windows")]
            {
                if let Some(win) = app.get_webview_window("main") {
                    if let Ok(hwnd) = win.hwnd() {
                        capture::windows::init_window_exclusion(hwnd.0 as isize);
                    }
                }
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                run_capture_loop(app_handle, state).await;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn set_target_language(
    language: String,
    state: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<capture::AppState>>>,
) -> Result<(), String> {
    state.lock().await.config.target_language = language;
    Ok(())
}

#[tauri::command]
async fn set_delta_threshold(
    threshold: f32,
    state: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<capture::AppState>>>,
) -> Result<(), String> {
    state.lock().await.config.delta_threshold = threshold;
    Ok(())
}

#[tauri::command]
async fn pause_capture(
    state: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<capture::AppState>>>,
) -> Result<(), String> {
    state.lock().await.is_capturing = false;
    Ok(())
}

#[tauri::command]
async fn resume_capture(
    state: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<capture::AppState>>>,
) -> Result<(), String> {
    state.lock().await.is_capturing = true;
    Ok(())
}

#[tauri::command]
async fn get_config(
    state: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<capture::AppState>>>,
) -> Result<config::AppConfig, String> {
    Ok(state.lock().await.config.clone())
}

#[tauri::command]
async fn save_config(
    config: config::AppConfig,
    state: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<capture::AppState>>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    config.save(dir).map_err(|e| e.to_string())?;
    state.lock().await.config = config;
    Ok(())
}
