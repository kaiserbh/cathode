//! A thin wrapper over the browser Clipboard API, for "Copy" buttons in the UI.
//!
//! Tauri's webview is a secure context, so `navigator.clipboard.writeText` is
//! available. Failures (no clipboard, permission denied) are swallowed: copying is a
//! convenience, never load-bearing.

use wasm_bindgen_futures::JsFuture;

/// Write `text` to the system clipboard. Best-effort; errors are ignored.
pub async fn copy(text: String) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let clipboard = window.navigator().clipboard();
    let _ = JsFuture::from(clipboard.write_text(&text)).await;
}
