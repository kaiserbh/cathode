//! Source-related commands: list categories and streams for an Xtream account.
//!
//! The `#[tauri::command]` entry points stay thin: they wrap the testable
//! `*_impl` functions (generic over `Transport`) in a tracing span and map
//! `CoreError` to the serializable `AppError`. The real orchestration lives in
//! `cathode_core`.

use cathode_core::catalog::Catalog;
use cathode_core::error::{AppError, CoreError};
use cathode_core::model::{Category, SeriesInfo, Stream, StreamKind};
use cathode_core::sources::xtream::{
    credentials_from_record, fetch_categories, fetch_series_info, fetch_streams, source_record,
    XtreamCredentials, XtreamSource,
};
use cathode_core::transport::Transport;
use tauri::State;
use tokio::task;
use tracing::{info_span, Instrument};

use crate::state::{AppState, CatalogState};

/// The stable catalog key for an account.
pub(crate) fn source_id(creds: &XtreamCredentials) -> String {
    XtreamSource::from_credentials(creds).source_id()
}

pub(crate) fn join_err(e: task::JoinError) -> AppError {
    AppError {
        code: "storage".to_string(),
        message: format!("background task failed: {e}"),
    }
}

/// Fetch live categories for the given credentials. Generic for testability.
pub async fn list_categories_impl<T: Transport>(
    transport: &T,
    creds: &XtreamCredentials,
    kind: StreamKind,
) -> Result<Vec<Category>, CoreError> {
    let source = XtreamSource::from_credentials(creds);
    fetch_categories(&source, transport, kind).await
}

/// Fetch streams of a content kind for the given credentials, optionally scoped to a
/// category.
pub async fn list_streams_impl<T: Transport>(
    transport: &T,
    creds: &XtreamCredentials,
    kind: StreamKind,
    category_id: Option<&str>,
) -> Result<Vec<Stream>, CoreError> {
    let source = XtreamSource::from_credentials(creds);
    fetch_streams(&source, transport, kind, category_id).await
}

#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
    kind: StreamKind,
) -> Result<Vec<Category>, AppError> {
    let span = info_span!("list_categories", base_url = %creds.base_url, ?kind);
    let categories = async {
        match list_categories_impl(&state.transport, &creds, kind).await {
            Ok(v) => {
                tracing::info!(count = v.len(), ?kind, "fetched categories");
                Ok(v)
            }
            Err(e) => {
                tracing::warn!("fetch categories failed: {e}");
                Err(AppError::from(e))
            }
        }
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
            cat.replace_categories(&source_id(&creds), kind, &to_cache)?;
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
    kind: StreamKind,
    category_id: Option<String>,
) -> Result<Vec<Stream>, AppError> {
    let span =
        info_span!("list_streams", base_url = %creds.base_url, ?kind, category = ?category_id);
    let streams = async {
        match list_streams_impl(&state.transport, &creds, kind, category_id.as_deref()).await {
            Ok(v) => {
                tracing::info!(count = v.len(), ?kind, "fetched streams");
                Ok(v)
            }
            Err(e) => {
                tracing::warn!("fetch streams failed: {e}");
                Err(AppError::from(e))
            }
        }
    }
    .instrument(span)
    .await?;

    // Cache only when scoped to a concrete category (the bucket key); the "all
    // streams" shape is not something the UI requests.
    if let (Some(cat), Some(cid)) = (catalog.0.clone(), category_id.clone()) {
        let sid = source_id(&creds);
        let to_cache = streams.clone();
        match task::spawn_blocking(move || cat.replace_streams(&sid, kind, &cid, &to_cache)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("cache streams failed: {e}"),
            Err(e) => tracing::warn!("cache streams task failed: {e}"),
        }
    }
    Ok(streams)
}

/// Cached categories for an account + kind (empty if nothing cached / no catalog).
#[tauri::command]
pub async fn cached_categories(
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
    kind: StreamKind,
) -> Result<Vec<Category>, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Vec::new());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.categories(&sid, kind))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// Cached streams for an account + kind + category (empty if nothing cached / no catalog).
#[tauri::command]
pub async fn cached_streams(
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
    kind: StreamKind,
    category_id: String,
) -> Result<Vec<Stream>, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Vec::new());
    };
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.streams(&sid, kind, &category_id))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// Search the account's cached library (all kinds and categories) by name.
#[tauri::command]
pub async fn search_streams(
    catalog: State<'_, CatalogState>,
    creds: XtreamCredentials,
    query: String,
) -> Result<Vec<Stream>, AppError> {
    let Some(cat) = catalog.0.clone() else {
        return Ok(Vec::new());
    };
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let sid = source_id(&creds);
    task::spawn_blocking(move || cat.search_streams(&sid, query.trim()))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// Fetch the seasons and episodes of one series (not cached).
#[tauri::command]
pub async fn get_series_info(
    state: State<'_, AppState>,
    creds: XtreamCredentials,
    series_id: String,
) -> Result<SeriesInfo, AppError> {
    let source = XtreamSource::from_credentials(&creds);
    async {
        match fetch_series_info(&source, &state.transport, &series_id).await {
            Ok(info) => {
                tracing::info!(seasons = info.seasons.len(), "fetched series info");
                Ok(info)
            }
            Err(e) => {
                tracing::warn!("fetch series info failed: {e}");
                Err(AppError::from(e))
            }
        }
    }
    .instrument(info_span!("get_series_info", %series_id))
    .await
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
