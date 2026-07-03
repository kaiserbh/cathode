//! Parse an extended M3U/M3U8 playlist into the normalized model.
//!
//! Pure functions: the caller fetches (or reads) the text, these turn it into
//! `Stream`s. A playlist is a flat list of channels, so every entry is treated as
//! `StreamKind::Live`; categories come from each entry's `group-title`.
//!
//! M3U is loose, so the parser is deliberately tolerant: it skips the `#EXTM3U`
//! header and unknown `#EXT…` directives, accepts `\r\n` endings, honours
//! `#EXTGRP` section groups, and even handles a plain (non-extended) playlist that
//! is just a list of URLs.

use std::collections::HashSet;

use crate::error::CoreError;
use crate::model::{Category, CategoryId, Stream, StreamKind};

/// The group an entry falls under when it carries no `group-title`/`#EXTGRP`, so
/// ungrouped channels are still reachable (the UI browses strictly by category).
const DEFAULT_GROUP: &str = "Uncategorized";

/// Metadata gathered from an `#EXTINF` line, awaiting its URL on a later line.
struct Pending {
    name: String,
    tvg_id: Option<String>,
    logo: Option<String>,
    group: Option<String>,
}

/// Parse a playlist into normalized live streams.
///
/// `source_id` comes from [`super::M3uSource::source_id`] and combines with each
/// entry's stable key (the `tvg-id` if present, else `name|url`) to derive the
/// stable [`crate::model::StreamId`]. The entry's URL is kept verbatim in
/// `provider_id` so the source can resolve it for playback.
pub fn parse_playlist(text: &str, source_id: &str) -> Result<Vec<Stream>, CoreError> {
    let mut streams = Vec::new();
    let mut pending: Option<Pending> = None;
    // A sticky `#EXTGRP` applies to following entries until the next one.
    let mut current_group: Option<String> = None;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            let (attrs_part, name_part) = split_name(rest);
            let attrs = parse_attrs(attrs_part);
            let name = {
                let from_comma = name_part.trim();
                if from_comma.is_empty() {
                    attr(&attrs, "tvg-name")
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                } else {
                    from_comma.to_string()
                }
            };
            pending = Some(Pending {
                name,
                tvg_id: cleaned(attr(&attrs, "tvg-id")),
                logo: cleaned(attr(&attrs, "tvg-logo")),
                group: cleaned(attr(&attrs, "group-title")),
            });
        } else if let Some(grp) = line.strip_prefix("#EXTGRP:") {
            let group = cleaned(Some(grp.trim()));
            current_group = group.clone();
            // `#EXTGRP` may sit between the `#EXTINF` and its URL.
            if let Some(p) = pending.as_mut() {
                if p.group.is_none() {
                    p.group = group;
                }
            }
        } else if line.starts_with('#') {
            // Some other directive (`#EXTM3U`, `#EXTVLCOPT`, …) — ignore.
            continue;
        } else {
            // A URL/path line completes the current entry. With no preceding
            // `#EXTINF` this is a plain playlist; accept it only if it looks like a
            // URL so an HTML error page doesn't masquerade as channels.
            let entry = pending.take();
            if entry.is_none() && !looks_like_url(line) {
                continue;
            }
            let url = line.to_string();
            let Pending {
                name,
                tvg_id,
                logo,
                group,
            } = entry.unwrap_or(Pending {
                name: String::new(),
                tvg_id: None,
                logo: None,
                group: None,
            });
            let name = if name.is_empty() {
                name_from_url(&url)
            } else {
                name
            };
            let group = group
                .or_else(|| current_group.clone())
                .unwrap_or_else(|| DEFAULT_GROUP.to_string());
            let stable_key = match &tvg_id {
                Some(id) => id.clone(),
                None => format!("{name}|{url}"),
            };
            let mut stream = Stream::new(source_id, &stable_key, name, StreamKind::Live);
            // The playable URL lives in `provider_id` (the one allowed source
            // asymmetry); the stable id was already derived from the key above.
            stream.provider_id = url;
            stream.logo = logo;
            stream.epg_channel_id = tvg_id;
            stream.category_id = Some(CategoryId(group));
            streams.push(stream);
        }
    }

    Ok(streams)
}

/// Derive the category list from parsed streams: each distinct `group-title`, in
/// first-seen order. The group name is both the id and the display name (M3U has
/// no separate category ids).
pub fn categories_from_streams(streams: &[Stream]) -> Vec<Category> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for stream in streams {
        if let Some(cat) = &stream.category_id {
            if seen.insert(cat.0.clone()) {
                out.push(Category {
                    id: cat.clone(),
                    name: cat.0.clone(),
                });
            }
        }
    }
    out
}

/// Split an `#EXTINF` payload (everything after `#EXTINF:`) into its attribute part
/// and the display name. The name follows the first comma that is *not* inside a
/// quoted attribute value, so `group-title="A, B",Name` splits correctly.
fn split_name(rest: &str) -> (&str, &str) {
    let mut in_quotes = false;
    for (idx, c) in rest.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => return (&rest[..idx], &rest[idx + 1..]),
            _ => {}
        }
    }
    (rest, "")
}

/// Extract `key="value"` attributes from an `#EXTINF` line. Keys are lowercased so
/// `tvg-id` and `TVG-ID` both match; values may contain spaces and unicode.
fn parse_attrs(input: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if chars[i] == '=' && i + 1 < n && chars[i + 1] == '"' {
            // The key is the run of name characters immediately before the `=`.
            let mut k = i;
            while k > 0 {
                let c = chars[k - 1];
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    k -= 1;
                } else {
                    break;
                }
            }
            let key: String = chars[k..i].iter().collect();
            // The value runs to the next quote.
            let mut j = i + 2;
            while j < n && chars[j] != '"' {
                j += 1;
            }
            let value: String = chars[i + 2..j].iter().collect();
            if !key.is_empty() {
                out.push((key.to_ascii_lowercase(), value));
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// The value of an attribute, if present.
fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Trim a candidate string, mapping empty/absent to `None`.
fn cleaned(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Whether a bare line (no preceding `#EXTINF`) is plausibly a media URL/path. Keeps
/// stray text — notably HTML from an error page — from becoming bogus channels.
fn looks_like_url(line: &str) -> bool {
    line.contains("://") || line.starts_with('/')
}

/// A readable fallback name for a bare-URL entry: the last path segment without its
/// query or extension, else the whole URL.
fn name_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let last = path.rsplit('/').next().unwrap_or(path);
    let stem = last.rsplit_once('.').map(|(s, _)| s).unwrap_or(last);
    if stem.is_empty() {
        url.to_string()
    } else {
        stem.to_string()
    }
}

/// The EPG (XMLTV) URLs a playlist declares in its `#EXTM3U` header, via the
/// `x-tvg-url` attribute (or its `url-tvg` / `tvg-url` aliases). The value is a
/// comma-separated list — split, trim, drop empties, and de-duplicate (preserving
/// order). Returns empty when there is no header or no such attribute.
pub fn epg_urls_from_header(text: &str) -> Vec<String> {
    let Some(header) = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("#EXTM3U"))
    else {
        return Vec::new();
    };
    let attrs = parse_attrs(header);
    let Some(value) = attr(&attrs, "x-tvg-url")
        .or_else(|| attr(&attrs, "url-tvg"))
        .or_else(|| attr(&attrs, "tvg-url"))
    else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    value
        .split(',')
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .filter(|u| seen.insert(u.to_string()))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::derive_stream_id;

    #[test]
    fn parses_extended_entry_with_all_attributes() {
        let m3u = "#EXTM3U\n\
            #EXTINF:-1 tvg-id=\"bbc1.uk\" tvg-name=\"BBC One\" tvg-logo=\"http://logo/bbc.png\" group-title=\"UK\",BBC One\n\
            http://server/live/123.ts\n";
        let streams = parse_playlist(m3u, "src-1").unwrap();
        assert_eq!(streams.len(), 1);
        let s = &streams[0];
        assert_eq!(s.name, "BBC One");
        assert_eq!(s.kind, StreamKind::Live);
        // The URL is kept for playback; the id is derived from the tvg-id, not the URL.
        assert_eq!(s.provider_id, "http://server/live/123.ts");
        assert_eq!(s.id, derive_stream_id("src-1", "bbc1.uk"));
        assert_eq!(s.epg_channel_id.as_deref(), Some("bbc1.uk"));
        assert_eq!(s.logo.as_deref(), Some("http://logo/bbc.png"));
        assert_eq!(s.category_id.as_ref().unwrap().0, "UK");
    }

    #[test]
    fn id_falls_back_to_name_and_url_without_tvg_id() {
        let m3u = "#EXTM3U\n\
            #EXTINF:-1 group-title=\"Movies\",Some Movie\n\
            http://server/movie/9.mkv\n";
        let s = &parse_playlist(m3u, "src-1").unwrap()[0];
        assert_eq!(s.epg_channel_id, None);
        assert_eq!(
            s.id,
            derive_stream_id("src-1", "Some Movie|http://server/movie/9.mkv")
        );
        // A re-add at a different URL is a different stream; same url+name is stable.
        let again = &parse_playlist(m3u, "src-1").unwrap()[0];
        assert_eq!(s.id, again.id);
    }

    #[test]
    fn empty_attributes_become_none() {
        let m3u = "#EXTINF:-1 tvg-id=\"\" tvg-logo=\"\",Channel\nhttp://h/c.ts\n";
        let s = &parse_playlist(m3u, "src-1").unwrap()[0];
        assert_eq!(s.epg_channel_id, None);
        assert_eq!(s.logo, None);
    }

    #[test]
    fn group_title_with_comma_is_not_confused_for_the_name() {
        let m3u = "#EXTINF:-1 group-title=\"News, World\",CNN\nhttp://h/cnn.ts\n";
        let s = &parse_playlist(m3u, "src-1").unwrap()[0];
        assert_eq!(s.name, "CNN");
        assert_eq!(s.category_id.as_ref().unwrap().0, "News, World");
    }

    #[test]
    fn honours_extgrp_and_defaults_ungrouped() {
        let m3u = "#EXTM3U\n\
            #EXTGRP:Sports\n\
            #EXTINF:-1,Match One\n\
            http://h/1.ts\n\
            #EXTINF:-1,No Group\n\
            http://h/2.ts\n";
        let streams = parse_playlist(m3u, "src-1").unwrap();
        // The sticky #EXTGRP applies to both since neither sets group-title.
        assert_eq!(streams[0].category_id.as_ref().unwrap().0, "Sports");
        assert_eq!(streams[1].category_id.as_ref().unwrap().0, "Sports");
    }

    #[test]
    fn ungrouped_entry_gets_default_category() {
        let m3u = "#EXTINF:-1,Lonely\nhttp://h/x.ts\n";
        let s = &parse_playlist(m3u, "src-1").unwrap()[0];
        assert_eq!(s.category_id.as_ref().unwrap().0, DEFAULT_GROUP);
    }

    #[test]
    fn tolerates_crlf_blank_lines_and_unknown_directives() {
        let m3u = "#EXTM3U\r\n\r\n#EXTVLCOPT:network-caching=1000\r\n\
            #EXTINF:-1 tvg-id=\"a\",A\r\nhttp://h/a.ts\r\n";
        let streams = parse_playlist(m3u, "src-1").unwrap();
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].provider_id, "http://h/a.ts");
    }

    #[test]
    fn plain_playlist_of_bare_urls() {
        let m3u = "http://server/live/bbc-one.ts\nhttp://server/live/itv.m3u8\n";
        let streams = parse_playlist(m3u, "src-1").unwrap();
        assert_eq!(streams.len(), 2);
        // Name derived from the last path segment without its extension.
        assert_eq!(streams[0].name, "bbc-one");
        assert_eq!(streams[0].category_id.as_ref().unwrap().0, DEFAULT_GROUP);
    }

    #[test]
    fn skips_non_url_noise_lines() {
        // An HTML error page returned with a 200 must not become channels.
        let html = "<!DOCTYPE html>\n<html><body>Forbidden</body></html>\n";
        assert!(parse_playlist(html, "src-1").unwrap().is_empty());
    }

    #[test]
    fn categories_are_distinct_and_first_seen_order() {
        let m3u = "#EXTINF:-1 group-title=\"B\",One\nhttp://h/1\n\
            #EXTINF:-1 group-title=\"A\",Two\nhttp://h/2\n\
            #EXTINF:-1 group-title=\"B\",Three\nhttp://h/3\n";
        let streams = parse_playlist(m3u, "src-1").unwrap();
        let cats = categories_from_streams(&streams);
        let names: Vec<_> = cats.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["B", "A"]);
    }

    #[test]
    fn reads_comma_separated_epg_urls_from_header() {
        let m3u = "#EXTM3U x-tvg-url=\"http://e/a.xml.gz, http://e/b.xml.gz ,http://e/a.xml.gz\"\n\
            #EXTINF:-1,A\nhttp://h/a.ts\n";
        // Trimmed, and the duplicate is dropped while order is preserved.
        assert_eq!(
            epg_urls_from_header(m3u),
            vec!["http://e/a.xml.gz", "http://e/b.xml.gz"]
        );
    }

    #[test]
    fn epg_header_supports_url_tvg_alias() {
        let m3u = "#EXTM3U url-tvg=\"http://e/guide.xml\"\n";
        assert_eq!(epg_urls_from_header(m3u), vec!["http://e/guide.xml"]);
    }

    #[test]
    fn no_epg_header_is_empty() {
        assert!(epg_urls_from_header("#EXTM3U\n#EXTINF:-1,A\nhttp://h/a.ts\n").is_empty());
        assert!(epg_urls_from_header("http://h/a.ts\n").is_empty());
    }
}
