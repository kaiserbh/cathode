//! The normalized stream record.
//!
//! This is the heart of the shared contract: every source (Xtream today, M3U
//! later) maps onto this one shape, and the UI consumes it directly. It carries
//! no provider-specific fields. How to turn a stream into a playable URL depends
//! on the source and is modelled separately in a later increment, not here.

use serde::{Deserialize, Serialize};

use crate::model::category::CategoryId;
use crate::model::id::{derive_stream_id, StreamId};

/// What kind of content a stream is. Xtream separates its catalog into these
/// three classes; we keep the distinction so the UI can route each correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    Live,
    Vod,
    Series,
}

/// A single normalized stream (channel, movie, or series entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stream {
    pub id: StreamId,
    pub name: String,
    pub logo: Option<String>,
    pub category_id: Option<CategoryId>,
    pub kind: StreamKind,
}

impl Stream {
    /// Build a stream, deriving its stable [`StreamId`] from the source id and
    /// the provider's stable key (for Xtream, the `stream_id`).
    pub fn new(
        source_id: &str,
        stable_key: &str,
        name: impl Into<String>,
        kind: StreamKind,
    ) -> Self {
        Self {
            id: derive_stream_id(source_id, stable_key),
            name: name.into(),
            logo: None,
            category_id: None,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_derives_stable_id() {
        // The same source + provider key always yields the same id, so a re-sync
        // keeps favorites/history pointing at the right stream.
        let a = Stream::new("src-1", "12345", "BBC One", StreamKind::Live);
        let b = Stream::new("src-1", "12345", "BBC One HD (renamed)", StreamKind::Live);
        assert_eq!(
            a.id, b.id,
            "id must depend on the key, not the display name"
        );
        assert_eq!(a.id, derive_stream_id("src-1", "12345"));
    }

    #[test]
    fn json_round_trip() {
        // Streams cross the command boundary; serde must round-trip cleanly.
        let mut stream = Stream::new("src-1", "12345", "BBC One", StreamKind::Live);
        stream.logo = Some("http://logo.example/bbc.png".to_string());
        stream.category_id = Some(CategoryId("5".to_string()));

        let json = serde_json::to_string(&stream).unwrap();
        let back: Stream = serde_json::from_str(&json).unwrap();
        assert_eq!(stream, back);
    }

    #[test]
    fn kind_serializes_snake_case() {
        // The UI and any stored rows depend on this wire spelling.
        let json = serde_json::to_string(&StreamKind::Vod).unwrap();
        assert_eq!(json, "\"vod\"");
    }
}
