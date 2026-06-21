//! The Logs panel: a modal showing the captured debug log as colored, aligned rows,
//! with a level dropdown (Off disables capture) and icon actions to copy or clear. It
//! auto-scrolls to the newest line while open. Presentational — `Browse` owns the
//! polling and the handlers.

use cathode_core::model::{LogLevel, LogLine};
use dioxus::prelude::*;

use crate::components::icons::{Close, Copy, Trash};

/// The id of the scroll container, used to keep it pinned to the newest line.
const SCROLL_ID: &str = "cathode-log-scroll";

/// The dropdown options, in order.
const LEVELS: &[(LogLevel, &str)] = &[
    (LogLevel::Off, "Off"),
    (LogLevel::Error, "Error"),
    (LogLevel::Warn, "Warn"),
    (LogLevel::Info, "Info"),
    (LogLevel::Debug, "Debug"),
    (LogLevel::Trace, "Trace"),
];

fn level_value(level: LogLevel) -> &'static str {
    LEVELS
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, label)| *label)
        .unwrap_or("Off")
}

fn level_from_value(value: &str) -> LogLevel {
    LEVELS
        .iter()
        .find(|(_, label)| label.eq_ignore_ascii_case(value))
        .map(|(l, _)| *l)
        .unwrap_or(LogLevel::Off)
}

/// Color for a level's badge, matching its severity.
fn level_color(level: &str) -> &'static str {
    match level {
        "error" => "text-red-400",
        "warn" => "text-amber-400",
        "info" => "text-sky-400",
        "debug" => "text-emerald-400",
        _ => "text-neutral-500",
    }
}

#[component]
pub fn LogsPanel(
    logs: Vec<LogLine>,
    level: LogLevel,
    on_set_level: EventHandler<LogLevel>,
    on_copy: EventHandler<()>,
    on_clear: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    let current = level_value(level);

    // Keep the view pinned to the newest line whenever the set of lines changes.
    let count = logs.len();
    use_effect(use_reactive!(|count| {
        if count > 0 {
            let _ = document::eval(&format!(
                "var e=document.getElementById('{SCROLL_ID}'); if(e){{e.scrollTop=e.scrollHeight;}}"
            ));
        }
    }));

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-4 \
                sm:items-center",
            onclick: move |_| on_close.call(()),
            div {
                class: "flex max-h-[80vh] w-full max-w-3xl flex-col rounded-xl bg-white \
                    text-neutral-900 shadow-xl dark:bg-neutral-900 dark:text-neutral-100",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "flex items-center gap-3 border-b border-neutral-200 px-5 py-3 \
                        dark:border-neutral-800",
                    h2 { class: "text-base font-semibold", "Logs" }
                    label {
                        class: "ml-2 flex items-center gap-2 text-sm text-neutral-500",
                        "Level"
                        select {
                            class: "rounded-md border border-neutral-300 bg-transparent px-2 py-1 \
                                text-sm text-neutral-900 focus:outline-none focus:ring-2 \
                                focus:ring-sky-400 dark:border-neutral-700 dark:text-neutral-100",
                            value: current,
                            onchange: move |e| on_set_level.call(level_from_value(&e.value())),
                            for (_, name) in LEVELS.iter() {
                                option { value: *name, {*name} }
                            }
                        }
                    }
                    div {
                        class: "ml-auto flex items-center gap-1",
                        button {
                            class: BTN,
                            title: "Copy",
                            onclick: move |_| on_copy.call(()),
                            Copy { class: ICON }
                        }
                        button {
                            class: BTN,
                            title: "Clear",
                            onclick: move |_| on_clear.call(()),
                            Trash { class: ICON }
                        }
                        button {
                            class: BTN,
                            title: "Close",
                            onclick: move |_| on_close.call(()),
                            Close { class: ICON }
                        }
                    }
                }

                if logs.is_empty() {
                    p {
                        class: "px-5 py-10 text-center text-sm text-neutral-500",
                        if level == LogLevel::Off {
                            "Logging is off — pick a level to start capturing."
                        } else {
                            "No log lines captured yet."
                        }
                    }
                } else {
                    div {
                        id: SCROLL_ID,
                        class: "m-4 flex-1 overflow-auto rounded-md bg-neutral-950 p-3 font-mono \
                            text-xs leading-relaxed",
                        for line in logs.iter() {
                            div {
                                class: "flex gap-3",
                                span { class: "shrink-0 text-neutral-500", "{line.time}" }
                                span {
                                    class: "w-12 shrink-0 font-semibold uppercase {level_color(&line.level)}",
                                    "{line.level}"
                                }
                                span {
                                    class: "min-w-0 flex-1 break-words text-neutral-200",
                                    span { class: "text-neutral-500", "{line.target}: " }
                                    "{line.message}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

const BTN: &str = "rounded-full p-2 text-neutral-500 hover:bg-neutral-100 \
    focus:outline-none focus:ring-2 focus:ring-sky-400 dark:text-neutral-300 \
    dark:hover:bg-neutral-800";
const ICON: &str = "h-5 w-5";
