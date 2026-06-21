//! The Logs panel: a modal showing the captured debug log as colored, aligned rows,
//! with a level dropdown (Off disables capture), a search box, and icon actions to copy
//! or clear. The visible rows are filtered to the selected level and the search query,
//! and the view auto-scrolls to the newest line. Presentational — `Browse` owns the
//! polling and the handlers.

use cathode_core::model::{LogLevel, LogLine};
use dioxus::prelude::*;

use crate::components::icons::{Close, Copy, Search, Trash};

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

/// Severity rank of the selected level (0 = Off shows nothing, 5 = Trace shows all).
fn selected_rank(level: LogLevel) -> u8 {
    match level {
        LogLevel::Off => 0,
        LogLevel::Error => 1,
        LogLevel::Warn => 2,
        LogLevel::Info => 3,
        LogLevel::Debug => 4,
        LogLevel::Trace => 5,
    }
}

/// Severity rank of a captured line's level string.
fn line_rank(level: &str) -> u8 {
    match level {
        "error" => 1,
        "warn" => 2,
        "info" => 3,
        "debug" => 4,
        _ => 5,
    }
}

/// Badge color for a level.
fn level_color(level: &str) -> &'static str {
    match level {
        "error" => "text-red-400",
        "warn" => "text-amber-400",
        "info" => "text-sky-400",
        "debug" => "text-emerald-400",
        _ => "text-neutral-500",
    }
}

/// Message tint for a level — colored for signal, but readable.
fn message_color(level: &str) -> &'static str {
    match level {
        "error" => "text-red-300",
        "warn" => "text-amber-200",
        "info" => "text-neutral-100",
        "debug" => "text-neutral-300",
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
    let mut query = use_signal(String::new);

    // Filter to the selected level and the search query.
    let q = query().to_lowercase();
    let max_rank = selected_rank(level);
    let visible: Vec<&LogLine> = logs
        .iter()
        .filter(|l| line_rank(&l.level) <= max_rank)
        .filter(|l| {
            q.is_empty()
                || format!("{} {} {}", l.target, l.message, l.fields)
                    .to_lowercase()
                    .contains(&q)
        })
        .collect();
    let shown = visible.len();
    let total = logs.len();

    // Keep the view pinned to the newest visible line whenever it changes.
    use_effect(use_reactive!(|shown| {
        if shown > 0 {
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
                    h2 { class: "shrink-0 text-base font-semibold", "Logs" }
                    label {
                        class: "flex shrink-0 items-center gap-2 text-sm text-neutral-500",
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
                    // Search box (middle).
                    div {
                        class: "flex min-w-0 flex-1 items-center gap-2 rounded-md bg-neutral-100 \
                            px-2 py-1 focus-within:ring-2 focus-within:ring-sky-400 \
                            dark:bg-neutral-800",
                        Search { class: "h-4 w-4 shrink-0 text-neutral-400" }
                        input {
                            class: "w-full bg-transparent text-sm text-neutral-900 \
                                placeholder:text-neutral-400 focus:outline-none dark:text-neutral-100",
                            placeholder: "Filter logs…",
                            value: "{query}",
                            oninput: move |e| query.set(e.value()),
                        }
                        if !query().is_empty() {
                            span { class: "shrink-0 text-xs text-neutral-400", "{shown}/{total}" }
                            button {
                                class: "shrink-0 rounded p-0.5 text-neutral-400 \
                                    hover:text-neutral-700 dark:hover:text-neutral-200",
                                title: "Clear search",
                                onclick: move |_| query.set(String::new()),
                                Close { class: "h-3.5 w-3.5" }
                            }
                        }
                    }
                    div {
                        class: "ml-auto flex shrink-0 items-center gap-1",
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
                } else if visible.is_empty() {
                    p {
                        class: "px-5 py-10 text-center text-sm text-neutral-500",
                        "No lines match."
                    }
                } else {
                    div {
                        id: SCROLL_ID,
                        class: "m-4 flex-1 overflow-auto rounded-md bg-neutral-950 p-3 font-mono \
                            text-xs leading-relaxed",
                        for line in visible.iter() {
                            div {
                                class: "flex gap-3",
                                span { class: "shrink-0 text-neutral-600", "{line.time}" }
                                span {
                                    class: "w-12 shrink-0 font-semibold uppercase {level_color(&line.level)}",
                                    "{line.level}"
                                }
                                span {
                                    class: "min-w-0 flex-1 break-words",
                                    span { class: "text-indigo-400", "{line.target}: " }
                                    span { class: message_color(&line.level), "{line.message}" }
                                    if !line.fields.is_empty() {
                                        span { class: "text-cyan-400/70", " {line.fields}" }
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

const BTN: &str = "rounded-full p-2 text-neutral-500 hover:bg-neutral-100 \
    focus:outline-none focus:ring-2 focus:ring-sky-400 dark:text-neutral-300 \
    dark:hover:bg-neutral-800";
const ICON: &str = "h-5 w-5";
