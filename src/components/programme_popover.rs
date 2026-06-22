//! A small detail card for a single EPG programme, anchored to the clicked cell.
//!
//! dx Popover positions its content inline and would be clipped by the guide's scroll
//! container, so this is a fixed-position card rendered at the app root from the click
//! coordinates (clamped to the viewport). A transparent backdrop and Escape close it.

use cathode_core::model::Programme;
use dioxus::prelude::*;

use crate::components::icons::{Close, Play};
use crate::format::hhmm;
use crate::ui::button::{Button, ButtonSize, ButtonVariant};

/// Card width in rem; used to clamp the anchor so the card stays on-screen.
const CARD_W_REM: f64 = 18.0;

#[component]
pub fn ProgrammePopover(
    programme: Programme,
    /// Click coordinates (viewport pixels) to anchor the card near.
    x: f64,
    y: f64,
    on_play: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    // Clamp so the card never spills past the viewport edges.
    let style = format!(
        "left: clamp(0.5rem, {x}px, calc(100vw - {CARD_W_REM}rem - 0.5rem)); \
         top: clamp(0.5rem, {y}px, calc(100vh - 14rem));"
    );
    rsx! {
        // Transparent backdrop: a click anywhere outside the card closes it.
        div {
            class: "fixed inset-0 z-50",
            onclick: move |_| on_close.call(()),
            tabindex: "0",
            autofocus: true,
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    on_close.call(());
                }
            },
            div {
                class: "fixed w-72 rounded-lg border border-neutral-200 bg-white p-4 shadow-xl \
                    dark:border-neutral-700 dark:bg-neutral-900",
                style,
                onclick: move |e| e.stop_propagation(),
                div { class: "flex items-start gap-2",
                    h3 {
                        class: "min-w-0 flex-1 text-sm font-semibold text-neutral-900 \
                            dark:text-neutral-100",
                        {programme.title.clone()}
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::IconXs,
                        title: "Close",
                        onclick: move |_| on_close.call(()),
                        Close { class: "h-4 w-4" }
                    }
                }
                p { class: "mt-0.5 text-xs text-neutral-500 dark:text-neutral-400",
                    "{hhmm(programme.start)} – {hhmm(programme.stop)}"
                }
                if let Some(desc) = programme.description.as_ref() {
                    p { class: "mt-2 max-h-40 overflow-auto text-sm text-neutral-700 \
                            dark:text-neutral-300",
                        {desc.clone()}
                    }
                }
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    class: "mt-3 w-full justify-center",
                    onclick: move |_| on_play.call(()),
                    Play { class: "h-4 w-4" }
                    "Play channel"
                }
            }
        }
    }
}
