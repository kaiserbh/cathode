//! A transient toast notification. Presentational — the parent shows it for a moment
//! then drops it; this just renders a pill above everything else (`z-[60]`, over the
//! panels at `z-50`).

use dioxus::prelude::*;

use crate::components::icons::Check;

#[component]
pub fn Toast(message: String) -> Element {
    rsx! {
        div {
            class: "fixed bottom-6 left-1/2 z-[60] flex -translate-x-1/2 items-center gap-2 \
                rounded-lg bg-neutral-900 px-4 py-2 text-sm font-medium text-white shadow-lg \
                dark:bg-neutral-100 dark:text-neutral-900",
            Check { class: "h-4 w-4 text-emerald-400 dark:text-emerald-600" }
            {message}
        }
    }
}
