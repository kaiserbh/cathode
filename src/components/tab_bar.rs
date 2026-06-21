//! The top-level browse tabs: Channels, Favorites, History. Favorites and History
//! are shown only when their features are enabled in settings.

use dioxus::prelude::*;

/// Which browse view is active.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Channels,
    Favorites,
    History,
}

#[component]
pub fn TabBar(
    active: Tab,
    show_favorites: bool,
    show_history: bool,
    on_select: EventHandler<Tab>,
) -> Element {
    rsx! {
        nav {
            class: "flex gap-1 border-b border-neutral-200 px-2 dark:border-neutral-800",
            TabButton { label: "Channels", tab: Tab::Channels, active, on_select }
            if show_favorites {
                TabButton { label: "Favorites", tab: Tab::Favorites, active, on_select }
            }
            if show_history {
                TabButton { label: "History", tab: Tab::History, active, on_select }
            }
        }
    }
}

#[component]
fn TabButton(label: String, tab: Tab, active: Tab, on_select: EventHandler<Tab>) -> Element {
    let is_active = active == tab;
    let state = if is_active {
        "border-sky-500 text-sky-600 dark:text-sky-400"
    } else {
        "border-transparent text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-200"
    };
    rsx! {
        button {
            class: "border-b-2 px-3 py-2 text-sm font-medium focus:outline-none {state}",
            onclick: move |_| on_select.call(tab),
            "{label}"
        }
    }
}
