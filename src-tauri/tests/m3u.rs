//! Integration tests for the M3U source commands, exercising the real reqwest
//! transport against a `wiremock` server that serves a playlist, plus the local
//! file-loading path (parallel to `sources.rs`).

use cathode_core::sources::m3u::M3uCredentials;
use cathode_lib::commands::sources::{load_playlist_text, m3u_streams_impl};
use cathode_lib::http::ReqwestTransport;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SAMPLE_M3U: &str = "#EXTM3U\n\
    #EXTINF:-1 tvg-id=\"a.tv\" tvg-logo=\"http://logo/a.png\" group-title=\"News\",Channel A\n\
    http://media/a.ts\n\
    #EXTINF:-1 group-title=\"Sports\",Channel B\n\
    http://media/b.ts\n";

fn creds(url: String) -> M3uCredentials {
    M3uCredentials {
        name: "Test List".to_string(),
        url,
        epg_urls: vec![],
    }
}

#[tokio::test]
async fn fetches_and_parses_playlist_over_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/list.m3u"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SAMPLE_M3U))
        .mount(&server)
        .await;

    let streams = m3u_streams_impl(
        &ReqwestTransport::new(),
        &creds(format!("{}/list.m3u", server.uri())),
    )
    .await
    .unwrap();

    assert_eq!(streams.len(), 2);
    assert_eq!(streams[0].name, "Channel A");
    assert_eq!(streams[0].provider_id, "http://media/a.ts");
    assert_eq!(streams[0].epg_channel_id.as_deref(), Some("a.tv"));
    assert_eq!(streams[0].category_id.as_ref().unwrap().0, "News");
    assert_eq!(streams[1].category_id.as_ref().unwrap().0, "Sports");
}

#[tokio::test]
async fn http_error_status_becomes_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/list.m3u"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let result = m3u_streams_impl(
        &ReqwestTransport::new(),
        &creds(format!("{}/list.m3u", server.uri())),
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn loads_a_local_file_path() {
    let file = std::env::temp_dir().join("cathode_m3u_test.m3u");
    std::fs::write(&file, SAMPLE_M3U).unwrap();

    let text = load_playlist_text(&ReqwestTransport::new(), file.to_str().unwrap())
        .await
        .unwrap();
    assert!(text.contains("#EXTM3U"));

    std::fs::remove_file(&file).ok();
}
