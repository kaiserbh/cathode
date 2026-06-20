pub mod commands;
pub mod http;
pub mod state;

use state::AppState;
use tauri::Manager;
use tauri_plugin_libmpv::{MpvConfig, MpvExt};

/// The single player window label.
const MAIN_WINDOW: &str = "main";

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Initialize an embedded mpv instance for the main window. mpv renders to a
/// native surface composited under the transparent webview.
fn init_mpv(app: &tauri::App) {
    let config: MpvConfig = serde_json::from_value(serde_json::json!({
        "initialOptions": { "hwdec": "auto-safe" },
        "observedProperties": {}
    }))
    .expect("static mpv config is valid");

    match app.get_webview_window(MAIN_WINDOW) {
        Some(window) => {
            if let Err(e) = window.mpv().init(config, MAIN_WINDOW) {
                tracing::error!("failed to initialize mpv: {e}");
            }
        }
        None => tracing::error!("main window not found; mpv not initialized"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_libmpv::init())
        .manage(AppState::new())
        .setup(|app| {
            init_mpv(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::sources::list_categories,
            commands::sources::list_streams,
            commands::playback::play_stream,
            commands::playback::pause_playback,
            commands::playback::resume_playback,
            commands::playback::stop_playback
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
