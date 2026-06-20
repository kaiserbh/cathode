#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::views::Browse;

// Compiled by Tailwind (via dx) from /tailwind.css at build time.
static TAILWIND: Asset = asset!("/assets/tailwind.css");

pub fn App() -> Element {
    rsx! {
        link { rel: "stylesheet", href: TAILWIND }
        Browse {}
    }
}
