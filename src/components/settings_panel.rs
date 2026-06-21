//! The Options panel. Every feature is opt-out: toggles for favorites and watch
//! history (persisted), a session-only incognito switch that pauses recording, and
//! a button to erase history. Presentational; `Browse` owns the state.

use cathode_core::model::{ChannelView, Settings};
use dioxus::prelude::*;

use crate::components::Toggle;

#[component]
pub fn SettingsPanel(
    settings: Settings,
    incognito: bool,
    on_toggle_favorites: EventHandler<bool>,
    on_toggle_history: EventHandler<bool>,
    on_toggle_epg: EventHandler<bool>,
    on_set_view: EventHandler<ChannelView>,
    on_toggle_incognito: EventHandler<bool>,
    on_clear_history: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-4 \
                sm:items-center",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-lg rounded-xl bg-white text-neutral-900 shadow-xl \
                    dark:bg-neutral-900 dark:text-neutral-100",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "flex items-center justify-between border-b border-neutral-200 \
                        px-5 py-4 dark:border-neutral-800",
                    h2 { class: "text-base font-semibold", "Options" }
                    button {
                        class: "rounded-md px-2 py-1 text-sm text-neutral-500 \
                            hover:bg-neutral-100 dark:hover:bg-neutral-800",
                        onclick: move |_| on_close.call(()),
                        "Close"
                    }
                }

                div {
                    class: "flex flex-col divide-y divide-neutral-200 dark:divide-neutral-800",
                    ToggleRow {
                        label: "Favorites",
                        description: "Show the star on channels and the Favorites tab.",
                        value: settings.favorites_enabled,
                        on_toggle: on_toggle_favorites,
                    }
                    ToggleRow {
                        label: "Record watch history",
                        description: "Keep a list of what you've played.",
                        value: settings.history_enabled,
                        on_toggle: on_toggle_history,
                    }
                    ToggleRow {
                        label: "Program guide (EPG)",
                        description: "Fetch and show Now / Next on channels.",
                        value: settings.epg_enabled,
                        on_toggle: on_toggle_epg,
                    }
                    div {
                        class: "flex items-center justify-between px-5 py-4",
                        div {
                            span { class: "block text-sm font-medium", "Channel view" }
                            span {
                                class: "block text-xs text-neutral-500",
                                "How channels are laid out."
                            }
                        }
                        div {
                            class: "flex gap-1 rounded-lg bg-neutral-200 p-0.5 dark:bg-neutral-800",
                            ViewButton {
                                label: "Grid",
                                view: ChannelView::Grid,
                                current: settings.channel_view,
                                on_set_view,
                            }
                            ViewButton {
                                label: "List",
                                view: ChannelView::List,
                                current: settings.channel_view,
                                on_set_view,
                            }
                        }
                    }
                    ToggleRow {
                        label: "Incognito",
                        description: "Pause recording for this session only.",
                        value: incognito,
                        on_toggle: on_toggle_incognito,
                    }
                    div {
                        class: "flex items-center justify-between px-5 py-4",
                        div {
                            span { class: "block text-sm font-medium", "Watch history" }
                            span {
                                class: "block text-xs text-neutral-500",
                                "Erase everything you've watched."
                            }
                        }
                        button {
                            class: "shrink-0 rounded-md border border-red-300 px-3 py-1.5 \
                                text-sm font-medium text-red-700 hover:bg-red-50 \
                                dark:border-red-800 dark:text-red-300 dark:hover:bg-red-950",
                            onclick: move |_| on_clear_history.call(()),
                            "Clear"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ViewButton(
    label: String,
    view: ChannelView,
    current: ChannelView,
    on_set_view: EventHandler<ChannelView>,
) -> Element {
    let state = if view == current {
        "bg-white text-neutral-900 shadow-sm dark:bg-neutral-600 dark:text-white"
    } else {
        "text-neutral-600 hover:text-neutral-900 dark:text-neutral-300 dark:hover:text-white"
    };
    rsx! {
        button {
            class: "rounded-md px-3 py-1 text-sm font-medium focus:outline-none {state}",
            onclick: move |_| on_set_view.call(view),
            "{label}"
        }
    }
}

#[component]
fn ToggleRow(
    label: String,
    description: String,
    value: bool,
    on_toggle: EventHandler<bool>,
) -> Element {
    rsx! {
        div {
            class: "flex items-center justify-between px-5 py-4",
            div {
                span { class: "block text-sm font-medium", "{label}" }
                span { class: "block text-xs text-neutral-500", "{description}" }
            }
            Toggle { value, on_toggle }
        }
    }
}
