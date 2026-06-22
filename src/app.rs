#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::views::Browse;

// Compiled by Tailwind (via dx) from /tailwind.css at build time.
static TAILWIND: Asset = asset!("/assets/tailwind.css");
// CSS-variable theme for the vendored dioxus-primitives components (src/ui/). The
// per-component scoped CSS references these variables, so this must load once at root.
static DX_THEME: Asset = asset!("/assets/dx-components-theme.css");

pub fn App() -> Element {
    rsx! {
        link { rel: "stylesheet", href: TAILWIND }
        link { rel: "stylesheet", href: DX_THEME }
        Browse {}
    }
}
