//! Core error types.
//!
//! Errors are typed, never stringly. `CoreError` is the rich internal error (it
//! holds the source error for logging). `AppError` is the flattened, serializable
//! shape that crosses the command boundary, so the UI decodes a stable `code`
//! plus a message rather than a string blob. Both ends share this one definition.

use serde::{Deserialize, Serialize};
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

    /// A local-storage failure. The concrete catalog (rusqlite, in the shell)
    /// flattens its own error into a message so no native error type leaks here.
    #[error("storage error during {context}: {message}")]
    Storage {
        /// What we were doing (e.g. "load sources").
        context: &'static str,
        /// The underlying storage error, stringified.
        message: String,
    },

    /// An XML parse failure (XMLTV guide). The underlying parser error is
    /// flattened to a message.
    #[error("failed to parse {context}: {message}")]
    Xml {
        /// What we were parsing (e.g. "xmltv guide").
        context: &'static str,
        /// The underlying parse error, stringified.
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

    /// Convenience constructor for a storage failure with context.
    pub fn storage(context: &'static str, message: impl Into<String>) -> Self {
        Self::Storage {
            context,
            message: message.into(),
        }
    }

    /// Convenience constructor for an XML parse failure with context.
    pub fn xml(context: &'static str, message: impl Into<String>) -> Self {
        Self::Xml {
            context,
            message: message.into(),
        }
    }

    /// Stable, machine-readable category for this error.
    fn code(&self) -> &'static str {
        match self {
            CoreError::Json { .. } => "parse",
            CoreError::Network { .. } => "network",
            CoreError::Storage { .. } => "storage",
            CoreError::Xml { .. } => "parse",
        }
    }
}

/// A structured, serializable error for the command boundary.
///
/// Lives in `core` so both the shell (which produces it from a [`CoreError`])
/// and the frontend (which decodes it) share one definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppError {
    /// Stable category (e.g. "parse", "network").
    pub code: String,
    /// Human-readable detail for display/logging.
    pub message: String,
}

impl From<CoreError> for AppError {
    fn from(err: CoreError) -> Self {
        AppError {
            code: err.code().to_string(),
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_json_error_to_parse_code() {
        let core = CoreError::json(
            "live streams",
            serde_json::from_str::<serde_json::Value>("nope").unwrap_err(),
        );
        let app: AppError = core.into();
        assert_eq!(app.code, "parse");
        assert!(app.message.contains("live streams"));
    }

    #[test]
    fn maps_network_error_to_network_code() {
        let core = CoreError::network("live categories request", "connection refused");
        let app: AppError = core.into();
        assert_eq!(app.code, "network");
        assert!(app.message.contains("connection refused"));
    }

    #[test]
    fn maps_storage_error_to_storage_code() {
        let core = CoreError::storage("load sources", "database is locked");
        let app: AppError = core.into();
        assert_eq!(app.code, "storage");
        assert!(app.message.contains("load sources"));
        assert!(app.message.contains("database is locked"));
    }

    #[test]
    fn maps_xml_error_to_parse_code() {
        let core = CoreError::xml("xmltv guide", "unexpected eof");
        let app: AppError = core.into();
        assert_eq!(app.code, "parse");
        assert!(app.message.contains("xmltv guide"));
    }

    #[test]
    fn app_error_round_trips_for_the_frontend() {
        // The UI decodes this from the invoke rejection, so it must round-trip.
        let app = AppError {
            code: "network".to_string(),
            message: "boom".to_string(),
        };
        let json = serde_json::to_string(&app).unwrap();
        let back: AppError = serde_json::from_str(&json).unwrap();
        assert_eq!(app, back);
    }
}
