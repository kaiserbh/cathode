//! The top-level browse tabs: Live, Movies, Series, Favorites, History. The content
//! tabs (Live/Movies/Series) are always shown; Favorites and History appear only when
//! their features are enabled in settings.

use dioxus::prelude::*;

use crate::components::icons::{Film, Series as SeriesIcon, Tv};

/// Which browse view is active.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Live,
    Movies,
    Series,
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
            class: "shrink-0 flex gap-1 border-b border-neutral-200 px-2 dark:border-neutral-800",
            TabButton { label: "Live", tab: Tab::Live, active, on_select }
            TabButton { label: "Movies", tab: Tab::Movies, active, on_select }
            TabButton { label: "Series", tab: Tab::Series, active, on_select }
            if show_favorites {
                TabButton { label: "Favorites", tab: Tab::Favorites, active, on_select }
            }
            if show_history {
                TabButton { label: "History", tab: Tab::History, active, on_select }
            }
        }
    }
}

/// The icon for a tab.
fn tab_icon(tab: Tab) -> Element {
    let class = "h-4 w-4".to_string();
    match tab {
        Tab::Live => rsx! { Tv { class } },
        Tab::Movies => rsx! { Film { class } },
        Tab::Series => rsx! { SeriesIcon { class } },
        Tab::Favorites => rsx! { span { class: "text-base leading-none", "★" } },
        Tab::History => rsx! { span { class: "text-base leading-none", "↺" } },
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
            class: "flex items-center gap-1.5 border-b-2 px-3 py-2 text-sm font-medium \
                focus:outline-none {state}",
            onclick: move |_| on_select.call(tab),
            {tab_icon(tab)}
            {label}
        }
    }
}
