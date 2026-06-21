//! The EPG timeline guide: channels down the side, programmes laid out left-to-right
//! on a time axis. The whole thing scrolls in both axes; the time header sticks to
//! the top and the channel-label column sticks to the left. Click a channel (label
//! or any of its programme blocks) to play it.

use std::collections::HashMap;

use cathode_core::model::{Programme, Stream};
use dioxus::prelude::*;

use crate::epg::resolve;
use crate::format::hhmm;

/// Horizontal scale and layout constants.
const PX_PER_MIN: f64 = 4.0;
const TICK_MINUTES: i64 = 30;
const ROW_H: i64 = 56;

/// Pixel x-offset of a timestamp from the window start.
fn px(t: i64, from: i64) -> f64 {
    (t - from) as f64 / 60.0 * PX_PER_MIN
}

#[component]
pub fn EpgGuide(
    streams: Vec<Stream>,
    programmes: HashMap<String, Vec<Programme>>,
    from: i64,
    to: i64,
    now: i64,
    on_play: EventHandler<Stream>,
) -> Element {
    let timeline_w = px(to, from);
    let tick_count = ((to - from) / (TICK_MINUTES * 60)).max(0);
    let now_x = px(now, from);
    let now_visible = now >= from && now < to;

    rsx! {
        div {
            class: "h-full overflow-auto bg-white text-neutral-900 \
                dark:bg-neutral-950 dark:text-neutral-100",
            // Header: corner + time ticks, sticky to the top.
            div {
                class: "sticky top-0 z-20 flex border-b border-neutral-200 bg-white \
                    dark:border-neutral-800 dark:bg-neutral-950",
                div {
                    class: "sticky left-0 z-30 w-36 shrink-0 border-r border-neutral-200 \
                        bg-white px-3 py-1 text-xs font-medium text-neutral-500 \
                        dark:border-neutral-800 dark:bg-neutral-950",
                    "Channel"
                }
                div {
                    class: "relative shrink-0 h-7",
                    style: "width: {timeline_w}px",
                    {(0..=tick_count).map(|i| {
                        let t = from + i * TICK_MINUTES * 60;
                        let left = px(t, from);
                        rsx! {
                            span {
                                key: "{i}",
                                class: "absolute top-1 text-[11px] text-neutral-400",
                                style: "left: {left}px",
                                "{hhmm(t)}"
                            }
                        }
                    })}
                }
            }

            // One row per channel.
            {streams.iter().map(|stream| {
                let label_stream = stream.clone();
                let progs = resolve(&programmes, stream);
                rsx! {
                    div {
                        key: "{stream.id.0}",
                        class: "flex border-b border-neutral-100 dark:border-neutral-900",
                        button {
                            class: "sticky left-0 z-10 w-36 shrink-0 truncate border-r \
                                border-neutral-200 bg-white px-3 text-left text-sm \
                                hover:bg-neutral-100 focus:outline-none focus:ring-2 \
                                focus:ring-sky-500 dark:border-neutral-800 dark:bg-neutral-950 \
                                dark:hover:bg-neutral-800",
                            style: "height: {ROW_H}px",
                            onclick: move |_| on_play.call(label_stream.clone()),
                            "{stream.name}"
                        }
                        div {
                            class: "relative shrink-0",
                            style: "width: {timeline_w}px; height: {ROW_H}px",
                            if now_visible {
                                div {
                                    class: "absolute top-0 bottom-0 z-10 w-0.5 bg-sky-500",
                                    style: "left: {now_x}px",
                                }
                            }
                            if let Some(progs) = progs {
                                for programme in progs.iter() {
                                    {
                                        let left = px(programme.start, from).max(0.0);
                                        let right = px(programme.stop, from).min(timeline_w);
                                        let width = (right - left).max(2.0);
                                        let block_stream = stream.clone();
                                        rsx! {
                                            button {
                                                key: "{programme.start}",
                                                class: "absolute top-0.5 bottom-0.5 overflow-hidden \
                                                    rounded border border-neutral-200 bg-neutral-100 \
                                                    px-1.5 text-left hover:bg-neutral-200 \
                                                    focus:outline-none focus:ring-2 focus:ring-sky-500 \
                                                    dark:border-neutral-700 dark:bg-neutral-800 \
                                                    dark:hover:bg-neutral-700",
                                                style: "left: {left}px; width: {width}px",
                                                onclick: move |_| on_play.call(block_stream.clone()),
                                                span {
                                                    class: "block truncate text-xs font-medium",
                                                    "{programme.title}"
                                                }
                                                span {
                                                    class: "block truncate text-[10px] text-neutral-500 \
                                                        dark:text-neutral-400",
                                                    "{hhmm(programme.start)}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            })}
        }
    }
}
