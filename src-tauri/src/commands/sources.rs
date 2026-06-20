//! Source-related commands: list categories and streams for an Xtream account.
//!
//! The `#[tauri::command]` entry points stay thin: they wrap the testable
//! `*_impl` functions (generic over `Transport`) in a tracing span and map
//! `CoreError` to the serializable `AppError`. The real orchestration lives in
//! `cathode_core`.

use cathode_core::catalog::Catalog;
use cathode_core::error::{AppError, CoreError};
use cathode_core::model::{Category, Stream};
use cathode_core::sources::xtream::{
    credentials_from_record, fetch_live_categories, fetch_live_streams, source_record,
    XtreamCredentials, XtreamSource,
};
use cathode_core::transport::Transport;
use tauri::State;
use tokio::task;
use tracing::{info_span, Instrument};

use crate::state::{AppState, CatalogState};

/// The stable catalog key for an account.
fn source_id(creds: &XtreamCredentials) -> String {
    XtreamSource::from_credentials(creds).source_id()
}

fn join_err(e: task::JoinError) -> AppError {
    AppError {
        code: "storage".to_string(),
        message: format!("background task failed: {e}"),
    }
}

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
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
) -> Result<Vec<Category>, AppError> {
    let span = info_span!("list_categories", base_url = %creds.base_url);
    let categories = async {
        list_categories_impl(&state.transport, &creds)
            .await
            .map_err(AppError::from)
    }
    .instrument(span)
    .await?;

    // Write-through to the cache (and remember the source). Failures here are
    // logged, never surfaced: a working fetch must not fail on a cache hiccup.
    if let Some(cat) = catalog.0.clone() {
        let creds = creds.clone();
        let to_cache = categories.clone();
        match task::spawn_blocking(move || -> Result<(), CoreError> {
            cat.upsert_source(&source_record(&creds)?)?;
            cat.replace_categories(&source_id(&creds), &to_cache)?;
            Ok(())
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("cache categories failed: {e}"),
            Err(e) => tracing::warn!("cache categories task failed: {e}"),
        }
    }
    Ok(categories)
}

#[tauri::command]
pub async fn list_streams(
    state: State<'_, AppState>,
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
    category_id: Option<String>,
) -> Result<Vec<Stream>, AppError> {
    let span = info_span!("list_streams", base_url = %creds.base_url, category = ?category_id);
    let streams = async {
        list_streams_impl(&state.transport, &creds, category_id.as_deref())
            .await
            .map_err(AppError::from)
    }
    .instrument(span)
    .await?;

    // Cache only when scoped to a concrete category (the bucket key); the "all
    // streams" shape is not something the UI requests.
    if let (Some(cat), Some(cid)) = (catalog.0.clone(), category_id.clone()) {
        let sid = source_id(&creds);
        let to_cache = streams.clone();
        match task::spawn_blocking(move || cat.replace_streams(&sid, &cid, &to_cache)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("cache streams failed: {e}"),
            Err(e) => tracing::warn!("cache streams task failed: {e}"),
        }
    }
    Ok(streams)
}

/// Cached categories for an account (empty if nothing cached / no catalog).
#[tauri::command]
pub async fn cached_categories(
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
) -> Result<Vec<Category>, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Vec::new());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.categories(&sid))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// Cached streams for an account + category (empty if nothing cached / no catalog).
#[tauri::command]
pub async fn cached_streams(
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
    category_id: String,
) -> Result<Vec<Stream>, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Vec::new());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.streams(&sid, &category_id))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// All saved Xtream accounts, most-recently-used first.
#[tauri::command]
pub async fn saved_sources(
    catalog: State<'_, CatalogState>,
) -> Result<Vec<XtreamCredentials>, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Vec::new());
    };
    let records = task::spawn_blocking(move || cat.sources())
        .await
        .map_err(join_err)?
        .map_err(AppError::from)?;
    Ok(records
        .iter()
        .filter_map(|record| credentials_from_record(record).ok())
        .collect())
}

/// Forget a saved account and drop its cached catalog.
#[tauri::command]
pub async fn forget_source(
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
) -> Result<(), AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.delete_source(&sid))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}
