//! Fetch-and-parse orchestration for an M3U playlist served over HTTP.
//!
//! Generic over [`Transport`] so the same logic runs against a fake in tests and
//! against reqwest in the shell. This covers the URL case only; loading a playlist
//! from a local file is native-only and handled in the shell, which then calls
//! [`parse_playlist`] directly.

use crate::error::CoreError;
use crate::model::Stream;
use crate::transport::Transport;

use super::parse::parse_playlist;
use super::M3uSource;

/// Fetch a playlist over its URL and parse it into normalized live streams.
pub async fn fetch_playlist<T: Transport>(
    source: &M3uSource,
    transport: &T,
) -> Result<Vec<Stream>, CoreError> {
    let body = transport.get_text(source.playlist_url()).await?;
    parse_playlist(&body, &source.source_id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StreamKind;
    use std::sync::Mutex;

    /// A transport that returns a canned body and records the URL it was asked for.
    /// A `Mutex` keeps it `Sync` so the returned future is `Send`, as the trait
    /// requires; the body is moved into the future as owned data.
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

    #[test]
    fn fetches_from_the_playlist_url_and_parses() {
        let m3u = "#EXTM3U\n#EXTINF:-1 tvg-id=\"a\",A\nhttp://h/a.ts\n";
        let transport = FakeTransport::new(m3u);
        let source = M3uSource::new("http://host/list.m3u");

        let streams = pollster::block_on(fetch_playlist(&source, &transport)).unwrap();

        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].kind, StreamKind::Live);
        assert_eq!(streams[0].provider_id, "http://h/a.ts");
        assert_eq!(transport.requested_url().unwrap(), "http://host/list.m3u");
    }

    #[test]
    fn ids_are_scoped_to_the_source() {
        let m3u = "#EXTINF:-1 tvg-id=\"a\",A\nhttp://h/a.ts\n";
        let one = pollster::block_on(fetch_playlist(
            &M3uSource::new("http://host/one.m3u"),
            &FakeTransport::new(m3u),
        ))
        .unwrap();
        let two = pollster::block_on(fetch_playlist(
            &M3uSource::new("http://host/two.m3u"),
            &FakeTransport::new(m3u),
        ))
        .unwrap();
        // Same entry under two different playlists is two distinct records.
        assert_ne!(one[0].id, two[0].id);
    }
}
