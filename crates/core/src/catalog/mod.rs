//! The local catalog: persisted sources plus a cache of their categories and
//! streams.
//!
//! This is the trait only — pure Rust with no storage-engine types in its
//! signatures, so the rusqlite implementation stays in the Tauri shell and the
//! choice never leaks into `core` (and `core` stays WASM-safe). Methods are
//! synchronous; the shell runs the heavy ones on a blocking thread.
//!
//! Rows are keyed by `source_id` (and, for streams, the category) so the cache is
//! inherently multi-source: several Xtream accounts coexist without colliding, and
//! stream rows key on the stable [`crate::model::StreamId`] so a re-sync overwrites
//! cleanly rather than duplicating.

use crate::error::CoreError;
use crate::model::{Category, Stream, StreamKind};

/// A persisted source, opaque to the catalog: `payload` is a serialized,
/// source-kind-specific credential blob (for Xtream, JSON of `XtreamCredentials`).
/// Keeping it opaque lets the catalog stay source-agnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecord {
    /// Stable account id (e.g. `XtreamSource::source_id()`).
    pub id: String,
    /// Source kind discriminator (e.g. `"xtream"`).
    pub kind: String,
    /// Serialized credentials for this source.
    pub payload: String,
}

/// Persisted sources and their cached catalog. See the module docs for the keying
/// rules. Every method may fail with [`CoreError::Storage`].
pub trait Catalog {
    /// Insert or update a source, marking it the most recently used.
    fn upsert_source(&self, source: &SourceRecord) -> Result<(), CoreError>;

    /// All known sources, most-recently-used first.
    fn sources(&self) -> Result<Vec<SourceRecord>, CoreError>;

    /// Remove a source and everything stored under it (cached categories/streams,
    /// favorites, and history).
    fn delete_source(&self, id: &str) -> Result<(), CoreError>;

    /// Replace the cached categories for a `(source, kind)` with the given set.
    /// Content kinds (Live/VOD/Series) are isolated: their category ids may collide.
    fn replace_categories(
        &self,
        source_id: &str,
        kind: StreamKind,
        categories: &[Category],
    ) -> Result<(), CoreError>;

    /// The cached categories for a `(source, kind)` (empty if none cached).
    fn categories(&self, source_id: &str, kind: StreamKind) -> Result<Vec<Category>, CoreError>;

    /// Replace the cached streams for a `(source, kind, category)` with the given set.
    fn replace_streams(
        &self,
        source_id: &str,
        kind: StreamKind,
        category_id: &str,
        streams: &[Stream],
    ) -> Result<(), CoreError>;

    /// The cached streams for a `(source, kind, category)` (empty if none cached).
    fn streams(
        &self,
        source_id: &str,
        kind: StreamKind,
        category_id: &str,
    ) -> Result<Vec<Stream>, CoreError>;

    /// Search a source's cached streams (every kind and category) by name,
    /// case-insensitive, most relevant first. Covers whatever has been synced.
    fn search_streams(&self, source_id: &str, query: &str) -> Result<Vec<Stream>, CoreError>;

    /// A persisted app setting value, or `None` if unset. Settings are a generic
    /// key/value store; callers (de)serialize structured values themselves.
    fn get_setting(&self, key: &str) -> Result<Option<String>, CoreError>;

    /// Set (or overwrite) a persisted app setting value.
    fn set_setting(&self, key: &str, value: &str) -> Result<(), CoreError>;

    /// Mark a stream as a favorite of a source (no-op if already a favorite).
    fn add_favorite(&self, source_id: &str, stream: &Stream) -> Result<(), CoreError>;

    /// Remove a favorite by its stable stream id.
    fn remove_favorite(&self, source_id: &str, stream_id: &str) -> Result<(), CoreError>;

    /// A source's favorites, most-recently-added first.
    fn favorites(&self, source_id: &str) -> Result<Vec<Stream>, CoreError>;

    /// Record that a stream was watched now (de-duped per stream: re-watching moves
    /// it to the front rather than adding a second entry).
    fn record_watch(&self, source_id: &str, stream: &Stream) -> Result<(), CoreError>;

    /// A source's watch history, most-recently-watched first.
    fn history(&self, source_id: &str) -> Result<Vec<Stream>, CoreError>;

    /// Erase all watch history (every source). A privacy action.
    fn clear_history(&self) -> Result<(), CoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::category::CategoryId;
    use crate::model::StreamKind;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// In-memory `Catalog` for exercising the contract. MRU order is tracked with
    /// a monotonic counter so it is deterministic without a real clock.
    #[derive(Default)]
    struct FakeCatalog {
        seq: RefCell<u64>,
        sources: RefCell<HashMap<String, (SourceRecord, u64)>>,
        categories: RefCell<HashMap<(String, StreamKind), Vec<Category>>>,
        streams: RefCell<HashMap<(String, StreamKind, String), Vec<Stream>>>,
        settings: RefCell<HashMap<String, String>>,
        // Per source: each stream tagged with the seq at which it was added/watched,
        // so we can order most-recent-first deterministically.
        favorites: RefCell<HashMap<String, Vec<(Stream, u64)>>>,
        history: RefCell<HashMap<String, Vec<(Stream, u64)>>>,
    }

    impl FakeCatalog {
        fn next_seq(&self) -> u64 {
            let mut seq = self.seq.borrow_mut();
            *seq += 1;
            *seq
        }
    }

    /// Upsert a stream into a per-source recency list (newest seq wins), de-duping
    /// by stable id; returns the list sorted most-recent-first.
    fn recency_upsert(list: &mut Vec<(Stream, u64)>, stream: &Stream, seq: u64) {
        list.retain(|(s, _)| s.id != stream.id);
        list.push((stream.clone(), seq));
    }

    fn recency_sorted(list: &[(Stream, u64)]) -> Vec<Stream> {
        let mut rows = list.to_vec();
        rows.sort_by_key(|(_, seq)| std::cmp::Reverse(*seq));
        rows.into_iter().map(|(s, _)| s).collect()
    }

    impl Catalog for FakeCatalog {
        fn upsert_source(&self, source: &SourceRecord) -> Result<(), CoreError> {
            let mut seq = self.seq.borrow_mut();
            *seq += 1;
            self.sources
                .borrow_mut()
                .insert(source.id.clone(), (source.clone(), *seq));
            Ok(())
        }

        fn sources(&self) -> Result<Vec<SourceRecord>, CoreError> {
            let mut rows: Vec<_> = self.sources.borrow().values().cloned().collect();
            rows.sort_by_key(|row| std::cmp::Reverse(row.1));
            Ok(rows.into_iter().map(|(record, _)| record).collect())
        }

        fn delete_source(&self, id: &str) -> Result<(), CoreError> {
            self.sources.borrow_mut().remove(id);
            self.categories.borrow_mut().retain(|(sid, _), _| sid != id);
            self.streams.borrow_mut().retain(|(sid, _, _), _| sid != id);
            self.favorites.borrow_mut().remove(id);
            self.history.borrow_mut().remove(id);
            Ok(())
        }

        fn replace_categories(
            &self,
            source_id: &str,
            kind: StreamKind,
            categories: &[Category],
        ) -> Result<(), CoreError> {
            self.categories
                .borrow_mut()
                .insert((source_id.to_string(), kind), categories.to_vec());
            Ok(())
        }

        fn categories(
            &self,
            source_id: &str,
            kind: StreamKind,
        ) -> Result<Vec<Category>, CoreError> {
            Ok(self
                .categories
                .borrow()
                .get(&(source_id.to_string(), kind))
                .cloned()
                .unwrap_or_default())
        }

        fn replace_streams(
            &self,
            source_id: &str,
            kind: StreamKind,
            category_id: &str,
            streams: &[Stream],
        ) -> Result<(), CoreError> {
            self.streams.borrow_mut().insert(
                (source_id.to_string(), kind, category_id.to_string()),
                streams.to_vec(),
            );
            Ok(())
        }

        fn streams(
            &self,
            source_id: &str,
            kind: StreamKind,
            category_id: &str,
        ) -> Result<Vec<Stream>, CoreError> {
            Ok(self
                .streams
                .borrow()
                .get(&(source_id.to_string(), kind, category_id.to_string()))
                .cloned()
                .unwrap_or_default())
        }

        fn search_streams(&self, source_id: &str, query: &str) -> Result<Vec<Stream>, CoreError> {
            let needle = query.to_lowercase();
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for ((sid, _, _), streams) in self.streams.borrow().iter() {
                if sid != source_id {
                    continue;
                }
                for s in streams {
                    if s.name.to_lowercase().contains(&needle) && seen.insert(s.id.clone()) {
                        out.push(s.clone());
                    }
                }
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        }

        fn get_setting(&self, key: &str) -> Result<Option<String>, CoreError> {
            Ok(self.settings.borrow().get(key).cloned())
        }

        fn set_setting(&self, key: &str, value: &str) -> Result<(), CoreError> {
            self.settings
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn add_favorite(&self, source_id: &str, stream: &Stream) -> Result<(), CoreError> {
            let seq = self.next_seq();
            let mut favorites = self.favorites.borrow_mut();
            recency_upsert(
                favorites.entry(source_id.to_string()).or_default(),
                stream,
                seq,
            );
            Ok(())
        }

        fn remove_favorite(&self, source_id: &str, stream_id: &str) -> Result<(), CoreError> {
            if let Some(list) = self.favorites.borrow_mut().get_mut(source_id) {
                list.retain(|(s, _)| s.id.0 != stream_id);
            }
            Ok(())
        }

        fn favorites(&self, source_id: &str) -> Result<Vec<Stream>, CoreError> {
            Ok(self
                .favorites
                .borrow()
                .get(source_id)
                .map(|list| recency_sorted(list))
                .unwrap_or_default())
        }

        fn record_watch(&self, source_id: &str, stream: &Stream) -> Result<(), CoreError> {
            let seq = self.next_seq();
            let mut history = self.history.borrow_mut();
            recency_upsert(
                history.entry(source_id.to_string()).or_default(),
                stream,
                seq,
            );
            Ok(())
        }

        fn history(&self, source_id: &str) -> Result<Vec<Stream>, CoreError> {
            Ok(self
                .history
                .borrow()
                .get(source_id)
                .map(|list| recency_sorted(list))
                .unwrap_or_default())
        }

        fn clear_history(&self) -> Result<(), CoreError> {
            self.history.borrow_mut().clear();
            Ok(())
        }
    }

    fn record(id: &str) -> SourceRecord {
        SourceRecord {
            id: id.to_string(),
            kind: "xtream".to_string(),
            payload: format!("{{\"id\":\"{id}\"}}"),
        }
    }

    #[test]
    fn sources_are_returned_most_recently_used_first() {
        let cat = FakeCatalog::default();
        cat.upsert_source(&record("a")).unwrap();
        cat.upsert_source(&record("b")).unwrap();
        // Re-using "a" must float it back to the front.
        cat.upsert_source(&record("a")).unwrap();

        let ids: Vec<_> = cat.sources().unwrap().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn replace_categories_overwrites_per_source() {
        let cat = FakeCatalog::default();
        let sports = Category {
            id: CategoryId("1".to_string()),
            name: "Sports".to_string(),
        };
        let news = Category {
            id: CategoryId("2".to_string()),
            name: "News".to_string(),
        };
        cat.replace_categories("src-a", StreamKind::Live, &[sports.clone(), news.clone()])
            .unwrap();
        assert_eq!(
            cat.categories("src-a", StreamKind::Live).unwrap(),
            vec![sports, news.clone()]
        );

        // A re-sync replaces, not appends.
        cat.replace_categories("src-a", StreamKind::Live, std::slice::from_ref(&news))
            .unwrap();
        assert_eq!(
            cat.categories("src-a", StreamKind::Live).unwrap(),
            vec![news]
        );
        // Another source is untouched and empty.
        assert!(cat
            .categories("src-b", StreamKind::Live)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn streams_are_isolated_by_source_and_category() {
        let cat = FakeCatalog::default();
        let s1 = Stream::new("src-a", "1", "One", StreamKind::Live);
        let s2 = Stream::new("src-a", "2", "Two", StreamKind::Live);
        cat.replace_streams(
            "src-a",
            StreamKind::Live,
            "sports",
            &[s1.clone(), s2.clone()],
        )
        .unwrap();

        assert_eq!(
            cat.streams("src-a", StreamKind::Live, "sports").unwrap(),
            vec![s1, s2]
        );
        // Different category, and different source, are separate buckets.
        assert!(cat
            .streams("src-a", StreamKind::Live, "news")
            .unwrap()
            .is_empty());
        assert!(cat
            .streams("src-b", StreamKind::Live, "sports")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn cache_is_isolated_by_kind() {
        // Live and VOD can share a category id; they must not collide.
        let cat = FakeCatalog::default();
        let live = Stream::new("src-a", "1", "Live One", StreamKind::Live);
        let movie = Stream::new("src-a", "1", "Movie One", StreamKind::Vod);
        cat.replace_streams("src-a", StreamKind::Live, "5", std::slice::from_ref(&live))
            .unwrap();
        cat.replace_streams("src-a", StreamKind::Vod, "5", std::slice::from_ref(&movie))
            .unwrap();

        assert_eq!(
            cat.streams("src-a", StreamKind::Live, "5").unwrap(),
            vec![live]
        );
        assert_eq!(
            cat.streams("src-a", StreamKind::Vod, "5").unwrap(),
            vec![movie]
        );
    }

    #[test]
    fn search_matches_across_categories_and_kinds() {
        let cat = FakeCatalog::default();
        let sky_live = Stream::new("src-a", "1", "Sky Sports", StreamKind::Live);
        let skyfall = Stream::new("src-a", "2", "Skyfall", StreamKind::Vod);
        let other = Stream::new("src-a", "3", "BBC One", StreamKind::Live);
        cat.replace_streams(
            "src-a",
            StreamKind::Live,
            "sports",
            &[sky_live.clone(), other],
        )
        .unwrap();
        cat.replace_streams(
            "src-a",
            StreamKind::Vod,
            "films",
            std::slice::from_ref(&skyfall),
        )
        .unwrap();

        let names: Vec<_> = cat
            .search_streams("src-a", "sky")
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, vec!["Sky Sports".to_string(), "Skyfall".to_string()]);
        // Scoped to the source.
        assert!(cat.search_streams("src-b", "sky").unwrap().is_empty());
    }

    #[test]
    fn delete_source_clears_its_cache() {
        let cat = FakeCatalog::default();
        cat.upsert_source(&record("a")).unwrap();
        cat.replace_categories(
            "a",
            StreamKind::Live,
            &[Category {
                id: CategoryId("1".to_string()),
                name: "Sports".to_string(),
            }],
        )
        .unwrap();
        cat.replace_streams(
            "a",
            StreamKind::Live,
            "1",
            &[Stream::new("a", "9", "Nine", StreamKind::Live)],
        )
        .unwrap();
        cat.add_favorite("a", &Stream::new("a", "9", "Nine", StreamKind::Live))
            .unwrap();
        cat.record_watch("a", &Stream::new("a", "9", "Nine", StreamKind::Live))
            .unwrap();

        cat.delete_source("a").unwrap();

        assert!(cat.sources().unwrap().is_empty());
        assert!(cat.categories("a", StreamKind::Live).unwrap().is_empty());
        assert!(cat.streams("a", StreamKind::Live, "1").unwrap().is_empty());
        assert!(cat.favorites("a").unwrap().is_empty());
        assert!(cat.history("a").unwrap().is_empty());
    }

    #[test]
    fn settings_get_set_round_trip() {
        let cat = FakeCatalog::default();
        assert_eq!(cat.get_setting("settings").unwrap(), None);
        cat.set_setting("settings", "{\"history_enabled\":false}")
            .unwrap();
        assert_eq!(
            cat.get_setting("settings").unwrap(),
            Some("{\"history_enabled\":false}".to_string())
        );
    }

    #[test]
    fn favorites_add_remove_and_order() {
        let cat = FakeCatalog::default();
        let one = Stream::new("a", "1", "One", StreamKind::Live);
        let two = Stream::new("a", "2", "Two", StreamKind::Live);
        cat.add_favorite("a", &one).unwrap();
        cat.add_favorite("a", &two).unwrap();
        // Most-recently-added first.
        let ids: Vec<_> = cat
            .favorites("a")
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec![two.id.clone(), one.id.clone()]);
        // Adding again de-dupes (no second entry).
        cat.add_favorite("a", &one).unwrap();
        assert_eq!(cat.favorites("a").unwrap().len(), 2);

        cat.remove_favorite("a", &two.id.0).unwrap();
        assert_eq!(cat.favorites("a").unwrap(), vec![one]);
        // Another source is isolated.
        assert!(cat.favorites("b").unwrap().is_empty());
    }

    #[test]
    fn history_records_dedupes_orders_and_clears() {
        let cat = FakeCatalog::default();
        let one = Stream::new("a", "1", "One", StreamKind::Live);
        let two = Stream::new("a", "2", "Two", StreamKind::Live);
        cat.record_watch("a", &one).unwrap();
        cat.record_watch("a", &two).unwrap();
        // Re-watching "one" moves it to the front without duplicating.
        cat.record_watch("a", &one).unwrap();
        let ids: Vec<_> = cat
            .history("a")
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec![one.id.clone(), two.id.clone()]);

        cat.clear_history().unwrap();
        assert!(cat.history("a").unwrap().is_empty());
    }
}
