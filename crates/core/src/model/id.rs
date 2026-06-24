//! Stable internal identifiers.
//!
//! Favorites and history must survive a re-sync, so we never key on list
//! position. Instead we derive an id by hashing the source id together with the
//! most stable field a provider gives us. The preferred key, in
//! order, is the Xtream `stream_id`, then `tvg-id`, falling back to name + url
//! for raw M3U. This module takes the already-chosen key; choosing it lives with
//! each source parser.

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

/// A stable internal identifier for a stream.
///
/// Derived (never positional) so it survives a re-sync. Stored as a fixed-width
/// hex string so it is cheap to compare, log, and use as a map key on both ends.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamId(pub String);

/// Byte that separates the source id from the stable key when hashing.
///
/// Using an in-band delimiter that cannot appear in either field keeps the split
/// unambiguous, so `("ab", "c")` and `("a", "bc")` hash differently. A NUL byte
/// is safe here: provider ids and keys are text and never contain it.
const FIELD_SEP: u8 = 0x00;

/// Derive a [`StreamId`] from the source id and the most stable key available
/// for the stream (Xtream `stream_id`, else `tvg-id`, else name + url).
///
/// Deterministic: the same inputs always yield the same id, independent of list
/// order or call context.
pub fn derive_stream_id(source_id: &str, stable_key: &str) -> StreamId {
    let mut buf = Vec::with_capacity(source_id.len() + 1 + stable_key.len());
    buf.extend_from_slice(source_id.as_bytes());
    buf.push(FIELD_SEP);
    buf.extend_from_slice(stable_key.as_bytes());
    StreamId(format!("{:016x}", xxh3_64(&buf)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic() {
        // Same inputs must always produce the same id, run to run.
        let a = derive_stream_id("src-1", "12345");
        let b = derive_stream_id("src-1", "12345");
        assert_eq!(a, b);
    }

    #[test]
    fn differs_on_source() {
        // Same provider stream_id under two different sources is two records.
        assert_ne!(
            derive_stream_id("src-1", "12345"),
            derive_stream_id("src-2", "12345")
        );
    }

    #[test]
    fn differs_on_key() {
        // Two different streams within one source must not collide.
        assert_ne!(
            derive_stream_id("src-1", "12345"),
            derive_stream_id("src-1", "67890")
        );
    }

    #[test]
    fn golden_value_is_locked() {
        // Regression lock: if this changes, the hashing scheme changed and every
        // stored favorite/history id silently breaks. Treat a failure here as a
        // deliberate decision, not a free re-baseline.
        assert_eq!(derive_stream_id("src-1", "12345").0, "eac7ff5f94e3c143");
    }

    #[test]
    fn boundary_is_unambiguous() {
        // The source/key split must not be confusable: ("ab","c") != ("a","bc").
        assert_ne!(derive_stream_id("ab", "c"), derive_stream_id("a", "bc"));
    }
}
