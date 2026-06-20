pub mod commands;
pub mod http;
pub mod playback;
pub mod state;

use playback::Player;
use state::AppState;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
            match Player::new() {
                Ok(player) => {
                    app.manage(player);
                }
                Err(e) => tracing::error!("failed to initialize player: {}", e.message),
            }
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
