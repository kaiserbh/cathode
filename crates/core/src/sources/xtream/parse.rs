//! Map raw Xtream API responses onto the normalized model.
//!
//! These are pure functions: the caller fetches the JSON, these turn it into
//! `Category` / `Stream`. All provider quirks were already absorbed in `api.rs`.

use crate::error::CoreError;
use crate::model::{Category, CategoryId, Stream, StreamKind};

use super::api::{RawCategory, RawLiveStream};

/// Parse a `get_live_categories` response into normalized categories.
pub fn parse_live_categories(json: &str) -> Result<Vec<Category>, CoreError> {
    let raw: Vec<RawCategory> =
        serde_json::from_str(json).map_err(|e| CoreError::json("live categories", e))?;

    Ok(raw
        .into_iter()
        .map(|c| Category {
            id: CategoryId(c.category_id.0),
            name: c.category_name,
        })
        .collect())
}

/// Parse a `get_live_streams` response into normalized streams.
///
/// `source_id` comes from [`super::XtreamSource::source_id`] and combines with
/// each Xtream `stream_id` to derive the stable [`crate::model::StreamId`].
pub fn parse_live_streams(json: &str, source_id: &str) -> Result<Vec<Stream>, CoreError> {
    let raw: Vec<RawLiveStream> =
        serde_json::from_str(json).map_err(|e| CoreError::json("live streams", e))?;

    Ok(raw
        .into_iter()
        .map(|s| {
            let mut stream = Stream::new(source_id, &s.stream_id.0, s.name, StreamKind::Live);
            // An empty icon string is no icon.
            stream.logo = s.stream_icon.filter(|l| !l.is_empty());
            // An empty/absent category is no category.
            stream.category_id = s
                .category_id
                .map(|c| c.0)
                .filter(|c| !c.is_empty())
                .map(CategoryId);
            // An empty epg id means no EPG mapping for this channel.
            stream.epg_channel_id = s.epg_channel_id.filter(|e| !e.is_empty());
            stream
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_epg_channel_id_and_treats_empty_as_none() {
        let json = r#"[
            {"stream_id": 1, "name": "A", "epg_channel_id": "bbc1.uk"},
            {"stream_id": 2, "name": "B", "epg_channel_id": ""},
            {"stream_id": 3, "name": "C"}
        ]"#;
        let streams = parse_live_streams(json, "src-1").unwrap();
        assert_eq!(streams[0].epg_channel_id.as_deref(), Some("bbc1.uk"));
        assert_eq!(streams[1].epg_channel_id, None, "empty string -> None");
        assert_eq!(streams[2].epg_channel_id, None, "absent -> None");
    }
}
