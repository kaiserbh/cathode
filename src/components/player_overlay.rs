//! The player overlay. A transparent full-screen layer so the embedded mpv
//! surface shows through, with a control bar docked at the bottom.

use cathode_core::model::Stream;
use dioxus::prelude::*;

const CONTROL: &str = "rounded-md bg-white/10 px-4 py-2 text-sm font-medium text-white \
    hover:bg-white/20 focus:outline-none focus:ring-2 focus:ring-sky-400";

#[component]
pub fn PlayerOverlay(
    stream: Stream,
    paused: bool,
    on_pause: EventHandler<()>,
    on_resume: EventHandler<()>,
    on_stop: EventHandler<()>,
) -> Element {
    rsx! {
        // Transparent so mpv (composited behind the webview) is visible.
        div {
            class: "min-h-screen flex flex-col justify-end",
            div {
                class: "flex items-center gap-3 bg-black/60 p-4 text-white backdrop-blur-sm",
                span { class: "flex-1 truncate text-sm font-medium", "{stream.name}" }
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
