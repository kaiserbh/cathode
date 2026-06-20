//! The browse screen: connect to an Xtream account, then explore its live
//! categories and channels. Owns all the state; the components are presentational.

use cathode_core::error::AppError;
use cathode_core::model::{Category, CategoryId, Stream};
use cathode_core::sources::xtream::XtreamCredentials;
use dioxus::prelude::*;

use crate::bindings;
use crate::components::{CategoryList, ConnectForm, StreamGrid};

#[component]
pub fn Browse() -> Element {
    let creds = use_signal(|| None::<XtreamCredentials>);
    let mut categories = use_signal(Vec::<Category>::new);
    let mut selected = use_signal(|| None::<CategoryId>);
    let mut streams = use_signal(Vec::<Stream>::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<AppError>);

    let mut creds_w = creds;
    let on_connect = move |new_creds: XtreamCredentials| {
        creds_w.set(Some(new_creds.clone()));
        selected.set(None);
        streams.set(Vec::new());
        categories.set(Vec::new());
        error.set(None);
        loading.set(true);
        spawn(async move {
            match bindings::list_categories(&new_creds).await {
                Ok(list) => categories.set(list),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let on_select = move |id: CategoryId| {
        let Some(current) = creds.read().clone() else {
            return;
        };
        selected.set(Some(id.clone()));
        streams.set(Vec::new());
        error.set(None);
        loading.set(true);
        spawn(async move {
            match bindings::list_streams(&current, Some(&id.0)).await {
                Ok(list) => streams.set(list),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let connected = creds.read().is_some();

    rsx! {
        div {
            class: "min-h-screen flex flex-col bg-white text-neutral-900 \
                dark:bg-neutral-950 dark:text-neutral-100",
            header {
                class: "flex flex-col gap-3 border-b border-neutral-200 \
                    dark:border-neutral-800 p-4",
                h1 { class: "text-lg font-semibold", "Cathode" }
                ConnectForm { connecting: loading(), on_connect }
            }

            if let Some(err) = error() {
                div {
                    class: "m-4 rounded-md border border-red-300 bg-red-50 px-4 py-2 text-sm \
                        text-red-800 dark:border-red-800 dark:bg-red-950 dark:text-red-200",
                    "[{err.code}] {err.message}"
                }
            }

            if connected {
                div {
                    class: "flex flex-col md:flex-row flex-1 min-h-0",
                    CategoryList {
                        categories: categories(),
                        selected: selected(),
                        on_select,
                    }
                    main {
                        class: "flex-1 overflow-y-auto",
                        if loading() {
                            p { class: "p-6 text-sm text-neutral-500", "Loading…" }
                        } else {
                            StreamGrid { streams: streams() }
                        }
                    }
                }
            } else {
                p {
                    class: "p-6 text-sm text-neutral-500",
                    "Enter your Xtream details above to browse live channels."
                }
            }
        }
    }
}
