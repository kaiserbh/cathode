//! Streaming XMLTV parser.
//!
//! Guides are large, so we read events incrementally with quick-xml rather than
//! building a DOM. We pull `<programme channel start stop>` plus the first
//! `<title>` and drop everything else. Timestamps (`20240115223000 +0000`) become
//! Unix seconds via chrono; this crate never reads the clock (`now` is supplied by
//! the shell), so it stays WASM-safe.

use chrono::{DateTime, NaiveDateTime};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

use crate::error::CoreError;
use crate::model::Programme;

/// Parse an XMLTV document into programmes. Entries missing a channel, start, or
/// stop are skipped rather than failing the whole parse.
pub fn parse_xmltv(xml: &str) -> Result<Vec<Programme>, CoreError> {
    let mut reader = Reader::from_str(xml);
    let mut programmes = Vec::new();
    let mut current: Option<(String, i64, i64)> = None;
    let mut in_title = false;
    let mut title = String::new();

    loop {
        match reader
            .read_event()
            .map_err(|e| CoreError::xml("xmltv guide", e.to_string()))?
        {
            Event::Start(e) if e.name().as_ref() == b"programme" => {
                current = parse_programme_attrs(&e)?;
                title.clear();
                in_title = false;
            }
            Event::Start(e) if e.name().as_ref() == b"title" => in_title = true,
            Event::End(e) if e.name().as_ref() == b"title" => in_title = false,
            // First non-empty <title> text wins (providers may repeat it per lang).
            Event::Text(e) if in_title && title.is_empty() => {
                let text = e
                    .unescape()
                    .map_err(|e| CoreError::xml("xmltv title", e.to_string()))?;
                title = text.trim().to_string();
            }
            Event::End(e) if e.name().as_ref() == b"programme" => {
                if let Some((channel_id, start, stop)) = current.take() {
                    programmes.push(Programme {
                        channel_id,
                        title: std::mem::take(&mut title),
                        start,
                        stop,
                    });
                }
                title.clear();
                in_title = false;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(programmes)
}

/// Pull `channel`/`start`/`stop` off a `<programme>` tag; `None` if any is missing.
fn parse_programme_attrs(e: &BytesStart) -> Result<Option<(String, i64, i64)>, CoreError> {
    let mut channel = None;
    let mut start = None;
    let mut stop = None;
    for attr in e.attributes() {
        let attr = attr.map_err(|e| CoreError::xml("xmltv attribute", e.to_string()))?;
        let value = attr
            .unescape_value()
            .map_err(|e| CoreError::xml("xmltv attribute", e.to_string()))?;
        match attr.key.as_ref() {
            b"channel" => channel = Some(value.into_owned()),
            b"start" => start = parse_xmltv_time(&value),
            b"stop" => stop = parse_xmltv_time(&value),
            _ => {}
        }
    }
    Ok(match (channel, start, stop) {
        (Some(channel), Some(start), Some(stop)) => Some((channel, start, stop)),
        _ => None,
    })
}

/// Parse an XMLTV timestamp into Unix seconds. Accepts an offset with or without a
/// space; a bare timestamp is treated as UTC.
fn parse_xmltv_time(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y%m%d%H%M%S %z") {
        return Some(dt.timestamp());
    }
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y%m%d%H%M%S%z") {
        return Some(dt.timestamp());
    }
    NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M%S")
        .ok()
        .map(|ndt| ndt.and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<tv>
  <channel id="bbc1.uk"><display-name>BBC One</display-name></channel>
  <programme channel="bbc1.uk" start="20240115220000 +0000" stop="20240115223000 +0000">
    <title lang="en">News &amp; Weather</title>
    <desc>ignored</desc>
  </programme>
  <programme channel="bbc1.uk" start="20240115223000 +0000" stop="20240115230000 +0000">
    <title>Film</title>
  </programme>
  <programme channel="itv.uk" start="20240115220000 +0100" stop="20240115230000 +0100">
    <title>Drama</title>
  </programme>
  <programme channel="broken.uk" start="20240115220000 +0000">
    <title>Missing stop</title>
  </programme>
</tv>"#;

    #[test]
    fn parses_programmes_titles_and_times() {
        let progs = parse_xmltv(SAMPLE).unwrap();
        // The entry missing a stop is dropped.
        assert_eq!(progs.len(), 3);

        assert_eq!(progs[0].channel_id, "bbc1.uk");
        assert_eq!(progs[0].title, "News & Weather", "entities unescaped");
        // 2024-01-15 22:00:00 UTC.
        assert_eq!(progs[0].start, 1_705_356_000);
        assert_eq!(progs[0].stop, 1_705_357_800);

        // +0100 offset is applied: 22:00 +0100 == 21:00 UTC.
        assert_eq!(progs[2].channel_id, "itv.uk");
        assert_eq!(progs[2].start, 1_705_352_400);
    }

    #[test]
    fn empty_document_is_no_programmes() {
        assert!(parse_xmltv("<tv></tv>").unwrap().is_empty());
    }
}
