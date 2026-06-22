//! Renders a set of channels in the user's chosen view (grid, list, or timeline
//! guide). Keeps the switch in one place so the views (Channels/Favorites/History)
//! don't each duplicate it.

use std::collections::HashMap;

use cathode_core::model::{ChannelView, NowNext, Programme, Stream, StreamId};
use dioxus::prelude::*;

use crate::components::{ChannelList, EpgGuide, StreamGrid};

#[component]
pub fn ChannelPane(
    view: ChannelView,
    streams: Vec<Stream>,
    favorites_enabled: bool,
    favorite_ids: Vec<StreamId>,
    epg: HashMap<String, NowNext>,
    // Guide-only inputs; ignored by Grid/List.
    programmes: HashMap<String, Vec<Programme>>,
    guide_from: i64,
    guide_to: i64,
    now: i64,
    on_play: EventHandler<Stream>,
    on_toggle_favorite: EventHandler<Stream>,
    // Guide-only: open a programme's detail popover (programme, channel, click x/y).
    on_programme: EventHandler<(Programme, Stream, f64, f64)>,
) -> Element {
    match view {
        ChannelView::Grid => rsx! {
            StreamGrid {
                streams,
                favorites_enabled,
                favorite_ids,
                epg,
                on_play,
                on_toggle_favorite,
            }
        },
        ChannelView::List => rsx! {
            ChannelList {
                streams,
                favorites_enabled,
                favorite_ids,
                epg,
                on_play,
                on_toggle_favorite,
            }
        },
        ChannelView::Guide => rsx! {
            EpgGuide {
                streams,
                programmes,
                from: guide_from,
                to: guide_to,
                now,
                on_play,
                on_programme,
            }
        },
    }
}
