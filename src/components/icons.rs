//! Icon set, backed by Lucide via `dioxus-icons`. Each icon is a thin wrapper that
//! keeps the same name and a `class` prop, so call sites size and colour it with the
//! usual Tailwind utilities (`h-5 w-5 text-neutral-600`). Lucide draws with
//! `currentColor`, so the text-colour classes still control the tint.

use dioxus::prelude::*;
use dioxus_icons::lucide;

/// Define a named icon component that forwards `class` to a Lucide icon.
macro_rules! icon {
    ($name:ident => $lucide:ident) => {
        #[component]
        pub fn $name(class: String) -> Element {
            rsx! { lucide::$lucide { class } }
        }
    };
}

icon!(Play => Play);
icon!(Pause => Pause);
icon!(Stop => Square);
icon!(VolumeHigh => Volume2);
icon!(VolumeMuted => VolumeX);
icon!(FullscreenEnter => Maximize);
icon!(FullscreenExit => Minimize);
icon!(Settings => Settings);
icon!(Sources => Layers);
icon!(Close => X);
icon!(Copy => Copy);
icon!(Trash => Trash2);
icon!(Category => Shapes);
icon!(Tv => Tv);
icon!(Film => Film);
icon!(Series => MonitorPlay);
icon!(Search => Search);
icon!(Bug => Bug);
icon!(History => History);
icon!(Keyboard => Keyboard);

/// Favorite star. `filled` solidifies it (the colour comes from the caller's text
/// colour, since Lucide draws with `currentColor`).
#[component]
pub fn Star(class: String, #[props(default)] filled: bool) -> Element {
    rsx! {
        lucide::Star { class, fill: if filled { "currentColor" } else { "none" } }
    }
}
