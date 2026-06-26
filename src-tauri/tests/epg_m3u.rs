//! Integration tests for M3U EPG: header detection, gzip-aware guide loading, and
//! the assembled now/next data path (parse + filter-to-playlist + match).

use std::collections::HashSet;
use std::io::Write;

use cathode_core::epg::{filter_to_channels, normalize_name, now_next, parse_xmltv};
use cathode_core::sources::m3u::epg_urls_from_header;
use cathode_lib::commands::sources::{load_guide_text, load_playlist_text};
use cathode_lib::http::ReqwestTransport;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A guide whose programmes span a huge range, so `now_next` matches at any sane time.
const XMLTV: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<tv>
  <channel id="test.ch"><display-name>Test Channel</display-name></channel>
  <programme channel="test.ch" start="20000101000000 +0000" stop="20990101000000 +0000"><title>Always On</title></programme>
  <channel id="other.ch"><display-name>Some Shopping</display-name></channel>
  <programme channel="other.ch" start="20000101000000 +0000" stop="20990101000000 +0000"><title>Unwanted</title></programme>
</tv>"#;

fn gzip(data: &str) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data.as_bytes()).unwrap();
    enc.finish().unwrap()
}

async fn server() -> MockServer {
    let server = MockServer::start().await;
    let m3u = format!(
        "#EXTM3U x-tvg-url=\"{}/guide.xml.gz\"\n\
         #EXTINF:-1 tvg-id=\"test.ch\" group-title=\"Test\",Test Channel\n\
         http://media/test.ts\n",
        server.uri()
    );
    Mock::given(method("GET"))
        .and(path("/list.m3u8"))
        .respond_with(ResponseTemplate::new(200).set_body_string(m3u))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/guide.xml.gz"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(gzip(XMLTV)))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn detects_epg_urls_from_the_playlist_header() {
    let server = server().await;
    let text = load_playlist_text(
        &ReqwestTransport::new(),
        &format!("{}/list.m3u8", server.uri()),
    )
    .await
    .unwrap();
    assert_eq!(
        epg_urls_from_header(&text),
        vec![format!("{}/guide.xml.gz", server.uri())]
    );
}

#[tokio::test]
async fn loads_and_gunzips_a_gzipped_guide_over_http() {
    let server = server().await;
    let xml = load_guide_text(
        &ReqwestTransport::new(),
        &format!("{}/guide.xml.gz", server.uri()),
    )
    .await
    .unwrap();
    assert!(xml.contains("<tv>"));
    assert!(xml.contains("Always On"));
}

#[tokio::test]
async fn gunzips_a_local_gzipped_file() {
    let file = std::env::temp_dir().join("cathode_guide_test.xml.gz");
    std::fs::write(&file, gzip(XMLTV)).unwrap();

    let xml = load_guide_text(&ReqwestTransport::new(), file.to_str().unwrap())
        .await
        .unwrap();
    assert!(xml.contains("Always On"));

    std::fs::remove_file(&file).ok();
}

#[tokio::test]
async fn builds_now_next_filtered_to_the_playlist_channels() {
    let server = server().await;
    // The data path ensure_m3u_guide runs: load (gunzip) -> parse -> filter -> match.
    let xml = load_guide_text(
        &ReqwestTransport::new(),
        &format!("{}/guide.xml.gz", server.uri()),
    )
    .await
    .unwrap();
    let mut guide = parse_xmltv(&xml).unwrap();

    let wanted_ids = HashSet::from(["test.ch".to_string()]);
    let wanted_names = HashSet::from([normalize_name("Test Channel")]);
    filter_to_channels(&mut guide, &wanted_ids, &wanted_names);

    // Only the playlist's channel survives.
    assert_eq!(guide.channels.len(), 1);
    assert_eq!(guide.programmes.len(), 1);

    let map = now_next(&guide.programmes, 1_700_000_000);
    assert_eq!(map["test.ch"].now.as_ref().unwrap().title, "Always On");
    assert!(
        !map.contains_key("other.ch"),
        "unwanted channel filtered out"
    );
}
