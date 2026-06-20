//! Shared application state managed by Tauri.
//!
//! Held behind `tauri::State` and injected into command handlers. Today it holds
//! the HTTP transport and the local catalog handle.

use std::sync::Arc;

use crate::catalog_sqlite::SqliteCatalog;
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

/// The local catalog, or `None` if it failed to open. Wrapping it in an `Option`
/// (rather than refusing to manage it) keeps network browsing working when the
/// database is unavailable; cache reads just return empty and writes are skipped.
/// The `Arc` lets command handlers move a clone into `spawn_blocking`.
#[derive(Clone, Default)]
pub struct CatalogState(pub Option<Arc<SqliteCatalog>>);
