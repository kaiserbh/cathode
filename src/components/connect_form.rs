//! Add-source form: enter Xtream credentials, or point at an M3U/M3U8 playlist by
//! URL or local file.

use cathode_core::sources::SourceCredentials;
use cathode_core::sources::m3u::M3uCredentials;
use cathode_core::sources::xtream::XtreamCredentials;
use dioxus::prelude::*;

use crate::bindings;
use crate::ui::button::{Button, ButtonVariant};

const INPUT: &str = "w-full rounded-md border border-neutral-300 dark:border-neutral-700 \
    bg-white dark:bg-neutral-900 px-3 py-2 text-sm outline-none \
    focus:ring-2 focus:ring-sky-500";

/// Which kind of source the form is currently adding.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Xtream,
    M3u,
}

/// A fallback label for an M3U source when the user leaves Name blank: the
/// playlist's file name (or the raw location if there isn't one).
fn default_name(location: &str) -> String {
    let path = location.split(['?', '#']).next().unwrap_or(location);
    path.rsplit('/')
        .find(|s| !s.is_empty())
        .unwrap_or(location)
        .to_string()
}

/// Split the EPG field (one URL per line, or comma-separated) into trimmed,
/// de-duplicated XMLTV URLs.
fn parse_epg_urls(raw: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.split(['\n', ','])
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .filter(|u| seen.insert(u.to_string()))
        .map(str::to_string)
        .collect()
}

#[component]
pub fn ConnectForm(connecting: bool, on_connect: EventHandler<SourceCredentials>) -> Element {
    let mut mode = use_signal(|| Mode::Xtream);
    // Xtream fields.
    let mut base_url = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    // M3U fields.
    let mut name = use_signal(String::new);
    let mut url = use_signal(String::new);
    let mut epg = use_signal(String::new);
    let mut detecting = use_signal(|| false);

    let submit = move |event: FormEvent| {
        event.prevent_default();
        match mode() {
            Mode::Xtream => {
                let creds = XtreamCredentials {
                    base_url: base_url.read().trim().to_string(),
                    username: username.read().trim().to_string(),
                    password: password.read().clone(),
                };
                if creds.base_url.is_empty() || creds.username.is_empty() {
                    return;
                }
                on_connect.call(SourceCredentials::Xtream(creds));
            }
            Mode::M3u => {
                let location = url.read().trim().to_string();
                if location.is_empty() {
                    return;
                }
                let label = {
                    let n = name.read().trim().to_string();
                    if n.is_empty() {
                        default_name(&location)
                    } else {
                        n
                    }
                };
                on_connect.call(SourceCredentials::M3u(M3uCredentials {
                    name: label,
                    url: location,
                    epg_urls: parse_epg_urls(&epg.read()),
                }));
            }
        }
    };

    // Open the native file picker (via the shell) and fill in the location.
    let pick_file = move |_| {
        spawn(async move {
            if let Ok(Some(path)) = bindings::pick_playlist_file().await {
                url.set(path);
            }
        });
    };

    // Read the playlist's `#EXTM3U` header and pre-fill the EPG field with whatever
    // guide URLs it declares, for the user to trim down.
    let detect_epg = move |_| {
        let location = url.read().trim().to_string();
        if location.is_empty() || detecting() {
            return;
        }
        detecting.set(true);
        spawn(async move {
            if let Ok(urls) = bindings::detect_playlist_epg(&location).await
                && !urls.is_empty()
            {
                epg.set(urls.join("\n"));
            }
            detecting.set(false);
        });
    };

    let tab = |active: bool| {
        if active {
            "px-3 py-1.5 text-sm rounded-md bg-sky-600 text-white"
        } else {
            "px-3 py-1.5 text-sm rounded-md text-neutral-600 hover:bg-neutral-100 \
                dark:text-neutral-300 dark:hover:bg-neutral-800"
        }
    };

    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "flex gap-1 self-start rounded-md bg-neutral-100 p-1 dark:bg-neutral-900",
                button {
                    r#type: "button",
                    class: tab(mode() == Mode::Xtream),
                    onclick: move |_| mode.set(Mode::Xtream),
                    "Xtream"
                }
                button {
                    r#type: "button",
                    class: tab(mode() == Mode::M3u),
                    onclick: move |_| mode.set(Mode::M3u),
                    "M3U Playlist"
                }
            }
            form {
                class: "flex flex-col gap-2",
                onsubmit: submit,
                {match mode() {
                    Mode::Xtream => rsx! {
                        input {
                            class: INPUT,
                            r#type: "text",
                            placeholder: "http://host:port",
                            value: "{base_url}",
                            oninput: move |e| base_url.set(e.value()),
                        }
                        input {
                            class: INPUT,
                            r#type: "text",
                            placeholder: "Username",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                        }
                        input {
                            class: INPUT,
                            r#type: "password",
                            placeholder: "Password",
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    },
                    Mode::M3u => rsx! {
                        input {
                            class: INPUT,
                            r#type: "text",
                            placeholder: "Name (optional)",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                        div {
                            class: "flex gap-2",
                            input {
                                class: INPUT,
                                r#type: "text",
                                placeholder: "Playlist URL or file path",
                                value: "{url}",
                                oninput: move |e| url.set(e.value()),
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                r#type: "button",
                                class: "shrink-0",
                                onclick: pick_file,
                                "Browse…"
                            }
                        }
                        div {
                            class: "flex gap-2",
                            textarea {
                                class: "{INPUT} font-mono text-xs",
                                rows: 3,
                                placeholder: "EPG XMLTV URL(s), one per line (optional)",
                                value: "{epg}",
                                oninput: move |e| epg.set(e.value()),
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                r#type: "button",
                                class: "shrink-0 self-start",
                                disabled: detecting(),
                                onclick: detect_epg,
                                if detecting() { "Detecting…" } else { "Detect" }
                            }
                        }
                        p {
                            class: "text-xs text-neutral-500",
                            "Detect reads the playlist's EPG header. Keep only the guides you need."
                        }
                    },
                }}
                Button {
                    variant: ButtonVariant::Primary,
                    r#type: "submit",
                    disabled: connecting,
                    class: "self-start",
                    if connecting { "Connecting…" } else { "Add source" }
                }
            }
        }
    }
}
