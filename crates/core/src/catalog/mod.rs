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
use crate::model::{Category, Stream};

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

    /// Remove a source and any categories/streams cached under it.
    fn delete_source(&self, id: &str) -> Result<(), CoreError>;

    /// Replace the cached categories for a source with the given set.
    fn replace_categories(&self, source_id: &str, categories: &[Category])
        -> Result<(), CoreError>;

    /// The cached categories for a source (empty if none cached).
    fn categories(&self, source_id: &str) -> Result<Vec<Category>, CoreError>;

    /// Replace the cached streams for a `(source, category)` with the given set.
    fn replace_streams(
        &self,
        source_id: &str,
        category_id: &str,
        streams: &[Stream],
    ) -> Result<(), CoreError>;

    /// The cached streams for a `(source, category)` (empty if none cached).
    fn streams(&self, source_id: &str, category_id: &str) -> Result<Vec<Stream>, CoreError>;
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
        categories: RefCell<HashMap<String, Vec<Category>>>,
        streams: RefCell<HashMap<(String, String), Vec<Stream>>>,
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
            self.categories.borrow_mut().remove(id);
            self.streams.borrow_mut().retain(|(sid, _), _| sid != id);
            Ok(())
        }

        fn replace_categories(
            &self,
            source_id: &str,
            categories: &[Category],
        ) -> Result<(), CoreError> {
            self.categories
                .borrow_mut()
                .insert(source_id.to_string(), categories.to_vec());
            Ok(())
        }

        fn categories(&self, source_id: &str) -> Result<Vec<Category>, CoreError> {
            Ok(self
                .categories
                .borrow()
                .get(source_id)
                .cloned()
                .unwrap_or_default())
        }

        fn replace_streams(
            &self,
            source_id: &str,
            category_id: &str,
            streams: &[Stream],
        ) -> Result<(), CoreError> {
            self.streams.borrow_mut().insert(
                (source_id.to_string(), category_id.to_string()),
                streams.to_vec(),
            );
            Ok(())
        }

        fn streams(&self, source_id: &str, category_id: &str) -> Result<Vec<Stream>, CoreError> {
            Ok(self
                .streams
                .borrow()
                .get(&(source_id.to_string(), category_id.to_string()))
                .cloned()
                .unwrap_or_default())
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
        cat.replace_categories("src-a", &[sports.clone(), news.clone()])
            .unwrap();
        assert_eq!(cat.categories("src-a").unwrap(), vec![sports, news.clone()]);

        // A re-sync replaces, not appends.
        cat.replace_categories("src-a", std::slice::from_ref(&news))
            .unwrap();
        assert_eq!(cat.categories("src-a").unwrap(), vec![news]);
        // Another source is untouched and empty.
        assert!(cat.categories("src-b").unwrap().is_empty());
    }

    #[test]
    fn streams_are_isolated_by_source_and_category() {
        let cat = FakeCatalog::default();
        let s1 = Stream::new("src-a", "1", "One", StreamKind::Live);
        let s2 = Stream::new("src-a", "2", "Two", StreamKind::Live);
        cat.replace_streams("src-a", "sports", &[s1.clone(), s2.clone()])
            .unwrap();

        assert_eq!(cat.streams("src-a", "sports").unwrap(), vec![s1, s2]);
        // Different category, and different source, are separate buckets.
        assert!(cat.streams("src-a", "news").unwrap().is_empty());
        assert!(cat.streams("src-b", "sports").unwrap().is_empty());
    }

    #[test]
    fn delete_source_clears_its_cache() {
        let cat = FakeCatalog::default();
        cat.upsert_source(&record("a")).unwrap();
        cat.replace_categories(
            "a",
            &[Category {
                id: CategoryId("1".to_string()),
                name: "Sports".to_string(),
            }],
        )
        .unwrap();
        cat.replace_streams("a", "1", &[Stream::new("a", "9", "Nine", StreamKind::Live)])
            .unwrap();

        cat.delete_source("a").unwrap();

        assert!(cat.sources().unwrap().is_empty());
        assert!(cat.categories("a").unwrap().is_empty());
        assert!(cat.streams("a", "1").unwrap().is_empty());
    }
}
