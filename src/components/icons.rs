//! Inline SVG icons (Material-style paths), sized by the caller via `class`.

use dioxus::prelude::*;

/// Wrap a path in a 24×24 `currentColor` SVG with the caller's class.
fn icon(class: String, path: &'static str) -> Element {
    rsx! {
        svg {
            class,
            view_box: "0 0 24 24",
            fill: "currentColor",
            xmlns: "http://www.w3.org/2000/svg",
            path { d: path }
        }
    }
}

#[component]
pub fn Play(class: String) -> Element {
    icon(class, "M8 5v14l11-7z")
}

#[component]
pub fn Pause(class: String) -> Element {
    icon(class, "M6 5h4v14H6zM14 5h4v14h-4z")
}

#[component]
pub fn Stop(class: String) -> Element {
    icon(class, "M6 6h12v12H6z")
}

#[component]
pub fn VolumeHigh(class: String) -> Element {
    icon(
        class,
        "M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 \
         2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 \
         6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z",
    )
}

#[component]
pub fn VolumeMuted(class: String) -> Element {
    icon(
        class,
        "M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zM19 \
         12c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 \
         5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 \
         1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z",
    )
}

#[component]
pub fn FullscreenEnter(class: String) -> Element {
    icon(
        class,
        "M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z",
    )
}

#[component]
pub fn FullscreenExit(class: String) -> Element {
    icon(
        class,
        "M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z",
    )
}

#[component]
pub fn Settings(class: String) -> Element {
    icon(
        class,
        "M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z",
    )
}

#[component]
pub fn Sources(class: String) -> Element {
    icon(
        class,
        "M11.99 18.54l-7.37-5.73L3 14.07l9 7 9-7-1.63-1.27-7.38 5.74zM12 16l7.36-5.73L21 9l-9-7-9 7 1.63 1.27L12 16z",
    )
}

#[component]
pub fn Close(class: String) -> Element {
    icon(
        class,
        "M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z",
    )
}

#[component]
pub fn Copy(class: String) -> Element {
    icon(
        class,
        "M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 \
         2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z",
    )
}

#[component]
pub fn Trash(class: String) -> Element {
    icon(
        class,
        "M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z",
    )
}

#[component]
pub fn Bug(class: String) -> Element {
    icon(
        class,
        "M20 8h-2.81c-.45-.78-1.07-1.45-1.82-1.96L17 4.41 15.59 3l-2.17 2.17C12.96 5.06 12.49 5 12 \
         5c-.49 0-.96.06-1.41.17L8.41 3 7 4.41l1.62 1.63C7.88 6.55 7.26 7.22 6.81 8H4v2h2.09c-.05.33-.09.66-.09 \
         1v1H4v2h2.09c0 .34.04.67.09 1H4v2h2.81c1.04 1.79 2.97 3 5.19 3s4.15-1.21 5.19-3H20v-2h-2.09c.05-.33.09-.66.09-1v-1h2v-2h-2v-1c0-.34-.04-.67-.09-1H20V8zm-6 \
         8h-4v-2h4v2zm0-4h-4v-2h4v2z",
    )
}
