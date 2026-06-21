//! A captured log record.
//!
//! The shell captures `tracing` events as structured records (rather than pre-formatted
//! strings) so the UI can color by level, align columns, and indent wrapped lines. The
//! type lives here in `core` because it crosses the command boundary and both ends share
//! one definition. `message` is already credential-redacted by the shell.

use serde::{Deserialize, Serialize};

/// One captured log line. `level` is the lowercase severity (`error`…`trace`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLine {
    /// Local wall-clock time, formatted `HH:MM:SS.mmm`.
    pub time: String,
    /// Severity, lowercase: `error`, `warn`, `info`, `debug`, or `trace`.
    pub level: String,
    /// The event's target (usually the originating module path).
    pub target: String,
    /// The event message (credentials already redacted).
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trips() {
        let line = LogLine {
            time: "06:04:21.630".to_string(),
            level: "warn".to_string(),
            target: "cathode_lib::commands::sources".to_string(),
            message: "cache categories failed".to_string(),
        };
        let json = serde_json::to_string(&line).unwrap();
        assert_eq!(serde_json::from_str::<LogLine>(&json).unwrap(), line);
    }
}
