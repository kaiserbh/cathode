//! Cathode core.
//!
//! Pure, deterministic, WASM-safe domain logic shared by the Tauri shell and the
//! Dioxus frontend. No global state, no hidden I/O: network and disk are passed in
//! behind traits so tests can substitute fakes. See AGENTS.md for the rules.

pub mod catalog;
pub mod epg;
pub mod error;
pub mod model;
pub mod redact;
pub mod sources;
pub mod transport;
