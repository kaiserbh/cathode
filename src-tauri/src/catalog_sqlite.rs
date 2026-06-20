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
use cathode_core::model::{Category, Stream, StreamKind};
use rusqlite::{params, Connection};

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
fn migrate(conn: &Connection) -> Result<(), CoreError> {
    let version: i64 = conn
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
        .map_err(|e| store("create schema", e))?;
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
        tx.execute("DELETE FROM source WHERE id = ?1", params![id])
            .map_err(|e| store("delete source", e))?;
        tx.commit().map_err(|e| store("commit delete", e))
    }

    fn replace_categories(
        &self,
        source_id: &str,
        categories: &[Category],
    ) -> Result<(), CoreError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| store("begin categories", e))?;
        tx.execute(
            "DELETE FROM category WHERE source_id = ?1",
            params![source_id],
        )
        .map_err(|e| store("clear categories", e))?;
        {
            let mut stmt = tx
                .prepare("INSERT INTO category (source_id, id, name) VALUES (?1, ?2, ?3)")
                .map_err(|e| store("prepare category insert", e))?;
            for category in categories {
                stmt.execute(params![source_id, category.id.0, category.name])
                    .map_err(|e| store("insert category", e))?;
            }
        }
        tx.commit().map_err(|e| store("commit categories", e))
    }

    fn categories(&self, source_id: &str) -> Result<Vec<Category>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name FROM category WHERE source_id = ?1 ORDER BY rowid")
            .map_err(|e| store("prepare categories", e))?;
        let rows = stmt
            .query_map(params![source_id], |row| {
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
        category_id: &str,
        streams: &[Stream],
    ) -> Result<(), CoreError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| store("begin streams", e))?;
        tx.execute(
            "DELETE FROM stream WHERE source_id = ?1 AND category_id = ?2",
            params![source_id, category_id],
        )
        .map_err(|e| store("clear streams", e))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO stream
                       (source_id, category_id, stream_id, provider_id, name, logo, kind)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .map_err(|e| store("prepare stream insert", e))?;
            for stream in streams {
                stmt.execute(params![
                    source_id,
                    category_id,
                    stream.id.0,
                    stream.provider_id,
                    stream.name,
                    stream.logo,
                    kind_to_str(stream.kind),
                ])
                .map_err(|e| store("insert stream", e))?;
            }
        }
        tx.commit().map_err(|e| store("commit streams", e))
    }

    fn streams(&self, source_id: &str, category_id: &str) -> Result<Vec<Stream>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT stream_id, provider_id, name, logo, kind FROM stream
                 WHERE source_id = ?1 AND category_id = ?2 ORDER BY rowid",
            )
            .map_err(|e| store("prepare streams", e))?;
        let rows = stmt
            .query_map(params![source_id, category_id], |row| {
                let kind: String = row.get(4)?;
                Ok(Stream {
                    id: StreamId(row.get(0)?),
                    provider_id: row.get(1)?,
                    name: row.get(2)?,
                    logo: row.get(3)?,
                    category_id: Some(CategoryId(category_id.to_string())),
                    kind: kind_from_str(&kind),
                })
            })
            .map_err(|e| store("query streams", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| store("read streams", e))
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
        cat.replace_categories("src-a", &[sports.clone(), news.clone()])
            .unwrap();
        assert_eq!(cat.categories("src-a").unwrap(), vec![sports, news.clone()]);

        cat.replace_categories("src-a", std::slice::from_ref(&news))
            .unwrap();
        assert_eq!(cat.categories("src-a").unwrap(), vec![news]);
        assert!(cat.categories("src-b").unwrap().is_empty());
    }

    #[test]
    fn streams_round_trip_and_stay_isolated() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        let mut s1 = Stream::new("src-a", "1", "One", StreamKind::Live);
        s1.logo = Some("http://logo/1.png".to_string());
        let s2 = Stream::new("src-a", "2", "Two", StreamKind::Live);
        cat.replace_streams("src-a", "sports", &[s1.clone(), s2.clone()])
            .unwrap();

        let read = cat.streams("src-a", "sports").unwrap();
        // Stable id, provider id, logo, and kind all survive the round trip.
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].id, s1.id);
        assert_eq!(read[0].provider_id, "1");
        assert_eq!(read[0].logo, s1.logo);
        assert_eq!(read[0].kind, StreamKind::Live);
        // Other buckets are independent.
        assert!(cat.streams("src-a", "news").unwrap().is_empty());
        assert!(cat.streams("src-b", "sports").unwrap().is_empty());
    }

    #[test]
    fn delete_source_clears_its_cache() {
        let cat = SqliteCatalog::open_in_memory().unwrap();
        cat.upsert_source(&record("a")).unwrap();
        cat.replace_categories(
            "a",
            &[Category {
                id: CategoryId("1".to_string()),
                name: "Sports".to_string(),
            }],
        )
        .unwrap();
        cat.replace_streams("a", "1", &[Stream::new("a", "9", "Nine", StreamKind::Live)])
            .unwrap();

        cat.delete_source("a").unwrap();

        assert!(cat.sources().unwrap().is_empty());
        assert!(cat.categories("a").unwrap().is_empty());
        assert!(cat.streams("a", "1").unwrap().is_empty());
    }
}
