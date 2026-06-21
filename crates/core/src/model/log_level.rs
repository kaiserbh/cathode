//! The user-selected logging verbosity.
//!
//! A single control doubles as the on/off switch: `Off` disables capture entirely
//! (no overhead), and the other variants pick how verbose the in-memory log buffer is.
//! Kept as a plain enum here in `core` (no `tracing` dependency) so it stays WASM-safe
//! and both ends share one definition; the shell maps it onto a tracing level filter.

use serde::{Deserialize, Serialize};

/// Logging verbosity, persisted in [`super::Settings`]. Defaults to `Off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Logging disabled; nothing is captured.
    #[default]
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off() {
        assert_eq!(LogLevel::default(), LogLevel::Off);
    }

    #[test]
    fn serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&LogLevel::Debug).unwrap(),
            "\"debug\""
        );
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"off\"").unwrap(),
            LogLevel::Off
        );
    }
}
