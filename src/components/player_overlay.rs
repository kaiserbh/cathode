//! The player overlay: a transparent full-screen layer so the embedded mpv surface
//! shows through, with a control bar docked at the bottom that auto-hides (with the
//! cursor) after a short idle. Icon controls, a volume slider, mute, and fullscreen;
//! keyboard shortcuts and click-to-pause on the video.

use cathode_core::model::{NowNext, Stream};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use js_sys::Date;

use crate::components::icons::{
    FullscreenEnter, FullscreenExit, Pause, Play, Stop, VolumeHigh, VolumeMuted,
};
use crate::format::hhmm;

/// How long the pointer must be still before the controls hide, in milliseconds.
const IDLE_MS: f64 = 2500.0;
const ICON: &str = "h-5 w-5";
/// The large centered icon for the shortcut feedback HUD.
const HUD_ICON: &str = "h-10 w-10";
/// How long the shortcut HUD stays on screen, in milliseconds.
const HUD_MS: u32 = 800;

/// A transient, centered overlay shown when a playback shortcut fires.
#[derive(Clone, Copy, PartialEq)]
enum Hud {
    Volume(u8),
    Muted,
    Playing,
    Paused,
}

#[component]
pub fn PlayerOverlay(
    stream: Stream,
    paused: bool,
    now_next: Option<NowNext>,
    volume: u8,
    muted: bool,
    fullscreen: bool,
    on_toggle_pause: EventHandler<()>,
    on_stop: EventHandler<()>,
    on_set_volume: EventHandler<u8>,
    on_toggle_mute: EventHandler<()>,
    on_toggle_fullscreen: EventHandler<()>,
) -> Element {
    let mut visible = use_signal(|| true);
    let mut last_active = use_signal(Date::now);
    // The root element, kept so we can pull focus back to it after the user touches a
    // control. The shortcuts live on the root's `onkeydown`, so a focused slider/button
    // would otherwise swallow the same keys until the video is clicked.
    let mut root = use_signal(|| None::<std::rc::Rc<MountedData>>);
    let refocus = use_callback(move |()| {
        if let Some(node) = root() {
            spawn(async move {
                let _ = node.set_focus(true).await;
            });
        }
    });

    // The transient shortcut HUD. A sequence id ensures a newer HUD isn't cut short by
    // an older one's timer.
    let mut hud = use_signal(|| None::<Hud>);
    let mut hud_seq = use_signal(|| 0u32);
    let show_hud = use_callback(move |h: Hud| {
        let id = hud_seq() + 1;
        hud_seq.set(id);
        hud.set(Some(h));
        spawn(async move {
            TimeoutFuture::new(HUD_MS).await;
            if hud_seq() == id {
                hud.set(None);
            }
        });
    });

    use_future(move || async move {
        loop {
            TimeoutFuture::new(400).await;
            if visible() && Date::now() - last_active() > IDLE_MS {
                visible.set(false);
                // Don't strand focus on a control that just became unclickable.
                refocus.call(());
            }
        }
    });

    let bar_visibility = if visible() {
        "opacity-100"
    } else {
        "opacity-0 pointer-events-none"
    };
    let cursor = if visible() { "" } else { "cursor-none" };
    let silent = muted || volume == 0;

    rsx! {
        div {
            class: "relative flex h-screen flex-col justify-end outline-none {cursor}",
            tabindex: "0",
            autofocus: true,
            onmounted: move |e| root.set(Some(e.data())),
            onmousemove: move |_| {
                last_active.set(Date::now());
                if !visible() {
                    visible.set(true);
                }
            },
            // Click the video (anywhere not on the bar) toggles play/pause.
            onclick: move |_| {
                on_toggle_pause.call(());
                show_hud.call(if paused { Hud::Playing } else { Hud::Paused });
            },
            onkeydown: move |e| {
                match e.key() {
                    Key::Character(c) if c == " " => {
                        e.prevent_default();
                        on_toggle_pause.call(());
                        show_hud.call(if paused { Hud::Playing } else { Hud::Paused });
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("m") => {
                        let now_muted = !muted;
                        on_toggle_mute.call(());
                        show_hud.call(if now_muted { Hud::Muted } else { Hud::Volume(volume) });
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("f") => {
                        on_toggle_fullscreen.call(())
                    }
                    Key::ArrowUp => {
                        e.prevent_default();
                        let v = (volume + 5).min(100);
                        on_set_volume.call(v);
                        show_hud.call(Hud::Volume(v));
                    }
                    Key::ArrowDown => {
                        e.prevent_default();
                        let v = volume.saturating_sub(5);
                        on_set_volume.call(v);
                        show_hud.call(Hud::Volume(v));
                    }
                    Key::Escape => on_stop.call(()),
                    _ => {}
                }
            },
            // Transient shortcut feedback (volume / mute / play-pause), centered.
            if let Some(h) = hud() {
                div {
                    class: "pointer-events-none absolute inset-0 flex items-center justify-center",
                    div {
                        class: "flex w-28 flex-col items-center gap-1 rounded-2xl bg-black/40 \
                            py-6 text-white backdrop-blur-sm",
                        {match h {
                            Hud::Volume(_) => rsx! { VolumeHigh { class: HUD_ICON } },
                            Hud::Muted => rsx! { VolumeMuted { class: HUD_ICON } },
                            Hud::Playing => rsx! { Play { class: HUD_ICON } },
                            Hud::Paused => rsx! { Pause { class: HUD_ICON } },
                        }}
                        if let Hud::Volume(v) = h {
                            span {
                                class: "w-full text-center text-lg font-semibold tabular-nums",
                                "{v}%"
                            }
                        }
                    }
                }
            }
            div {
                class: "flex items-center gap-3 bg-black/60 p-3 text-white backdrop-blur-sm \
                    transition-opacity duration-300 {bar_visibility}",
                onclick: move |e| e.stop_propagation(),
                // After a click or a finished slider drag, hand focus back to the root
                // so the keyboard shortcuts keep working without clicking the video.
                onpointerup: move |_| refocus.call(()),
                div { class: "min-w-0 flex-1",
                    span { class: "block truncate text-sm font-medium", "{stream.name}" }
                    if let Some(nn) = now_next.as_ref() {
                        if let Some(now) = nn.now.as_ref() {
                            span { class: "block truncate text-xs text-white/70",
                                "Now: {now.title} · {hhmm(now.start)}–{hhmm(now.stop)}"
                            }
                        }
                        if let Some(next) = nn.next.as_ref() {
                            span { class: "block truncate text-[11px] text-white/50",
                                "Next: {next.title} · {hhmm(next.start)}"
                            }
                        }
                    }
                }
                div { class: "flex items-center gap-1",
                    button {
                        class: BTN,
                        title: if paused { "Play" } else { "Pause" },
                        onclick: move |_| on_toggle_pause.call(()),
                        if paused {
                            Play { class: ICON }
                        } else {
                            Pause { class: ICON }
                        }
                    }
                    button {
                        class: BTN,
                        title: "Stop",
                        onclick: move |_| on_stop.call(()),
                        Stop { class: ICON }
                    }
                    button {
                        class: BTN,
                        title: if silent { "Unmute" } else { "Mute" },
                        onclick: move |_| on_toggle_mute.call(()),
                        if silent {
                            VolumeMuted { class: ICON }
                        } else {
                            VolumeHigh { class: ICON }
                        }
                    }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "100",
                        value: "{volume}",
                        class: "h-1 w-24 cursor-pointer accent-sky-500",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<u8>() {
                                on_set_volume.call(v);
                            }
                        },
                    }
                    button {
                        class: BTN,
                        title: if fullscreen { "Exit fullscreen" } else { "Fullscreen" },
                        onclick: move |_| on_toggle_fullscreen.call(()),
                        if fullscreen {
                            FullscreenExit { class: ICON }
                        } else {
                            FullscreenEnter { class: ICON }
                        }
                    }
                }
            }
        }
    }
}

const BTN: &str = "rounded-full p-2 text-white hover:bg-white/10 focus:outline-none \
    focus:ring-2 focus:ring-sky-400";
