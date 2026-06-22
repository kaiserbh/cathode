//! The Sources panel: switch between saved Xtream accounts, add a new one, or
//! remove one. Presentational — `Browse` owns the state and the handlers.

use cathode_core::sources::xtream::XtreamCredentials;
use dioxus::prelude::*;

use crate::components::ConnectForm;
use crate::components::icons::Close;
use crate::ui::button::{Button, ButtonSize, ButtonVariant};

/// A human label for a saved account: `username @ host` (scheme stripped).
fn label(creds: &XtreamCredentials) -> String {
    let host = creds
        .base_url
        .strip_prefix("https://")
        .or_else(|| creds.base_url.strip_prefix("http://"))
        .unwrap_or(&creds.base_url);
    format!("{} @ {}", creds.username, host)
}

#[component]
pub fn SourcesPanel(
    sources: Vec<XtreamCredentials>,
    active: Option<XtreamCredentials>,
    connecting: bool,
    on_select: EventHandler<XtreamCredentials>,
    on_forget: EventHandler<XtreamCredentials>,
    on_connect: EventHandler<XtreamCredentials>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        // Dimmed backdrop; clicking it closes the panel.
        div {
            class: "fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-4 \
                sm:items-center",
            onclick: move |_| on_close.call(()),
            // The card. Stop propagation so clicks inside don't close it.
            div {
                class: "w-full max-w-lg rounded-xl bg-white text-neutral-900 shadow-xl \
                    dark:bg-neutral-900 dark:text-neutral-100",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "flex items-center justify-between border-b border-neutral-200 \
                        px-5 py-4 dark:border-neutral-800",
                    h2 { class: "text-base font-semibold", "Sources" }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::IconSm,
                        title: "Close",
                        onclick: move |_| on_close.call(()),
                        Close { class: "h-5 w-5" }
                    }
                }

                if sources.is_empty() {
                    p {
                        class: "px-5 py-10 text-center text-sm text-neutral-500",
                        "No accounts yet. Add one below."
                    }
                } else {
                    ul {
                        class: "flex flex-col gap-1 p-3",
                        for source in sources.iter().cloned() {
                            {
                                let is_active = active.as_ref() == Some(&source);
                                let select_source = source.clone();
                                let forget_source = source.clone();
                                let row = if is_active {
                                    "bg-sky-50 dark:bg-sky-950"
                                } else {
                                    "hover:bg-neutral-100 dark:hover:bg-neutral-800"
                                };
                                rsx! {
                                    li {
                                        key: "{source.base_url}|{source.username}",
                                        class: "flex items-center gap-2 rounded-md {row}",
                                        button {
                                            class: "flex-1 truncate px-3 py-2 text-left text-sm",
                                            onclick: move |_| on_select.call(select_source.clone()),
                                            if is_active {
                                                span { class: "mr-2 text-sky-600", "●" }
                                            }
                                            "{label(&source)}"
                                        }
                                        button {
                                            class: "shrink-0 rounded-md px-3 py-2 \
                                                text-neutral-400 hover:text-red-600",
                                            title: "Remove",
                                            onclick: move |e| {
                                                e.stop_propagation();
                                                on_forget.call(forget_source.clone());
                                            },
                                            Close { class: "h-4 w-4" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    class: "border-t border-neutral-200 px-5 py-4 dark:border-neutral-800",
                    h3 { class: "mb-2 text-sm font-medium text-neutral-500", "Add account" }
                    ConnectForm { connecting, on_connect }
                }
            }
        }
    }
}
