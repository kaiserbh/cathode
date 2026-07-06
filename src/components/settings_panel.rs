//! The Options panel. Every feature is opt-out: toggles for favorites and watch
//! history (persisted), a session-only incognito switch that pauses recording, and
//! a button to erase history. Presentational; `Browse` owns the state.

use cathode_core::model::{ChannelView, Settings};
use dioxus::prelude::*;

use crate::components::PanelDialog;
use crate::components::Toggle;
use crate::components::icons::Close;
use crate::ui::button::{Button, ButtonSize, ButtonVariant};

#[component]
pub fn SettingsPanel(
    settings: Settings,
    incognito: bool,
    on_toggle_favorites: EventHandler<bool>,
    on_toggle_history: EventHandler<bool>,
    on_toggle_epg: EventHandler<bool>,
    on_set_view: EventHandler<ChannelView>,
    on_set_volume: EventHandler<u8>,
    on_toggle_incognito: EventHandler<bool>,
    on_clear_history: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        PanelDialog { class: "max-w-lg", on_close,
            div {
            class: "flex items-center justify-between border-b border-neutral-200 \
                px-5 py-4 dark:border-neutral-800",
                h2 { class: "text-base font-semibold", "Options" }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::IconSm,
                    title: "Close",
                    onclick: move |_| on_close.call(()),
                    Close { class: "h-5 w-5" }
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
                        ViewButton {
                            label: "Guide",
                            view: ChannelView::Guide,
                            current: settings.channel_view,
                            on_set_view,
                        }
                    }
                }
                div {
                    class: "flex items-center justify-between gap-4 px-5 py-4",
                    div {
                        span { class: "block text-sm font-medium", "Default volume" }
                        span {
                            class: "block text-xs text-neutral-500",
                            "Volume applied when playback starts."
                        }
                    }
                    div {
                        class: "flex shrink-0 items-center gap-2",
                        input {
                            r#type: "range",
                            min: "0",
                            max: "100",
                            value: "{settings.volume}",
                            // --vol drives the fill; --track tints the remainder so the
                            // slider reads well on the panel (not just the dark player bar).
                            style: "--vol: {settings.volume}%; --track: rgba(120, 120, 120, 0.35)",
                            class: "cathode-range w-32",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u8>() {
                                    on_set_volume.call(v);
                                }
                            },
                        }
                        span {
                            class: "w-9 text-right text-xs tabular-nums text-neutral-500",
                            "{settings.volume}%"
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
                    Button {
                        variant: ButtonVariant::Destructive,
                        size: ButtonSize::Sm,
                        class: "shrink-0",
                        onclick: move |_| on_clear_history.call(()),
                        "Clear"
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
            {label}
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
                span { class: "block text-sm font-medium", {label} }
                span { class: "block text-xs text-neutral-500", {description} }
            }
            Toggle { value, on_toggle }
        }
    }
}
