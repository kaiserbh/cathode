//! A shared modal shell for the app's panels, built on the dioxus-primitives Dialog.
//! It supplies the backdrop, focus-trap, and escape-to-close behaviour while overriding
//! the dx-dialog container styling so panels keep Cathode's look: edge-to-edge headers
//! and rows, a solid panel background, left-aligned, no built-in padding. The caller
//! passes a `max-w-*` width class and the panel's header + body as children.

use dioxus::prelude::*;

use crate::ui::dialog::Dialog;

#[component]
pub fn PanelDialog(
    /// Extra Tailwind classes for the container — at least a `max-w-*` width, plus any
    /// `max-h-*`/`overflow-*` a scrolling panel needs.
    class: String,
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        Dialog {
            open: Some(true),
            on_open_change: move |open: bool| if !open { on_close.call(()) },
            class: "{class} w-full p-0! gap-0! rounded-xl! border-0! text-left! shadow-xl \
                bg-white! text-neutral-900! dark:bg-neutral-900! dark:text-neutral-100!",
            {children}
        }
    }
}
