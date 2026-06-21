//! The window titlebar. With the macOS overlay titlebar style the webview extends
//! under the OS titlebar, so this bar doubles as the drag region (move the window) and
//! the home for the Incognito badge and the Options / Sources icon buttons. There is
//! no app title (the OS title is hidden); the left side is empty draggable space that
//! sits under the native traffic lights, with the controls grouped on the right.

use dioxus::prelude::*;

use crate::components::icons::{Bug, Settings, Sources};

const BTN: &str = "rounded-full p-2 text-neutral-600 hover:bg-neutral-100 \
    focus:outline-none focus:ring-2 focus:ring-sky-400 dark:text-neutral-300 \
    dark:hover:bg-neutral-800";
const ICON: &str = "h-5 w-5";

#[component]
pub fn TitleBar(
    incognito: bool,
    on_logs: EventHandler<()>,
    on_options: EventHandler<()>,
    on_sources: EventHandler<()>,
) -> Element {
    rsx! {
        header {
            // The empty bar itself drags the window; the traffic lights float over its
            // left end. Controls cluster on the right.
            "data-tauri-drag-region": "true",
            class: "shrink-0 flex h-11 items-center border-b border-neutral-200 \
                px-3 dark:border-neutral-800 dark:bg-neutral-950 bg-white",
            div { class: "ml-auto flex items-center gap-2",
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
