//! Combine and trim guides.
//!
//! A playlist can declare several XMLTV sources (and some are huge), so the shell
//! parses each into a [`Guide`], filters it down to the channels the playlist
//! actually carries, and merges the results into one. These helpers are pure and
//! WASM-safe; fetching/decompression lives in the shell.

use std::collections::HashSet;

use crate::epg::parse::Guide;
use crate::epg::r#match::normalize_name;

/// Merge several guides into one: concatenate programmes, and concatenate channels
/// de-duplicated by id (the first occurrence of an id wins).
pub fn merge_guides(guides: Vec<Guide>) -> Guide {
    let mut merged = Guide::default();
    let mut seen_channels = HashSet::new();
    for guide in guides {
        merged.programmes.extend(guide.programmes);
        for channel in guide.channels {
            if seen_channels.insert(channel.id.clone()) {
                merged.channels.push(channel);
            }
        }
    }
    merged
}

/// Drop everything a playlist can't use, so a huge guide collapses to just the
/// relevant channels. A channel is kept when its id is one of `wanted_ids` (the
/// playlist's `tvg-id`s) or one of its display-names normalizes into `wanted_names`
/// (so the name-based fallback still resolves). Programmes are kept when their
/// channel id survives.
pub fn filter_to_channels(
    guide: &mut Guide,
    wanted_ids: &HashSet<String>,
    wanted_names: &HashSet<String>,
) {
    let mut keep: HashSet<String> = wanted_ids.clone();
    for channel in &guide.channels {
        if channel
            .display_names
            .iter()
            .any(|n| wanted_names.contains(&normalize_name(n)))
        {
            keep.insert(channel.id.clone());
        }
    }
    guide.programmes.retain(|p| keep.contains(&p.channel_id));
    guide.channels.retain(|c| keep.contains(&c.id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epg::parse::EpgChannel;
    use crate::model::Programme;

    fn prog(channel: &str, title: &str) -> Programme {
        Programme {
            channel_id: channel.to_string(),
            title: title.to_string(),
            description: None,
            start: 0,
            stop: 100,
        }
    }

    fn channel(id: &str, names: &[&str]) -> EpgChannel {
        EpgChannel {
            id: id.to_string(),
            display_names: names.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn merge_concatenates_programmes_and_dedupes_channels() {
        let a = Guide {
            programmes: vec![prog("x", "A")],
            channels: vec![channel("x", &["X"])],
        };
        let b = Guide {
            programmes: vec![prog("y", "B")],
            // "x" repeats across files; the first wins.
            channels: vec![channel("x", &["X dup"]), channel("y", &["Y"])],
        };
        let merged = merge_guides(vec![a, b]);
        assert_eq!(merged.programmes.len(), 2);
        let ids: Vec<_> = merged.channels.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["x", "y"]);
    }

    #[test]
    fn filter_keeps_wanted_ids_and_name_matches_only() {
        let mut guide = Guide {
            programmes: vec![
                prog("bbc1.uk", "by id"),
                prog("itv-xyz", "by name"),
                prog("noise.tv", "unwanted"),
            ],
            channels: vec![
                channel("bbc1.uk", &["BBC One"]),
                channel("itv-xyz", &["ITV"]),
                channel("noise.tv", &["Some Shopping Channel"]),
            ],
        };
        let wanted_ids = HashSet::from(["bbc1.uk".to_string()]);
        // The playlist has an "ITV HD" channel with no tvg-id -> matches by name.
        let wanted_names = HashSet::from([normalize_name("ITV HD")]);

        filter_to_channels(&mut guide, &wanted_ids, &wanted_names);

        let kept: Vec<_> = guide.programmes.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(kept, vec!["by id", "by name"]);
        let chans: Vec<_> = guide.channels.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(chans, vec!["bbc1.uk", "itv-xyz"]);
    }
}
