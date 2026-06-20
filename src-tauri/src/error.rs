//! Frontend-facing error type.
//!
//! Commands map `cathode_core::error::CoreError` to this serializable shape so
//! the UI receives structured failures (a stable `code` plus a human message),
//! never a stringly-typed blob.

use cathode_core::error::CoreError;
use serde::Serialize;

/// A structured error returned across the command boundary.
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    /// Stable, machine-readable category (e.g. "parse", "network").
    pub code: String,
    /// Human-readable detail for display/logging.
    pub message: String,
}

impl From<CoreError> for AppError {
    fn from(err: CoreError) -> Self {
        let code = match &err {
            CoreError::Json { .. } => "parse",
            CoreError::Network { .. } => "network",
        };
        AppError {
            code: code.to_string(),
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
}
