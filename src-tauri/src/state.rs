//! Shared application state managed by Tauri.
//!
//! Held behind `tauri::State` and injected into command handlers. Grows over
//! time (db handle, current source); today it holds the HTTP transport.

use crate::http::ReqwestTransport;

/// Process-wide state. Cheap to construct; the transport reuses one client.
#[derive(Debug, Default)]
pub struct AppState {
    pub transport: ReqwestTransport,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
