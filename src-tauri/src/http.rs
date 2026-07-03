//! The concrete HTTP transport: reqwest behind `cathode_core`'s `Transport`.
//!
//! This is the only place reqwest is used. It flattens reqwest errors into
//! `CoreError::network` so no native error type crosses back into `core`.

use cathode_core::error::CoreError;
use cathode_core::redact;
use cathode_core::transport::Transport;
use std::future::Future;

/// Identifies Cathode to providers. Some Xtream panels reject requests without a
/// recognizable User-Agent, so we always send one.
const USER_AGENT: &str = concat!("Cathode/", env!("CARGO_PKG_VERSION"));

/// A `Transport` backed by a shared reqwest client (connection pool reused).
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl ReqwestTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ReqwestTransport {
    /// Fetch the raw bytes at a URL. Used for content the text path can't carry
    /// safely — notably gzipped XMLTV guides (`*.xml.gz`), which the caller gunzips.
    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, CoreError> {
        tracing::debug!(url = %redact::secrets(url), "GET bytes");
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CoreError::network("request", redact::secrets(&e.to_string())))?
            .error_for_status()
            .map_err(|e| CoreError::network("response status", redact::secrets(&e.to_string())))?;
        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| CoreError::network("body", redact::secrets(&e.to_string())))
    }
}

impl Transport for ReqwestTransport {
    fn get_text(&self, url: &str) -> impl Future<Output = Result<String, CoreError>> + Send {
        tracing::debug!(url = %redact::secrets(url), "xtream GET");
        // Build the request before the async block so the future owns no borrow
        // of `url` or `self` beyond the client clone.
        let request = self.client.get(url).send();
        async move {
            // reqwest's error Display includes the full URL, which carries the
            // account credentials — redact before the message escapes into logs/UI.
            let response = request
                .await
                .map_err(|e| CoreError::network("xtream request", redact::secrets(&e.to_string())))?
                .error_for_status()
                .map_err(|e| {
                    CoreError::network("xtream response status", redact::secrets(&e.to_string()))
                })?;
            response
                .text()
                .await
                .map_err(|e| CoreError::network("xtream body", redact::secrets(&e.to_string())))
        }
    }
}
