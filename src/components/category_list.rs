//! The category picker. A horizontal scroll row on small screens, a vertical
//! sidebar on md+ (the single `flex md:flex-col` switch handles both).

use cathode_core::model::{Category, CategoryId};
use dioxus::prelude::*;

#[component]
pub fn CategoryList(
    categories: Vec<Category>,
    selected: Option<CategoryId>,
    on_select: EventHandler<CategoryId>,
) -> Element {
    rsx! {
        nav {
            class: "flex md:flex-col gap-1 overflow-x-auto md:overflow-y-auto \
                md:w-56 md:shrink-0 border-b md:border-b-0 md:border-r \
                border-neutral-200 dark:border-neutral-800 p-2",
            {categories.iter().map(|category| {
                let id = category.id.clone();
                let is_selected = selected.as_ref() == Some(&category.id);
                let base = "shrink-0 text-left rounded-md px-3 py-2 text-sm whitespace-nowrap \
                    focus:outline-none focus:ring-2 focus:ring-sky-500";
                let state = if is_selected {
                    "bg-sky-600 text-white"
                } else {
                    "hover:bg-neutral-200 dark:hover:bg-neutral-800"
                };
                rsx! {
                    button {
                        key: "{category.id.0}",
                        class: "{base} {state}",
                        onclick: move |_| on_select.call(id.clone()),
                        "{category.name}"
                    }
                }
            })}
        }
    }
}
