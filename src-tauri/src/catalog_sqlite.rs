//! The rusqlite implementation of `cathode_core`'s `Catalog` trait.
//!
//! This is the only place SQLite is used; nothing rusqlite crosses back into
//! `core`. The connection is blocking, so async command handlers call these
//! methods through `tokio::task::spawn_blocking`. Bulk writes run in a single
//! transaction with a prepared statement.

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use cathode_core::catalog::{Catalog, SourceRecord};
use cathode_core::error::CoreError;
use cathode_core::model::category::CategoryId;
use cathode_core::model::id::StreamId;
use cathode_core::model::{Category, Programme, Stream, StreamKind};
use rusqlite::{params, Connection, OptionalExtension, Row};

/// A `Catalog` backed by a single mutex-guarded SQLite connection.
pub struct SqliteCatalog {
    conn: Mutex<Connection>,
}

fn store(context: &'static str, e: impl std::fmt::Display) -> CoreError {
    CoreError::storage(context, e.to_string())
}

/// Milliseconds since the Unix epoch (best effort; 0 if the clock is before it).
fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn kind_to_str(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Live => "live",
        StreamKind::Vod => "vod",
        StreamKind::Series => "series",
    }
}

fn kind_from_str(s: &str) -> StreamKind {
    match s {
        "vod" => StreamKind::Vod,
        "series" => StreamKind::Series,
        _ => StreamKind::Live,
    }
}

/// A strictly-increasing stamp for an ordering column (`added_at`/`watched_at`),
/// robust to several writes landing within the same millisecond. `max_sql` selects
/// the current max of that column.
fn next_stamp(conn: &Connection, max_sql: &str) -> Result<i64, CoreError> {
    let max: i64 = conn
        .query_row(max_sql, [], |r| r.get(0))
        .map_err(|e| store("read max stamp", e))?;
    Ok(unix_millis().max(max + 1))
}

/// Read a stream snapshot from a favorite/history row selected as
/// `(stream_id, provider_id, name, logo, kind, category_id, epg_channel_id, ext)`.
fn row_to_stream(row: &Row) -> rusqlite::Result<Stream> {
    let kind: String = row.get(4)?;
    let category_id: Option<String> = row.get(5)?;
    Ok(Stream {
        id: StreamId(row.get(0)?),
        provider_id: row.get(1)?,
        name: row.get(2)?,
        logo: row.get(3)?,
        category_id: category_id.map(CategoryId),
        kind: kind_from_str(&kind),
        epg_channel_id: row.get(6)?,
        container_extension: row.get(7)?,
    })
}

impl SqliteCatalog {
    /// Open (creating if needed) the catalog at `path` and run migrations.
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let conn = Connection::open(path).map_err(|e| store("open database", e))?;
        Self::from_connection(conn)
    }

    /// An in-memory catalog, for tests.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, CoreError> {
        let conn = Connection::open_in_memory().map_err(|e| store("open database", e))?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, CoreError> {
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

/// Hand-rolled migration keyed on `PRAGMA user_version` (no extra dependency).
/// Steps run in sequence, so an existing v1 database upgrades to v2 without
/// touching its v1 tables or data.
fn migrate(conn: &Connection) -> Result<(), CoreError> {
    let mut version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| store("read schema version", e))?;

    if version < 1 {
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE source (
                 id           TEXT PRIMARY KEY NOT NULL,
                 kind         TEXT NOT NULL,
                 payload      TEXT NOT NULL,
                 last_used_at INTEGER NOT NULL
             );
             CREATE TABLE category (
                 source_id TEXT NOT NULL,
                 id        TEXT NOT NULL,
                 name      TEXT NOT NULL,
                 PRIMARY KEY (source_id, id)
             );
             CREATE TABLE stream (
                 source_id   TEXT NOT NULL,
                 category_id TEXT NOT NULL,
                 stream_id   TEXT NOT NULL,
                 provider_id TEXT NOT NULL,
                 name        TEXT NOT NULL,
                 logo        TEXT,
                 kind        TEXT NOT NULL,
                 PRIMARY KEY (source_id, category_id, stream_id)
             );
             PRAGMA user_version = 1;
             COMMIT;",
        )
        .map_err(|e| store("create schema v1", e))?;
        version = 1;
    }

    if version < 2 {
        // Settings (global key/value) plus per-source favorites and watch history.
        // Favorite/history rows store a stream snapshot so they display without the
        // category cache; `added_at`/`watched_at` order them most-recent-first.
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE setting (
                 key   TEXT PRIMARY KEY NOT NULL,
                 value TEXT NOT NULL
             );
             CREATE TABLE favorite (
                 source_id   TEXT NOT NULL,
                 stream_id   TEXT NOT NULL,
                 provider_id TEXT NOT NULL,
                 name        TEXT NOT NULL,
                 logo        TEXT,
                 kind        TEXT NOT NULL,
                 category_id TEXT,
                 added_at    INTEGER NOT NULL,
                 PRIMARY KEY (source_id, stream_id)
             );
             CREATE TABLE history (
                 source_id   TEXT NOT NULL,
                 stream_id   TEXT NOT NULL,
                 provider_id TEXT NOT NULL,
                 name        TEXT NOT NULL,
                 logo        TEXT,
                 kind        TEXT NOT NULL,
                 category_id TEXT,
                 watched_at  INTEGER NOT NULL,
                 PRIMARY KEY (source_id, stream_id)
             );
             PRAGMA user_version = 2;
             COMMIT;",
        )
        .map_err(|e| store("create schema v2", e))?;
        version = 2;
    }

    if version < 3 {
        // EPG: carry each stream's epg_channel_id (tvg-id) through the snapshot
        // tables so cached/favorite/history cards can match guide programmes.
        conn.execute_batch(
            "BEGIN;
             ALTER TABLE stream ADD COLUMN epg_channel_id TEXT;
             ALTER TABLE favorite ADD COLUMN epg_channel_id TEXT;
             ALTER TABLE history ADD COLUMN epg_channel_id TEXT;
             PRAGMA user_version = 3;
             COMMIT;",
        )
        .map_err(|e| store("create schema v3", e))?;
        version = 3;
    }

    if version < 4 {
        // VOD/Series: the cached category/stream tables must be keyed by content
        // kind (Live/VOD/Series can share a category id) and remember the playable
        // file extension. These caches are disposable, so rebuild them; the
        // favorite/history snapshots are preserved and just gain a nullable `ext`.
        conn.execute_batch(
            "BEGIN;
             DROP TABLE IF EXISTS category;
             DROP TABLE IF EXISTS stream;
             CREATE TABLE category (
                 source_id TEXT NOT NULL,
                 kind      TEXT NOT NULL,
                 id        TEXT NOT NULL,
                 name      TEXT NOT NULL,
                 PRIMARY KEY (source_id, kind, id)
             );
             CREATE TABLE stream (
                 source_id      TEXT NOT NULL,
                 kind           TEXT NOT NULL,
                 category_id    TEXT NOT NULL,
                 stream_id      TEXT NOT NULL,
                 provider_id    TEXT NOT NULL,
                 name           TEXT NOT NULL,
                 logo           TEXT,
                 epg_channel_id TEXT,
                 ext            TEXT,
                 PRIMARY KEY (source_id, kind, category_id, stream_id)
             );
             ALTER TABLE favorite ADD COLUMN ext TEXT;
             ALTER TABLE history ADD COLUMN ext TEXT;
             PRAGMA user_version = 4;
             COMMIT;",
        )
        .map_err(|e| store("create schema v4", e))?;
        version = 4;
    }

    if version < 5 {
        // Persisted EPG: cache the parsed guide so it loads from disk without a fresh
        // XMLTV download. Disposable cache, replaced wholesale per source. No natural
        // primary key (a channel has many programmes); the index serves window queries.
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE programme (
                 source_id   TEXT NOT NULL,
                 channel_id  TEXT NOT NULL,
                 title       TEXT NOT NULL,
                 description TEXT,
                 start       INTEGER NOT NULL,
                 stop        INTEGER NOT NULL
             );
             CREATE INDEX programme_window ON programme (source_id, stop, start);
             PRAGMA user_version = 5;
             COMMIT;",
        )
        .map_err(|e| store("create schema v5", e))?;
    }

    Ok(())
}

impl Catalog for SqliteCatalog {
    fn upsert_source(&self, source: &SourceRecord) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        // Stamp a strictly-increasing "last used" so most-recently-used ordering is
        // robust even when several upserts land within the same millisecond.
        let max: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(last_used_at), 0) FROM source",
                [],
                |r| r.get(0),
            )
            .map_err(|e| store("read last_used_at", e))?;
        let stamp = unix_millis().max(max + 1);
        conn.execute(
            "INSERT INTO source (id, kind, payload, last_used_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                 kind = excluded.kind,
                 payload = excluded.payload,
                 last_used_at = excluded.last_used_at",
            params![source.id, source.kind, source.payload, stamp],
        )
        .map_err(|e| store("upsert source", e))?;
        Ok(())
    }

    fn sources(&self) -> Result<Vec<SourceRecord>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, kind, payload FROM source ORDER BY last_used_at DESC")
            .map_err(|e| store("prepare sources", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SourceRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    payload: row.get(2)?,
                })
            })
            .map_err(|e| store("query sources", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| store("read sources", e))
    }

    fn delete_source(&self, id: &str) -> Result<(), CoreError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| store("begin delete", e))?;
        tx.execute("DELETE FROM stream WHERE source_id = ?1", params![id])
            .map_err(|e| store("delete streams", e))?;
        tx.execute("DELETE FROM category WHERE source_id = ?1", params![id])
            .map_err(|e| store("delete categories", e))?;
        tx.execute("DELETE FROM favorite WHERE source_id = ?1", params![id])
            .map_err(|e| store("delete favorites", e))?;
        tx.execute("DELETE FROM history WHERE source_id = ?1", params![id])
            .map_err(|e| store("delete history", e))?;
        tx.execute("DELETE FROM programme WHERE source_id = ?1", params![id])
            .map_err(|e| store("delete programmes", e))?;
        tx.execute("DELETE FROM source WHERE id = ?1", params![id])
            .map_err(|e| store("delete source", e))?;
        tx.commit().map_err(|e| store("commit delete", e))
    }

    fn replace_categories(
        &self,
        source_id: &str,
        kind: StreamKind,
        categories: &[Category],
    ) -> Result<(), CoreError> {
        let kind = kind_to_str(kind);
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| store("begin categories", e))?;
        tx.execute(
            "DELETE FROM category WHERE source_id = ?1 AND kind = ?2",
            params![source_id, kind],
        )
        .map_err(|e| store("clear categories", e))?;
        {
            let mut stmt = tx
                .prepare("INSERT INTO category (source_id, kind, id, name) VALUES (?1, ?2, ?3, ?4)")
                .map_err(|e| store("prepare category insert", e))?;
            for category in categories {
                stmt.execute(params![source_id, kind, category.id.0, category.name])
                    .map_err(|e| store("insert category", e))?;
            }
        }
        tx.commit().map_err(|e| store("commit categories", e))
    }

    fn categories(&self, source_id: &str, kind: StreamKind) -> Result<Vec<Category>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, name FROM category WHERE source_id = ?1 AND kind = ?2 ORDER BY rowid",
            )
            .map_err(|e| store("prepare categories", e))?;
        let rows = stmt
            .query_map(params![source_id, kind_to_str(kind)], |row| {
                Ok(Category {
                    id: CategoryId(row.get(0)?),
                    name: row.get(1)?,
                })
            })
            .map_err(|e| store("query categories", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| store("read categories", e))
    }

    fn replace_streams(
        &self,
        source_id: &str,
        kind: StreamKind,
        category_id: &str,
        streams: &[Stream],
    ) -> Result<(), CoreError> {
        let kind = kind_to_str(kind);
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| store("begin streams", e))?;
        tx.execute(
            "DELETE FROM stream WHERE source_id = ?1 AND kind = ?2 AND category_id = ?3",
            params![source_id, kind, category_id],
        )
        .map_err(|e| store("clear streams", e))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO stream
                       (source_id, kind, category_id, stream_id, provider_id, name, logo,
                        epg_channel_id, ext)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )
                .map_err(|e| store("prepare stream insert", e))?;
            for stream in streams {
                stmt.execute(params![
                    source_id,
                    kind,
                    category_id,
                    stream.id.0,
                    stream.provider_id,
                    stream.name,
                    stream.logo,
                    stream.epg_channel_id,
                    stream.container_extension,
                ])
                .map_err(|e| store("insert stream", e))?;
            }
        }
        tx.commit().map_err(|e| store("commit streams", e))
    }

    fn streams(
        &self,
        source_id: &str,
        kind: StreamKind,
        category_id: &str,
    ) -> Result<Vec<Stream>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT stream_id, provider_id, name, logo, kind, epg_channel_id, ext FROM stream
                 WHERE source_id = ?1 AND kind = ?2 AND category_id = ?3 ORDER BY rowid",
            )
            .map_err(|e| store("prepare streams", e))?;
        let rows = stmt
            .query_map(params![source_id, kind_to_str(kind), category_id], |row| {
                let kind: String = row.get(4)?;
                Ok(Stream {
                    id: StreamId(row.get(0)?),
                    provider_id: row.get(1)?,
                    name: row.get(2)?,
                    logo: row.get(3)?,
                    category_id: Some(CategoryId(category_id.to_string())),
                    kind: kind_from_str(&kind),
                    epg_channel_id: row.get(5)?,
                    container_extension: row.get(6)?,
                })
            })
            .map_err(|e| store("query streams", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| store("read streams", e))
    }

    fn search_streams(&self, source_id: &str, query: &str) -> Result<Vec<Stream>, CoreError> {
        // Escape the LIKE wildcards so a query of "100%" matches literally.
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT stream_id, provider_id, name, logo, kind, category_id, epg_channel_id, ext
                 FROM stream
                 WHERE source_id = ?1 AND name LIKE ?2 ESCAPE '\\'
                 ORDER BY name LIMIT 200",
            )
            .map_err(|e| store("prepare search", e))?;
        let rows = stmt
            .query_map(params![source_id, pattern], row_to_stream)
            .map_err(|e| store("query search", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| store("read search", e))
    }

    fn replace_programmes(
        &self,
        source_id: &str,
        programmes: &[Programme],
    ) -> Result<(), CoreError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| store("begin programmes", e))?;
        tx.execute(
            "DELETE FROM programme WHERE source_id = ?1",
            params![source_id],
        )
        .map_err(|e| store("clear programmes", e))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO programme
                       (source_id, channel_id, title, description, start, stop)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )
                .map_err(|e| store("prepare programmes", e))?;
            for p in programmes {
                stmt.execute(params![
                    source_id,
                    p.channel_id,
                    p.title,
                    p.description,
                    p.start,
                    p.stop,
                ])
                .map_err(|e| store("insert programme", e))?;
            }
        }
        tx.commit().map_err(|e| store("commit programmes", e))
    }

    fn programmes(&self, source_id: &str, from: i64, to: i64) -> Result<Vec<Programme>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT channel_id, title, description, start, stop FROM programme
                 WHERE source_id = ?1 AND stop > ?2 AND start < ?3
                 ORDER BY start",
            )
            .map_err(|e| store("prepare programmes", e))?;
        let rows = stmt
            .query_map(params![source_id, from, to], |row| {
                Ok(Programme {
                    channel_id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    start: row.get(3)?,
                    stop: row.get(4)?,
                })
            })
            .map_err(|e| store("query programmes", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| store("read programmes", e))
    }

    fn get_setting(&self, key: &str) -> Result<Option<String>, CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM setting WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| store("get setting", e))
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO setting (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| store("set setting", e))?;
        Ok(())
    }

    fn add_favorite(&self, source_id: &str, stream: &Stream) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        let added_at = next_stamp(&conn, "SELECT COALESCE(MAX(added_at), 0) FROM favorite")?;
        let category_id = stream.category_id.as_ref().map(|c| c.0.clone());
        conn.execute(
            "INSERT INTO favorite
               (source_id, stream_id, provider_id, name, logo, kind, category_id, added_at,
                epg_channel_id, ext)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(source_id, stream_id) DO NOTHING",
            params![
                source_id,
                stream.id.0,
                stream.provider_id,
                stream.name,
                stream.logo,
                kind_to_str(stream.kind),
                category_id,
                added_at,
                stream.epg_channel_id,
                stream.container_extension,
            ],
        )
        .map_err(|e| store("add favorite", e))?;
        Ok(())
    }

    fn remove_favorite(&self, source_id: &str, stream_id: &str) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM favorite WHERE source_id = ?1 AND stream_id = ?2",
            params![source_id, stream_id],
        )
        .map_err(|e| store("remove favorite", e))?;
        Ok(())
    }

    fn favorites(&self, source_id: &str) -> Result<Vec<Stream>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT stream_id, provider_id, name, logo, kind, category_id, epg_channel_id, ext
                 FROM favorite WHERE source_id = ?1 ORDER BY added_at DESC",
            )
            .map_err(|e| store("prepare favorites", e))?;
        let rows = stmt
            .query_map(params![source_id], row_to_stream)
            .map_err(|e| store("query favorites", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| store("read favorites", e))
    }

    fn record_watch(&self, source_id: &str, stream: &Stream) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        let watched_at = next_stamp(&conn, "SELECT COALESCE(MAX(watched_at), 0) FROM history")?;
        let category_id = stream.category_id.as_ref().map(|c| c.0.clone());
        conn.execute(
            "INSERT INTO history
               (source_id, stream_id, provider_id, name, logo, kind, category_id, watched_at,
                epg_channel_id, ext)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(source_id, stream_id) DO UPDATE SET
                 provider_id = excluded.provider_id,
                 name = excluded.name,
                 logo = excluded.logo,
                 kind = excluded.kind,
                 category_id = excluded.category_id,
                 watched_at = excluded.watched_at,
                 epg_channel_id = excluded.epg_channel_id,
                 ext = excluded.ext",
            params![
                source_id,
                stream.id.0,
                stream.provider_id,
                stream.name,
                stream.logo,
                kind_to_str(stream.kind),
                category_id,
                watched_at,
                stream.epg_channel_id,
                stream.container_extension,
            ],
        )
        .map_err(|e| store("record watch", e))?;
        Ok(())
    }

    fn history(&self, source_id: &str) -> Result<Vec<Stream>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT stream_id, provider_id, name, logo, kind, category_id, epg_channel_id, ext
                 FROM history WHERE source_id = ?1 ORDER BY watched_at DESC",
            )
            .map_err(|e| store("prepare history", e))?;
        let rows = stmt
            .query_map(params![source_id], row_to_stream)
            .map_err(|e| store("query history", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| store("read history", e))
    }

    fn clear_history(&self) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM history", [])
            .map_err(|e| store("clear history", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str) -> SourceRecord {
        SourceRecord {
            id: id.to_string(),
            kind: "xtream".to_string(),
            payload: format!("{{\"id\":\"{id}\"}}"),
        }
    }

    #[test]
    fn sources_round_trip_most_recently_used_first() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        cat.upsert_source(&record("a")).unwrap();
        cat.upsert_source(&record("b")).unwrap();
        // Re-using "a" floats it back to the front even within the same millisecond.
        cat.upsert_source(&record("a")).unwrap();

        let sources = cat.sources().unwrap();
        let ids: Vec<_> = sources.iter().map(|s| s.id.clone()).collect();
        assert_eq!(ids, vec!["a", "b"]);
        // The payload round-trips so credentials can be recovered.
        assert_eq!(sources[0].payload, record("a").payload);
    }

    #[test]
    fn programmes_persist_and_window() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let p = |start: i64, stop: i64, desc: Option<&str>| Programme {
            channel_id: "bbc1.uk".to_string(),
            title: "Show".to_string(),
            description: desc.map(str::to_string),
            start,
            stop,
        };
        cat.replace_programmes(
            "a",
            &[
                p(100, 200, Some("first")),
                p(200, 300, None),
                p(300, 400, Some("third")),
            ],
        )
        .unwrap();

        // Overlap query returns only the first two, sorted by start, with descriptions.
        let got = cat.programmes("a", 150, 250).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].start, 100);
        assert_eq!(got[0].description.as_deref(), Some("first"));
        assert_eq!(got[1].description, None);

        // A re-sync replaces the whole guide for the source.
        cat.replace_programmes("a", &[p(500, 600, None)]).unwrap();
        assert_eq!(cat.programmes("a", 0, 10_000).unwrap().len(), 1);
        // Another source is empty.
        assert!(cat.programmes("b", 0, 10_000).unwrap().is_empty());
    }

    #[test]
    fn categories_replace_rather_than_append() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let sports = Category {
            id: CategoryId("1".to_string()),
            name: "Sports".to_string(),
        };
        let news = Category {
            id: CategoryId("2".to_string()),
            name: "News".to_string(),
        };
        cat.replace_categories("src-a", StreamKind::Live, &[sports.clone(), news.clone()])
            .unwrap();
        assert_eq!(
            cat.categories("src-a", StreamKind::Live).unwrap(),
            vec![sports, news.clone()]
        );

        cat.replace_categories("src-a", StreamKind::Live, std::slice::from_ref(&news))
            .unwrap();
        assert_eq!(
            cat.categories("src-a", StreamKind::Live).unwrap(),
            vec![news]
        );
        assert!(cat
            .categories("src-b", StreamKind::Live)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn streams_round_trip_and_stay_isolated() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let mut s1 = Stream::new("src-a", "1", "One", StreamKind::Live);
        s1.logo = Some("http://logo/1.png".to_string());
        let s2 = Stream::new("src-a", "2", "Two", StreamKind::Live);
        cat.replace_streams(
            "src-a",
            StreamKind::Live,
            "sports",
            &[s1.clone(), s2.clone()],
        )
        .unwrap();

        let read = cat.streams("src-a", StreamKind::Live, "sports").unwrap();
        // Stable id, provider id, logo, and kind all survive the round trip.
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].id, s1.id);
        assert_eq!(read[0].provider_id, "1");
        assert_eq!(read[0].logo, s1.logo);
        assert_eq!(read[0].kind, StreamKind::Live);
        // Other buckets are independent.
        assert!(cat
            .streams("src-a", StreamKind::Live, "news")
            .unwrap()
            .is_empty());
        assert!(cat
            .streams("src-b", StreamKind::Live, "sports")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn cache_is_isolated_by_kind_and_round_trips_ext() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let live = Stream::new("a", "1", "Live One", StreamKind::Live);
        let mut movie = Stream::new("a", "1", "Movie One", StreamKind::Vod);
        movie.container_extension = Some("mkv".to_string());
        // Same category id "5" under two kinds must not collide.
        cat.replace_streams("a", StreamKind::Live, "5", std::slice::from_ref(&live))
            .unwrap();
        cat.replace_streams("a", StreamKind::Vod, "5", std::slice::from_ref(&movie))
            .unwrap();

        let live_read = cat.streams("a", StreamKind::Live, "5").unwrap();
        let vod_read = cat.streams("a", StreamKind::Vod, "5").unwrap();
        assert_eq!(live_read.len(), 1);
        assert_eq!(live_read[0].name, "Live One");
        assert_eq!(vod_read[0].name, "Movie One");
        assert_eq!(vod_read[0].container_extension.as_deref(), Some("mkv"));
    }

    #[test]
    fn search_matches_names_across_categories_and_kinds() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let sky = Stream::new("a", "1", "Sky Sports", StreamKind::Live);
        let skyfall = Stream::new("a", "2", "Skyfall", StreamKind::Vod);
        let bbc = Stream::new("a", "3", "BBC One", StreamKind::Live);
        cat.replace_streams("a", StreamKind::Live, "sports", &[sky, bbc])
            .unwrap();
        cat.replace_streams(
            "a",
            StreamKind::Vod,
            "films",
            std::slice::from_ref(&skyfall),
        )
        .unwrap();

        let names: Vec<_> = cat
            .search_streams("a", "sky")
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, vec!["Sky Sports".to_string(), "Skyfall".to_string()]);
        assert!(cat.search_streams("b", "sky").unwrap().is_empty());
    }

    #[test]
    fn delete_source_clears_its_cache() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        cat.upsert_source(&record("a")).unwrap();
        cat.replace_categories(
            "a",
            StreamKind::Live,
            &[Category {
                id: CategoryId("1".to_string()),
                name: "Sports".to_string(),
            }],
        )
        .unwrap();
        cat.replace_streams(
            "a",
            StreamKind::Live,
            "1",
            &[Stream::new("a", "9", "Nine", StreamKind::Live)],
        )
        .unwrap();
        cat.add_favorite("a", &Stream::new("a", "9", "Nine", StreamKind::Live))
            .unwrap();
        cat.record_watch("a", &Stream::new("a", "9", "Nine", StreamKind::Live))
            .unwrap();

        cat.delete_source("a").unwrap();

        assert!(cat.sources().unwrap().is_empty());
        assert!(cat.categories("a", StreamKind::Live).unwrap().is_empty());
        assert!(cat.streams("a", StreamKind::Live, "1").unwrap().is_empty());
        assert!(cat.favorites("a").unwrap().is_empty());
        assert!(cat.history("a").unwrap().is_empty());
    }

    #[test]
    fn settings_get_set_round_trip() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        assert_eq!(cat.get_setting("settings").unwrap(), None);
        cat.set_setting("settings", "{\"favorites_enabled\":false}")
            .unwrap();
        assert_eq!(
            cat.get_setting("settings").unwrap(),
            Some("{\"favorites_enabled\":false}".to_string())
        );
        // Overwrite, not append.
        cat.set_setting("settings", "{}").unwrap();
        assert_eq!(cat.get_setting("settings").unwrap(), Some("{}".to_string()));
    }

    #[test]
    fn favorites_round_trip_order_and_isolation() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let one = Stream::new("a", "1", "One", StreamKind::Live);
        let two = Stream::new("a", "2", "Two", StreamKind::Live);
        cat.add_favorite("a", &one).unwrap();
        cat.add_favorite("a", &two).unwrap();
        cat.add_favorite("a", &one).unwrap(); // idempotent

        let ids: Vec<_> = cat
            .favorites("a")
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec![two.id.clone(), one.id.clone()]);

        cat.remove_favorite("a", &two.id.0).unwrap();
        assert_eq!(cat.favorites("a").unwrap(), vec![one]);
        assert!(cat.favorites("b").unwrap().is_empty());
    }

    #[test]
    fn history_records_dedupes_orders_and_clears() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let one = Stream::new("a", "1", "One", StreamKind::Live);
        let two = Stream::new("a", "2", "Two", StreamKind::Live);
        cat.record_watch("a", &one).unwrap();
        cat.record_watch("a", &two).unwrap();
        cat.record_watch("a", &one).unwrap(); // re-watch -> front, no dup

        let ids: Vec<_> = cat
            .history("a")
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec![one.id.clone(), two.id.clone()]);

        cat.clear_history().unwrap();
        assert!(cat.history("a").unwrap().is_empty());
    }

    #[test]
    fn v1_database_upgrades_to_latest_without_data_loss() {
        // Build a v1 database by hand (the pre-library schema) with one source row.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE source (
                 id TEXT PRIMARY KEY NOT NULL, kind TEXT NOT NULL,
                 payload TEXT NOT NULL, last_used_at INTEGER NOT NULL
             );
             CREATE TABLE category (
                 source_id TEXT NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL,
                 PRIMARY KEY (source_id, id)
             );
             CREATE TABLE stream (
                 source_id TEXT NOT NULL, category_id TEXT NOT NULL, stream_id TEXT NOT NULL,
                 provider_id TEXT NOT NULL, name TEXT NOT NULL, logo TEXT, kind TEXT NOT NULL,
                 PRIMARY KEY (source_id, category_id, stream_id)
             );
             INSERT INTO source VALUES ('a', 'xtream', '{}', 1);
             PRAGMA user_version = 1;",
        )
        .unwrap();

        // Migrating brings it to v2: old data survives and the new features work.
        let cat = SqliteCatalog::from_connection(conn).unwrap();
        assert_eq!(
            cat.sources().unwrap().len(),
            1,
            "v1 source row must survive"
        );
        cat.add_favorite("a", &Stream::new("a", "1", "One", StreamKind::Live))
            .unwrap();
        assert_eq!(cat.favorites("a").unwrap().len(), 1);
        cat.set_setting("k", "v").unwrap();
        assert_eq!(cat.get_setting("k").unwrap(), Some("v".to_string()));
    }

    #[test]
    fn epg_channel_id_round_trips_through_caches() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let mut stream = Stream::new("a", "1", "One", StreamKind::Live);
        stream.epg_channel_id = Some("bbc1.uk".to_string());

        cat.replace_streams(
            "a",
            StreamKind::Live,
            "sports",
            std::slice::from_ref(&stream),
        )
        .unwrap();
        cat.add_favorite("a", &stream).unwrap();
        cat.record_watch("a", &stream).unwrap();

        assert_eq!(
            cat.streams("a", StreamKind::Live, "sports").unwrap()[0]
                .epg_channel_id
                .as_deref(),
            Some("bbc1.uk")
        );
        assert_eq!(
            cat.favorites("a").unwrap()[0].epg_channel_id.as_deref(),
            Some("bbc1.uk")
        );
        assert_eq!(
            cat.history("a").unwrap()[0].epg_channel_id.as_deref(),
            Some("bbc1.uk")
        );
    }
}
