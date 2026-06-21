pub mod catalog_sqlite;
pub mod commands;
pub mod http;
pub mod playback;
pub mod state;

use std::sync::Arc;

use catalog_sqlite::SqliteCatalog;
use cathode_core::error::AppError;
use playback::Player;
use state::{AppState, CatalogState};
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Open the local catalog under the app data directory, creating the directory if
/// needed. Returns the opened catalog; the caller logs and continues on failure.
fn open_catalog(app: &tauri::App) -> Result<SqliteCatalog, AppError> {
    let dir = app.path().app_data_dir().map_err(|e| AppError {
        code: "storage".to_string(),
        message: format!("resolve app data dir: {e}"),
    })?;
    std::fs::create_dir_all(&dir).map_err(|e| AppError {
        code: "storage".to_string(),
        message: format!("create app data dir: {e}"),
    })?;
    SqliteCatalog::open(&dir.join("catalog.sqlite")).map_err(AppError::from)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .setup(|app| {
            // Create the native playback backend. If mpv is unavailable the rest
            // of the app still runs; playback commands will error until fixed.
            match playback::create_mpv() {
                Ok(mpv) => {
                    app.manage(Player::new(mpv));

                    // macOS: attach the native GL surface that video composites
                    // into, behind the transparent webview, and wire mpv's render
                    // context to it.
                    #[cfg(target_os = "macos")]
                    match app.get_webview_window("main") {
                        Some(window) => {
                            if let Err(e) = playback::macos::attach(&window, mpv) {
                                tracing::error!("failed to attach video surface: {}", e.message);
                            }
                        }
                        None => tracing::error!("no main window to attach video surface"),
                    }
                }
                Err(e) => tracing::error!("failed to initialize player: {}", e.message),
            }

            // Open the local catalog (saved sources + cached categories/streams).
            // On failure the app still runs with caching disabled.
            let catalog = match open_catalog(app) {
                Ok(catalog) => Some(Arc::new(catalog)),
                Err(e) => {
                    tracing::error!("failed to open catalog: {}", e.message);
                    None
                }
            };
            app.manage(CatalogState(catalog));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::sources::list_categories,
            commands::sources::list_streams,
            commands::sources::cached_categories,
            commands::sources::cached_streams,
            commands::sources::saved_sources,
            commands::sources::forget_source,
            commands::library::get_settings,
            commands::library::set_settings,
            commands::library::list_favorites,
            commands::library::add_favorite,
            commands::library::remove_favorite,
            commands::library::list_history,
            commands::library::record_watch,
            commands::library::clear_history,
            commands::epg::epg_now_next,
            commands::epg::epg_programmes,
            commands::playback::play_stream,
            commands::playback::pause_playback,
            commands::playback::resume_playback,
            commands::playback::stop_playback
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
