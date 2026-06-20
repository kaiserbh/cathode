//! Playback commands.
//!
//! Thin wrappers over our native [`Player`] backend so the UI controls mpv only
//! through these commands. The backend can change (plugin → libmpv2 render API,
//! per platform) without the frontend or these signatures changing.

use cathode_core::error::AppError;
use cathode_core::sources::xtream::{XtreamCredentials, XtreamSource};
use tauri::State;
use tracing::info_span;

use crate::playback::Player;

/// Load and play a live stream by its provider id (Xtream `stream_id`).
#[tauri::command]
pub fn play_stream(
    player: State<'_, Player>,
    creds: XtreamCredentials,
    stream_id: String,
) -> Result<(), AppError> {
    let _span = info_span!("play_stream", stream_id = %stream_id).entered();
    let url = XtreamSource::from_credentials(&creds).live_stream_url(&stream_id, "ts");
    player.load(&url)
}

#[tauri::command]
pub fn pause_playback(player: State<'_, Player>) -> Result<(), AppError> {
    player.set_pause(true)
}

#[tauri::command]
pub fn resume_playback(player: State<'_, Player>) -> Result<(), AppError> {
    player.set_pause(false)
}

#[tauri::command]
pub fn stop_playback(player: State<'_, Player>) -> Result<(), AppError> {
    player.stop()
}
