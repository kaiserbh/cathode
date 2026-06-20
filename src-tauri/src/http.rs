//! The concrete HTTP transport: reqwest behind `cathode_core`'s `Transport`.
//!
//! This is the only place reqwest is used. It flattens reqwest errors into
//! `CoreError::network` so no native error type crosses back into `core`.

use cathode_core::error::CoreError;
use cathode_core::transport::Transport;
use std::future::Future;

/// A `Transport` backed by a shared reqwest client (connection pool reused).
#[derive(Debug, Clone, Default)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transport for ReqwestTransport {
    fn get_text(&self, url: &str) -> impl Future<Output = Result<String, CoreError>> + Send {
        // Build the request before the async block so the future owns no borrow
        // of `url` or `self` beyond the client clone.
        let request = self.client.get(url).send();
        async move {
            let response = request
                .await
                .map_err(|e| CoreError::network("xtream request", e.to_string()))?
                .error_for_status()
                .map_err(|e| CoreError::network("xtream response status", e.to_string()))?;
            response
                .text()
                .await
                .map_err(|e| CoreError::network("xtream body", e.to_string()))
        }
    }
}
