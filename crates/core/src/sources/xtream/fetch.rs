//! Fetch-and-parse orchestration for Xtream.
//!
//! Generic over [`Transport`] so the same logic runs against a fake in tests and
//! against reqwest in the shell. Each function builds the right `player_api.php`
//! URL, fetches the body, and hands it to the matching parser.

use crate::error::CoreError;
use crate::model::{Category, SeriesInfo, Stream, StreamKind};
use crate::transport::Transport;

use super::parse::{
    parse_categories, parse_live_streams, parse_series, parse_series_info, parse_vod_streams,
};
use super::XtreamSource;

/// The `player_api.php` action for a content kind's category list.
fn categories_action(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Live => "get_live_categories",
        StreamKind::Vod => "get_vod_categories",
        StreamKind::Series => "get_series_categories",
    }
}

/// The `player_api.php` action for a content kind's stream/series list.
fn streams_action(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Live => "get_live_streams",
        StreamKind::Vod => "get_vod_streams",
        StreamKind::Series => "get_series",
    }
}

/// Fetch and parse the categories of a given content kind for a source.
pub async fn fetch_categories<T: Transport>(
    source: &XtreamSource,
    transport: &T,
    kind: StreamKind,
) -> Result<Vec<Category>, CoreError> {
    let url = source.player_api_url(categories_action(kind), None);
    let body = transport.get_text(&url).await?;
    parse_categories(&body)
}

/// Fetch and parse the streams of a given content kind for a source, optionally
/// scoped to a category. Series come back as entries to drill into, not playables.
pub async fn fetch_streams<T: Transport>(
    source: &XtreamSource,
    transport: &T,
    kind: StreamKind,
    category_id: Option<&str>,
) -> Result<Vec<Stream>, CoreError> {
    let url = source.player_api_url(streams_action(kind), category_id);
    let body = transport.get_text(&url).await?;
    let source_id = source.source_id();
    match kind {
        StreamKind::Live => parse_live_streams(&body, &source_id),
        StreamKind::Vod => parse_vod_streams(&body, &source_id),
        StreamKind::Series => parse_series(&body, &source_id),
    }
}

/// Fetch and parse the seasons/episodes for one series.
pub async fn fetch_series_info<T: Transport>(
    source: &XtreamSource,
    transport: &T,
    series_id: &str,
) -> Result<SeriesInfo, CoreError> {
    let url = source.series_info_url(series_id);
    let body = transport.get_text(&url).await?;
    parse_series_info(&body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StreamKind;
    use std::sync::Mutex;

    const CATEGORIES_JSON: &str =
        include_str!("../../../tests/fixtures/xtream_live_categories.json");
    const STREAMS_JSON: &str = include_str!("../../../tests/fixtures/xtream_live_streams.json");

    /// A transport that returns a canned body and records the URL it was asked
    /// for. A `Mutex` (not `RefCell`) keeps it `Sync`, so the returned future is
    /// `Send` as the trait requires; the body is moved into the future as owned
    /// data so nothing non-`Send` is borrowed across the await.
    struct FakeTransport {
        body: String,
        last_url: Mutex<Option<String>>,
    }

    impl FakeTransport {
        fn new(body: &str) -> Self {
            Self {
                body: body.to_string(),
                last_url: Mutex::new(None),
            }
        }
        fn requested_url(&self) -> Option<String> {
            self.last_url.lock().unwrap().clone()
        }
    }

    impl Transport for FakeTransport {
        fn get_text(
            &self,
            url: &str,
        ) -> impl std::future::Future<Output = Result<String, CoreError>> + Send {
            *self.last_url.lock().unwrap() = Some(url.to_string());
            let body = self.body.clone();
            async move { Ok(body) }
        }
    }

    fn source() -> XtreamSource {
        XtreamSource::new("http://host:8080", "user", "pass")
    }

    #[test]
    fn fetches_categories_from_the_right_url() {
        let transport = FakeTransport::new(CATEGORIES_JSON);
        let categories =
            pollster::block_on(fetch_categories(&source(), &transport, StreamKind::Live)).unwrap();

        assert_eq!(categories.len(), 2);
        assert_eq!(
            transport.requested_url().unwrap(),
            "http://host:8080/player_api.php?username=user&password=pass&action=get_live_categories"
        );
    }

    #[test]
    fn fetches_streams_with_category_filter() {
        let transport = FakeTransport::new(STREAMS_JSON);
        let streams = pollster::block_on(fetch_streams(
            &source(),
            &transport,
            StreamKind::Live,
            Some("1"),
        ))
        .unwrap();

        assert_eq!(streams.len(), 3);
        assert!(streams.iter().all(|s| s.kind == StreamKind::Live));
        assert_eq!(
            transport.requested_url().unwrap(),
            "http://host:8080/player_api.php?username=user&password=pass&action=get_live_streams&category_id=1"
        );
    }

    #[test]
    fn vod_streams_use_the_vod_action() {
        let transport = FakeTransport::new("[]");
        let _ = pollster::block_on(fetch_streams(&source(), &transport, StreamKind::Vod, None));
        assert_eq!(
            transport.requested_url().unwrap(),
            "http://host:8080/player_api.php?username=user&password=pass&action=get_vod_streams"
        );
    }

    #[test]
    fn series_use_the_series_action() {
        let transport = FakeTransport::new("[]");
        let _ = pollster::block_on(fetch_streams(
            &source(),
            &transport,
            StreamKind::Series,
            None,
        ));
        assert_eq!(
            transport.requested_url().unwrap(),
            "http://host:8080/player_api.php?username=user&password=pass&action=get_series"
        );
    }

    #[test]
    fn propagates_parse_errors() {
        let transport = FakeTransport::new("not json");
        let result = pollster::block_on(fetch_categories(&source(), &transport, StreamKind::Live));
        assert!(matches!(result, Err(CoreError::Json { .. })));
    }
}
