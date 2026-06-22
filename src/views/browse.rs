//! The browse screen: pick (or add) an Xtream account, then explore its live
//! channels, favorites, and watch history. Owns all the state; components are
//! presentational.
//!
//! Sources, favorites, history, and settings are persisted. On launch the
//! most-recently-used account is auto-opened from cache and refreshed. An Options
//! panel toggles features (favorites, history) and an incognito session switch;
//! nothing is forced on the user.

use std::collections::{HashMap, HashSet};

use cathode_core::error::AppError;
use cathode_core::model::{
    Category, CategoryId, ChannelView, Episode, LogLevel, LogLine, NowNext, Programme, SeriesInfo,
    Settings, Stream, StreamId, StreamKind,
};
use cathode_core::sources::xtream::XtreamCredentials;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::bindings;
use dioxus_primitives::toast::{ToastOptions, use_toast};

use crate::components::{
    CategoryList, ChannelPane, LogsPanel, PlayerOverlay, SearchResults, SeriesDetail,
    SettingsPanel, SourcesPanel, Spinner, Tab, TabBar, TitleBar,
};

/// Move a source to the front of the most-recently-used list (de-duplicated).
fn bump_source(mut sources: Signal<Vec<XtreamCredentials>>, c: XtreamCredentials) {
    let mut list = sources.write();
    list.retain(|s| s != &c);
    list.insert(0, c);
}

/// The content kind a tab browses, or `None` for the library tabs (Favorites/History).
fn tab_kind(tab: Tab) -> Option<StreamKind> {
    match tab {
        Tab::Live => Some(StreamKind::Live),
        Tab::Movies => Some(StreamKind::Vod),
        Tab::Series => Some(StreamKind::Series),
        _ => None,
    }
}

/// Record whether the active account offers a content kind (>=1 category).
fn mark_available(available: &mut Signal<HashSet<StreamKind>>, kind: StreamKind, present: bool) {
    if present {
        available.write().insert(kind);
    } else {
        available.write().remove(&kind);
    }
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
    let mut show_logs = use_signal(|| false);
    let mut logs = use_signal(Vec::<LogLine>::new);
    let toasts = use_toast();
    let mut tab = use_signal(|| Tab::Live);
    let mut epg = use_signal(HashMap::<String, NowNext>::new);
    let mut programmes = use_signal(HashMap::<String, Vec<Programme>>::new);
    let mut guide_from = use_signal(|| 0i64);
    let mut guide_to = use_signal(|| 0i64);
    let mut fullscreen = use_signal(|| false);
    // Series drill-down: the opened series and its (lazily fetched) seasons/episodes.
    let mut series_detail = use_signal(|| None::<Stream>);
    let mut series_info = use_signal(|| None::<SeriesInfo>);
    // Library search: the query and its matches (across the whole cached library).
    let mut search_query = use_signal(String::new);
    let mut search_results = use_signal(Vec::<Stream>::new);
    // `(source|kind)` keys we've already background-prefetched this session, so the
    // sweep that warms the cache runs at most once per account + content kind.
    let mut prefetched = use_signal(HashSet::<String>::new);
    // Content kinds the active account actually offers (>=1 category). Drives which
    // content tabs are shown, so a live-only provider doesn't get dead Movies/Series tabs.
    let mut available = use_signal(HashSet::<StreamKind>::new);

    // Make a source active: reset to the Live tab (the per-kind effect below loads its
    // categories), and load its favorites + history + EPG.
    let activate = use_callback(move |new_creds: XtreamCredentials| {
        creds.set(Some(new_creds.clone()));
        selected.set(None);
        streams.set(Vec::new());
        categories.set(Vec::new());
        favorites.set(Vec::new());
        history.set(Vec::new());
        epg.set(HashMap::new());
        tab.set(Tab::Live);
        error.set(None);

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
    });

    // Load the categories for a content kind: paint the cache instantly, then refresh
    // from the network (dropping the response if the user switched kinds meanwhile).
    let load_kind = use_callback(move |(current, kind): (XtreamCredentials, StreamKind)| {
        selected.set(None);
        streams.set(Vec::new());
        categories.set(Vec::new());
        error.set(None);

        let cached_creds = current.clone();
        spawn(async move {
            if let Ok(cached) = bindings::cached_categories(&cached_creds, kind).await
                && tab_kind(tab()) == Some(kind)
                && !cached.is_empty()
                && categories.read().is_empty()
            {
                categories.set(cached);
            }
        });

        loading.set(true);
        spawn(async move {
            let result = bindings::list_categories(&current, kind).await;
            if tab_kind(tab()) != Some(kind) {
                return; // the user switched tabs while we were fetching
            }
            match result {
                Ok(list) => {
                    categories.set(list.clone());
                    // Warm the cache for the whole library: fetch every category's
                    // streams in the background (once per account + kind) so search is
                    // comprehensive and switching categories is instant.
                    let key = format!("{}|{}|{:?}", current.base_url, current.username, kind);
                    if !prefetched.read().contains(&key) {
                        prefetched.write().insert(key);
                        let pf_creds = current.clone();
                        spawn(async move {
                            for c in list {
                                let _ =
                                    bindings::list_streams(&pf_creds, kind, Some(&c.id.0)).await;
                            }
                        });
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    // Whenever the active account or content tab changes, (re)load that kind's
    // categories. Library tabs (Favorites/History) carry no kind and are skipped.
    use_effect(move || {
        let t = tab();
        if let (Some(kind), Some(current)) = (tab_kind(t), creds.read().clone()) {
            load_kind.call((current, kind));
        }
    });

    // Detect which content kinds the active account offers (>=1 category) so the tab bar
    // can hide kinds the provider doesn't have. Cheap: one category list per kind, read
    // from cache first (instant) then refreshed from the network.
    use_effect(move || {
        let Some(current) = creds.read().clone() else {
            available.set(HashSet::new());
            return;
        };
        available.set(HashSet::new());
        for kind in [StreamKind::Live, StreamKind::Vod, StreamKind::Series] {
            let current = current.clone();
            spawn(async move {
                if let Ok(cached) = bindings::cached_categories(&current, kind).await
                    && creds.read().as_ref() == Some(&current)
                {
                    mark_available(&mut available, kind, !cached.is_empty());
                }
                if let Ok(list) = bindings::list_categories(&current, kind).await
                    && creds.read().as_ref() == Some(&current)
                {
                    mark_available(&mut available, kind, !list.is_empty());
                }
            });
        }
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
            // Apply the persisted capture level to the backend on launch.
            let _ = bindings::set_log_level(s.log_level).await;
        }
    });

    // While the Logs panel is open, refresh the captured lines.
    use_future(move || async move {
        loop {
            TimeoutFuture::new(1000).await;
            if show_logs()
                && let Ok(lines) = bindings::get_logs().await
            {
                logs.set(lines);
            }
        }
    });

    // Fetch the windowed guide programmes (for the timeline) for a 6h window from the
    // current half-hour. Stamps the window it fetched so the timeline positions match.
    let load_programmes = use_callback(move |current: XtreamCredentials| {
        let now = crate::format::now_unix();
        let from = now - now.rem_euclid(1800);
        let to = from + 6 * 3600;
        guide_from.set(from);
        guide_to.set(to);
        spawn(async move {
            if let Ok(map) = bindings::epg_programmes(&current, from, to).await {
                programmes.set(map);
            }
        });
    });

    // Load timeline programmes whenever the Guide view is active (and EPG is on).
    use_effect(move || {
        let s = settings();
        if s.epg_enabled
            && s.channel_view == ChannelView::Guide
            && let Some(current) = creds.read().clone()
        {
            load_programmes.call(current);
        }
    });

    // Keep now/next (and the guide, in Guide view) current as time passes. The guide
    // is cached server-side after the first fetch, so this just recomputes against
    // the clock — no re-download.
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
                if settings.read().channel_view == ChannelView::Guide {
                    load_programmes.call(current);
                }
            }
        }
    });

    // Briefly show a toast. The dx ToastProvider handles timing and stacking.
    let show_toast = use_callback(move |msg: String| {
        toasts.info(msg, ToastOptions::new());
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
        let Some(kind) = tab_kind(tab()) else {
            return;
        };
        selected.set(Some(id.clone()));
        streams.set(Vec::new());
        error.set(None);

        let cached_creds = current.clone();
        let cached_id = id.clone();
        spawn(async move {
            if let Ok(cached) = bindings::cached_streams(&cached_creds, kind, &cached_id.0).await {
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
            let result = bindings::list_streams(&current, kind, Some(&id.0)).await;
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

    // Actually start playback of a concrete playable (a Live/VOD stream or a series
    // episode synthesized as a Series stream).
    let play_now = use_callback(move |stream: Stream| {
        let Some(current) = creds.read().clone() else {
            return;
        };
        let play_stream = stream.clone();
        playing.set(Some(stream.clone()));
        paused.set(false);
        error.set(None);

        let play_creds = current.clone();
        let s = *settings.read();
        spawn(async move {
            if let Err(e) = bindings::play_stream(&play_creds, &play_stream).await {
                error.set(Some(e));
                return;
            }
            // Apply the persisted volume/mute so a fresh session honours them.
            let _ = bindings::set_volume(s.volume).await;
            let _ = bindings::set_mute(s.muted).await;
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

    // Open a series' drill-down (fetch its seasons/episodes lazily).
    let open_series = use_callback(move |series: Stream| {
        let Some(current) = creds.read().clone() else {
            return;
        };
        let series_id = series.provider_id.clone();
        series_detail.set(Some(series));
        series_info.set(None);
        spawn(async move {
            if let Ok(info) = bindings::get_series_info(&current, &series_id).await {
                series_info.set(Some(info));
            }
        });
    });

    // A card click: a series opens its drill-down; anything else plays.
    let on_play = use_callback(move |stream: Stream| {
        if stream.kind == StreamKind::Series {
            open_series.call(stream);
        } else {
            play_now.call(stream);
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

    // While playing, render a transparent overlay so the embedded mpv surface
    // shows through; otherwise render the opaque browse UI.
    if let Some(stream) = playing() {
        let s = settings();
        // Show the playing channel's now/next when EPG is on and we have a match
        // (by epg_channel_id or, failing that, by normalized name).
        let now_next = if s.epg_enabled {
            crate::epg::resolve(&epg.read(), &stream).cloned()
        } else {
            None
        };
        return rsx! {
            PlayerOverlay {
                stream,
                paused: paused(),
                now_next,
                volume: s.volume,
                muted: s.muted,
                fullscreen: fullscreen(),
                on_toggle_pause: move |_| {
                    let now_paused = !paused();
                    paused.set(now_paused);
                    spawn(async move {
                        let _ = if now_paused {
                            bindings::pause().await
                        } else {
                            bindings::resume().await
                        };
                    });
                },
                on_stop: move |_| {
                    playing.set(None);
                    spawn(async move {
                        let _ = bindings::stop().await;
                    });
                },
                on_set_volume: move |v: u8| {
                    let mut s = settings();
                    s.volume = v;
                    save_settings.call(s);
                    spawn(async move {
                        let _ = bindings::set_volume(v).await;
                    });
                },
                on_toggle_mute: move |_| {
                    let mut s = settings();
                    s.muted = !s.muted;
                    let muted = s.muted;
                    save_settings.call(s);
                    spawn(async move {
                        let _ = bindings::set_mute(muted).await;
                    });
                },
                on_toggle_fullscreen: move |_| {
                    spawn(async move {
                        if let Ok(fs) = bindings::toggle_fullscreen().await {
                            fullscreen.set(fs);
                        }
                    });
                },
            }
        };
    }

    let connected = creds.read().is_some();
    let current_settings = settings();
    let favorites_enabled = current_settings.favorites_enabled;
    let history_enabled = current_settings.history_enabled;
    let channel_view = current_settings.channel_view;
    let favorite_ids: Vec<StreamId> = favorites().iter().map(|s| s.id.clone()).collect();

    // Which content kinds this account offers (hides Movies/Series for live-only providers).
    let has_movies = available.read().contains(&StreamKind::Vod);
    let has_series = available.read().contains(&StreamKind::Series);

    // A disabled feature's or unavailable kind's tab falls back to Live.
    let current_tab = match tab() {
        Tab::Movies if !has_movies => Tab::Live,
        Tab::Series if !has_series => Tab::Live,
        Tab::Favorites if !favorites_enabled => Tab::Live,
        Tab::History if !history_enabled => Tab::Live,
        t => t,
    };

    rsx! {
        div {
            class: "h-screen overflow-hidden flex flex-col bg-white text-neutral-900 \
                dark:bg-neutral-950 dark:text-neutral-100",
            TitleBar {
                incognito: incognito(),
                search: search_query(),
                on_search: move |q: String| {
                    search_query.set(q.clone());
                    let trimmed = q.trim().to_string();
                    if trimmed.is_empty() {
                        search_results.set(Vec::new());
                        return;
                    }
                    if let Some(current) = creds.read().clone() {
                        spawn(async move {
                            if let Ok(list) = bindings::search_streams(&current, &trimmed).await
                                && search_query().trim() == trimmed
                            {
                                search_results.set(list);
                            }
                        });
                    }
                },
                on_logs: move |_| {
                    let open = !show_logs();
                    show_logs.set(open);
                    if open {
                        spawn(async move {
                            if let Ok(lines) = bindings::get_logs().await {
                                logs.set(lines);
                            }
                        });
                    }
                },
                on_options: move |_| show_settings.set(!show_settings()),
                on_sources: move |_| show_sources.set(!show_sources()),
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
                    show_movies: has_movies,
                    show_series: has_series,
                    show_favorites: favorites_enabled,
                    show_history: history_enabled,
                    on_select: move |t| tab.set(t),
                }
                match current_tab {
                    Tab::Live | Tab::Movies | Tab::Series => {
                        // EPG and the timeline Guide view are Live-only; Movies/Series
                        // use Grid/List with no guide data.
                        let is_live = current_tab == Tab::Live;
                        let view = match (is_live, channel_view) {
                            (false, ChannelView::Guide) => ChannelView::Grid,
                            (_, v) => v,
                        };
                        let pane_epg = if is_live { epg() } else { HashMap::new() };
                        let pane_prog = if is_live { programmes() } else { HashMap::new() };
                        rsx! {
                            div {
                                class: "flex flex-col md:flex-row flex-1 min-h-0",
                                CategoryList {
                                    categories: categories(),
                                    selected: selected(),
                                    on_select,
                                }
                                main {
                                    class: "flex-1 min-h-0 overflow-y-auto",
                                    if selected().is_none() {
                                        div {
                                            class: "flex h-full flex-col items-center justify-center \
                                                gap-3 text-neutral-400",
                                            crate::components::icons::Category { class: "h-12 w-12".to_string() }
                                            p { class: "text-sm", "Pick a category to start browsing." }
                                        }
                                    } else if loading() && streams().is_empty() {
                                        Spinner {}
                                    } else {
                                        ChannelPane {
                                            view,
                                            streams: streams(),
                                            favorites_enabled,
                                            favorite_ids: favorite_ids.clone(),
                                            epg: pane_epg,
                                            programmes: pane_prog,
                                            guide_from: guide_from(),
                                            guide_to: guide_to(),
                                            now: crate::format::now_unix(),
                                            on_play: move |s| on_play.call(s),
                                            on_toggle_favorite: move |s| toggle_favorite.call(s),
                                        }
                                    }
                                }
                            }
                        }
                    }
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
                                    programmes: programmes(),
                                    guide_from: guide_from(),
                                    guide_to: guide_to(),
                                    now: crate::format::now_unix(),
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
                                    programmes: programmes(),
                                    guide_from: guide_from(),
                                    guide_to: guide_to(),
                                    now: crate::format::now_unix(),
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

        if show_logs() {
            LogsPanel {
                logs: logs(),
                level: current_settings.log_level,
                on_set_level: move |l: LogLevel| {
                    let mut s = settings();
                    s.log_level = l;
                    save_settings.call(s);
                    spawn(async move {
                        let _ = bindings::set_log_level(l).await;
                    });
                },
                on_copy: move |_| {
                    let text = logs()
                        .iter()
                        .map(|l| {
                            let base =
                                format!("{} {} {}: {}", l.time, l.level.to_uppercase(), l.target, l.message);
                            if l.fields.is_empty() {
                                base
                            } else {
                                format!("{base} {}", l.fields)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    spawn(async move {
                        crate::clipboard::copy(text).await;
                        show_toast.call("Copied to clipboard".to_string());
                    });
                },
                on_clear: move |_| {
                    logs.set(Vec::new());
                    spawn(async move {
                        let _ = bindings::clear_logs().await;
                    });
                },
                on_close: move |_| show_logs.set(false),
            }
        }

        if let Some(series) = series_detail() {
            SeriesDetail {
                series: series.clone(),
                info: series_info(),
                on_play_episode: move |ep: Episode| {
                    let stream = Stream {
                        id: StreamId(format!("ep:{}", ep.id)),
                        provider_id: ep.id.clone(),
                        name: format!("{} · S{}E{}", series.name, ep.season, ep.episode),
                        logo: series.logo.clone(),
                        category_id: None,
                        kind: StreamKind::Series,
                        epg_channel_id: None,
                        container_extension: ep.container_extension.clone(),
                    };
                    play_now.call(stream);
                },
                on_close: move |_| series_detail.set(None),
            }
        }

        if !search_query().is_empty() {
            SearchResults {
                results: search_results(),
                on_select: move |s: Stream| {
                    search_query.set(String::new());
                    search_results.set(Vec::new());
                    on_play.call(s);
                },
                on_close: move |_| {
                    search_query.set(String::new());
                    search_results.set(Vec::new());
                },
            }
        }
    }
}
