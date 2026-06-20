//! The channel grid. Responsive columns; cards are focusable for TV navigation.
//! Selecting a channel is a placeholder until the playback increment.

use cathode_core::model::Stream;
use dioxus::prelude::*;

#[component]
pub fn StreamGrid(streams: Vec<Stream>, on_play: EventHandler<Stream>) -> Element {
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
                rsx! {
                    button {
                        key: "{stream.id.0}",
                        class: "flex flex-col items-center gap-2 rounded-lg p-3 \
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
                    }
                }
            })}
        }
    }
}
