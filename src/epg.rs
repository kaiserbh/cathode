//! Frontend EPG lookup: resolve a stream to its guide data.
//!
//! The backend keys both the now/next map and the programmes map by channel id and
//! by normalized display-name, so a stream matches by its `epg_channel_id` when it
//! has one, and otherwise by its normalized name (the name-based fallback).

use std::collections::HashMap;

use cathode_core::epg::normalize_name;
use cathode_core::model::Stream;

/// Find a stream's guide value: by `epg_channel_id` first, then by normalized name.
pub fn resolve<'a, T>(map: &'a HashMap<String, T>, stream: &Stream) -> Option<&'a T> {
    stream
        .epg_channel_id
        .as_ref()
        .and_then(|id| map.get(id))
        .or_else(|| map.get(&normalize_name(&stream.name)))
}
