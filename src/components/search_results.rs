//! The library search results dropdown, anchored under the titlebar. Lists matching
//! streams with a kind badge; selecting one plays it (Live/VOD) or opens it (Series).
//! Presentational — `Browse` runs the search and handles selection.

use cathode_core::model::{Stream, StreamKind};
use dioxus::prelude::*;

/// A short label + color for a stream's kind badge.
fn kind_badge(kind: StreamKind) -> (&'static str, &'static str) {
    match kind {
        StreamKind::Live => (
            "Live",
            "bg-sky-100 text-sky-700 dark:bg-sky-900 dark:text-sky-300",
        ),
        StreamKind::Vod => (
            "Movie",
            "bg-amber-100 text-amber-700 dark:bg-amber-900 dark:text-amber-300",
        ),
        StreamKind::Series => (
            "Series",
            "bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300",
        ),
    }
}

#[component]
pub fn SearchResults(
    results: Vec<Stream>,
    on_select: EventHandler<Stream>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        // Click-away backdrop (transparent) so selecting elsewhere closes the dropdown.
        div {
            class: "fixed inset-0 z-40",
            onclick: move |_| on_close.call(()),
            div {
                class: "absolute left-1/2 top-10 max-h-[70vh] w-full max-w-xl -translate-x-1/2 \
                    overflow-y-auto rounded-xl border border-neutral-200 bg-white text-neutral-900 \
                    shadow-xl dark:border-neutral-800 dark:bg-neutral-900 dark:text-neutral-100",
                onclick: move |e| e.stop_propagation(),
                if results.is_empty() {
                    p {
                        class: "px-4 py-6 text-center text-sm text-neutral-500",
                        "No matches in the synced library."
                    }
                } else {
                    ul {
                        class: "divide-y divide-neutral-100 dark:divide-neutral-800",
                        for stream in results.iter().cloned() {
                            {
                                let (label, color) = kind_badge(stream.kind);
                                let pick = stream.clone();
                                rsx! {
                                    li {
                                        button {
                                            class: "flex w-full items-center gap-3 px-3 py-2 text-left \
                                                hover:bg-neutral-100 dark:hover:bg-neutral-800",
                                            onclick: move |_| on_select.call(pick.clone()),
                                            if let Some(logo) = stream.logo.as_ref() {
                                                img {
                                                    class: "h-8 w-8 shrink-0 rounded object-contain",
                                                    src: logo.as_str(),
                                                    alt: "",
                                                }
                                            } else {
                                                div { class: "h-8 w-8 shrink-0 rounded bg-neutral-200 dark:bg-neutral-700" }
                                            }
                                            span { class: "min-w-0 flex-1 truncate text-sm", "{stream.name}" }
                                            span {
                                                class: "shrink-0 rounded-full px-2 py-0.5 text-xs font-medium {color}",
                                                {label}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
