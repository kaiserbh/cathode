//! The EPG timeline guide: channels down the side, programmes laid out left-to-right
//! on a time axis. Rows are virtualized with the dioxus-primitives `VirtualList` so a
//! large category stays light. The time header is a separate strip whose horizontal
//! scroll is bridged to the virtualized body, so the two stay aligned; clicking a
//! programme opens its detail popover, clicking a channel label plays it.
//!
//! Note: `VirtualList` positions rows inside a `translateY` canvas, and a CSS transform
//! breaks `position: sticky` for descendants, so the channel label is the first cell of
//! each row rather than pinned to the left during horizontal scroll.

use std::collections::HashMap;

use cathode_core::model::{Programme, Stream};
use dioxus::prelude::*;

use crate::epg::resolve;
use crate::format::hhmm;
use crate::ui::virtual_list::VirtualList;

/// Horizontal scale and layout constants.
const PX_PER_MIN: f64 = 4.0;
const TICK_MINUTES: i64 = 30;
const ROW_H: i64 = 56;

/// Bridges the header strip's horizontal scroll to the virtualized body so they stay
/// aligned (retries briefly until both elements exist; cleans up on unmount).
const HEADER_SYNC_JS: &str = r#"
    let body = null, header = null;
    for (let i = 0; i < 60 && (!body || !header); i++) {
        body = document.querySelector('.dx-virtual-list-container');
        header = document.getElementById('cathode-epg-header');
        if (body && header) break;
        await new Promise(r => setTimeout(r, 50));
    }
    if (!body || !header) return;
    const sync = () => { header.scrollLeft = body.scrollLeft; };
    body.addEventListener('scroll', sync, { passive: true });
    await dioxus.recv();
    body.removeEventListener('scroll', sync);
"#;

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
    /// Clicking a programme cell opens its detail popover: the programme, the channel
    /// it's on, and the click's viewport coordinates to anchor the card.
    on_programme: EventHandler<(Programme, Stream, f64, f64)>,
) -> Element {
    let timeline_w = px(to, from);
    let tick_count = ((to - from) / (TICK_MINUTES * 60)).max(0);
    let now_x = px(now, from);
    let now_visible = now >= from && now < to;

    // Held for the component's life so the bridged scroll listener isn't torn down early.
    let _header_sync = use_signal(|| document::eval(HEADER_SYNC_JS));

    let row_streams = streams.clone();
    let row_programmes = programmes.clone();

    rsx! {
        div {
            class: "flex h-full flex-col bg-white text-neutral-900 \
                dark:bg-neutral-950 dark:text-neutral-100",
            // Time header: corner + ticks, scroll-synced with the body below.
            div {
                id: "cathode-epg-header",
                class: "flex shrink-0 overflow-hidden border-b border-neutral-200 \
                    dark:border-neutral-800",
                div {
                    class: "w-36 shrink-0 border-r border-neutral-200 px-3 py-1 text-xs \
                        font-medium text-neutral-500 dark:border-neutral-800",
                    "Channel"
                }
                div {
                    class: "relative h-7 shrink-0",
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

            // Virtualized channel rows. The container scrolls both axes.
            VirtualList {
                count: row_streams.len(),
                estimate_size: move |_idx: usize| ROW_H as u32,
                style: "overflow: auto; flex: 1 1 0%;",
                render_item: move |idx: usize| {
                    let stream = row_streams[idx].clone();
                    let progs = resolve(&row_programmes, &stream).cloned();
                    let label_stream = stream.clone();
                    rsx! {
                        div {
                            class: "flex border-b border-neutral-100 dark:border-neutral-900",
                            style: "height: {ROW_H}px",
                            button {
                                class: "w-36 shrink-0 truncate border-r border-neutral-200 \
                                    bg-white px-3 text-left text-sm hover:bg-neutral-100 \
                                    focus:outline-none focus:ring-2 focus:ring-sky-500 \
                                    dark:border-neutral-800 dark:bg-neutral-950 \
                                    dark:hover:bg-neutral-800",
                                onclick: move |_| on_play.call(label_stream.clone()),
                                {stream.name.clone()}
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
                                if let Some(progs) = progs.as_ref() {
                                    for programme in progs.iter() {
                                        {
                                            let left = px(programme.start, from).max(0.0);
                                            let right = px(programme.stop, from).min(timeline_w);
                                            let width = (right - left).max(2.0);
                                            let block_stream = stream.clone();
                                            let block_prog = programme.clone();
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
                                                    onclick: move |e: MouseEvent| {
                                                        let c = e.client_coordinates();
                                                        on_programme.call((
                                                            block_prog.clone(),
                                                            block_stream.clone(),
                                                            c.x,
                                                            c.y,
                                                        ));
                                                    },
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
                },
            }
        }
    }
}
