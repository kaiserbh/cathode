//! Match programmes to a point in time, producing each channel's now/next.

use std::collections::HashMap;

use crate::epg::parse::EpgChannel;
use crate::model::{NowNext, Programme};

/// Quality suffixes stripped when normalizing a channel name, longest first so
/// `fhd`/`uhd` win over `hd`.
const QUALITY_SUFFIXES: [&str; 6] = ["uhd", "fhd", "hevc", "hd", "sd", "4k"];

/// Normalize a channel name for fuzzy matching: ascii-lowercase, alphanumerics
/// only, with a trailing quality tag removed. So `"BBC One HD"`, `"bbc  one"`, and
/// `"BBC One"` all collapse to `"bbcone"`.
pub fn normalize_name(name: &str) -> String {
    let mut s: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    for suffix in QUALITY_SUFFIXES {
        if s.len() > suffix.len() {
            if let Some(stripped) = s.strip_suffix(suffix) {
                s = stripped.to_string();
                break;
            }
        }
    }
    s
}

/// Build a `normalized-display-name -> channel id` index for name-based matching.
/// The first channel claiming a name wins.
pub fn name_index(channels: &[EpgChannel]) -> HashMap<String, String> {
    let mut index = HashMap::new();
    for channel in channels {
        for display_name in &channel.display_names {
            let key = normalize_name(display_name);
            if !key.is_empty() {
                index.entry(key).or_insert_with(|| channel.id.clone());
            }
        }
    }
    index
}

/// Build a `channel_id -> NowNext` map for the given instant (`now`, Unix seconds).
/// Only channels with a current or upcoming programme appear; `next` is the soonest
/// programme starting after `now`. Input order does not matter.
pub fn now_next(programmes: &[Programme], now: i64) -> HashMap<String, NowNext> {
    let mut map: HashMap<String, NowNext> = HashMap::new();
    for programme in programmes {
        if programme.start <= now && now < programme.stop {
            map.entry(programme.channel_id.clone()).or_default().now = Some(programme.clone());
        } else if programme.start > now {
            let entry = map.entry(programme.channel_id.clone()).or_default();
            if entry
                .next
                .as_ref()
                .is_none_or(|n| programme.start < n.start)
            {
                entry.next = Some(programme.clone());
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(channel: &str, title: &str, start: i64, stop: i64) -> Programme {
        Programme {
            channel_id: channel.to_string(),
            title: title.to_string(),
            description: None,
            start,
            stop,
        }
    }

    #[test]
    fn picks_current_and_soonest_upcoming() {
        // Deliberately out of order to prove ordering doesn't matter.
        let progs = vec![
            prog("a", "Later", 300, 400),
            prog("a", "Now", 100, 200),
            prog("a", "Next", 200, 300),
            prog("b", "Past", 0, 50),
        ];
        let map = now_next(&progs, 150);

        let a = &map["a"];
        assert_eq!(a.now.as_ref().unwrap().title, "Now");
        assert_eq!(a.next.as_ref().unwrap().title, "Next", "soonest after now");
        // Channel b only has a past programme, so it doesn't appear.
        assert!(!map.contains_key("b"));
    }

    #[test]
    fn upcoming_only_has_next_but_no_now() {
        let progs = vec![prog("a", "Soon", 500, 600)];
        let map = now_next(&progs, 100);
        assert!(map["a"].now.is_none());
        assert_eq!(map["a"].next.as_ref().unwrap().title, "Soon");
    }

    #[test]
    fn normalize_collapses_case_spacing_and_quality() {
        assert_eq!(normalize_name("BBC One HD"), "bbcone");
        assert_eq!(normalize_name("bbc  one"), "bbcone");
        assert_eq!(normalize_name("BBC One"), "bbcone");
        assert_eq!(normalize_name("Sky Sports 4K"), "skysports");
        // A bare quality-like name isn't stripped to nothing.
        assert_eq!(normalize_name("HD"), "hd");
    }

    #[test]
    fn name_index_maps_each_display_name_to_its_id() {
        let channels = vec![
            EpgChannel {
                id: "bbc1.uk".to_string(),
                display_names: vec!["BBC One".to_string(), "BBC1".to_string()],
            },
            EpgChannel {
                id: "itv.uk".to_string(),
                display_names: vec!["ITV".to_string()],
            },
        ];
        let index = name_index(&channels);
        assert_eq!(index.get("bbcone").map(String::as_str), Some("bbc1.uk"));
        assert_eq!(index.get("bbc1").map(String::as_str), Some("bbc1.uk"));
        assert_eq!(index.get("itv").map(String::as_str), Some("itv.uk"));
    }
}
