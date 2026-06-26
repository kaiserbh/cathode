//! Source adapters: turn a provider's wire format into the normalized model.
//!
//! Each source lives in its own submodule. They all emit the same `Stream` /
//! `Category` types; the only place a source's identity is allowed to surface is
//! URL resolution, which is modelled on the source itself.
//!
//! [`SourceCredentials`] is the one polymorphic value the rest of the app passes
//! around, so commands, the catalog, and the UI stay source-agnostic.

pub mod m3u;
pub mod xtream;

use serde::{Deserialize, Serialize};

use crate::catalog::SourceRecord;
use crate::error::CoreError;

use m3u::M3uCredentials;
use xtream::XtreamCredentials;

/// Credentials for any kind of source, as they cross the command boundary.
///
/// Adjacently tagged (`{"type": "...", "data": {...}}`) so it round-trips reliably
/// through both `serde_json` (Tauri side) and `serde-wasm-bindgen` (the WASM UI,
/// which deserializes the `saved_sources` result JS→Rust).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SourceCredentials {
    Xtream(XtreamCredentials),
    M3u(M3uCredentials),
}

impl SourceCredentials {
    /// The stable account/playlist id, used to key the catalog and a stream's
    /// stable id. Dispatches to the underlying source.
    pub fn source_id(&self) -> String {
        match self {
            Self::Xtream(c) => xtream::XtreamSource::from_credentials(c).source_id(),
            Self::M3u(c) => m3u::M3uSource::from_credentials(c).source_id(),
        }
    }

    /// Serialize to the opaque [`SourceRecord`] the catalog persists.
    pub fn to_record(&self) -> Result<SourceRecord, CoreError> {
        match self {
            Self::Xtream(c) => xtream::source_record(c),
            Self::M3u(c) => m3u::source_record(c),
        }
    }
}

/// Recover credentials from a persisted [`SourceRecord`], selecting the source kind
/// by its discriminator. An unknown kind is a storage error.
pub fn credentials_from_record(record: &SourceRecord) -> Result<SourceCredentials, CoreError> {
    match record.kind.as_str() {
        xtream::XTREAM_KIND => {
            xtream::credentials_from_record(record).map(SourceCredentials::Xtream)
        }
        m3u::M3U_KIND => m3u::credentials_from_record(record).map(SourceCredentials::M3u),
        other => Err(CoreError::storage(
            "unknown source kind",
            format!("unrecognized source kind {other:?}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xtream() -> SourceCredentials {
        SourceCredentials::Xtream(XtreamCredentials {
            base_url: "http://host:8080".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
        })
    }

    fn m3u() -> SourceCredentials {
        SourceCredentials::M3u(M3uCredentials {
            name: "My List".to_string(),
            url: "http://host/list.m3u".to_string(),
            epg_urls: vec![],
        })
    }

    #[test]
    fn source_id_dispatches_per_kind() {
        assert_eq!(xtream().source_id(), "xtream:http://host:8080|user");
        assert_eq!(m3u().source_id(), "m3u:http://host/list.m3u");
    }

    #[test]
    fn record_round_trips_for_each_kind() {
        for creds in [xtream(), m3u()] {
            let record = creds.to_record().unwrap();
            assert_eq!(record.id, creds.source_id());
            assert_eq!(credentials_from_record(&record).unwrap(), creds);
        }
    }

    #[test]
    fn unknown_record_kind_is_a_storage_error() {
        let record = SourceRecord {
            id: "x".to_string(),
            kind: "rss".to_string(),
            payload: "{}".to_string(),
        };
        let err = credentials_from_record(&record).unwrap_err();
        assert_eq!(crate::error::AppError::from(err).code, "storage");
    }

    #[test]
    fn serde_is_adjacently_tagged() {
        // The wire shape the UI and Tauri both rely on.
        let json = serde_json::to_value(m3u()).unwrap();
        assert_eq!(json["type"], "m3u");
        assert_eq!(json["data"]["url"], "http://host/list.m3u");
        // And it round-trips back.
        let back: SourceCredentials = serde_json::from_value(json).unwrap();
        assert_eq!(back, m3u());
    }
}
