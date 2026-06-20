//! Raw Xtream API response shapes, as they come off the wire.
//!
//! These mirror the provider's JSON exactly and are an implementation detail of
//! the parser; nothing outside this module should see them. They exist so the
//! quirk-handling lives in one place before we map onto the clean model.

use serde::Deserialize;

/// A value Xtream sends as either a JSON string or a JSON number.
///
/// Providers are inconsistent: `stream_id` and `category_id` come back as `123`
/// from one and `"123"` from another. We normalize both to a `String` at the
/// boundary so the rest of the parser never has to care.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexStr(pub String);

impl<'de> Deserialize<'de> for FlexStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Str(String),
            Int(i64),
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Str(s) => FlexStr(s),
            Raw::Int(n) => FlexStr(n.to_string()),
        })
    }
}

/// Raw `get_live_categories` entry.
#[derive(Debug, Deserialize)]
pub struct RawCategory {
    pub category_id: FlexStr,
    pub category_name: String,
}

/// Raw `get_live_streams` entry. Fields beyond these are ignored.
#[derive(Debug, Deserialize)]
pub struct RawLiveStream {
    pub stream_id: FlexStr,
    pub name: String,
    #[serde(default)]
    pub stream_icon: Option<String>,
    #[serde(default)]
    pub category_id: Option<FlexStr>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("\"123\"", "123")] // string form
    #[case("123", "123")] // numeric form
    #[case("0", "0")]
    fn flexstr_accepts_string_or_number(#[case] json: &str, #[case] expected: &str) {
        let parsed: FlexStr = serde_json::from_str(json).unwrap();
        assert_eq!(parsed, FlexStr(expected.to_string()));
    }

    #[test]
    fn raw_live_stream_handles_missing_optionals() {
        // No stream_icon, no category_id: both default to None, stream_id coerces.
        let json = r#"{"stream_id": 42, "name": "Bare Channel"}"#;
        let raw: RawLiveStream = serde_json::from_str(json).unwrap();
        assert_eq!(raw.stream_id, FlexStr("42".to_string()));
        assert_eq!(raw.name, "Bare Channel");
        assert!(raw.stream_icon.is_none());
        assert!(raw.category_id.is_none());
    }

    #[test]
    fn raw_live_stream_handles_null_category() {
        let json = r#"{"stream_id": "7", "name": "C", "stream_icon": null, "category_id": null}"#;
        let raw: RawLiveStream = serde_json::from_str(json).unwrap();
        assert!(raw.stream_icon.is_none());
        assert!(raw.category_id.is_none());
    }
}
