//! The channel grid. Responsive columns; cards are focusable for TV navigation.
//! Each card plays on click and, when favorites are enabled, carries a star toggle.

use std::collections::HashMap;

use cathode_core::model::{NowNext, Stream, StreamId};
use dioxus::prelude::*;

#[component]
pub fn StreamGrid(
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
        div {
            class: "grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-6 gap-3 p-3",
            {streams.iter().map(|stream| {
                let played = stream.clone();
                let favorited = stream.clone();
                let is_favorite = favorite_ids.contains(&stream.id);
                let now_next = stream.epg_channel_id.as_ref().and_then(|id| epg.get(id));
                rsx! {
                    div {
                        key: "{stream.id.0}",
                        class: "relative",
                        button {
                            class: "flex w-full flex-col items-center gap-2 rounded-lg p-3 \
                                bg-neutral-100 dark:bg-neutral-900 \
                                hover:bg-neutral-200 dark:hover:bg-neutral-800 \
                                focus:outline-none focus:ring-2 focus:ring-sky-500",
                            onclick: move |_| on_play.call(played.clone()),
                            if let Some(logo) = stream.logo.as_ref() {
                                img {
                                    class: "h-16 w-16 object-contain",
                                    src: "{logo}",
                                    alt: "{stream.name}",
                                }
                            } else {
                                div {
                                    class: "flex h-16 w-16 items-center justify-center rounded \
                                        bg-neutral-300 dark:bg-neutral-700 text-xs text-neutral-600 \
                                        dark:text-neutral-300",
                                    "TV"
                                }
                            }
                            span { class: "w-full truncate text-center text-sm", "{stream.name}" }
                            if let Some(nn) = now_next {
                                if let Some(now) = nn.now.as_ref() {
                                    span {
                                        class: "w-full truncate text-center text-xs \
                                            text-neutral-500 dark:text-neutral-400",
                                        "{now.title}"
                                    }
                                }
                                if let Some(next) = nn.next.as_ref() {
                                    span {
                                        class: "w-full truncate text-center text-[11px] \
                                            text-neutral-400 dark:text-neutral-500",
                                        "Next: {next.title}"
                                    }
                                }
                            }
                        }
                        if favorites_enabled {
                            button {
                                class: "absolute right-1.5 top-1.5 rounded-full bg-black/40 \
                                    px-1.5 text-base leading-6 text-white hover:bg-black/60 \
                                    focus:outline-none focus:ring-2 focus:ring-sky-400",
                                title: if is_favorite { "Remove favorite" } else { "Add favorite" },
                                onclick: move |e| {
                                    e.stop_propagation();
                                    on_toggle_favorite.call(favorited.clone());
                                },
                                if is_favorite { "★" } else { "☆" }
                            }
                        }
                    }
                }
            })}
        }
    }
}
