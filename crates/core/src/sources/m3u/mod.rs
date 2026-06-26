//! Plain M3U / M3U8 playlist source.
//!
//! Unlike Xtream there is no API and no per-stream URL building: a playlist is a
//! single document (fetched by URL or read from a file) whose entries already
//! carry their playable URLs. This module owns the credentials, the stable source
//! id, and persistence; [`parse`] maps the document onto the normalized model and
//! [`fetch`] loads it over a [`crate::transport::Transport`].

pub mod fetch;
pub mod parse;

pub use fetch::fetch_playlist;
pub use parse::{categories_from_streams, epg_urls_from_header, parse_playlist};

use serde::{Deserialize, Serialize};

use crate::catalog::SourceRecord;
use crate::error::CoreError;

/// Discriminator stored on a persisted M3U source.
pub const M3U_KIND: &str = "m3u";

/// Turn credentials into the opaque [`SourceRecord`] the catalog persists: the id
/// is the stable playlist id, and the payload is the credentials as JSON.
pub fn source_record(creds: &M3uCredentials) -> Result<SourceRecord, CoreError> {
    let payload = serde_json::to_string(creds)
        .map_err(|e| CoreError::storage("serialize m3u source", e.to_string()))?;
    Ok(SourceRecord {
        id: M3uSource::from_credentials(creds).source_id(),
        kind: M3U_KIND.to_string(),
        payload,
    })
}

/// Recover credentials from a persisted [`SourceRecord`] (the inverse of
/// [`source_record`]).
pub fn credentials_from_record(record: &SourceRecord) -> Result<M3uCredentials, CoreError> {
    serde_json::from_str(&record.payload)
        .map_err(|e| CoreError::storage("deserialize m3u source", e.to_string()))
}

/// An M3U playlist source as it crosses the command boundary from the UI.
///
/// `name` is the user-visible label; `url` is either an `http(s)` URL or a local
/// file path — the shell decides how to load it (`core` never touches the disk).
/// `epg_urls` are the XMLTV guide sources to merge for this playlist (often
/// pre-filled from the playlist's `#EXTM3U x-tvg-url` header, then user-trimmed); it
/// does not affect [`M3uSource::source_id`], so editing it preserves favorites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct M3uCredentials {
    pub name: String,
    pub url: String,
    /// XMLTV EPG URLs (or file paths) for this playlist. `#[serde(default)]` keeps
    /// records written before this field existed loadable.
    #[serde(default)]
    pub epg_urls: Vec<String>,
}

/// A configured M3U playlist: where to load it from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct M3uSource {
    location: String,
}

impl M3uSource {
    /// Build a source from a playlist location (URL or file path).
    pub fn new(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
        }
    }

    /// Build a source from credentials received across the command boundary.
    pub fn from_credentials(creds: &M3uCredentials) -> Self {
        Self::new(&creds.url)
    }

    /// A stable identifier for this playlist, used as the `source_id` half of a
    /// stream's stable id. Distinct across playlists; independent of the label.
    pub fn source_id(&self) -> String {
        format!("m3u:{}", self.location)
    }

    /// The location to load the playlist from (URL or file path).
    pub fn playlist_url(&self) -> &str {
        &self.location
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds() -> M3uCredentials {
        M3uCredentials {
            name: "My List".to_string(),
            url: "http://host/playlist.m3u".to_string(),
            epg_urls: vec!["http://host/guide.xml.gz".to_string()],
        }
    }

    #[test]
    fn source_id_is_stable_per_playlist() {
        let source = M3uSource::from_credentials(&creds());
        assert_eq!(source.source_id(), "m3u:http://host/playlist.m3u");
        // Neither the label nor the EPG list affects the id; only the location does,
        // so editing the EPG of a saved playlist keeps its favorites/history.
        let edited = M3uCredentials {
            name: "Renamed".to_string(),
            url: "http://host/playlist.m3u".to_string(),
            epg_urls: vec![],
        };
        assert_eq!(
            M3uSource::from_credentials(&edited).source_id(),
            source.source_id()
        );
    }

    #[test]
    fn source_record_round_trips_through_credentials() {
        let record = source_record(&creds()).unwrap();
        assert_eq!(record.id, M3uSource::from_credentials(&creds()).source_id());
        assert_eq!(record.kind, M3U_KIND);
        assert_eq!(credentials_from_record(&record).unwrap(), creds());
    }

    #[test]
    fn record_without_epg_field_still_loads() {
        // A payload written before `epg_urls` existed must deserialize (serde default).
        let record = SourceRecord {
            id: "m3u:http://host/playlist.m3u".to_string(),
            kind: M3U_KIND.to_string(),
            payload: r#"{"name":"Old","url":"http://host/playlist.m3u"}"#.to_string(),
        };
        let creds = credentials_from_record(&record).unwrap();
        assert_eq!(creds.url, "http://host/playlist.m3u");
        assert!(creds.epg_urls.is_empty());
    }

    #[test]
    fn credentials_from_a_bad_record_is_a_storage_error() {
        let record = SourceRecord {
            id: "x".to_string(),
            kind: M3U_KIND.to_string(),
            payload: "not json".to_string(),
        };
        let err = credentials_from_record(&record).unwrap_err();
        assert_eq!(crate::error::AppError::from(err).code, "storage");
    }
}
