//! EPG programmes and the now/next pair the UI shows per channel.
//!
//! Timestamps are Unix seconds (UTC) so they compare cheaply and cross the command
//! boundary without a date type. Matching to a channel is by `channel_id`, which is
//! the XMLTV channel id / Xtream `epg_channel_id` (tvg-id) carried on a `Stream`.

use serde::{Deserialize, Serialize};

/// A single guide entry for one channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Programme {
    /// XMLTV channel id (matches a stream's `epg_channel_id`).
    pub channel_id: String,
    pub title: String,
    /// The `<desc>` text, when the guide carries one.
    #[serde(default)]
    pub description: Option<String>,
    /// Start time, Unix seconds (UTC).
    pub start: i64,
    /// Stop time, Unix seconds (UTC).
    pub stop: i64,
}

/// What's on a channel now and what's up next.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NowNext {
    pub now: Option<Programme>,
    pub next: Option<Programme>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trips() {
        // Programmes and now/next cross the command boundary.
        let nn = NowNext {
            now: Some(Programme {
                channel_id: "bbc1.uk".to_string(),
                title: "News".to_string(),
                description: Some("The latest headlines.".to_string()),
                start: 1_700_000_000,
                stop: 1_700_001_800,
            }),
            next: None,
        };
        let json = serde_json::to_string(&nn).unwrap();
        assert_eq!(serde_json::from_str::<NowNext>(&json).unwrap(), nn);
    }
}
