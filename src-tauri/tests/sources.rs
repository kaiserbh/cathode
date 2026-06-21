//! Integration tests for the source commands, exercising the real reqwest
//! transport against a `wiremock` server that stands in for an Xtream provider.

use cathode_core::model::StreamKind;
use cathode_core::sources::xtream::XtreamCredentials;
use cathode_lib::commands::sources::{list_categories_impl, list_streams_impl};
use cathode_lib::http::ReqwestTransport;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CATEGORIES_JSON: &str = r#"[
    {"category_id": "1", "category_name": "News"},
    {"category_id": 2, "category_name": "Sports"}
]"#;

const STREAMS_JSON: &str = r#"[
    {"stream_id": 1001, "name": "BBC News", "stream_icon": "http://logo/bbc.png", "category_id": "1"},
    {"stream_id": "1002", "name": "Sky Sports", "stream_icon": "", "category_id": 2}
]"#;

fn creds(base_url: String) -> XtreamCredentials {
    XtreamCredentials {
        base_url,
        username: "user".to_string(),
        password: "pass".to_string(),
    }
}

#[tokio::test]
async fn list_categories_hits_player_api_and_parses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/player_api.php"))
        .and(query_param("action", "get_live_categories"))
        .respond_with(ResponseTemplate::new(200).set_body_string(CATEGORIES_JSON))
        .mount(&server)
        .await;

    let categories = list_categories_impl(
        &ReqwestTransport::new(),
        &creds(server.uri()),
        StreamKind::Live,
    )
    .await
    .unwrap();

    assert_eq!(categories.len(), 2);
    assert_eq!(categories[0].name, "News");
}

#[tokio::test]
async fn list_streams_passes_category_filter_and_parses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/player_api.php"))
        .and(query_param("action", "get_live_streams"))
        .and(query_param("category_id", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(STREAMS_JSON))
        .mount(&server)
        .await;

    let streams = list_streams_impl(
        &ReqwestTransport::new(),
        &creds(server.uri()),
        StreamKind::Live,
        Some("1"),
    )
    .await
    .unwrap();

    assert_eq!(streams.len(), 2);
    assert_eq!(streams[0].name, "BBC News");
    // Empty icon string collapses to None through the parser.
    assert_eq!(streams[1].logo, None);
}

#[tokio::test]
async fn http_error_status_becomes_network_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/player_api.php"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let result = list_categories_impl(
        &ReqwestTransport::new(),
        &creds(server.uri()),
        StreamKind::Live,
    )
    .await;
    assert!(result.is_err());
}
