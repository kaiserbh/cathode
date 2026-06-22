//! The compact list view of channels: one row each, with room for richer EPG
//! (Now / Next with times). Same behaviour as the grid — click to play, star to
//! favorite.

use std::collections::HashMap;

use cathode_core::model::{NowNext, Stream, StreamId};
use dioxus::prelude::*;

use crate::components::icons::Star;
use crate::format::hhmm;

#[component]
pub fn ChannelList(
    streams: Vec<Stream>,
    favorites_enabled: bool,
    favorite_ids: Vec<StreamId>,
    epg: HashMap<String, NowNext>,
    on_play: EventHandler<Stream>,
    on_toggle_favorite: EventHandler<Stream>,
) -> Element {
    if streams.is_empty() {
        return rsx! {
            p { class: "p-6 text-sm text-neutral-500", "No channels to show yet." }
        };
    }

    rsx! {
        ul {
            class: "flex flex-col divide-y divide-neutral-200 p-2 dark:divide-neutral-800",
            {streams.iter().map(|stream| {
                let played = stream.clone();
                let favorited = stream.clone();
                let is_favorite = favorite_ids.contains(&stream.id);
                let now_next = crate::epg::resolve(&epg, stream);
                rsx! {
                    li {
                        key: "{stream.id.0}",
                        class: "flex items-center gap-2",
                        button {
                            class: "flex min-w-0 flex-1 items-center gap-3 rounded-md p-2 \
                                text-left hover:bg-neutral-100 dark:hover:bg-neutral-800 \
                                focus:outline-none focus:ring-2 focus:ring-sky-500",
                            onclick: move |_| on_play.call(played.clone()),
                            if let Some(logo) = stream.logo.as_ref() {
                                img {
                                    class: "h-10 w-10 shrink-0 object-contain",
                                    src: logo.as_str(),
                                    alt: "{stream.name}",
                                }
                            } else {
                                div {
                                    class: "flex h-10 w-10 shrink-0 items-center justify-center \
                                        rounded bg-neutral-300 text-[10px] text-neutral-600 \
                                        dark:bg-neutral-700 dark:text-neutral-300",
                                    "TV"
                                }
                            }
                            div {
                                class: "min-w-0 flex-1",
                                span { class: "block truncate text-sm font-medium", "{stream.name}" }
                                if let Some(nn) = now_next {
                                    if let Some(now) = nn.now.as_ref() {
                                        span {
                                            class: "block truncate text-xs text-neutral-500 \
                                                dark:text-neutral-400",
                                            "Now: {now.title} · {hhmm(now.start)}"
                                        }
                                    }
                                    if let Some(next) = nn.next.as_ref() {
                                        span {
                                            class: "block truncate text-[11px] text-neutral-400 \
                                                dark:text-neutral-500",
                                            "Next: {next.title} · {hhmm(next.start)}"
                                        }
                                    }
                                }
                            }
                        }
                        if favorites_enabled {
                            button {
                                class: "shrink-0 rounded-full px-2 text-lg text-neutral-400 \
                                    hover:text-amber-400 focus:outline-none focus:ring-2 \
                                    focus:ring-sky-400",
                                title: if is_favorite { "Remove favorite" } else { "Add favorite" },
                                onclick: move |e| {
                                    e.stop_propagation();
                                    on_toggle_favorite.call(favorited.clone());
                                },
                                Star { class: "h-5 w-5", filled: is_favorite }
                            }
                        }
                    }
                }
            })}
        }
    }
}
