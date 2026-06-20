//! Xtream Codes source.
//!
//! Xtream exposes a JSON API at `player_api.php` and serves streams from
//! path-style URLs. This module builds those URLs and (via `parse`) maps the raw
//! API responses onto the normalized model. It is pure: no network here, the
//! caller fetches the bytes and hands them to the parse functions.

pub mod api;
pub mod parse;

pub use parse::{parse_live_categories, parse_live_streams};

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

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
    /// identity is allowed to surface (AGENTS.md): Xtream uses a path-style URL.
    pub fn live_stream_url(&self, stream_id: &str, ext: &str) -> String {
        format!(
            "{}/live/{}/{}/{}.{}",
            self.base_url, self.username, self.password, stream_id, ext
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
    fn source_id_is_stable_per_account() {
        // Same account -> same id; trailing slash must not change it.
        assert_eq!(source().source_id(), "xtream:http://host:8080|user");
        assert_eq!(
            XtreamSource::new("http://host:8080/", "user", "pass").source_id(),
            source().source_id()
        );
    }
}
