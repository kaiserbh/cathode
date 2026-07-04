//! Playback commands.
//!
//! Thin wrappers over our native [`Player`] backend so the UI controls mpv only
//! through these commands. The backend can change (plugin → libmpv2 render API,
//! per platform) without the frontend or these signatures changing.

use cathode_core::error::AppError;
use cathode_core::model::{Stream, StreamKind};
use cathode_core::sources::xtream::XtreamSource;
use cathode_core::sources::SourceCredentials;
use tauri::State;
use tracing::info_span;

use crate::playback::Player;

/// Load and play a stream. For Xtream the playable URL is built from the content
/// kind (Live plays `.ts`; VOD/Series use the stream's `container_extension`,
/// defaulting to `mp4`). For an M3U source the entry's URL is already the playable
/// URL, kept verbatim in `provider_id`.
#[tauri::command]
pub fn play_stream(
    player: State<'_, Player>,
    creds: SourceCredentials,
    stream: Stream,
) -> Result<(), AppError> {
    let _span =
        info_span!("play_stream", provider_id = %stream.provider_id, kind = ?stream.kind).entered();
    tracing::info!(provider_id = %stream.provider_id, kind = ?stream.kind, "playing stream");
    let url = match &creds {
        SourceCredentials::Xtream(creds) => {
            let source = XtreamSource::from_credentials(creds);
            let id = &stream.provider_id;
            let ext = stream.container_extension.as_deref();
            match stream.kind {
                StreamKind::Live => source.live_stream_url(id, "ts"),
                StreamKind::Vod => source.vod_stream_url(id, ext.unwrap_or("mp4")),
                StreamKind::Series => source.series_episode_url(id, ext.unwrap_or("mp4")),
            }
        }
        SourceCredentials::M3u(_) => stream.provider_id.clone(),
    };
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

#[tauri::command]
pub fn set_volume(player: State<'_, Player>, volume: u8) -> Result<(), AppError> {
    player.set_volume(cathode_core::model::settings::volume_to_mpv_gain(volume))
}

#[tauri::command]
pub fn set_mute(player: State<'_, Player>, muted: bool) -> Result<(), AppError> {
    player.set_mute(muted)
}

/// Toggle the main window's fullscreen, returning the new state.
#[tauri::command]
pub fn toggle_fullscreen(window: tauri::WebviewWindow) -> Result<bool, AppError> {
    let fullscreen = !window.is_fullscreen().map_err(|e| AppError {
        code: "playback".to_string(),
        message: format!("read fullscreen: {e}"),
    })?;
    window.set_fullscreen(fullscreen).map_err(|e| AppError {
        code: "playback".to_string(),
        message: format!("set fullscreen: {e}"),
    })?;
    Ok(fullscreen)
}

/// Set the main window's fullscreen state explicitly. Used to force fullscreen off
/// when playback stops, so the user is never stranded in a chrome-less fullscreen
/// window with no way to exit (native fullscreen hides the title bar on Windows).
#[tauri::command]
pub fn set_fullscreen(window: tauri::WebviewWindow, fullscreen: bool) -> Result<(), AppError> {
    window.set_fullscreen(fullscreen).map_err(|e| AppError {
        code: "playback".to_string(),
        message: format!("set fullscreen: {e}"),
    })
}
