//! File-picker command for adding a local M3U playlist.
//!
//! The Dioxus UI runs as WASM and cannot import the dialog plugin's JS API, so the
//! native open-file dialog is exposed through this command instead. It runs on the
//! async runtime (off the main thread), so `blocking_pick_file` is safe here.

use cathode_core::error::AppError;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

/// Open the native file picker for a playlist; returns the chosen path, or `None`
/// if the user cancelled.
#[tauri::command]
pub async fn pick_playlist_file(app: AppHandle) -> Result<Option<String>, AppError> {
    let picked = app
        .dialog()
        .file()
        .add_filter("Playlists", &["m3u", "m3u8"])
        .blocking_pick_file();
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}
