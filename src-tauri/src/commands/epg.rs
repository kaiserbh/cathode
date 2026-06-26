//! EPG commands: fetch + cache an account's XMLTV guide, return now/next per
//! channel and windowed programmes for the timeline guide.
//!
//! The guide is fetched and parsed once per source per session (held in
//! `AppState.epg`); later calls just recompute against the current time, so the UI
//! can poll cheaply. Both responses are keyed by channel id and additionally by
//! normalized display-name, so the frontend can resolve a channel by name when it
//! has no `epg_channel_id`. EPG is best-effort — a provider without `xmltv.php`
//! surfaces an error the caller can ignore.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use cathode_core::catalog::Catalog;
use cathode_core::epg::{
    filter_to_channels, merge_guides, name_index, normalize_name, now_next, parse_xmltv,
};
use cathode_core::error::{AppError, CoreError};
use cathode_core::model::{NowNext, Programme};
use cathode_core::sources::m3u::M3uCredentials;
use cathode_core::sources::xtream::{XtreamCredentials, XtreamSource};
use cathode_core::sources::SourceCredentials;
use cathode_core::transport::Transport;
use tauri::{AppHandle, Manager, State};
use tokio::task;
use tracing::{info_span, Instrument};

use crate::commands::sources::{ensure_playlist, join_err, load_guide_text, source_id};
use crate::state::{AppState, CatalogState};

/// How far around `now` to read cached programmes when computing now/next from disk.
const NOW_NEXT_WINDOW: i64 = 24 * 3600;

/// Cap on how many XMLTV files we'll fetch for one playlist, so a header listing
/// dozens of guides can't trigger an unbounded download.
const MAX_EPG_FILES: usize = 10;

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Ensure the source's guide is parsed and cached (fetch off the async runtime on
/// a miss). Holds the lock only for the quick presence check and the insert, never
/// across the await. On a successful parse the guide is also written through to the
/// SQLite catalog so a later launch can load it without re-downloading.
async fn ensure_guide(
    state: &AppState,
    catalog: &CatalogState,
    creds: &XtreamCredentials,
    sid: &str,
) -> Result<(), AppError> {
    if state.epg.lock().unwrap().contains_key(sid) {
        return Ok(());
    }
    let url = XtreamSource::from_credentials(creds).xmltv_url();
    let xml = async { state.transport.get_text(&url).await }
        .instrument(info_span!("epg_fetch", source = %sid))
        .await
        .map_err(|e| {
            tracing::warn!("epg fetch failed: {e}");
            AppError::from(e)
        })?;
    let guide = task::spawn_blocking(move || parse_xmltv(&xml))
        .await
        .map_err(join_err)?
        .map_err(|e| {
            tracing::warn!("epg parse failed: {e}");
            AppError::from(e)
        })?;
    tracing::info!(
        programmes = guide.programmes.len(),
        channels = guide.channels.len(),
        "parsed guide"
    );

    // Write-through to the cache (best-effort; a cache hiccup must not fail the fetch).
    if let Some(cat) = catalog.0.clone() {
        let sid = sid.to_string();
        let programmes = guide.programmes.clone();
        match task::spawn_blocking(move || cat.replace_programmes(&sid, &programmes)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("cache programmes failed: {e}"),
            Err(e) => tracing::warn!("cache programmes task failed: {e}"),
        }
    }

    state.epg.lock().unwrap().insert(sid.to_string(), guide);
    Ok(())
}

/// Kick off a background guide refresh after serving from the cache, so the in-memory
/// guide and the SQLite cache become current for the next call. De-duped via
/// `epg_fetching` so the parallel now/next and programmes calls don't both download.
fn spawn_refresh(app: AppHandle, creds: XtreamCredentials, sid: String) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if state.epg.lock().unwrap().contains_key(&sid) {
            return;
        }
        if !state.epg_fetching.lock().unwrap().insert(sid.clone()) {
            return; // a refresh for this source is already in flight
        }
        let catalog = app.state::<CatalogState>();
        let _ = ensure_guide(&state, &catalog, &creds, &sid).await;
        state.epg_fetching.lock().unwrap().remove(&sid);
    });
}

/// Read cached programmes from the catalog (off the async runtime). Returns empty
/// when there is no catalog or nothing cached.
async fn cached_programmes(
    catalog: &CatalogState,
    sid: &str,
    from: i64,
    to: i64,
) -> Vec<Programme> {
    let Some(cat) = catalog.0.clone() else {
        return Vec::new();
    };
    let sid = sid.to_string();
    match task::spawn_blocking(move || cat.programmes(&sid, from, to)).await {
        Ok(Ok(list)) => list,
        Ok(Err(e)) => {
            tracing::warn!("read cached programmes failed: {e}");
            Vec::new()
        }
        Err(e) => {
            tracing::warn!("read cached programmes task failed: {e}");
            Vec::new()
        }
    }
}

/// Group a flat, start-sorted programme list by channel id (used for the cache path,
/// which has no channel display-name index to alias by).
fn group_by_channel(programmes: Vec<Programme>) -> HashMap<String, Vec<Programme>> {
    let mut map: HashMap<String, Vec<Programme>> = HashMap::new();
    for p in programmes {
        map.entry(p.channel_id.clone()).or_default().push(p);
    }
    map
}

/// Build (once per session) the merged, channel-filtered guide for an M3U playlist
/// from the EPG URLs it carries, caching it in `AppState.epg` like the Xtream path.
/// Each source is fetched (gzip-aware), parsed off the async runtime, and trimmed to
/// the playlist's own channels so a huge guide collapses to what's relevant.
async fn ensure_m3u_guide(
    state: &AppState,
    catalog: &CatalogState,
    creds: &M3uCredentials,
    sid: &str,
) -> Result<(), AppError> {
    if state.epg.lock().unwrap().contains_key(sid) {
        return Ok(());
    }

    // The playlist's channels drive filtering: match by tvg-id, then by name.
    let streams = ensure_playlist(state, creds)
        .await
        .map_err(AppError::from)?;
    let wanted_ids: HashSet<String> = streams
        .iter()
        .filter_map(|s| s.epg_channel_id.clone())
        .collect();
    let wanted_names: HashSet<String> = streams.iter().map(|s| normalize_name(&s.name)).collect();

    let mut urls = creds.epg_urls.clone();
    if urls.len() > MAX_EPG_FILES {
        tracing::warn!(
            count = urls.len(),
            cap = MAX_EPG_FILES,
            "capping EPG sources for playlist"
        );
        urls.truncate(MAX_EPG_FILES);
    }

    let mut guides = Vec::new();
    for url in &urls {
        let text = match load_guide_text(&state.transport, url).await {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!("m3u epg fetch failed: {e}");
                continue;
            }
        };
        let ids = wanted_ids.clone();
        let names = wanted_names.clone();
        match task::spawn_blocking(move || {
            let mut guide = parse_xmltv(&text)?;
            filter_to_channels(&mut guide, &ids, &names);
            Ok::<_, CoreError>(guide)
        })
        .await
        {
            Ok(Ok(guide)) => guides.push(guide),
            Ok(Err(e)) => tracing::warn!("m3u epg parse failed: {e}"),
            Err(e) => tracing::warn!("m3u epg parse task failed: {e}"),
        }
    }

    let guide = merge_guides(guides);
    tracing::info!(
        programmes = guide.programmes.len(),
        channels = guide.channels.len(),
        "built m3u guide"
    );

    // Write-through to the SQLite cache (best-effort), like the Xtream guide path.
    if let Some(cat) = catalog.0.clone() {
        let sid = sid.to_string();
        let programmes = guide.programmes.clone();
        match task::spawn_blocking(move || cat.replace_programmes(&sid, &programmes)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("cache m3u programmes failed: {e}"),
            Err(e) => tracing::warn!("cache m3u programmes task failed: {e}"),
        }
    }

    state.epg.lock().unwrap().insert(sid.to_string(), guide);
    Ok(())
}

/// Now/next for an M3U playlist: build its guide if needed, then serve from memory.
async fn m3u_now_next(
    state: &AppState,
    catalog: &CatalogState,
    creds: &M3uCredentials,
    sid: &str,
    now: i64,
) -> Result<HashMap<String, NowNext>, AppError> {
    ensure_m3u_guide(state, catalog, creds, sid).await?;
    let guide = state.epg.lock().unwrap();
    let Some(guide) = guide.get(sid) else {
        return Ok(HashMap::new());
    };
    let mut map = now_next(&guide.programmes, now);
    add_name_aliases(&mut map, &name_index(&guide.channels));
    Ok(map)
}

/// Windowed programmes for an M3U playlist's timeline guide.
async fn m3u_programmes(
    state: &AppState,
    catalog: &CatalogState,
    creds: &M3uCredentials,
    sid: &str,
    from: i64,
    to: i64,
) -> Result<HashMap<String, Vec<Programme>>, AppError> {
    ensure_m3u_guide(state, catalog, creds, sid).await?;
    let guide = state.epg.lock().unwrap();
    let Some(guide) = guide.get(sid) else {
        return Ok(HashMap::new());
    };
    Ok(window_from_guide(guide, from, to))
}

/// Now/next for every channel of an account that has guide data, keyed by channel
/// id and by normalized display-name.
#[tauri::command]
pub async fn epg_now_next(
    app: AppHandle,
    state: State<'_, AppState>,
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
) -> Result<HashMap<String, NowNext>, AppError> {
    let sid = source_id(&creds);
    let now = unix_now();
    // An M3U playlist builds its guide from the EPG URLs it carries.
    let creds = match creds {
        SourceCredentials::M3u(m3u) => {
            return m3u_now_next(&state, &catalog, &m3u, &sid, now).await
        }
        SourceCredentials::Xtream(c) => c,
    };

    // Already loaded this session: serve from memory (with name-alias fallback).
    {
        let guide = state.epg.lock().unwrap();
        if let Some(guide) = guide.get(&sid) {
            let mut map = now_next(&guide.programmes, now);
            add_name_aliases(&mut map, &name_index(&guide.channels));
            return Ok(map);
        }
    }

    // Serve from the SQLite cache (no network round-trip), refreshing in the
    // background. Cache-only serves match by channel id; the name-alias fallback
    // returns once the in-memory guide is loaded.
    let cached =
        cached_programmes(&catalog, &sid, now - NOW_NEXT_WINDOW, now + NOW_NEXT_WINDOW).await;
    if !cached.is_empty() {
        spawn_refresh(app, creds, sid);
        return Ok(now_next(&cached, now));
    }

    // Cold: fetch + parse, then serve from memory.
    ensure_guide(&state, &catalog, &creds, &sid).await?;
    let guide = state.epg.lock().unwrap();
    let Some(guide) = guide.get(&sid) else {
        return Ok(HashMap::new());
    };
    let mut map = now_next(&guide.programmes, now);
    add_name_aliases(&mut map, &name_index(&guide.channels));
    Ok(map)
}

/// Programmes overlapping `[from, to]` for every channel, keyed by channel id and
/// normalized display-name, each list sorted by start.
#[tauri::command]
pub async fn epg_programmes(
    app: AppHandle,
    state: State<'_, AppState>,
    catalog: State<'_, CatalogState>,
    creds: SourceCredentials,
    from: i64,
    to: i64,
) -> Result<HashMap<String, Vec<Programme>>, AppError> {
    let sid = source_id(&creds);
    // An M3U playlist builds its guide from the EPG URLs it carries.
    let creds = match creds {
        SourceCredentials::M3u(m3u) => {
            return m3u_programmes(&state, &catalog, &m3u, &sid, from, to).await
        }
        SourceCredentials::Xtream(c) => c,
    };

    // Already loaded this session: serve from memory (with name-alias fallback).
    {
        let guide = state.epg.lock().unwrap();
        if let Some(guide) = guide.get(&sid) {
            return Ok(window_from_guide(guide, from, to));
        }
    }

    // Serve from the SQLite cache (no network round-trip), refreshing in the
    // background. Cache-only serves match by channel id (no name-alias fallback until
    // the in-memory guide loads).
    let cached = cached_programmes(&catalog, &sid, from, to).await;
    if !cached.is_empty() {
        spawn_refresh(app, creds, sid);
        return Ok(group_by_channel(cached));
    }

    // Cold: fetch + parse, then serve from memory.
    ensure_guide(&state, &catalog, &creds, &sid).await?;
    let guide = state.epg.lock().unwrap();
    let Some(guide) = guide.get(&sid) else {
        return Ok(HashMap::new());
    };
    Ok(window_from_guide(guide, from, to))
}

/// Build the windowed, per-channel programme map from an in-memory guide, aliased by
/// normalized display-name so name-only matches resolve.
fn window_from_guide(
    guide: &cathode_core::epg::Guide,
    from: i64,
    to: i64,
) -> HashMap<String, Vec<Programme>> {
    let mut map: HashMap<String, Vec<Programme>> = HashMap::new();
    for programme in &guide.programmes {
        if programme.stop > from && programme.start < to {
            map.entry(programme.channel_id.clone())
                .or_default()
                .push(programme.clone());
        }
    }
    for list in map.values_mut() {
        list.sort_by_key(|p| p.start);
    }
    add_name_aliases(&mut map, &name_index(&guide.channels));
    map
}

/// For each `normalized-name -> channel id` mapping, alias the channel's value under
/// the name too (without overwriting a real channel-id key), so the frontend can
/// look up by name when a stream has no `epg_channel_id`.
fn add_name_aliases<T: Clone>(map: &mut HashMap<String, T>, names: &HashMap<String, String>) {
    for (name, channel_id) in names {
        if map.contains_key(name) {
            continue;
        }
        if let Some(value) = map.get(channel_id).cloned() {
            map.insert(name.clone(), value);
        }
    }
}
