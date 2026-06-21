//! The window titlebar. With the macOS overlay titlebar style the webview extends
//! under the OS titlebar, so this bar doubles as the drag region (move the window) and
//! the home for the wordmark, the incognito badge, and the Options / Sources icon
//! buttons. The left padding clears the native traffic lights.

use dioxus::prelude::*;

use crate::components::icons::{Settings, Sources};

const BTN: &str = "rounded-full p-2 text-neutral-600 hover:bg-neutral-100 \
    focus:outline-none focus:ring-2 focus:ring-sky-400 dark:text-neutral-300 \
    dark:hover:bg-neutral-800";
const ICON: &str = "h-5 w-5";

#[component]
pub fn TitleBar(
    incognito: bool,
    on_options: EventHandler<()>,
    on_sources: EventHandler<()>,
) -> Element {
    rsx! {
        header {
            // The bar itself drags the window. The traffic lights sit in the left pad.
            "data-tauri-drag-region": "true",
            class: "shrink-0 flex h-11 items-center border-b border-neutral-200 \
                bg-white pl-20 pr-3 dark:border-neutral-800 dark:bg-neutral-950",
            // Wordmark + badge: a drag region too, since Tauri only drags when the
            // pointer target itself carries the attribute.
            div {
                "data-tauri-drag-region": "true",
                class: "flex items-center gap-3",
                h1 { class: "text-sm font-semibold", "Cathode" }
                if incognito {
                    span { class: "rounded-full bg-neutral-800 px-2 py-0.5 text-xs font-medium \
                            text-neutral-100 dark:bg-neutral-200 dark:text-neutral-900",
                        "Incognito"
                    }
                }
            }
            div { class: "ml-auto flex items-center gap-1",
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
