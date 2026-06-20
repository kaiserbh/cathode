//! The browse screen: pick (or add) an Xtream account, then explore its live
//! categories and channels. Owns all the state; the components are presentational.
//!
//! Sources are persisted: on launch the most-recently-used account is auto-opened
//! from cache and refreshed in the background, and a Sources panel switches between,
//! adds, or removes accounts.

use cathode_core::error::AppError;
use cathode_core::model::{Category, CategoryId, Stream};
use cathode_core::sources::xtream::XtreamCredentials;
use dioxus::prelude::*;

use crate::bindings;
use crate::components::{CategoryList, PlayerOverlay, SourcesPanel, Spinner, StreamGrid};

/// Move a source to the front of the most-recently-used list (de-duplicated).
fn bump_source(mut sources: Signal<Vec<XtreamCredentials>>, c: XtreamCredentials) {
    let mut list = sources.write();
    list.retain(|s| s != &c);
    list.insert(0, c);
}

#[component]
pub fn Browse() -> Element {
    let mut creds = use_signal(|| None::<XtreamCredentials>);
    let mut categories = use_signal(Vec::<Category>::new);
    let mut selected = use_signal(|| None::<CategoryId>);
    let mut streams = use_signal(Vec::<Stream>::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<AppError>);
    let mut playing = use_signal(|| None::<Stream>);
    let mut paused = use_signal(|| false);
    let mut sources = use_signal(Vec::<XtreamCredentials>::new);
    let mut show_sources = use_signal(|| false);

    // Make a source active: paint its cached categories instantly, then refresh
    // from the network (which also persists the source and updates the cache).
    let activate = use_callback(move |new_creds: XtreamCredentials| {
        creds.set(Some(new_creds.clone()));
        selected.set(None);
        streams.set(Vec::new());
        categories.set(Vec::new());
        error.set(None);

        let cached_creds = new_creds.clone();
        spawn(async move {
            if let Ok(cached) = bindings::cached_categories(&cached_creds).await {
                // Only paint the cache if the network hasn't already answered.
                if !cached.is_empty() && categories.read().is_empty() {
                    categories.set(cached);
                }
            }
        });

        loading.set(true);
        spawn(async move {
            match bindings::list_categories(&new_creds).await {
                Ok(list) => categories.set(list),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    // On launch, load saved accounts and auto-open the most recent one.
    use_future(move || async move {
        match bindings::saved_sources().await {
            Ok(list) if !list.is_empty() => {
                sources.set(list.clone());
                activate.call(list[0].clone());
            }
            Ok(_) => show_sources.set(true),
            Err(e) => {
                error.set(Some(e));
                show_sources.set(true);
            }
        }
    });

    let on_connect = move |new_creds: XtreamCredentials| {
        activate.call(new_creds.clone());
        bump_source(sources, new_creds);
        show_sources.set(false);
    };

    let on_select_source = move |c: XtreamCredentials| {
        activate.call(c.clone());
        bump_source(sources, c);
        show_sources.set(false);
    };

    let on_forget = move |c: XtreamCredentials| {
        let target = c.clone();
        spawn(async move {
            let _ = bindings::forget_source(&target).await;
        });
        sources.write().retain(|s| s != &c);
        if creds.read().as_ref() == Some(&c) {
            creds.set(None);
            categories.set(Vec::new());
            streams.set(Vec::new());
            selected.set(None);
        }
    };

    let on_select = move |id: CategoryId| {
        let Some(current) = creds.read().clone() else {
            return;
        };
        selected.set(Some(id.clone()));
        streams.set(Vec::new());
        error.set(None);

        let cached_creds = current.clone();
        let cached_id = id.clone();
        spawn(async move {
            if let Ok(cached) = bindings::cached_streams(&cached_creds, &cached_id.0).await {
                if !cached.is_empty() && streams.read().is_empty() {
                    streams.set(cached);
                }
            }
        });

        loading.set(true);
        spawn(async move {
            match bindings::list_streams(&current, Some(&id.0)).await {
                Ok(list) => streams.set(list),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let on_play = move |stream: Stream| {
        let Some(current) = creds.read().clone() else {
            return;
        };
        let provider_id = stream.provider_id.clone();
        playing.set(Some(stream));
        paused.set(false);
        error.set(None);
        spawn(async move {
            if let Err(e) = bindings::play_stream(&current, &provider_id).await {
                error.set(Some(e));
            }
        });
    };

    let on_pause = move |_| {
        paused.set(true);
        spawn(async move {
            let _ = bindings::pause().await;
        });
    };
    let on_resume = move |_| {
        paused.set(false);
        spawn(async move {
            let _ = bindings::resume().await;
        });
    };
    let on_stop = move |_| {
        playing.set(None);
        spawn(async move {
            let _ = bindings::stop().await;
        });
    };

    // While playing, render a transparent overlay so the embedded mpv surface
    // shows through; otherwise render the opaque browse UI.
    if let Some(stream) = playing() {
        return rsx! {
            PlayerOverlay {
                stream,
                paused: paused(),
                on_pause,
                on_resume,
                on_stop,
            }
        };
    }

    let connected = creds.read().is_some();

    rsx! {
        div {
            class: "min-h-screen flex flex-col bg-white text-neutral-900 \
                dark:bg-neutral-950 dark:text-neutral-100",
            header {
                class: "flex items-center justify-between border-b border-neutral-200 \
                    dark:border-neutral-800 p-4",
                h1 { class: "text-lg font-semibold", "Cathode" }
                button {
                    class: "rounded-md border border-neutral-300 px-3 py-2 text-sm \
                        font-medium hover:bg-neutral-100 focus:outline-none focus:ring-2 \
                        focus:ring-sky-400 dark:border-neutral-700 dark:hover:bg-neutral-800",
                    onclick: move |_| show_sources.set(!show_sources()),
                    "Sources"
                }
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
                        // Show the spinner only when there's nothing cached to show.
                        // With cached channels present we render them immediately and
                        // let the background refresh swap in fresh data silently.
                        if loading() && streams().is_empty() {
                            Spinner {}
                        } else {
                            StreamGrid { streams: streams(), on_play }
                        }
                    }
                }
            } else {
                p {
                    class: "p-6 text-sm text-neutral-500",
                    "Open Sources to add or pick an Xtream account."
                }
            }
        }

        if show_sources() {
            SourcesPanel {
                sources: sources(),
                active: creds(),
                connecting: loading(),
                on_select: on_select_source,
                on_forget,
                on_connect,
                on_close: move |_| show_sources.set(false),
            }
        }
    }
}
