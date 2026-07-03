//! Integration tests for the M3U source: parse a representative playlist fixture
//! into the normalized model (parallel to `xtream.rs`).

use cathode_core::model::{derive_stream_id, StreamKind};
use cathode_core::sources::m3u::{categories_from_streams, parse_playlist};

const SAMPLE_M3U: &str = include_str!("fixtures/sample.m3u");

#[test]
fn parses_every_entry_as_live() {
    let streams = parse_playlist(SAMPLE_M3U, "m3u:list").unwrap();
    assert_eq!(streams.len(), 4);
    assert!(streams.iter().all(|s| s.kind == StreamKind::Live));
}

#[test]
fn maps_attributes_and_keeps_playback_urls() {
    let streams = parse_playlist(SAMPLE_M3U, "m3u:list").unwrap();

    let bbc = &streams[0];
    assert_eq!(bbc.name, "BBC One");
    assert_eq!(bbc.provider_id, "http://server/live/bbc1.ts");
    assert_eq!(bbc.id, derive_stream_id("m3u:list", "bbc1.uk"));
    assert_eq!(bbc.epg_channel_id.as_deref(), Some("bbc1.uk"));
    assert_eq!(bbc.logo.as_deref(), Some("http://logo/bbc.png"));
    assert_eq!(bbc.category_id.as_ref().unwrap().0, "UK");

    // No tvg-id -> id derived from name + url; group still parsed.
    let sky = &streams[2];
    assert_eq!(sky.epg_channel_id, None);
    assert_eq!(
        sky.id,
        derive_stream_id(
            "m3u:list",
            "Sky Sports Main Event|http://server/live/skysports.ts"
        )
    );
    assert_eq!(sky.category_id.as_ref().unwrap().0, "Sports");

    // Ungrouped entry falls into the default category so it stays browseable.
    let orphan = &streams[3];
    assert_eq!(orphan.category_id.as_ref().unwrap().0, "Uncategorized");
}

#[test]
fn categories_are_distinct_in_first_seen_order() {
    let streams = parse_playlist(SAMPLE_M3U, "m3u:list").unwrap();
    let names: Vec<_> = categories_from_streams(&streams)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, vec!["UK", "Sports", "Uncategorized"]);
}
