//! The top-level browse tabs: Live, Movies, Series, Favorites, History. The content
//! tabs (Live/Movies/Series) are always shown; Favorites and History appear only when
//! their features are enabled in settings.
//!
//! Built on the dioxus-primitives `Tabs`, driven (controlled) by the `Tab` enum that
//! `Browse` owns — the tabs only render the bar; the content switch stays in `Browse`,
//! so there's a single source of truth. The empty strip is a window drag region.

use dioxus::prelude::*;

use crate::components::icons::{Film, History, Series as SeriesIcon, Star, Tv};
use crate::ui::tabs::{TabList, TabTrigger, Tabs};

/// Which browse view is active.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Live,
    Movies,
    Series,
    Favorites,
    History,
}

/// The stable string id for a tab (the primitives' tab value).
fn tab_to_str(tab: Tab) -> &'static str {
    match tab {
        Tab::Live => "live",
        Tab::Movies => "movies",
        Tab::Series => "series",
        Tab::Favorites => "favorites",
        Tab::History => "history",
    }
}

fn tab_from_str(value: &str) -> Tab {
    match value {
        "movies" => Tab::Movies,
        "series" => Tab::Series,
        "favorites" => Tab::Favorites,
        "history" => Tab::History,
        _ => Tab::Live,
    }
}

const TRIGGER: &str = "inline-flex items-center gap-1.5";

#[component]
pub fn TabBar(
    active: Tab,
    show_movies: bool,
    show_series: bool,
    show_favorites: bool,
    show_history: bool,
    on_select: EventHandler<Tab>,
) -> Element {
    rsx! {
        Tabs {
            value: Some(tab_to_str(active).to_string()),
            on_value_change: move |v: String| on_select.call(tab_from_str(&v)),
            horizontal: true,
            // The empty space in the tab strip drags the window (macOS); triggers are
            // buttons, so they aren't drag targets.
            TabList { "data-tauri-drag-region": "true",
                TabTrigger { value: "live", index: 0usize, class: TRIGGER,
                    Tv { class: "h-4 w-4" }
                    "Live"
                }
                if show_movies {
                    TabTrigger { value: "movies", index: 1usize, class: TRIGGER,
                        Film { class: "h-4 w-4" }
                        "Movies"
                    }
                }
                if show_series {
                    TabTrigger { value: "series", index: 2usize, class: TRIGGER,
                        SeriesIcon { class: "h-4 w-4" }
                        "Series"
                    }
                }
                if show_favorites {
                    TabTrigger { value: "favorites", index: 3usize, class: TRIGGER,
                        Star { class: "h-4 w-4", filled: true }
                        "Favorites"
                    }
                }
                if show_history {
                    TabTrigger { value: "history", index: 4usize, class: TRIGGER,
                        History { class: "h-4 w-4" }
                        "History"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_string_round_trips() {
        for tab in [
            Tab::Live,
            Tab::Movies,
            Tab::Series,
            Tab::Favorites,
            Tab::History,
        ] {
            assert_eq!(tab_from_str(tab_to_str(tab)), tab);
        }
        // Unknown ids fall back to Live.
        assert_eq!(tab_from_str("nope"), Tab::Live);
    }
}
