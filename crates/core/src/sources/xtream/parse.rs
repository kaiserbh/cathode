//! Map raw Xtream API responses onto the normalized model.
//!
//! These are pure functions: the caller fetches the JSON, these turn it into
//! `Category` / `Stream`. All provider quirks were already absorbed in `api.rs`.

use crate::error::CoreError;
use crate::model::{derive_stream_id, Category, CategoryId, Stream, StreamKind};

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
        .map(|s| Stream {
            id: derive_stream_id(source_id, &s.stream_id.0),
            name: s.name,
            // An empty icon string is no icon.
            logo: s.stream_icon.filter(|l| !l.is_empty()),
            // An empty/absent category is no category.
            category_id: s
                .category_id
                .map(|c| c.0)
                .filter(|c| !c.is_empty())
                .map(CategoryId),
            kind: StreamKind::Live,
        })
        .collect())
}
