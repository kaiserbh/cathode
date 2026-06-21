//! User-controlled feature settings.
//!
//! Cathode forces nothing: every optional feature can be turned off. These flags
//! are persisted (in the catalog) and cross the command boundary, so they live in
//! `core` and both ends share one definition. `#[serde(default)]` means older or
//! partial stored JSON still deserializes, filling any missing field from `Default`.

use serde::{Deserialize, Serialize};

/// Persisted feature toggles. Both default on; the user opts out, not in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Whether the favorites feature (stars + the Favorites tab) is available.
    pub favorites_enabled: bool,
    /// Whether plays are recorded to watch history.
    pub history_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            favorites_enabled: true,
            history_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_on() {
        let s = Settings::default();
        assert!(s.favorites_enabled);
        assert!(s.history_enabled);
    }

    #[test]
    fn json_round_trips() {
        let s = Settings {
            favorites_enabled: false,
            history_enabled: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
    }

    #[test]
    fn missing_fields_fall_back_to_default() {
        // Forward compatibility: an empty/partial object fills from Default.
        assert_eq!(
            serde_json::from_str::<Settings>("{}").unwrap(),
            Settings::default()
        );
        let partial: Settings = serde_json::from_str(r#"{"history_enabled":false}"#).unwrap();
        assert!(partial.favorites_enabled);
        assert!(!partial.history_enabled);
    }
}
