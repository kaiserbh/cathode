//! On/off switch. A thin wrapper over the dioxus-primitives `Switch` so call sites keep
//! a simple `value` / `on_toggle` API and the switch styling stays consistent with the
//! rest of the dx components.

use dioxus::prelude::*;

use crate::ui::switch::Switch;

#[component]
pub fn Toggle(value: bool, on_toggle: EventHandler<bool>) -> Element {
    rsx! {
        Switch {
            checked: Some(value),
            on_checked_change: move |checked: bool| on_toggle.call(checked),
        }
    }
}
