//! The single place the Dioxus frontend talks to the Tauri shell.
//!
//! Dioxus runs as WASM and cannot import a Tauri plugin's JS API, so every call
//! into the backend goes through `window.__TAURI__.core.invoke` here. Components
//! call the typed wrappers below, never `invoke` directly, so command names and
//! argument shapes live in exactly one file. Results and errors are the shared
//! `cathode_core` types.

use cathode_core::error::AppError;
use cathode_core::model::{Category, Stream};
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
