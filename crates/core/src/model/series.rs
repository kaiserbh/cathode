//! Series detail: the seasons-and-episodes breakdown for one series.
//!
//! A series entry (a [`crate::model::Stream`] with `kind = Series`) is not played
//! directly; the user drills into it to fetch this structure and pick an episode.
//! These types cross the command boundary for the drill-down UI.

use serde::{Deserialize, Serialize};

/// A series' seasons, each holding its episodes. Built from Xtream `get_series_info`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SeriesInfo {
    pub seasons: Vec<Season>,
}

/// One season of a series, ordered by `number`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Season {
    pub number: u32,
    pub episodes: Vec<Episode>,
}

/// One playable episode. `id` is the provider's episode id, used to build the
/// playable `/series/{u}/{p}/{id}.{ext}` URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub title: String,
    pub season: u32,
    pub episode: u32,
    /// Playable file extension (e.g. `mp4`/`mkv`); `None` falls back to a default.
    pub container_extension: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trips() {
        let info = SeriesInfo {
            seasons: vec![Season {
                number: 1,
                episodes: vec![Episode {
                    id: "501".to_string(),
                    title: "Pilot".to_string(),
                    season: 1,
                    episode: 1,
                    container_extension: Some("mkv".to_string()),
                }],
            }],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert_eq!(serde_json::from_str::<SeriesInfo>(&json).unwrap(), info);
    }
}
