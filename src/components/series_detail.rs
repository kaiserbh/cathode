//! The series drill-down: a modal showing a series' cover, a season selector, and the
//! selected season's episodes. Presentational — `Browse` fetches the info and plays the
//! chosen episode.

use cathode_core::model::{Episode, SeriesInfo, Stream};
use dioxus::prelude::*;

use crate::components::icons::{Close, Play};
use crate::components::{PanelDialog, Spinner};
use crate::ui::button::{Button, ButtonSize, ButtonVariant};

#[component]
pub fn SeriesDetail(
    series: Stream,
    info: Option<SeriesInfo>,
    on_play_episode: EventHandler<Episode>,
    on_close: EventHandler<()>,
) -> Element {
    let mut season_idx = use_signal(|| 0usize);

    rsx! {
    PanelDialog { class: "max-w-2xl max-h-[85vh] overflow-hidden", on_close,
        // Header: cover + title + close.
        div {
            class: "flex items-center gap-3 border-b border-neutral-200 p-4 \
                dark:border-neutral-800",
                if let Some(logo) = series.logo.as_ref() {
                    img {
                        class: "h-16 w-12 shrink-0 rounded object-cover",
                        src: logo.as_str(),
                        alt: "{series.name}",
                    }
                }
                h2 { class: "min-w-0 flex-1 truncate text-lg font-semibold", "{series.name}" }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::IconSm,
                    class: "shrink-0",
                    title: "Close",
                    onclick: move |_| on_close.call(()),
                    Close { class: "h-5 w-5" }
                }
            }

            match info {
                None => rsx! { Spinner {} },
                Some(info) if info.seasons.is_empty() => rsx! {
                    p {
                        class: "px-5 py-10 text-center text-sm text-neutral-500",
                        "No episodes found for this series."
                    }
                },
                Some(info) => {
                    let idx = season_idx().min(info.seasons.len() - 1);
                    let season = &info.seasons[idx];
                    rsx! {
                        // Season selector.
                        div {
                            class: "flex shrink-0 gap-1 overflow-x-auto border-b border-neutral-200 \
                                p-2 dark:border-neutral-800",
                            for (i, s) in info.seasons.iter().enumerate() {
                                {
                                    let cls = if i == idx {
                                        "shrink-0 rounded-md bg-sky-600 px-3 py-1 text-sm font-medium text-white"
                                    } else {
                                        "shrink-0 rounded-md px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800"
                                    };
                                    rsx! {
                                        button {
                                            class: cls,
                                            onclick: move |_| season_idx.set(i),
                                            "Season {s.number}"
                                        }
                                    }
                                }
                            }
                        }
                        // Episodes.
                        ul {
                            class: "flex-1 divide-y divide-neutral-200 overflow-y-auto \
                                dark:divide-neutral-800",
                            for ep in season.episodes.iter().cloned() {
                                li {
                                    class: "flex items-center gap-3 px-4 py-2",
                                    span {
                                        class: "w-12 shrink-0 text-xs font-medium text-neutral-500",
                                        "S{ep.season}E{ep.episode}"
                                    }
                                    span { class: "min-w-0 flex-1 truncate text-sm", "{ep.title}" }
                                    Button {
                                        variant: ButtonVariant::Primary,
                                        size: ButtonSize::IconSm,
                                        class: "shrink-0 rounded-full!",
                                        title: "Play",
                                        onclick: move |_| on_play_episode.call(ep.clone()),
                                        Play { class: "h-4 w-4" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
