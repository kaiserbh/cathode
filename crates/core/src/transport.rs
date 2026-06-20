//! The HTTP transport seam.
//!
//! `core` never performs I/O itself. It builds URLs and parses responses; the
//! actual fetch is injected behind this trait so tests can substitute a fake and
//! the real implementation (reqwest) can live in the native shell. This keeps
//! `core` WASM-safe and deterministic.

use crate::error::CoreError;
use std::future::Future;

/// Fetch the text body at a URL.
///
/// The returned future is explicitly `Send` (via return-position `impl Trait`)
/// so that callers awaiting it inside a Tauri command handler stay `Send`, as
/// the tokio runtime requires.
pub trait Transport {
    fn get_text(&self, url: &str) -> impl Future<Output = Result<String, CoreError>> + Send;
}
