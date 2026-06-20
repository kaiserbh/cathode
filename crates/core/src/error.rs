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

    /// A network/transport failure. The concrete client (reqwest, in the shell)
    /// flattens its own error into a message so no native error type leaks here.
    #[error("network error during {context}: {message}")]
    Network {
        /// What we were doing (e.g. "live categories request").
        context: &'static str,
        /// The underlying transport error, stringified.
        message: String,
    },
}

impl CoreError {
    /// Convenience constructor for a JSON parse failure with context.
    pub fn json(context: &'static str, source: serde_json::Error) -> Self {
        Self::Json { context, source }
    }

    /// Convenience constructor for a transport failure with context.
    pub fn network(context: &'static str, message: impl Into<String>) -> Self {
        Self::Network {
            context,
            message: message.into(),
        }
    }
}
