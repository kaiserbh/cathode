//! Renders a set of channels in the user's chosen view (grid or list). Keeps the
//! grid/list switch in one place so the views (Channels/Favorites/History) don't
//! each duplicate it.

use std::collections::HashMap;

use cathode_core::model::{ChannelView, NowNext, Stream, StreamId};
use dioxus::prelude::*;

use crate::components::{ChannelList, StreamGrid};

#[component]
pub fn ChannelPane(
    view: ChannelView,
    streams: Vec<Stream>,
    favorites_enabled: bool,
    favorite_ids: Vec<StreamId>,
    epg: HashMap<String, NowNext>,
    on_play: EventHandler<Stream>,
    on_toggle_favorite: EventHandler<Stream>,
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
    }
}
