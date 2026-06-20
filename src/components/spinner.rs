//! A centered loading spinner, shown while content is being fetched with nothing
//! cached to display yet.

use dioxus::prelude::*;

#[component]
pub fn Spinner() -> Element {
    rsx! {
        div {
            class: "flex h-full items-center justify-center p-10",
            div {
                class: "h-8 w-8 animate-spin rounded-full border-2 border-neutral-300 \
                    border-t-sky-500 dark:border-neutral-700 dark:border-t-sky-400",
                role: "status",
                "aria-label": "Loading",
            }
        }
    }
}
