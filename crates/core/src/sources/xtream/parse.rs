//! Map raw Xtream API responses onto the normalized model.
//!
//! These are pure functions: the caller fetches the JSON, these turn it into
//! `Category` / `Stream` / `SeriesInfo`. All provider quirks were already absorbed
//! in `api.rs`.

use crate::error::CoreError;
use crate::model::{Category, CategoryId, Episode, Season, SeriesInfo, Stream, StreamKind};

use super::api::{RawCategory, RawLiveStream, RawSeries, RawSeriesInfo, RawVodStream};

/// Parse a `get_{live,vod,series}_categories` response into normalized categories.
/// The shape is identical across content kinds.
pub fn parse_categories(json: &str) -> Result<Vec<Category>, CoreError> {
    let raw: Vec<RawCategory> =
        serde_json::from_str(json).map_err(|e| CoreError::json("categories", e))?;

    Ok(raw
        .into_iter()
        .map(|c| Category {
            id: CategoryId(c.category_id.0),
            name: c.category_name,
        })
        .collect())
}

/// Parse a `get_live_streams` response into normalized live streams.
///
/// `source_id` comes from [`super::XtreamSource::source_id`] and combines with
/// each Xtream `stream_id` to derive the stable [`crate::model::StreamId`].
pub fn parse_live_streams(json: &str, source_id: &str) -> Result<Vec<Stream>, CoreError> {
    let raw: Vec<RawLiveStream> =
        serde_json::from_str(json).map_err(|e| CoreError::json("live streams", e))?;

    Ok(raw
        .into_iter()
        .map(|s| {
            let mut stream = Stream::new(source_id, &s.stream_id.0, s.name, StreamKind::Live);
            stream.logo = s.stream_icon.filter(|l| !l.is_empty());
            stream.category_id = clean_category(s.category_id.map(|c| c.0));
            // An empty epg id means no EPG mapping for this channel.
            stream.epg_channel_id = s.epg_channel_id.filter(|e| !e.is_empty());
            stream
        })
        .collect())
}

/// Parse a `get_vod_streams` response into normalized VOD streams, carrying the
/// playable file extension.
pub fn parse_vod_streams(json: &str, source_id: &str) -> Result<Vec<Stream>, CoreError> {
    let raw: Vec<RawVodStream> =
        serde_json::from_str(json).map_err(|e| CoreError::json("vod streams", e))?;

    Ok(raw
        .into_iter()
        .map(|s| {
            let mut stream = Stream::new(source_id, &s.stream_id.0, s.name, StreamKind::Vod);
            stream.logo = s.stream_icon.filter(|l| !l.is_empty());
            stream.category_id = clean_category(s.category_id.map(|c| c.0));
            stream.container_extension = s.container_extension.filter(|e| !e.is_empty());
            stream
        })
        .collect())
}

/// Parse a `get_series` response into normalized series entries (not directly
/// playable; the UI drills into each via `get_series_info`).
pub fn parse_series(json: &str, source_id: &str) -> Result<Vec<Stream>, CoreError> {
    let raw: Vec<RawSeries> =
        serde_json::from_str(json).map_err(|e| CoreError::json("series", e))?;

    Ok(raw
        .into_iter()
        .map(|s| {
            let mut stream = Stream::new(source_id, &s.series_id.0, s.name, StreamKind::Series);
            stream.logo = s.cover.filter(|l| !l.is_empty());
            stream.category_id = clean_category(s.category_id.map(|c| c.0));
            stream
        })
        .collect())
}

/// Parse a `get_series_info` response into seasons and episodes, sorted by number.
pub fn parse_series_info(json: &str) -> Result<SeriesInfo, CoreError> {
    let raw: RawSeriesInfo =
        serde_json::from_str(json).map_err(|e| CoreError::json("series info", e))?;

    let mut seasons: Vec<Season> = raw
        .episodes
        .into_iter()
        .map(|(season_key, raw_eps)| {
            let number: u32 = season_key.parse().unwrap_or(0);
            let mut episodes: Vec<Episode> = raw_eps
                .into_iter()
                .enumerate()
                .map(|(i, e)| {
                    let episode = e
                        .episode_num
                        .and_then(|n| n.0.parse().ok())
                        .unwrap_or(i as u32 + 1);
                    Episode {
                        id: e.id.0,
                        title: e.title.unwrap_or_else(|| format!("Episode {episode}")),
                        season: number,
                        episode,
                        container_extension: e.container_extension.filter(|x| !x.is_empty()),
                    }
                })
                .collect();
            episodes.sort_by_key(|e| e.episode);
            Season { number, episodes }
        })
        .collect();
    seasons.sort_by_key(|s| s.number);
    Ok(SeriesInfo { seasons })
}

/// An empty or absent category id is no category.
fn clean_category(raw: Option<String>) -> Option<CategoryId> {
    raw.filter(|c| !c.is_empty()).map(CategoryId)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_epg_channel_id_and_treats_empty_as_none() {
        let json = r#"[
            {"stream_id": 1, "name": "A", "epg_channel_id": "bbc1.uk"},
            {"stream_id": 2, "name": "B", "epg_channel_id": ""},
            {"stream_id": 3, "name": "C"}
        ]"#;
        let streams = parse_live_streams(json, "src-1").unwrap();
        assert_eq!(streams[0].epg_channel_id.as_deref(), Some("bbc1.uk"));
        assert_eq!(streams[1].epg_channel_id, None, "empty string -> None");
        assert_eq!(streams[2].epg_channel_id, None, "absent -> None");
    }

    #[test]
    fn vod_streams_carry_kind_and_extension() {
        let json = r#"[
            {"stream_id": 10, "name": "Skyfall", "container_extension": "mkv", "category_id": "7"},
            {"stream_id": 11, "name": "No Ext", "container_extension": ""}
        ]"#;
        let streams = parse_vod_streams(json, "src-1").unwrap();
        assert_eq!(streams[0].kind, StreamKind::Vod);
        assert_eq!(streams[0].container_extension.as_deref(), Some("mkv"));
        assert_eq!(streams[0].category_id.as_ref().unwrap().0, "7");
        assert_eq!(streams[1].container_extension, None, "empty ext -> None");
    }

    #[test]
    fn series_use_series_id_and_cover() {
        let json = r#"[{"series_id": 99, "name": "The Show", "cover": "http://c/x.jpg"}]"#;
        let streams = parse_series(json, "src-1").unwrap();
        assert_eq!(streams[0].kind, StreamKind::Series);
        assert_eq!(streams[0].provider_id, "99");
        assert_eq!(streams[0].logo.as_deref(), Some("http://c/x.jpg"));
    }

    #[test]
    fn series_info_groups_and_sorts_seasons_and_episodes() {
        let json = r#"{
            "episodes": {
                "2": [{"id": "201", "title": "S2E1", "episode_num": 1, "container_extension": "mp4"}],
                "1": [
                    {"id": "102", "title": "S1E2", "episode_num": 2},
                    {"id": "101", "title": "S1E1", "episode_num": 1}
                ]
            }
        }"#;
        let info = parse_series_info(json).unwrap();
        assert_eq!(info.seasons.len(), 2);
        assert_eq!(info.seasons[0].number, 1);
        assert_eq!(info.seasons[1].number, 2);
        // Episodes sorted within the season.
        assert_eq!(info.seasons[0].episodes[0].id, "101");
        assert_eq!(info.seasons[0].episodes[1].id, "102");
        assert_eq!(
            info.seasons[1].episodes[0].container_extension.as_deref(),
            Some("mp4")
        );
    }

    #[test]
    fn series_info_defaults_title_when_missing() {
        let json = r#"{"episodes": {"1": [{"id": "1", "episode_num": 1}]}}"#;
        let info = parse_series_info(json).unwrap();
        assert_eq!(info.seasons[0].episodes[0].title, "Episode 1");
    }
}
