//! Pure, presentation-free filtering helpers for stream lists. Kept out of the
//! components so the logic is unit-testable and shared by the search dropdown and
//! the in-list quick filter.

use cathode_core::model::{Stream, StreamKind};

/// Filter streams by a case-insensitive substring of their `name`. An empty or
/// whitespace-only query returns every stream (so an empty filter box is a no-op).
pub fn filter_by_name(streams: &[Stream], query: &str) -> Vec<Stream> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return streams.to_vec();
    }
    streams
        .iter()
        .filter(|s| s.name.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}

/// Count streams per kind as `(live, vod, series)`, for the search dropdown's
/// per-kind filter chips.
pub fn count_by_kind(streams: &[Stream]) -> (usize, usize, usize) {
    let mut counts = (0, 0, 0);
    for s in streams {
        match s.kind {
            StreamKind::Live => counts.0 += 1,
            StreamKind::Vod => counts.1 += 1,
            StreamKind::Series => counts.2 += 1,
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Stream> {
        vec![
            Stream::new("src", "1", "BBC One HD", StreamKind::Live),
            Stream::new("src", "2", "Sky Sports", StreamKind::Live),
            Stream::new("src", "3", "The Matrix", StreamKind::Vod),
            Stream::new("src", "4", "Breaking Bad", StreamKind::Series),
        ]
    }

    #[test]
    fn filters_by_substring() {
        let got = filter_by_name(&sample(), "sky");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Sky Sports");
    }

    #[test]
    fn filter_is_case_insensitive() {
        // Query casing must not matter, in either direction.
        assert_eq!(filter_by_name(&sample(), "MATRIX").len(), 1);
        assert_eq!(filter_by_name(&sample(), "bbc one hd").len(), 1);
    }

    #[test]
    fn empty_query_passes_everything_through() {
        assert_eq!(filter_by_name(&sample(), "").len(), 4);
        assert_eq!(filter_by_name(&sample(), "   ").len(), 4);
    }

    #[test]
    fn no_match_yields_empty() {
        assert!(filter_by_name(&sample(), "nonexistent").is_empty());
    }

    #[test]
    fn counts_per_kind() {
        assert_eq!(count_by_kind(&sample()), (2, 1, 1));
        assert_eq!(count_by_kind(&[]), (0, 0, 0));
    }
}
