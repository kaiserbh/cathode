//! The single place the Dioxus frontend talks to the Tauri shell.
//!
//! Dioxus runs as WASM and cannot import a Tauri plugin's JS API, so every call
//! into the backend goes through `window.__TAURI__.core.invoke` here. Components
//! call the typed wrappers below, never `invoke` directly, so command names and
//! argument shapes live in exactly one file. Results and errors are the shared
//! `cathode_core` types.

use std::collections::HashMap;

use cathode_core::error::AppError;
use cathode_core::model::{Category, LogLevel, NowNext, Programme, Settings, Stream};
use cathode_core::sources::xtream::XtreamCredentials;
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // `catch` turns a rejected command Promise into Err(JsValue) (the serialized
    // AppError) instead of an unrecoverable JS exception.
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// Tauri converts JS camelCase arg keys to Rust snake_case, so `category_id` must
// go over the wire as `categoryId`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CategoriesArgs<'a> {
    creds: &'a XtreamCredentials,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamsArgs<'a> {
    creds: &'a XtreamCredentials,
    category_id: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayArgs<'a> {
    creds: &'a XtreamCredentials,
    stream_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsArgs<'a> {
    settings: &'a Settings,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FavoriteArgs<'a> {
    creds: &'a XtreamCredentials,
    stream: &'a Stream,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveFavoriteArgs<'a> {
    creds: &'a XtreamCredentials,
    stream_id: &'a str,
}

/// Empty args object for commands that take no parameters.
#[derive(Serialize)]
struct NoArgs {}

fn encode_args(args: &impl Serialize) -> Result<JsValue, AppError> {
    serde_wasm_bindgen::to_value(args).map_err(|e| AppError {
        code: "encode".to_string(),
        message: e.to_string(),
    })
}

fn decode_err(err: JsValue) -> AppError {
    serde_wasm_bindgen::from_value::<AppError>(err).unwrap_or_else(|_| AppError {
        code: "unknown".to_string(),
        message: "the command failed".to_string(),
    })
}

/// Invoke a command, encoding args and decoding either the result or the
/// structured `AppError` from a rejection.
async fn call<T: DeserializeOwned>(cmd: &str, args: &impl Serialize) -> Result<T, AppError> {
    match invoke(cmd, encode_args(args)?).await {
        Ok(value) => serde_wasm_bindgen::from_value(value).map_err(|e| AppError {
            code: "decode".to_string(),
            message: e.to_string(),
        }),
        Err(err) => Err(decode_err(err)),
    }
}

/// Invoke a command that returns no data; only success or a structured error.
async fn call_unit(cmd: &str, args: &impl Serialize) -> Result<(), AppError> {
    match invoke(cmd, encode_args(args)?).await {
        Ok(_) => Ok(()),
        Err(err) => Err(decode_err(err)),
    }
}

/// All saved Xtream accounts, most-recently-used first.
pub async fn saved_sources() -> Result<Vec<XtreamCredentials>, AppError> {
    call("saved_sources", &NoArgs {}).await
}

/// Forget a saved account and drop its cached catalog.
pub async fn forget_source(creds: &XtreamCredentials) -> Result<(), AppError> {
    call_unit("forget_source", &CategoriesArgs { creds }).await
}

/// Cached categories for an account (empty if nothing cached yet).
pub async fn cached_categories(creds: &XtreamCredentials) -> Result<Vec<Category>, AppError> {
    call("cached_categories", &CategoriesArgs { creds }).await
}

/// Cached streams for an account + category (empty if nothing cached yet).
pub async fn cached_streams(
    creds: &XtreamCredentials,
    category_id: &str,
) -> Result<Vec<Stream>, AppError> {
    call(
        "cached_streams",
        &StreamsArgs {
            creds,
            category_id: Some(category_id),
        },
    )
    .await
}

/// List the live categories for an Xtream account.
pub async fn list_categories(creds: &XtreamCredentials) -> Result<Vec<Category>, AppError> {
    call("list_categories", &CategoriesArgs { creds }).await
}

/// List the live streams for an Xtream account, optionally scoped to a category.
pub async fn list_streams(
    creds: &XtreamCredentials,
    category_id: Option<&str>,
) -> Result<Vec<Stream>, AppError> {
    call("list_streams", &StreamsArgs { creds, category_id }).await
}

/// Start playing a live stream (by its provider id) in the embedded mpv surface.
pub async fn play_stream(creds: &XtreamCredentials, stream_id: &str) -> Result<(), AppError> {
    call_unit("play_stream", &PlayArgs { creds, stream_id }).await
}

pub async fn pause() -> Result<(), AppError> {
    call_unit("pause_playback", &NoArgs {}).await
}

pub async fn resume() -> Result<(), AppError> {
    call_unit("resume_playback", &NoArgs {}).await
}

pub async fn stop() -> Result<(), AppError> {
    call_unit("stop_playback", &NoArgs {}).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VolumeArgs {
    volume: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MuteArgs {
    muted: bool,
}

/// Set playback volume (0–100).
pub async fn set_volume(volume: u8) -> Result<(), AppError> {
    call_unit("set_volume", &VolumeArgs { volume }).await
}

/// Mute or unmute playback.
pub async fn set_mute(muted: bool) -> Result<(), AppError> {
    call_unit("set_mute", &MuteArgs { muted }).await
}

/// Toggle the window's fullscreen; returns the new state.
pub async fn toggle_fullscreen() -> Result<bool, AppError> {
    call("toggle_fullscreen", &NoArgs {}).await
}

/// Current feature settings (favorites/history toggles).
pub async fn get_settings() -> Result<Settings, AppError> {
    call("get_settings", &NoArgs {}).await
}

/// Persist feature settings.
pub async fn set_settings(settings: &Settings) -> Result<(), AppError> {
    call_unit("set_settings", &SettingsArgs { settings }).await
}

/// An account's favorites, most-recently-added first.
pub async fn list_favorites(creds: &XtreamCredentials) -> Result<Vec<Stream>, AppError> {
    call("list_favorites", &CategoriesArgs { creds }).await
}

/// Mark a stream as a favorite of an account.
pub async fn add_favorite(creds: &XtreamCredentials, stream: &Stream) -> Result<(), AppError> {
    call_unit("add_favorite", &FavoriteArgs { creds, stream }).await
}

/// Remove a favorite by its stable stream id.
pub async fn remove_favorite(creds: &XtreamCredentials, stream_id: &str) -> Result<(), AppError> {
    call_unit("remove_favorite", &RemoveFavoriteArgs { creds, stream_id }).await
}

/// An account's watch history, most-recently-watched first.
pub async fn list_history(creds: &XtreamCredentials) -> Result<Vec<Stream>, AppError> {
    call("list_history", &CategoriesArgs { creds }).await
}

/// Record that a stream was watched (the caller gates this on settings/incognito).
pub async fn record_watch(creds: &XtreamCredentials, stream: &Stream) -> Result<(), AppError> {
    call_unit("record_watch", &FavoriteArgs { creds, stream }).await
}

/// Erase all watch history.
pub async fn clear_history() -> Result<(), AppError> {
    call_unit("clear_history", &NoArgs {}).await
}

/// Now/next per channel (keyed by `epg_channel_id`) for an account's XMLTV guide.
pub async fn epg_now_next(creds: &XtreamCredentials) -> Result<HashMap<String, NowNext>, AppError> {
    call("epg_now_next", &CategoriesArgs { creds }).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgrammesArgs<'a> {
    creds: &'a XtreamCredentials,
    from: i64,
    to: i64,
}

/// Programmes overlapping `[from, to]` per channel, for the timeline guide.
pub async fn epg_programmes(
    creds: &XtreamCredentials,
    from: i64,
    to: i64,
) -> Result<HashMap<String, Vec<Programme>>, AppError> {
    call("epg_programmes", &ProgrammesArgs { creds, from, to }).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogLevelArgs {
    level: LogLevel,
}

/// The captured debug-log lines, oldest first.
pub async fn get_logs() -> Result<Vec<String>, AppError> {
    call("get_logs", &NoArgs {}).await
}

/// Drop all captured log lines.
pub async fn clear_logs() -> Result<(), AppError> {
    call_unit("clear_logs", &NoArgs {}).await
}

/// Set the live capture level (`Off` disables capture).
pub async fn set_log_level(level: LogLevel) -> Result<(), AppError> {
    call_unit("set_log_level", &LogLevelArgs { level }).await
}
