//! Library commands: settings, favorites, and watch history.
//!
//! Thin wrappers over the catalog, all run on a blocking thread. Favorites and
//! history are scoped to the account identified by the passed credentials; the
//! settings are global and stored as one JSON blob under the `settings` key.

use cathode_core::catalog::Catalog;
use cathode_core::error::AppError;
use cathode_core::model::{Settings, Stream};
use cathode_core::sources::SourceCredentials;
use tauri::State;
use tokio::task;

use crate::commands::sources::{join_err, source_id};
use crate::state::CatalogState;

/// The catalog key the serialized [`Settings`] blob lives under.
const SETTINGS_KEY: &str = "settings";

/// Current feature settings (defaults if unset or unreadable).
#[tauri::command]
pub async fn get_settings(catalog: State<'_, CatalogState>) -> Result<Settings, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Settings::default());
    };
    let raw = task::spawn_blocking(move || cat.get_setting(SETTINGS_KEY))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)?;
    // A missing or corrupt blob falls back to defaults rather than erroring.
    Ok(raw
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default())
}

/// Persist feature settings.
#[tauri::command]
pub async fn set_settings(
    catalog: State<'_, CatalogState>,
    settings: Settings,
) -> Result<(), AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(());
    };
    let json = serde_json::to_string(&settings).map_err(|e| AppError {
        code: "storage".to_string(),
        message: format!("serialize settings: {e}"),
    })?;
    task::spawn_blocking(move || cat.set_setting(SETTINGS_KEY, &json))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// An account's favorites, most-recently-added first.
#[tauri::command]
pub async fn list_favorites(
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
) -> Result<Vec<Stream>, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Vec::new());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.favorites(&sid))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn add_favorite(
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
    stream: Stream,
) -> Result<(), AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.add_favorite(&sid, &stream))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn remove_favorite(
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
    stream_id: String,
) -> Result<(), AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.remove_favorite(&sid, &stream_id))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// An account's watch history, most-recently-watched first.
#[tauri::command]
pub async fn list_history(
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
) -> Result<Vec<Stream>, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Vec::new());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.history(&sid))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// Record a play in watch history. The frontend only calls this when recording is
/// enabled and not in incognito, so the command itself records unconditionally.
#[tauri::command]
pub async fn record_watch(
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
    stream: Stream,
) -> Result<(), AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.record_watch(&sid, &stream))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// Erase all watch history.
#[tauri::command]
pub async fn clear_history(catalog: State<'_, CatalogState>) -> Result<(), AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(());
    };
    task::spawn_blocking(move || cat.clear_history())
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}
