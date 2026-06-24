//! Xtream Codes source.
//!
//! Xtream exposes a JSON API at `player_api.php` and serves streams from
//! path-style URLs. This module builds those URLs and (via `parse`) maps the raw
//! API responses onto the normalized model. It is pure: no network here, the
//! caller fetches the bytes and hands them to the parse functions.

pub mod api;
pub mod fetch;
pub mod parse;

pub use fetch::{fetch_categories, fetch_series_info, fetch_streams};
pub use parse::{
    parse_categories, parse_live_streams, parse_series, parse_series_info, parse_vod_streams,
};

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};

use crate::catalog::SourceRecord;
use crate::error::CoreError;

/// Discriminator stored on a persisted Xtream source.
pub const XTREAM_KIND: &str = "xtream";

/// Turn credentials into the opaque [`SourceRecord`] the catalog persists: the id
/// is the stable account id, and the payload is the credentials as JSON.
pub fn source_record(creds: &XtreamCredentials) -> Result<SourceRecord, CoreError> {
    let payload = serde_json::to_string(creds)
        .map_err(|e| CoreError::storage("serialize xtream source", e.to_string()))?;
    Ok(SourceRecord {
        id: XtreamSource::from_credentials(creds).source_id(),
        kind: XTREAM_KIND.to_string(),
        payload,
    })
}

/// Recover credentials from a persisted [`SourceRecord`] (the inverse of
/// [`source_record`]).
pub fn credentials_from_record(record: &SourceRecord) -> Result<XtreamCredentials, CoreError> {
    serde_json::from_str(&record.payload)
        .map_err(|e| CoreError::storage("deserialize xtream source", e.to_string()))
}

/// Xtream account credentials as they cross the command boundary from the UI.
///
/// A plain serde value (so it round-trips through `invoke`); turn it into an
/// [`XtreamSource`] with [`XtreamSource::from_credentials`] to get URL building.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XtreamCredentials {
    pub base_url: String,
    pub username: String,
    pub password: String,
}

/// A configured Xtream account: where to reach it and how to authenticate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XtreamSource {
    base_url: String,
    username: String,
    password: String,
}

impl XtreamSource {
    /// Build a source. The trailing slash on `base_url` is normalized away so URL
    /// construction never produces a double slash.
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self {
            base_url,
            username: username.into(),
            password: password.into(),
        }
    }

    /// Build a source from credentials received across the command boundary.
    pub fn from_credentials(creds: &XtreamCredentials) -> Self {
        Self::new(&creds.base_url, &creds.username, &creds.password)
    }

    /// A stable identifier for this account, used as the `source_id` half of a
    /// stream's stable id. Independent of trailing-slash differences; distinct
    /// across providers and usernames.
    pub fn source_id(&self) -> String {
        format!("xtream:{}|{}", self.base_url, self.username)
    }

    /// Build a `player_api.php` request URL for the given action, optionally
    /// scoped to a category. Credentials are percent-encoded for the query.
    pub fn player_api_url(&self, action: &str, category_id: Option<&str>) -> String {
        let user = encode(&self.username);
        let pass = encode(&self.password);
        let mut url = format!(
            "{}/player_api.php?username={}&password={}&action={}",
            self.base_url, user, pass, action
        );
        if let Some(category_id) = category_id {
            url.push_str("&category_id=");
            url.push_str(&encode(category_id));
        }
        url
    }

    /// Build the playable URL for a live stream. This is the one place a source's
    /// identity is allowed to surface: Xtream uses a path-style URL.
    pub fn live_stream_url(&self, stream_id: &str, ext: &str) -> String {
        format!(
            "{}/live/{}/{}/{}.{}",
            self.base_url, self.username, self.password, stream_id, ext
        )
    }

    /// Build the playable URL for a VOD (movie) stream.
    pub fn vod_stream_url(&self, stream_id: &str, ext: &str) -> String {
        format!(
            "{}/movie/{}/{}/{}.{}",
            self.base_url, self.username, self.password, stream_id, ext
        )
    }

    /// Build the playable URL for a series episode (by its episode id).
    pub fn series_episode_url(&self, episode_id: &str, ext: &str) -> String {
        format!(
            "{}/series/{}/{}/{}.{}",
            self.base_url, self.username, self.password, episode_id, ext
        )
    }

    /// Build the `get_series_info` request URL for one series.
    pub fn series_info_url(&self, series_id: &str) -> String {
        let mut url = self.player_api_url("get_series_info", None);
        url.push_str("&series_id=");
        url.push_str(&encode(series_id));
        url
    }

    /// Build the XMLTV guide URL for the whole account. Credentials are
    /// percent-encoded for the query.
    pub fn xmltv_url(&self) -> String {
        format!(
            "{}/xmltv.php?username={}&password={}",
            self.base_url,
            encode(&self.username),
            encode(&self.password)
        )
    }
}

/// Percent-encode a query value. `NON_ALPHANUMERIC` is deliberately conservative:
/// it over-encodes a few unreserved characters, which servers decode back fine,
/// and it guarantees credentials with `@`, `/`, spaces, etc. stay valid.
fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> XtreamSource {
        XtreamSource::new("http://host:8080", "user", "pass")
    }

    #[test]
    fn credentials_round_trip_and_build_a_source() {
        let creds = XtreamCredentials {
            base_url: "http://host:8080/".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        let json = serde_json::to_string(&creds).unwrap();
        let back: XtreamCredentials = serde_json::from_str(&json).unwrap();
        assert_eq!(creds, back);

        // Building through from_credentials applies the same trailing-slash
        // normalization as new(), so the source id matches.
        assert_eq!(XtreamSource::from_credentials(&creds), source());
    }

    #[test]
    fn player_api_url_without_category() {
        assert_eq!(
            source().player_api_url("get_live_categories", None),
            "http://host:8080/player_api.php?username=user&password=pass&action=get_live_categories"
        );
    }

    #[test]
    fn player_api_url_with_category() {
        assert_eq!(
            source().player_api_url("get_live_streams", Some("5")),
            "http://host:8080/player_api.php?username=user&password=pass&action=get_live_streams&category_id=5"
        );
    }

    #[test]
    fn trailing_slash_on_base_is_trimmed() {
        let s = XtreamSource::new("http://host:8080/", "user", "pass");
        assert_eq!(
            s.player_api_url("get_live_categories", None),
            "http://host:8080/player_api.php?username=user&password=pass&action=get_live_categories"
        );
    }

    #[test]
    fn credentials_are_percent_encoded_in_query() {
        let s = XtreamSource::new("http://host:8080", "u@1", "p/2 3");
        assert_eq!(
            s.player_api_url("get_live_categories", None),
            "http://host:8080/player_api.php?username=u%401&password=p%2F2%203&action=get_live_categories"
        );
    }

    #[test]
    fn live_stream_url_uses_path_style() {
        assert_eq!(
            source().live_stream_url("1001", "ts"),
            "http://host:8080/live/user/pass/1001.ts"
        );
    }

    #[test]
    fn vod_and_series_urls_use_path_style() {
        assert_eq!(
            source().vod_stream_url("2002", "mkv"),
            "http://host:8080/movie/user/pass/2002.mkv"
        );
        assert_eq!(
            source().series_episode_url("3003", "mp4"),
            "http://host:8080/series/user/pass/3003.mp4"
        );
    }

    #[test]
    fn series_info_url_appends_series_id() {
        assert_eq!(
            source().series_info_url("42"),
            "http://host:8080/player_api.php?username=user&password=pass&action=get_series_info&series_id=42"
        );
    }

    #[test]
    fn xmltv_url_is_query_style_with_encoded_creds() {
        assert_eq!(
            source().xmltv_url(),
            "http://host:8080/xmltv.php?username=user&password=pass"
        );
        let s = XtreamSource::new("http://host:8080", "u@1", "p/2");
        assert_eq!(
            s.xmltv_url(),
            "http://host:8080/xmltv.php?username=u%401&password=p%2F2"
        );
    }

    #[test]
    fn source_record_round_trips_through_credentials() {
        let creds = XtreamCredentials {
            base_url: "http://host:8080".to_string(),
            username: "user".to_string(),
            password: "p@ss/word".to_string(),
        };
        let record = source_record(&creds).unwrap();
        // The record keys on the stable account id and tags the kind.
        assert_eq!(record.id, source().source_id());
        assert_eq!(record.kind, XTREAM_KIND);
        // And it recovers the exact credentials.
        assert_eq!(credentials_from_record(&record).unwrap(), creds);
    }

    #[test]
    fn credentials_from_a_bad_record_is_a_storage_error() {
        let record = SourceRecord {
            id: "x".to_string(),
            kind: XTREAM_KIND.to_string(),
            payload: "not json".to_string(),
        };
        let err = credentials_from_record(&record).unwrap_err();
        assert_eq!(crate::error::AppError::from(err).code, "storage");
    }

    #[test]
    fn source_id_is_stable_per_account() {
        // Same account -> same id; trailing slash must not change it.
        assert_eq!(source().source_id(), "xtream:http://host:8080|user");
        assert_eq!(
            XtreamSource::new("http://host:8080/", "user", "pass").source_id(),
            source().source_id()
        );
    }
}
