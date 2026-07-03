//! Source-related commands: list categories and streams for a saved source.
//!
//! A source is either an Xtream account or a plain M3U/M3U8 playlist, carried as a
//! [`SourceCredentials`]. The `#[tauri::command]` entry points stay thin: they
//! dispatch on the source kind, wrap the testable `*_impl` functions (generic over
//! `Transport`) in a tracing span, and map `CoreError` to the serializable
//! `AppError`. The real orchestration lives in `cathode_core`.
//!
//! Xtream exposes a JSON API queried per content kind and category; an M3U playlist
//! is a single document covering the whole catalog, so it is downloaded+parsed once
//! per session (cached in `AppState.playlists`) and then sliced into categories and
//! streams. Every M3U entry is a Live channel, so non-Live kinds return empty
//! without fetching.

use cathode_core::catalog::Catalog;
use cathode_core::error::{AppError, CoreError};
use cathode_core::model::{Category, SeriesInfo, Stream, StreamKind};
use cathode_core::sources::m3u::{
    categories_from_streams, epg_urls_from_header, parse_playlist, M3uCredentials, M3uSource,
};
use cathode_core::sources::xtream::{
    fetch_categories, fetch_series_info, fetch_streams, XtreamCredentials, XtreamSource,
};
use cathode_core::sources::{credentials_from_record, SourceCredentials};
use cathode_core::transport::Transport;
use std::io::Read;
use tauri::State;
use tokio::task;
use tracing::{info_span, Instrument};

use crate::http::ReqwestTransport;
use crate::state::{AppState, CatalogState};

/// The stable catalog key for a source (account or playlist).
pub(crate) fn source_id(creds: &SourceCredentials) -> String {
    creds.source_id()
}

pub(crate) fn join_err(e: task::JoinError) -> AppError {
    AppError {
        code: "storage".to_string(),
        message: format!("background task failed: {e}"),
    }
}

/// Fetch the categories of a content kind for an Xtream account. Generic for testability.
pub async fn list_categories_impl<T: Transport>(
    transport: &T,
    creds: &XtreamCredentials,
    kind: StreamKind,
) -> Result<Vec<Category>, CoreError> {
    let source = XtreamSource::from_credentials(creds);
    fetch_categories(&source, transport, kind).await
}

/// Fetch streams of a content kind for an Xtream account, optionally scoped to a
/// category. Generic for testability.
pub async fn list_streams_impl<T: Transport>(
    transport: &T,
    creds: &XtreamCredentials,
    kind: StreamKind,
    category_id: Option<&str>,
) -> Result<Vec<Stream>, CoreError> {
    let source = XtreamSource::from_credentials(creds);
    fetch_streams(&source, transport, kind, category_id).await
}

/// Whether a playlist location is an HTTP(S) URL (vs. a local file path).
fn is_http(location: &str) -> bool {
    let lower = location.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Load a playlist's raw text: an `http(s)` URL goes through the transport; anything
/// else is read from disk (a `file://` prefix is stripped). File reading is
/// native-only, which is why this lives in the shell rather than `core`.
pub async fn load_playlist_text<T: Transport>(
    transport: &T,
    location: &str,
) -> Result<String, CoreError> {
    if is_http(location) {
        transport.get_text(location).await
    } else {
        let path = location.strip_prefix("file://").unwrap_or(location);
        std::fs::read_to_string(path)
            .map_err(|e| CoreError::storage("read playlist file", e.to_string()))
    }
}

/// Load and parse an M3U playlist into normalized live streams. Generic for
/// testability (the integration test serves the playlist over HTTP).
pub async fn m3u_streams_impl<T: Transport>(
    transport: &T,
    creds: &M3uCredentials,
) -> Result<Vec<Stream>, CoreError> {
    let source = M3uSource::from_credentials(creds);
    let text = load_playlist_text(transport, source.playlist_url()).await?;
    parse_playlist(&text, &source.source_id())
}

/// The parsed playlist for an M3U source, downloaded+parsed once per session and
/// cached in `AppState.playlists` (keyed by `source_id`). Shared with the EPG path,
/// which needs the playlist's channels to filter its guide.
pub(crate) async fn ensure_playlist(
    state: &AppState,
    creds: &M3uCredentials,
) -> Result<Vec<Stream>, CoreError> {
    let sid = M3uSource::from_credentials(creds).source_id();
    if let Some(streams) = state.playlists.lock().unwrap().get(&sid).cloned() {
        return Ok(streams);
    }
    let streams = m3u_streams_impl(&state.transport, creds).await?;
    state.playlists.lock().unwrap().insert(sid, streams.clone());
    Ok(streams)
}

/// Load an XMLTV guide's text from an `http(s)` URL or local file, transparently
/// gunzipping a gzipped body (`*.xml.gz`). Guides are commonly served gzipped, so
/// the text path (which assumes UTF-8) can't be used directly.
pub async fn load_guide_text(
    transport: &ReqwestTransport,
    location: &str,
) -> Result<String, CoreError> {
    let bytes = if is_http(location) {
        transport.get_bytes(location).await?
    } else {
        let path = location.strip_prefix("file://").unwrap_or(location);
        std::fs::read(path).map_err(|e| CoreError::storage("read guide file", e.to_string()))?
    };
    Ok(decode_guide_bytes(&bytes))
}

/// Decode guide bytes to text, gunzipping first when they start with the gzip magic
/// (`1f 8b`). A decode failure falls back to a lossy read of the raw bytes.
fn decode_guide_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        if flate2::read::GzDecoder::new(bytes)
            .read_to_end(&mut out)
            .is_ok()
        {
            return String::from_utf8_lossy(&out).into_owned();
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// Categories for an M3U source's content kind. Only Live is populated; other kinds
/// return empty *without* loading the playlist.
async fn m3u_categories(
    state: &AppState,
    creds: &M3uCredentials,
    kind: StreamKind,
) -> Result<Vec<Category>, CoreError> {
    if kind != StreamKind::Live {
        return Ok(Vec::new());
    }
    let streams = ensure_playlist(state, creds).await?;
    Ok(categories_from_streams(&streams))
}

/// Streams for an M3U source's content kind, optionally scoped to a category. Only
/// Live is populated.
async fn m3u_streams(
    state: &AppState,
    creds: &M3uCredentials,
    kind: StreamKind,
    category_id: Option<&str>,
) -> Result<Vec<Stream>, CoreError> {
    if kind != StreamKind::Live {
        return Ok(Vec::new());
    }
    let streams = ensure_playlist(state, creds).await?;
    Ok(match category_id {
        Some(cid) => streams
            .into_iter()
            .filter(|s| s.category_id.as_ref().map(|c| c.0.as_str()) == Some(cid))
            .collect(),
        None => streams,
    })
}

#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
    kind: StreamKind,
) -> Result<Vec<Category>, AppError> {
    let span = info_span!("list_categories", source = %creds.source_id(), ?kind);
    let categories = async {
        let result = match &creds {
            SourceCredentials::Xtream(c) => list_categories_impl(&state.transport, c, kind).await,
            SourceCredentials::M3u(c) => m3u_categories(&state, c, kind).await,
        };
        match result {
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
            cat.upsert_source(&creds.to_record()?)?;
            cat.replace_categories(&creds.source_id(), kind, &to_cache)?;
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
    creds: SourceCredentials,
    kind: StreamKind,
    category_id: Option<String>,
) -> Result<Vec<Stream>, AppError> {
    let span =
        info_span!("list_streams", source = %creds.source_id(), ?kind, category = ?category_id);
    let streams = async {
        let result = match &creds {
            SourceCredentials::Xtream(c) => {
                list_streams_impl(&state.transport, c, kind, category_id.as_deref()).await
            }
            SourceCredentials::M3u(c) => m3u_streams(&state, c, kind, category_id.as_deref()).await,
        };
        match result {
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

/// Cached categories for a source + kind (empty if nothing cached / no catalog).
#[tauri::command]
pub async fn cached_categories(
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
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

/// Cached streams for a source + kind + category (empty if nothing cached / no catalog).
#[tauri::command]
pub async fn cached_streams(
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
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

/// Search the source's cached library (all kinds and categories) by name.
#[tauri::command]
pub async fn search_streams(
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
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

/// Fetch the seasons and episodes of one series (Xtream only; not cached). An M3U
/// source has no series, so this yields an empty result.
#[tauri::command]
pub async fn get_series_info(
    state: State<'_, AppState>,
    creds: SourceCredentials,
    series_id: String,
) -> Result<SeriesInfo, AppError> {
    let SourceCredentials::Xtream(creds) = &creds else {
        return Ok(SeriesInfo {
            seasons: Vec::new(),
        });
    };
    let source = XtreamSource::from_credentials(creds);
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

/// All saved sources (Xtream accounts and M3U playlists), most-recently-used first.
#[tauri::command]
pub async fn saved_sources(
    catalog: State<'_, CatalogState>,
) -> Result<Vec<SourceCredentials>, AppError> {
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

/// Forget a saved source and drop its cached catalog (and its session playlist + EPG
/// caches, so re-adding it reloads fresh).
#[tauri::command]
pub async fn forget_source(
    state: State<'_, AppState>,
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
) -> Result<(), AppError> {
    let sid = source_id(&creds);
    state.playlists.lock().unwrap().remove(&sid);
    state.epg.lock().unwrap().remove(&sid);
    let Some(cat) = catalog.0.clone() else {
        return Ok(());
    };
    task::spawn_blocking(move || cat.delete_source(&sid))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)
}

/// Detect the EPG (XMLTV) URLs a playlist declares in its `#EXTM3U` header, for
/// pre-filling the M3U form's EPG field. Loads the playlist (URL or local file) and
/// reads its `x-tvg-url`/`url-tvg` attribute.
#[tauri::command]
pub async fn detect_playlist_epg(
    state: State<'_, AppState>,
    location: String,
) -> Result<Vec<String>, AppError> {
    let text = load_playlist_text(&state.transport, &location)
        .await
        .map_err(AppError::from)?;
    Ok(epg_urls_from_header(&text))
}
