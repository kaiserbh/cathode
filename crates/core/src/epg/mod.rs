//! Electronic Program Guide: parse XMLTV and match programmes to a point in time.
//!
//! Pure and WASM-safe — parsing and matching take their inputs (the XML, the
//! current time) as arguments; no I/O and no clock.

pub mod r#match;
pub mod merge;
pub mod parse;

pub use merge::{filter_to_channels, merge_guides};
pub use parse::{parse_xmltv, EpgChannel, Guide};
pub use r#match::{name_index, normalize_name, now_next};
