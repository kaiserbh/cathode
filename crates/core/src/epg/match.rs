//! Match programmes to a point in time, producing each channel's now/next.

use std::collections::HashMap;

use crate::model::{NowNext, Programme};

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
}
