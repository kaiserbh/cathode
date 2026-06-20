//! Playback commands.
//!
//! These wrap the `tauri-plugin-libmpv` `MpvExt` API so the UI controls mpv only
//! through our own commands (AGENTS.md). Because the UI never sees the plugin, if
//! the plugin can't composite on a platform we can swap these bodies for a custom
//! libmpv integration without touching the frontend or bindings.

use cathode_core::error::AppError;
use cathode_core::sources::xtream::{XtreamCredentials, XtreamSource};
use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_libmpv::MpvExt;
use tracing::info_span;

/// The single player window. One window today; revisit if we add more.
const WINDOW: &str = "main";

/// Map a plugin error into our serializable AppError.
fn playback_err(context: &str, e: impl std::fmt::Display) -> AppError {
    AppError {
        code: "playback".to_string(),
        message: format!("{context}: {e}"),
    }
}

/// Load and play a live stream by its provider id (Xtream `stream_id`).
#[tauri::command]
pub fn play_stream(
    app: AppHandle,
    creds: XtreamCredentials,
    stream_id: String,
) -> Result<(), AppError> {
    let _span = info_span!("play_stream", stream_id = %stream_id).entered();

    let url = XtreamSource::from_credentials(&creds).live_stream_url(&stream_id, "ts");
    let mpv = app.mpv();
    mpv.command("loadfile", &vec![json!(url)], WINDOW)
        .map_err(|e| playback_err("loadfile", e))?;
    mpv.set_property("pause", &json!(false), WINDOW)
        .map_err(|e| playback_err("resume", e))?;
    Ok(())
}

#[tauri::command]
pub fn pause_playback(app: AppHandle) -> Result<(), AppError> {
    app.mpv()
        .set_property("pause", &json!(true), WINDOW)
        .map_err(|e| playback_err("pause", e))
}

#[tauri::command]
pub fn resume_playback(app: AppHandle) -> Result<(), AppError> {
    app.mpv()
        .set_property("pause", &json!(false), WINDOW)
        .map_err(|e| playback_err("resume", e))
}

#[tauri::command]
pub fn stop_playback(app: AppHandle) -> Result<(), AppError> {
    let no_args: Vec<Value> = Vec::new();
    app.mpv()
        .command("stop", &no_args, WINDOW)
        .map_err(|e| playback_err("stop", e))
}
