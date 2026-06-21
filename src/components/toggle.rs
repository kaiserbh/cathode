//! A switch toggle. The single source of truth for on/off switches, so the layout
//! is correct everywhere it's used.
//!
//! Layout note: the track is a flex container with padding and the knob is a flex
//! child, so the knob is geometrically contained — `w-11` (44px) minus `p-0.5`
//! (2+2) leaves 40px, and a 20px knob shifted by 20px (`translate-x-5`) lands flush
//! inside with a 2px margin. It cannot overrun the track regardless of origin.

use dioxus::prelude::*;

#[component]
pub fn Toggle(value: bool, on_toggle: EventHandler<bool>) -> Element {
    let track = if value {
        "bg-sky-600"
    } else {
        "bg-neutral-300 dark:bg-neutral-700"
    };
    let knob = if value {
        "translate-x-5"
    } else {
        "translate-x-0"
    };
    rsx! {
        button {
            r#type: "button",
            class: "inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full \
                p-0.5 transition-colors focus:outline-none focus:ring-2 focus:ring-sky-400 {track}",
            "aria-pressed": "{value}",
            onclick: move |_| on_toggle.call(!value),
            span {
                class: "h-5 w-5 rounded-full bg-white shadow transition-transform {knob}",
            }
        }
    }
}
