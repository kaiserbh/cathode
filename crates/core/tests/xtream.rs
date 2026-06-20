//! Integration tests for the Xtream parser, driven by realistic fixture JSON
//! (including the string-vs-number id quirk and missing optional fields).

use cathode_core::model::{CategoryId, StreamKind};
use cathode_core::sources::xtream::{parse_live_categories, parse_live_streams, XtreamSource};

const CATEGORIES_JSON: &str = include_str!("fixtures/xtream_live_categories.json");
const STREAMS_JSON: &str = include_str!("fixtures/xtream_live_streams.json");

#[test]
fn parses_live_categories() {
    let categories = parse_live_categories(CATEGORIES_JSON).unwrap();
    assert_eq!(categories.len(), 2);
    assert_eq!(categories[0].id, CategoryId("1".to_string()));
    assert_eq!(categories[0].name, "News");
    // Numeric category_id from the wire coerces to the same string form.
    assert_eq!(categories[1].id, CategoryId("2".to_string()));
}

#[test]
fn parses_live_streams_onto_normalized_model() {
    let source = XtreamSource::new("http://host:8080", "user", "pass");
    let streams = parse_live_streams(STREAMS_JSON, &source.source_id()).unwrap();

    assert_eq!(streams.len(), 3);

    // Every live stream maps to kind Live, regardless of id wire format.
    assert!(streams.iter().all(|s| s.kind == StreamKind::Live));

    // First stream: full data.
    assert_eq!(streams[0].name, "BBC News");
    assert_eq!(
        streams[0].logo.as_deref(),
        Some("http://logo.example/bbc.png")
    );
    assert_eq!(streams[0].category_id, Some(CategoryId("1".to_string())));

    // Second stream: empty logo string becomes None; numeric category coerces.
    assert_eq!(streams[1].logo, None);
    assert_eq!(streams[1].category_id, Some(CategoryId("2".to_string())));

    // Third stream: no category at all.
    assert_eq!(streams[2].category_id, None);
}

#[test]
fn stream_ids_are_stable_and_match_derivation() {
    let source = XtreamSource::new("http://host:8080", "user", "pass");
    let streams = parse_live_streams(STREAMS_JSON, &source.source_id()).unwrap();

    // The id must be derived from the source id + the Xtream stream_id, so a
    // re-sync (re-parse) yields identical ids.
    let expected = cathode_core::model::derive_stream_id(&source.source_id(), "1001");
    assert_eq!(streams[0].id, expected);

    // And ids are distinct across streams.
    assert_ne!(streams[0].id, streams[1].id);
}
