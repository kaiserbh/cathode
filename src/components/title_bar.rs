//! The window titlebar. With the macOS overlay titlebar style the webview extends
//! under the OS titlebar, so this bar doubles as the drag region (move the window). It
//! holds a centered library search box and, on the right, the Incognito badge and the
//! Logs / Options / Sources icon buttons. The OS title is hidden; the empty space is
//! draggable and the native traffic lights float over its left end.

use dioxus::prelude::*;

use crate::components::icons::{Bug, Close, Search, Settings, Sources};

const BTN: &str = "rounded-md p-1 text-neutral-600 hover:bg-neutral-100 \
    focus:outline-none focus:ring-2 focus:ring-sky-400 dark:text-neutral-300 \
    dark:hover:bg-neutral-800";
const ICON: &str = "h-4 w-4";

#[component]
pub fn TitleBar(
    incognito: bool,
    search: String,
    on_search: EventHandler<String>,
    on_logs: EventHandler<()>,
    on_options: EventHandler<()>,
    on_sources: EventHandler<()>,
) -> Element {
    let has_query = !search.is_empty();
    rsx! {
        header {
            "data-tauri-drag-region": "true",
            class: "shrink-0 flex h-8 items-center gap-2 border-b border-neutral-200 \
                px-2 dark:border-neutral-800 dark:bg-neutral-950 bg-white",
            onmousedown: move |_| {
                // Belt-and-suspenders drag: call startDragging() directly so dragging
                // works even if Tauri's attribute-based listener missed the element
                // (injected script runs before Dioxus/WASM boots the DOM).
                // Skip interactive elements so clicks on the search input and buttons
                // don't accidentally start a window drag.
                let _ = document::eval(
                    r#"(function(){
                        var t = event.target;
                        var skip = ['INPUT','BUTTON','SELECT','TEXTAREA','A'];
                        if (!skip.includes(t.tagName)) {
                            window.__TAURI__.window.getCurrent().startDragging();
                        }
                    })()"#,
                );
            },
            // Left spacer: covers the native macOS traffic-light buttons (~72 px) plus
            // an extra margin so there is visible drag area to their right.
            div { class: "w-28 shrink-0" }
            // Centered search; the surrounding empty space stays draggable.
            div { class: "flex flex-1 justify-center",
                div {
                    class: "flex h-6 w-full max-w-md items-center gap-1.5 rounded-md bg-neutral-100 \
                        px-2 focus-within:ring-2 focus-within:ring-sky-400 dark:bg-neutral-800",
                    Search { class: "h-3.5 w-3.5 shrink-0 text-neutral-400" }
                    input {
                        class: "w-full bg-transparent text-xs text-neutral-900 \
                            placeholder:text-neutral-400 focus:outline-none dark:text-neutral-100",
                        placeholder: "Search channels, movies, series…",
                        value: search,
                        oninput: move |e| on_search.call(e.value()),
                    }
                    if has_query {
                        button {
                            class: "shrink-0 rounded p-0.5 text-neutral-400 hover:text-neutral-700 \
                                dark:hover:text-neutral-200",
                            title: "Clear search",
                            onclick: move |_| on_search.call(String::new()),
                            Close { class: "h-3.5 w-3.5" }
                        }
                    }
                }
            }
            div { class: "flex shrink-0 items-center gap-0.5",
                if incognito {
                    span { class: "rounded-full bg-neutral-800 px-2 py-0.5 text-xs font-medium \
                            text-neutral-100 dark:bg-neutral-200 dark:text-neutral-900",
                        "Incognito"
                    }
                }
                button {
                    class: BTN,
                    title: "Logs",
                    onclick: move |_| on_logs.call(()),
                    Bug { class: ICON }
                }
                button {
                    class: BTN,
                    title: "Options",
                    onclick: move |_| on_options.call(()),
                    Settings { class: ICON }
                }
                button {
                    class: BTN,
                    title: "Sources",
                    onclick: move |_| on_sources.call(()),
                    Sources { class: ICON }
                }
            }
        }
    }
}
