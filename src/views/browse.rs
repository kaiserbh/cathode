//! The browse screen: pick (or add) an Xtream account, then explore its live
//! channels, favorites, and watch history. Owns all the state; components are
//! presentational.
//!
//! Sources, favorites, history, and settings are persisted. On launch the
//! most-recently-used account is auto-opened from cache and refreshed. An Options
//! panel toggles features (favorites, history) and an incognito session switch;
//! nothing is forced on the user.

use std::collections::HashMap;

use cathode_core::error::AppError;
use cathode_core::model::{Category, CategoryId, ChannelView, NowNext, Settings, Stream, StreamId};
use cathode_core::sources::xtream::XtreamCredentials;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::bindings;
use crate::components::{
    CategoryList, ChannelPane, PlayerOverlay, SettingsPanel, SourcesPanel, Spinner, Tab, TabBar,
};

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
    let mut settings = use_signal(Settings::default);
    let mut incognito = use_signal(|| false);
    let mut favorites = use_signal(Vec::<Stream>::new);
    let mut history = use_signal(Vec::<Stream>::new);
    let mut show_settings = use_signal(|| false);
    let mut tab = use_signal(|| Tab::Channels);
    let mut epg = use_signal(HashMap::<String, NowNext>::new);

    // Make a source active: paint its cached channels instantly, refresh from the
    // network, and load its favorites + history.
    let activate = use_callback(move |new_creds: XtreamCredentials| {
        creds.set(Some(new_creds.clone()));
        selected.set(None);
        streams.set(Vec::new());
        categories.set(Vec::new());
        favorites.set(Vec::new());
        history.set(Vec::new());
        epg.set(HashMap::new());
        tab.set(Tab::Channels);
        error.set(None);

        let cached_creds = new_creds.clone();
        spawn(async move {
            if let Ok(cached) = bindings::cached_categories(&cached_creds).await {
                if !cached.is_empty() && categories.read().is_empty() {
                    categories.set(cached);
                }
            }
        });

        let fav_creds = new_creds.clone();
        spawn(async move {
            if let Ok(list) = bindings::list_favorites(&fav_creds).await {
                favorites.set(list);
            }
        });
        let hist_creds = new_creds.clone();
        spawn(async move {
            if let Ok(list) = bindings::list_history(&hist_creds).await {
                history.set(list);
            }
        });

        if settings.read().epg_enabled {
            let epg_creds = new_creds.clone();
            spawn(async move {
                // EPG is best-effort: a provider without a guide just shows none.
                if let Ok(map) = bindings::epg_now_next(&epg_creds).await {
                    epg.set(map);
                }
            });
        }

        loading.set(true);
        spawn(async move {
            match bindings::list_categories(&new_creds).await {
                Ok(list) => categories.set(list),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    // On launch: load saved accounts (auto-open the most recent) and settings.
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
    use_future(move || async move {
        if let Ok(s) = bindings::get_settings().await {
            settings.set(s);
        }
    });

    // Keep now/next current as time passes. The guide is cached server-side after the
    // first fetch, so this just recomputes against the clock — no re-download.
    use_future(move || async move {
        loop {
            TimeoutFuture::new(60_000).await;
            if !settings.read().epg_enabled {
                continue;
            }
            if let Some(current) = creds.read().clone() {
                if let Ok(map) = bindings::epg_now_next(&current).await {
                    epg.set(map);
                }
            }
        }
    });

    // Persist a settings change and reflect it locally.
    let save_settings = use_callback(move |new: Settings| {
        settings.set(new);
        spawn(async move {
            let _ = bindings::set_settings(&new).await;
        });
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
            favorites.set(Vec::new());
            history.set(Vec::new());
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
                // Drop the result if the user has since switched categories, and
                // only paint the cache if the network hasn't already answered.
                if selected.read().as_ref() == Some(&cached_id)
                    && !cached.is_empty()
                    && streams.read().is_empty()
                {
                    streams.set(cached);
                }
            }
        });

        loading.set(true);
        let net_id = id.clone();
        spawn(async move {
            let result = bindings::list_streams(&current, Some(&id.0)).await;
            // A response for a category the user already left is stale — drop it.
            if selected.read().as_ref() != Some(&net_id) {
                return;
            }
            match result {
                Ok(list) => streams.set(list),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let on_play = use_callback(move |stream: Stream| {
        let Some(current) = creds.read().clone() else {
            return;
        };
        let provider_id = stream.provider_id.clone();
        playing.set(Some(stream.clone()));
        paused.set(false);
        error.set(None);

        let play_creds = current.clone();
        spawn(async move {
            if let Err(e) = bindings::play_stream(&play_creds, &provider_id).await {
                error.set(Some(e));
            }
        });

        // Record to history only when enabled and not incognito.
        if settings.read().history_enabled && !incognito() {
            history.write().retain(|s| s.id != stream.id);
            history.write().insert(0, stream.clone());
            let watch_creds = current.clone();
            spawn(async move {
                let _ = bindings::record_watch(&watch_creds, &stream).await;
            });
        }
    });

    let toggle_favorite = use_callback(move |stream: Stream| {
        let Some(current) = creds.read().clone() else {
            return;
        };
        let is_favorite = favorites.read().iter().any(|s| s.id == stream.id);
        if is_favorite {
            favorites.write().retain(|s| s.id != stream.id);
            let id = stream.id.0.clone();
            spawn(async move {
                let _ = bindings::remove_favorite(&current, &id).await;
            });
        } else {
            favorites.write().insert(0, stream.clone());
            spawn(async move {
                let _ = bindings::add_favorite(&current, &stream).await;
            });
        }
    });

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
    let current_settings = settings();
    let favorites_enabled = current_settings.favorites_enabled;
    let history_enabled = current_settings.history_enabled;
    let channel_view = current_settings.channel_view;
    let favorite_ids: Vec<StreamId> = favorites().iter().map(|s| s.id.clone()).collect();

    // A disabled feature's tab falls back to Channels.
    let current_tab = match tab() {
        Tab::Favorites if !favorites_enabled => Tab::Channels,
        Tab::History if !history_enabled => Tab::Channels,
        t => t,
    };

    rsx! {
        div {
            class: "h-screen overflow-hidden flex flex-col bg-white text-neutral-900 \
                dark:bg-neutral-950 dark:text-neutral-100",
            header {
                class: "shrink-0 flex items-center justify-between border-b border-neutral-200 \
                    dark:border-neutral-800 p-4",
                div {
                    class: "flex items-center gap-3",
                    h1 { class: "text-lg font-semibold", "Cathode" }
                    if incognito() {
                        span {
                            class: "rounded-full bg-neutral-800 px-2 py-0.5 text-xs \
                                font-medium text-neutral-100 dark:bg-neutral-200 \
                                dark:text-neutral-900",
                            "Incognito"
                        }
                    }
                }
                div {
                    class: "flex items-center gap-2",
                    HeaderButton { label: "Options", onclick: move |_| show_settings.set(!show_settings()) }
                    HeaderButton { label: "Sources", onclick: move |_| show_sources.set(!show_sources()) }
                }
            }

            if let Some(err) = error() {
                div {
                    class: "shrink-0 m-4 rounded-md border border-red-300 bg-red-50 px-4 py-2 \
                        text-sm text-red-800 dark:border-red-800 dark:bg-red-950 \
                        dark:text-red-200",
                    "[{err.code}] {err.message}"
                }
            }

            if connected {
                TabBar {
                    active: current_tab,
                    show_favorites: favorites_enabled,
                    show_history: history_enabled,
                    on_select: move |t| tab.set(t),
                }
                match current_tab {
                    Tab::Channels => rsx! {
                        div {
                            class: "flex flex-col md:flex-row flex-1 min-h-0",
                            CategoryList {
                                categories: categories(),
                                selected: selected(),
                                on_select,
                            }
                            main {
                                class: "flex-1 min-h-0 overflow-y-auto",
                                if loading() && streams().is_empty() {
                                    Spinner {}
                                } else {
                                    ChannelPane {
                                        view: channel_view,
                                        streams: streams(),
                                        favorites_enabled,
                                        favorite_ids: favorite_ids.clone(),
                                        epg: epg(),
                                        on_play: move |s| on_play.call(s),
                                        on_toggle_favorite: move |s| toggle_favorite.call(s),
                                    }
                                }
                            }
                        }
                    },
                    Tab::Favorites => rsx! {
                        main {
                            class: "flex-1 overflow-y-auto",
                            if favorites().is_empty() {
                                p {
                                    class: "p-6 text-sm text-neutral-500",
                                    "No favorites yet. Tap the star on a channel to add one."
                                }
                            } else {
                                ChannelPane {
                                    view: channel_view,
                                    streams: favorites(),
                                    favorites_enabled,
                                    favorite_ids: favorite_ids.clone(),
                                    epg: epg(),
                                    on_play: move |s| on_play.call(s),
                                    on_toggle_favorite: move |s| toggle_favorite.call(s),
                                }
                            }
                        }
                    },
                    Tab::History => rsx! {
                        main {
                            class: "flex-1 overflow-y-auto",
                            if history().is_empty() {
                                p {
                                    class: "p-6 text-sm text-neutral-500",
                                    "Nothing watched yet."
                                }
                            } else {
                                ChannelPane {
                                    view: channel_view,
                                    streams: history(),
                                    favorites_enabled,
                                    favorite_ids: favorite_ids.clone(),
                                    epg: epg(),
                                    on_play: move |s| on_play.call(s),
                                    on_toggle_favorite: move |s| toggle_favorite.call(s),
                                }
                            }
                        }
                    },
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

        if show_settings() {
            SettingsPanel {
                settings: current_settings,
                incognito: incognito(),
                on_toggle_favorites: move |v| {
                    let mut s = settings();
                    s.favorites_enabled = v;
                    save_settings.call(s);
                },
                on_toggle_history: move |v| {
                    let mut s = settings();
                    s.history_enabled = v;
                    save_settings.call(s);
                },
                on_toggle_epg: move |v| {
                    let mut s = settings();
                    s.epg_enabled = v;
                    save_settings.call(s);
                    if v {
                        if let Some(current) = creds.read().clone() {
                            spawn(async move {
                                if let Ok(map) = bindings::epg_now_next(&current).await {
                                    epg.set(map);
                                }
                            });
                        }
                    } else {
                        epg.set(HashMap::new());
                    }
                },
                on_set_view: move |v: ChannelView| {
                    let mut s = settings();
                    s.channel_view = v;
                    save_settings.call(s);
                },
                on_toggle_incognito: move |v| incognito.set(v),
                on_clear_history: move |_| {
                    history.set(Vec::new());
                    spawn(async move {
                        let _ = bindings::clear_history().await;
                    });
                },
                on_close: move |_| show_settings.set(false),
            }
        }
    }
}

#[component]
fn HeaderButton(label: String, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-neutral-300 px-3 py-2 text-sm font-medium \
                hover:bg-neutral-100 focus:outline-none focus:ring-2 focus:ring-sky-400 \
                dark:border-neutral-700 dark:hover:bg-neutral-800",
            onclick: move |e| onclick.call(e),
            "{label}"
        }
    }
}
