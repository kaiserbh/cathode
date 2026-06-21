//! EPG command: fetch + cache an account's XMLTV guide, return now/next per channel.
//!
//! The guide is fetched and parsed once per source per session (held in
//! `AppState.epg`); subsequent calls just recompute now/next against the current
//! time, so the UI can poll cheaply to keep labels fresh. EPG is best-effort — a
//! provider without `xmltv.php` surfaces an error the caller can ignore.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use cathode_core::epg::{now_next, parse_xmltv};
use cathode_core::error::AppError;
use cathode_core::model::NowNext;
use cathode_core::sources::xtream::{XtreamCredentials, XtreamSource};
use cathode_core::transport::Transport;
use tauri::State;
use tokio::task;
use tracing::{info_span, Instrument};

use crate::commands::sources::{join_err, source_id};
use crate::state::AppState;

/// Now/next for every channel of an account that has guide data.
#[tauri::command]
pub async fn epg_now_next(
    state: State<'_, AppState>,
    creds: XtreamCredentials,
) -> Result<HashMap<String, NowNext>, AppError> {
    let sid = source_id(&creds);

    // Cache miss: fetch the guide and parse it off the async runtime, then store it.
    if !state.epg.lock().unwrap().contains_key(&sid) {
        let url = XtreamSource::from_credentials(&creds).xmltv_url();
        let xml = async { state.transport.get_text(&url).await }
            .instrument(info_span!("epg_fetch", source = %sid))
            .await
            .map_err(AppError::from)?;
        let programmes = task::spawn_blocking(move || parse_xmltv(&xml))
            .await
            .map_err(join_err)?
            .map_err(AppError::from)?;
        state.epg.lock().unwrap().insert(sid.clone(), programmes);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let guide = state.epg.lock().unwrap();
    let programmes = guide.get(&sid).map(Vec::as_slice).unwrap_or(&[]);
    Ok(now_next(programmes, now))
}
