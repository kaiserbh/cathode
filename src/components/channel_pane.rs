//! Renders a set of channels in the user's chosen view (grid, list, or timeline
//! guide). Keeps the switch in one place so the views (Channels/Favorites/History)
//! don't each duplicate it. The Grid and List views carry a quick filter that
//! narrows the shown list by name; the Guide (a virtualized timeline with its own
//! scroll machinery) is left as-is.

use std::collections::HashMap;

use cathode_core::model::{ChannelView, NowNext, Programme, Stream, StreamId};
use dioxus::prelude::*;

use crate::components::icons::{Close, Search};
use crate::components::{ChannelList, EpgGuide, StreamGrid};

#[component]
pub fn ChannelPane(
    view: ChannelView,
    streams: Vec<Stream>,
    favorites_enabled: bool,
    favorite_ids: Vec<StreamId>,
    epg: HashMap<String, NowNext>,
    // Guide-only inputs; ignored by Grid/List.
    programmes: HashMap<String, Vec<Programme>>,
    guide_from: i64,
    guide_to: i64,
    now: i64,
    on_play: EventHandler<Stream>,
    on_toggle_favorite: EventHandler<Stream>,
    // Guide-only: open a programme's detail popover (programme, channel, click x/y).
    on_programme: EventHandler<(Programme, Stream, f64, f64)>,
) -> Element {
    // Called unconditionally so the hook order stays stable when `view` switches
    // between Guide and Grid/List for the same pane instance.
    let mut query = use_signal(String::new);

    // The Guide is a virtualized timeline with bespoke scroll syncing, so the quick
    // filter (and its sticky header) applies only to the Grid and List views.
    if let ChannelView::Guide = view {
        return rsx! {
            EpgGuide {
                streams,
                programmes,
                from: guide_from,
                to: guide_to,
                now,
                on_play,
                on_programme,
            }
        };
    }

    let filtered = crate::filter::filter_by_name(&streams, &query());

    rsx! {
        // Quick filter for the current list; sticks to the top of the scroll area.
        div {
            class: "sticky top-0 z-10 flex items-center gap-1.5 border-b border-neutral-200 \
                bg-white/90 px-3 py-1.5 backdrop-blur dark:border-neutral-800 dark:bg-neutral-950/90",
            Search { class: "h-3.5 w-3.5 shrink-0 text-neutral-400" }
            input {
                class: "w-full bg-transparent text-sm text-neutral-900 placeholder:text-neutral-400 \
                    focus:outline-none dark:text-neutral-100",
                placeholder: "Filter this list…",
                value: "{query}",
                oninput: move |e| query.set(e.value()),
            }
            if !query().is_empty() {
                button {
                    class: "shrink-0 rounded p-0.5 text-neutral-400 hover:text-neutral-700 \
                        dark:hover:text-neutral-200",
                    title: "Clear filter",
                    onclick: move |_| query.set(String::new()),
                    Close { class: "h-3.5 w-3.5" }
                }
            }
        }
        if filtered.is_empty() {
            p { class: "p-6 text-sm text-neutral-500", "No matches in this list." }
        } else if let ChannelView::List = view {
            ChannelList {
                streams: filtered,
                favorites_enabled,
                favorite_ids,
                epg,
                on_play,
                on_toggle_favorite,
            }
        } else {
            StreamGrid {
                streams: filtered,
                favorites_enabled,
                favorite_ids,
                epg,
                on_play,
                on_toggle_favorite,
            }
        }
    }
}
