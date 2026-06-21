//! The player overlay. A transparent full-screen layer so the embedded mpv
//! surface shows through, with a control bar docked at the bottom that hides
//! itself (and the cursor) after a short idle and reappears on mouse movement.

use cathode_core::model::{NowNext, Stream};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use js_sys::Date;

use crate::format::hhmm;

const CONTROL: &str = "rounded-md bg-white/10 px-4 py-2 text-sm font-medium text-white \
    hover:bg-white/20 focus:outline-none focus:ring-2 focus:ring-sky-400";

/// How long the pointer must be still before the controls hide, in milliseconds.
const IDLE_MS: f64 = 2500.0;

#[component]
pub fn PlayerOverlay(
    stream: Stream,
    paused: bool,
    now_next: Option<NowNext>,
    on_pause: EventHandler<()>,
    on_resume: EventHandler<()>,
    on_stop: EventHandler<()>,
) -> Element {
    let mut visible = use_signal(|| true);
    let mut last_active = use_signal(Date::now);

    // Poll for inactivity instead of arming a timer on every mouse move: a move
    // just records a timestamp, and this loop hides the controls once they have
    // been idle past the threshold.
    use_future(move || async move {
        loop {
            TimeoutFuture::new(400).await;
            if visible() && Date::now() - last_active() > IDLE_MS {
                visible.set(false);
            }
        }
    });

    let bar_visibility = if visible() {
        "opacity-100"
    } else {
        "opacity-0 pointer-events-none"
    };
    let cursor = if visible() { "" } else { "cursor-none" };

    rsx! {
        // Transparent so mpv (composited behind the webview) is visible. Tracks
        // pointer movement to reveal the controls.
        div {
            class: "min-h-screen flex flex-col justify-end {cursor}",
            onmousemove: move |_| {
                last_active.set(Date::now());
                if !visible() {
                    visible.set(true);
                }
            },
            div {
                class: "flex items-center gap-3 bg-black/60 p-4 text-white backdrop-blur-sm \
                    transition-opacity duration-300 {bar_visibility}",
                div {
                    class: "min-w-0 flex-1",
                    span { class: "block truncate text-sm font-medium", "{stream.name}" }
                    if let Some(nn) = now_next.as_ref() {
                        if let Some(now) = nn.now.as_ref() {
                            span {
                                class: "block truncate text-xs text-white/70",
                                "Now: {now.title} · {hhmm(now.start)}–{hhmm(now.stop)}"
                            }
                        }
                        if let Some(next) = nn.next.as_ref() {
                            span {
                                class: "block truncate text-[11px] text-white/50",
                                "Next: {next.title} · {hhmm(next.start)}"
                            }
                        }
                    }
                }
                if paused {
                    button {
                        class: CONTROL,
                        onclick: move |_| on_resume.call(()),
                        "Resume"
                    }
                } else {
                    button {
                        class: CONTROL,
                        onclick: move |_| on_pause.call(()),
                        "Pause"
                    }
                }
                button {
                    class: CONTROL,
                    onclick: move |_| on_stop.call(()),
                    "Stop"
                }
            }
        }
    }
}
