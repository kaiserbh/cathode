//! Source-related commands: list categories and streams for an Xtream account.
//!
//! The `#[tauri::command]` entry points stay thin: they wrap the testable
//! `*_impl` functions (generic over `Transport`) in a tracing span and map
//! `CoreError` to the serializable `AppError`. The real orchestration lives in
//! `cathode_core`.

use cathode_core::error::CoreError;
use cathode_core::model::{Category, Stream};
use cathode_core::sources::xtream::{
    fetch_live_categories, fetch_live_streams, XtreamCredentials, XtreamSource,
};
use cathode_core::transport::Transport;
use tauri::State;
use tracing::{info_span, Instrument};

use crate::error::AppError;
use crate::state::AppState;

/// Fetch live categories for the given credentials. Generic for testability.
pub async fn list_categories_impl<T: Transport>(
    transport: &T,
    creds: &XtreamCredentials,
) -> Result<Vec<Category>, CoreError> {
    let source = XtreamSource::from_credentials(creds);
    fetch_live_categories(&source, transport).await
}

/// Fetch live streams for the given credentials, optionally scoped to a category.
pub async fn list_streams_impl<T: Transport>(
    transport: &T,
    creds: &XtreamCredentials,
    category_id: Option<&str>,
) -> Result<Vec<Stream>, CoreError> {
    let source = XtreamSource::from_credentials(creds);
    fetch_live_streams(&source, transport, category_id).await
}

#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
    creds: XtreamCredentials,
) -> Result<Vec<Category>, AppError> {
    let span = info_span!("list_categories", base_url = %creds.base_url);
    async move {
        list_categories_impl(&state.transport, &creds)
            .await
            .map_err(AppError::from)
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn list_streams(
    state: State<'_, AppState>,
    creds: XtreamCredentials,
    category_id: Option<String>,
) -> Result<Vec<Stream>, AppError> {
    let span = info_span!("list_streams", base_url = %creds.base_url, category = ?category_id);
    async move {
        list_streams_impl(&state.transport, &creds, category_id.as_deref())
            .await
            .map_err(AppError::from)
    }
    .instrument(span)
    .await
}
