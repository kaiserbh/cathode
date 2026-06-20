//! Core error type.
//!
//! Errors are typed, never stringly. The Tauri shell maps `CoreError` to a
//! serializable `AppError` for the frontend; within `core`
//! we keep the rich source error for logging.

use thiserror::Error;

/// Anything that can go wrong inside `core`.
#[derive(Debug, Error)]
pub enum CoreError {
    /// JSON from a provider did not match the expected shape.
    #[error("failed to parse {context}: {source}")]
    Json {
        /// What we were trying to parse, for the log line (e.g. "live streams").
        context: &'static str,
        #[source]
        source: serde_json::Error,
    },
}

impl CoreError {
    /// Convenience constructor for a JSON parse failure with context.
    pub fn json(context: &'static str, source: serde_json::Error) -> Self {
        Self::Json { context, source }
    }
}
