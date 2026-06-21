//! User-controlled feature settings.
//!
//! Cathode forces nothing: every optional feature can be turned off. These flags
//! are persisted (in the catalog) and cross the command boundary, so they live in
//! `core` and both ends share one definition. `#[serde(default)]` means older or
//! partial stored JSON still deserializes, filling any missing field from `Default`.

use serde::{Deserialize, Serialize};

/// How the channel list is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelView {
    /// A responsive grid of logo cards.
    #[default]
    Grid,
    /// A compact one-per-row list with more room for guide text.
    List,
    /// A channels × time timeline (the EPG guide).
    Guide,
}

/// Persisted feature toggles. Defaults are on / Grid; the user opts out, not in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Whether the favorites feature (stars + the Favorites tab) is available.
    pub favorites_enabled: bool,
    /// Whether plays are recorded to watch history.
    pub history_enabled: bool,
    /// Whether the EPG (now/next guide) is fetched and shown.
    pub epg_enabled: bool,
    /// How the channel list is displayed.
    pub channel_view: ChannelView,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            favorites_enabled: true,
            history_enabled: true,
            epg_enabled: true,
            channel_view: ChannelView::Grid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_on_and_grid() {
        let s = Settings::default();
        assert!(s.favorites_enabled);
        assert!(s.history_enabled);
        assert!(s.epg_enabled);
        assert_eq!(s.channel_view, ChannelView::Grid);
    }

    #[test]
    fn json_round_trips() {
        let s = Settings {
            favorites_enabled: false,
            history_enabled: true,
            epg_enabled: false,
            channel_view: ChannelView::List,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
    }

    #[test]
    fn channel_view_serializes_lowercase() {
        // The stored JSON and any UI comparisons depend on this spelling.
        assert_eq!(
            serde_json::to_string(&ChannelView::List).unwrap(),
            "\"list\""
        );
    }

    #[test]
    fn missing_fields_fall_back_to_default() {
        // Forward compatibility: an empty/partial object fills from Default, so an
        // older stored Settings (without epg_enabled / channel_view) still loads.
        assert_eq!(
            serde_json::from_str::<Settings>("{}").unwrap(),
            Settings::default()
        );
        let partial: Settings = serde_json::from_str(r#"{"history_enabled":false}"#).unwrap();
        assert!(partial.favorites_enabled);
        assert!(!partial.history_enabled);
        assert!(partial.epg_enabled);
        assert_eq!(partial.channel_view, ChannelView::Grid);
    }
}
