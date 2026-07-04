//! User-controlled feature settings.
//!
//! Cathode forces nothing: every optional feature can be turned off. These flags
//! are persisted (in the catalog) and cross the command boundary, so they live in
//! `core` and both ends share one definition. `#[serde(default)]` means older or
//! partial stored JSON still deserializes, filling any missing field from `Default`.

use serde::{Deserialize, Serialize};

use super::LogLevel;

/// Convert a user-facing volume slider position (0–100) into the linear
/// amplitude gain mpv expects. mpv's `volume` is a linear multiplier on the
/// audio samples, but loudness perception is non-linear, so a 1:1 mapping makes
/// the low end feel almost silent. A square-root taper lifts the quiet end so
/// equal slider steps feel more even. Endpoints are preserved: 0 → 0, 100 → 100.
pub fn volume_to_mpv_gain(slider: u8) -> f64 {
    let s = (slider.min(100) as f64) / 100.0;
    s.sqrt() * 100.0
}

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
    /// Playback volume, 0–100.
    pub volume: u8,
    /// Whether playback is muted.
    pub muted: bool,
    /// Debug logging verbosity. `Off` (the default) disables capture entirely.
    pub log_level: LogLevel,
    /// Whether the one-time "Press ? for shortcuts" hint has been shown. Set the
    /// first time the player opens so the hint never repeats.
    pub shortcuts_hint_seen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            favorites_enabled: true,
            history_enabled: true,
            epg_enabled: true,
            channel_view: ChannelView::Grid,
            volume: 100,
            muted: false,
            log_level: LogLevel::Off,
            shortcuts_hint_seen: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_taper_preserves_endpoints() {
        assert_eq!(volume_to_mpv_gain(0), 0.0);
        assert_eq!(volume_to_mpv_gain(100), 100.0);
        // Out-of-range clamps to full, never amplifies past unity.
        assert_eq!(volume_to_mpv_gain(200), 100.0);
    }

    #[test]
    fn volume_taper_boosts_the_low_end() {
        // The whole point: equal-position low values get lifted well above their
        // raw number so they are actually audible.
        assert!((volume_to_mpv_gain(30) - 54.77).abs() < 0.01);
        assert!(volume_to_mpv_gain(30) > 30.0);
        assert!(volume_to_mpv_gain(15) > 15.0);
    }

    #[test]
    fn volume_taper_is_monotonic() {
        let mut prev = volume_to_mpv_gain(0);
        for s in 1..=100u8 {
            let g = volume_to_mpv_gain(s);
            assert!(g > prev, "gain must increase: {s} gave {g} <= {prev}");
            prev = g;
        }
    }

    #[test]
    fn defaults_are_on_and_grid() {
        let s = Settings::default();
        assert!(s.favorites_enabled);
        assert!(s.history_enabled);
        assert!(s.epg_enabled);
        assert_eq!(s.channel_view, ChannelView::Grid);
        assert_eq!(s.volume, 100);
        assert!(!s.muted);
        assert_eq!(s.log_level, LogLevel::Off);
        assert!(!s.shortcuts_hint_seen);
    }

    #[test]
    fn json_round_trips() {
        let s = Settings {
            favorites_enabled: false,
            history_enabled: true,
            epg_enabled: false,
            channel_view: ChannelView::List,
            volume: 42,
            muted: true,
            log_level: LogLevel::Debug,
            shortcuts_hint_seen: true,
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
        assert_eq!(partial.volume, 100);
        assert!(!partial.muted);
        assert_eq!(partial.log_level, LogLevel::Off);
        assert!(!partial.shortcuts_hint_seen);
    }
}
