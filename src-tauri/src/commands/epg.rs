//! EPG commands: fetch + cache an account's XMLTV guide, return now/next per
//! channel and windowed programmes for the timeline guide.
//!
//! The guide is fetched and parsed once per source per session (held in
//! `AppState.epg`); later calls just recompute against the current time, so the UI
//! can poll cheaply. Both responses are keyed by channel id and additionally by
//! normalized display-name, so the frontend can resolve a channel by name when it
//! has no `epg_channel_id`. EPG is best-effort — a provider without `xmltv.php`
//! surfaces an error the caller can ignore.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use cathode_core::epg::{name_index, now_next, parse_xmltv};
use cathode_core::error::AppError;
use cathode_core::model::{NowNext, Programme};
use cathode_core::sources::xtream::{XtreamCredentials, XtreamSource};
use cathode_core::transport::Transport;
use tauri::State;
use tokio::task;
use tracing::{info_span, Instrument};

use crate::commands::sources::{join_err, source_id};
use crate::state::AppState;

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Ensure the source's guide is parsed and cached (fetch off the async runtime on
/// a miss). Holds the lock only for the quick presence check and the insert, never
/// across the await.
async fn ensure_guide(
    state: &AppState,
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
        .map_err(AppError::from)?;
    let guide = task::spawn_blocking(move || parse_xmltv(&xml))
        .await
        .map_err(join_err)?
        .map_err(AppError::from)?;
    state.epg.lock().unwrap().insert(sid.to_string(), guide);
    Ok(())
}

/// Now/next for every channel of an account that has guide data, keyed by channel
/// id and by normalized display-name.
#[tauri::command]
pub async fn epg_now_next(
    state: State<'_, AppState>,
    creds: XtreamCredentials,
) -> Result<HashMap<String, NowNext>, AppError> {
    let sid = source_id(&creds);
    ensure_guide(&state, &creds, &sid).await?;

    let now = unix_now();
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
    state: State<'_, AppState>,
    creds: XtreamCredentials,
    from: i64,
    to: i64,
) -> Result<HashMap<String, Vec<Programme>>, AppError> {
    let sid = source_id(&creds);
    ensure_guide(&state, &creds, &sid).await?;

    let guide = state.epg.lock().unwrap();
    let Some(guide) = guide.get(&sid) else {
        return Ok(HashMap::new());
    };
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
    Ok(map)
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
