//! The Logs panel: a modal that shows the captured debug log, with a level dropdown
//! (Off disables capture), plus Copy and Clear actions for attaching logs to a bug
//! report. Presentational — `Browse` owns the polling and the handlers.

use cathode_core::model::LogLevel;
use dioxus::prelude::*;

use crate::components::icons::Close;

/// The dropdown options, in order: `(stored value, label)`.
const LEVELS: &[(LogLevel, &str)] = &[
    (LogLevel::Off, "Off"),
    (LogLevel::Error, "Error"),
    (LogLevel::Warn, "Warn"),
    (LogLevel::Info, "Info"),
    (LogLevel::Debug, "Debug"),
    (LogLevel::Trace, "Trace"),
];

/// The lowercase wire spelling of a level (matches its serde representation).
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

#[component]
pub fn LogsPanel(
    logs: Vec<String>,
    level: LogLevel,
    on_set_level: EventHandler<LogLevel>,
    on_copy: EventHandler<()>,
    on_clear: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    let current = level_value(level);
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
                            onclick: move |_| on_copy.call(()),
                            "Copy"
                        }
                        button {
                            class: BTN,
                            onclick: move |_| on_clear.call(()),
                            "Clear"
                        }
                        button {
                            class: "rounded-full p-1.5 text-neutral-500 hover:bg-neutral-100 \
                                dark:hover:bg-neutral-800",
                            title: "Close",
                            onclick: move |_| on_close.call(()),
                            Close { class: "h-5 w-5" }
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
                    pre {
                        class: "m-4 flex-1 overflow-auto rounded-md bg-neutral-950 p-3 text-xs \
                            leading-relaxed text-neutral-200 whitespace-pre-wrap break-all",
                        for line in logs.iter() {
                            "{line}\n"
                        }
                    }
                }
            }
        }
    }
}

const BTN: &str = "rounded-md px-3 py-1.5 text-sm font-medium text-neutral-700 \
    hover:bg-neutral-100 focus:outline-none focus:ring-2 focus:ring-sky-400 \
    dark:text-neutral-300 dark:hover:bg-neutral-800";
